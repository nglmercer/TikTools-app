mod catalog;
mod installation;
mod inventory;
mod jobs;
mod live_status;
mod registry;
mod rollback;
mod updates;
mod verification;

#[cfg(test)]
use catalog::load_catalog_versions;
use catalog::registry_package_live;
#[cfg(test)]
use installation::product_package_spec;
use live_status::{classify_live_releases, installed_live_status};
#[cfg(test)]
use updates::bounded_update_query;
use verification::verify_current_cloud_release;
#[cfg(test)]
use verification::{validate_current_cloud_release, verify_catalog_release};

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_installer::{
    update_for_document, CommitOptions, Consent, InstallError, InstallErrorCode, InstallOutcome,
    InstallReceipt, InstalledVersions, JobHandle, JobKind, JobReporter, JobSnapshot, JobStage,
    PackageOrigin, PackagePreview, PackageSpec, PackageSpecInput, PreparedPackage, UpdateInfo,
    UpdatePolicy, UpdateReport,
};
use tiktools_plugin_registry::{
    Advisory, AutoUpdate, LiveReleaseStatus, PackageDocument, RegistryDefaults, RegistrySource,
    RegistryStatus, SearchQuery, SearchResults, OFFICIAL_REGISTRY_NAME,
};

use super::OperationError;
use crate::{
    events::DomainEvent,
    plugin_packages::{CoreActivation, PluginPackages},
    AppCore,
};

#[cfg(test)]
#[path = "sideload_policy_tests.rs"]
mod sideload_policy_tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PolicyPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<AutoUpdate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AvailableRelease {
    pub version: String,
    pub channel: String,
    pub published_at: u64,
    pub yanked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advisory: Option<Advisory>,
    pub compatible: bool,
    #[serde(default)]
    pub live_status: LiveReleaseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginVersions {
    #[serde(flatten)]
    pub installed: InstalledVersions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    pub available: Vec<AvailableRelease>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRootKey {
    pub key_id: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRootPreview {
    pub url: String,
    pub keys: Vec<RegistryRootKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryAddInput {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub fingerprints: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPackageView {
    pub registry: String,
    pub document: PackageDocument,
    #[serde(default)]
    pub live_statuses: BTreeMap<String, LiveReleaseStatus>,
}

enum InstallConsent {
    Reviewed(String),
    Automatic,
}

impl AppCore {
    fn packages(&self) -> Result<Arc<PluginPackages>, OperationError> {
        self.plugin_packages()
            .cloned()
            .map_err(OperationError::install)
    }

    pub(crate) fn after_package_change(&self, outcome: &InstallOutcome) {
        self.start_enabled_event_subscribers();
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        let plugin_id = outcome.receipt.id.clone();
        self.events.publish_domain(match &outcome.previous_version {
            None => DomainEvent::PluginInstalled { plugin_id },
            Some(from) => DomainEvent::PluginUpdated {
                plugin_id,
                from_version: Some(from.clone()),
                to_version: outcome.receipt.version.clone(),
            },
        });
    }

    pub(crate) fn adopt_user_plugin_packages(&self) {
        let Ok(packages) = self.packages() else {
            return;
        };
        let user_plugins: Vec<String> = self
            .plugins
            .list()
            .into_iter()
            .filter(|plugin| plugin.source == tiktools_plugin_loader::PluginSource::User)
            .map(|plugin| plugin.manifest.id)
            .collect();
        match packages
            .installer
            .adopt(user_plugins.iter().map(String::as_str))
        {
            Ok(adopted) if !adopted.is_empty() => {
                tracing::info!(count = adopted.len(), "adopted unmanaged plugin packages")
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "plugin package adoption failed"),
        }
    }
}

#[derive(Debug, Clone)]
struct LiveArtifactStatus {
    status: LiveReleaseStatus,
    targets: BTreeMap<String, LiveTargetStatus>,
}

#[derive(Debug, Clone)]
struct LiveTargetStatus {
    status: LiveReleaseStatus,
    digest: String,
    key_id: Option<String>,
}

fn registry_error(error: tiktools_plugin_registry::RegistryError) -> OperationError {
    OperationError::install(InstallError::from(error))
}

fn registry_package_error(error: tiktools_plugin_package::PackageError) -> OperationError {
    OperationError::install(InstallError::from(error))
}

#[cfg(test)]
mod fixed_source_tests;
