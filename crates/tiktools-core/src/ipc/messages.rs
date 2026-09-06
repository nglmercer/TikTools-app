//! Rust wire model for `src/shared/messages.ts`.
//!
//! The model intentionally keeps automation/live payloads as JSON values until
//! their parity ports are complete. Field names and message discriminators are
//! the compatibility boundary used by the existing Vue UI.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tiktools_plugin_api::{MediaKind, MediaPickerMode, MediaSelection};

pub type JsonObject = BTreeMap<String, Value>;

/// Sensible upper bound for a native `.plugin` archive path. Windows extended
/// paths max out at 32_767 characters; the 2 MB IPC envelope is the other
/// bound. The installer itself remains responsible for canonicalization.
pub const MAX_PLUGIN_PACKAGE_PATH_LEN: usize = 32_767;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PartialPointsConfig {
    #[serde(default)]
    pub currency_name: Option<String>,
    #[serde(default)]
    pub points_per_coin: Option<f64>,
    #[serde(default)]
    pub points_per_coin_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_share: Option<f64>,
    #[serde(default)]
    pub points_per_share_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_chat: Option<f64>,
    #[serde(default)]
    pub points_per_chat_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_like: Option<f64>,
    #[serde(default)]
    pub points_per_like_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_follow: Option<f64>,
    #[serde(default)]
    pub points_per_follow_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_join: Option<f64>,
    #[serde(default)]
    pub points_per_join_enabled: Option<bool>,
    #[serde(default)]
    pub sub_bonus_multiplier: Option<f64>,
    #[serde(default)]
    pub points_per_level: Option<f64>,
}

