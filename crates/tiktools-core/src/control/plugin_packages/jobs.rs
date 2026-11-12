use super::*;

impl AppCore {
    pub fn plugin_jobs(&self) -> Result<Vec<JobSnapshot>, OperationError> {
        Ok(self.packages()?.jobs.list())
    }

    pub fn plugin_job(&self, job_id: &str) -> Result<JobSnapshot, OperationError> {
        self.packages()?
            .jobs
            .get(job_id)
            .ok_or_else(|| OperationError::not_found(format!("Job `{job_id}` does not exist.")))
    }

    pub fn plugin_job_cancel(&self, job_id: &str) -> Result<bool, OperationError> {
        Ok(self.packages()?.jobs.cancel(job_id))
    }

    pub(super) fn spawn_package_job(
        self: &Arc<Self>,
        kind: JobKind,
        spec: PackageSpec,
        consent: InstallConsent,
        policy: Option<UpdatePolicy>,
    ) -> Result<JobSnapshot, OperationError> {
        let packages = self.packages()?;
        let label = spec.to_string();
        let job =
            Arc::new(self.start_job(&packages, kind, spec.plugin_id(), Some(label.clone()))?);
        let snapshot = job
            .snapshot()
            .ok_or_else(|| OperationError::internal("job vanished"))?;
        let core = Arc::clone(self);
        tokio::spawn(async move {
            let result = core
                .run_package_job(&packages, &job, spec, label, consent, policy)
                .await;
            core.finish_package_job(&job, result);
        });
        Ok(snapshot)
    }

    pub(super) fn start_job(
        &self,
        packages: &PluginPackages,
        kind: JobKind,
        plugin_id: Option<&str>,
        spec: Option<String>,
    ) -> Result<JobHandle, OperationError> {
        if let Some(plugin_id) = plugin_id {
            if packages.installer.is_busy(plugin_id)
                || packages.jobs.active_for(plugin_id).is_some()
            {
                return Err(OperationError::install(InstallError::busy(plugin_id)));
            }
        }
        Ok(packages
            .jobs
            .start(kind, plugin_id.map(ToOwned::to_owned), spec))
    }

    pub(super) fn finish_package_job(
        &self,
        job: &JobHandle,
        result: Result<InstallOutcome, InstallError>,
    ) {
        match &result {
            Ok(outcome) => self.after_package_change(outcome),
            Err(error) => {
                tracing::warn!(job = %job.id(), code = ?error.code, %error, "plugin package job failed")
            }
        }
        job.finish(&result);
    }

