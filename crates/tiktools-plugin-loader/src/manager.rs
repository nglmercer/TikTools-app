//! Shared manager state, runtime registry and public entry point.
//!
//! Child modules implement cohesive operations on this same state; they do not
//! introduce separate locks, registries or worker owners. Lock order and poison
//! policy are documented on `PluginManager` and retained at each operation.

mod admission;
mod calls;
mod catalog;
mod health;
mod shutdown;
mod startup;

pub use admission::{PluginEventDeliveryPermit, PluginPackageLease};

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    thread,
    time::Duration,
};

use tiktools_plugin_api::{
    sync::{recover_mutex, recover_rwlock_read},
    PluginRuntimeKind,
};
use tokio::sync::broadcast;

#[cfg(feature = "native-plugins")]
use crate::NativePluginRuntime;

use crate::{
    worker::RunningInstance, DeclarativePluginRuntime, DiscoveredPlugin, EmittedEvent,
    NapiVmPluginRuntime, PluginLoaderError, PluginRoot, PluginRuntime, ProcessPluginRuntime,
    WasmPluginRuntime,
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

type UncertainStartups =
    Arc<Mutex<BTreeMap<String, Vec<thread::JoinHandle<Result<(), PluginLoaderError>>>>>>;

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
    uncertain_startups: UncertainStartups,
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

    pub fn is_running(&self, id: &str) -> bool {
        recover_rwlock_read(&self.instances, "plugin instances").contains_key(id)
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
