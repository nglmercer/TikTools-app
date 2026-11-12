//! Serialized native package install/removal and durable source fences.
use super::*;

impl AppCore {
    #[cfg(feature = "plugin-install")]
    pub fn install_plugin(
        &self,
        archive: impl AsRef<std::path::Path>,
        replace_existing: bool,
    ) -> Result<tiktools_plugin_installer::InstallOutcome, tiktools_plugin_installer::InstallError>
    {
        use tiktools_plugin_installer::{
            CommitOptions, Consent, NoopReporter, PackageOrigin, SignaturePolicy,
        };
        let packages = self.plugin_packages()?;
        let prepared = packages.installer.prepare(
            archive.as_ref(),
            PackageOrigin::File,
            SignaturePolicy::AllowWithConsent,
        )?;
        let _sideload = packages.try_sideload_permit()?;
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                plugin_packages::unavailable("plugin invocation lease is unavailable")
            })?;
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        let _source_lease = plugin_packages::try_product_mutation_lease(self)?;
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        let package_sync = plugin_packages::capture_package_sync_account(self);
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        plugin_packages::reserve_package_sync_source(self, &prepared.manifest.id)?;
        #[cfg(all(feature = "persistence", not(feature = "cloud-auth")))]
        self.db
            .invalidate_plugin_package_source(&prepared.manifest.id)
            .map_err(tiktools_plugin_installer::InstallError::io)?;
        let activation = plugin_packages::CoreActivation::new(self);
        let outcome = packages.installer.commit(
            &prepared,
            &CommitOptions {
                consent: Consent::Implicit { replace_existing },
                policy: None,
            },
            &activation,
            &NoopReporter,
        );
        #[cfg(feature = "persistence")]
        let _final_generation = self.db.invalidate_plugin_package_source(&prepared.manifest.id).map_err(|error| {
            self.rebuild_processor_index(); self.refresh_automation_snapshot();
            tiktools_plugin_installer::InstallError::io(format!("package operation may have changed files but its final source fence failed: {error}"))
        })?;
        let outcome = outcome?;
        self.rebuild_processor_index();
        activation.finish()?;
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        plugin_packages::confirm_package_sync_source(
            self,
            package_sync,
            &outcome.receipt.id,
            &outcome.receipt.version,
            _final_generation,
        );
        Ok(outcome)
    }

    #[cfg(feature = "plugin-install")]
    pub fn uninstall_plugin(
        &self,
        id: &str,
        purge_data: bool,
    ) -> Result<(), tiktools_plugin_installer::InstallError> {
        use tiktools_plugin_installer::{InstallError, InstallErrorCode};
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                plugin_packages::unavailable("plugin invocation lease is unavailable")
            })?;
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        let _source_lease = plugin_packages::try_product_mutation_lease(self)?;
        let packages = self.plugin_packages()?;
        let plugin = self
            .plugins
            .get(id)
            .ok_or_else(|| InstallError::not_installed(id))?;
        if plugin.source != tiktools_plugin_loader::PluginSource::User {
            return Err(InstallError::new(
                InstallErrorCode::Unsupported,
                "only user-installed plugin packages can be uninstalled",
            ));
        }
        let data = purge_data
            .then(|| plugin_packages::plugin_data_dir(self.db.paths(), id))
            .flatten();
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        let package_sync = plugin_packages::capture_package_sync_account(self);
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        let source_generation = plugin_packages::reserve_package_sync_source(self, id)?;
        #[cfg(all(feature = "persistence", not(feature = "cloud-auth")))]
        let source_generation = self
            .db
            .invalidate_plugin_package_source(id)
            .map_err(InstallError::io)?;
        let activation = plugin_packages::CoreActivation::new(self);
        let removal = packages
            .installer
            .uninstall(id, data.as_deref(), &activation);
        #[cfg(feature = "persistence")]
        if removal.is_err() {
            if let Err(error) = self.db.invalidate_plugin_package_source(id) {
                self.rebuild_processor_index();
                self.refresh_automation_snapshot();
                return Err(InstallError::io(format!(
                    "package removal may have changed files but its final source fence failed: {error}"
                )));
            }
        }
        removal?;
        self.clear_plugin_activation(id);
        self.rebuild_processor_index();
        #[cfg(feature = "persistence")]
        match self
            .db
            .remove_plugin_state_after_package(id, source_generation)
        {
            Ok(true) => {}
            Ok(false) => {
                self.refresh_automation_snapshot();
                return Err(InstallError::new(
                    InstallErrorCode::IoError,
                    "plugin package was removed but its persisted state changed; review required",
                ));
            }
            Err(error) => {
                // The transaction rolls back its final generation on error.
                // Preserve a separate fence after filesystem mutation even
                // when persisted-state cleanup fails.
                let fence = self.db.invalidate_plugin_package_source(id);
                self.refresh_automation_snapshot();
                if let Err(fence_error) = fence {
                    return Err(InstallError::io(format!(
                        "plugin was removed, persisted state cleanup failed, and final source fence failed: {fence_error}"
                    )));
                }
                return Err(InstallError::io(format!(
                    "plugin package was removed but persisted state could not be cleared: {error}"
                )));
            }
        }
        activation.finish()?;
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        if let Some(generation) = source_generation.checked_add(1) {
            plugin_packages::confirm_package_removal_source(self, package_sync, id, generation);
        }
        Ok(())
    }

    #[cfg(feature = "plugin-install")]
    pub(crate) fn plugin_packages(
        &self,
    ) -> Result<&Arc<plugin_packages::PluginPackages>, tiktools_plugin_installer::InstallError>
    {
        self.packages
            .as_ref()
            .map_err(|error| plugin_packages::unavailable(error))
    }
}