    async fn run_package_job(
        self: &Arc<Self>,
        packages: &Arc<PluginPackages>,
        job: &Arc<JobHandle>,
        spec: PackageSpec,
        label: String,
        consent: InstallConsent,
        policy: Option<UpdatePolicy>,
    ) -> Result<InstallOutcome, InstallError> {
        let reviewed = match (&consent, &spec) {
            // A Cloud review never authorizes stale metadata. Resolve the
            // active generation and recheck yanks, keys, digest and the
            // exact consent token immediately before dispatching the commit.
            (InstallConsent::Reviewed(token), PackageSpec::Registry { .. }) => {
                let _ = packages.take_review(token, &label);
                None
            }
            (InstallConsent::Reviewed(token), _) => packages.take_review(token, &label),
            (InstallConsent::Automatic, _) => None,
        };
        let (mut prepared, trust_guard, target) = match reviewed {
            Some(prepared) => (prepared, None, None),
            None => {
                self.fetch_prepared(packages, &spec, job.as_ref(), &job.cancel_flag())
                    .await?
            }
        };
        let preview = packages.installer.preview(&prepared);
        job.describe(
            Some(&prepared.manifest.id),
            preview.installed_version.as_deref(),
            Some(&prepared.manifest.version),
        );
        let token = match consent {
            InstallConsent::Reviewed(token) => token,
            InstallConsent::Automatic => {
                let receipt = packages.installer.receipt(&prepared.manifest.id)?;
                let same_publisher = receipt.as_ref().is_some_and(|receipt| {
                    receipt.signature.is_verified()
                        && receipt.signature.publisher == preview.signature.publisher
                });
                if !preview.signature.is_verified()
                    || !same_publisher
                    || preview.permission_diff.adds_access()
                {
                    return Err(InstallError::new(
                        InstallErrorCode::ConsentRequired,
                        format!(
                            "{} needs review before it can be updated",
                            prepared.manifest.id
                        ),
                    )
                    .with_details(serde_json::to_value(&preview).unwrap_or_default()));
                }
                preview.consent_token.clone()
            }
        };
        job.check()?;
        if let PackageOrigin::Registry { registry, url } = &prepared.origin {
            let guard = trust_guard.as_ref().ok_or_else(|| {
                InstallError::new(
                    InstallErrorCode::Unsupported,
                    "Cloud trust review is unavailable",
                )
            })?;
            let generation = packages
                .registry()
                .check_fixed_generation(guard, registry, url)
                .await?;
            let target = target.as_deref().ok_or_else(|| {
                InstallError::new(
                    InstallErrorCode::Unsupported,
                    "Cloud artifact target is unavailable",
                )
            })?;
            tokio::time::timeout(
                std::time::Duration::from_secs(20),
                verify_current_cloud_release(&prepared, target, generation),
            )
            .await
            .map_err(|_| {
                InstallError::new(
                    InstallErrorCode::Unsupported,
                    "Cloud release status timed out",
                )
            })??;
            packages
                .registry()
                .check_fixed_generation(guard, registry, url)
                .await?;
            prepared.bind_verified_cloud_artifact(target, generation)?;
        }
        job.check()?;
        let installer = packages.installer.clone();
        let packages = Arc::clone(packages);
        let core = Arc::clone(self);
        let worker_job = Arc::clone(job);
        tokio::task::spawn_blocking(move || {
            // For Cloud packages this retains the exact verified trust
            // snapshot until the filesystem transaction completes.
            let _trust_guard = trust_guard;
            let _sideload = match &prepared.origin {
                PackageOrigin::File => Some(packages.try_sideload_permit()?),
                PackageOrigin::Registry { .. } => None,
                _ => return Err(InstallError::new(InstallErrorCode::Unsupported, "unsupported plugin package origin")),
            };
            let _invocation = core.plugin_state.invocation_gate.lock().map_err(|_| {
                crate::plugin_packages::unavailable("plugin invocation lease is unavailable")
            })?;
            #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
            let _source_lease = crate::plugin_packages::try_product_mutation_lease(&core)?;
            #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
            let package_sync = crate::plugin_packages::capture_package_sync_account(&core);
            #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
            crate::plugin_packages::reserve_package_sync_source(&core, &prepared.manifest.id)?;
            #[cfg(all(feature = "persistence", not(feature = "cloud-auth")))]
            core.db
                .invalidate_plugin_package_source(&prepared.manifest.id)
                .map_err(InstallError::io)?;
            let activation = CoreActivation::new(&core);
            let outcome = installer.commit(
                &prepared,
                &CommitOptions {
                    consent: Consent::Token(token),
                    policy,
                },
                &activation,
                worker_job.as_ref(),
            );
            #[cfg(feature = "persistence")]
            let _final_generation = core.db.invalidate_plugin_package_source(&prepared.manifest.id).map_err(|error| {
                core.rebuild_processor_index();
                core.refresh_automation_snapshot();
                InstallError::io(format!(
                    "package operation may have changed files but its final source fence failed: {error}"
                ))
            })?;
            let outcome = outcome?;
            activation.finish()?;
            #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
            crate::plugin_packages::confirm_package_sync_source(&core, package_sync, &outcome.receipt.id, &outcome.receipt.version, _final_generation);
            Ok(outcome)
        })
        .await
        .unwrap_or_else(|error| Err(InstallError::io(error)))
    }
}
