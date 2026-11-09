use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::{manifest::is_valid_plugin_id, HostCompat, PluginManifest};
use tiktools_plugin_package::{
    Checksums, PackageArchive, PackageError, PackageErrorKind, PackageSignature, Sha256Digest,
    TrustStore, CHECKSUMS_FILE, MANIFEST_FILE, MAX_FILES, MAX_MANIFEST_BYTES, SIGNATURE_FILE,
};

use crate::{
    jobs::{JobReporter, JobStage},
    layout::{move_dir, now, sanitize_version, StoreLayout},
    locks::KeyedLocks,
    native_libs::backfill_native_libs,
    preview::{consent_token, requires_sideload_consent},
    receipt::{read_receipt, write_atomic, write_receipt},
    ChangeKind, CloudArtifactIdentity, InstallError, InstallErrorCode, InstallReceipt,
    PackageOrigin, PackagePreview, PermissionDiff, ReceiptStore, RetainedVersion, SignatureState,
    SignatureStatus, UpdatePolicy,
};

pub const DEFAULT_RETAINED_VERSIONS: usize = 2;
const MAX_INSTALLED_CHECKSUMS_BYTES: usize = 8 * 1024 * 1024;
const MAX_INSTALLED_SIGNATURE_BYTES: usize = 16 * 1024;
const MAX_INSTALLED_RECEIPT_BYTES: usize = 256 * 1024;

/// Reads a bounded regular file without accepting a symlink as the final
/// component. This is an observation under the cooperative store lease; it
/// cannot fence unrelated processes that directly rewrite the filesystem.
fn read_observed_file(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, InstallError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.file_type().is_file() || metadata.len() > limit as u64 {
        return Err(InstallError::new(
            InstallErrorCode::InvalidPackage,
            "installed package metadata is not a bounded regular file",
        ));
    }
    let mut file = File::open(path)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() != metadata.len() {
        return Err(InstallError::new(
            InstallErrorCode::InvalidPackage,
            "installed package changed during review",
        ));
    }
    let mut bytes = Vec::new();
    (&mut file).take(limit as u64 + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() > limit
        || bytes.len() as u64 != before.len()
        || after.len() != before.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(InstallError::new(
            InstallErrorCode::InvalidPackage,
            "installed package changed during review",
        ));
    }
    Ok(Some(bytes))
}

fn required_observed_file(path: &Path, limit: usize) -> Result<Vec<u8>, InstallError> {
    read_observed_file(path, limit)?.ok_or_else(|| {
        InstallError::new(
            InstallErrorCode::InvalidPackage,
            "installed package metadata is missing",
        )
    })
}

fn remove_optional_file(path: &Path) -> Result<(), InstallError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

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

impl PreparedPackage {
    pub fn path(&self) -> &Path {
        &self.original_path
    }

    pub fn size(&self) -> u64 {
        self.archive.size()
    }

    /// The trusted product caller invokes this only after verifying the exact
    /// artifact against signed metadata and the current Cloud generation.
    pub fn bind_verified_cloud_artifact(
        &mut self,
        target: &str,
        generation: u64,
    ) -> Result<(), InstallError> {
        if !matches!(self.origin, PackageOrigin::Registry { .. })
            || !self.signature.is_verified()
            || generation == 0
            || target.is_empty()
            || target.len() > 128
            || !target
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "invalid verified Cloud artifact identity",
            ));
        }
        self.cloud_artifact = Some(CloudArtifactIdentity {
            target: target.to_owned(),
            generation: generation.to_string(),
        });
        Ok(())
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

