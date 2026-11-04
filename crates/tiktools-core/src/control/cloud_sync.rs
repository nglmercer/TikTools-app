//! Cloud plugin-selection sync through the shared control plane.
//!
//! Everything here is nonsecret: the opt-in flag, pending/conflict counts,
//! and error text. Remote records are applied only to already-installed
//! plugins and never re-staged, which is what prevents sync loops. Version
//! drift in a remote record does not install or update anything; the
//! registry UI remains the only update path.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_cloud_sync::{SyncCoordError, SyncStatus};

use super::OperationError;
use crate::AppCore;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncNowResult {
    pub pushed: usize,
    pub pulled: usize,
    pub applied: usize,
    pub skipped: usize,
    pub conflicts: Vec<String>,
}

fn sync_error(error: SyncCoordError) -> OperationError {
    let message = error.to_string();
    let reason = error.code();
    let mut operation = match reason {
        "disabled" | "not-signed-in" | "network" | "unauthorized" => {
            OperationError::unavailable(message)
        }
        "already-running" => OperationError::conflict(message),
        "no-pending-edit" => OperationError::not_found(message),
        _ => OperationError::internal(message),
    };
    operation.data = Some(serde_json::json!({ "reason": reason }));
    operation
}

impl AppCore {
    fn sync_coordinator(&self) -> &tiktools_cloud_sync::SyncCoordinator {
        self.sync.coordinator()
    }

    pub async fn sync_status(&self) -> Result<SyncStatus, OperationError> {
        Ok(self.sync_coordinator().status().await)
    }

    pub fn sync_set_enabled(&self, enabled: bool) -> Result<SyncStatus, OperationError> {
        self.sync_coordinator()
            .set_enabled(enabled)
            .map_err(sync_error)
    }

    /// Pull, push, persist, then apply remote selections to already-installed
    /// plugins. Applying never stages, so remote changes cannot loop back.
    pub async fn sync_now(&self) -> Result<SyncNowResult, OperationError> {
        let outcome = self
            .sync_coordinator()
            .sync_now()
            .await
            .map_err(sync_error)?;
        let mut applied = 0_usize;
        let mut skipped = 0_usize;
        for remote in &outcome.remote {
            let enabled = remote
                .selection
                .as_ref()
                .map(|selection| selection.enabled)
                .unwrap_or(false);
            match self.plugin_apply_remote_selection(&remote.key, enabled) {
                Ok(true) => applied += 1,
                Ok(false) => skipped += 1,
                Err(_) => skipped += 1,
            }
        }
        Ok(SyncNowResult {
            pushed: outcome.pushed,
            pulled: outcome.pulled,
            applied,
            skipped,
            conflicts: outcome.conflicts,
        })
    }

    /// Resolve one conflicted key. Keeping the local edit rebases it for the
    /// next upload; taking the remote drops the edit and applies the remote
    /// selection to the installed plugin immediately.
    pub async fn sync_resolve_selection(
        &self,
        key: &str,
        keep_local: bool,
    ) -> Result<SyncStatus, OperationError> {
        if keep_local {
            self.sync_coordinator()
                .keep_local(key)
                .map_err(sync_error)?;
        } else {
            let remote = self
                .sync_coordinator()
                .take_remote(key)
                .map_err(sync_error)?;
            let enabled = remote
                .as_ref()
                .map(|selection| selection.enabled)
                .unwrap_or(false);
            let _ = self.plugin_apply_remote_selection(key, enabled);
        }
        Ok(self.sync_coordinator().status().await)
    }
}
