//! TikTools application core.
//!
//! This crate owns domain orchestration and the host-side message contract. It
//! intentionally has no dependency on Winit, Wry, tray-icon, or any other GUI
//! implementation. The desktop crate supplies a `HostEmitter` and forwards
//! UI work to this crate from its Tokio runtime.

#[cfg(feature = "live-benchmark")]
pub mod benchmark;
pub mod contracts;
pub mod control;
pub mod db;
pub mod events;
#[cfg(feature = "app-update")]
pub use tiktools_app_update as app_update;
pub mod ipc;
pub mod paths;
pub mod platform;
pub mod services;
pub use tiktools_collaboration as collaboration;

mod automation_runtime;
#[cfg(feature = "cloud-auth")]
pub mod cloud_account;
#[cfg(all(feature = "cloud-auth", feature = "http"))]
pub mod cloud_bundles;
#[cfg(all(feature = "cloud-auth", feature = "http"))]
mod cloud_http;
#[cfg(all(feature = "cloud-auth", feature = "http"))]
pub mod cloud_integrations;
#[cfg(all(feature = "cloud-auth", feature = "http"))]
pub mod cloud_peer_host;
#[cfg(all(feature = "cloud-auth", feature = "http", feature = "plugin-install"))]
pub mod cloud_publisher;
#[cfg(all(feature = "cloud-auth", feature = "http"))]
pub mod cloud_remote_controller;
#[cfg(all(feature = "cloud-auth", feature = "http"))]
pub mod cloud_remote_host;
#[cfg(feature = "cloud-auth")]
pub mod cloud_sync;
#[cfg(all(feature = "cloud-auth", feature = "http"))]
pub mod cloud_teams;
mod helpers;
mod hotkey_bindings;
pub mod input_access;
mod ipc_handlers;
mod live_events;
pub(crate) use live_events::AutomationEvent;
#[cfg(feature = "app-update")]
mod app_updates;
mod collaboration_host;
mod emission;
mod initialization;
pub mod live_monitor;
mod media_operations;
mod persistence;
mod plugin_diagnostics;
mod plugin_events;
#[cfg(feature = "plugin-install")]
mod plugin_installation;
mod plugin_intents;
mod plugin_invoker;
#[cfg(feature = "plugin-install")]
mod plugin_packages;
mod plugin_processors;
mod plugin_runtime;
mod private_file;
mod runtime_state;
mod shutdown;
#[cfg(test)]
mod tests;

pub(crate) use automation_runtime::samples::{sample_automation_event, sample_user};
pub(crate) use helpers::*;
pub(crate) use tiktools_plugin_api::sync::{
    recover_mutex, recover_rwlock_read, recover_rwlock_write,
};

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use tiktools_plugin_api::{
    AudioPlayOptions, AudioPlaybackResult, MediaFileRef, MediaPickerOptions, MediaSelection,
};
use tiktools_plugin_loader::{plugin_roots, PluginManager};

#[cfg(feature = "native-tiktok")]
use tiktools_tiktok::{events::TikToolsEvent as NativeLiveEvent, ClientEvent, ConnectRequest};

#[cfg(not(feature = "native-tiktok"))]
#[derive(Debug)]
#[allow(dead_code)]
struct ConnectRequest {
    unique_id: String,
    session_cookie: String,
    room_id: Option<String>,
}

use crate::{
    events::EventBus,
    ipc::messages::{HostMessage, PageMessage},
    paths::AppPaths,
    runtime_state::{
        AutomationRuntimeState, PluginRuntimeState, ProcessorRuntimeState, TransportHealthState,
    },
    services::{
        builtin_action_types, builtin_node_catalog, builtin_translations,
        media_selection_from_path_with_kind, option_sources::OptionSourceService,
        validate_audio_file_ref, validate_media_picker_options, AppStateService, AutomationService,
        AwardOptions, CapabilityBroker, LiveService, PointAction, PointsService,
    },
};

