use std::{
    collections::BTreeMap,
    fs,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use tiktools_plugin_api::{
    sync::{recover_mutex, recover_rwlock_read, recover_rwlock_write},
    PluginRuntimeKind,
};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::{
    discovery::{read_discovered_plugin, MAX_DIRECTORY_ENTRIES},
    quickjs::{native_addons_allowed, untrusted_native_addons},
    worker::{run_instance_worker, QueuedCall, RunningInstance, WorkerMsg},
    DeclarativePluginRuntime, DiscoveredPlugin, EmittedEvent, NapiVmPluginRuntime,
    NativePluginRuntime, PluginLoaderError, PluginRoot, PluginRuntime, ProcessPluginRuntime,
    WasmPluginRuntime, WorkerContext,
};

/// Cold-start allowance for the first call of a process generation. A fresh
/// child may still be building its engine (seconds) while the descriptor
/// deadline only bounds steady-state calls (textintel answers warm calls in
/// ~1ms against a 250ms deadline). Without grace the host kills every cold
/// child before it can warm up, so the plugin can never recover.
const PROCESS_FIRST_CALL_GRACE: Duration = Duration::from_secs(10);

/// Default bound for joining a plugin worker during stop/shutdown. A guest
/// stuck in `onUnload` or native teardown cannot block process shutdown
/// forever: the wait is abandoned with an explicit error report (the thread
/// keeps running and its outcome is unknown) instead of hanging. Overridable
/// per manager so tests can prove the bound without a process-wide knob.
pub(crate) const PLUGIN_STOP_DEADLINE: Duration = Duration::from_secs(5);

/// Default bound for the napi-vm startup handshake (`onLoad`). It runs while
/// the global lifecycle lock is held, so it also bounds how long one stuck
/// plugin can delay every other start/stop.
pub(crate) const PLUGIN_START_DEADLINE: Duration = Duration::from_secs(15);

/// Upper bound on workers kept for later reaping after a hot-path failure
/// retirement. Beyond this the oldest are abandoned explicitly (logged), so
/// a plugin failing in a tight loop cannot grow the list without bound.
const MAX_RETIRED_WORKERS: usize = 32;

/// Poll interval while a bounded join waits for a worker to finish. The
/// worker's own exit is the primary signal; this only bounds how long a
/// missed wakeup delays the joiner.
const JOIN_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Per-manager lifecycle deadlines. Held together so a single lock covers
/// both bounds and a test can replace them atomically.
#[derive(Debug, Clone, Copy)]
struct PluginDeadlines {
    stop: Duration,
    start: Duration,
}

impl Default for PluginDeadlines {
    fn default() -> Self {
        Self {
            stop: PLUGIN_STOP_DEADLINE,
            start: PLUGIN_START_DEADLINE,
        }
    }
}

/// A worker whose join handle is owned by the manager rather than by the
/// caller that observed the failure. The hot path that retires a failed
/// generation cannot block on a join, so it hands the handle here; reaping
/// is opportunistic (every later retire/stop/shutdown) and bounded.
struct RetiredWorker {
    id: String,
    token: u64,
    worker: Option<thread::JoinHandle<Result<(), PluginLoaderError>>>,
}

/// Snapshot of the host-owned worker; not a guarantee of plugin health or of
/// any process not owned by this manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginWorkerObservation {
    WorkerPresent,
    Stopped,
    Uncertain,
}

struct StoppingGuard<'a> {
    manager: &'a PluginManager,
    id: String,
}

