//! Shared installer configuration, contracts and current-state projection.
//!
//! Child modules operate on this same installer and cooperative store lease.
//! Filesystem transitions, activation recovery and consent checks retain their
//! original ordering; no module creates a separate package lock or receipt store.

mod evidence;
mod installation;
mod lease;
mod review;
mod rollback;
mod storage;
mod uninstall;

use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::{manifest::is_valid_plugin_id, HostCompat, PluginManifest};
use tiktools_plugin_package::{PackageArchive, Sha256Digest, TrustStore, MANIFEST_FILE};

use crate::{
    layout::{now, StoreLayout},
    locks::KeyedLocks,
    ChangeKind, CloudArtifactIdentity, InstallError, InstallReceipt, PackageOrigin, ReceiptStore,
    RetainedVersion, SignatureState, UpdatePolicy,
};

pub const DEFAULT_RETAINED_VERSIONS: usize = 2;

pub trait ActivationHooks: Send + Sync {
    fn deactivate(&self, plugin_id: &str) -> Result<bool, String>;
    fn activate(&self, plugin_id: &str, was_running: bool) -> Result<(), String>;
}

pub struct NoopHooks;

impl ActivationHooks for NoopHooks {
    fn deactivate(&self, _plugin_id: &str) -> Result<bool, String> {
        Ok(false)
    }

