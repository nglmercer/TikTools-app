use std::{
    collections::BTreeMap,
    fs,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex, RwLock,
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use tiktools_plugin_api::{
    sync::{mutex_or_recover, read_or_recover, write_or_recover},
    PluginRuntimeKind,
};

use crate::{
    discovery::{read_discovered_plugin, MAX_DIRECTORY_ENTRIES},
    worker::{run_instance_worker, QueuedCall, RunningInstance, WorkerMsg},
    DeclarativePluginRuntime, DiscoveredPlugin, NativePluginRuntime, PluginLoaderError, PluginRoot,
    PluginRuntime, ProcessPluginRuntime, WasmPluginRuntime,
};

/// Cold-start allowance for the first call of a process generation. A fresh
/// child may still be building its engine (seconds) while the descriptor
/// deadline only bounds steady-state calls (textintel answers warm calls in
/// ~1ms against a 250ms deadline). Without grace the host kills every cold
/// child before it can warm up, so the plugin can never recover.
const PROCESS_FIRST_CALL_GRACE: Duration = Duration::from_secs(10);

/// Maps a poisoned lock to a typed loader error. The poisoning is logged
/// with the shared lock-recovery target so poisoned failures stay visible
/// in telemetry instead of crashing the host.
fn lock_poisoned(lock: &'static str) -> PluginLoaderError {
    tracing::warn!(
        target: tiktools_plugin_api::sync::LOCK_RECOVERY_TARGET,
        lock = lock,
        "plugin lock poisoned; failing operation"
    );
    PluginLoaderError::LockPoisoned(lock.to_owned())
}

#[derive(Default)]
pub struct RuntimeRegistry {
    runtimes: BTreeMap<PluginRuntimeKind, Arc<dyn PluginRuntime>>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        #[cfg(feature = "native-plugins")]
        registry.register(Arc::new(NativePluginRuntime));
        registry.register(Arc::new(ProcessPluginRuntime));
        registry.register(Arc::new(WasmPluginRuntime));
        registry.register(Arc::new(DeclarativePluginRuntime));
        registry
    }

    pub fn register(&mut self, runtime: Arc<dyn PluginRuntime>) {
        self.runtimes.insert(runtime.kind(), runtime);
    }

    fn get(&self, kind: PluginRuntimeKind) -> Option<Arc<dyn PluginRuntime>> {
        self.runtimes.get(&kind).cloned()
    }
}

#[derive(Default)]
struct PluginRegistry {
    entries: BTreeMap<String, DiscoveredPlugin>,
}

/// Runtime plugin manager. Discovery is deterministic: later roots override
/// earlier roots by plugin id, so development overrides can replace built-ins
/// without a compiled registration list.
pub struct PluginManager {
    roots: Vec<PluginRoot>,
    registry: RwLock<PluginRegistry>,
    runtimes: RuntimeRegistry,
    instances: RwLock<BTreeMap<String, Arc<RunningInstance>>>,
    lifecycle: Mutex<()>,
    next_token: AtomicU64,
}

impl PluginManager {
    pub fn new(roots: Vec<PluginRoot>) -> Self {
        Self::with_runtimes(roots, RuntimeRegistry::new())
    }

    pub fn with_runtimes(roots: Vec<PluginRoot>, runtimes: RuntimeRegistry) -> Self {
        Self {
            roots,
            registry: RwLock::new(PluginRegistry::default()),
            runtimes,
            instances: RwLock::new(BTreeMap::new()),
            lifecycle: Mutex::new(()),
            next_token: AtomicU64::new(1),
        }
    }

    pub fn roots(&self) -> &[PluginRoot] {
        &self.roots
    }

