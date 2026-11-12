//! Rollback of retained packages, including local provenance checks and restoration when
//! activation fails.

use super::{adopted_receipt, ActivationHooks, InstallOutcome, Installer};
use crate::{
    jobs::{JobReporter, JobStage},
    layout::{move_dir, now},
    receipt::read_receipt,
    ChangeKind, InstallError, InstallErrorCode, InstallReceipt, RetainedVersion,
};
use std::{fs, path::Path};
use tiktools_plugin_api::PluginManifest;
use tiktools_plugin_package::MANIFEST_FILE;

impl Installer {
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

    pub(super) fn rollback_under_store_lock(
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
}
