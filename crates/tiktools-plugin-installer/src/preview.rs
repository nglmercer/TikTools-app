use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::{
    manifest::parse_version, Incompatibility, PackageMetadata, PluginManifest, PluginSecurityModel,
};
use tiktools_plugin_package::Sha256Digest;

use crate::{PackageOrigin, SignatureState, SignatureStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Install,
    Update,
    Reinstall,
    Downgrade,
}

impl ChangeKind {
    pub fn between(installed: Option<&str>, incoming: &str) -> Self {
        let Some(installed) = installed else {
            return Self::Install;
        };
        match (parse_version(installed), parse_version(incoming)) {
            (Some(current), Some(next)) if next > current => Self::Update,
            (Some(current), Some(next)) if next < current => Self::Downgrade,
            (Some(_), Some(_)) => Self::Reinstall,
            _ if installed == incoming => Self::Reinstall,
            _ => Self::Update,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Update => "update",
            Self::Reinstall => "reinstall",
            Self::Downgrade => "downgrade",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDiff {
    pub added_permissions: Vec<String>,
    pub removed_permissions: Vec<String>,
    pub added_capabilities: Vec<String>,
    pub removed_capabilities: Vec<String>,
}

impl PermissionDiff {
    pub fn between(
        granted_permissions: &[String],
        granted_capabilities: &[String],
        manifest: &PluginManifest,
    ) -> Self {
        let (added_permissions, removed_permissions) =
            difference(granted_permissions, &manifest.permissions);
        let (added_capabilities, removed_capabilities) =
            difference(granted_capabilities, &manifest.capabilities);
        Self {
            added_permissions,
            removed_permissions,
            added_capabilities,
            removed_capabilities,
        }
    }

    pub fn adds_access(&self) -> bool {
        !self.added_permissions.is_empty() || !self.added_capabilities.is_empty()
    }
}

fn difference(before: &[String], after: &[String]) -> (Vec<String>, Vec<String>) {
    let before: BTreeSet<_> = before.iter().collect();
    let after: BTreeSet<_> = after.iter().collect();
    (
        after
            .difference(&before)
            .map(|value| (*value).clone())
            .collect(),
        before
            .difference(&after)
            .map(|value| (*value).clone())
            .collect(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SecurityModel {
    Trusted,
    Isolated,
    Sandboxed,
}

impl From<PluginSecurityModel> for SecurityModel {
    fn from(model: PluginSecurityModel) -> Self {
        match model {
            PluginSecurityModel::Trusted => Self::Trusted,
            PluginSecurityModel::Isolated => Self::Isolated,
            PluginSecurityModel::Sandboxed => Self::Sandboxed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackagePreview {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub runtime: String,
    pub security_model: SecurityModel,
    pub permissions: Vec<String>,
    pub capabilities: Vec<String>,
    pub size: u64,
    pub digest: String,
    pub origin: PackageOrigin,
    pub signature: SignatureState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<serde_json::Value>")]
    pub incompatibility: Option<Incompatibility>,
    pub change: ChangeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    pub permission_diff: PermissionDiff,
    pub requires_sideload_consent: bool,
    #[schemars(with = "serde_json::Value")]
    pub metadata: PackageMetadata,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
    pub consent_token: String,
}

impl PackagePreview {
    pub fn is_installable(&self) -> bool {
        self.incompatibility.is_none()
    }
}

pub(crate) fn consent_token(
    manifest: &PluginManifest,
    digest: &Sha256Digest,
    signature: &SignatureState,
    change: ChangeKind,
) -> String {
    let mut permissions = manifest.permissions.clone();
    permissions.sort();
    let mut capabilities = manifest.capabilities.clone();
    capabilities.sort();
    let material = [
        "tiktools-consent-v1".to_owned(),
        manifest.id.clone(),
        manifest.version.clone(),
        digest.to_string(),
        manifest.runtime.to_string(),
        signature.consent_label(),
        change.label().to_owned(),
        permissions.join(","),
        capabilities.join(","),
    ]
    .join("\n");
    Sha256Digest::of_bytes(material.as_bytes()).to_string()
}

pub(crate) fn requires_sideload_consent(
    origin: &PackageOrigin,
    signature: &SignatureState,
) -> bool {
    !origin.is_registry() && signature.status != SignatureStatus::Verified
}
