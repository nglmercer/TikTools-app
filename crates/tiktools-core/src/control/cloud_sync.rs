//! Cloud plugin-selection sync through the shared control plane.
//!
//! Everything here is nonsecret: the opt-in flag, pending/conflict counts,
//! and error text. Remote records are applied only to already-installed
//! plugins and never re-staged, which is what prevents sync loops. Version
//! drift in a remote record does not install or update anything; the
//! registry UI remains the only update path.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_cloud_sync::{
    SyncCoordError, SyncStatus, V1Edit as Edit, V1RecoveryChoice, V1RecoveryOutcome,
    V1UncertainReview,
};

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

/// A nonsecret projection of a single selection shown during explicit review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncRecoveryValue {
    pub state: String,
    pub version: Option<String>,
    pub enabled: Option<bool>,
}

impl SyncRecoveryValue {
    fn absent() -> Self {
        Self {
            state: "absent".into(),
            version: None,
            enabled: None,
        }
    }
    fn deleted() -> Self {
        Self {
            state: "deleted".into(),
            version: None,
            enabled: None,
        }
    }
    fn selection(value: &tiktools_cloud_sync::PluginSelection) -> Self {
        Self {
            state: "selected".into(),
            version: Some(value.version.clone()),
            enabled: Some(value.enabled),
        }
    }
    fn edit(value: &Edit) -> Self {
        match value {
            Edit::Put(value) => Self::selection(value),
            Edit::Delete => Self::deleted(),
        }
    }
    fn record(value: Option<&tiktools_cloud_sync::V1Record>) -> Self {
        match value {
            None => Self::absent(),
            Some(record) if record.deleted => Self::deleted(),
            Some(record) => record
                .value
                .as_ref()
                .map(Self::selection)
                .unwrap_or_else(Self::absent),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncRecoveryReview {
    pub handle: String,
    pub key: String,
    pub base: SyncRecoveryValue,
    pub original: SyncRecoveryValue,
    pub local: SyncRecoveryValue,
    pub cloud: SyncRecoveryValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncRecoveryChoice {
    KeepLocal,
    TakeCloud,
    TakeCloudIncludingNewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncRecoveryOutcome {
    Resolved,
    NewerEditNeedsReview,
}

pub(crate) struct RecoveryHandle {
    token: String,
    review: V1UncertainReview,
    newer_confirmation_required: bool,
}

fn sync_error(error: SyncCoordError) -> OperationError {
    let message = error.to_string();
    let reason = error.code();
    let mut operation = match reason {
        "disabled" | "not-signed-in" | "network" | "unauthorized" => {
            OperationError::unavailable(message)
        }
        "already-running" | "stale-review" => OperationError::conflict(message),
        "no-pending-edit" | "no-uncertain-write" => OperationError::not_found(message),
        _ => OperationError::internal(message),
    };
    operation.data = Some(serde_json::json!({ "reason": reason }));
    operation
}

impl AppCore {
    fn sync_coordinator(&self) -> Result<&tiktools_cloud_sync::SyncCoordinator, OperationError> {
        self.sync.coordinator().map_err(|reason| {
            let mut operation =
                OperationError::unavailable(format!("cloud sync is unavailable: {reason}"));
            operation.data = Some(serde_json::json!({ "reason": "storage" }));
            operation
        })
    }

    pub async fn sync_status(&self) -> Result<SyncStatus, OperationError> {
        Ok(self.sync_coordinator()?.status().await)
    }

    pub fn sync_set_enabled(&self, enabled: bool) -> Result<SyncStatus, OperationError> {
        self.sync_coordinator()?
            .set_enabled(enabled)
            .map_err(sync_error)
    }

    /// Pull, push, persist, then apply remote selections to already-installed
    /// plugins. Applying never stages, so remote changes cannot loop back.
    pub async fn sync_now(&self) -> Result<SyncNowResult, OperationError> {
        let outcome = self
            .sync_coordinator()?
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
            self.sync_coordinator()?
                .keep_local(key)
                .map_err(sync_error)?;
        } else {
            let remote = self
                .sync_coordinator()?
                .take_remote(key)
                .map_err(sync_error)?;
            let enabled = remote
                .as_ref()
                .map(|selection| selection.enabled)
                .unwrap_or(false);
            let _ = self.plugin_apply_remote_selection(key, enabled);
        }
        Ok(self.sync_coordinator()?.status().await)
    }

    /// List only keys; marker details and credentials stay in the native process.
    pub async fn sync_uncertain_keys(&self) -> Result<Vec<String>, OperationError> {
        self.sync_coordinator()?
            .uncertain_v1_keys()
            .await
            .map_err(sync_error)
    }

    pub async fn sync_review_uncertain(
        &self,
        key: &str,
    ) -> Result<SyncRecoveryReview, OperationError> {
        let mut slot = self.sync.recovery.lock().await;
        *slot = None;
        let review = self
            .sync_coordinator()?
            .review_uncertain_v1(key)
            .await
            .map_err(sync_error)?;
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes)
            .map_err(|_| OperationError::internal("could not create review handle"))?;
        let token = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let projection = SyncRecoveryReview {
            handle: token.clone(),
            key: review.original().key().to_owned(),
            base: SyncRecoveryValue::record(review.original().base_record()),
            original: SyncRecoveryValue::edit(&review.original().pending().edit),
            local: review
                .local()
                .and_then(|entry| entry.pending.as_ref())
                .map(|p| SyncRecoveryValue::edit(&p.edit))
                .unwrap_or_else(SyncRecoveryValue::absent),
            cloud: SyncRecoveryValue::record(review.remote()),
        };
        *slot = Some(RecoveryHandle {
            token,
            review,
            newer_confirmation_required: false,
        });
        Ok(projection)
    }

    /// Resolution remains unavailable until local mutations and sync staging
    /// share a crash-safe boundary. Otherwise even keeping the local value can
    /// upload an older staged edit after a newer change failed to stage.
    pub async fn sync_resolve_uncertain(
        &self,
        _handle: &str,
        _choice: SyncRecoveryChoice,
    ) -> Result<SyncRecoveryOutcome, OperationError> {
        Err(OperationError::unavailable(
            "recovery resolution is unavailable while local changes are being protected",
        ))
    }

    /// The native review remains private and session-bound. This must only be
    /// wired after durable source/application fencing is in place.
    #[allow(dead_code)]
    async fn sync_resolve_uncertain_when_fenced(
        &self,
        handle: &str,
        choice: SyncRecoveryChoice,
    ) -> Result<SyncRecoveryOutcome, OperationError> {
        let mut slot = self.sync.recovery.lock().await;
        let mut item = slot
            .take()
            .ok_or_else(|| OperationError::conflict("review expired; review again"))?;
        if item.token != handle {
            *slot = Some(item);
            return Err(OperationError::conflict("review expired; review again"));
        }
        let native_choice = match choice {
            SyncRecoveryChoice::KeepLocal => V1RecoveryChoice::KeepLocal,
            SyncRecoveryChoice::TakeCloud => V1RecoveryChoice::TakeCloudIfUnchanged,
            SyncRecoveryChoice::TakeCloudIncludingNewer if item.newer_confirmation_required => {
                V1RecoveryChoice::TakeCloudIncludingNewer
            }
            SyncRecoveryChoice::TakeCloudIncludingNewer => {
                *slot = Some(item);
                return Err(OperationError::conflict("review newer local changes first"));
            }
        };
        match self
            .sync_coordinator()?
            .resolve_uncertain_v1(&item.review, native_choice)
            .await
            .map_err(sync_error)?
        {
            V1RecoveryOutcome::Resolved => Ok(SyncRecoveryOutcome::Resolved),
            V1RecoveryOutcome::NewerEditNeedsReview => {
                item.newer_confirmation_required = true;
                *slot = Some(item);
                Ok(SyncRecoveryOutcome::NewerEditNeedsReview)
            }
        }
    }
}
