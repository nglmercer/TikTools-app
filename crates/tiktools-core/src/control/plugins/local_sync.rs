//! Cooperative source admission, durable sync reservations and exact runtime observations.
//! Source writes never perform network publication.

use crate::{control::OperationError, AppCore};

/// A local mutation's durable sync candidate. Public enable/disable uses the
/// selected protocol; other writers and the v2 drain remain to be integrated.
/// Legacy v1 staging remains separate until its drain joins this boundary.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
pub(super) struct ReservedLocalPluginMutation {
    candidate: Option<crate::db::PluginSyncOutboxEntry>,
    pub(super) legacy_staging: bool,
    source_lease: Option<tiktools_cloud_sync::ProductEmissionLease>,
}

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
impl std::fmt::Debug for ReservedLocalPluginMutation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReservedLocalPluginMutation")
            .field("candidate", &self.candidate)
            .field("legacy_staging", &self.legacy_staging)
            .field("source_lease", &self.source_lease.is_some())
            .finish()
    }
}

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
#[derive(Clone, Copy)]
pub(super) enum LocalMutationProtocol {
    V1,
    V2,
    Selected,
}

/// Holds the invocation boundary before the coordinator's short admission
/// lease. This is a momentary host-worker observation, not a promise that a
/// child process is healthy or that the observation survives this guard.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
pub(crate) struct PluginSyncRuntimeGuard<'a> {
    core: &'a AppCore,
    _invocation: std::sync::MutexGuard<'a, ()>,
}

/// Caller holds the plugin invocation gate throughout the callback.
#[cfg(all(
    feature = "cloud-auth",
    feature = "persistence",
    feature = "plugin-install"
))]
pub(crate) fn with_fenced_plugin_removal<T>(
    core: &AppCore,
    id: &str,
    operation: impl FnOnce() -> T,
) -> Option<T> {
    let quarantine = core.plugin_state.apply_quarantine.read().ok()?;
    if quarantine.as_ref()?.contains(id) {
        return None;
    }
    let packages = core.plugin_packages().ok()?;
    let store = packages.installer.try_lock_store().ok()?;
    if !store
        .matches_roots(&core.db.paths().plugins, &core.db.paths().plugin_store())
        .ok()?
    {
        return None;
    }
    let (_, plugins) = core.plugins.list_complete_for_cutover().ok()?;
    let present = plugins.iter().any(|plugin| plugin.manifest.id == id);
    let activation = core.plugin_state.activation.read().ok()?.get(id).copied();
    if activation.is_some_and(|state| state.installed) {
        return None;
    }
    if activation.is_none()
        && (present
            || store.inspect_verified(id).ok()?
                != tiktools_plugin_installer::InstalledPackageEvidence::Absent)
    {
        return None;
    }
    if core.plugins.observe_worker(id).ok()?
        != tiktools_plugin_loader::PluginWorkerObservation::Stopped
    {
        return None;
    }
    Some(operation())
}

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
impl PluginSyncRuntimeGuard<'_> {
    /// Hold the cooperative package store while admitting an observed local
    /// removal. Missing inventory, uncertain workers or a replacement plugin
    /// cannot be interpreted as an absent source.
    #[cfg(feature = "plugin-install")]
    pub(crate) fn with_removal<T>(&self, id: &str, operation: impl FnOnce() -> T) -> Option<T> {
        with_fenced_plugin_removal(self.core, id, operation)
    }

    #[cfg(not(feature = "plugin-install"))]
    pub(crate) fn with_removal<T>(&self, _id: &str, _operation: impl FnOnce() -> T) -> Option<T> {
        None
    }

    pub(crate) fn matches_selection(&self, id: &str, version: &str, enabled: bool) -> bool {
        let Ok(quarantine) = self.core.plugin_state.apply_quarantine.read() else {
            return false;
        };
        if quarantine
            .as_ref()
            .is_none_or(|blocked| blocked.contains(id))
        {
            return false;
        }
        drop(quarantine);
        let Ok(Some(plugin)) = self.core.plugins.get_for_admission(id) else {
            return false;
        };
        if plugin.manifest.id != id
            || plugin.manifest.version != version
            || (enabled && !plugin.available)
        {
            return false;
        }
        let Ok(activation) = self.core.plugin_state.activation.read() else {
            return false;
        };
        if !activation
            .get(id)
            .is_some_and(|value| value.installed && value.enabled == enabled)
        {
            return false;
        }
        drop(activation);
        let expected = if enabled {
            tiktools_plugin_loader::PluginWorkerObservation::WorkerPresent
        } else {
            tiktools_plugin_loader::PluginWorkerObservation::Stopped
        };
        self.core
            .plugins
            .observe_worker(id)
            .is_ok_and(|actual| actual == expected)
    }
}

