//! Application event bus independent of the desktop event loop.
//!
//! [`DomainEvent`] is the single authoritative event bus: every client
//! (WebView, CLI, stdio, local IPC, tests, agents) observes the same typed
//! topics. There is no UI-oriented bus; legacy `HostMessage` pushes stay on
//! the emitter path, never on this bus.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

mod domain;
#[cfg(feature = "native-tiktok")]
mod generations;
mod history;
mod projection;
mod publication;
mod subscriptions;
use projection::project_creator_event;

/// Domain event observed identically by every control client.
///
/// The serde form is `{ "topic": "<dotted.topic>", "data": { ... } }`, which
/// matches the JSON-RPC event notification envelope used on stdio and IPC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "topic", content = "data")]
pub enum DomainEvent {
    /// An owner Cloud account or sync setting may have changed. Contains no
    /// account identity or credentials. Delivery is advisory and asynchronous;
    /// it does not authorize displaying a previously fetched private value.
    #[serde(rename = "cloud.context.invalidated")]
    CloudContextInvalidated {},
    #[serde(rename = "app.update.stateChanged", rename_all = "camelCase")]
    AppUpdateStateChanged { state: Value },
    #[serde(rename = "plugin.installed", rename_all = "camelCase")]
    PluginInstalled { plugin_id: String },
    #[serde(rename = "plugin.uninstalled", rename_all = "camelCase")]
    PluginUninstalled { plugin_id: String },
    #[serde(rename = "plugin.started", rename_all = "camelCase")]
    PluginStarted { plugin_id: String },
    #[serde(rename = "plugin.stopped", rename_all = "camelCase")]
    PluginStopped { plugin_id: String },
    #[serde(rename = "plugin.progress", rename_all = "camelCase")]
    PluginProgress {
        plugin_id: String,
        state: String,
        progress: Option<f32>,
        message: String,
    },
    #[serde(rename = "plugin.settings-changed", rename_all = "camelCase")]
    PluginSettingsChanged { plugin_id: String },
    #[serde(rename = "plugin.updated", rename_all = "camelCase")]
    PluginUpdated {
        plugin_id: String,
        from_version: Option<String>,
        to_version: String,
    },
    #[serde(rename = "plugin.job", rename_all = "camelCase")]
    PluginJob { job: Value },
    #[serde(rename = "plugin.updates", rename_all = "camelCase")]
    PluginUpdates { report: Value },
    /// Validated spontaneous plugin event (hotkeys, timers, watchers),
    /// published immediately after poll/validation and before automation
    /// enrichment/execution, so control-plane subscribers observe raw
    /// plugin events even when automation drops, throttles, or fails.
    #[serde(rename = "plugin.event", rename_all = "camelCase")]
    PluginEvent {
        plugin_id: String,
        event_type: String,
        event: serde_json::Value,
    },
    /// Plugin listener health snapshot (for example `hotkey.status`
    /// payloads), published alongside the typed plugin event so UIs can
    /// render backend state without polling.
    #[serde(rename = "plugin.status", rename_all = "camelCase")]
    PluginStatus {
        plugin_id: String,
        status: serde_json::Value,
    },
    /// One automation action finished (ok or error). Published with every
    /// recorded run so subscribers observe completions without polling.
    #[serde(rename = "automation.run.completed", rename_all = "camelCase")]
    AutomationRunCompleted { run: serde_json::Value },
    /// The authoritative recent-runs list changed. Carries the same list
    /// as the legacy `behavior-runs` push; new subscribers should use
    /// this topic instead of the compatibility push.
    #[serde(rename = "automation.runs.changed", rename_all = "camelCase")]
    AutomationRunsChanged { runs: Vec<serde_json::Value> },
    #[serde(rename = "live.connected", rename_all = "camelCase")]
    LiveConnected {
        unique_id: Option<String>,
        room_id: Option<String>,
    },
    #[serde(rename = "live.disconnected")]
    LiveDisconnected,
    #[serde(rename = "live.monitor", rename_all = "camelCase")]
    LiveMonitorChanged {
        status: crate::live_monitor::LiveMonitorStatus,
    },
    #[serde(rename = "live.event", rename_all = "camelCase")]
    LiveEvent {
        event_type: String,
        event: serde_json::Value,
    },
    /// UI-ready live event (render shape), published alongside the
    /// automation-shaped `live.event` so the frontend can migrate off the
    /// legacy `live-event` push without depending on automation capacity.
    #[serde(rename = "live.ui-event", rename_all = "camelCase")]
    LiveUiEvent { event: serde_json::Value },
    /// Bounded per-room likes ranking calculated from accepted native
    /// deltas, not UI feed rows, room snapshots or points.
    #[serde(rename = "live.likes-ranking", rename_all = "camelCase")]
    LiveLikesRanking {
        creator_id: String,
        room_id: String,
        leaders: Vec<Value>,
    },
    /// Bounded wider native like projection for read-only widgets.
    #[serde(rename = "live.likes-ranking-full", rename_all = "camelCase")]
    LiveLikesRankingFull {
        creator_id: String,
        room_id: String,
        leaders: Vec<Value>,
        #[serde(default)]
        integrity_status: String,
    },
    #[serde(rename = "competition.started")]
    CompetitionStarted(crate::contracts::CompetitionAutomationData),
    #[serde(rename = "competition.paused")]
    CompetitionPaused(crate::contracts::CompetitionAutomationData),
    #[serde(rename = "competition.resumed")]
    CompetitionResumed(crate::contracts::CompetitionAutomationData),
    #[serde(rename = "competition.cancelled")]
    CompetitionCancelled(crate::contracts::CompetitionAutomationData),
    #[serde(rename = "competition.finished")]
    CompetitionFinished(crate::contracts::CompetitionAutomationData),
    #[serde(rename = "competition.winner.selected")]
    CompetitionWinnerSelected(crate::contracts::CompetitionAutomationData),
    #[serde(rename = "competition.action.completed")]
    CompetitionActionCompleted(crate::contracts::CompetitionAutomationData),
    #[serde(rename = "competition.action.failed")]
    CompetitionActionFailed(crate::contracts::CompetitionAutomationData),
    /// A committed, immutable result for read-only reveal clients. This is
    /// a presentation snapshot and never triggers automation.
    #[serde(rename = "competition.result", rename_all = "camelCase")]
    CompetitionResult {
        result: tiktools_competitions::RevealSnapshot,
    },
    /// Presentation-only snapshot; reconnecting clients use rankings.snapshot.
    #[serde(rename = "ranking.updated", rename_all = "camelCase")]
    RankingUpdated {
        snapshot: tiktools_aggregation::Snapshot,
    },
    #[serde(rename = "room.stats", rename_all = "camelCase")]
    RoomStats {
        viewers: u64,
        total_users: u64,
        top_viewers: Vec<serde_json::Value>,
    },
    #[serde(rename = "gifts.catalog", rename_all = "camelCase")]
    GiftsCatalog { gifts: Vec<serde_json::Value> },
    #[serde(rename = "live.reconnecting", rename_all = "camelCase")]
    LiveReconnecting { attempt: u64, delay_ms: u64 },
    #[serde(rename = "live.error", rename_all = "camelCase")]
    LiveError { phase: String, message: String },
    #[serde(rename = "points.changed", rename_all = "camelCase")]
    PointsChanged {
        unique_id: String,
        delta: f64,
        total_points: f64,
        level: u32,
    },
    #[serde(rename = "points.reset", rename_all = "camelCase")]
    PointsReset { unique_id: Option<String> },
    #[serde(rename = "workflow.changed", rename_all = "camelCase")]
    WorkflowChanged {
        kind: String,
        id: String,
        change: String,
    },
    #[serde(rename = "creator.changed", rename_all = "camelCase")]
    CreatorChanged { unique_id: Option<String> },
    #[serde(rename = "analytics.updated", rename_all = "camelCase")]
    AnalyticsUpdated { creator_unique_id: String },
    #[serde(rename = "shutdown")]
    Shutdown,
}