    pub fn scan(&self) -> Result<Vec<DiscoveredPlugin>, PluginLoaderError> {
        let mut discovered = BTreeMap::new();
        for root in &self.roots {
            if !root.path.exists() {
                continue;
            }
            let entries = fs::read_dir(&root.path).map_err(|error| {
                PluginLoaderError::InvalidDirectory(format!("{}: {error}", root.path.display()))
            })?;
            for (index, entry) in entries.enumerate() {
                if index >= MAX_DIRECTORY_ENTRIES {
                    tracing::warn!(path = %root.path.display(), "plugin directory entry limit reached");
                    break;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        tracing::warn!(path = %root.path.display(), %error, "could not inspect plugin entry");
                        continue;
                    }
                };
                let directory = entry.path();
                if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                    continue;
                }
                match read_discovered_plugin(&directory, root.source) {
                    Ok(plugin) => {
                        if let Some(previous) =
                            discovered.insert(plugin.manifest.id.clone(), plugin)
                        {
                            tracing::debug!(
                                id = %previous.manifest.id,
                                "plugin overridden by a later runtime root"
                            );
                        }
                    }
                    Err(error) => {
                        tracing::warn!(path = %directory.display(), %error, "plugin skipped")
                    }
                }
            }
        }

        let running_ids = self
            .instances
            .read()
            .map_err(|_| lock_poisoned("plugin instances"))?
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let result: Vec<_> = discovered
            .into_values()
            .map(|mut plugin| {
                // The active instance registry is authoritative. A scan must
                // never report a live runtime as stopped merely because it
                // rebuilt the discovered package record.
                plugin.running = running_ids.contains(&plugin.manifest.id);
                plugin
            })
            .collect();
        let mut registry = self
            .registry
            .write()
            .map_err(|_| lock_poisoned("plugin registry"))?;
        registry.entries = result
            .iter()
            .cloned()
            .map(|plugin| (plugin.manifest.id.clone(), plugin))
            .collect();
        Ok(result)
    }

    pub fn list(&self) -> Vec<DiscoveredPlugin> {
        read_or_recover(&self.registry, "plugin registry")
            .entries
            .values()
            .cloned()
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<DiscoveredPlugin> {
        read_or_recover(&self.registry, "plugin registry")
            .entries
            .get(id)
            .cloned()
    }

    pub fn is_running(&self, id: &str) -> bool {
        read_or_recover(&self.instances, "plugin instances").contains_key(id)
    }

    pub fn start(&self, id: &str) -> Result<(), PluginLoaderError> {
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| lock_poisoned("plugin lifecycle"))?;
        if self
            .instances
            .read()
            .map_err(|_| lock_poisoned("plugin instances"))?
            .contains_key(id)
        {
            return Ok(());
        }
        let plugin = self
            .get(id)
            .ok_or_else(|| PluginLoaderError::NotFound(id.to_owned()))?;
        if !plugin.available {
            return Err(PluginLoaderError::RuntimeUnavailable(
                plugin
                    .reason
                    .unwrap_or_else(|| "plugin is unavailable".to_owned()),
            ));
        }
        let runtime = self.runtimes.get(plugin.manifest.runtime).ok_or_else(|| {
            PluginLoaderError::RuntimeUnavailable(plugin.manifest.runtime.to_string())
        })?;
        let instance = runtime.load(&plugin.manifest, &plugin.directory)?;
        let token = self.next_token.fetch_add(1, Ordering::AcqRel);
        let (tx, rx) = mpsc::channel();
        let worker_id = id.to_owned();
        let worker = thread::Builder::new()
            .name("tiktools-plugin".to_owned())
            .spawn(move || run_instance_worker(worker_id, instance, rx))
            .map_err(|error| {
                PluginLoaderError::Runtime(format!("could not start plugin worker: {error}"))
            })?;
        // The worker thread is already spawned here, so a poisoned map
        // recovers instead of failing midway through the mutation.
        write_or_recover(&self.instances, "plugin instances").insert(
            id.to_owned(),
            Arc::new(RunningInstance {
                token,
                kind: plugin.manifest.runtime,
                cold: AtomicBool::new(true),
                tx,
                worker: Mutex::new(Some(worker)),
            }),
        );
        self.set_running(id, true);
        Ok(())
    }

    pub fn stop(&self, id: &str) -> Result<(), PluginLoaderError> {
        let instance = {
            let _lifecycle = self
                .lifecycle
                .lock()
                .map_err(|_| lock_poisoned("plugin lifecycle"))?;
            self.remove_instance(id)
        };
        let Some(instance) = instance else {
            self.set_running(id, false);
            return Ok(());
        };
        let _ = instance.tx.send(WorkerMsg::Shutdown);
        // The instance is already removed from the map here, so a
        // poisoned worker lock recovers instead of failing the stop.
        let worker = mutex_or_recover(&instance.worker, "plugin worker").take();
        let result = match worker {
            Some(worker) => worker
                .join()
                .map_err(|_| PluginLoaderError::Runtime(format!("plugin `{id}` worker panicked")))
                .and_then(|inner| inner),
            None => Ok(()),
        };
        self.set_running(id, false);
        result
    }

    pub fn stop_all(&self) {
        let ids: Vec<String> = read_or_recover(&self.instances, "plugin instances")
            .keys()
            .cloned()
            .collect();
        for id in ids {
            if let Err(error) = self.stop(&id) {
                tracing::warn!(id = %id, %error, "plugin shutdown failed");
            }
        }
    }

    pub fn call(&self, id: &str, request: &Value) -> Result<Value, PluginLoaderError> {
        self.call_with_timeout(id, request, Duration::from_secs(30))
    }

    /// Claims the cold-start grace for one process instance, if still
    /// unclaimed, and returns the deadline the caller must enforce.
    ///
    /// Tokio-side callers race a deadline against the worker-side waiter, so
    /// both must cover the same budget: without this, a fresh child still
    /// building its engine (seconds) is failed by the steady-state descriptor
    /// deadline (milliseconds) before the worker's own grace can warm it up.
    /// Exactly one first call per generation wins the swap; the worker's swap
    /// in [`Self::call_with_deadline`] then finds the grace already consumed,
    /// so concurrent first calls never double-extend it.
    pub fn claim_cold_start_grace(&self, id: &str, timeout: Duration) -> Duration {
        let cold_process = read_or_recover(&self.instances, "plugin instances")
            .get(id)
            .filter(|instance| instance.kind == PluginRuntimeKind::Process)
            .is_some_and(|instance| instance.cold.swap(false, Ordering::AcqRel));
        if cold_process {
            timeout.max(PROCESS_FIRST_CALL_GRACE)
        } else {
            timeout
        }
    }

    /// Calls one plugin instance. Calls stay serialized per instance through
    /// its worker queue, so a slow call delays (but never starves) later
    /// calls to the same plugin. Keep processor plugins fast and never
    /// combine slow model preparation with enrichment in one process.
    ///
    /// TODO(protocol-v2): per-QoS-class instances or multiplexed process
    /// requests would let realtime processors skip ahead of background
    /// actions sharing one process.
    pub fn call_with_timeout(
        &self,
        id: &str,
        request: &Value,
        timeout: Duration,
    ) -> Result<Value, PluginLoaderError> {
        self.call_with_deadline(id, request, Instant::now() + timeout)
    }

    /// Calls one plugin instance with an absolute deadline covering queueing
    /// plus execution. The worker discards requests that expire while queued
    /// instead of executing stale work; the waiter gives up at the same
    /// deadline. A timeout never removes a native instance, while transport
    /// and protocol errors retire it so the next start loads fresh state.
    ///
    /// Process instances differ twice: the first call of a generation gets
    /// cold-start grace (a fresh child may still be building its engine),
    /// and a timeout retires the instance because the worker enforces the
    /// same deadline on its I/O and kills the child when it fires. Keeping a
    /// timed-out process instance would fail the next call instantly with
    /// "plugin process stdin is unavailable".
    pub fn call_with_deadline(
        &self,
        id: &str,
        request: &Value,
        deadline: Instant,
    ) -> Result<Value, PluginLoaderError> {
        let bytes = serde_json::to_vec(request)
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        let instance = self
            .instances
            .read()
            .map_err(|_| lock_poisoned("plugin instances"))?
            .get(id)
            .cloned()
            .ok_or_else(|| PluginLoaderError::NotFound(id.to_owned()))?;
        let now = Instant::now();
        if now >= deadline {
            return Err(PluginLoaderError::Timeout(format!(
                "plugin `{id}` call already expired"
            )));
        }
        let is_process = instance.kind == PluginRuntimeKind::Process;
        let deadline = if is_process && instance.cold.swap(false, Ordering::AcqRel) {
            deadline.max(now + PROCESS_FIRST_CALL_GRACE)
        } else {
            deadline
        };
        let remaining = deadline - now;
        let (respond, answer) = mpsc::channel();
        instance
            .tx
            .send(WorkerMsg::Call(QueuedCall {
                request: bytes,
                timeout: remaining,
                deadline,
                respond,
            }))
            .map_err(|_| {
                self.remove_failed_worker(id, instance.token);
                PluginLoaderError::Runtime(format!("plugin `{id}` worker is gone"))
            })?;
        let response = answer
            .recv_timeout(remaining)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => {
                    if is_process {
                        self.remove_failed_worker(id, instance.token);
                    }
                    PluginLoaderError::Timeout(format!("plugin `{id}` call timed out"))
                }
                mpsc::RecvTimeoutError::Disconnected => {
                    self.remove_failed_worker(id, instance.token);
                    PluginLoaderError::Runtime(format!("plugin `{id}` worker died"))
                }
            })?;
        let response = match response {
            Ok(response) => response,
            // The worker discarded this request as expired; the instance
            // itself is healthy, so it stays running.
            Err(error @ PluginLoaderError::Timeout(_)) => return Err(error),
            Err(error) => {
                self.remove_failed_worker(id, instance.token);
                return Err(error);
            }
        };
        match serde_json::from_slice(&response) {
            Ok(response) => Ok(response),
            Err(error) => {
                self.remove_failed_worker(id, instance.token);
                Err(PluginLoaderError::Runtime(format!(
                    "plugin returned invalid JSON: {error}"
                )))
            }
        }
    }

    pub fn start_all(&self) -> Vec<(String, Result<(), PluginLoaderError>)> {
        self.list()
            .into_iter()
            .map(|plugin| {
                let id = plugin.manifest.id;
                let result = self.start(&id);
                (id, result)
            })
            .collect()
    }

    fn set_running(&self, id: &str, running: bool) {
        if let Some(plugin) = write_or_recover(&self.registry, "plugin registry")
            .entries
            .get_mut(id)
        {
            plugin.running = running;
        }
    }

    fn remove_instance(&self, id: &str) -> Option<Arc<RunningInstance>> {
        write_or_recover(&self.instances, "plugin instances").remove(id)
    }

    /// Retires the instance generation identified by `token`; a newer start
    /// is never disturbed. The worker is asked to stop but not joined, so a
    /// stuck plugin cannot block the hot path that observed the failure.
    fn remove_failed_worker(&self, id: &str, token: u64) {
        let removed = {
            let _lifecycle = mutex_or_recover(&self.lifecycle, "plugin lifecycle");
            let mut instances = write_or_recover(&self.instances, "plugin instances");
            if instances
                .get(id)
                .is_some_and(|current| current.token == token)
            {
                instances.remove(id)
            } else {
                None
            }
        };
        if let Some(instance) = removed {
            let _ = instance.tx.send(WorkerMsg::Shutdown);
            self.set_running(id, false);
        }
    }
}

#[cfg(test)]
impl PluginManager {
    /// Poisons one running instance's worker lock so tests can exercise
    /// the stop-path recovery. Test-only; production code never calls this.
    pub(crate) fn poison_worker_for_test(&self, id: &str) {
        if let Ok(instances) = self.instances.read() {
            if let Some(instance) = instances.get(id) {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _guard = instance.worker.lock().unwrap();
                    panic!("test poison");
                }));
            }
        }
    }

    /// Poisons the registry, instance, lifecycle, and worker locks so
    /// tests can exercise every poison path. Test-only.
    pub(crate) fn poison_locks_for_test(&self) {
        if let Ok(instances) = self.instances.read() {
            for instance in instances.values() {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _guard = instance.worker.lock().unwrap();
                    panic!("test poison");
                }));
            }
        }
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _registry = self.registry.write().unwrap();
            let _instances = self.instances.write().unwrap();
            let _lifecycle = self.lifecycle.lock().unwrap();
            panic!("test poison");
        }));
    }
}
