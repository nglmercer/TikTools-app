use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_plugin_api::Incompatibility;
use tiktools_plugin_package::{PackageError, PackageErrorKind};
use tiktools_plugin_registry::RegistryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InstallErrorCode {
    AlreadyInstalled,
    NotInstalled,
    NotFound,
    NoCompatibleRelease,
    DownloadFailed,
    DigestMismatch,
    SignatureRequired,
    SignatureInvalid,
    UntrustedPublisher,
    Revoked,
    InvalidPackage,
    Incompatible,
    ConsentRequired,
    DowngradeBlocked,
    Busy,
    Cancelled,
    IoError,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct InstallError {
    pub code: InstallErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl InstallError {
    pub fn new(code: InstallErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn io(error: impl std::fmt::Display) -> Self {
        Self::new(InstallErrorCode::IoError, error.to_string())
    }

    pub fn busy(plugin_id: &str) -> Self {
        Self::new(
            InstallErrorCode::Busy,
            format!("another operation is already running for plugin `{plugin_id}`"),
        )
    }

    pub fn cancelled() -> Self {
        Self::new(InstallErrorCode::Cancelled, "the operation was cancelled")
    }

    pub fn not_installed(plugin_id: &str) -> Self {
        Self::new(
            InstallErrorCode::NotInstalled,
            format!("plugin `{plugin_id}` is not installed in the user plugin directory"),
        )
    }
}

impl From<PackageError> for InstallError {
    fn from(error: PackageError) -> Self {
        let code = match error.kind() {
            PackageErrorKind::InvalidPackage => InstallErrorCode::InvalidPackage,
            PackageErrorKind::Incompatible => InstallErrorCode::Incompatible,
            PackageErrorKind::Signature => InstallErrorCode::SignatureInvalid,
            PackageErrorKind::UntrustedPublisher => InstallErrorCode::UntrustedPublisher,
            PackageErrorKind::Revoked => InstallErrorCode::Revoked,
            PackageErrorKind::Io => InstallErrorCode::IoError,
        };
        Self::new(code, error.to_string())
    }
}

impl From<Incompatibility> for InstallError {
    fn from(incompatibility: Incompatibility) -> Self {
        let details = serde_json::to_value(&incompatibility).ok();
        Self {
            code: InstallErrorCode::Incompatible,
            message: incompatibility.to_string(),
            details,
        }
    }
}

impl From<RegistryError> for InstallError {
    fn from(error: RegistryError) -> Self {
        let message = error.to_string();
        match error {
            RegistryError::NotFound(_) | RegistryError::UnknownRegistry(_) => {
                Self::new(InstallErrorCode::NotFound, message)
            }
            RegistryError::Status { status: 404, .. } => {
                Self::new(InstallErrorCode::NotFound, message)
            }
            RegistryError::NoCompatibleRelease { rejections, .. } => {
                Self::new(InstallErrorCode::NoCompatibleRelease, message)
                    .with_details(serde_json::json!({ "rejections": rejections }))
            }
            RegistryError::DigestMismatch { .. } | RegistryError::SizeMismatch { .. } => {
                Self::new(InstallErrorCode::DigestMismatch, message)
            }
            RegistryError::Cancelled => Self::cancelled(),
            RegistryError::Signature { source, .. } => {
                let mut error = InstallError::from(source);
                error.message = message;
                error
            }
            RegistryError::Package(error) => error.into(),
            RegistryError::Io(_) => Self::new(InstallErrorCode::IoError, message),
            RegistryError::InsecureUrl(_)
            | RegistryError::InvalidName(_)
            | RegistryError::DuplicateRegistry(_) => {
                Self::new(InstallErrorCode::Unsupported, message)
            }
            _ => Self::new(InstallErrorCode::DownloadFailed, message),
        }
    }
}

impl From<std::io::Error> for InstallError {
    fn from(error: std::io::Error) -> Self {
        Self::io(error)
    }
}
