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
use tiktools_plugin_api::PluginRuntimeKind;

use crate::{
    discovery::{read_discovered_plugin, MAX_DIRECTORY_ENTRIES},
    worker::{run_instance_worker, QueuedCall, RunningInstance, WorkerMsg},
    DiscoveredPlugin, NativePluginRuntime, PluginLoaderError, PluginRoot, PluginRuntime,
    ProcessPluginRuntime, WasmPluginRuntime,
};

/// Cold-start allowance for the first call of a process generation. A fresh
/// child may still be building its engine (seconds) while the descriptor
/// deadline only bounds steady-state calls (textintel answers warm calls in
/// ~1ms against a 250ms deadline). Without grace the host kills every cold
/// child before it can warm up, so the plugin can never recover.
const PROCESS_FIRST_CALL_GRACE: Duration = Duration::from_secs(10);
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
            .expect("plugin instances poisoned")
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
        let mut registry = self.registry.write().expect("plugin registry poisoned");
        registry.entries = result
            .iter()
            .cloned()
            .map(|plugin| (plugin.manifest.id.clone(), plugin))
            .collect();
        Ok(result)
    }

    pub fn list(&self) -> Vec<DiscoveredPlugin> {
        self.registry
            .read()
            .expect("plugin registry poisoned")
            .entries
            .values()
            .cloned()
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<DiscoveredPlugin> {
        self.registry
            .read()
            .expect("plugin registry poisoned")
            .entries
            .get(id)
            .cloned()
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.instances
            .read()
            .expect("plugin instances poisoned")
            .contains_key(id)
    }

    pub fn start(&self, id: &str) -> Result<(), PluginLoaderError> {
        let _lifecycle = self.lifecycle.lock().expect("plugin lifecycle poisoned");
        if self
            .instances
            .read()
            .expect("plugin instances poisoned")
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
        self.instances
            .write()
            .expect("plugin instances poisoned")
            .insert(
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
            let _lifecycle = self.lifecycle.lock().expect("plugin lifecycle poisoned");
            self.remove_instance(id)
        };
        let Some(instance) = instance else {
            self.set_running(id, false);
            return Ok(());
        };
        let _ = instance.tx.send(WorkerMsg::Shutdown);
        let worker = instance
            .worker
            .lock()
            .expect("plugin worker poisoned")
            .take();
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
        let ids: Vec<String> = self
            .instances
            .read()
            .expect("plugin instances poisoned")
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
            .expect("plugin instances poisoned")
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
        if let Some(plugin) = self
            .registry
            .write()
            .expect("plugin registry poisoned")
            .entries
            .get_mut(id)
        {
            plugin.running = running;
        }
    }

    fn remove_instance(&self, id: &str) -> Option<Arc<RunningInstance>> {
        self.instances
            .write()
            .expect("plugin instances poisoned")
            .remove(id)
    }

    /// Retires the instance generation identified by `token`; a newer start
    /// is never disturbed. The worker is asked to stop but not joined, so a
    /// stuck plugin cannot block the hot path that observed the failure.
    fn remove_failed_worker(&self, id: &str, token: u64) {
        let removed = {
            let _lifecycle = self.lifecycle.lock().expect("plugin lifecycle poisoned");
            let mut instances = self.instances.write().expect("plugin instances poisoned");
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
