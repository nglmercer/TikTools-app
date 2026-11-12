//! Private archive snapshots and consent previews. Preparation does not mutate the installed
//! package or execute plugin code.

use super::{Installer, PreparedPackage, SignaturePolicy};
use crate::{
    layout::now,
    preview::{consent_token, requires_sideload_consent},
    ChangeKind, CloudArtifactIdentity, InstallError, InstallErrorCode, PackageOrigin,
    PackagePreview, PermissionDiff, SignatureState, SignatureStatus,
};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    sync::Arc,
};
use tiktools_plugin_package::{PackageArchive, PackageError, PackageErrorKind};

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

impl Installer {
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
}