/// Atomically records the desired local state and, when opted in, an
/// identity-bound candidate. The coordinator keeps protocol admission and
/// identity stable for the entire SQLite transaction. This helper performs
/// no plugin runtime operation and cannot publish to Cloud.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
#[allow(dead_code)]
fn reserve_local_plugin_mutation(
    sync: &tiktools_cloud_sync::SyncCoordinator,
    db: &crate::db::DatabaseManager,
    id: &str,
    version: &str,
    installed: bool,
    enabled: bool,
) -> Result<ReservedLocalPluginMutation, OperationError> {
    reserve_local_plugin_mutation_prepared(sync, db, id, version, installed, enabled, |_| Ok(()))
        .map(|(reservation, ())| reservation)
}

/// Prepare runtime confirmation while the exact opt-in/identity context is
/// held, before the SQLite source transaction. Local-only edits need no
/// runtime sync identity or entropy.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
fn reserve_local_plugin_mutation_prepared<T>(
    sync: &tiktools_cloud_sync::SyncCoordinator,
    db: &crate::db::DatabaseManager,
    id: &str,
    version: &str,
    installed: bool,
    enabled: bool,
    prepare: impl FnOnce(bool) -> Result<T, OperationError>,
) -> Result<(ReservedLocalPluginMutation, T), OperationError> {
    reserve_local_plugin_mutation_prepared_for(
        sync,
        db,
        id,
        version,
        installed,
        enabled,
        LocalMutationProtocol::V1,
        prepare,
    )
}

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn reserve_local_plugin_mutation_prepared_for<T>(
    sync: &tiktools_cloud_sync::SyncCoordinator,
    db: &crate::db::DatabaseManager,
    id: &str,
    version: &str,
    installed: bool,
    enabled: bool,
    protocol: LocalMutationProtocol,
    prepare: impl FnOnce(bool) -> Result<T, OperationError>,
) -> Result<(ReservedLocalPluginMutation, T), OperationError> {
    // Keep the cross-process gate through runtime confirmation, not merely
    // through the SQLite source transaction.
    let source_lease = if matches!(
        protocol,
        LocalMutationProtocol::V2 | LocalMutationProtocol::Selected
    ) {
        let fence = tiktools_cloud_sync::ProductEmissionFence::open(
            db.paths().data.join("plugin-sync-emission"),
        )
        .map_err(|_| OperationError::unavailable("sync emission fence unavailable"))?;
        Some(
            fence
                .try_untracked_edit()
                .map_err(|_| OperationError::unavailable("sync emission is in progress"))?,
        )
    } else {
        None
    };
    let operation = |context: Option<tiktools_cloud_sync::V1LocalEditContext>,
                     legacy_staging,
                     untracked|
     -> Result<(ReservedLocalPluginMutation, T), OperationError> {
        let prepared = prepare(context.is_some())?;
        let intent = context
            .as_ref()
            .map(|context| crate::db::LocalPluginSyncIntent {
                origin: &context.origin,
                account_id: &context.account_id,
                config_generation: context.config_generation,
                version: installed.then_some(version),
            });
        let generation = if untracked {
            db.set_plugin_state_with_untracked_sync(id, installed, enabled)
        } else {
            db.set_plugin_state_with_sync_intent(id, installed, enabled, intent)
        }
        .map_err(|error| OperationError::internal(error.to_string()))?;
        let candidate = context.map(|context| crate::db::PluginSyncOutboxEntry {
            origin: context.origin,
            account_id: context.account_id,
            plugin_id: id.to_owned(),
            generation,
            source_generation: generation,
            config_generation: context.config_generation,
            runtime_epoch: None,
            reviewed_base: Some("direct".to_owned()),
            version: installed.then(|| version.to_owned()),
            enabled: installed.then_some(enabled),
        });
        Ok((
            ReservedLocalPluginMutation {
                candidate,
                legacy_staging,
                source_lease: None,
            },
            prepared,
        ))
    };
    let (mut reservation, prepared) = match protocol {
        LocalMutationProtocol::V1 => {
            sync.with_v1_local_edit(|context| operation(context, false, false))
        }
        LocalMutationProtocol::V2 => {
            sync.with_v2_local_edit(|context| operation(context, false, false))
        }
        LocalMutationProtocol::Selected => sync.with_product_local_edit(|selected| {
            use tiktools_cloud_sync::ProductLocalEditContext;
            match selected {
                ProductLocalEditContext::Legacy => operation(None, true, true),
                ProductLocalEditContext::OptedOut | ProductLocalEditContext::V2(None) => {
                    operation(None, false, true)
                }
                ProductLocalEditContext::V2(Some(context)) => {
                    operation(Some(context), false, false)
                }
            }
        }),
    }
    .map_err(|_| OperationError::unavailable("Cloud sync admission is unavailable"))??;
    reservation.source_lease = source_lease;
    Ok((reservation, prepared))
}