impl PartialPointsConfig {
    pub fn apply(self, config: &mut PointsConfig) {
        if let Some(value) = self.currency_name {
            config.currency_name = value;
        }
        if let Some(value) = self.points_per_coin {
            config.points_per_coin = value;
        }
        if let Some(value) = self.points_per_coin_enabled {
            config.points_per_coin_enabled = value;
        }
        if let Some(value) = self.points_per_share {
            config.points_per_share = value;
        }
        if let Some(value) = self.points_per_share_enabled {
            config.points_per_share_enabled = value;
        }
        if let Some(value) = self.points_per_chat {
            config.points_per_chat = value;
        }
        if let Some(value) = self.points_per_chat_enabled {
            config.points_per_chat_enabled = value;
        }
        if let Some(value) = self.points_per_like {
            config.points_per_like = value;
        }
        if let Some(value) = self.points_per_like_enabled {
            config.points_per_like_enabled = value;
        }
        if let Some(value) = self.points_per_follow {
            config.points_per_follow = value;
        }
        if let Some(value) = self.points_per_follow_enabled {
            config.points_per_follow_enabled = value;
        }
        if let Some(value) = self.points_per_join {
            config.points_per_join = value;
        }
        if let Some(value) = self.points_per_join_enabled {
            config.points_per_join_enabled = value;
        }
        if let Some(value) = self.sub_bonus_multiplier {
            config.sub_bonus_multiplier = value;
        }
        if let Some(value) = self.points_per_level {
            config.points_per_level = value;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointsConfig {
    pub currency_name: String,
    pub points_per_coin: f64,
    pub points_per_coin_enabled: bool,
    pub points_per_share: f64,
    pub points_per_share_enabled: bool,
    pub points_per_chat: f64,
    pub points_per_chat_enabled: bool,
    pub points_per_like: f64,
    pub points_per_like_enabled: bool,
    pub points_per_follow: f64,
    pub points_per_follow_enabled: bool,
    pub points_per_join: f64,
    pub points_per_join_enabled: bool,
    pub sub_bonus_multiplier: f64,
    pub points_per_level: f64,
}

impl PointsConfig {
    /// Normalizes values at the service boundary so legacy databases and IPC
    /// updates use the same safe ranges. Manual adjustments can still be
    /// negative; per-event rates and the level threshold cannot.
    pub fn normalize(&mut self) {
        const MAX_RATE: f64 = 1_000_000_000.0;
        const MAX_MULTIPLIER: f64 = 10_000.0;
        const MIN_POINTS_PER_LEVEL: f64 = 10.0;

        if self.currency_name.trim().is_empty() {
            self.currency_name = "Points".to_owned();
        } else {
            self.currency_name = self.currency_name.trim().chars().take(64).collect();
        }
        self.points_per_coin = finite_nonnegative(self.points_per_coin, 1.0, MAX_RATE);
        self.points_per_share = finite_nonnegative(self.points_per_share, 3.0, MAX_RATE);
        self.points_per_chat = finite_nonnegative(self.points_per_chat, 1.0, MAX_RATE);
        self.points_per_like = finite_nonnegative(self.points_per_like, 0.1, MAX_RATE);
        self.points_per_follow = finite_nonnegative(self.points_per_follow, 5.0, MAX_RATE);
        self.points_per_join = finite_nonnegative(self.points_per_join, 0.5, MAX_RATE);
        self.sub_bonus_multiplier =
            finite_nonnegative(self.sub_bonus_multiplier, 0.0, MAX_MULTIPLIER);
        self.points_per_level = if self.points_per_level.is_finite() {
            self.points_per_level.clamp(MIN_POINTS_PER_LEVEL, MAX_RATE)
        } else {
            100.0
        };
    }
}

fn finite_nonnegative(value: f64, fallback: f64, maximum: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, maximum)
    } else {
        fallback
    }
}

impl Default for PointsConfig {
    fn default() -> Self {
        Self {
            currency_name: "Points".to_owned(),
            points_per_coin: 1.0,
            points_per_coin_enabled: true,
            points_per_share: 3.0,
            points_per_share_enabled: true,
            points_per_chat: 1.0,
            points_per_chat_enabled: true,
            points_per_like: 0.1,
            points_per_like_enabled: true,
            points_per_follow: 5.0,
            points_per_follow_enabled: true,
            points_per_join: 0.5,
            points_per_join_enabled: false,
            sub_bonus_multiplier: 0.0,
            points_per_level: 100.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum PageMessage {
    #[serde(rename = "connect")]
    Connect {
        #[serde(rename = "uniqueId")]
        unique_id: String,
        #[serde(rename = "sessionCookie")]
        session_cookie: String,
        #[serde(default, rename = "roomId")]
        room_id: Option<String>,
    },
    #[serde(rename = "pick-live")]
    PickLive {
        #[serde(rename = "sessionCookie")]
        session_cookie: String,
    },
    #[serde(rename = "open-media-picker")]
    OpenMediaPicker {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(default)]
        mode: MediaPickerMode,
        #[serde(default)]
        kind: MediaKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(
            default,
            rename = "initialDirectory",
            skip_serializing_if = "Option::is_none"
        )]
        initial_directory: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        extensions: Vec<String>,
    },
    #[serde(rename = "disconnect")]
    Disconnect,
    #[serde(rename = "get-points-config")]
    GetPointsConfig,
    #[serde(rename = "update-points-config")]
    UpdatePointsConfig { config: PartialPointsConfig },
    #[serde(rename = "get-leaderboard")]
    GetLeaderboard {
        #[serde(default)]
        limit: Option<i64>,
    },
    #[serde(rename = "reset-points")]
    ResetPoints {
        #[serde(default, rename = "uniqueId")]
        unique_id: Option<String>,
    },
    #[serde(rename = "adjust-points")]
    AdjustPoints {
        #[serde(rename = "uniqueId")]
        unique_id: String,
        delta: f64,
    },
    #[serde(rename = "get-creator")]
    GetCreator {
        #[serde(default, rename = "uniqueId")]
        unique_id: Option<String>,
    },
    #[serde(rename = "get-recent-creators")]
    GetRecentCreators {
        #[serde(default)]
        limit: Option<i64>,
    },
    #[serde(rename = "get-app-state")]
    GetAppState {
        #[serde(default)]
        keys: Option<Vec<String>>,
    },
    #[serde(rename = "set-app-state")]
    SetAppState { key: String, value: String },
    #[serde(rename = "clear-creator-history")]
    ClearCreatorHistory,
    #[serde(rename = "debug-gift")]
    DebugGift {
        #[serde(default, rename = "giftId")]
        gift_id: Option<String>,
    },
    #[serde(rename = "get-automation-workflows")]
    GetAutomationWorkflows,
    #[serde(rename = "get-automation-nodes")]
    GetAutomationNodes,
    #[serde(rename = "get-automation-context")]
    GetAutomationContext,
    #[serde(rename = "save-automation-workflow")]
    SaveAutomationWorkflow { graph: Value },
    #[serde(rename = "delete-automation-workflow")]
    DeleteAutomationWorkflow { id: String },
    #[serde(rename = "set-automation-workflow-enabled")]
    SetAutomationWorkflowEnabled { id: String, enabled: bool },
    #[serde(rename = "analyze-automation-script")]
    AnalyzeAutomationScript {
        #[serde(rename = "nodeId")]
        node_id: String,
        source: String,
        offset: u64,
        #[serde(default, rename = "eventType")]
        event_type: Option<String>,
    },
    #[serde(rename = "get-gift-catalog")]
    GetGiftCatalog,
    #[serde(rename = "get-behavior")]
    GetBehavior,
    #[serde(rename = "save-action")]
    SaveAction { action: Value },
    #[serde(rename = "delete-action")]
    DeleteAction { id: String },
    #[serde(rename = "set-action-enabled")]
    SetActionEnabled { id: String, enabled: bool },
    #[serde(rename = "test-action")]
    TestAction {
        action: Value,
        #[serde(default)]
        trigger: Option<String>,
    },
    #[serde(rename = "save-event")]
    SaveEvent { event: Value },
    #[serde(rename = "delete-event")]
    DeleteEvent { id: String },
    #[serde(rename = "set-event-enabled")]
    SetEventEnabled { id: String, enabled: bool },
    #[serde(rename = "test-event")]
    TestEvent { event: Value },
    // Logical installed-state toggle only. It never touches the filesystem;
    // real `.plugin` package installation uses `install-plugin-package`.
    #[serde(rename = "set-plugin-install")]
    SetPluginInstall { id: String, installed: bool },
    // Real package installation. The frontend supplies only the archive path;
    // plugin identity always comes from `plugin.json` via `PluginInstaller`.
    #[serde(rename = "install-plugin-package")]
    InstallPluginPackage {
        path: String,
        #[serde(default, rename = "replaceExisting")]
        replace_existing: bool,
    },
    #[serde(rename = "uninstall-plugin-package")]
    UninstallPluginPackage { id: String },
    #[serde(rename = "set-plugin-enabled")]
    SetPluginEnabled { id: String, enabled: bool },
    #[serde(rename = "get-plugin-settings")]
    GetPluginSettings { id: String },
    #[serde(rename = "save-plugin-settings")]
    SavePluginSettings { id: String, values: JsonObject },
    #[serde(rename = "get-action-options")]
    GetActionOptions { source: String },
}

impl PageMessage {
    pub fn parse(raw: &str) -> Result<Self, IpcMessageError> {
        if raw.len() > 2 * 1024 * 1024 {
            return Err(IpcMessageError::TooLarge);
        }
        let message: Self = serde_json::from_str(raw)?;
        message.validate()?;
        Ok(message)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Connect { .. } => "connect",
            Self::PickLive { .. } => "pick-live",
            Self::OpenMediaPicker { .. } => "open-media-picker",
            Self::Disconnect => "disconnect",
            Self::GetPointsConfig => "get-points-config",
            Self::UpdatePointsConfig { .. } => "update-points-config",
            Self::GetLeaderboard { .. } => "get-leaderboard",
            Self::ResetPoints { .. } => "reset-points",
            Self::AdjustPoints { .. } => "adjust-points",
            Self::GetCreator { .. } => "get-creator",
            Self::GetRecentCreators { .. } => "get-recent-creators",
            Self::GetAppState { .. } => "get-app-state",
            Self::SetAppState { .. } => "set-app-state",
            Self::ClearCreatorHistory => "clear-creator-history",
            Self::DebugGift { .. } => "debug-gift",
            Self::GetAutomationWorkflows => "get-automation-workflows",
            Self::GetAutomationNodes => "get-automation-nodes",
            Self::GetAutomationContext => "get-automation-context",
            Self::SaveAutomationWorkflow { .. } => "save-automation-workflow",
            Self::DeleteAutomationWorkflow { .. } => "delete-automation-workflow",
            Self::SetAutomationWorkflowEnabled { .. } => "set-automation-workflow-enabled",
            Self::AnalyzeAutomationScript { .. } => "analyze-automation-script",
            Self::GetGiftCatalog => "get-gift-catalog",
            Self::GetBehavior => "get-behavior",
            Self::SaveAction { .. } => "save-action",
            Self::DeleteAction { .. } => "delete-action",
            Self::SetActionEnabled { .. } => "set-action-enabled",
            Self::TestAction { .. } => "test-action",
            Self::SaveEvent { .. } => "save-event",
            Self::DeleteEvent { .. } => "delete-event",
            Self::SetEventEnabled { .. } => "set-event-enabled",
            Self::TestEvent { .. } => "test-event",
            Self::SetPluginInstall { .. } => "set-plugin-install",
            Self::InstallPluginPackage { .. } => "install-plugin-package",
            Self::UninstallPluginPackage { .. } => "uninstall-plugin-package",
            Self::SetPluginEnabled { .. } => "set-plugin-enabled",
            Self::GetPluginSettings { .. } => "get-plugin-settings",
            Self::SavePluginSettings { .. } => "save-plugin-settings",
            Self::GetActionOptions { .. } => "get-action-options",
        }
    }

