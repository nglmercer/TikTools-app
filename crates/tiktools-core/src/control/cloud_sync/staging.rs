//! Portable owner preferences and plugin-setting decisions under v2 fences.
use super::OperationError;
use crate::AppCore;

impl AppCore {
    /// Stage owner-owned desktop preferences under v2 fences. The native
    /// object is adapted to the portable subset; unknown fields stay local.
    pub async fn sync_v2_stage_preferences(
        self: &std::sync::Arc<Self>,
        native: serde_json::Value,
    ) -> Result<(), OperationError> {
        #[cfg(feature = "persistence")]
        {
            let portable = tiktools_cloud_sync::preference_from_native(&native)
                .ok_or_else(|| OperationError::invalid("preferences must be an object"))?;
            let (session, _) = self.v2_identity().await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let core_session = session.clone();
            let target = cloud_client::WorkspaceKey::new("preferences", "desktop")
                .map_err(|_| OperationError::invalid("invalid preferences key"))?;
            let edit = cloud_client::WorkspaceEdit::Put(portable);
            let core = self.clone();
            tokio::task::spawn_blocking(move || {
                core.sync
                    .coordinator()
                    .map_err(|_| OperationError::unavailable("Cloud sync storage is unavailable"))?
                    .stage_v2_product_edit(core_session, target, edit)
                    .map_err(|_| OperationError::conflict("Cloud v2 staging was rejected"))
            })
            .await
            .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
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
            let _ = native;
            Err(OperationError::unavailable(
                "Cloud v2 preferences require persistence",
            ))
        }
    }

    /// Stage owner-owned plugin settings under v2 fences. The native object
    /// is adapted to the portable envelope; unknown fields stay local.
    pub async fn sync_v2_stage_plugin_settings(
        self: &std::sync::Arc<Self>,
        plugin_id: &str,
        native: serde_json::Value,
    ) -> Result<(), OperationError> {
        #[cfg(feature = "persistence")]
        {
            if cloud_client::WorkspaceKey::new("plugin-settings", plugin_id).is_err() {
                return Err(OperationError::invalid("invalid plugin id"));
            }
            let portable = tiktools_cloud_sync::settings_from_native(plugin_id, &native)
                .ok_or_else(|| OperationError::invalid("settings must be an object"))?;
            let (session, _) = self.v2_identity().await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let core_session = session.clone();
            let target = cloud_client::WorkspaceKey::new("plugin-settings", plugin_id)
                .map_err(|_| OperationError::invalid("invalid plugin id"))?;
            let edit = cloud_client::WorkspaceEdit::Put(portable);
            let core = self.clone();
            tokio::task::spawn_blocking(move || {
                core.sync
                    .coordinator()
                    .map_err(|_| OperationError::unavailable("Cloud sync storage is unavailable"))?
                    .stage_v2_product_edit(core_session, target, edit)
                    .map_err(|_| OperationError::conflict("Cloud v2 staging was rejected"))
            })
            .await
            .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
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
            let _ = (plugin_id, native);
            Err(OperationError::unavailable(
                "Cloud v2 plugin settings require persistence",
            ))
        }
    }
}
