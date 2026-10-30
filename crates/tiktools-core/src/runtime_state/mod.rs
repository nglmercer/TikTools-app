//! AppCore-owned runtime state grouped by subsystem.
//!
//! These structs only group fields that previously lived directly on
//! [`AppCore`](crate::AppCore); they introduce no new behavior, no new
//! storage, and no public API. Each group owns the synchronization
//! primitives for one concern so subsystem state can be reasoned about in
//! isolation:
//!
//! - [`AutomationRuntimeState`] — last automation event memory, context
//!   emission throttle, sequence numbers, and native-live automation slots.
//! - [`PluginRuntimeState`] — plugin health/backoff, install/enable
//!   activation, poll and observer lifecycle, and the install lock.
//! - [`ProcessorRuntimeState`] — processor health, metrics, settings cache,
//!   contribution index, and the shared enrichment semaphore.
//! - [`TransportHealthState`] — IPC/WebView degradation flags and reliable
//!   event-gap counters reported through `system.health`.
//!
//! Out of scope on purpose: the `live`, `points`, `automation`, `media`,
//! `plugins`, `db`, `app_state`, and `events` subsystems keep their current
//! ownership, as do connection/session state, diagnostics projections, and
//! shutdown flags (see the `AppCore` field docs).

mod automation;
mod plugins;
mod transport;

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc, Mutex, RwLock,
    },
};

use tokio::sync::Notify;

use crate::{
    plugin_events::fanout::FanoutCache,
    plugin_processors::{
        ContributionIndex, ProcessorKey, ProcessorMetrics, ProcessorSettingsStore,
        MAX_TOTAL_PROCESSOR_SLOTS,
    },
    plugin_runtime::PluginActivation,
    PluginHealth,
};

/// Last-event memory and admission control for the automation pipeline.
pub(crate) struct AutomationRuntimeState {
    pub(crate) last_event: RwLock<Option<serde_json::Value>>,
    pub(crate) last_event_at: RwLock<Option<u64>>,
    /// Last observed envelope per event type (`tiktok.gift`, `hotkey.pressed`,
    /// ...). The test harness replays these so emulation runs against real
    /// data instead of generic samples; keyed by type, so one chat message
    /// never evicts the last gift.
    pub(crate) last_events: RwLock<BTreeMap<String, serde_json::Value>>,
    pub(crate) last_events_at: RwLock<BTreeMap<String, u64>>,
    pub(crate) last_context_emit_at: AtomicU64,
    pub(crate) sequence: AtomicU64,
    /// Bounds native-live automation work. Events arriving while all slots
    /// are occupied are intentionally dropped; live delivery must remain
    /// responsive and disposable events must not create an unbounded task
    /// backlog.
    #[cfg_attr(
        not(any(feature = "native-tiktok", feature = "persistence")),
        allow(dead_code)
    )]
    pub(crate) slots: Arc<tokio::sync::Semaphore>,
    /// Saturation diagnostics: admitted vs. rejected automation work.
    pub(crate) admitted: AtomicU64,
    pub(crate) rejected: AtomicU64,
    /// Execution diagnostics: finished live events, action outcomes, and
    /// summed execution latency (ms) for mean computation.
    pub(crate) events_completed: AtomicU64,
    pub(crate) actions_ok: AtomicU64,
    pub(crate) actions_failed: AtomicU64,
    pub(crate) exec_latency_sum_ms: AtomicU64,
}

impl Default for AutomationRuntimeState {
    fn default() -> Self {
        Self {
            last_event: RwLock::new(None),
            last_event_at: RwLock::new(None),
            last_events: RwLock::new(BTreeMap::new()),
            last_events_at: RwLock::new(BTreeMap::new()),
            last_context_emit_at: AtomicU64::new(0),
            sequence: AtomicU64::new(0),
            slots: Arc::new(tokio::sync::Semaphore::new(32)),
            admitted: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            events_completed: AtomicU64::new(0),
            actions_ok: AtomicU64::new(0),
            actions_failed: AtomicU64::new(0),
            exec_latency_sum_ms: AtomicU64::new(0),
        }
    }
}