pub use services::{
    MediaApiError, MediaError, MediaHost, MediaHostError, MediaHostFuture, NoopMediaHost,
};
#[cfg(feature = "plugin-install")]
pub use tiktools_plugin_installer as plugin_installer;
#[cfg(feature = "plugin-install")]
pub use tiktools_plugin_registry as plugin_registry;

pub trait HostEmitter: Send + Sync {
    fn emit(&self, message: HostMessage);
    /// Hosts can opt out of compatibility visualization pushes. Defaults
    /// preserve existing emitters; accounting/run recording never depends on it.
    fn wants_automation_history(&self) -> bool {
        true
    }
    fn wants_leaderboard_snapshots(&self) -> bool {
        true
    }
    fn wants_automation_context_updates(&self) -> bool {
        true
    }
    /// Optional aggregate-only host transport diagnostics; headless hosts
    /// retain the default null value and need no GUI dependency.
    fn diagnostics(&self) -> serde_json::Value {
        serde_json::Value::Null
    }
}

pub(crate) struct PluginHealth {
    pub(crate) consecutive_failures: u32,
    pub(crate) next_retry_at: Option<Instant>,
}

/// The Rust application service graph. Each subsystem has separate ownership
/// so a future database/live/plugin implementation can be tested in isolation.
pub struct AppCore {
    collaboration: Mutex<Option<Arc<collaboration::CollaborationService>>>,
    collaboration_gateway_available: AtomicBool,
    collaboration_peer_available: AtomicBool,
    coordination_events: tokio::sync::broadcast::Sender<Arc<collaboration::CoordinationEvent>>,
    coordination_streams: collaboration::StreamHub,
    automation_mutations: Mutex<()>,
    automation_write_epoch: AtomicU64,
    automation_write_settled: tokio::sync::Notify,
    #[cfg(feature = "persistence")]
    coordination_gate: Mutex<()>,
    #[cfg(all(test, feature = "persistence"))]
    approved_points_coordination_wait: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    #[cfg(feature = "persistence")]
    owner_adoption_gate: Mutex<()>,
    #[cfg(feature = "persistence")]
    coordination_worker: control::CoordinationWorker,
    #[cfg(feature = "app-update")]
    app_updater: RwLock<Option<Arc<tiktools_app_update::AppUpdater>>>,
    pub live: Arc<LiveService>,
    live_monitor: live_monitor::LiveMonitor,
    pub points: Arc<PointsService>,
    pub automation: Arc<AutomationService>,
    #[cfg(feature = "persistence")]
    pub(crate) effects: Result<services::effects::EffectJournal, String>,
    pub capabilities: Arc<CapabilityBroker>,
    pub media: Arc<dyn MediaHost>,
    pub plugins: Arc<PluginManager>,
    pub db: Arc<db::DatabaseManager>,
    #[cfg(feature = "persistence")]
    analytics_writer: db::AnalyticsWriter,
    pub app_state: Arc<AppStateService>,
    pub events: EventBus,
    emitter: Arc<dyn HostEmitter>,
    pub(crate) automation_state: AutomationRuntimeState,
    #[cfg(all(feature = "persistence", feature = "native-tiktok"))]
    last_analytics_emit_at: AtomicU64,
    /// Monotonic revision of the persisted hotkey behavior projection. The
    /// plugin poll consumes this asynchronously so UI writes never wait for
    /// a process-plugin round trip.
    hotkey_sync_revision: AtomicU64,
    hotkey_synced_revision: AtomicU64,
    last_leaderboard_emit_at: AtomicU64,
    #[cfg(feature = "http")]
    http_client: Option<reqwest::Client>,
    #[cfg(feature = "http")]
    http_client_error: Option<String>,
    #[cfg(feature = "native-tiktok")]
    connection_sequence: AtomicU64,
    #[cfg(feature = "native-tiktok")]
    live_points_work: services::points_work::PointsWorkService,
    connection_context: RwLock<Option<LiveContext>>,
    #[cfg(feature = "native-tiktok")]
    live_pump_started: AtomicBool,
    /// Cancels *new* LIVE event admission only. Cancelling it stops the pump,
    /// which closes the queue sender and lets the worker drain everything
    /// already accepted. It is deliberately not the worker's token.
    #[cfg(feature = "native-tiktok")]
    live_admission: tokio_util::sync::CancellationToken,
    /// Emergency stop for the ordered worker, cancelled only after the
    /// graceful drain deadline expires. The worker then stops between items
    /// and the shutdown ladder accounts for whatever is left.
    #[cfg(feature = "native-tiktok")]
    live_ingress_stop: tokio_util::sync::CancellationToken,
    /// Bounded stage deadlines for `AppCore::stop_live_ingress`.
    #[cfg(feature = "native-tiktok")]
    live_shutdown_deadlines: Mutex<crate::live_events::pump::LiveShutdownDeadlines>,
    #[cfg(feature = "native-tiktok")]
    live_pump_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Ordered ingress worker: the sole consumer of the bounded LIVE queue.
    /// Joined at shutdown after the pump so accepted events drain.
    #[cfg(feature = "native-tiktok")]
    live_ingress_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Bounded staged-ingestion admission counters (accepted, shed,
    /// lagged, fenced). Always present with the native transport.
    #[cfg(feature = "native-tiktok")]
    pub(crate) live_ingress: crate::live_events::ingress::LiveIngressStats,
    /// Canonical replay guard: exact `(type, msg_id)` replays are skipped
    /// with a counter before any side effect. Survives reconnects by
    /// design (replays arrive after them); bounded by TTL plus capacity.
    #[cfg(feature = "native-tiktok")]
    pub(crate) live_dedup: crate::live_events::dedup::LiveDedup,
    #[cfg(feature = "native-tiktok")]
    pub(crate) live_totals: crate::live_events::totals::LiveTotals,
    pub(crate) aggregations: services::aggregations::AggregationService,
    pub(crate) competitions: services::competitions::CompetitionService,
    option_sources: OptionSourceService,
    pub(crate) plugin_state: PluginRuntimeState,
    pub(crate) processor_state: ProcessorRuntimeState,
    pub(crate) plugin_last_events:
        Mutex<BTreeMap<String, crate::plugin_diagnostics::PluginLastEvent>>,
    pub(crate) plugin_event_drops: Mutex<BTreeMap<String, u64>>,
    pub(crate) plugin_delivery:
        Mutex<BTreeMap<String, crate::plugin_events::fanout::PluginDeliveryStats>>,
    pub(crate) hotkey_sync_state: Mutex<crate::plugin_diagnostics::HotkeySyncState>,
    shutdown_started: AtomicBool,
    shutdown_finished: AtomicBool,
    shutdown_update_safe: AtomicBool,
    shutdown_complete: tokio::sync::Notify,
    pub(crate) transport_state: TransportHealthState,
    #[cfg(feature = "plugin-install")]
    pub(crate) packages: Result<Arc<plugin_packages::PluginPackages>, String>,
    #[cfg(feature = "cloud-auth")]
    pub(crate) cloud: Arc<cloud_account::CloudAccount>,
    #[cfg(feature = "cloud-auth")]
    pub(crate) sync: Arc<cloud_sync::CloudSync>,
    #[cfg(all(feature = "cloud-auth", feature = "http", feature = "plugin-install"))]
    pub(crate) publisher: Arc<cloud_publisher::CloudPublisher>,
    #[cfg(all(feature = "cloud-auth", feature = "http"))]
    pub(crate) teams: Arc<cloud_teams::CloudTeams>,
    #[cfg(all(feature = "cloud-auth", feature = "http"))]
    pub(crate) remote_host: Arc<cloud_remote_host::RemoteHostLink>,
}
