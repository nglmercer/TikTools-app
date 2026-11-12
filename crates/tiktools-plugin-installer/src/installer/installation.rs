//! Consent-checked installation and package replacement. The swap retains activation recovery
//! and candidate preservation ordering.

use super::{
    storage::remove_optional_file, ActivationHooks, CommitOptions, Consent, CurrentState,
    InstallOutcome, Installer, PreparedPackage,
};
use crate::{
    jobs::{JobReporter, JobStage},
    layout::{move_dir, now},
    native_libs::backfill_native_libs,
    receipt::write_receipt,
    InstallError, InstallErrorCode, InstallReceipt, PackagePreview, RetainedVersion,
};
use std::{fs, io, path::Path};

impl Installer {
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

    pub(super) fn commit_under_store_lock(
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

    pub(super) fn swap_in(
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
}