    fn activate(&self, _plugin_id: &str, _was_running: bool) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignaturePolicy {
    Require,
    AllowWithConsent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Consent {
    Token(String),
    Implicit { replace_existing: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitOptions {
    pub consent: Consent,
    pub policy: Option<UpdatePolicy>,
}

#[derive(Debug, Clone)]
pub struct InstallerConfig {
    pub plugins_dir: PathBuf,
    pub store_dir: PathBuf,
    pub host: HostCompat,
    pub retain_versions: usize,
    pub provider_endpoints: Option<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct PreparedPackage {
    archive: PackageArchive,
    original_path: PathBuf,
    // Keeps the private copy alive across all clones until the last review ends.
    _snapshot: Arc<tempfile::TempDir>,
    pub origin: PackageOrigin,
    pub digest: Sha256Digest,
    pub manifest: PluginManifest,
    pub signature: SignatureState,
    pub changelog: Option<String>,
    checksums: Option<Vec<u8>>,
    cloud_artifact: Option<CloudArtifactIdentity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstallOutcome {
    pub receipt: InstallReceipt,
    pub directory: String,
    pub change: ChangeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstalledVersions {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<InstallReceipt>,
    pub retained: Vec<RetainedVersion>,
}

/// A fresh observation of the installed tree under a cooperative store lease.
/// This does not prove the original operation's outcome, authorize clearing a
/// journal, or attest that the plugin runtime is stopped or reconciled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstalledPackageEvidence {
    Absent,
    Verified {
        id: String,
        version: String,
        checksums_digest: Sha256Digest,
        publisher: String,
        key_id: String,
        trust_source: String,
    },
}

pub struct Installer {
    layout: StoreLayout,
    host: HostCompat,
    retain: usize,
    provider_endpoints: Option<(String, String)>,
    receipts: ReceiptStore,
    trust: Arc<RwLock<TrustStore>>,
    locks: KeyedLocks,
}

/// Cooperative access through installers sharing either the plugin root or
/// the receipt store root. The lease is tied to this installer and released
/// when dropped. It is not an authorization to install or start a plugin;
/// older binaries and direct filesystem or ReceiptStore writers are not fenced.
/// Only equal canonical roots contend: overlapping ancestor/descendant roots
/// require separate configuration validation before recovery can be enabled.
///
/// Mutations require an exclusive borrow of the lease, preventing callers from
/// using one shared store guard for concurrent package operations or reads.
///
/// ```compile_fail
/// use tiktools_plugin_installer::{ActivationHooks, LockedPackageStore};
/// fn cannot_mutate_through_shared_lease(store: &LockedPackageStore<'_>, hooks: &dyn ActivationHooks) {
///     store.uninstall("demo.plugin", None, hooks).unwrap();
/// }
/// ```
pub struct LockedPackageStore<'a> {
    installer: &'a Installer,
    _operation: crate::locks::PackageOperationLease,
    _store_operation: Option<crate::locks::PackageOperationLease>,
}

impl Installer {
    pub fn new(config: InstallerConfig, trust: Arc<RwLock<TrustStore>>) -> Self {
        let layout = StoreLayout::new(config.plugins_dir, config.store_dir);
        Self {
            receipts: ReceiptStore::new(layout.receipts()),
            layout,
            host: config.host,
            retain: config.retain_versions,
            provider_endpoints: config.provider_endpoints,
            trust,
            locks: KeyedLocks::default(),
        }
    }

    pub fn layout(&self) -> &StoreLayout {
        &self.layout
    }

    pub fn host(&self) -> &HostCompat {
        &self.host
    }

    pub fn trust(&self) -> Arc<RwLock<TrustStore>> {
        self.trust.clone()
    }

    pub fn receipts(&self) -> &ReceiptStore {
        &self.receipts
    }

    pub fn is_busy(&self, plugin_id: &str) -> bool {
        self.locks.is_locked(plugin_id)
    }

    pub fn receipt(&self, plugin_id: &str) -> Result<Option<InstallReceipt>, InstallError> {
        self.receipts.load(plugin_id)
    }

    pub fn installed_manifest(&self, plugin_id: &str) -> Option<PluginManifest> {
        if !is_valid_plugin_id(plugin_id) {
            return None;
        }
        let text = fs::read_to_string(self.layout.package(plugin_id).join(MANIFEST_FILE)).ok()?;
        PluginManifest::from_json_str(&text).ok()
    }

    pub fn cleanup_staging(&self) {
        // Another process may be extracting or rolling back a package. A
        // contended or unsafe lock must leave every staging directory intact.
        let Ok(_operation) =
            crate::locks::try_package_operation(&self.layout.plugins, "package store")
        else {
            tracing::warn!("plugin staging cleanup deferred while another operation is active or the lock is unavailable");
            return;
        };
        let staging = self.layout.staging();
        match fs::symlink_metadata(&staging) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            _ => {
                tracing::warn!("plugin staging cleanup skipped: staging is not a real directory");
                return;
            }
        }
        if let Ok(entries) = fs::read_dir(&staging) {
            for entry in entries.flatten() {
                // A crash or failed restoration may leave the only copy of a
                // removed package here. Never delete it during startup cleanup.
                if entry.file_name().to_string_lossy().starts_with("removed-") {
                    continue;
                }
                // A rollback may have moved the only retained copy into a
                // staging directory before a crash. Without a durable
                // operation journal its contents cannot be classified as
                // disposable; remove only empty directories.
                let _ = fs::remove_dir(entry.path());
            }
        }
    }

    pub fn set_policy(
        &self,
        plugin_id: &str,
        policy: UpdatePolicy,
    ) -> Result<InstallReceipt, InstallError> {
        self.try_lock_store_for(plugin_id)?
            .set_policy(plugin_id, policy)
    }

    fn set_policy_under_store_lock(
        &self,
        plugin_id: &str,
        policy: UpdatePolicy,
    ) -> Result<InstallReceipt, InstallError> {
        let mut receipt = self
            .receipts
            .load(plugin_id)?
            .ok_or_else(|| InstallError::not_installed(plugin_id))?;
        receipt.policy = policy;
        self.receipts.save(&receipt)?;
        Ok(receipt)
    }

    pub fn adopt<'a>(
        &self,
        plugin_ids: impl IntoIterator<Item = &'a str>,
    ) -> Result<Vec<InstallReceipt>, InstallError> {
        let _operation = self.try_lock_store_for("package store")?;
        let mut adopted = Vec::new();
        for plugin_id in plugin_ids {
            if self.receipts.load(plugin_id)?.is_some() {
                continue;
            }
            let Some(manifest) = self.installed_manifest(plugin_id) else {
                continue;
            };
            let receipt = adopted_receipt(&manifest, now());
            self.receipts.save(&receipt)?;
            adopted.push(receipt);
        }
        Ok(adopted)
    }

    fn current_state(&self, plugin_id: &str) -> Result<Option<CurrentState>, InstallError> {
        let directory = self.layout.package(plugin_id);
        if !directory.exists() {
            return Ok(None);
        }
        let manifest = self.installed_manifest(plugin_id);
        let receipt = match (self.receipts.load(plugin_id)?, &manifest) {
            (Some(receipt), Some(manifest)) if receipt.version == manifest.version => receipt,
            (Some(receipt), Some(manifest)) => {
                // The active manifest no longer matches its receipt: do not
                // inherit its origin or signature. Retain only independently
                // identifiable historical entries so recovery remains possible.
                let mut adopted = adopted_receipt(manifest, now());
                if receipt.id == plugin_id && manifest.id == plugin_id {
                    self.ensure_versions_dir(plugin_id, false)?;
                    adopted.retained = receipt
                        .retained
                        .into_iter()
                        .filter(|entry| {
                            let path = self.layout.retained(plugin_id, &entry.version);
                            Self::retained_matches(&path, plugin_id, &entry.version)
                                || (!path.exists()
                                    && self.legacy_retained(plugin_id, &entry.version).is_some_and(
                                        |legacy| {
                                            Self::retained_matches(
                                                &legacy,
                                                plugin_id,
                                                &entry.version,
                                            )
                                        },
                                    ))
                        })
                        .collect();
                }
                adopted
            }
            (Some(receipt), None) => receipt,
            (None, Some(manifest)) => adopted_receipt(manifest, now()),
            (None, None) => InstallReceipt {
                retained: Vec::new(),
                ..adopted_receipt_parts(
                    plugin_id,
                    "unknown",
                    "unknown",
                    Vec::new(),
                    Vec::new(),
                    now(),
                )
            },
        };
        Ok(Some(CurrentState { directory, receipt }))
    }
}

struct CurrentState {
    directory: PathBuf,
    receipt: InstallReceipt,
}

fn adopted_receipt(manifest: &PluginManifest, timestamp: u64) -> InstallReceipt {
    adopted_receipt_parts(
        &manifest.id,
        &manifest.version,
        &manifest.runtime.to_string(),
        manifest.permissions.clone(),
        manifest.capabilities.clone(),
        timestamp,
    )
}

fn adopted_receipt_parts(
    plugin_id: &str,
    version: &str,
    runtime: &str,
    permissions: Vec<String>,
    capabilities: Vec<String>,
    timestamp: u64,
) -> InstallReceipt {
    InstallReceipt {
        id: plugin_id.to_owned(),
        version: version.to_owned(),
        origin: PackageOrigin::Adopted,
        archive_digest: None,
        cloud_artifact: None,
        signature: SignatureState::unsigned(),
        runtime: runtime.to_owned(),
        granted_permissions: permissions,
        granted_capabilities: capabilities,
        installed_at: timestamp,
        updated_at: timestamp,
        policy: UpdatePolicy::default(),
        retained: Vec::new(),
    }
}
