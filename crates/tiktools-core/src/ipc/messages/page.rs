use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_plugin_api::{MediaKind, MediaPickerMode};

use super::{
    install::MAX_PLUGIN_PACKAGE_PATH_LEN,
    points::PartialPointsConfig,
    validation::{
        bounded_string, bounded_value, is_primitive_setting, optional_bounded, valid_setting_key,
        valid_token, IpcMessageError,
    },
    JsonObject,
};

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
    #[serde(rename = "test-processor")]
    TestProcessor {
        #[serde(rename = "pluginId")]
        plugin_id: String,
        #[serde(rename = "processorId")]
        processor_id: String,
        event: Value,
    },
    #[serde(rename = "get-processor-status")]
    GetProcessorStatus,
    #[serde(rename = "get-analytics-summary")]
    GetAnalyticsSummary {
        #[serde(default, rename = "creatorUniqueId")]
        creator_unique_id: Option<String>,
        #[serde(default, rename = "startDay")]
        start_day: Option<i64>,
        #[serde(default, rename = "endDay")]
        end_day: Option<i64>,
        #[serde(default)]
        limit: Option<i64>,
    },
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
            Self::TestProcessor { .. } => "test-processor",
            Self::GetProcessorStatus => "get-processor-status",
            Self::GetAnalyticsSummary { .. } => "get-analytics-summary",
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
            Self::TestProcessor {
                plugin_id,
                processor_id,
                event,
            } => {
                bounded_string(plugin_id, "pluginId", 128)?;
                bounded_string(processor_id, "processorId", 128)?;
                if !event.is_object()
                    || serde_json::to_vec(event)
                        .map(|bytes| bytes.len() > 256 * 1024)
                        .unwrap_or(true)
                {
                    return Err(IpcMessageError::InvalidField("event"));
                }
            }
            Self::GetAnalyticsSummary {
                creator_unique_id: Some(creator),
                ..
            } => {
                bounded_string(creator, "creatorUniqueId", 256)?;
            }
            Self::GetAnalyticsSummary { .. } => {}
            _ => {}
        }
        Ok(())
    }
}
impl fmt::Display for PageMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.type_name())
    }
}