/// Stable conversion failure when a [`DomainEvent`] cannot be serialized
/// into its transport-neutral envelope. This indicates an invariant
/// violation (every variant must serialize); callers must log it and,
/// for reliable events, record a gap instead of delivering `null` data.
#[derive(Debug, thiserror::Error)]
pub enum EventEnvelopeError {
    #[error("domain event failed to serialize: {0}")]
    Serialization(String),
    #[error("serialized domain event is missing its data payload")]
    MissingData,
}

/// Bounded feed replay payload, matching the existing desktop recovery contract.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeedBackfill {
    pub oldest_cursor: u64,
    pub latest_cursor: u64,
    pub depth: usize,
    pub gap: bool,
    pub truncated: bool,
    pub events: Vec<FeedEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FeedEntry {
    pub cursor: u64,
    pub event: Value,
}

/// A creator-local projection. Neither cursors nor the stream identity reveal
/// the sequence or volume of other creators' activity.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatorFeedPage {
    pub creator_id: String,
    pub stream_id: Option<String>,
    pub oldest_cursor: String,
    pub latest_cursor: String,
    pub next_cursor: String,
    pub gap: bool,
    pub truncated: bool,
    pub events: Vec<CreatorFeedEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatorFeedEvent {
    pub cursor: String,
    pub kind: String,
    pub author: String,
    pub author_lookup_id: Option<String>,
    pub nickname: Option<String>,
    pub text: String,
}

#[derive(Debug)]
pub struct CreatorFeedUnavailable;

