//! Legacy status, opt-in and sync compatibility operations.
use super::OperationError;
use super::{sync_error, SyncNowResult};
use crate::AppCore;
use tiktools_cloud_sync::SyncStatus;

impl AppCore {
    pub async fn sync_status(&self) -> Result<SyncStatus, OperationError> {
        Ok(self.sync_coordinator()?.status().await)
    }

    pub fn sync_set_enabled(&self, enabled: bool) -> Result<SyncStatus, OperationError> {
        let _invalidate = super::super::cloud_account::CloudContextInvalidation::new(&self.events);
        self.sync_coordinator()?
            .set_enabled(enabled)
            .map_err(sync_error)
    }

    /// Pull and push selections. Remote values remain in the durable account
    /// snapshot; applying them needs a session-, consent- and source-fenced
    /// review and a recoverable local-runtime outcome.
    pub async fn sync_now(&self) -> Result<SyncNowResult, OperationError> {
        let outcome = self
            .sync_coordinator()?
            .sync_now()
            .await
            .map_err(sync_error)?;
        let skipped = outcome.remote.len();
        Ok(SyncNowResult {
            pushed: outcome.pushed,
            pulled: outcome.pulled,
            applied: 0,
            skipped,
            conflicts: outcome.conflicts,
            application_deferred: skipped != 0,
        })
    }

    /// Keep both sides of a conflict until source-fenced recovery can bind the
    /// reviewed Cloud and local states to a durable outcome. Neither choice
    /// may discard or rebase a newer local edit through the legacy bypass.
    pub async fn sync_resolve_selection(
        &self,
        _key: &str,
        _keep_local: bool,
    ) -> Result<SyncStatus, OperationError> {
        let mut error = OperationError::unavailable(
            "Cloud conflict resolution is unavailable until local application and recovery are protected",
        );
        error.data = Some(serde_json::json!({ "reason": "conflict-review-unavailable" }));
        Err(error)
    }
}
