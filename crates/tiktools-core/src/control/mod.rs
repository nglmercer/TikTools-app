//! Headless operations API: the single authoritative operation path.
//!
//! Every client (WebView IPC, CLI, JSON stdio, local IPC, tests, agents) is
//! expected to go through these value-returning operations. Unlike the
//! `PageMessage` handlers, these methods never emit [`HostMessage`]s; the
//! WebView layer emits UI messages after calling the same operations.
//!
//! [`HostMessage`]: crate::ipc::messages::HostMessage

mod analytics;
mod automation;
mod helpers;
mod live;
mod media;
mod plugin_settings;
mod plugins;
mod points;
mod processors;
mod system;
mod widgets;
mod workflows;

pub use crate::input_access::InputAccessResult;
pub use analytics::GiftDebugResult;
pub use automation::{AutomationKind, ScriptAnalysisResult};
#[cfg(any(test, feature = "native-tiktok"))]
pub(crate) use helpers::check_session_cookie_len;
pub(crate) use helpers::{clean_plugin_id, clean_record_id, fresh_record_id};
pub use live::LiveStatus;
pub use plugin_settings::{PluginConnectionResult, PluginProvisionResult, PluginSettingsResult};
pub use plugins::{PluginActionOutcome, PluginInstallResult};
pub use processors::ProcessorOutcomeDto;
pub use system::{DoctorCheck, DoctorReport};
pub use widgets::{WidgetsCopyResult, WidgetsState, WidgetsStatusResult, GATEWAY_PLUGIN_ID};

/// Machine-readable operation failure. The control API maps this 1:1 onto
/// the JSON-RPC error envelope (`code` + `message`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationError {
    pub code: &'static str,
    pub message: String,
}

impl OperationError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found",
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_params",
            message: message.into(),
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "unavailable",
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "conflict",
            message: message.into(),
        }
    }

    /// Authenticated caller acts outside its own plugin scope (cross-plugin
    /// action invocation or option-source read). The owner check resolves
    /// through the action catalog, never through name prefixes.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            code: "forbidden",
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            code: "timeout",
            message: message.into(),
        }
    }

    /// Desktop-only capability (native dialogs, audio output) requested on a
    /// headless host. Callers must not emulate the capability.
    pub fn capability_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "capability_unavailable",
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal",
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for OperationError {}

impl From<tiktools_plugin_loader::PluginLoaderError> for OperationError {
    fn from(error: tiktools_plugin_loader::PluginLoaderError) -> Self {
        match &error {
            tiktools_plugin_loader::PluginLoaderError::NotFound(_) => {
                Self::not_found(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::RuntimeUnavailable(_) => {
                Self::unavailable(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::Timeout(_) => {
                Self::timeout(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::Manifest(_)
            | tiktools_plugin_loader::PluginLoaderError::InvalidDirectory(_)
            | tiktools_plugin_loader::PluginLoaderError::Runtime(_)
            | tiktools_plugin_loader::PluginLoaderError::LockPoisoned(_) => {
                Self::internal(error.to_string())
            }
        }
    }
}
