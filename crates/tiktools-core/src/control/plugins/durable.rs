//! Desired plugin state and runtime confirmation under one invocation gate. Failed or
//! interrupted effects retain unconfirmed durable intent.

use super::local_sync::{
    confirm_local_plugin_mutation, reserve_local_plugin_mutation_prepared_for,
    LocalMutationProtocol,
};
use crate::{control::OperationError, events::DomainEvent, AppCore};

impl AppCore {
    /// Native local enable/disable with an account-bound durable intent.
    /// The invocation gate spans the source transaction,
    /// runtime effect and confirmation, while the coordinator holds its
    /// shorter admission/identity lease only for the SQLite transaction.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn plugin_set_enabled_with_durable_intent(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<(), OperationError> {
        self.plugin_set_enabled_with_durable_intent_for(
            id,
            enabled,
            LocalMutationProtocol::V1,
            None,
        )
    }

    /// V2 counterpart for the future product path. This remains private until
    /// every local writer and package mutation joins the same admission and
    /// runtime reconciliation protocol.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn plugin_set_enabled_with_v2_durable_intent(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<(), OperationError> {
        self.plugin_set_enabled_with_durable_intent_for(
            id,
            enabled,
            LocalMutationProtocol::V2,
            None,
        )
    }

    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    pub(super) fn plugin_set_enabled_with_durable_intent_for(
        &self,
        id: &str,
        enabled: bool,
        protocol: LocalMutationProtocol,
        source_attempted: Option<&std::cell::Cell<bool>>,
    ) -> Result<(), OperationError> {
        let invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        let sync = self
            .sync
            .coordinator()
            .map_err(|_| OperationError::unavailable("Cloud sync admission is unavailable"))?;
        // Only an opted-in identity needs entropy, acquired before any source
        // write; local-only editing must not depend on a Cloud runtime epoch.
        let (reservation, runtime_epoch) = reserve_local_plugin_mutation_prepared_for(
            sync,
            &self.db,
            &id,
            &plugin.manifest.version,
            true,
            enabled,
            protocol,
            |has_candidate| {
                if let Some(source_attempted) = source_attempted {
                    source_attempted.set(true);
                }
                if has_candidate {
                    self.sync.runtime_epoch().map(Some).map_err(|_| {
                        OperationError::unavailable("plugin runtime identity is unavailable")
                    })
                } else {
                    Ok(None)
                }
            },
        )?;

        // A failed or interrupted start/stop keeps the committed candidate
        // unconfirmed; it must be reconciled rather than uploaded or replayed.
        if enabled {
            self.plugins.start_generation_before_events(&id, || {
                self.set_plugin_activation(&id, true, true);
            })?;
        } else {
            self.plugins.stop(&id)?;
            self.set_plugin_activation(&id, true, false);
        }
        let confirmation = match runtime_epoch {
            Some(epoch) => confirm_local_plugin_mutation(&self.db, &reservation, epoch),
            None => Ok(()),
        };
        drop(invocation);

        // Even when confirmation fails, the local effect already happened.
        // Refresh local projections before returning that failure to callers.
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        self.events.publish_domain(if enabled {
            DomainEvent::PluginStarted {
                plugin_id: id.clone(),
            }
        } else {
            DomainEvent::PluginStopped {
                plugin_id: id.clone(),
            }
        });
        if reservation.legacy_staging {
            self.stage_selection(&id, &plugin.manifest.version, enabled);
        }
        confirmation
    }

    /// Select the current protocol and reserve the source while holding the
    /// same short admission used by cutover. Runtime failure or an uncertain
    /// worker leaves the intent durable but unconfirmed for later review.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn plugin_set_installed_with_product_intent(
        &self,
        id: &str,
        installed: bool,
    ) -> Result<(), OperationError> {
        self.plugin_set_installed_with_product_intent_for(id, installed, None)
    }

    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    pub(super) fn plugin_set_installed_with_product_intent_for(
        &self,
        id: &str,
        installed: bool,
        source_attempted: Option<&std::cell::Cell<bool>>,
    ) -> Result<(), OperationError> {
        let invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        let sync = self
            .sync
            .coordinator()
            .map_err(|_| OperationError::unavailable("Cloud sync admission is unavailable"))?;
        let (reservation, runtime_epoch) = reserve_local_plugin_mutation_prepared_for(
            sync,
            &self.db,
            &id,
            &plugin.manifest.version,
            installed,
            true,
            LocalMutationProtocol::Selected,
            |has_candidate| {
                if let Some(source_attempted) = source_attempted {
                    source_attempted.set(true);
                }
                if has_candidate {
                    self.sync.runtime_epoch().map(Some).map_err(|_| {
                        OperationError::unavailable("plugin runtime identity is unavailable")
                    })
                } else {
                    Ok(None)
                }
            },
        )?;
        if !installed {
            self.plugins.stop(&id)?;
        }
        self.set_plugin_activation(&id, installed, true);
        let expected = if installed {
            tiktools_plugin_loader::PluginWorkerObservation::WorkerPresent
        } else {
            tiktools_plugin_loader::PluginWorkerObservation::Stopped
        };
        let confirmation = match runtime_epoch {
            Some(epoch)
                if self
                    .plugins
                    .observe_worker(&id)
                    .is_ok_and(|actual| actual == expected) =>
            {
                confirm_local_plugin_mutation(&self.db, &reservation, epoch)
            }
            Some(_) => {
                tracing::warn!(plugin = %id, "local plugin sync intent needs runtime reconciliation");
                Ok(())
            }
            None => Ok(()),
        };
        drop(invocation);
        if installed {
            self.start_enabled_event_subscribers();
        }
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        if reservation.legacy_staging {
            if installed {
                self.stage_selection(&id, &plugin.manifest.version, true);
            } else {
                self.stage_removal(&id);
            }
        }
        confirmation
    }
}
