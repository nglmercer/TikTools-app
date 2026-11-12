//! Informational v2 cutover previews rechecked against current v1 input.
use crate::{CutoverPreview, PluginSelection, V1CutoverReceipt};
use crate::{SyncCoordError, SyncCoordinator};
use std::sync::Arc;

impl SyncCoordinator {
    /// Build an informational v2 migration candidate from the exact current
    /// v1 snapshot and the supplied local selections. This does not authorize
    /// or start a cutover: the product must capture and recheck local source,
    /// account and consent under its cutover fence before accepting a review.
    pub fn preview_v2_cutover(
        &self,
        session: &Arc<cloud_client::Session>,
        selections: &[PluginSelection],
    ) -> Result<CutoverPreview, SyncCoordError> {
        self.preview_v2_cutover_with_snapshot_hook(session, selections, || {})
    }

    pub(crate) fn preview_v2_cutover_with_snapshot_hook(
        &self,
        session: &Arc<cloud_client::Session>,
        selections: &[PluginSelection],
        after_snapshot: impl FnOnce(),
    ) -> Result<CutoverPreview, SyncCoordError> {
        self.preview_v2_cutover_inner(session, selections, None, after_snapshot)
    }

    pub(crate) fn preview_v2_cutover_inner(
        &self,
        session: &Arc<cloud_client::Session>,
        selections: &[PluginSelection],
        receipts: Option<&[V1CutoverReceipt]>,
        after_snapshot: impl FnOnce(),
    ) -> Result<CutoverPreview, SyncCoordError> {
        let epoch = self
            .capture_config_epoch(session)
            .ok_or(SyncCoordError::StaleReview)?;
        let origin = session.instance_origin();
        let account = session.account_id().to_owned();
        let snapshot = || -> Result<_, SyncCoordError> {
            let lease = self.protocol.try_v1()?;
            let _admission = lease.try_admission()?;
            lease.ensure_v1(&origin, &account)?;
            // These durable journals can contain work not represented by the
            // v1 snapshot and prevent a complete migration candidate.
            lease.ensure_outbox_mode(&origin, &account, receipts.is_some())?;
            lease.ensure_no_uncertain(&origin, &account)?;
            self.auth
                .with_current_session_permit(session, || {
                    if !self.is_config_epoch_current(&epoch, session) {
                        return Err(SyncCoordError::StaleReview);
                    }
                    Ok(lease.legacy_store(&origin, &account)?.0.load()?)
                })
                .map_err(|_| SyncCoordError::StaleReview)?
        };
        let legacy = snapshot()?;
        after_snapshot();
        // Validation can be expensive for large candidates. Do not block
        // logout, local admission, or protocol cutover while it runs.
        let preview = match receipts {
            None => {
                CutoverPreview::new(origin.clone(), account.clone(), selections, legacy.as_ref())
            }
            Some(receipts) => CutoverPreview::new_marked(
                origin.clone(),
                account.clone(),
                selections,
                legacy.as_ref(),
                receipts,
            ),
        }
        .map_err(|error| SyncCoordError::Local(error.to_string()))?;
        let current = snapshot()?;
        preview
            .verify_inputs(&origin, &account, selections, current.as_ref())
            .map_err(|_| SyncCoordError::StaleReview)?;
        Ok(preview)
    }
}