    fn validate(&self) -> Result<(), IpcMessageError> {
        match self {
            Self::Connect {
                unique_id,
                session_cookie,
                room_id,
            } => {
                bounded_string(unique_id, "uniqueId", 256)?;
                bounded_value(session_cookie, "sessionCookie", 256 * 1024)?;
                optional_bounded(room_id.as_deref(), "roomId", 256)?;
            }
            Self::PickLive { session_cookie } => {
                bounded_value(session_cookie, "sessionCookie", 256 * 1024)?
            }
            Self::OpenMediaPicker {
                request_id,
                title,
                initial_directory,
                extensions,
                ..
            } => {
                bounded_string(request_id, "requestId", 128)?;
                if extensions.len() > 32
                    || extensions.iter().any(|extension| {
                        extension.is_empty()
                            || extension.len() > 16
                            || !extension.chars().all(|character| {
                                character.is_ascii_alphanumeric()
                                    || matches!(character, '+' | '-' | '_')
                            })
                    })
                {
                    return Err(IpcMessageError::InvalidField("extensions"));
                }
                optional_bounded(title.as_deref(), "title", 256)?;
                optional_bounded(initial_directory.as_deref(), "initialDirectory", 4_096)?;
            }
            Self::AdjustPoints { unique_id, delta } => {
                bounded_string(unique_id, "uniqueId", 256)?;
                if !delta.is_finite() {
                    return Err(IpcMessageError::InvalidField("delta"));
                }
            }
            Self::UpdatePointsConfig { config } => config.validate()?,
            Self::AnalyzeAutomationScript {
                node_id,
                source,
                offset,
                ..
            } => {
                bounded_string(node_id, "nodeId", 256)?;
                if source.len() > 128 * 1024 || *offset > 128 * 1024 {
                    return Err(IpcMessageError::InvalidField("source/offset"));
                }
            }
            Self::GetActionOptions { source } => {
                if !valid_token(source) || source.len() > 64 {
                    return Err(IpcMessageError::InvalidField("source"));
                }
            }
            Self::SavePluginSettings { id, values } => {
                bounded_string(id, "id", 128)?;
                if values.len() > 32
                    || values
                        .iter()
                        .any(|(key, value)| !valid_setting_key(key) || !is_primitive_setting(value))
                {
                    return Err(IpcMessageError::InvalidField("values"));
                }
            }
            Self::InstallPluginPackage { path, .. } => {
                // Never trust a frontend-supplied plugin id or destination: only
                // the canonical path to the `.plugin` archive is accepted here.
                // Identity always comes from `plugin.json` inside the installer.
                if path.is_empty() || path.len() > MAX_PLUGIN_PACKAGE_PATH_LEN {
                    return Err(IpcMessageError::InvalidField("path"));
                }
                if path.contains('\0') {
                    return Err(IpcMessageError::InvalidField("path"));
                }
            }
            Self::UninstallPluginPackage { id } => bounded_string(id, "id", 128)?,
            _ => {}
        }
        Ok(())
    }
}

impl PartialPointsConfig {
    fn validate(&self) -> Result<(), IpcMessageError> {
        for value in [
            self.points_per_coin,
            self.points_per_share,
            self.points_per_chat,
            self.points_per_like,
            self.points_per_follow,
            self.points_per_join,
            self.sub_bonus_multiplier,
            self.points_per_level,
        ] {
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(IpcMessageError::InvalidField("points config"));
            }
        }
        Ok(())
    }
}

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