/// Confirm only after the caller, under its plugin mutation gate, has
/// observed that the runtime reflects the reserved source state. Stale or
/// replaced reservations remain durable and cannot be acknowledged.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
#[allow(dead_code)]
pub(super) fn confirm_local_plugin_mutation(
    db: &crate::db::DatabaseManager,
    reservation: &ReservedLocalPluginMutation,
    runtime_epoch: &str,
) -> Result<(), OperationError> {
    let Some(candidate) = reservation.candidate.as_ref() else {
        return Ok(());
    };
    let confirmed = db
        .confirm_plugin_sync_runtime(candidate, runtime_epoch)
        .map_err(|error| OperationError::internal(error.to_string()))?;
    if !confirmed {
        return Err(OperationError::conflict(
            "plugin state changed before sync confirmation",
        ));
    }
    Ok(())
}

impl AppCore {
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    pub(super) fn try_plugin_source_lease(
        &self,
    ) -> Result<tiktools_cloud_sync::ProductEmissionLease, OperationError> {
        tiktools_cloud_sync::ProductEmissionFence::open(
            self.db.paths().data.join("plugin-sync-emission"),
        )
        .map_err(|_| OperationError::unavailable("sync emission fence unavailable"))?
        .try_untracked_edit()
        .map_err(|_| OperationError::unavailable("sync emission is in progress"))
    }
    /// Obtain before protocol admission. The returned guard must be released
    /// before any network request. Only cooperative callers sharing this core
    /// are fenced; filesystem writers and other processes remain separate.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn try_plugin_sync_runtime_guard(
        &self,
    ) -> Result<PluginSyncRuntimeGuard<'_>, OperationError> {
        let invocation =
            self.plugin_state.invocation_gate.try_lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        Ok(PluginSyncRuntimeGuard {
            core: self,
            _invocation: invocation,
        })
    }

    /// Stage a local selection change for upload. Inert unless sync is
    /// enabled and an account is known; never fails the local operation.
    #[cfg(feature = "cloud-auth")]
    pub(super) fn stage_selection(&self, id: &str, version: &str, enabled: bool) {
        let Ok(coordinator) = self.sync.coordinator() else {
            return;
        };
        if let Err(error) = coordinator.record_local_change(id, version, enabled) {
            tracing::warn!(plugin = %id, %error, "cloud sync staging failed");
        }
    }

    /// Stage a local uninstall for upload. Same opt-in and failure
    /// semantics as [`Self::stage_selection`].
    #[cfg(feature = "cloud-auth")]
    pub(super) fn stage_removal(&self, id: &str) {
        let Ok(coordinator) = self.sync.coordinator() else {
            return;
        };
        if let Err(error) = coordinator.record_local_removal(id) {
            tracing::warn!(plugin = %id, %error, "cloud sync staging failed");
        }
    }
}

#[cfg(all(test, feature = "cloud-auth", feature = "persistence"))]
mod tests;
