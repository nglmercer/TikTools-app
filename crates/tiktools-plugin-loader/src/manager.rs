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
    PluginManifest, PluginRuntimeKind,
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

/// Bound the number of unresolved startup threads a manager can retain.
const MAX_UNCERTAIN_STARTUPS: usize = 16;

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

struct StartingGuard<'a>(&'a AtomicU64);

impl Drop for StartingGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
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
    /// Only an uninterrupted, complete scan can support migration preview.
    complete_for_cutover: bool,
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
    /// Sticky per-manager unknown outcomes, including the owned thread handles.
    /// A completed thread alone cannot prove what its external effects were.
    uncertain_startups:
        Arc<Mutex<BTreeMap<String, Vec<thread::JoinHandle<Result<(), PluginLoaderError>>>>>>,
    starts_inflight: AtomicU64,
    /// Once checked shutdown begins, no later start can be admitted.
    shutdown_admission_closed: AtomicBool,
    package_fences: Mutex<BTreeMap<String, u64>>,
    /// Count event publications that have crossed admission for each plugin.
    /// Package fence admission is serialized with increments under
    /// `package_fences`; the count may remain held across asynchronous work.
    event_deliveries: Arc<Mutex<BTreeMap<String, usize>>>,
    next_package_token: AtomicU64,
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

/// Exclusive admission for one package transaction. Dropping without a
/// successful finish deliberately leaves the package fenced for this manager.
pub struct PluginPackageLease {
    manager: Arc<PluginManager>,
    id: String,
    token: u64,
    failed: AtomicBool,
    finished: AtomicBool,
    started: Mutex<Option<u64>>,
}

/// A host event admitted for a specific live plugin generation. Holding this
/// permit prevents a package transaction from beginning until delivery has
/// completed or the future is dropped. It does not fence non-package stops or
/// effects already dispatched by the event.
pub struct PluginEventDeliveryPermit {
    deliveries: Arc<Mutex<BTreeMap<String, usize>>>,
    id: String,
    manifest: Arc<PluginManifest>,
}

impl PluginEventDeliveryPermit {
    /// The immutable manifest with which this exact worker was started.
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
}

impl Drop for PluginEventDeliveryPermit {
    fn drop(&mut self) {
        let mut deliveries = recover_mutex(&self.deliveries, "plugin event deliveries");
        if let Some(count) = deliveries.get_mut(&self.id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                deliveries.remove(&self.id);
            }
        }
    }
}

impl PluginPackageLease {
    pub fn stop_if_present(&self) -> Result<bool, PluginLoaderError> {
        let result = self.manager.stop_with_package(&self.id, Some(self.token));
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }

    pub fn start(&self) -> Result<(), PluginLoaderError> {
        let result = self
            .manager
            .start_generation_with_package(&self.id, || {}, Some(self.token))
            .and_then(|generation| {
                *self
                    .started
                    .lock()
                    .map_err(|_| lock_poisoned("plugin package lease generation"))? =
                    Some(generation);
                Ok(())
            });
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }

    pub fn finish(self) -> Result<(), PluginLoaderError> {
        if self.failed.load(Ordering::Acquire) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin package lease requires recovery".into(),
            ));
        }
        let _lifecycle = self.manager.lifecycle.try_lock().map_err(|_| {
            PluginLoaderError::RuntimeUnavailable("plugin lifecycle is busy or unavailable".into())
        })?;
        self.manager.reap_retired();
        self.manager.ensure_startup_known(&self.id)?;
        self.manager.ensure_no_retiring_worker(&self.id)?;
        let mut fences = self
            .manager
            .package_fences
            .lock()
            .map_err(|_| lock_poisoned("plugin package fences"))?;
        if fences.get(&self.id) != Some(&self.token) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin package lease is invalid".into(),
            ));
        }
        let started = *self
            .started
            .lock()
            .map_err(|_| lock_poisoned("plugin package lease generation"))?;
        if let Some(generation) = started {
            let instances = self
                .manager
                .instances
                .read()
                .map_err(|_| lock_poisoned("plugin instances"))?;
            let instance = instances
                .get(&self.id)
                .filter(|instance| instance.token == generation)
                .ok_or_else(|| {
                    PluginLoaderError::RuntimeUnavailable(
                        "package worker generation changed".into(),
                    )
                })?;
            if !instance
                .worker
                .lock()
                .map_err(|_| lock_poisoned("plugin worker"))?
                .as_ref()
                .is_some_and(|worker| !worker.is_finished())
            {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "package worker exited before completion".into(),
                ));
            }
            instance
                .events
                .activate_with(|| {
                    fences.remove(&self.id);
                })
                .map_err(PluginLoaderError::RuntimeUnavailable)?;
        } else {
            fences.remove(&self.id);
        }
        self.finished.store(true, Ordering::Release);
        Ok(())
    }
}

