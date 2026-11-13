//! Explicit v2 passes, pending-source staging and cache rebuild.
#[cfg(feature = "persistence")]
use super::sync_error;
use super::OperationError;
use super::WorkspaceV2SyncResult;
use crate::AppCore;

impl AppCore {
    async fn require_variable_sync(self: &std::sync::Arc<Self>) -> Result<(), OperationError> {
        #[cfg(feature = "persistence")]
        {
            let (session, _) = self.v2_identity().await?;
            crate::control::cloud_variables::require_variables(&session.instance_origin()).await
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "Cloud v2 variables require persistence",
            ))
        }
    }
    /// Explicit v2 pass for an already selected, account-scoped protocol.
    /// Supported downloaded plugin state changes are applied when eligible.
    pub async fn sync_v2_now(
        self: &std::sync::Arc<Self>,
    ) -> Result<WorkspaceV2SyncResult, OperationError> {
        self.sync_v2_with_restore(true).await
    }

    pub(crate) async fn sync_v2_with_restore(
        self: &std::sync::Arc<Self>,
        restore: bool,
    ) -> Result<WorkspaceV2SyncResult, OperationError> {
        self.sync_v2_scoped(restore, tiktools_cloud_sync::WorkspaceSyncScope::All)
            .await
    }

    pub async fn sync_v2_section(
        self: &std::sync::Arc<Self>,
        section: super::SyncSection,
    ) -> Result<WorkspaceV2SyncResult, OperationError> {
        let collections: &[&str] = match section {
            super::SyncSection::Plugins => &["plugin-selections", "plugin-settings"],
            super::SyncSection::Variables => {
                self.require_variable_sync().await?;
                &["variables"]
            }
        };
        self.sync_v2_scoped(
            true,
            tiktools_cloud_sync::WorkspaceSyncScope::Collections(
                collections.iter().map(|name| (*name).to_owned()).collect(),
            ),
        )
        .await
    }

    /// Refresh the shared cache without publishing or applying any section.
    pub async fn sync_v2_variables_fetch(
        self: &std::sync::Arc<Self>,
    ) -> Result<WorkspaceV2SyncResult, OperationError> {
        self.require_variable_sync().await?;
        self.sync_v2_scoped(
            false,
            tiktools_cloud_sync::WorkspaceSyncScope::Collections(Default::default()),
        )
        .await
    }

    async fn sync_v2_scoped(
        self: &std::sync::Arc<Self>,
        restore: bool,
        scope: tiktools_cloud_sync::WorkspaceSyncScope,
    ) -> Result<WorkspaceV2SyncResult, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let include_plugins = match &scope {
                tiktools_cloud_sync::WorkspaceSyncScope::All => true,
                tiktools_cloud_sync::WorkspaceSyncScope::Collections(collections) => {
                    collections.contains("plugin-selections")
                        || collections.contains("plugin-settings")
                }
            };
            use tiktools_cloud_sync::ProductLocalEditContext;
            let core = self.clone();
            let selected = tokio::task::spawn_blocking(move || {
                core.sync
                    .coordinator()
                    .map_err(|_| OperationError::unavailable("Cloud sync storage is unavailable"))?
                    .with_product_local_edit(|context| context)
                    .map_err(sync_error)
            })
            .await
            .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
            let ProductLocalEditContext::V2(Some(identity)) = selected else {
                return Err(OperationError::unavailable(
                    "Cloud v2 is not enabled for the current account",
                ));
            };
            if restore && self.cloud.auth().session().await.is_none() {
                let _ = self.cloud.auth().restore().await;
            }
            let session = self
                .cloud
                .auth()
                .session()
                .await
                .ok_or_else(|| OperationError::unavailable("Cloud sign-in is required"))?;
            if session.instance_origin() != identity.origin
                || session.account_id() != identity.account_id
            {
                return Err(OperationError::conflict("Cloud account changed; try again"));
            }
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = coordinator
                .capture_v2_config_epoch(&session)
                .ok_or_else(|| OperationError::conflict("Cloud sync setting changed; try again"))?;
            let core = self.clone();
            let origin = identity.origin.clone();
            let account = identity.account_id.clone();
            let outbox_more =
                tokio::task::spawn_blocking(move || -> Result<bool, OperationError> {
                    if !include_plugins {
                        return Ok(false);
                    }
                    let coordinator = core.sync.coordinator().map_err(|_| {
                        OperationError::unavailable("Cloud sync storage is unavailable")
                    })?;
                    let source = crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(
                        &core, &origin, &account,
                    );
                    for _ in 0..50 {
                        if core
                            .db
                            .next_plugin_sync_intent(&origin, &account)
                            .map_err(|_| {
                                OperationError::unavailable("Local sync queue is unavailable")
                            })?
                            .is_none()
                        {
                            return Ok(false);
                        }
                        coordinator
                            .stage_v2_product_decision(&source)
                            .map_err(sync_error)?;
                    }
                    Ok(core
                        .db
                        .next_plugin_sync_intent(&origin, &account)
                        .map_err(|_| {
                            OperationError::unavailable("Local sync queue is unavailable")
                        })?
                        .is_some())
                })
                .await
                .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
            let source = std::sync::Arc::new(if include_plugins {
                crate::cloud_sync::product_v2_outbox::OwnedSqliteV2UploadSource::new(
                    self.clone(),
                    identity.origin.clone(),
                    identity.account_id.clone(),
                )
            } else {
                crate::cloud_sync::product_v2_outbox::OwnedSqliteV2UploadSource::variables_only(
                    self.clone(),
                    identity.origin.clone(),
                    identity.account_id.clone(),
                )
            });
            let outcome = coordinator
                .sync_v2_product_scoped(session.clone(), source, scope)
                .await
                .map_err(|_| {
                    OperationError::unavailable(
                        "Cloud v2 sync could not complete; pending changes were retained",
                    )
                })?;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            #[cfg(feature = "plugin-install")]
            if include_plugins {
                self.apply_cached_v2_selections(session.clone(), identity, outcome.remote.clone())
                    .await;
            }
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            Ok(WorkspaceV2SyncResult {
                pushed: outcome.pushed,
                pulled: outcome.pulled,
                workspace_pending: outcome.pending,
                available_records: outcome.remote.len(),
                conflicts: outcome
                    .conflicts
                    .into_iter()
                    .map(|key| format!("{}/{}", key.collection, key.key))
                    .collect(),
                more_queued: outbox_more || outcome.more_queued,
            })
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (restore, scope);
            Err(OperationError::unavailable(
                "Cloud v2 sync requires persistence",
            ))
        }
    }

    /// Explicit rebuild from a consistent export after cursor expiry. Never
    /// adopts contents into the running host.
    pub async fn sync_v2_rebuild(self: &std::sync::Arc<Self>) -> Result<(), OperationError> {
        #[cfg(feature = "persistence")]
        {
            let (session, _) = self.v2_identity().await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            coordinator
                .rebuild_v2_product(session.clone())
                .await
                .map_err(|_| OperationError::unavailable("Cloud v2 rebuild could not complete"))?;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "Cloud v2 rebuild requires persistence",
            ))
        }
    }
}