/// Machine-readable plugin installation failure reason. The frontend replace
/// flow must depend on `code`, never on substring-matching `error`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PluginInstallErrorCode {
    #[serde(rename = "already-installed")]
    AlreadyInstalled,
    #[serde(rename = "invalid-package")]
    InvalidPackage,
    #[serde(rename = "incompatible")]
    Incompatible,
    #[serde(rename = "io-error")]
    IoError,
    #[serde(rename = "unknown")]
    Unknown,
}

/// Classifies installer failures at the IPC boundary so the UI does not parse
/// arbitrary Rust error strings. Only `already-installed` drives the replace
/// confirmation; every other failure is shown as a plain error.
pub fn classify_plugin_install_error(message: &str) -> PluginInstallErrorCode {
    let lower = message.to_ascii_lowercase();
    if lower.contains("already installed") {
        return PluginInstallErrorCode::AlreadyInstalled;
    }
    if lower.contains("compatib")
        || lower.contains("protocolversion")
        || lower.contains("protocol version")
        || lower.contains("abi")
        || lower.contains("unsupported schema")
        || lower.contains("schema version")
        || lower.contains("signature")
        || lower.contains("requires ")
        || lower.contains("no build for this platform")
    {
        return PluginInstallErrorCode::Incompatible;
    }
    if lower.contains("permission")
        || lower.contains("read-only")
        || lower.contains("read only")
        || lower.contains("disk")
        || lower.contains("os error")
    {
        return PluginInstallErrorCode::IoError;
    }
    if lower.contains("archive")
        || lower.contains("checksum")
        || lower.contains("manifest")
        || lower.contains("plugin.json")
        || lower.contains("extension")
        || lower.contains("traversal")
        || lower.contains("symlink")
        || lower.contains("symbolic link")
        || lower.contains("unsafe")
        || lower.contains("duplicate")
        || lower.contains("too many")
        || lower.contains("exceeds")
        || lower.contains("invalid")
        || lower.contains("missing")
        || lower.contains("mismatch")
        || lower.contains("not a file")
        || lower.contains("not valid")
        || lower.contains("could not")
    {
        return PluginInstallErrorCode::InvalidPackage;
    }
    PluginInstallErrorCode::Unknown
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
    ActionOptions { source: String, options: Vec<Value> },
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

#[derive(Debug, Error)]
pub enum IpcMessageError {
    #[error("IPC message is larger than the 2 MB limit")]
    TooLarge,
    #[error("IPC message is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IPC message field `{0}` is invalid")]
    InvalidField(&'static str),
}

impl fmt::Display for PageMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.type_name())
    }
}

