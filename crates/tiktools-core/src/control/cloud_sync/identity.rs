//! Coordinator access and exact current v2 identity/consent capture.
use super::sync_error;
use super::OperationError;
use crate::AppCore;

impl AppCore {
    /// UI snapshots share the protocol lease; they never reserve the source
    /// mutation slot. Only in-memory results cross the final account guard.
    #[cfg(feature = "persistence")]
    pub(in super::super) async fn v2_workspace_read<T: Send + 'static>(
        self: &std::sync::Arc<Self>,
        read: impl FnOnce(
                &tiktools_cloud_sync::SyncCoordinator,
                &std::sync::Arc<cloud_client::Session>,
            ) -> Result<T, tiktools_cloud_sync::WorkspaceCoordError>
            + Send
            + 'static,
    ) -> Result<T, OperationError> {
        let auth = self.cloud.auth();
        // Account startup explicitly restores saved credentials. A local list
        // must never initiate network traffic or sign a logged-out account in.
        let session = auth
            .session()
            .await
            .ok_or_else(|| OperationError::unavailable("Cloud sign-in is required"))?;
        let coordinator = self.sync_coordinator()?.clone();
        let captured = session.clone();
        #[cfg(test)]
        let core = self.clone();
        let (value, lease) = tokio::task::spawn_blocking(move || {
            let lease = coordinator.product_read().map_err(sync_error)?;
            if !lease.v2_selected(Some(&captured)).map_err(sync_error)? {
                return Err(OperationError::unavailable(
                    "Cloud v2 is not enabled for the current account",
                ));
            }
            #[cfg(test)]
            if let Some(started) = core.sync.v2_read_started.lock().unwrap().take() {
                let _ = started.send(());
            }
            let value = read(&coordinator, &captured)
                .map_err(|_| OperationError::unavailable("Cloud v2 changes are unavailable"))?;
            lease.check_current().map_err(sync_error)?;
            Ok((value, lease))
        })
        .await
        .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
        let result = auth
            .with_current_session_read(&session, || value)
            .await
            .map_err(|_| OperationError::conflict("Cloud account changed; try again"));
        drop(lease);
        result
    }

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