impl LockedPackageStore<'_> {
    /// Bind a journal caller to this lease's actual plugin and store roots.
    /// The configured roots must exist; aliases of the same directories are
    /// accepted. External filesystem changes remain outside this lease.
    pub fn matches_roots(&self, plugins: &Path, store: &Path) -> Result<bool, InstallError> {
        Ok(
            fs::canonicalize(plugins)? == fs::canonicalize(&self.installer.layout.plugins)?
                && fs::canonicalize(store)? == fs::canonicalize(&self.installer.layout.store)?,
        )
    }
    /// Read the receipt while the package-store lease remains held. The
    /// receipt alone does not prove the files or runtime are consistent.
    pub fn receipt(&self, plugin_id: &str) -> Result<Option<InstallReceipt>, InstallError> {
        self.installer.receipt(plugin_id)
    }

    /// Read the installed manifest while the lease remains held. Missing or
    /// malformed manifests both return `None` and must not authorize recovery.
    pub fn installed_manifest(&self, plugin_id: &str) -> Option<PluginManifest> {
        self.installer.installed_manifest(plugin_id)
    }

    /// Apply an already constructed policy while retaining this same lease.
    /// Callers that merge a patch must also read the receipt under the lease.
    pub fn set_policy(
        &mut self,
        plugin_id: &str,
        policy: UpdatePolicy,
    ) -> Result<InstallReceipt, InstallError> {
        self.installer
            .set_policy_under_store_lock(plugin_id, policy)
    }

    /// Recheck the actual installed files and a currently trusted publisher
    /// signature while retaining this store lease. Unsigned, adopted,
    /// inconsistent or unreadable installations return an error rather than
    /// being treated as absent. The observation is not a durable receipt and
    /// must be repeated before a future recovery decision. Cooperative locks
    /// do not fence older binaries or external filesystem writers. Concurrent
    /// replacement of paths by noncooperating writers and Windows junctions or
    /// reparse points have not been verified as a secure boundary.
    pub fn inspect_verified(
        &self,
        plugin_id: &str,
    ) -> Result<InstalledPackageEvidence, InstallError> {
        self.installer.inspect_verified_under_store_lock(plugin_id)
    }

    pub fn commit(
        &mut self,
        prepared: &PreparedPackage,
        options: &CommitOptions,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<InstallOutcome, InstallError> {
        self.installer
            .commit_under_store_lock(prepared, options, hooks, reporter)
    }

    pub fn rollback(
        &mut self,
        plugin_id: &str,
        version: Option<&str>,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<InstallOutcome, InstallError> {
        self.installer
            .rollback_under_store_lock(plugin_id, version, hooks, reporter, false)
    }

    /// Restore only a previously reviewed local package whose retained tree
    /// still matches its installation-time proof. The provenance check and
    /// swap both happen under this store lease.
    pub fn rollback_local(
        &mut self,
        plugin_id: &str,
        version: Option<&str>,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<InstallOutcome, InstallError> {
        self.installer
            .rollback_under_store_lock(plugin_id, version, hooks, reporter, true)
    }

    pub fn uninstall(
        &mut self,
        plugin_id: &str,
        purge_data: Option<&Path>,
        hooks: &dyn ActivationHooks,
    ) -> Result<(), InstallError> {
        self.installer
            .uninstall_under_store_lock(plugin_id, purge_data, hooks)
    }
}

impl Installer {
    fn ensure_store_child(&self, relative: &Path, create: bool) -> Result<bool, InstallError> {
        let store = match fs::symlink_metadata(&self.layout.store) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound && !create => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        if !store.file_type().is_dir() {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "package store is not a real directory",
            ));
        }
        let mut current = self.layout.store.clone();
        for component in relative.components() {
            let std::path::Component::Normal(name) = component else {
                return Err(InstallError::new(
                    InstallErrorCode::InvalidPackage,
                    "invalid package store path",
                ));
            };
            current.push(name);
            let metadata = match fs::symlink_metadata(&current) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound && !create => {
                    return Ok(false)
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    match fs::create_dir(&current) {
                        Ok(()) => {}
                        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                        Err(error) => return Err(error.into()),
                    }
                    fs::symlink_metadata(&current)?
                }
                Err(error) => return Err(error.into()),
            };
            if !metadata.file_type().is_dir() {
                return Err(InstallError::new(
                    InstallErrorCode::InvalidPackage,
                    "package store path is not a real directory",
                ));
            }
        }
        Ok(true)
    }

    fn ensure_versions_dir(&self, plugin_id: &str, create: bool) -> Result<bool, InstallError> {
        if !is_valid_plugin_id(plugin_id) {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "invalid plugin ID",
            ));
        }
        self.ensure_store_child(&Path::new("versions").join(plugin_id), create)
    }

    fn legacy_retained(&self, plugin_id: &str, version: &str) -> Option<PathBuf> {
        let component = sanitize_version(version);
        if matches!(component.as_str(), "" | "." | "..") {
            return None;
        }
        Some(self.layout.versions(plugin_id).join(component))
    }

    fn retained_matches(path: &Path, plugin_id: &str, version: &str) -> bool {
        if !fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir()) {
            return false;
        }
        required_observed_file(&path.join(MANIFEST_FILE), MAX_MANIFEST_BYTES)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|text| PluginManifest::from_json_str(&text).ok())
            .is_some_and(|manifest| manifest.id == plugin_id && manifest.version == version)
    }

    pub(crate) fn migration_receipt_proof(
        root: &Path,
        receipt: &[u8],
        plugin_id: &str,
    ) -> Result<String, InstallError> {
        let checksums =
            required_observed_file(&root.join(CHECKSUMS_FILE), MAX_INSTALLED_CHECKSUMS_BYTES)?;
        Checksums::parse(&checksums)?.verify_tree_with_directory_limit(
            root,
            plugin_id,
            MAX_FILES * 4,
        )?;
        let signature =
            read_observed_file(&root.join(SIGNATURE_FILE), MAX_INSTALLED_SIGNATURE_BYTES)?;
        let identity = serde_json::to_vec(&(
            Sha256Digest::of_bytes(receipt).to_string(),
            Sha256Digest::of_bytes(&checksums).to_string(),
            signature
                .as_deref()
                .map(Sha256Digest::of_bytes)
                .map(|v| v.to_string()),
        ))
        .map_err(InstallError::io)?;
        Ok(Sha256Digest::of_bytes(&identity).to_string())
    }

    // Old sanitization was not injective and permitted parent components.
    // Migrate only a real directory whose manifest identifies this exact
    // version. Never delete ambiguous entries or attach an unrelated receipt.
    fn migrate_legacy_retained(&self, plugin_id: &str, version: &str) -> Result<(), InstallError> {
        if !self.ensure_versions_dir(plugin_id, false)? {
            return Ok(());
        }
        let destination = self.layout.retained(plugin_id, version);
        let Some(legacy) = self.legacy_retained(plugin_id, version) else {
            return Ok(());
        };
        let old_receipt = self
            .layout
            .versions(plugin_id)
            .join(format!("{}.json", sanitize_version(version)));
        let new_receipt = self.layout.retained_receipt(plugin_id, version);
        let marker = self.layout.retained_migration_proof(plugin_id, version);
        let receipt_bytes = read_observed_file(&old_receipt, MAX_INSTALLED_RECEIPT_BYTES)
            .ok()
            .flatten()
            .filter(|bytes| {
                serde_json::from_slice::<InstallReceipt>(bytes)
                    .ok()
                    .is_some_and(|receipt| receipt.id == plugin_id && receipt.version == version)
            });
        let exists = match fs::symlink_metadata(&destination) {
            Ok(_) if Self::retained_matches(&destination, plugin_id, version) => true,
            Ok(_) => {
                return Err(InstallError::new(
                    InstallErrorCode::InvalidPackage,
                    "retained destination contains unexpected data",
                ))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if !exists {
            if !Self::retained_matches(&legacy, plugin_id, version) {
                return Ok(());
            }
            remove_optional_file(&marker)?;
            if let Some(bytes) = &receipt_bytes {
                if let Ok(proof) = Self::migration_receipt_proof(&legacy, bytes, plugin_id) {
                    write_atomic(&marker, proof.as_bytes())?;
                }
            }
            fs::rename(&legacy, &destination)?;
        }
        let matches = receipt_bytes.as_ref().is_some_and(|bytes| {
            read_observed_file(&marker, 64)
                .ok()
                .flatten()
                .is_some_and(|marker| {
                    Self::migration_receipt_proof(&destination, bytes, plugin_id)
                        .is_ok_and(|proof| marker == proof.as_bytes())
                })
        });
        if matches
            && !legacy.exists()
            && matches!(fs::symlink_metadata(&new_receipt), Err(error) if error.kind() == io::ErrorKind::NotFound)
        {
            fs::rename(old_receipt, new_receipt)?;
            remove_optional_file(&marker)?;
        }
        Ok(())
    }

    // The proof binds the retained tree to the local artifact previously
    // reviewed by this installer. It is an integrity check under the
    // cooperative store lease, not protection against a process that can
    // arbitrarily rewrite both the store and its proofs.
    fn local_proof(&self, root: &Path, receipt: &InstallReceipt) -> Result<String, InstallError> {
        if !matches!(receipt.origin, PackageOrigin::File)
            || receipt
                .archive_digest
                .as_deref()
                .and_then(Sha256Digest::parse)
                .is_none()
        {
            return Err(InstallError::new(
                InstallErrorCode::Unsupported,
                "rollback target is not a reviewed local package",
            ));
        }
        let manifest_bytes = required_observed_file(&root.join(MANIFEST_FILE), MAX_MANIFEST_BYTES)?;
        let manifest_text = std::str::from_utf8(&manifest_bytes).map_err(|error| {
            InstallError::new(InstallErrorCode::InvalidPackage, error.to_string())
        })?;
        let manifest = PluginManifest::from_json_str(manifest_text).map_err(|error| {
            InstallError::new(InstallErrorCode::InvalidPackage, error.to_string())
        })?;
        if manifest.id != receipt.id
            || manifest.version != receipt.version
            || manifest.runtime.to_string() != receipt.runtime
            || manifest.permissions != receipt.granted_permissions
            || manifest.capabilities != receipt.granted_capabilities
        {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "local rollback receipt and package disagree",
            ));
        }
        let checksums =
            required_observed_file(&root.join(CHECKSUMS_FILE), MAX_INSTALLED_CHECKSUMS_BYTES)?;
        Checksums::parse(&checksums)?.verify_tree_with_directory_limit(
            root,
            &receipt.id,
            MAX_FILES * 4,
        )?;
        let signature =
            read_observed_file(&root.join(SIGNATURE_FILE), MAX_INSTALLED_SIGNATURE_BYTES)?;
        match (receipt.signature.status, signature.as_deref()) {
            (SignatureStatus::Unsigned, None) => {}
            (status, Some(bytes)) if status != SignatureStatus::Unsigned => {
                let parsed = PackageSignature::parse(bytes)?;
                if receipt.signature.publisher.as_deref() != Some(parsed.publisher.as_str())
                    || receipt.signature.key_id.as_deref() != Some(parsed.key_id.as_str())
                {
                    return Err(InstallError::new(
                        InstallErrorCode::InvalidPackage,
                        "local package signature and review disagree",
                    ));
                }
                let trust = self
                    .trust
                    .read()
                    .map_err(|_| InstallError::io("trust store lock poisoned"))?;
                match (
                    status,
                    trust.verify_package(&parsed, &checksums, &receipt.id, now()),
                ) {
                    (SignatureStatus::Verified, Ok(_)) => {}
                    (SignatureStatus::Untrusted, Err(error))
                        if error.kind() == PackageErrorKind::UntrustedPublisher => {}
                    _ => {
                        return Err(InstallError::new(
                            InstallErrorCode::UntrustedPublisher,
                            "local package trust changed; review it again",
                        ))
                    }
                }
            }
            _ => {
                return Err(InstallError::new(
                    InstallErrorCode::InvalidPackage,
                    "local package signature and review disagree",
                ))
            }
        }
        let identity = serde_json::to_vec(&(
            &receipt.id,
            &receipt.version,
            &receipt.origin,
            &receipt.archive_digest,
            &receipt.signature,
            &receipt.runtime,
            &receipt.granted_permissions,
            &receipt.granted_capabilities,
            Sha256Digest::of_bytes(&checksums).to_string(),
            signature
                .as_deref()
                .map(Sha256Digest::of_bytes)
                .map(|v| v.to_string()),
        ))
        .map_err(InstallError::io)?;
        Ok(Sha256Digest::of_bytes(&identity).to_string())
    }

    fn matching_local_proof(&self, root: &Path, receipt: &InstallReceipt, path: &Path) -> bool {
        let Ok(Some(proof)) = read_observed_file(path, 64) else {
            return false;
        };
        self.local_proof(root, receipt)
            .is_ok_and(|expected| proof == expected.as_bytes())
    }

    fn record_local_proof(
        &self,
        root: &Path,
        receipt: &InstallReceipt,
        path: &Path,
        expected: &str,
    ) {
        if let Ok(proof) = self.local_proof(root, receipt) {
            if proof != expected {
                return;
            }
            let _ = write_atomic(path, proof.as_bytes());
        }
    }

    fn observed_receipt(&self, plugin_id: &str) -> Result<Option<Vec<u8>>, InstallError> {
        let directory = self.layout.receipts();
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if !metadata.file_type().is_dir()
            || fs::canonicalize(&directory)?.parent()
                != Some(fs::canonicalize(&self.layout.store)?.as_path())
        {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "receipt directory is not confined to the package store",
            ));
        }
        read_observed_file(&self.layout.receipt(plugin_id), MAX_INSTALLED_RECEIPT_BYTES)
    }

    fn inspect_verified_under_store_lock(
        &self,
        plugin_id: &str,
    ) -> Result<InstalledPackageEvidence, InstallError> {
        if !is_valid_plugin_id(plugin_id) {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "invalid plugin ID",
            ));
        }
        let path = self.layout.package(plugin_id);
        let receipt_bytes = self.observed_receipt(plugin_id)?;
        let root = match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound && receipt_bytes.is_none() => {
                return Ok(InstalledPackageEvidence::Absent);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(InstallError::new(
                    InstallErrorCode::InvalidPackage,
                    "package is missing but its receipt remains",
                ));
            }
            Err(error) => return Err(error.into()),
            Ok(_) => self.checked_package_dir(plugin_id)?,
        };
        let receipt_bytes = receipt_bytes.ok_or_else(|| {
            InstallError::new(
                InstallErrorCode::InvalidPackage,
                "installed package receipt is missing",
            )
        })?;
        let receipt: InstallReceipt = serde_json::from_slice(&receipt_bytes).map_err(|_| {
            InstallError::new(
                InstallErrorCode::InvalidPackage,
                "installed package receipt is invalid",
            )
        })?;
        let manifest_path = root.join(MANIFEST_FILE);
        let checksums_path = root.join(CHECKSUMS_FILE);
        let signature_path = root.join(SIGNATURE_FILE);
        let manifest_bytes = required_observed_file(&manifest_path, MAX_MANIFEST_BYTES)?;
        let manifest_text = std::str::from_utf8(&manifest_bytes).map_err(|_| {
            InstallError::new(
                InstallErrorCode::InvalidPackage,
                "installed manifest is not UTF-8",
            )
        })?;
        let manifest = PluginManifest::from_json_str(manifest_text).map_err(|error| {
            InstallError::new(InstallErrorCode::InvalidPackage, error.to_string())
        })?;
        manifest.validate_compatibility().map_err(|error| {
            InstallError::new(InstallErrorCode::InvalidPackage, error.to_string())
        })?;
        self.host.check_manifest(&manifest)?;
        if manifest.id != plugin_id
            || receipt.id != plugin_id
            || receipt.version != manifest.version
            || receipt.runtime != manifest.runtime.to_string()
            || receipt.granted_permissions != manifest.permissions
            || receipt.granted_capabilities != manifest.capabilities
        {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "installed manifest and receipt disagree",
            ));
        }
        // This is only a syntactically valid receipt declaration: the
        // extracted tree cannot prove which original archive supplied it.
        // Do not return it as verified artifact evidence.
        let _declared_archive_digest = receipt
            .archive_digest
            .as_deref()
            .and_then(Sha256Digest::parse)
            .ok_or_else(|| {
                InstallError::new(
                    InstallErrorCode::InvalidPackage,
                    "installed package archive identity is missing",
                )
            })?;
        if receipt.signature.status != SignatureStatus::Verified {
            return Err(InstallError::new(
                InstallErrorCode::SignatureRequired,
                "installed package does not have a verified publisher receipt",
            ));
        }
        let checksums = required_observed_file(&checksums_path, MAX_INSTALLED_CHECKSUMS_BYTES)?;
        let signature = required_observed_file(&signature_path, MAX_INSTALLED_SIGNATURE_BYTES)?;
        let parsed = PackageSignature::parse(&signature)?;
        Checksums::parse(&checksums)?.verify_tree_with_directory_limit(
            &root,
            plugin_id,
            MAX_FILES * 4,
        )?;
        let verified = self
            .trust
            .read()
            .map_err(|_| InstallError::io("trust store lock poisoned"))?
            .verify_package(&parsed, &checksums, plugin_id, now())?;
        if receipt.signature.publisher.as_deref() != Some(verified.publisher.as_str())
            || receipt.signature.key_id.as_deref() != Some(verified.key_id.as_str())
        {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "installed signature and receipt disagree",
            ));
        }
        // Refuse an inconsistent observation if these controlling files were
        // replaced while the rest of the tree was being checked.
        if self.observed_receipt(plugin_id)?.as_deref() != Some(receipt_bytes.as_slice())
            || required_observed_file(&manifest_path, MAX_MANIFEST_BYTES)? != manifest_bytes
            || required_observed_file(&checksums_path, MAX_INSTALLED_CHECKSUMS_BYTES)? != checksums
            || required_observed_file(&signature_path, MAX_INSTALLED_SIGNATURE_BYTES)? != signature
        {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "installed package changed during review",
            ));
        }
        Ok(InstalledPackageEvidence::Verified {
            id: plugin_id.to_owned(),
            version: manifest.version,
            checksums_digest: Sha256Digest::of_bytes(&checksums),
            publisher: verified.publisher,
            key_id: verified.key_id.to_string(),
            trust_source: verified.source,
        })
    }

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

    /// Acquire the same process-wide store lock used by every cooperative
    /// package mutation. Never hold a sync admission or CloudAuth lock while
    /// waiting for a store lease; acquisition itself is nonblocking.
    pub fn try_lock_store(&self) -> Result<LockedPackageStore<'_>, InstallError> {
        self.try_lock_store_for("package store")
    }

    fn try_lock_store_for(&self, label: &str) -> Result<LockedPackageStore<'_>, InstallError> {
        // Both acquisitions are nonblocking. On any second-lock failure the
        // first lease drops immediately, so opposite root orders cannot wait
        // on one another. Canonical aliases are locked once.
        let operation = crate::locks::try_package_operation(&self.layout.plugins, label)?;
        fs::create_dir_all(&self.layout.store)?;
        let same = fs::canonicalize(&self.layout.plugins)? == fs::canonicalize(&self.layout.store)?;
        let store_operation = if same {
            None
        } else {
            Some(crate::locks::try_package_operation(
                &self.layout.store,
                label,
            )?)
        };
        Ok(LockedPackageStore {
            installer: self,
            _operation: operation,
            _store_operation: store_operation,
        })
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

    pub fn prepare(
        &self,
        archive_path: &Path,
        origin: PackageOrigin,
        policy: SignaturePolicy,
    ) -> Result<PreparedPackage, InstallError> {
        let source = PackageArchive::open(archive_path)?;
        let original_path = source.path().to_owned();
        let mut input = File::open(&original_path).map_err(InstallError::io)?;
        let before = input.metadata().map_err(InstallError::io)?;
        if !before.is_file() || before.len() != source.size() {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "package changed while preparing the review",
            ));
        }
        let snapshot = Arc::new(
            tempfile::Builder::new()
                .prefix("tiktools-package-review-")
                .tempdir()
                .map_err(InstallError::io)?,
        );
        let snapshot_path = snapshot.path().join("review.plugin");
        let mut output = File::create(&snapshot_path).map_err(InstallError::io)?;
        // Bound the copy even if the source grows while it is being read.
        let copied = io::copy(&mut (&mut input).take(before.len() + 1), &mut output)
            .map_err(InstallError::io)?;
        let after = input.metadata().map_err(InstallError::io)?;
        if copied != before.len()
            || after.len() != before.len()
            || before.modified().ok() != after.modified().ok()
        {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "package changed while preparing the review",
            ));
        }
        drop(output);
        let archive = PackageArchive::open(&snapshot_path)?;
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
            original_path,
            _snapshot: snapshot,
            origin,
            digest,
            manifest,
            signature,
            changelog: inspection.changelog,
            checksums: inspection.checksums,
            cloud_artifact: None,
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
        self.try_lock_store_for(&prepared.manifest.id)?
            .commit(prepared, options, hooks, reporter)
    }

    fn commit_under_store_lock(
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
            cloud_artifact: prepared.cloud_artifact.clone(),
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
        // Capture the identity before activation can run plugin code. A later
        // checksum rewrite, even together with changed files, cannot acquire a
        // proof for the previously reviewed package.
        let expected_local_proof = self.local_proof(&extracted.root, &receipt).ok();
        self.swap_in(id, &extracted.root, previous.as_ref(), hooks)?;
        let receipt = self.finalize(receipt)?;
        if let Some(expected) = expected_local_proof {
            self.record_local_proof(
                &self.layout.package(id),
                &receipt,
                &self.layout.installed_local_proof(id),
                &expected,
            );
        }
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
        self.try_lock_store_for(plugin_id)?
            .rollback(plugin_id, version, hooks, reporter)
    }

    /// Product rollback restricted to a retained, verified local package.
    pub fn rollback_local(
        &self,
        plugin_id: &str,
        version: Option<&str>,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<InstallOutcome, InstallError> {
        self.try_lock_store_for(plugin_id)?
            .rollback_local(plugin_id, version, hooks, reporter)
    }

    fn rollback_under_store_lock(
        &self,
        plugin_id: &str,
        version: Option<&str>,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
        local_only: bool,
    ) -> Result<InstallOutcome, InstallError> {
        let _guard = self.locks.try_lock(plugin_id)?;
        self.ensure_versions_dir(plugin_id, false)?;
        self.ensure_store_child(Path::new("local-proofs"), false)?;
        self.ensure_store_child(Path::new("receipts"), false)?;
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
        if local_only && target_version == current.receipt.version {
            return Err(InstallError::new(
                InstallErrorCode::Unsupported,
                "rollback target is already installed",
            ));
        }
        self.migrate_legacy_retained(plugin_id, &target_version)?;
        let retained_dir = self.layout.retained(plugin_id, &target_version);
        if !Self::retained_matches(&retained_dir, plugin_id, &target_version) {
            return Err(InstallError::new(
                InstallErrorCode::NotFound,
                format!("{plugin_id} {target_version} is not retained"),
            ));
        }
        let manifest = fs::read_to_string(retained_dir.join(MANIFEST_FILE))
            .ok()
            .and_then(|text| PluginManifest::from_json_str(&text).ok())
            .filter(|manifest| manifest.id == plugin_id && manifest.version == target_version)
            .ok_or_else(|| {
                InstallError::new(
                    InstallErrorCode::NotFound,
                    format!("{plugin_id} {target_version} is not retained"),
                )
            })?;
        let retained_receipt_path = self.layout.retained_receipt(plugin_id, &target_version);
        let retained_receipt = read_receipt(&retained_receipt_path)?
            .filter(|receipt| receipt.id == plugin_id && receipt.version == target_version);
        let mut expected_local_proof = None;
        if local_only {
            let eligible = current
                .receipt
                .retained
                .iter()
                .any(|entry| entry.version == target_version)
                && retained_receipt.as_ref().is_some_and(|receipt| {
                    receipt.id == plugin_id
                        && receipt.version == target_version
                        && self.matching_local_proof(
                            &retained_dir,
                            receipt,
                            &self.layout.retained_local_proof(plugin_id, &target_version),
                        )
                });
            if !eligible {
                return Err(InstallError::new(
                    InstallErrorCode::Unsupported,
                    "rollback target cannot be verified as the reviewed local package; reinstall the version after review",
                ));
            }
            expected_local_proof = retained_receipt
                .as_ref()
                .and_then(|receipt| self.local_proof(&retained_dir, receipt).ok());
        }
        self.host.check_manifest(&manifest)?;
        reporter.stage(JobStage::Activate);
        let staging = self.layout.new_staging_dir(plugin_id)?;
        let candidate = staging.join("package");
        move_dir(&retained_dir, &candidate)?;
        match self.swap_in(plugin_id, &candidate, Some(&current), hooks) {
            Ok(()) => {}
            Err(error) => {
                if candidate.exists() {
                    let restored_candidate = fs::read_to_string(candidate.join(MANIFEST_FILE))
                        .ok()
                        .and_then(|text| PluginManifest::from_json_str(&text).ok())
                        .is_some_and(|manifest| {
                            manifest.id == plugin_id && manifest.version == target_version
                        });
                    if !restored_candidate {
                        return Err(InstallError::io(format!(
                            "{error}; unexpected rollback staging preserved at {}",
                            candidate.display()
                        )));
                    }
                    if let Err(validation) = self.ensure_versions_dir(plugin_id, true) {
                        return Err(InstallError::io(format!(
                            "{error}; retained package preserved at {} because its store changed: {validation}",
                            candidate.display()
                        )));
                    }
                    if let Err(restore) = move_dir(&candidate, &retained_dir) {
                        // The rollback candidate may be the only copy of the
                        // retained version. Never delete it when restoration
                        // fails; startup cleanup also preserves nonempty staging.
                        return Err(InstallError::io(format!(
                            "{error}; retained package preserved at {} because restoration failed: {restore}",
                            candidate.display()
                        )));
                    }
                }
                let _ = fs::remove_dir_all(&staging);
                return Err(error);
            }
        };
        let _ = fs::remove_dir_all(&staging);
        self.ensure_versions_dir(plugin_id, false).map_err(|error| {
            InstallError::io(format!("package metadata changed during rollback; package may already have changed: {error}"))
        })?;
        let base = retained_receipt.unwrap_or_else(|| adopted_receipt(&manifest, now()));
        let _ = fs::remove_file(&retained_receipt_path);
        let _ = fs::remove_file(self.layout.retained_local_proof(plugin_id, &target_version));
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
        if let Some(expected) = expected_local_proof {
            self.record_local_proof(
                &self.layout.package(plugin_id),
                &receipt,
                &self.layout.installed_local_proof(plugin_id),
                &expected,
            );
        }
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
        self.try_lock_store_for(plugin_id)?
            .uninstall(plugin_id, purge_data, hooks)
    }

    fn uninstall_under_store_lock(
        &self,
        plugin_id: &str,
        purge_data: Option<&Path>,
        hooks: &dyn ActivationHooks,
    ) -> Result<(), InstallError> {
        let _guard = self.locks.try_lock(plugin_id)?;
        self.ensure_versions_dir(plugin_id, false)?;
        self.ensure_store_child(Path::new("local-proofs"), false)?;
        self.ensure_store_child(Path::new("receipts"), false)?;
        let target = self.checked_package_dir(plugin_id)?;
        let was_running = hooks.deactivate(plugin_id).map_err(InstallError::io)?;
        let after_deactivate = self
            .ensure_store_child(Path::new("receipts"), false)
            .and_then(|_| self.ensure_store_child(Path::new("local-proofs"), false))
            .and_then(|_| self.ensure_versions_dir(plugin_id, false));
        if let Err(error) = after_deactivate {
            let resume = hooks.activate(plugin_id, was_running);
            return Err(InstallError::io(format!("package metadata changed while stopping plugin: {error}; runtime recovery: {resume:?}")));
        }
        let trash = match self.layout.new_staging_dir("removed") {
            Ok(trash) => trash,
            Err(error) => {
                return Err(match hooks.activate(plugin_id, was_running) {
                    Ok(()) => InstallError::io(format!("could not prepare plugin removal: {error}")),
                    Err(restore) => InstallError::io(format!(
                        "could not prepare plugin removal and runtime recovery failed: {error}; {restore}"
                    )),
                });
            }
        };
        if let Err(error) = fs::rename(&target, trash.join(plugin_id)) {
            let _ = fs::remove_dir_all(&trash);
            return Err(match hooks.activate(plugin_id, was_running) {
                Ok(()) => InstallError::io(format!("could not remove plugin package: {error}")),
                Err(restore) => InstallError::io(format!(
                    "could not remove plugin package and runtime recovery failed: {error}; {restore}"
                )),
            });
        }
        if let Err(error) = hooks.activate(plugin_id, false) {
            let saved = trash.join(plugin_id);
            return match fs::rename(&saved, &target) {
                Ok(()) => {
                    let resumed = hooks.activate(plugin_id, was_running);
                    let _ = fs::remove_dir_all(&trash);
                    match resumed {
                        Ok(()) => Err(InstallError::io(format!(
                            "could not refresh plugins after uninstall; package restored: {error}"
                        ))),
                        Err(resume) => Err(InstallError::io(format!(
                            "could not refresh plugins after uninstall; package restored but runtime recovery failed: {error}; {resume}"
                        ))),
                    }
                }
                Err(restore) => Err(InstallError::io(format!(
                    "could not refresh plugins after uninstall; package preserved at {} because restoration failed: {error}; {restore}",
                    saved.display()
                ))),
            };
        }
        if let Err(error) = self
            .ensure_store_child(Path::new("receipts"), false)
            .and_then(|_| self.ensure_versions_dir(plugin_id, false))
        {
            return Err(InstallError::io(format!(
                "package metadata changed during removal: {error}; package preserved at {}",
                trash.join(plugin_id).display()
            )));
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
        if !is_valid_plugin_id(plugin_id) {
            return Err(InstallError::new(
                InstallErrorCode::InvalidPackage,
                "invalid plugin ID",
            ));
        }
        self.ensure_versions_dir(plugin_id, false)?;
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
                    .filter(|entry| {
                        let path = self.layout.retained(plugin_id, &entry.version);
                        Self::retained_matches(&path, plugin_id, &entry.version)
                            || (!path.exists()
                                && self.legacy_retained(plugin_id, &entry.version).is_some_and(
                                    |legacy| {
                                        Self::retained_matches(&legacy, plugin_id, &entry.version)
                                    },
                                ))
                    })
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
        self.ensure_store_child(Path::new("local-proofs"), true)?;
        self.ensure_store_child(Path::new("receipts"), true)?;
        if previous.is_some() {
            self.ensure_versions_dir(plugin_id, true)?;
        }
        if let Some(state) = previous {
            for entry in &state.receipt.retained {
                self.migrate_legacy_retained(plugin_id, &entry.version)?;
            }
            self.migrate_legacy_retained(plugin_id, &state.receipt.version)?;
            let destination = self.layout.retained(plugin_id, &state.receipt.version);
            match fs::symlink_metadata(&destination) {
                Ok(_)
                    if !Self::retained_matches(&destination, plugin_id, &state.receipt.version) =>
                {
                    return Err(InstallError::new(
                        InstallErrorCode::InvalidPackage,
                        "retained destination contains unexpected data",
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        // Invalidate old proofs before changing either path. Older releases
        // may have left stale receipts when receipt writes failed, so they
        // cannot acquire the new proof retroactively.
        let installed_proof = self.layout.installed_local_proof(plugin_id);
        let previous_local = previous.and_then(|state| {
            self.matching_local_proof(&state.directory, &state.receipt, &installed_proof)
                .then(|| self.local_proof(&state.directory, &state.receipt).ok())
                .flatten()
        });
        remove_optional_file(&installed_proof)?;
        if let Some(state) = previous {
            remove_optional_file(
                &self
                    .layout
                    .retained_local_proof(plugin_id, &state.receipt.version),
            )?;
        }
        let was_running = hooks.deactivate(plugin_id).map_err(InstallError::io)?;
        let after_deactivate = self
            .ensure_store_child(Path::new("receipts"), false)
            .and_then(|_| self.ensure_store_child(Path::new("local-proofs"), false))
            .and_then(|_| self.ensure_versions_dir(plugin_id, false));
        if let Err(error) = after_deactivate {
            let resume = hooks.activate(plugin_id, was_running);
            return Err(InstallError::io(format!("package metadata changed while stopping plugin: {error}; runtime recovery: {resume:?}")));
        }
        let mut retained_to = None;
        if let Some(state) = previous {
            let destination = self.layout.retained(plugin_id, &state.receipt.version);
            if fs::symlink_metadata(&destination).is_ok() {
                if !Self::retained_matches(&destination, plugin_id, &state.receipt.version) {
                    let resume = hooks.activate(plugin_id, was_running);
                    return Err(InstallError::io(format!("retained destination changed while stopping plugin; runtime recovery: {resume:?}")));
                }
                if let Err(error) = fs::remove_dir_all(&destination) {
                    let _ = hooks.activate(plugin_id, was_running);
                    return Err(error.into());
                }
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
            } else if let Some(expected) = previous_local {
                self.record_local_proof(
                    &destination,
                    &state.receipt,
                    &self
                        .layout
                        .retained_local_proof(plugin_id, &state.receipt.version),
                    &expected,
                );
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
            let mut preserved = None;
            if let Err(move_error) = move_dir(&target, candidate) {
                let failed = self.layout.new_staging_dir("failed")?;
                let saved = failed.join(plugin_id);
                if let Err(save_error) = move_dir(&target, &saved) {
                    // Do not remove either location: the target can still be
                    // the only copy, and no rollback can safely overwrite it.
                    return Err(InstallError::io(format!(
                        "plugin {plugin_id} could not be activated: {message}; could not preserve candidate: {move_error}; {save_error}"
                    )));
                }
                preserved = Some(saved);
            }
            let restore_error = retained_to.as_ref().and_then(|retained| {
                move_dir(retained, &target)
                    .err()
                    .map(|error| (retained, error))
            });
            let restart_error = hooks.activate(plugin_id, was_running).err();
            let mut detail = preserved
                .map(|path| format!("; candidate preserved at {}", path.display()))
                .unwrap_or_default();
            if let Some((path, error)) = restore_error {
                detail.push_str(&format!(
                    "; previous package remains at {}: {error}",
                    path.display()
                ));
            }
            if let Some(error) = restart_error {
                detail.push_str(&format!("; runtime recovery failed: {error}"));
            }
            return Err(InstallError::io(format!(
                "plugin {plugin_id} could not be activated: {message}{detail}"
            )));
        }
        Ok(())
    }

    fn finalize(&self, mut receipt: InstallReceipt) -> Result<InstallReceipt, InstallError> {
        self.ensure_versions_dir(&receipt.id, false)?;
        self.ensure_store_child(Path::new("receipts"), true)?;
        receipt.retained.sort_by_key(|entry| entry.retained_at);
        while receipt.retained.len() > self.retain {
            let removed = receipt.retained.remove(0);
            let directory = self.layout.retained(&receipt.id, &removed.version);
            if !Self::retained_matches(&directory, &receipt.id, &removed.version) {
                continue;
            }
            if directory.exists() {
                fs::remove_dir_all(&directory)?;
            }
            let _ = fs::remove_file(self.layout.retained_receipt(&receipt.id, &removed.version));
            let _ = fs::remove_file(
                self.layout
                    .retained_local_proof(&receipt.id, &removed.version),
            );
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
