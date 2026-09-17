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
            Self::PointsChanged { .. } => "points.changed",
            Self::WorkflowChanged { .. } => "workflow.changed",
            Self::CreatorChanged { .. } => "creator.changed",
            Self::AnalyticsUpdated { .. } => "analytics.updated",
            Self::Shutdown => "shutdown",
        }
    }
}

#[derive(Clone)]
pub struct EventBus {
    domain: broadcast::Sender<DomainEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (domain, _) = broadcast::channel(capacity);
        Self { domain }
    }

    pub fn publish_domain(&self, event: DomainEvent) {
        let _ = self.domain.send(event);
    }

    pub fn subscribe_domain(&self) -> broadcast::Receiver<DomainEvent> {
        self.domain.subscribe()
    }
}
