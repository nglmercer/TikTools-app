use super::*;

impl AppCore {
    pub fn plugin_package_rollback(
        self: &Arc<Self>,
        plugin_id: &str,
        version: Option<String>,
    ) -> Result<JobSnapshot, OperationError> {
        let packages = self.packages()?;
        let job = Arc::new(self.start_job(&packages, JobKind::Rollback, Some(plugin_id), None)?);
        let snapshot = job
            .snapshot()
            .ok_or_else(|| OperationError::internal("job vanished"))?;
        let core = Arc::clone(self);
        let plugin_id = plugin_id.to_owned();
        tokio::spawn(async move {
            let installer = packages.installer.clone();
            let worker_core = Arc::clone(&core);
            let worker_job = Arc::clone(&job);
            let result = tokio::task::spawn_blocking(move || {
                let _sideload = packages.try_sideload_permit()?;
                let _invocation =
                    worker_core
                        .plugin_state
                        .invocation_gate
                        .lock()
                        .map_err(|_| {
                            crate::plugin_packages::unavailable(
                                "plugin invocation lease is unavailable",
                            )
                        })?;
                #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
                let _source_lease = crate::plugin_packages::try_product_mutation_lease(&worker_core)?;
                #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
                let package_sync = crate::plugin_packages::capture_package_sync_account(&worker_core);
                #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
                crate::plugin_packages::reserve_package_sync_source(&worker_core, &plugin_id)?;
                #[cfg(all(feature = "persistence", not(feature = "cloud-auth")))]
                worker_core
                    .db
                    .invalidate_plugin_package_source(&plugin_id)
                    .map_err(InstallError::io)?;
                let activation = CoreActivation::new(&worker_core);
                let outcome = installer.rollback_local(
                    &plugin_id,
                    version.as_deref(),
                    &activation,
                    worker_job.as_ref(),
                );
                #[cfg(feature = "persistence")]
                let _final_generation = worker_core.db.invalidate_plugin_package_source(&plugin_id).map_err(|error| {
                    worker_core.rebuild_processor_index();
                    worker_core.refresh_automation_snapshot();
                    InstallError::io(format!(
                        "package rollback may have changed files but its final source fence failed: {error}"
                    ))
                })?;
                let outcome = outcome?;
                activation.finish()?;
                #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
                crate::plugin_packages::confirm_package_sync_source(&worker_core, package_sync, &outcome.receipt.id, &outcome.receipt.version, _final_generation);
                Ok(outcome)
            })
            .await
            .unwrap_or_else(|error| Err(InstallError::io(error)));
            core.finish_package_job(&job, result);
        });
        Ok(snapshot)
    }
}
