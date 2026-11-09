use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::{manifest::is_valid_plugin_id, HostCompat, PluginManifest};
use tiktools_plugin_package::{
    PackageArchive, PackageError, PackageErrorKind, Sha256Digest, TrustStore, MANIFEST_FILE,
};

use crate::{
    jobs::{JobReporter, JobStage},
    layout::{move_dir, now, StoreLayout},
    locks::KeyedLocks,
    native_libs::backfill_native_libs,
    preview::{consent_token, requires_sideload_consent},
    receipt::{read_receipt, write_receipt},
    ChangeKind, InstallError, InstallErrorCode, InstallReceipt, PackageOrigin, PackagePreview,
    PermissionDiff, ReceiptStore, RetainedVersion, SignatureState, SignatureStatus, UpdatePolicy,
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
    pub origin: PackageOrigin,
    pub digest: Sha256Digest,
    pub manifest: PluginManifest,
    pub signature: SignatureState,
    pub changelog: Option<String>,
    checksums: Option<Vec<u8>>,
}

impl PreparedPackage {
    pub fn path(&self) -> &Path {
        self.archive.path()
    }

    pub fn size(&self) -> u64 {
        self.archive.size()
    }
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

pub struct Installer {
    layout: StoreLayout,
    host: HostCompat,
    retain: usize,
    provider_endpoints: Option<(String, String)>,
    receipts: ReceiptStore,
    trust: Arc<RwLock<TrustStore>>,
    locks: KeyedLocks,
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
        let staging = self.layout.staging();
        if let Ok(entries) = fs::read_dir(&staging) {
            for entry in entries.flatten() {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }

    pub fn prepare(
        &self,
        archive_path: &Path,
        origin: PackageOrigin,
        policy: SignaturePolicy,
    ) -> Result<PreparedPackage, InstallError> {
        let archive = PackageArchive::open(archive_path)?;
        let digest = archive.digest()?;
        let inspection = archive.inspect()?;
        let manifest = inspection.manifest;
        let signature = match (&inspection.signature, &inspection.checksums) {
            (Some(signature), Some(checksums)) => {
                let trust = self
                    .trust
                    .read()
                    .map_err(|_| InstallError::io("trust store lock poisoned"))?;
                match trust.verify_package(signature, checksums, &manifest.id, now()) {
                    Ok(verified) => SignatureState {
                        status: SignatureStatus::Verified,
                        publisher: Some(verified.publisher),
                        key_id: Some(verified.key_id.to_string()),
                        source: Some(verified.source),
                        reason: None,
                    },
                    Err(error) if error.kind() == PackageErrorKind::UntrustedPublisher => {
                        SignatureState {
                            status: SignatureStatus::Untrusted,
                            publisher: Some(signature.publisher.clone()),
                            key_id: Some(signature.key_id.to_string()),
                            source: None,
                            reason: Some(error.to_string()),
                        }
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            (Some(_), None) => {
                return Err(PackageError::MissingChecksums(manifest.id.clone()).into())
            }
            (None, _) => SignatureState::unsigned(),
        };
        if policy == SignaturePolicy::Require && !signature.is_verified() {
            let code = match signature.status {
                SignatureStatus::Unsigned => InstallErrorCode::SignatureRequired,
                _ => InstallErrorCode::UntrustedPublisher,
            };
            return Err(InstallError::new(
                code,
                signature.reason.clone().unwrap_or_else(|| {
                    format!("{} must be signed by a trusted publisher", manifest.id)
                }),
            ));
        }
        Ok(PreparedPackage {
            archive,
            origin,
            digest,
            manifest,
            signature,
            changelog: inspection.changelog,
            checksums: inspection.checksums,
        })
    }

    pub fn preview(&self, prepared: &PreparedPackage) -> PackagePreview {
        let manifest = &prepared.manifest;
        let installed = self.installed_manifest(&manifest.id);
        let receipt = self.receipts.load(&manifest.id).ok().flatten();
        let change = ChangeKind::between(
            installed.as_ref().map(|manifest| manifest.version.as_str()),
            &manifest.version,
        );
        let permission_diff = match (&receipt, &installed) {
            (Some(receipt), _) => PermissionDiff::between(
                &receipt.granted_permissions,
                &receipt.granted_capabilities,
                manifest,
            ),
            (None, Some(installed)) => {
                PermissionDiff::between(&installed.permissions, &installed.capabilities, manifest)
            }
            (None, None) => PermissionDiff::between(&[], &[], manifest),
        };
        PackagePreview {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            icon: manifest.icon.clone(),
            runtime: manifest.runtime.to_string(),
            security_model: manifest.security_model().into(),
            permissions: manifest.permissions.clone(),
            capabilities: manifest.capabilities.clone(),
            size: prepared.size(),
            digest: prepared.digest.to_string(),
            origin: prepared.origin.clone(),
            signature: prepared.signature.clone(),
            incompatibility: self.host.check_manifest(manifest).err(),
            change,
            installed_version: installed.map(|manifest| manifest.version),
            permission_diff,
            requires_sideload_consent: requires_sideload_consent(
                &prepared.origin,
                &prepared.signature,
            ),
            metadata: manifest.metadata.clone(),
            changelog: prepared.changelog.clone(),
            consent_token: consent_token(manifest, &prepared.digest, &prepared.signature, change),
        }
    }

    pub fn commit(
        &self,
        prepared: &PreparedPackage,
        options: &CommitOptions,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<InstallOutcome, InstallError> {
        let id = prepared.manifest.id.clone();
        let _guard = self.locks.try_lock(&id)?;
        reporter.check()?;
        let preview = self.preview(prepared);
        if let Some(incompatibility) = preview.incompatibility.clone() {
            return Err(incompatibility.into());
        }
        match &options.consent {
            Consent::Token(token) if *token == preview.consent_token => {}
            Consent::Token(_) => {
                return Err(InstallError::new(
                    InstallErrorCode::ConsentRequired,
                    format!("the reviewed package no longer matches {id}; review it again"),
                )
                .with_details(serde_json::to_value(&preview).unwrap_or_default()))
            }
            Consent::Implicit { replace_existing } => {
                if preview.installed_version.is_some() && !replace_existing {
                    return Err(InstallError::new(
                        InstallErrorCode::AlreadyInstalled,
                        format!("plugin is already installed: {id}"),
                    ));
                }
            }
        }
        reporter.stage(JobStage::Extract);
        fs::create_dir_all(&self.layout.plugins)?;
        let staging = self.layout.new_staging_dir(&id)?;
        let result = self.commit_staged(prepared, &preview, options, hooks, reporter, &staging);
        let _ = fs::remove_dir_all(&staging);
        result
    }

    fn commit_staged(
        &self,
        prepared: &PreparedPackage,
        preview: &PackagePreview,
        options: &CommitOptions,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
        staging: &Path,
    ) -> Result<InstallOutcome, InstallError> {
        let id = prepared.manifest.id.as_str();
        let extract_root = staging.join("package");
        fs::create_dir_all(&extract_root)?;
        let extracted = prepared.archive.extract(&extract_root)?;
        if extracted.manifest.id != prepared.manifest.id
            || extracted.manifest.version != prepared.manifest.version
            || prepared
                .checksums
                .as_deref()
                .is_some_and(|checksums| checksums != extracted.checksums.as_slice())
        {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                format!("{id} changed between review and installation"),
            ));
        }
        reporter.check()?;
        backfill_native_libs(
            &extracted.manifest,
            &extracted.root,
            staging,
            self.provider_endpoints.as_ref(),
        )?;
        reporter.check()?;

        reporter.stage(JobStage::Activate);
        let previous = self.current_state(id)?;
        self.swap_in(id, &extracted.root, previous.as_ref(), hooks)?;

        let timestamp = now();
        let mut retained = previous
            .as_ref()
            .map(|state| state.receipt.retained.clone())
            .unwrap_or_default();
        if let Some(state) = &previous {
            retained.retain(|entry| entry.version != state.receipt.version);
            retained.push(RetainedVersion {
                version: state.receipt.version.clone(),
                retained_at: timestamp,
            });
        }
        let receipt = InstallReceipt {
            id: id.to_owned(),
            version: extracted.manifest.version.clone(),
            origin: prepared.origin.clone(),
            archive_digest: Some(prepared.digest.to_string()),
            signature: prepared.signature.clone(),
            runtime: extracted.manifest.runtime.to_string(),
            granted_permissions: extracted.manifest.permissions.clone(),
            granted_capabilities: extracted.manifest.capabilities.clone(),
            installed_at: previous
                .as_ref()
                .map(|state| state.receipt.installed_at)
                .unwrap_or(timestamp),
            updated_at: timestamp,
            policy: options
                .policy
                .clone()
                .or_else(|| previous.as_ref().map(|state| state.receipt.policy.clone()))
                .unwrap_or_default(),
            retained,
        };
        let receipt = self.finalize(receipt)?;
        reporter.stage(JobStage::Cleanup);
        Ok(InstallOutcome {
            directory: self.layout.package(id).display().to_string(),
            change: preview.change,
            previous_version: previous.map(|state| state.receipt.version),
            receipt,
        })
    }

    pub fn rollback(
        &self,
        plugin_id: &str,
        version: Option<&str>,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<InstallOutcome, InstallError> {
        let _guard = self.locks.try_lock(plugin_id)?;
        let current = self
            .current_state(plugin_id)?
            .ok_or_else(|| InstallError::not_installed(plugin_id))?;
        let target_version = match version {
            Some(version) => version.to_owned(),
            None => current
                .receipt
                .retained
                .last()
                .map(|entry| entry.version.clone())
                .ok_or_else(|| {
                    InstallError::new(
                        InstallErrorCode::NotFound,
                        format!("no previous version of {plugin_id} is retained"),
                    )
                })?,
        };
        let retained_dir = self.layout.retained(plugin_id, &target_version);
        let manifest = fs::read_to_string(retained_dir.join(MANIFEST_FILE))
            .ok()
            .and_then(|text| PluginManifest::from_json_str(&text).ok())
            .filter(|manifest| manifest.id == plugin_id)
            .ok_or_else(|| {
                InstallError::new(
                    InstallErrorCode::NotFound,
                    format!("{plugin_id} {target_version} is not retained"),
                )
            })?;
        self.host.check_manifest(&manifest)?;
        reporter.stage(JobStage::Activate);
        let staging = self.layout.new_staging_dir(plugin_id)?;
        let candidate = staging.join("package");
        move_dir(&retained_dir, &candidate)?;
        match self.swap_in(plugin_id, &candidate, Some(&current), hooks) {
            Ok(()) => {}
            Err(error) => {
                let _ = move_dir(&candidate, &retained_dir);
                let _ = fs::remove_dir_all(&staging);
                return Err(error);
            }
        };
        let _ = fs::remove_dir_all(&staging);
        let retained_receipt_path = self.layout.retained_receipt(plugin_id, &target_version);
        let base = read_receipt(&retained_receipt_path)?
            .unwrap_or_else(|| adopted_receipt(&manifest, now()));
        let _ = fs::remove_file(&retained_receipt_path);
        let timestamp = now();
        let mut retained: Vec<RetainedVersion> = current
            .receipt
            .retained
            .iter()
            .filter(|entry| {
                entry.version != target_version && entry.version != current.receipt.version
            })
            .cloned()
            .collect();
        retained.push(RetainedVersion {
            version: current.receipt.version.clone(),
            retained_at: timestamp,
        });
        let mut policy = current.receipt.policy.clone();
        policy.pinned = true;
        let receipt = InstallReceipt {
            version: manifest.version.clone(),
            updated_at: timestamp,
            installed_at: current.receipt.installed_at,
            policy,
            retained,
            ..base
        };
        let receipt = self.finalize(receipt)?;
        reporter.stage(JobStage::Cleanup);
        Ok(InstallOutcome {
            directory: self.layout.package(plugin_id).display().to_string(),
            change: ChangeKind::between(Some(&current.receipt.version), &manifest.version),
            previous_version: Some(current.receipt.version),
            receipt,
        })
    }

    pub fn uninstall(
        &self,
        plugin_id: &str,
        purge_data: Option<&Path>,
        hooks: &dyn ActivationHooks,
    ) -> Result<(), InstallError> {
        let _guard = self.locks.try_lock(plugin_id)?;
        let target = self.checked_package_dir(plugin_id)?;
        let was_running = hooks.deactivate(plugin_id).map_err(InstallError::io)?;
        let trash = self.layout.new_staging_dir("removed")?;
        if let Err(error) = fs::rename(&target, trash.join(plugin_id)) {
            let _ = fs::remove_dir_all(&trash);
            let _ = hooks.activate(plugin_id, was_running);
            return Err(InstallError::io(format!(
                "could not remove plugin package: {error}"
            )));
        }
        if let Err(error) = hooks.activate(plugin_id, false) {
            tracing::warn!(plugin = %plugin_id, %error, "plugin rescan failed after uninstall");
        }
        let _ = fs::remove_dir_all(&trash);
        self.receipts.remove(plugin_id)?;
        let versions = self.layout.versions(plugin_id);
        if versions.exists() {
            fs::remove_dir_all(versions)?;
        }
        if let Some(data) = purge_data.filter(|path| path.exists()) {
            fs::remove_dir_all(data)?;
        }
        Ok(())
    }

    pub fn versions(&self, plugin_id: &str) -> Result<InstalledVersions, InstallError> {
        let receipt = self.receipts.load(plugin_id)?;
        let installed = self
            .installed_manifest(plugin_id)
            .map(|manifest| manifest.version);
        let retained = receipt
            .as_ref()
            .map(|receipt| {
                receipt
                    .retained
                    .iter()
                    .filter(|entry| self.layout.retained(plugin_id, &entry.version).is_dir())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        Ok(InstalledVersions {
            id: plugin_id.to_owned(),
            installed,
            receipt,
            retained,
        })
    }

    pub fn set_policy(
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
            (Some(receipt), Some(manifest)) => InstallReceipt {
                version: manifest.version.clone(),
                ..receipt
            },
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

    fn swap_in(
        &self,
        plugin_id: &str,
        candidate: &Path,
        previous: Option<&CurrentState>,
        hooks: &dyn ActivationHooks,
    ) -> Result<(), InstallError> {
        let target = self.layout.package(plugin_id);
        if target.parent() != Some(self.layout.plugins.as_path()) {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "plugin installation target escaped the plugin directory",
            ));
        }
        let was_running = hooks.deactivate(plugin_id).map_err(InstallError::io)?;
        let mut retained_to = None;
        if let Some(state) = previous {
            let destination = self.layout.retained(plugin_id, &state.receipt.version);
            if destination.exists() {
                fs::remove_dir_all(&destination)?;
            }
            if let Err(error) = move_dir(&state.directory, &destination) {
                let _ = hooks.activate(plugin_id, was_running);
                return Err(error.into());
            }
            if let Err(error) = write_receipt(
                &self
                    .layout
                    .retained_receipt(plugin_id, &state.receipt.version),
                &InstallReceipt {
                    retained: Vec::new(),
                    ..state.receipt.clone()
                },
            ) {
                tracing::warn!(plugin = %plugin_id, %error, "could not record the retained receipt");
            }
            retained_to = Some(destination);
        }
        if let Err(error) = move_dir(candidate, &target) {
            if let Some(retained) = &retained_to {
                let _ = move_dir(retained, &target);
            }
            let _ = hooks.activate(plugin_id, was_running);
            return Err(error.into());
        }
        if let Err(message) = hooks.activate(plugin_id, was_running) {
            // Hand the rejected package back at the candidate path so the
            // caller decides its fate: installs discard their staging
            // directory, while a rollback must return it to the retained
            // store instead of losing that version.
            if move_dir(&target, candidate).is_err() {
                let failed = self.layout.new_staging_dir("failed")?;
                let _ = move_dir(&target, &failed.join(plugin_id));
                let _ = fs::remove_dir_all(&failed);
            }
            if let Some(retained) = &retained_to {
                let _ = move_dir(retained, &target);
            }
            let _ = hooks.activate(plugin_id, was_running);
            return Err(InstallError::io(format!(
                "plugin {plugin_id} could not be activated: {message}"
            )));
        }
        Ok(())
    }

    fn finalize(&self, mut receipt: InstallReceipt) -> Result<InstallReceipt, InstallError> {
        receipt.retained.sort_by_key(|entry| entry.retained_at);
        while receipt.retained.len() > self.retain {
            let removed = receipt.retained.remove(0);
            let directory = self.layout.retained(&receipt.id, &removed.version);
            if directory.exists() {
                fs::remove_dir_all(&directory)?;
            }
            let _ = fs::remove_file(self.layout.retained_receipt(&receipt.id, &removed.version));
        }
        self.receipts.save(&receipt)?;
        Ok(receipt)
    }

    fn checked_package_dir(&self, plugin_id: &str) -> Result<PathBuf, InstallError> {
        if !is_valid_plugin_id(plugin_id) {
            return Err(InstallError::not_installed(plugin_id));
        }
        let target = self.layout.package(plugin_id);
        let metadata =
            fs::symlink_metadata(&target).map_err(|_| InstallError::not_installed(plugin_id))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "plugin package is not a regular directory",
            ));
        }
        let root = fs::canonicalize(&self.layout.plugins)?;
        let package = fs::canonicalize(&target)?;
        if package.parent() != Some(root.as_path()) {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "plugin package is outside the user plugin directory",
            ));
        }
        Ok(target)
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