impl Drop for StoppingGuard<'_> {
    fn drop(&mut self) {
        let mut stopping = recover_mutex(&self.manager.stopping, "plugin stopping");
        if let Some(count) = stopping.get_mut(&self.id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                stopping.remove(&self.id);
            }
        }
    }
}

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
        registry.register(Arc::new(NapiVmPluginRuntime));
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
///
/// Lock-poison policy, per primitive (mixed behavior is deliberate; each
/// site below names the rule it follows):
///
/// - `registry`: reads recover (`list`, `get`); the full rebuild in `scan`
///   is strict (a torn rebuild must not publish a partial registry), while
///   `set_running` recovers (it runs past the irreversible boundary — the
///   worker is already spawned or removed — so it must not fail midway).
/// - `instances`: reads recover where a stale snapshot is safe (`list`,
///   `is_running`, `claim_cold_start_grace`, `stop_all` enumeration);
///   `start`'s existence check and `call`'s lookup are strict (acting on a
///   torn map could double-start or misroute); mutation recovers during
///   cleanup/retirement (`remove_instance`, `remove_failed_worker`) and in
///   `start` after the worker thread is already spawned.
/// - `lifecycle`: normal mutations are strict (`start`, `stop` hold it
///   fail-closed so a torn transition fails typed); shutdown
///   (`stop_for_shutdown`) and failure retirement (`remove_failed_worker`)
///   recover so one poisoned transition cannot wedge cleanup forever.
/// - `worker` mutex: cleanup/stop recovers (`finish_stop` takes the handle
///   after the instance is already out of the map).
pub struct PluginManager {
    roots: Vec<PluginRoot>,
    registry: RwLock<PluginRegistry>,
    runtimes: RuntimeRegistry,
    instances: RwLock<BTreeMap<String, Arc<RunningInstance>>>,
    lifecycle: Mutex<()>,
    next_token: AtomicU64,
    emit_bus: broadcast::Sender<EmittedEvent>,
    /// Bumped on every registry/instance mutation (scan, start, stop,
    /// generation retirement). The host's event fan-out cache validates
    /// against this instead of re-cloning the registry per event.
    registry_revision: AtomicU64,
    /// Lifecycle join bounds. Per manager (not process-wide env reads on the
    /// stop path) so one manager's deadline cannot be shortened by another
    /// manager, and tests can inject a short bound without a global.
    deadlines: Mutex<PluginDeadlines>,
    /// Workers retired by a failure observed on the hot path, awaiting an
    /// opportunistic bounded reap. Ownership is retained here: a handle is
    /// never dropped without its outcome being joined or explicitly
    /// abandoned.
    retired: Mutex<Vec<RetiredWorker>>,
    /// Stop/retirement may continue after its instance leaves the map. Keep
    /// that interval observable rather than claiming the worker has stopped.
    stopping: Mutex<BTreeMap<String, usize>>,
    /// Workers explicitly abandoned past a deadline (outcome unknown).
    abandoned_workers: AtomicU64,
    isolated_admission: Arc<tokio::sync::Semaphore>,
}

