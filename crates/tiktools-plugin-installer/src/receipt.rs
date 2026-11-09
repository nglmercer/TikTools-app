use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::manifest::is_valid_plugin_id;
use tiktools_plugin_registry::{AutoUpdate, CHANNEL_STABLE};

use crate::InstallError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PackageOrigin {
    Registry { registry: String, url: String },
    Url { url: String },
    File,
    Builtin,
    Development,
    Adopted,
}

impl PackageOrigin {
    pub fn is_registry(&self) -> bool {
        matches!(self, Self::Registry { .. })
    }

    pub fn registry_name(&self) -> Option<&str> {
        match self {
            Self::Registry { registry, .. } => Some(registry),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SignatureStatus {
    Verified,
    Unsigned,
    Untrusted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignatureState {
    pub status: SignatureStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl SignatureState {
    pub fn unsigned() -> Self {
        Self {
            status: SignatureStatus::Unsigned,
            publisher: None,
            key_id: None,
            source: None,
            reason: None,
        }
    }

    pub fn is_verified(&self) -> bool {
        self.status == SignatureStatus::Verified
    }

    pub(crate) fn consent_label(&self) -> String {
        match self.status {
            SignatureStatus::Verified => format!(
                "verified:{}:{}",
                self.publisher.as_deref().unwrap_or_default(),
                self.key_id.as_deref().unwrap_or_default()
            ),
            SignatureStatus::Unsigned => "unsigned".to_owned(),
            SignatureStatus::Untrusted => {
                format!("untrusted:{}", self.key_id.as_deref().unwrap_or_default())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePolicy {
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub auto_update: AutoUpdate,
    #[serde(default)]
    pub pinned: bool,
}

impl Default for UpdatePolicy {
    fn default() -> Self {
        Self {
            channel: default_channel(),
            auto_update: AutoUpdate::default(),
            pinned: false,
        }
    }
}

fn default_channel() -> String {
    CHANNEL_STABLE.to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetainedVersion {
    pub version: String,
    pub retained_at: u64,
}

/// Artifact provenance recorded by the trusted Cloud install path after its
/// signed metadata and current generation have been verified. Older receipts
/// intentionally have no such proof and cannot infer it from today’s index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudArtifactIdentity {
    pub target: String,
    /// Decimal string preserves backend generation precision across clients.
    pub generation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstallReceipt {
    pub id: String,
    pub version: String,
    pub origin: PackageOrigin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_artifact: Option<CloudArtifactIdentity>,
    pub signature: SignatureState,
    pub runtime: String,
    #[serde(default)]
    pub granted_permissions: Vec<String>,
    #[serde(default)]
    pub granted_capabilities: Vec<String>,
    pub installed_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub policy: UpdatePolicy,
    #[serde(default)]
    pub retained: Vec<RetainedVersion>,
}

#[derive(Debug, Clone)]
pub struct ReceiptStore {
    directory: PathBuf,
}

impl ReceiptStore {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub fn load(&self, plugin_id: &str) -> Result<Option<InstallReceipt>, InstallError> {
        if !is_valid_plugin_id(plugin_id) {
            return Ok(None);
        }
        read_receipt(&self.path(plugin_id))
    }

    pub fn save(&self, receipt: &InstallReceipt) -> Result<(), InstallError> {
        write_receipt(&self.path(&receipt.id), receipt)
    }

    pub fn remove(&self, plugin_id: &str) -> Result<(), InstallError> {
        match fs::remove_file(self.path(plugin_id)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn list(&self) -> Result<Vec<InstallReceipt>, InstallError> {
        let entries = match fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut receipts = Vec::new();
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            match read_receipt(&path) {
                Ok(Some(receipt)) => receipts.push(receipt),
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "skipping unreadable plugin receipt")
                }
            }
        }
        receipts.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(receipts)
    }

    fn path(&self, plugin_id: &str) -> PathBuf {
        self.directory.join(format!("{plugin_id}.json"))
    }
}

pub(crate) fn read_receipt(path: &Path) -> Result<Option<InstallReceipt>, InstallError> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| InstallError::io(format!("{}: {error}", path.display()))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn write_receipt(path: &Path, receipt: &InstallReceipt) -> Result<(), InstallError> {
    let mut bytes = serde_json::to_vec_pretty(receipt).map_err(InstallError::io)?;
    bytes.push(b'\n');
    write_atomic(path, &bytes)?;
    Ok(())
}

/// Use an unpredictable, exclusively-created sibling and rename it into
/// place. In particular, never open a pre-existing destination symlink.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), InstallError> {
    let parent = path
        .parent()
        .ok_or_else(|| InstallError::io("missing parent directory"))?;
    if let Err(error) = fs::symlink_metadata(parent) {
        if error.kind() == io::ErrorKind::NotFound {
            fs::create_dir_all(parent)?;
        } else {
            return Err(error.into());
        }
    }
    if !fs::symlink_metadata(parent)?.file_type().is_dir() {
        return Err(InstallError::io(
            "package metadata parent is not a real directory",
        ));
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary
        .persist(path)
        .map_err(|error| InstallError::io(error.error))?;
    Ok(())
}
