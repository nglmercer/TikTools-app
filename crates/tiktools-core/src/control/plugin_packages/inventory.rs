use super::*;

impl AppCore {
    pub async fn plugin_package_versions(
        self: &Arc<Self>,
        plugin_id: &str,
    ) -> Result<PluginVersions, OperationError> {
        let packages = self.packages()?;
        let installed = packages
            .installer
            .versions(plugin_id)
            .map_err(OperationError::install)?;
        let registry_name = installed
            .receipt
            .as_ref()
            .and_then(|receipt| receipt.origin.registry_name().map(ToOwned::to_owned));
        let host = packages.installer.host().clone();
        let lookup = registry_package_live(&packages, plugin_id, registry_name.as_deref()).await;
        let (registry, available, registry_error) = match lookup {
            Ok((source, document, live)) => {
                let mut available: Vec<AvailableRelease> = document
                    .releases
                    .iter()
                    .map(|release| AvailableRelease {
                        version: release.version.clone(),
                        channel: release.channel.clone(),
                        published_at: release.published_at,
                        yanked: release.yanked,
                        advisory: release.advisory.clone(),
                        compatible: release_is_compatible(release, &host),
                        live_status: live
                            .get(&release.version)
                            .map_or(LiveReleaseStatus::Unknown, |entry| entry.status),
                        changelog: release.changelog.clone(),
                    })
                    .collect();
                available.reverse();
                (Some(source.name), available, None)
            }
            Err(error) => (registry_name, Vec::new(), Some(error.to_string())),
        };
        Ok(PluginVersions {
            installed,
            registry,
            available,
            registry_error,
        })
    }

    pub fn plugin_package_receipts(&self) -> Result<Vec<InstallReceipt>, OperationError> {
        self.packages()?
            .installer
            .receipts()
            .list()
            .map_err(OperationError::install)
    }

    pub fn plugin_package_policy(
        &self,
        plugin_id: &str,
        patch: PolicyPatch,
    ) -> Result<InstallReceipt, OperationError> {
        let packages = self.packages()?;
        let mut store = packages
            .installer
            .try_lock_store()
            .map_err(OperationError::install)?;
        let receipt = store
            .receipt(plugin_id)
            .map_err(OperationError::install)?
            .ok_or_else(|| OperationError::install(InstallError::not_installed(plugin_id)))?;
        let mut policy = receipt.policy;
        if let Some(channel) = patch.channel {
            if !matches!(channel.as_str(), "stable" | "beta") {
                return Err(OperationError::invalid(
                    "channel must be `stable` or `beta`",
                ));
            }
            policy.channel = channel;
        }
        if let Some(auto_update) = patch.auto_update {
            policy.auto_update = auto_update;
        }
        if let Some(pinned) = patch.pinned {
            policy.pinned = pinned;
        }
        store
            .set_policy(plugin_id, policy)
            .map_err(OperationError::install)
    }
}

fn release_is_compatible(
    release: &tiktools_plugin_registry::Release,
    host: &tiktools_plugin_api::HostCompat,
) -> bool {
    let Some(runtime) = release.runtime_kind() else {
        return false;
    };
    release.artifacts.iter().any(|artifact| {
        if !host.accepts_artifact_target(&artifact.target) {
            return false;
        }
        let targets = if artifact.target == tiktools_plugin_registry::ANY_TARGET {
            Vec::new()
        } else {
            vec![artifact.target.clone()]
        };
        host.check(release.requirements(runtime, &targets)).is_ok()
    })
}