impl Drop for PluginPackageLease {
    fn drop(&mut self) {
        if self.finished.load(Ordering::Acquire) {
            return;
        }
        if let Ok(started) = self.started.lock() {
            if let Some(generation) = *started {
                if let Ok(instances) = self.manager.instances.read() {
                    if let Some(instance) = instances
                        .get(&self.id)
                        .filter(|instance| instance.token == generation)
                    {
                        instance.events.close();
                        instance.admission.close();
                        let _ = instance.tx.send(WorkerMsg::Shutdown);
                    }
                }
            }
        }
    }
}

/// Push-bus buffer: burst headroom for guest-emitted events while core
/// drains. Bounded: a lagging subscriber drops with a `Lagged` error
/// instead of growing memory, and emits stay small by construction.
const EMIT_BUS_CAPACITY: usize = 256;

impl PluginManager {
    // Called with lifecycle held. A failure retirement may leave the instance
    // map before handing its worker to the retired list; `stopping` bridges it.
    fn ensure_no_retiring_worker(&self, id: &str) -> Result<(), PluginLoaderError> {
        if self
            .stopping
            .lock()
            .map_err(|_| lock_poisoned("plugin stopping"))?
            .contains_key(id)
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin worker is stopping".into(),
            ));
        }
        let retired = self
            .retired
            .lock()
            .map_err(|_| lock_poisoned("plugin retired workers"))?;
        if self.abandoned_workers.load(Ordering::Acquire) != 0
            || retired.iter().any(|worker| worker.id == id)
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin worker outcome is uncertain".into(),
            ));
        }
        Ok(())
    }

    /// Fence all ordinary starts while a package transaction is in progress.
    /// A failed or interrupted transaction keeps the fence until restart.
    pub fn begin_package_mutation(
        self: &Arc<Self>,
        id: &str,
    ) -> Result<PluginPackageLease, PluginLoaderError> {
        let _lifecycle = self.lifecycle.try_lock().map_err(|_| {
            PluginLoaderError::RuntimeUnavailable("plugin lifecycle is busy or unavailable".into())
        })?;
        if self.shutdown_admission_closed.load(Ordering::Acquire) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin manager is shutting down".into(),
            ));
        }
        self.ensure_startup_known(id)?;
        self.reap_retired();
        self.ensure_no_retiring_worker(id)?;
        let mut fences = self
            .package_fences
            .lock()
            .map_err(|_| lock_poisoned("plugin package fences"))?;
        if fences.contains_key(id) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin package mutation is already active or requires recovery".into(),
            ));
        }
        if self
            .event_deliveries
            .lock()
            .map_err(|_| lock_poisoned("plugin event deliveries"))?
            .contains_key(id)
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin event publication is still in progress".into(),
            ));
        }
        let token = self
            .next_package_token
            .try_update(Ordering::AcqRel, Ordering::Acquire, |token| {
                token.checked_add(1)
            })
            .map_err(|_| {
                PluginLoaderError::RuntimeUnavailable("plugin token space exhausted".into())
            })?;
        fences.insert(id.to_owned(), token);
        Ok(PluginPackageLease {
            manager: Arc::clone(self),
            id: id.to_owned(),
            token,
            failed: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            started: Mutex::new(None),
        })
    }

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
            uncertain_startups: Arc::new(Mutex::new(BTreeMap::new())),
            starts_inflight: AtomicU64::new(0),
            shutdown_admission_closed: AtomicBool::new(false),
            package_fences: Mutex::new(BTreeMap::new()),
            event_deliveries: Arc::new(Mutex::new(BTreeMap::new())),
            next_package_token: AtomicU64::new(1),
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
        self.reap_uncertain_startups();
        {
            let mut registry = self
                .registry
                .write()
                .map_err(|_| lock_poisoned("plugin registry"))?;
            registry.complete_for_cutover = false;
            self.bump_registry_revision();
        }
        let mut discovered = BTreeMap::new();
        let mut complete = true;
        for root in &self.roots {
            match fs::metadata(&root.path) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // An absent configured root cannot prove an empty local
                    // inventory, even in a freshly started host.
                    complete = false;
                    continue;
                }
                Err(error) => {
                    complete = false;
                    tracing::warn!(path = %root.path.display(), %error, "could not inspect plugin root");
                    continue;
                }
            }
            let entries = fs::read_dir(&root.path).map_err(|error| {
                PluginLoaderError::InvalidDirectory(format!("{}: {error}", root.path.display()))
            })?;
            for (index, entry) in entries.enumerate() {
                if index >= MAX_DIRECTORY_ENTRIES {
                    complete = false;
                    tracing::warn!(path = %root.path.display(), "plugin directory entry limit reached");
                    break;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        complete = false;
                        tracing::warn!(path = %root.path.display(), %error, "could not inspect plugin entry");
                        continue;
                    }
                };
                let directory = entry.path();
                let hidden = entry.file_name().to_string_lossy().starts_with('.');
                if hidden {
                    continue;
                }
                let kind = match entry.file_type() {
                    Ok(kind) => kind,
                    Err(error) => {
                        complete = false;
                        tracing::warn!(path = %directory.display(), %error, "could not inspect plugin entry type");
                        continue;
                    }
                };
                if kind.is_symlink() {
                    complete = false;
                }
                if !kind.is_dir() {
                    continue;
                }
                match read_discovered_plugin(&directory, root.source) {
                    Ok(plugin) => {
                        if let Some(previous) =
                            discovered.insert(plugin.manifest.id.clone(), plugin)
                        {
                            complete = false;
                            tracing::debug!(
                                id = %previous.manifest.id,
                                "plugin overridden by a later runtime root"
                            );
                        }
                    }
                    Err(error) => {
                        complete = false;
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
        registry.complete_for_cutover = complete;
        self.bump_registry_revision();
        drop(registry);
        Ok(result)
    }

    pub fn list(&self) -> Vec<DiscoveredPlugin> {
        recover_rwlock_read(&self.registry, "plugin registry")
            .entries
            .values()
            .cloned()
            .collect()
    }

    /// Inventory for an informational migration preview. It is not a proof
    /// that files have stayed unchanged since the scan, nor permission to
    /// apply or migrate them. A failed/partial scan invalidates this evidence
    /// while ordinary plugin listing and local operation remain available.
    /// Every configured root, including optional development roots, must exist;
    /// absence cannot distinguish an empty inventory from a missing store.
    pub fn list_complete_for_cutover(
        &self,
    ) -> Result<(u64, Vec<DiscoveredPlugin>), PluginLoaderError> {
        let registry = self
            .registry
            .read()
            .map_err(|_| lock_poisoned("plugin registry"))?;
        if !registry.complete_for_cutover {
            return Err(PluginLoaderError::InvalidDirectory(
                "plugin inventory is incomplete".into(),
            ));
        }
        Ok((
            self.registry_revision(),
            registry.entries.values().cloned().collect(),
        ))
    }

    /// Hold the current complete catalog through a synchronous native
    /// callback. The callback must not reenter the manager or do network I/O.
    /// This protects only the in-process registry, not source files or other
    /// processes. Slow filesystem work must run on a blocking thread.
    pub fn with_complete_cutover_catalog<R>(
        &self,
        operation: impl FnOnce(u64, Vec<DiscoveredPlugin>) -> R,
    ) -> Result<R, PluginLoaderError> {
        let registry = self
            .registry
            .read()
            .map_err(|_| lock_poisoned("plugin registry"))?;
        if !registry.complete_for_cutover {
            return Err(PluginLoaderError::InvalidDirectory(
                "plugin inventory is incomplete".into(),
            ));
        }
        let result = operation(
            self.registry_revision(),
            registry.entries.values().cloned().collect(),
        );
        drop(registry);
        Ok(result)
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

    /// Reject activation or package changes while a prior startup outcome is
    /// unknown in this manager, including when there is no mapped worker.
    pub fn ensure_startup_known(&self, id: &str) -> Result<(), PluginLoaderError> {
        let uncertain = self
            .uncertain_startups
            .lock()
            .map_err(|_| lock_poisoned("plugin uncertain startups"))?;
        if uncertain.contains_key(id) {
            return Err(PluginLoaderError::StartupUncertain(format!(
                "plugin `{id}` requires startup recovery"
            )));
        }
        Ok(())
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
        if self
            .package_fences
            .lock()
            .map_err(|_| lock_poisoned("plugin package fences"))?
            .contains_key(id)
        {
            return Ok(PluginWorkerObservation::Uncertain);
        }
        if self
            .uncertain_startups
            .lock()
            .map_err(|_| lock_poisoned("plugin uncertain startups"))?
            .contains_key(id)
        {
            return Ok(PluginWorkerObservation::Uncertain);
        }
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
        self.start_generation_before_events(id, || {})
    }

    /// Runs a short host activation callback after a successful start and
    /// before startup events are released. It also runs when an existing
    /// worker is reused. The callback must not reenter the manager or wait.
    /// Host invocation admission must be acquired before calling this method.
    pub fn start_generation_before_events(
        &self,
        id: &str,
        before_events: impl FnOnce(),
    ) -> Result<u64, PluginLoaderError> {
        self.start_generation_with_package(id, before_events, None)
    }

    fn start_generation_with_package(
        &self,
        id: &str,
        before_events: impl FnOnce(),
        package_token: Option<u64>,
    ) -> Result<u64, PluginLoaderError> {
        if self.shutdown_admission_closed.load(Ordering::Acquire) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin manager is shutting down".into(),
            ));
        }
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| lock_poisoned("plugin lifecycle"))?;
        if self.shutdown_admission_closed.load(Ordering::Acquire) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin manager is shutting down".into(),
            ));
        }
        {
            let fences = self
                .package_fences
                .lock()
                .map_err(|_| lock_poisoned("plugin package fences"))?;
            if fences.get(id).copied() != package_token {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "plugin package mutation is active or requires recovery".into(),
                ));
            }
        }
        if package_token.is_some() {
            self.ensure_no_retiring_worker(id)?;
        }
        self.reap_uncertain_startups();
        {
            let uncertain = self
                .uncertain_startups
                .lock()
                .map_err(|_| lock_poisoned("plugin uncertain startups"))?;
            if uncertain.contains_key(id) {
                return Err(PluginLoaderError::StartupUncertain(format!(
                    "plugin `{id}` requires startup recovery"
                )));
            }
        }
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
                before_events();
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
        if self
            .uncertain_startups
            .lock()
            .map_err(|_| lock_poisoned("plugin uncertain startups"))?
            .len()
            >= MAX_UNCERTAIN_STARTUPS
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "too many unresolved plugin startups".into(),
            ));
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
        self.starts_inflight.fetch_add(1, Ordering::AcqRel);
        let _starting = StartingGuard(&self.starts_inflight);
        let artifact_fingerprint = crate::plugin_artifact_fingerprint(&plugin).ok();
        // Runtimes with an owned worker (napi-vm) serve the call queue on
        // their own thread, bypassing the generic worker: one thread per
        // plugin instead of two, with identical queue/deadline semantics.
        let uncertain = Arc::clone(&self.uncertain_startups);
        let uncertain_id = id.to_owned();
        let retain = Arc::new(move |worker| {
            recover_mutex(&uncertain, "plugin uncertain startups")
                .entry(uncertain_id.clone())
                .or_default()
                .push(worker);
        });
        let mut worker_ctx = WorkerContext::new(
            self.emit_bus.clone(),
            self.deadlines().start,
            id.to_owned(),
            token,
            Some(retain),
        );
        let (tx, worker) = if let Some(owned) =
            runtime.spawn_worker(&plugin.manifest, &plugin.directory, &worker_ctx)
        {
            match owned {
                Ok(owned) => owned.into_parts(),
                Err(error) => {
                    if matches!(error, PluginLoaderError::StartupUncertain(_)) {
                        recover_mutex(&self.uncertain_startups, "plugin uncertain startups")
                            .entry(id.to_owned())
                            .or_default();
                    }
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
                    if matches!(error, PluginLoaderError::StartupUncertain(_)) {
                        recover_mutex(&self.uncertain_startups, "plugin uncertain startups")
                            .entry(id.to_owned())
                            .or_default();
                    }
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
        let instance = Arc::new(RunningInstance {
            token,
            manifest: Arc::new(plugin.manifest.clone()),
            events: worker_ctx.emit_sender(),
            artifact_fingerprint,
            kind: plugin.manifest.runtime,
            cold: AtomicBool::new(true),
            tx,
            admission: Arc::new(tokio::sync::Semaphore::new(
                crate::worker::PLUGIN_CALL_CAPACITY,
            )),
            worker: Mutex::new(Some(worker)),
        });
        if self.shutdown_admission_closed.load(Ordering::Acquire) {
            instance.events.close();
            instance.admission.close();
            let _ = instance.tx.send(WorkerMsg::Shutdown);
            self.retire_instance(id, token, instance);
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin manager is shutting down".into(),
            ));
        }
        recover_rwlock_write(&self.instances, "plugin instances").insert(id.to_owned(), instance);
        self.set_running(id, true);
        if package_token.is_none() {
            if let Err(error) = worker_ctx.activate_events(before_events) {
                if let Some(instance) = self.retire_generation_locked(id, token) {
                    self.retire_instance(id, token, instance);
                }
                return Err(PluginLoaderError::RuntimeUnavailable(error));
            }
        } else {
            worker_ctx.defer_events_to_package();
        }
        tracing::info!(
            id = %id,
            elapsed_ms = start_started.elapsed().as_millis() as u64,
            "plugin started"
        );
        Ok(token)
    }

    pub fn stop(&self, id: &str) -> Result<(), PluginLoaderError> {
        self.stop_if_present(id).map(|_| ())
    }

    /// Atomically determine whether a worker was running and stop it. An
    /// unresolved startup must never be reported as a successful absence.
    pub fn stop_if_present(&self, id: &str) -> Result<bool, PluginLoaderError> {
        self.stop_with_package(id, None)
    }

    fn stop_with_package(
        &self,
        id: &str,
        package_token: Option<u64>,
    ) -> Result<bool, PluginLoaderError> {
        let (instance, stopping) = {
            let _lifecycle = self
                .lifecycle
                .lock()
                .map_err(|_| lock_poisoned("plugin lifecycle"))?;
            if self
                .package_fences
                .lock()
                .map_err(|_| lock_poisoned("plugin package fences"))?
                .get(id)
                .copied()
                != package_token
            {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "plugin package mutation is active or requires recovery".into(),
                ));
            }
            if package_token.is_some() {
                self.ensure_no_retiring_worker(id)?;
            }
            if self
                .uncertain_startups
                .lock()
                .map_err(|_| lock_poisoned("plugin uncertain startups"))?
                .contains_key(id)
            {
                return Err(PluginLoaderError::StartupUncertain(format!(
                    "plugin `{id}` requires startup recovery"
                )));
            }
            let stopping = self.note_stopping(id);
            let removed = self.remove_instance(id);
            if let Some(instance) = removed.as_ref() {
                // Close admission and ask the worker to stop while the
                // lifecycle lock is still held: a start of the same id
                // cannot slip in between the removal and the shutdown
                // request, so two generations never coexist for one id.
                instance.events.close();
                instance.admission.close();
                let _ = instance.tx.send(WorkerMsg::Shutdown);
            }
            // The flag flips under the same lock hold that removed the
            // instance, so an old stop can never clear a newer start's
            // running state.
            self.set_running(id, false);
            (removed, stopping)
        };
        let was_running = instance.is_some();
        let result = self.finish_stop(id, instance);
        drop(stopping);
        result.map(|()| was_running)
    }

    /// Best-effort stop used only by shutdown paths. Unlike [`Self::stop`],
    /// a poisoned lifecycle lock is recovered (clearing the flag) instead
    /// of failing typed, so one torn transition can never wedge shutdown
    /// forever. Worker failures are logged and the instance is always
    /// removed with its running state cleared.
    fn stop_for_shutdown(&self, id: &str) -> bool {
        let (instance, stopping) = {
            let _lifecycle = match self.lifecycle.try_lock() {
                Ok(guard) => guard,
                Err(std::sync::TryLockError::WouldBlock) => return false,
                Err(std::sync::TryLockError::Poisoned(error)) => {
                    tracing::warn!("recovering poisoned plugin lifecycle during shutdown");
                    self.lifecycle.clear_poison();
                    error.into_inner()
                }
            };
            if recover_mutex(&self.package_fences, "plugin package fences").contains_key(id) {
                return false;
            }
            if recover_mutex(&self.uncertain_startups, "plugin uncertain startups").contains_key(id)
            {
                return false;
            }
            let stopping = self.note_stopping(id);
            let removed = self.remove_instance(id);
            if let Some(instance) = removed.as_ref() {
                instance.events.close();
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
                recover_mutex(&self.uncertain_startups, "plugin uncertain startups")
                    .entry(id.to_owned())
                    .or_default()
                    .push(worker);
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
        let _ = self.stop_all_snapshot();
    }

    /// Permanently close start admission and report whether all workers and
    /// in-flight stops finished. A false result remains unsafe for updating.
    pub fn stop_all_checked(&self) -> bool {
        self.close_start_admission();
        self.stop_all_snapshot()
    }

    /// Close admission immediately at host shutdown, before awaiting other
    /// tasks. It remains closed for this manager's lifetime.
    pub fn close_start_admission(&self) {
        self.shutdown_admission_closed
            .store(true, Ordering::Release);
    }

    /// Nonterminal best-effort snapshot used by `stop_all` callers.
    fn stop_all_snapshot(&self) -> bool {
        // A start holds lifecycle while its outcome is unresolved. Do not
        // report a clean shutdown merely because its worker is not in the
        // instance map yet; avoid blocking indefinitely on native loading.
        let lifecycle = match self.lifecycle.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => return false,
            Err(std::sync::TryLockError::Poisoned(error)) => {
                tracing::warn!("recovering poisoned plugin lifecycle during shutdown");
                self.lifecycle.clear_poison();
                error.into_inner()
            }
        };
        if self.starts_inflight.load(Ordering::Acquire) != 0 {
            return false;
        }
        if self
            .package_fences
            .lock()
            .map_or(true, |fences| !fences.is_empty())
        {
            return false;
        }
        drop(lifecycle);
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
        self.reap_uncertain_startups();
        let uncertain = self
            .uncertain_startups
            .lock()
            .map_or(true, |starts| !starts.is_empty());
        let stopping = self.stopping.lock().map_or(true, |stops| !stops.is_empty());
        stopped
            && !uncertain
            && !stopping
            && self.starts_inflight.load(Ordering::Acquire) == 0
            && self.retired_worker_count() == 0
            && self.abandoned_worker_count() == 0
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
        if self
            .package_fences
            .lock()
            .map_or(true, |fences| fences.contains_key(id))
        {
            return None;
        }
        recover_rwlock_read(&self.instances, "plugin instances")
            .get(id)
            .map(|instance| instance.token)
    }

    /// Fail closed if the precise host-owned worker has exited or its map or
    /// worker lock is poisoned. This is a snapshot, not an effects lease.
    pub fn is_live_generation(&self, id: &str, generation: u64) -> bool {
        let Ok(fences) = self.package_fences.lock() else {
            return false;
        };
        if fences.contains_key(id) {
            return false;
        }
        let Ok(instances) = self.instances.read() else {
            return false;
        };
        let Some(instance) = instances
            .get(id)
            .filter(|instance| instance.token == generation)
        else {
            return false;
        };
        let Ok(worker) = instance.worker.lock() else {
            return false;
        };
        worker.as_ref().is_some_and(|handle| !handle.is_finished())
    }

    /// Atomically admit event delivery for the exact live worker and exclude
    /// package mutation for this plugin until the returned permit is dropped.
    /// A blocked delivery makes package mutation fail quickly; it never holds
    /// a mutex across an await or prevents unrelated plugins from mutating.
    pub fn try_reserve_event_delivery(
        &self,
        id: &str,
        generation: u64,
    ) -> Option<PluginEventDeliveryPermit> {
        let fences = self.package_fences.lock().ok()?;
        if fences.contains_key(id) {
            return None;
        }
        let instances = self.instances.read().ok()?;
        let instance = instances
            .get(id)
            .filter(|instance| instance.token == generation)?;
        let worker = instance.worker.lock().ok()?;
        if worker.as_ref().is_none_or(|handle| handle.is_finished()) {
            return None;
        }
        let mut deliveries = self.event_deliveries.lock().ok()?;
        let count = deliveries.get(id).copied().unwrap_or(0).checked_add(1)?;
        deliveries.insert(id.to_owned(), count);
        Some(PluginEventDeliveryPermit {
            deliveries: Arc::clone(&self.event_deliveries),
            id: id.to_owned(),
            manifest: Arc::clone(&instance.manifest),
        })
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
        if self
            .package_fences
            .lock()
            .map_err(|_| lock_poisoned("plugin package fences"))?
            .contains_key(id)
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin package mutation is active or requires recovery".into(),
            ));
        }
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
        let bytes = serde_json::to_vec(request)
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        if Instant::now() >= deadline {
            return Err(PluginLoaderError::Timeout(format!(
                "plugin `{id}` call already expired"
            )));
        }
        let (instance, is_process, remaining, answer, sent) = {
            // Admission and enqueue are synchronous and ordered against package
            // reservation; never hold this mutex across the asynchronous reply.
            let fences = self
                .package_fences
                .lock()
                .map_err(|_| lock_poisoned("plugin package fences"))?;
            if fences.contains_key(id) {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "plugin package mutation is active or requires recovery".into(),
                ));
            }
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
            let sent = instance.tx.send(WorkerMsg::Call(QueuedCall {
                admission: Some(admission),
                request: bytes,
                timeout: remaining,
                deadline,
                respond,
            }));
            (instance, is_process, remaining, answer, sent)
        };
        sent.map_err(|_| {
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
        let mut registry = recover_rwlock_write(&self.registry, "plugin registry");
        if let Some(plugin) = registry.entries.get_mut(id) {
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
            instance.events.close();
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

    /// Join completed unknown startup threads without clearing their sticky
    /// outcome: a thread exit alone cannot undo or classify external effects.
    fn reap_uncertain_startups(&self) {
        let finished = {
            let Ok(mut startups) = self.uncertain_startups.lock() else {
                return;
            };
            let mut finished = Vec::new();
            for (id, workers) in startups.iter_mut() {
                let mut pending = Vec::new();
                for worker in workers.drain(..) {
                    if worker.is_finished() {
                        finished.push((id.clone(), worker));
                    } else {
                        pending.push(worker);
                    }
                }
                *workers = pending;
            }
            finished
        };
        for (id, worker) in finished {
            match worker.join() {
                Ok(Ok(())) => {
                    tracing::info!(plugin = %id, "uncertain startup worker exited; recovery remains required")
                }
                Ok(Err(error)) => {
                    tracing::warn!(plugin = %id, %error, "uncertain startup worker exited with an error")
                }
                Err(_) => tracing::warn!(plugin = %id, "uncertain startup worker panicked"),
            }
        }
    }

    /// Workers still awaiting an opportunistic reap (diagnostics).
    #[cfg(test)]
    pub(crate) fn uncertain_startup_worker_count_for_test(&self, id: &str) -> usize {
        recover_mutex(&self.uncertain_startups, "plugin uncertain startups")
            .get(id)
            .map_or(0, Vec::len)
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
