use thiserror::Error;

/// Errors exposed to plugin authors. Protocol adapters deliberately serialize
/// only this display text, never a Rust backtrace.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("unsupported action: {0}")]
    UnsupportedAction(String),

    #[error("capability unavailable: {0}")]
    CapabilityUnavailable(String),

    #[error("plugin error: {0}")]
    Other(String),
}

impl PluginError {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest(message.into())
    }

    pub fn unsupported(action: impl Into<String>) -> Self {
        Self::UnsupportedAction(action.into())
    }

    pub fn capability_unavailable(capability: impl Into<String>) -> Self {
        Self::CapabilityUnavailable(capability.into())
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

pub type PluginResult<T> = Result<T, PluginError>;
#[derive(Debug, Error)]
pub enum PluginProtocolError {
    #[error("plugin result must be a JSON object")]
    NotAnObject,
    #[error("plugin result field `{0}` is invalid")]
    InvalidField(&'static str),
    #[error("plugin result field `{field}` is invalid: {message}")]
    InvalidValue {
        field: &'static str,
        message: String,
    },
}