/// Plugin health, activation, and background-task lifecycle.
pub(crate) struct PluginRuntimeState {
    pub(crate) health: Mutex<BTreeMap<String, PluginHealth>>,
    pub(crate) activation: RwLock<BTreeMap<String, PluginActivation>>,
    /// Bumped on every activation write (enable/disable/install). The
    /// event fan-out cache validates against this instead of re-reading
    /// activation per plugin per event.
    pub(crate) activation_revision: AtomicU64,
    /// Precomputed subscription fan-out, rebuilt only when the loader or
    /// activation revision moves.
    pub(crate) fanout: Mutex<FanoutCache>,
    /// Last fan-out overload warn (millis) for rate-limited logging.
    pub(crate) fanout_overload_warned_at: AtomicU64,
    pub(crate) poll_started: AtomicBool,
    pub(crate) poll_shutdown: Arc<Notify>,
    pub(crate) poll_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub(crate) push_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub(crate) observer_started: AtomicBool,
    pub(crate) observer_shutdown: tokio_util::sync::CancellationToken,
    pub(crate) observer_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub(crate) observer_worker_count: AtomicU64,
    #[cfg(feature = "plugin-install")]
    pub(crate) install_lock: Mutex<()>,
}

impl Default for PluginRuntimeState {
    fn default() -> Self {
        Self {
            health: Mutex::new(BTreeMap::new()),
            activation: RwLock::new(BTreeMap::new()),
            activation_revision: AtomicU64::new(1),
            fanout: Mutex::new(FanoutCache::default()),
            fanout_overload_warned_at: AtomicU64::new(0),
            poll_started: AtomicBool::new(false),
            poll_shutdown: Arc::new(Notify::new()),
            poll_task: Mutex::new(None),
            push_task: Mutex::new(None),
            observer_started: AtomicBool::new(false),
            observer_shutdown: tokio_util::sync::CancellationToken::new(),
            observer_task: Mutex::new(None),
            observer_worker_count: AtomicU64::new(0),
            #[cfg(feature = "plugin-install")]
            install_lock: Mutex::new(()),
        }
    }
}

/// Pre-filter processor pipeline caches and admission control.
pub(crate) struct ProcessorRuntimeState {
    pub(crate) health: Mutex<BTreeMap<ProcessorKey, PluginHealth>>,
    pub(crate) metrics: Mutex<BTreeMap<ProcessorKey, ProcessorMetrics>>,
    pub(crate) settings: ProcessorSettingsStore,
    pub(crate) index: RwLock<ContributionIndex>,
    #[cfg_attr(
        not(any(feature = "native-tiktok", feature = "persistence")),
        allow(dead_code)
    )]
    pub(crate) slots: Arc<tokio::sync::Semaphore>,
}

impl Default for ProcessorRuntimeState {
    fn default() -> Self {
        Self {
            health: Mutex::new(BTreeMap::new()),
            metrics: Mutex::new(BTreeMap::new()),
            settings: ProcessorSettingsStore::default(),
            index: RwLock::new(ContributionIndex::default()),
            slots: Arc::new(tokio::sync::Semaphore::new(MAX_TOTAL_PROCESSOR_SLOTS)),
        }
    }
}

/// Control-transport degradation flags and reliable-lane gap counters.
#[derive(Default)]
pub(crate) struct TransportHealthState {
    pub(crate) ipc_error: RwLock<Option<String>>,
    pub(crate) webview_error: RwLock<Option<String>>,
    /// Cumulative reliable-lane event gaps observed by any transport
    /// (IPC connections, WebView forwarder). Reported through
    /// `system.health`; see `record_event_gap`.
    pub(crate) event_gaps: AtomicU64,
    /// `now_millis()` of the most recent reliable gap, or 0 when no gap
    /// has been recorded since boot/acknowledge.
    pub(crate) last_event_gap_at: AtomicU64,
}