fn bounded_string(value: &str, _field: &'static str, max: usize) -> Result<(), IpcMessageError> {
    if value.is_empty() || value.len() > max {
        return Err(IpcMessageError::InvalidField(_field));
    }
    Ok(())
}

fn bounded_value(value: &str, field: &'static str, max: usize) -> Result<(), IpcMessageError> {
    if value.len() > max {
        return Err(IpcMessageError::InvalidField(field));
    }
    Ok(())
}

fn optional_bounded(
    value: Option<&str>,
    field: &'static str,
    max: usize,
) -> Result<(), IpcMessageError> {
    if let Some(value) = value {
        bounded_string(value, field, max)?;
    }
    Ok(())
}

fn valid_token(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '_' | '-')
        })
}

fn valid_setting_key(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z' | 'A'..='Z'))
        && value.len() <= 64
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn is_primitive_setting(value: &Value) -> bool {
    match value {
        Value::String(value) => value.len() <= 4_096,
        Value::Number(_) | Value::Bool(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_existing_camel_case_contract() {
        let message = PageMessage::parse(
            r#"{"type":"connect","uniqueId":"creator","sessionCookie":"sid","roomId":"room"}"#,
        )
        .unwrap();
        assert_eq!(message.type_name(), "connect");
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("uniqueId"));
        assert!(json.contains("sessionCookie"));
    }

    #[test]
    fn rejects_unknown_discriminator_and_bad_settings() {
        assert!(PageMessage::parse(r#"{"type":"not-a-message"}"#).is_err());
        assert!(PageMessage::parse(
            r#"{"type":"save-plugin-settings","id":"demo","values":{"nested":{}}}"#,
        )
        .is_err());
        assert!(PageMessage::parse(&format!(
            r#"{{"type":"save-plugin-settings","id":"demo","values":{{"value":"{}"}}}}"#,
            "x".repeat(4_097)
        ))
        .is_err());
    }

    #[test]
    fn host_message_has_wire_type() {
        let json = HostMessage::PointsConfig {
            config: PointsConfig::default(),
        }
        .to_json()
        .unwrap();
        assert!(json.starts_with(r#"{"type":"points-config""#));
        assert!(json.contains("pointsPerCoin"));
    }

    #[test]
    fn media_picker_preserves_the_public_json_contract() {
        let message = PageMessage::parse(
            r#"{"type":"open-media-picker","requestId":"media-1","mode":"file","kind":"audio","extensions":["wav","mp3"]}"#,
        )
        .unwrap();
        assert_eq!(message.type_name(), "open-media-picker");
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("requestId"));
        assert!(!json.contains("initialDirectory"));

        let response = HostMessage::MediaSelected {
            request_id: "media-1".to_owned(),
            selection: None,
            error: None,
        }
        .to_json()
        .unwrap();
        assert_eq!(
            response,
            r#"{"type":"media-selected","requestId":"media-1"}"#
        );
    }

    #[test]
    fn install_plugin_package_round_trips_and_validates_path() {
        let message = PageMessage::parse(
            r#"{"type":"install-plugin-package","path":"C:\\Temp\\demo.plugin","replaceExisting":false}"#,
        )
        .unwrap();
        assert_eq!(message.type_name(), "install-plugin-package");
        match message {
            PageMessage::InstallPluginPackage {
                path,
                replace_existing,
            } => {
                assert_eq!(path, r"C:\Temp\demo.plugin");
                assert!(!replace_existing);
            }
            other => panic!("unexpected message: {other:?}"),
        }
        let with_replace = PageMessage::parse(
            r#"{"type":"install-plugin-package","path":"/tmp/demo.plugin","replaceExisting":true}"#,
        )
        .unwrap();
        match with_replace {
            PageMessage::InstallPluginPackage {
                replace_existing, ..
            } => assert!(replace_existing),
            other => panic!("unexpected message: {other:?}"),
        }
        // replaceExisting defaults to false so the first install never
        // silently replaces an existing plugin.
        let defaulted =
            PageMessage::parse(r#"{"type":"install-plugin-package","path":"/tmp/demo.plugin"}"#)
                .unwrap();
        match defaulted {
            PageMessage::InstallPluginPackage {
                replace_existing, ..
            } => assert!(!replace_existing),
            other => panic!("unexpected message: {other:?}"),
        }

        assert!(PageMessage::parse(r#"{"type":"install-plugin-package","path":""}"#).is_err());
        assert!(PageMessage::parse(r#"{"type":"install-plugin-package","path":"a\0b"}"#,).is_err());
        let oversized = "x".repeat(MAX_PLUGIN_PACKAGE_PATH_LEN + 1);
        assert!(PageMessage::parse(&format!(
            r#"{{"type":"install-plugin-package","path":"{oversized}"}}"#
        ))
        .is_err());
    }

    #[test]
    fn plugin_install_result_uses_structured_codes() {
        let success =
            HostMessage::plugin_install_success("demo".to_owned(), "1.0.0".to_owned(), false)
                .to_json()
                .unwrap();
        assert!(success.contains(r#""type":"plugin-install-result""#));
        assert!(success.contains(r#""success":true"#));
        assert!(success.contains(r#""id":"demo""#));

        let failure = HostMessage::plugin_install_failure(
            PluginInstallErrorCode::AlreadyInstalled,
            "plugin is already installed: demo".to_owned(),
        )
        .to_json()
        .unwrap();
        assert!(failure.contains(r#""success":false"#));
        assert!(failure.contains("already-installed"));

        assert_eq!(
            classify_plugin_install_error("plugin is already installed: demo"),
            PluginInstallErrorCode::AlreadyInstalled
        );
        assert_eq!(
            classify_plugin_install_error("plugin manifest field `id` is invalid"),
            PluginInstallErrorCode::InvalidPackage
        );
        assert_eq!(
            classify_plugin_install_error("plugin manifest field `protocolVersion` is invalid"),
            PluginInstallErrorCode::Incompatible
        );
        assert_eq!(
            classify_plugin_install_error("checksum mismatch in demo: index.js"),
            PluginInstallErrorCode::InvalidPackage
        );
    }

    #[test]
    fn plugin_progress_message_preserves_optional_progress() {
        let message = HostMessage::PluginProgress {
            plugin_id: "sonicboom.tts".to_owned(),
            state: PluginProgressState::Downloading,
            progress: Some(0.5),
            message: "Downloading model: 50%.".to_owned(),
        }
        .to_json()
        .unwrap();
        assert_eq!(
            message,
            r#"{"type":"plugin-progress","pluginId":"sonicboom.tts","state":"downloading","progress":0.5,"message":"Downloading model: 50%."}"#
        );
    }

    #[test]
    fn uninstall_plugin_package_has_a_dedicated_wire_contract() {
        let message =
            PageMessage::parse(r#"{"type":"uninstall-plugin-package","id":"demo"}"#).unwrap();
        assert_eq!(message.type_name(), "uninstall-plugin-package");
        assert!(matches!(message, PageMessage::UninstallPluginPackage { id } if id == "demo"));

        let success = HostMessage::plugin_uninstall_success("demo".to_owned())
            .to_json()
            .unwrap();
        assert_eq!(
            success,
            r#"{"type":"plugin-uninstall-result","success":true,"id":"demo"}"#
        );
    }
}
