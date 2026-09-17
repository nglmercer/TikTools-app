use serde_json::Value;
use tiktools_core::control::OperationError;

/// RPC-facing error: `{ code, message, data? }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub data: Option<Value>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new("method_not_found", format!("unknown method `{method}`"))
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new("invalid_params", message)
    }

    pub fn timeout() -> Self {
        Self::new("timeout", "request timed out")
    }

    pub fn too_large() -> Self {
        Self::new("too_large", "request exceeds the size limit")
    }

    pub fn capability_unavailable(message: impl Into<String>) -> Self {
        Self::new("capability_unavailable", message)
    }

    pub fn host_unavailable() -> Self {
        Self::new(
            "host_unavailable",
            "TikTools control host is not running.",
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }

    /// Re-scopes a generic `not_found` to a domain code such as
    /// `plugin_not_found`. Other codes pass through untouched.
    pub fn scoped_not_found(self, code: &str) -> Self {
        if self.code == "not_found" {
            Self {
                code: code.to_owned(),
                ..self
            }
        } else {
            self
        }
    }
}

impl From<OperationError> for ApiError {
    fn from(error: OperationError) -> Self {
        Self::new(error.code(), error.message().to_owned())
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}
