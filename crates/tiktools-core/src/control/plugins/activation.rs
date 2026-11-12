//! Local installed/enabled state and explicit runtime start/stop operations. Fallbacks retain
//! the original source-write and invocation boundaries.

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
use super::local_sync::LocalMutationProtocol;
use crate::{control::OperationError, events::DomainEvent, recover_rwlock_read, AppCore};

impl AppCore {
    #[allow(clippy::needless_return)]
    pub fn plugin_set_installed(&self, id: &str, installed: bool) -> Result<(), OperationError> {
        #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
        {
            let attempted = std::cell::Cell::new(false);
            return match self.plugin_set_installed_with_product_intent_for(
                id,
                installed,
                Some(&attempted),
            ) {
                Err(_) if !attempted.get() => self.plugin_set_installed_untracked(id, installed),
                result => result,
            };
        }
        #[cfg(not(all(feature = "cloud-auth", feature = "persistence")))]
        self.plugin_set_installed_untracked(id, installed)
    }

    fn plugin_set_installed_untracked(
        &self,
        id: &str,
        installed: bool,
    ) -> Result<(), OperationError> {
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
        let _source_lease = self.try_plugin_source_lease()?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        #[cfg(feature = "cloud-auth")]
        let version = plugin.manifest.version.clone();
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        self.db
            .set_plugin_state_with_untracked_sync(&id, installed, true)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        #[cfg(all(feature = "persistence", not(feature = "cloud-auth")))]
        self.db
            .set_plugin_state(&id, installed, true)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        if !installed {
            // Stopping a non-running plugin is a no-op success; surface real
            // shutdown failures.
            self.plugins.stop(&id)?;
        }
        self.set_plugin_activation(&id, installed, true);
        drop(_invocation);
        if installed {
            self.start_enabled_event_subscribers();
        }
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        #[cfg(feature = "cloud-auth")]
        if installed {
            self.stage_selection(&id, &version, true);
        } else {
            self.stage_removal(&id);
        }
        Ok(())
    }

    #[allow(clippy::needless_return)]
    pub fn plugin_set_enabled(&self, id: &str, enabled: bool) -> Result<(), OperationError> {
        #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
        {
            // Fall back only when admission failed before any source write.
            // An uncertain runtime or SQLite result must never be repeated.
            let attempted = std::cell::Cell::new(false);
            return match self.plugin_set_enabled_with_durable_intent_for(
                id,
                enabled,
                LocalMutationProtocol::Selected,
                Some(&attempted),
            ) {
                Err(_) if !attempted.get() => {
                    self.plugin_set_enabled_with_untracked_intent(id, enabled)
                }
                result => result,
            };
        }
        #[cfg(not(all(feature = "cloud-auth", feature = "persistence")))]
        {
            let plugin = self.require_discovered(id)?;
            let id = plugin.manifest.id.clone();
            #[cfg(feature = "cloud-auth")]
            let version = plugin.manifest.version.clone();
            self.set_enabled_impl(&id, enabled, false)?;
            #[cfg(feature = "cloud-auth")]
            self.stage_selection(&id, &version, enabled);
            Ok(())
        }
    }

