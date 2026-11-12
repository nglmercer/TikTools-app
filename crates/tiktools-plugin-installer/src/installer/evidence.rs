//! Bounded observations and integrity proofs for installed and retained packages. These
//! checks run under the existing cooperative store lease.

use super::{InstalledPackageEvidence, Installer};
use crate::{
    layout::now, receipt::write_atomic, InstallError, InstallErrorCode, InstallReceipt,
    PackageOrigin, SignatureStatus,
};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};
use tiktools_plugin_api::{manifest::is_valid_plugin_id, PluginManifest};
use tiktools_plugin_package::{
    Checksums, PackageErrorKind, PackageSignature, Sha256Digest, CHECKSUMS_FILE, MANIFEST_FILE,
    MAX_FILES, MAX_MANIFEST_BYTES, SIGNATURE_FILE,
};

const MAX_INSTALLED_CHECKSUMS_BYTES: usize = 8 * 1024 * 1024;
const MAX_INSTALLED_SIGNATURE_BYTES: usize = 16 * 1024;
pub(super) const MAX_INSTALLED_RECEIPT_BYTES: usize = 256 * 1024;

/// Reads a bounded regular file without accepting a symlink as the final
/// component. This is an observation under the cooperative store lease; it
/// cannot fence unrelated processes that directly rewrite the filesystem.
pub(super) fn read_observed_file(
    path: &Path,
    limit: usize,
) -> Result<Option<Vec<u8>>, InstallError> {
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

pub(super) fn required_observed_file(path: &Path, limit: usize) -> Result<Vec<u8>, InstallError> {
    read_observed_file(path, limit)?.ok_or_else(|| {
        InstallError::new(
            InstallErrorCode::InvalidPackage,
            "installed package metadata is missing",
        )
    })
}

impl Installer {
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

    // The proof binds the retained tree to the local artifact previously
    // reviewed by this installer. It is an integrity check under the
    // cooperative store lease, not protection against a process that can
    // arbitrarily rewrite both the store and its proofs.
    pub(super) fn local_proof(
        &self,
        root: &Path,
        receipt: &InstallReceipt,
    ) -> Result<String, InstallError> {
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

    pub(super) fn matching_local_proof(
        &self,
        root: &Path,
        receipt: &InstallReceipt,
        path: &Path,
    ) -> bool {
        let Ok(Some(proof)) = read_observed_file(path, 64) else {
            return false;
        };
        self.local_proof(root, receipt)
            .is_ok_and(|expected| proof == expected.as_bytes())
    }

    pub(super) fn record_local_proof(
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

    pub(super) fn inspect_verified_under_store_lock(
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
}
