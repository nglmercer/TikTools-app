//! Coordinator access and exact current v2 identity/consent capture.
use super::sync_error;
use super::OperationError;
use crate::AppCore;

impl AppCore {
    pub(in super::super) fn sync_coordinator(
        &self,
    ) -> Result<&tiktools_cloud_sync::SyncCoordinator, OperationError> {
        self.sync.coordinator().map_err(|reason| {
            let mut operation =
                OperationError::unavailable(format!("cloud sync is unavailable: {reason}"));
            operation.data = Some(serde_json::json!({ "reason": "storage" }));
            operation
        })
    }

    pub(in super::super) async fn v2_identity(
        self: &std::sync::Arc<Self>,
    ) -> Result<
        (
            std::sync::Arc<cloud_client::Session>,
            tiktools_cloud_sync::V2LocalEditContext,
        ),
        OperationError,
    > {
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
        if self.cloud.auth().session().await.is_none() {
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
        Ok((session, identity))
    }

    pub(in super::super) fn v2_epoch(
        &self,
        coordinator: &tiktools_cloud_sync::SyncCoordinator,
        session: &std::sync::Arc<cloud_client::Session>,
    ) -> Result<tiktools_cloud_sync::SyncConfigEpoch, OperationError> {
        coordinator
            .capture_v2_config_epoch(session)
            .ok_or_else(|| OperationError::conflict("Cloud sync setting changed; try again"))
    }
}
