use serde::{Deserialize, Serialize};

pub const MAX_PLUGIN_PACKAGE_PATH_LEN: usize = 32_767;

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

impl PluginInstallErrorCode {
    pub fn from_install_code(code: &str) -> Self {
        match code {
            "already-installed" => Self::AlreadyInstalled,
            "incompatible" | "no-compatible-release" => Self::Incompatible,
            "io-error" => Self::IoError,
            "invalid-package"
            | "digest-mismatch"
            | "signature-required"
            | "signature-invalid"
            | "untrusted-publisher"
            | "revoked"
            | "downgrade-blocked" => Self::InvalidPackage,
            _ => Self::Unknown,
        }
    }
}
