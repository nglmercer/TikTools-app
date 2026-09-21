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
    pub(crate) last_context_emit_at: AtomicU64,
    pub(crate) sequence: AtomicU64,
    /// Bounds native-live automation work. Events arriving while all slots
    /// are occupied are intentionally dropped; live delivery must remain
    /// responsive and disposable events must not create an unbounded task
    /// backlog.
    #[cfg(feature = "native-tiktok")]
    pub(crate) slots: Arc<tokio::sync::Semaphore>,
}

impl Default for AutomationRuntimeState {
    fn default() -> Self {
        Self {
            last_event: RwLock::new(None),
            last_event_at: RwLock::new(None),
            last_context_emit_at: AtomicU64::new(0),
            sequence: AtomicU64::new(0),
            #[cfg(feature = "native-tiktok")]
            slots: Arc::new(tokio::sync::Semaphore::new(32)),
        }
    }
}

/// Plugin health, activation, and background-task lifecycle.
pub(crate) struct PluginRuntimeState {
    pub(crate) health: Mutex<BTreeMap<String, PluginHealth>>,
    pub(crate) activation: RwLock<BTreeMap<String, PluginActivation>>,
    pub(crate) poll_started: AtomicBool,
    pub(crate) poll_shutdown: Arc<Notify>,
    pub(crate) poll_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub(crate) observer_started: AtomicBool,
    pub(crate) observer_shutdown: tokio_util::sync::CancellationToken,
    pub(crate) observer_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub(crate) install_lock: Mutex<()>,
}

impl Default for PluginRuntimeState {
    fn default() -> Self {
        Self {
            health: Mutex::new(BTreeMap::new()),
            activation: RwLock::new(BTreeMap::new()),
            poll_started: AtomicBool::new(false),
            poll_shutdown: Arc::new(Notify::new()),
            poll_task: Mutex::new(None),
            observer_started: AtomicBool::new(false),
            observer_shutdown: tokio_util::sync::CancellationToken::new(),
            observer_task: Mutex::new(None),
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