/// Push-bus buffer: burst headroom for guest-emitted events while core
/// drains. Bounded: a lagging subscriber drops with a `Lagged` error
/// instead of growing memory, and emits stay small by construction.
const EMIT_BUS_CAPACITY: usize = 256;

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
            emit_bus: broadcast::channel(EMIT_BUS_CAPACITY).0,
            registry_revision: AtomicU64::new(1),
            deadlines: Mutex::new(PluginDeadlines::default()),
            retired: Mutex::new(Vec::new()),
            stopping: Mutex::new(BTreeMap::new()),
            abandoned_workers: AtomicU64::new(0),
            isolated_admission: Arc::new(tokio::sync::Semaphore::new(16)),
        }
    }

    /// Lifecycle join bounds in force.
    fn deadlines(&self) -> PluginDeadlines {
        *recover_mutex(&self.deadlines, "plugin deadlines")
    }

    /// Monotonic revision of registry/instance state. Any change that can
    /// alter event delivery (install, uninstall, start, stop, generation
    /// retirement) bumps it; the host rebuilds its cached fan-out only
    /// when this moves.
    pub fn registry_revision(&self) -> u64 {
        self.registry_revision.load(Ordering::Acquire)
    }

    fn bump_registry_revision(&self) {
        self.registry_revision.fetch_add(1, Ordering::AcqRel);
    }

    pub fn roots(&self) -> &[PluginRoot] {
        &self.roots
    }

    /// Subscribe to guest-pushed events from every managed plugin. Core
    /// runs one forwarder over this; the poll path is unchanged and stays
    /// as the fallback for runtimes and guests without push.
    pub fn subscribe_emitted_events(&self) -> broadcast::Receiver<EmittedEvent> {
        self.emit_bus.subscribe()
    }

    pub fn scan(&self) -> Result<Vec<DiscoveredPlugin>, PluginLoaderError> {
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| lock_poisoned("plugin lifecycle"))?;
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
                let hidden = entry.file_name().to_string_lossy().starts_with('.');
                if hidden || !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
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
        drop(registry);
        self.bump_registry_revision();
        Ok(result)
    }

    pub fn list(&self) -> Vec<DiscoveredPlugin> {
        recover_rwlock_read(&self.registry, "plugin registry")
            .entries
            .values()
            .cloned()
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<DiscoveredPlugin> {
        recover_rwlock_read(&self.registry, "plugin registry")
            .entries
            .get(id)
            .cloned()
    }

    /// Reads catalog metadata without recovering a poisoned registry. Use
    /// this when the result authorizes a new runtime or sync operation.
    pub fn get_for_admission(
        &self,
        id: &str,
    ) -> Result<Option<DiscoveredPlugin>, PluginLoaderError> {
        let registry = self
            .registry
            .read()
            .map_err(|_| lock_poisoned("plugin registry"))?;
        Ok(registry.entries.get(id).cloned())
    }

    pub fn is_running(&self, id: &str) -> bool {
        recover_rwlock_read(&self.instances, "plugin instances").contains_key(id)
    }

    /// A conservative instantaneous observation of host-owned workers.
    /// It does not prove the health of a child process, fence other hosts,
    /// or prevent a worker from exiting after the observation.
    pub fn observe_worker(&self, id: &str) -> Result<PluginWorkerObservation, PluginLoaderError> {
        let _lifecycle = match self.lifecycle.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => {
                return Ok(PluginWorkerObservation::Uncertain)
            }
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(lock_poisoned("plugin lifecycle"))
            }
        };
        let stopping = self
            .stopping
            .lock()
            .map_err(|_| lock_poisoned("plugin stopping"))?;
        if stopping.contains_key(id) {
            return Ok(PluginWorkerObservation::Uncertain);
        }
        let retired = self
            .retired
            .lock()
            .map_err(|_| lock_poisoned("plugin retired workers"))?;
        // Overflow retirement increments the abandoned count while holding
        // this mutex. Check after taking it so a removed handle cannot become
        // invisible between our count check and the retired-list observation.
        if self.abandoned_workers.load(Ordering::Acquire) != 0 {
            return Ok(PluginWorkerObservation::Uncertain);
        }
        if retired.iter().any(|worker| worker.id == id) {
            return Ok(PluginWorkerObservation::Uncertain);
        }
        let instances = self
            .instances
            .read()
            .map_err(|_| lock_poisoned("plugin instances"))?;
        let Some(instance) = instances.get(id) else {
            return Ok(PluginWorkerObservation::Stopped);
        };
        let worker = instance
            .worker
            .lock()
            .map_err(|_| lock_poisoned("plugin worker"))?;
        Ok(
            if worker.as_ref().is_some_and(|handle| !handle.is_finished()) {
                PluginWorkerObservation::WorkerPresent
            } else {
                PluginWorkerObservation::Uncertain
            },
        )
    }

    /// Caller holds lifecycle. Recovery below is fail-closed if this mutex is
    /// poisoned, but normal stop must still make a best effort to finish.
    fn note_stopping(&self, id: &str) -> StoppingGuard<'_> {
        let mut stopping = recover_mutex(&self.stopping, "plugin stopping");
        let count = stopping.entry(id.to_owned()).or_default();
        *count = count.saturating_add(1);
        StoppingGuard {
            manager: self,
            id: id.to_owned(),
        }
    }

    pub fn start(&self, id: &str) -> Result<(), PluginLoaderError> {
        self.start_generation(id).map(|_| ())
    }

    /// Starts or reuses a worker and returns its identity while lifecycle is
    /// still locked. A subsequent call can bind to that exact worker instead
    /// of accidentally crossing a concurrent stop/restart.
    pub fn start_generation(&self, id: &str) -> Result<u64, PluginLoaderError> {
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| lock_poisoned("plugin lifecycle"))?;
        // A worker that died without a caller observing it still holds its
        // map entry; `contains_key` alone would report it running forever
        // and refuse to restart it. Only a live generation short-circuits
        // here — a finished one is retired (ownership retained) and this
        // start spawns a fresh generation. The lookup guard is dropped
        // before retiring: retirement takes the instances write lock, and
        // an edition-2021 `if let` would otherwise hold the read guard
        // across that call.
        let existing = recover_rwlock_read(&self.instances, "plugin instances")
            .get(id)
            .cloned();
        if let Some(existing) = existing {
            if !existing.worker_finished() {
                return Ok(existing.token);
            }
            tracing::warn!(
                id = %id,
                token = existing.token,
                "plugin worker exited without being observed; retiring the dead generation"
            );
            if let Some(instance) = self.retire_generation_locked(id, existing.token) {
                self.retire_instance(id, existing.token, instance);
            }
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
        if !plugin.manifest.native_addons.is_empty()
            && !native_addons_allowed(&plugin.manifest, plugin.source)
        {
            return Err(untrusted_native_addons(&plugin.manifest.id));
        }
        let runtime = self.runtimes.get(plugin.manifest.runtime).ok_or_else(|| {
            PluginLoaderError::RuntimeUnavailable(plugin.manifest.runtime.to_string())
        })?;
        // Lifecycle entry point: a load that never finishes (a guest stuck
        // in onLoad, a native addon hanging) shows here as `starting` with
        // no matching `started`, which is the whole diagnostic.
        let start_started = std::time::Instant::now();
        tracing::info!(
            id = %id,
            runtime = %plugin.manifest.runtime,
            "plugin starting"
        );
        let token = self.next_token.fetch_add(1, Ordering::AcqRel);
        let artifact_fingerprint = crate::plugin_artifact_fingerprint(&plugin).ok();
        // Runtimes with an owned worker (napi-vm) serve the call queue on
        // their own thread, bypassing the generic worker: one thread per
        // plugin instead of two, with identical queue/deadline semantics.
        let worker_ctx = WorkerContext::new(self.emit_bus.clone(), self.deadlines().start);
        let (tx, worker) = if let Some(owned) =
            runtime.spawn_worker(&plugin.manifest, &plugin.directory, &worker_ctx)
        {
            match owned {
                Ok(owned) => owned.into_parts(),
                Err(error) => {
                    tracing::warn!(
                        id = %id,
                        runtime = %plugin.manifest.runtime,
                        elapsed_ms = start_started.elapsed().as_millis() as u64,
                        %error,
                        "plugin failed to start"
                    );
                    return Err(error);
                }
            }
        } else {
            let instance = match runtime.load(&plugin.manifest, &plugin.directory) {
                Ok(instance) => instance,
                Err(error) => {
                    tracing::warn!(
                        id = %id,
                        runtime = %plugin.manifest.runtime,
                        elapsed_ms = start_started.elapsed().as_millis() as u64,
                        %error,
                        "plugin failed to start"
                    );
                    return Err(error);
                }
            };
            let (tx, rx) = mpsc::unbounded_channel();
            let worker_id = id.to_owned();
            let worker = thread::Builder::new()
                .name("tiktools-plugin".to_owned())
                .spawn(move || run_instance_worker(worker_id, instance, rx))
                .map_err(|error| {
                    PluginLoaderError::Runtime(format!("could not start plugin worker: {error}"))
                })?;
            (tx, worker)
        };
        let artifact_fingerprint = artifact_fingerprint.filter(|before| {
            crate::plugin_artifact_fingerprint(&plugin).ok().as_ref() == Some(before)
        });
        // The worker thread is already spawned here, so a poisoned map
        // recovers instead of failing midway through the mutation.
        recover_rwlock_write(&self.instances, "plugin instances").insert(
            id.to_owned(),
            Arc::new(RunningInstance {
                token,
                artifact_fingerprint,
                kind: plugin.manifest.runtime,
                cold: AtomicBool::new(true),
                tx,
                admission: Arc::new(tokio::sync::Semaphore::new(
                    crate::worker::PLUGIN_CALL_CAPACITY,
                )),
                worker: Mutex::new(Some(worker)),
            }),
        );
        self.set_running(id, true);
        tracing::info!(
            id = %id,
            elapsed_ms = start_started.elapsed().as_millis() as u64,
            "plugin started"
        );
        Ok(token)
    }

    pub fn stop(&self, id: &str) -> Result<(), PluginLoaderError> {
        let (instance, stopping) = {
            let _lifecycle = self
                .lifecycle
                .lock()
                .map_err(|_| lock_poisoned("plugin lifecycle"))?;
            let stopping = self.note_stopping(id);
            let removed = self.remove_instance(id);
            if let Some(instance) = removed.as_ref() {
                // Close admission and ask the worker to stop while the
                // lifecycle lock is still held: a start of the same id
                // cannot slip in between the removal and the shutdown
                // request, so two generations never coexist for one id.
                instance.admission.close();
                let _ = instance.tx.send(WorkerMsg::Shutdown);
            }
            // The flag flips under the same lock hold that removed the
            // instance, so an old stop can never clear a newer start's
            // running state.
            self.set_running(id, false);
            (removed, stopping)
        };
        let result = self.finish_stop(id, instance);
        drop(stopping);
        result
    }

    /// Best-effort stop used only by shutdown paths. Unlike [`Self::stop`],
    /// a poisoned lifecycle lock is recovered (clearing the flag) instead
    /// of failing typed, so one torn transition can never wedge shutdown
    /// forever. Worker failures are logged and the instance is always
    /// removed with its running state cleared.
    fn stop_for_shutdown(&self, id: &str) -> bool {
        let (instance, stopping) = {
            let _lifecycle = recover_mutex(&self.lifecycle, "plugin lifecycle");
            let stopping = self.note_stopping(id);
            let removed = self.remove_instance(id);
            if let Some(instance) = removed.as_ref() {
                instance.admission.close();
                let _ = instance.tx.send(WorkerMsg::Shutdown);
            }
            self.set_running(id, false);
            (removed, stopping)
        };
        let result = match self.finish_stop(id, instance) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(id = %id, %error, "plugin shutdown failed");
                false
            }
        };
        drop(stopping);
        result
    }

    /// Joins an already-stopped instance within the stop deadline. The
    /// caller has already removed it from the map and asked it to stop
    /// under the lifecycle lock, so a poisoned worker lock recovers instead
    /// of failing the stop. Shared by the strict [`Self::stop`] and the
    /// best-effort shutdown path.
    fn finish_stop(
        &self,
        id: &str,
        instance: Option<Arc<RunningInstance>>,
    ) -> Result<(), PluginLoaderError> {
        let Some(instance) = instance else {
            tracing::debug!(id = %id, "plugin stop requested while not running");
            return Ok(());
        };
        // Stop entry point: a join that never returns (a guest stuck in
        // onUnload, native teardown hanging) shows here as `stopping`
        // with no matching `stopped`.
        let stop_started = std::time::Instant::now();
        tracing::info!(id = %id, "plugin stopping");
        // The instance is already removed from the map here, so a
        // poisoned worker lock recovers instead of failing the stop.
        let worker = recover_mutex(&instance.worker, "plugin worker").take();
        let result = match worker {
            Some(worker) => self.join_worker(id, worker, self.deadlines().stop),
            None => Ok(()),
        };
        match &result {
            Ok(()) => tracing::info!(
                id = %id,
                elapsed_ms = stop_started.elapsed().as_millis() as u64,
                "plugin stopped"
            ),
            Err(error) => tracing::warn!(
                id = %id,
                elapsed_ms = stop_started.elapsed().as_millis() as u64,
                %error,
                "plugin stop failed"
            ),
        }
        self.reap_retired();
        result
    }

    /// Joins one worker within `deadline`. Polling `is_finished` is the
    /// only way to bound a std join without handing the handle to another
    /// thread; on expiry the handle is dropped (the thread detaches) and
    /// the abandonment is reported explicitly, mirroring the LIVE ingress
    /// ladder: bounded, with an "outcome unknown" report instead of a hang.
    fn join_worker(
        &self,
        id: &str,
        worker: thread::JoinHandle<Result<(), PluginLoaderError>>,
        deadline: Duration,
    ) -> Result<(), PluginLoaderError> {
        let started = Instant::now();
        while !worker.is_finished() {
            if started.elapsed() >= deadline {
                self.abandoned_workers.fetch_add(1, Ordering::Relaxed);
                tracing::error!(
                    id = %id,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    deadline_ms = deadline.as_millis() as u64,
                    "plugin worker missed the stop deadline; abandoning the thread (outcome unknown)"
                );
                return Err(PluginLoaderError::Runtime(format!(
                    "plugin `{id}` worker did not stop within {}ms; thread abandoned with unknown outcome",
                    deadline.as_millis()
                )));
            }
            thread::sleep(JOIN_POLL_INTERVAL);
        }
        worker
            .join()
            .map_err(|_| PluginLoaderError::Runtime(format!("plugin `{id}` worker panicked")))
            .and_then(|inner| inner)
    }

    pub fn stop_all(&self) {
        let _ = self.stop_all_checked();
    }

    /// Shut down all plugins and report whether every worker actually stopped.
    /// Used by the application updater to avoid replacing a running install.
    pub fn stop_all_checked(&self) -> bool {
        let ids: Vec<String> = recover_rwlock_read(&self.instances, "plugin instances")
            .keys()
            .cloned()
            .collect();
        let mut stopped = true;
        for id in ids {
            stopped &= self.stop_for_shutdown(&id);
        }
        // Shutdown is the last structured chance to join retired workers
        // before the manager itself is dropped.
        self.reap_retired();
        stopped && self.retired_worker_count() == 0 && self.abandoned_worker_count() == 0
    }

    pub async fn call(&self, id: &str, request: &Value) -> Result<Value, PluginLoaderError> {
        self.call_with_timeout(id, request, Duration::from_secs(30))
            .await
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
        let cold_process = recover_rwlock_read(&self.instances, "plugin instances")
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
    pub async fn call_with_timeout(
        &self,
        id: &str,
        request: &Value,
        timeout: Duration,
    ) -> Result<Value, PluginLoaderError> {
        self.call_with_deadline(id, request, Instant::now() + timeout)
            .await
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
    pub async fn call_with_deadline(
        &self,
        id: &str,
        request: &Value,
        deadline: Instant,
    ) -> Result<Value, PluginLoaderError> {
        self.call_generation_deadline(id, request, deadline, None)
            .await
    }

    pub fn running_generation(&self, id: &str) -> Option<u64> {
        recover_rwlock_read(&self.instances, "plugin instances")
            .get(id)
            .map(|instance| instance.token)
    }

    /// Exact package identity captured before this runtime generation loaded.
    /// Missing identity keeps legacy Owner execution available, but cannot be
    /// used for delegated approval. A disk hash alone cannot identify code
    /// already loaded by a native worker.
    pub fn running_artifact_fingerprint(&self, id: &str) -> Option<String> {
        recover_rwlock_read(&self.instances, "plugin instances")
            .get(id)
            .and_then(|instance| instance.artifact_fingerprint.clone())
    }

    /// Prepare under the host's trusted catalog/settings lease. This reads
    /// package data and freezes it, but never runs initialization or plugin
    /// code. The resulting linear value still requires host final admission.
    pub fn prepare_isolated_call(
        &self,
        id: &str,
        artifact_digest: &str,
        snapshot_directory: std::path::PathBuf,
        settings: Value,
    ) -> Result<crate::PreparedIsolatedPlugin, PluginLoaderError> {
        let plugin = self
            .get(id)
            .ok_or_else(|| PluginLoaderError::NotFound(id.into()))?;
        if !plugin.available {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin is unavailable".into(),
            ));
        }
        if !plugin.manifest.native_addons.is_empty()
            && !native_addons_allowed(&plugin.manifest, plugin.source)
        {
            return Err(untrusted_native_addons(id));
        }
        let runtime = self.runtimes.get(plugin.manifest.runtime).ok_or_else(|| {
            PluginLoaderError::RuntimeUnavailable("plugin runtime is unavailable".into())
        })?;
        crate::PreparedIsolatedPlugin::new(
            plugin,
            runtime,
            snapshot_directory,
            artifact_digest,
            settings,
            Arc::clone(&self.isolated_admission),
        )
    }

    /// Orders trusted final admission against every scan/start/stop/retirement.
    /// The callback must be synchronous and must not reenter lifecycle methods.
    pub fn with_catalog_lease<R>(
        &self,
        apply: impl FnOnce(&Self) -> R,
    ) -> Result<R, PluginLoaderError> {
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| lock_poisoned("plugin lifecycle"))?;
        Ok(apply(self))
    }

    pub async fn call_with_generation(
        &self,
        id: &str,
        request: &Value,
        timeout: Duration,
        generation: u64,
    ) -> Result<Value, PluginLoaderError> {
        self.call_generation_deadline(id, request, Instant::now() + timeout, Some(generation))
            .await
    }

    async fn call_generation_deadline(
        &self,
        id: &str,
        request: &Value,
        deadline: Instant,
        generation: Option<u64>,
    ) -> Result<Value, PluginLoaderError> {
        let instance = self
            .instances
            .read()
            .map_err(|_| lock_poisoned("plugin instances"))?
            .get(id)
            .cloned()
            .ok_or_else(|| {
                if generation.is_some() {
                    PluginLoaderError::RuntimeUnavailable("plugin generation changed".into())
                } else {
                    PluginLoaderError::NotFound(id.to_owned())
                }
            })?;
        if generation.is_some_and(|generation| generation != instance.token) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin generation changed".into(),
            ));
        }
        let admission = Arc::clone(&instance.admission)
            .try_acquire_owned()
            .map_err(|_| PluginLoaderError::Overloaded(id.to_owned()))?;
        let bytes = serde_json::to_vec(request)
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
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
        let (respond, answer) = oneshot::channel();
        instance
            .tx
            .send(WorkerMsg::Call(QueuedCall {
                admission: Some(admission),
                request: bytes,
                timeout: remaining,
                deadline,
                respond,
            }))
            .map_err(|_| {
                self.remove_failed_worker(id, instance.token);
                PluginLoaderError::Runtime(format!("plugin `{id}` worker is gone"))
            })?;
        // Async wait: no thread is spent here, unlike the old blocking
        // `recv_timeout` inside `spawn_blocking`. Timeout and worker-death
        // semantics are unchanged: a dropped oneshot sender means the
        // worker died without answering, exactly like a disconnected
        // channel before.
        let response = tokio::time::timeout(remaining, answer)
            .await
            .map_err(|_| {
                if is_process {
                    self.remove_failed_worker(id, instance.token);
                }
                PluginLoaderError::Timeout(format!("plugin `{id}` call timed out"))
            })?
            .map_err(|_| {
                self.remove_failed_worker(id, instance.token);
                PluginLoaderError::Runtime(format!("plugin `{id}` worker died"))
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
        // A stopped/replaced worker may still complete an in-flight call.
        // Do not surface its result to a generation-bound caller. This is an
        // instantaneous check, not a lease covering later host side effects.
        if let Some(expected) = generation {
            let instances = self
                .instances
                .read()
                .map_err(|_| lock_poisoned("plugin instances"))?;
            if !instances
                .get(id)
                .is_some_and(|current| current.token == expected)
            {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "plugin generation changed".into(),
                ));
            }
        }
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

    /// Aggregate admission occupancy, without request payloads or plugin IDs.
    pub fn outstanding_call_count(&self) -> usize {
        recover_rwlock_read(&self.instances, "plugin instances")
            .values()
            .map(|instance| {
                crate::worker::PLUGIN_CALL_CAPACITY
                    .saturating_sub(instance.admission.available_permits())
            })
            .sum()
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
        if let Some(plugin) = recover_rwlock_write(&self.registry, "plugin registry")
            .entries
            .get_mut(id)
        {
            plugin.running = running;
        }
        self.bump_registry_revision();
    }

    fn remove_instance(&self, id: &str) -> Option<Arc<RunningInstance>> {
        recover_rwlock_write(&self.instances, "plugin instances").remove(id)
    }

    /// Removes the instance generation `token` (only if it is still the
    /// current one), closes its admission and asks it to stop. The caller
    /// must already hold the lifecycle lock, so a concurrent start of the
    /// same id can never be mislabelled or left running alongside the
    /// generation being retired.
    fn retire_generation_locked(&self, id: &str, token: u64) -> Option<Arc<RunningInstance>> {
        let removed = {
            let mut instances = recover_rwlock_write(&self.instances, "plugin instances");
            if instances
                .get(id)
                .is_some_and(|current| current.token == token)
            {
                instances.remove(id)
            } else {
                None
            }
        };
        if let Some(instance) = removed.as_ref() {
            instance.admission.close();
            let _ = instance.tx.send(WorkerMsg::Shutdown);
        }
        if removed.is_some() {
            self.set_running(id, false);
        }
        removed
    }

    /// Retires the instance generation identified by `token`; a newer start
    /// is never disturbed. The hot path that observed the failure cannot
    /// block on a join, so the worker's handle keeps its owner here and is
    /// reaped (bounded) later instead of being dropped unobserved.
    fn remove_failed_worker(&self, id: &str, token: u64) {
        let (removed, stopping) = {
            let _lifecycle = recover_mutex(&self.lifecycle, "plugin lifecycle");
            let stopping = self.note_stopping(id);
            (self.retire_generation_locked(id, token), stopping)
        };
        if let Some(instance) = removed {
            self.retire_instance(id, token, instance);
        }
        drop(stopping);
    }

    /// Hands a retired worker's join handle to the manager for bounded
    /// reaping. Never blocks: the caller may be serving a call. The list is
    /// bounded, and overflow abandons the oldest threads explicitly with
    /// the same report as a missed stop deadline.
    fn retire_instance(&self, id: &str, token: u64, instance: Arc<RunningInstance>) {
        let worker = recover_mutex(&instance.worker, "plugin worker").take();
        let mut retired = recover_mutex(&self.retired, "plugin retired workers");
        while retired.len() >= MAX_RETIRED_WORKERS {
            let oldest = retired.remove(0);
            self.abandoned_workers.fetch_add(1, Ordering::Relaxed);
            tracing::error!(
                id = %oldest.id,
                token = oldest.token,
                "retired plugin worker backlog is full; abandoning the oldest thread (outcome unknown)"
            );
            drop(oldest.worker);
        }
        retired.push(RetiredWorker {
            id: id.to_owned(),
            token,
            worker,
        });
    }

    /// Joins every retired worker that has already finished and logs its
    /// outcome; still-running ones stay for a later pass. Bounded: it never
    /// waits on a live thread, so it is safe on the hot path.
    fn reap_retired(&self) {
        let finished: Vec<RetiredWorker> = {
            let mut retired = recover_mutex(&self.retired, "plugin retired workers");
            let mut finished = Vec::new();
            let mut kept = Vec::new();
            for entry in retired.drain(..) {
                match &entry.worker {
                    Some(worker) if !worker.is_finished() => kept.push(entry),
                    _ => finished.push(entry),
                }
            }
            *retired = kept;
            finished
        };
        for entry in finished {
            let Some(worker) = entry.worker else { continue };
            match worker.join() {
                Ok(Ok(())) => tracing::info!(
                    id = %entry.id,
                    token = entry.token,
                    "retired plugin worker stopped"
                ),
                Ok(Err(error)) => tracing::warn!(
                    id = %entry.id,
                    token = entry.token,
                    %error,
                    "retired plugin worker exited with an error"
                ),
                Err(_) => tracing::warn!(
                    id = %entry.id,
                    token = entry.token,
                    "retired plugin worker panicked"
                ),
            }
        }
    }

    /// Workers still awaiting an opportunistic reap (diagnostics).
    pub fn retired_worker_count(&self) -> usize {
        recover_mutex(&self.retired, "plugin retired workers")
            .iter()
            .filter(|entry| entry.worker.is_some())
            .count()
    }

    /// Workers explicitly abandoned past a deadline, outcome unknown
    /// (diagnostics). Zero in the healthy case.
    pub fn abandoned_worker_count(&self) -> u64 {
        self.abandoned_workers.load(Ordering::Relaxed)
    }
}