#[derive(Clone)]
struct PublishedEvent {
    event: Arc<DomainEvent>,
    ui_sequence: Option<u64>,
    /// Host-only routing metadata. JSON can never opt into plugin execution.
    observer_allowed: bool,
}

struct FeedHistoryEntry {
    cursor: u64,
    event: Arc<DomainEvent>,
}
struct CreatorFeedStream {
    id: String,
    next: u64,
    last_global: u64,
    /// Sticky for the lifetime of this bounded stream. A drop can precede
    /// the first retained event, so no cursor position can describe it.
    ingress_gap: bool,
    events: std::collections::VecDeque<CreatorFeedEvent>,
}
#[derive(Default)]
#[cfg_attr(not(feature = "native-tiktok"), allow(dead_code))]
struct FeedHistoryState {
    entries: std::collections::VecDeque<FeedHistoryEntry>,
    creators: std::collections::BTreeMap<String, CreatorFeedStream>,
    /// Host-only bindings; event payload JSON never supplies these values.
    live_generations: std::collections::BTreeMap<u64, String>,
    pending_ingress_gaps: std::collections::BTreeSet<u64>,
    unverified_generations: std::collections::BTreeSet<u64>,
    unverified_expired_through: Option<u64>,
    scoped_unavailable: bool,
}
type FeedHistory = Arc<Mutex<FeedHistoryState>>;

pub fn valid_creator_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 256 && id.trim() == id && !id.chars().any(char::is_control)
}

#[derive(Clone)]
pub struct EventBus {
    reliable: broadcast::Sender<PublishedEvent>,
    lossy: broadcast::Sender<PublishedEvent>,
    plugin_reliable: broadcast::Sender<PublishedEvent>,
    plugin_lossy: broadcast::Sender<PublishedEvent>,
    ui_sequence: Arc<Mutex<u64>>,
    history_subscribers: Arc<std::sync::atomic::AtomicUsize>,
    feed_history: FeedHistory,
}

/// Lane-aware `recv` failure. A lagged lane reports once and then
/// streams its retained backlog; the other lane is unaffected. Reliable
/// lag is never silent: the caller must emit an explicit gap/resync
/// signal so clients refresh authoritative state instead of assuming a
/// complete stream. Lossy lag only skipped feed/snapshots and needs no
/// signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainRecvError {
    /// Authoritative events were lost on the reliable lane; `u64` counts
    /// the skipped messages. The caller must send a gap notification.
    ReliableLagged(u64),
    /// Feed/snapshot messages were skipped on the lossy lane; safe to
    /// log at debug level and continue with no gap signal.
    LossyLagged(u64),
    /// The bus is gone and both lanes are drained.
    Closed,
}

/// Lane-aware `try_recv` failure. See [`DomainRecvError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainTryRecvError {
    /// Authoritative events were lost on the reliable lane; `u64` counts
    /// the skipped messages. The caller must send a gap notification and
    /// keep draining, never stop.
    ReliableLagged(u64),
    /// Feed/snapshot messages were skipped on the lossy lane; safe to
    /// continue draining with no gap signal.
    LossyLagged(u64),
    Empty,
    Closed,
}

/// Merged view over the reliable and lossy lanes. `recv`/`try_recv`
/// report lane-aware lag (recoverable: the caller handles the lag and
/// continues) or `Closed` (the bus is gone). Reliable messages are
/// always drained first, so a state transition never waits behind feed
/// backlog.
pub struct DomainSubscription {
    reliable: broadcast::Receiver<PublishedEvent>,
    lossy: broadcast::Receiver<PublishedEvent>,
    last_ui_sequence: u64,
    history_owner: Option<Arc<std::sync::atomic::AtomicUsize>>,
}

/// Cannot be constructed from an ordinary presentation subscription.
pub(crate) struct PluginDomainSubscription {
    inner: DomainSubscription,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod shared_projection_tests;

#[cfg(test)]
mod history_interest_tests;

#[cfg(test)]
mod feed_history_tests;

impl EventBus {
    /// `capacity` sizes the reliable lane; the lossy lane gets four times
    /// that for high-rate feed bursts.
    pub fn new(capacity: usize) -> Self {
        let (reliable, _) = broadcast::channel(capacity);
        let (lossy, _) = broadcast::channel(capacity.saturating_mul(4).max(1));
        let (plugin_reliable, _) = broadcast::channel(capacity);
        let (plugin_lossy, _) = broadcast::channel(capacity.saturating_mul(4).max(1));
        Self {
            reliable,
            lossy,
            plugin_reliable,
            plugin_lossy,
            ui_sequence: Arc::new(Mutex::new(0)),
            history_subscribers: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            feed_history: Arc::new(Mutex::new(FeedHistoryState::default())),
        }
    }
}
