//! Application event bus independent of the desktop event loop.
//!
//! [`DomainEvent`] is the single authoritative event bus: every client
//! (WebView, CLI, stdio, local IPC, tests, agents) observes the same typed
//! topics. There is no UI-oriented bus; legacy `HostMessage` pushes stay on
//! the emitter path, never on this bus.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Domain event observed identically by every control client.
///
/// The serde form is `{ "topic": "<dotted.topic>", "data": { ... } }`, which
/// matches the JSON-RPC event notification envelope used on stdio and IPC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "topic", content = "data")]
pub enum DomainEvent {
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
    #[serde(rename = "live.connected", rename_all = "camelCase")]
    LiveConnected {
        unique_id: Option<String>,
        room_id: Option<String>,
    },
    #[serde(rename = "live.disconnected")]
    LiveDisconnected,
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

impl DomainEvent {
    pub fn topic(&self) -> &'static str {
        match self {
            Self::PluginInstalled { .. } => "plugin.installed",
            Self::PluginUninstalled { .. } => "plugin.uninstalled",
            Self::PluginStarted { .. } => "plugin.started",
            Self::PluginStopped { .. } => "plugin.stopped",
            Self::PluginProgress { .. } => "plugin.progress",
            Self::PluginSettingsChanged { .. } => "plugin.settings-changed",
            Self::LiveConnected { .. } => "live.connected",
            Self::LiveDisconnected => "live.disconnected",
            Self::LiveEvent { .. } => "live.event",
            Self::LiveUiEvent { .. } => "live.ui-event",
            Self::RoomStats { .. } => "room.stats",
            Self::GiftsCatalog { .. } => "gifts.catalog",
            Self::LiveReconnecting { .. } => "live.reconnecting",
            Self::LiveError { .. } => "live.error",
            Self::PointsChanged { .. } => "points.changed",
            Self::WorkflowChanged { .. } => "workflow.changed",
            Self::CreatorChanged { .. } => "creator.changed",
            Self::AnalyticsUpdated { .. } => "analytics.updated",
            Self::Shutdown => "shutdown",
        }
    }
}

impl DomainEvent {
    /// High-rate feed and superseding snapshots (live feed, room stats,
    /// analytics, gift catalog, plugin progress) travel the lossy lane:
    /// a lagged subscriber skips them and takes the latest. Every other
    /// event — connection/lifecycle transitions, errors, points changes,
    /// shutdown — travels the reliable lane, so a feed burst can never
    /// overwrite a state transition a subscriber has not read yet.
    /// Mirrors the WebView `Coalescable`/`Droppable` classes.
    pub fn is_lossy(&self) -> bool {
        matches!(
            self,
            Self::LiveEvent { .. }
                | Self::LiveUiEvent { .. }
                | Self::RoomStats { .. }
                | Self::AnalyticsUpdated { .. }
                | Self::GiftsCatalog { .. }
                | Self::PluginProgress { .. }
        )
    }
}

#[derive(Clone)]
pub struct EventBus {
    reliable: broadcast::Sender<DomainEvent>,
    lossy: broadcast::Sender<DomainEvent>,
}

impl EventBus {
    /// `capacity` sizes the reliable lane; the lossy lane gets four times
    /// that for high-rate feed bursts.
    pub fn new(capacity: usize) -> Self {
        let (reliable, _) = broadcast::channel(capacity);
        let (lossy, _) = broadcast::channel(capacity.saturating_mul(4).max(1));
        Self { reliable, lossy }
    }

    /// Publishes to the reliable or lossy lane by event class. Like
    /// `broadcast::send`, this never blocks and never fails for a slow
    /// subscriber: laggards skip on their own cursor only.
    pub fn publish_domain(&self, event: DomainEvent) {
        if event.is_lossy() {
            let _ = self.lossy.send(event);
        } else {
            let _ = self.reliable.send(event);
        }
    }

    pub fn subscribe_domain(&self) -> DomainSubscription {
        DomainSubscription {
            reliable: self.reliable.subscribe(),
            lossy: self.lossy.subscribe(),
        }
    }
}

/// Merged view over the reliable and lossy lanes with `broadcast`
/// semantics: `recv`/`try_recv` report `Lagged` (recoverable: the caller
/// logs and continues) or `Closed` (the bus is gone) exactly like a
/// single receiver. Reliable messages are always drained first, so a
/// state transition never waits behind feed backlog.
pub struct DomainSubscription {
    reliable: broadcast::Receiver<DomainEvent>,
    lossy: broadcast::Receiver<DomainEvent>,
}

impl DomainSubscription {
    /// Receives the next event, reliable lane first. A lagged lane
    /// reports `Lagged` once and then streams its retained backlog; the
    /// other lane is unaffected.
    pub async fn recv(&mut self) -> Result<DomainEvent, broadcast::error::RecvError> {
        use broadcast::error::{RecvError, TryRecvError};
        match self.try_recv() {
            Err(TryRecvError::Empty) => {}
            Ok(event) => return Ok(event),
            Err(TryRecvError::Lagged(skipped)) => return Err(RecvError::Lagged(skipped)),
            Err(TryRecvError::Closed) => return Err(RecvError::Closed),
        }
        // Both lanes are empty: wait biased toward reliable. A lossy
        // wake may overtake a reliable message published a moment later;
        // the next `recv` takes reliable first, so control events stay
        // prompt while feed stays merely ordered.
        tokio::select! {
            biased;
            message = self.reliable.recv() => {
                // The bus died mid-wait: drain lossy stragglers buffered
                // before the drop instead of losing them to Closed.
                if matches!(message, Err(broadcast::error::RecvError::Closed)) {
                    if let Ok(straggler) = self.lossy.try_recv() {
                        return Ok(straggler);
                    }
                }
                message
            }
            message = self.lossy.recv() => message,
        }
    }

    pub fn try_recv(&mut self) -> Result<DomainEvent, broadcast::error::TryRecvError> {
        use broadcast::error::TryRecvError;
        match self.reliable.try_recv() {
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Closed) => {
                // The bus is gone: drain lossy stragglers before Closed.
                return match self.lossy.try_recv() {
                    Err(TryRecvError::Empty) => Err(TryRecvError::Closed),
                    other => other,
                };
            }
            other => return other,
        }
        match self.lossy.try_recv() {
            // Both lanes empty and the reliable sender is alive (it would
            // report Closed, not Empty, otherwise), so more events can
            // still arrive: report Empty, never Closed.
            Err(TryRecvError::Empty) => Err(TryRecvError::Empty),
            other => other,
        }
    }
}
