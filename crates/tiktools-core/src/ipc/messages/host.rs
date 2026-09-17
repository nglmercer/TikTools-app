use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_plugin_api::MediaSelection;

use super::{install::PluginInstallErrorCode, points::PointsConfig};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorPhase {
    Connect,
    Live,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum HostMessage {
    #[serde(rename = "connection")]
    Connection {
        status: ConnectionStatus,
        #[serde(skip_serializing_if = "Option::is_none", rename = "uniqueId")]
        unique_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "roomId")]
        room_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "avatarUrl")]
        avatar_url: Option<String>,
    },
    #[serde(rename = "live-event")]
    LiveEvent { event: Value },
    #[serde(rename = "room-stats")]
    RoomStats {
        viewers: u64,
        #[serde(rename = "totalUsers")]
        total_users: u64,
        #[serde(rename = "topViewers")]
        top_viewers: Vec<Value>,
    },
    #[serde(rename = "reconnecting")]
    Reconnecting {
        attempt: u32,
        #[serde(rename = "delayMs")]
        delay_ms: u64,
    },
    #[serde(rename = "error")]
    Error { phase: ErrorPhase, message: String },
    #[serde(rename = "media-selected")]
    MediaSelected {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        selection: Option<MediaSelection>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "points-config")]
    PointsConfig { config: PointsConfig },
    #[serde(rename = "leaderboard")]
    Leaderboard { viewers: Vec<Value> },
    #[serde(rename = "points-awarded")]
    PointsAwarded {
        #[serde(rename = "uniqueId")]
        unique_id: String,
        delta: f64,
        #[serde(rename = "totalPoints")]
        total_points: f64,
        level: u32,
    },
    #[serde(rename = "creator-state")]
    CreatorState { creator: Option<Value> },
    #[serde(rename = "recent-creators")]
    RecentCreators { creators: Vec<Value> },
    #[serde(rename = "app-state")]
    AppState { state: BTreeMap<String, String> },
    #[serde(rename = "gift-catalog")]
    GiftCatalog { gifts: Vec<Value> },
    #[serde(rename = "gift-debug")]
    GiftDebug {
        #[serde(skip_serializing_if = "Option::is_none", rename = "giftId")]
        gift_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "iconUrl")]
        icon_url: Option<String>,
        #[serde(rename = "hasIcon")]
        has_icon: bool,
        #[serde(rename = "totalGifts")]
        total_gifts: u64,
    },
    #[serde(rename = "automation-workflows")]
    AutomationWorkflows { workflows: Vec<Value> },
    #[serde(rename = "automation-node-catalog")]
    AutomationNodeCatalog { nodes: Vec<Value> },
    #[serde(rename = "automation-context")]
    AutomationContext {
        event: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "capturedAt")]
        captured_at: Option<u64>,
    },
    #[serde(rename = "hotkey-status")]
    HotkeyStatus { status: Value },
    #[serde(rename = "automation-script-analysis")]
    AutomationScriptAnalysis { analysis: Value },
    #[serde(rename = "automation-error")]
    AutomationError { message: String },
    #[serde(rename = "behavior")]
    Behavior { snapshot: Value },
    #[serde(rename = "behavior-runs")]
    BehaviorRuns { runs: Vec<Value> },
    #[serde(rename = "behavior-test-result")]
    BehaviorTestResult { runs: Vec<Value> },
    #[serde(rename = "behavior-error")]
    BehaviorError { message: String },
    #[serde(rename = "plugin-settings")]
    PluginSettings {
        id: String,
        schema: Value,
        #[serde(skip_serializing_if = "Option::is_none", rename = "uiHints")]
        ui_hints: Option<Value>,
        values: Value,
    },
    #[serde(rename = "plugin-progress")]
    PluginProgress {
        #[serde(rename = "pluginId")]
        plugin_id: String,
        state: PluginProgressState,
        #[serde(skip_serializing_if = "Option::is_none")]
        progress: Option<f32>,
        message: String,
    },
    #[serde(rename = "action-options")]
    ActionOptions {
        source: String,
        options: Vec<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "plugin-action-result")]
    PluginActionResult {
        #[serde(rename = "actionType")]
        action_type: String,
        ok: bool,
        summary: String,
        logs: Vec<String>,
        #[serde(rename = "durationMs")]
        duration_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "plugin-connection-result")]
    PluginConnectionResult {
        id: String,
        ok: bool,
        #[serde(rename = "latencyMs")]
        latency_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "plugin-install-result")]
    PluginInstallResult {
        success: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        replaced: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<PluginInstallErrorCode>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "plugin-uninstall-result")]
    PluginUninstallResult {
        success: bool,
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "processor-test-result")]
    ProcessorTestResult {
        #[serde(rename = "pluginId")]
        plugin_id: String,
        #[serde(rename = "processorId")]
        processor_id: String,
        ok: bool,
        #[serde(rename = "durationMs")]
        duration_ms: u64,
        result: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "processor-status")]
    ProcessorStatus { processors: Value },
    #[serde(rename = "analytics-summary")]
    AnalyticsSummary { summary: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PluginProgressState {
    Downloading,
    Loading,
    Ready,
    Failed,
}

impl HostMessage {
    pub fn connection_disconnected() -> Self {
        Self::Connection {
            status: ConnectionStatus::Disconnected,
            unique_id: None,
            title: None,
            room_id: None,
            avatar_url: None,
        }
    }

    pub fn plugin_install_success(id: String, version: String, replaced: bool) -> Self {
        Self::PluginInstallResult {
            success: true,
            id: Some(id),
            version: Some(version),
            replaced: Some(replaced),
            code: None,
            error: None,
        }
    }

    pub fn plugin_install_failure(code: PluginInstallErrorCode, error: String) -> Self {
        Self::PluginInstallResult {
            success: false,
            id: None,
            version: None,
            replaced: None,
            code: Some(code),
            error: Some(error),
        }
    }

    pub fn plugin_uninstall_success(id: String) -> Self {
        Self::PluginUninstallResult {
            success: true,
            id,
            error: None,
        }
    }

    pub fn plugin_uninstall_failure(id: String, error: String) -> Self {
        Self::PluginUninstallResult {
            success: false,
            id,
            error: Some(error),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