impl Drop for PluginManager {
    /// Last-chance reap: every retained join handle is either joined (its
    /// outcome logged) or already accounted as abandoned. Without this a
    /// manager dropped mid-flight would detach its retired threads
    /// unobserved.
    fn drop(&mut self) {
        self.reap_retired();
    }
}

#[cfg(test)]
impl PluginManager {
    /// Test-only: replace the stop join bound, so tests can prove the
    /// deadline without waiting the production five seconds. Production
    /// code never calls this.
    pub(crate) fn set_stop_deadline_for_test(&self, deadline: Duration) {
        recover_mutex(&self.deadlines, "plugin deadlines").stop = deadline;
    }

    /// Test-only: replace the startup handshake bound (see
    /// [`Self::set_stop_deadline_for_test`]).
    pub(crate) fn set_start_deadline_for_test(&self, deadline: Duration) {
        recover_mutex(&self.deadlines, "plugin deadlines").start = deadline;
    }

    /// Test-only: poison one running instance's worker lock so tests can
    /// exercise the stop-path recovery. Test-only; production code never calls this.
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

    /// Test-only: name of the thread serving `id`, so tests can prove which
    /// worker implementation a runtime resolved to (owned vs generic).
    pub(crate) fn worker_thread_name_for_test(&self, id: &str) -> Option<String> {
        let instances = self.instances.read().ok()?;
        let instance = instances.get(id)?;
        let worker = instance.worker.lock().ok()?;
        worker.as_ref()?.thread().name().map(str::to_owned)
    }

    /// Test-only: whether the mapped worker for `id` has exited (or its
    /// handle was taken). Lets a test wait out a dead-but-mapped generation
    /// deterministically instead of guessing a thread schedule.
    pub(crate) fn worker_finished_for_test(&self, id: &str) -> bool {
        recover_rwlock_read(&self.instances, "plugin instances")
            .get(id)
            .map(|instance| instance.worker_finished())
            .unwrap_or(true)
    }
}
