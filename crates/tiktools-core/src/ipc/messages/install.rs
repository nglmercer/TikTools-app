use serde::{Deserialize, Serialize};

/// Sensible upper bound for a native `.plugin` archive path. Windows extended
/// paths max out at 32_767 characters; the 2 MB IPC envelope is the other
/// bound. The installer itself remains responsible for canonicalization.
pub const MAX_PLUGIN_PACKAGE_PATH_LEN: usize = 32_767;
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