    /// Exercise selected-protocol admission without the public fallback.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn plugin_set_enabled_with_product_intent(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<(), OperationError> {
        self.plugin_set_enabled_with_durable_intent_for(
            id,
            enabled,
            LocalMutationProtocol::Selected,
            None,
        )
    }

    /// Prototype for local operation when Cloud admission cannot be opened.
    /// The marker and local source commit atomically while excluding a
    /// cooperating product emission. No Cloud identity is inferred here.
    /// This remains private until every product emitter uses this fence and
    /// a user-facing reconciliation flow exists.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn plugin_set_enabled_with_untracked_intent(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<(), OperationError> {
        let invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id;
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        let fence = tiktools_cloud_sync::ProductEmissionFence::open(
            self.db.paths().data.join("plugin-sync-emission"),
        )
        .map_err(|_| OperationError::unavailable("sync emission fence unavailable"))?;
        let lease = fence
            .try_untracked_edit()
            .map_err(|_| OperationError::unavailable("sync emission is in progress"))?;
        self.db
            .set_plugin_state_with_untracked_sync(&id, true, enabled)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        if enabled {
            self.plugins.start_generation_before_events(&id, || {
                self.set_plugin_activation(&id, true, true);
            })?;
        } else {
            self.plugins.stop(&id)?;
            self.set_plugin_activation(&id, true, false);
        }
        drop(lease);
        drop(invocation);
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        self.events.publish_domain(if enabled {
            DomainEvent::PluginStarted { plugin_id: id }
        } else {
            DomainEvent::PluginStopped { plugin_id: id }
        });
        Ok(())
    }

    #[allow(dead_code)]
    fn set_enabled_impl(
        &self,
        id: &str,
        enabled: bool,
        only_installed: bool,
    ) -> Result<bool, OperationError> {
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
        let _source_lease = self.try_plugin_source_lease()?;
        if self.plugin_apply_quarantined(id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        if only_installed {
            // Check the installed state under the same gate as the mutation:
            // a concurrent uninstall must never be undone by a remote value.
            if self.require_discovered(id).is_err() {
                return Ok(false);
            }
            let activation =
                recover_rwlock_read(&self.plugin_state.activation, "plugin activation");
            if let Some(state) = activation.get(id) {
                if !state.installed || state.enabled == enabled {
                    return Ok(false);
                }
            } else if enabled {
                // Discovered plugins without an activation row default to
                // installed and enabled, matching plugin_ready().
                return Ok(false);
            }
        }
        #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
        if only_installed {
            // Legacy remote application is not a local edit and must not
            // feed the outbound reconciliation marker.
            self.db
                .set_plugin_state(id, true, enabled)
                .map_err(|error| OperationError::internal(error.to_string()))?;
        } else {
            self.db
                .set_plugin_state_with_untracked_sync(id, true, enabled)
                .map_err(|error| OperationError::internal(error.to_string()))?;
        }
        #[cfg(all(feature = "persistence", not(feature = "cloud-auth")))]
        self.db
            .set_plugin_state(id, true, enabled)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let result = if enabled {
            self.plugins
                .start_generation_before_events(id, || {
                    self.set_plugin_activation(id, true, true);
                })
                .map(|_| ())
        } else {
            self.plugins.stop(id).map(|()| {
                self.set_plugin_activation(id, true, false);
            })
        };
        result?;
        drop(_invocation);
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        self.events.publish_domain(if enabled {
            DomainEvent::PluginStarted {
                plugin_id: id.to_owned(),
            }
        } else {
            DomainEvent::PluginStopped {
                plugin_id: id.to_owned(),
            }
        });
        Ok(true)
    }

    /// Legacy low-level operation, retained only for existing compatibility
    /// tests. It does not bind account, consent, Cloud revision or crash
    /// recovery and must not be called from a product sync path.
    #[cfg(all(feature = "cloud-auth", test))]
    pub(crate) fn plugin_apply_remote_selection(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<bool, OperationError> {
        let plugin = match self.require_discovered(id) {
            Ok(plugin) => plugin,
            Err(_) => return Ok(false),
        };
        self.set_enabled_impl(&plugin.manifest.id, enabled, true)
    }

    pub fn plugin_start(&self, id: &str) -> Result<(), OperationError> {
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
        let _source_lease = self.try_plugin_source_lease()?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        self.plugins.start(&id)?;
        drop(_invocation);
        self.events
            .publish_domain(DomainEvent::PluginStarted { plugin_id: id });
        Ok(())
    }

    pub fn plugin_stop(&self, id: &str) -> Result<(), OperationError> {
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
        let _source_lease = self.try_plugin_source_lease()?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.plugins.stop(&id)?;
        drop(_invocation);
        self.events
            .publish_domain(DomainEvent::PluginStopped { plugin_id: id });
        Ok(())
    }
}
