//! Explicit recovery for historical install flags whose package is absent.
//! No package, plugin data, receipt, conflict or Cloud queue is deleted.
use crate::{
    control::{OperationError, WorkspaceInventoryReview},
    AppCore,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::Arc};

impl AppCore {
    pub(crate) fn review_missing_plugin_inventory(
        &self,
        session: Arc<cloud_client::Session>,
        expected: Option<&str>,
    ) -> Result<WorkspaceInventoryReview, OperationError> {
        if expected.is_some_and(|value| {
            value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(OperationError::invalid("invalid inventory review"));
        }
        let _runtime = self.try_plugin_sync_runtime_guard()?;
        let _emission = self.try_plugin_source_lease()?;
        let packages = self.plugin_packages().map_err(|_| unavailable())?;
        let store = packages
            .installer
            .try_lock_store()
            .map_err(|_| unavailable())?;
        if !store
            .matches_roots(&self.db.paths().plugins, &self.db.paths().plugin_store())
            .map_err(|_| unavailable())?
        {
            return Err(unavailable());
        }
        let quarantine = self
            .plugin_state
            .apply_quarantine
            .read()
            .map_err(|_| unavailable())?;
        if !quarantine.as_ref().is_some_and(|items| items.is_empty()) {
            return Err(unavailable());
        }
        let _identity = self
            .cloud
            .auth()
            .try_session_emission_permit(&session)
            .map_err(|_| unavailable())?;
        let (result, mut activation) = self.db.reconcile_plugin_inventory(
            &session.instance_origin(),
            session.account_id(),
            |tx, source| {
                self.plugins
                    .with_complete_cutover_catalog(|revision, plugins| {
                        let catalog: BTreeSet<_> = plugins
                            .iter()
                            .map(|plugin| plugin.manifest.id.clone())
                            .collect();
                        let activation = self
                            .plugin_state
                            .activation
                            .write()
                            .map_err(|_| unavailable())?;
                        let missing: Vec<_> = source
                            .rows
                            .iter()
                            .filter(|row| {
                                row.state.is_some_and(|(installed, _)| installed)
                                    && !catalog.contains(&row.id)
                            })
                            .map(|row| row.id.clone())
                            .collect();
                        for id in &missing {
                            if store.inspect_verified(id).map_err(|_| unavailable())?
                                != tiktools_plugin_installer::InstalledPackageEvidence::Absent
                                || self.plugins.observe_worker(id).map_err(|_| unavailable())?
                                    != tiktools_plugin_loader::PluginWorkerObservation::Stopped
                            {
                                return Err(unavailable());
                            }
                        }
                        let evidence = format!(
                            "inventory-v1|{}|{}|{source:?}|{revision}|{catalog:?}|{activation:?}",
                            session.instance_origin(),
                            session.account_id()
                        );
                        let review = Sha256::digest(evidence.as_bytes())
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>();
                        if let Some(expected) = expected {
                            if expected != review {
                                return Err(OperationError::conflict(
                                    "inventory changed; review the missing packages again",
                                ));
                            }
                            self.cloud
                                .auth()
                                .with_current_session(&session, || {
                                    crate::db::DatabaseManager::archive_missing_plugin_flags(
                                        tx, &missing, &review,
                                    )
                                })
                                .map_err(|_| unavailable())??;
                        }
                        Ok((
                            WorkspaceInventoryReview {
                                review,
                                plugins: missing,
                            },
                            activation,
                        ))
                    })
                    .map_err(|_| unavailable())?
            },
        )?;
        // Publish the runtime mirror only after the durable transaction committed.
        if expected.is_some() {
            for id in &result.plugins {
                if let Some(state) = activation.get_mut(id) {
                    state.installed = false;
                }
            }
        }
        Ok(result)
    }
}
fn unavailable() -> OperationError {
    OperationError::unavailable("local inventory changed or is busy; retry the review")
}
