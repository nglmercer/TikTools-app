//! Nonsecret v1 uncertainty review and the gated native recovery path.
use super::OperationError;
use super::{
    sync_error, RecoveryHandle, SyncRecoveryChoice, SyncRecoveryOutcome, SyncRecoveryReview,
    SyncRecoveryValue,
};
use crate::AppCore;
use tiktools_cloud_sync::{V1Edit as Edit, V1RecoveryChoice, V1RecoveryOutcome};

impl AppCore {
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

impl SyncRecoveryValue {
    pub(super) fn absent() -> Self {
        Self {
            state: "absent".into(),
            version: None,
            enabled: None,
        }
    }
    pub(super) fn deleted() -> Self {
        Self {
            state: "deleted".into(),
            version: None,
            enabled: None,
        }
    }
    pub(super) fn selection(value: &tiktools_cloud_sync::PluginSelection) -> Self {
        Self {
            state: "selected".into(),
            version: Some(value.version.clone()),
            enabled: Some(value.enabled),
        }
    }
    pub(super) fn edit(value: &Edit) -> Self {
        match value {
            Edit::Put(value) => Self::selection(value),
            Edit::Delete => Self::deleted(),
        }
    }
    pub(super) fn record(value: Option<&tiktools_cloud_sync::V1Record>) -> Self {
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
