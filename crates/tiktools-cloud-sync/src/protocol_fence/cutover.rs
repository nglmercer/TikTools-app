//! Reviewed preparation, resume and finalization of the protocol change.
use super::ProtocolFenceError;
use super::{
    candidate_hash, CutoverLease, CutoverPreview, ProtocolMode, ProtocolRecord,
    ResumeProtocolRecord,
};
use cloud_client::{PluginSelection, StateStore, WorkspaceStore};
use std::path::PathBuf;

impl CutoverLease {
    #[allow(dead_code)] // Used by the not-yet-exposed native v2 consent review.
    pub(crate) fn resume_record(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<ResumeProtocolRecord, ProtocolFenceError> {
        let record = self
            .fence
            .read_record(origin, account)?
            .ok_or(ProtocolFenceError::StaleCutover)?;
        // A historical preparation does not bind all reviewed inputs.
        if record.mode == ProtocolMode::Preparing && !matches!(record.format_version, 2 | 3) {
            return Err(ProtocolFenceError::StaleCutover);
        }
        if record.format_version == 3 && !self.fence.product_outbox_required(origin, account)? {
            return Err(ProtocolFenceError::ProductOutboxNotEnabled);
        }
        Ok(ResumeProtocolRecord(record))
    }

    /// Reconstruct an interrupted preparation for a new explicit review.
    /// The v1 inputs, installed selections and resulting candidate must hash
    /// to the original prepared marker; no migration state is changed.
    pub(crate) fn preview_preparing_with_receipts(
        &self,
        origin: &str,
        account: &str,
        selections: &[PluginSelection],
        receipts: Option<&[crate::V1CutoverReceipt]>,
    ) -> Result<CutoverPreview, ProtocolFenceError> {
        self.ensure_reviewed_mode(origin, account, receipts.is_some())?;
        if self.uncertain_uploads(origin, account)? != 0 {
            return Err(ProtocolFenceError::UncertainV1);
        }
        let record = self
            .fence
            .read_record(origin, account)?
            .ok_or(ProtocolFenceError::StaleCutover)?;
        if record.format_version != if receipts.is_some() { 3 } else { 2 }
            || record.mode != ProtocolMode::Preparing
        {
            return Err(ProtocolFenceError::StaleCutover);
        }
        let legacy = self.fence.legacy_store(origin, account)?.0.load()?;
        let preview = match receipts {
            Some(receipts) => CutoverPreview::new_marked(
                origin.into(),
                account.into(),
                selections,
                legacy.as_ref(),
                receipts,
            ),
            None => CutoverPreview::new(origin.into(), account.into(), selections, legacy.as_ref()),
        }
        .map_err(|_| ProtocolFenceError::StaleCutover)?;
        if record.candidate_sha256 != candidate_hash(&preview)? {
            return Err(ProtocolFenceError::StaleCutover);
        }
        Ok(preview)
    }

    pub(crate) fn preview_preparing_legacy_promotion(
        &self,
        origin: &str,
        account: &str,
        selections: &[PluginSelection],
    ) -> Result<CutoverPreview, ProtocolFenceError> {
        self.ensure_reviewed_mode(origin, account, true)?;
        if self.uncertain_uploads(origin, account)? != 0 {
            return Err(ProtocolFenceError::UncertainV1);
        }
        let record = self
            .fence
            .read_record(origin, account)?
            .ok_or(ProtocolFenceError::StaleCutover)?;
        if record.format_version != 3 || record.mode != ProtocolMode::Preparing {
            return Err(ProtocolFenceError::StaleCutover);
        }
        let legacy = self.fence.legacy_store(origin, account)?.0.load()?;
        let preview = CutoverPreview::new_legacy_promotion(
            origin.into(),
            account.into(),
            selections,
            legacy.as_ref(),
        )
        .map_err(|_| ProtocolFenceError::StaleCutover)?;
        if record.candidate_sha256 != candidate_hash(&preview)? {
            return Err(ProtocolFenceError::StaleCutover);
        }
        Ok(preview)
    }

    /// Detect a repeated acknowledgement of the same completed cutover.
    /// Workspace content can legitimately evolve after completion, so the
    /// immutable protocol record is the evidence, not the present snapshot.
    pub(crate) fn completed_matching(
        &self,
        preview: &CutoverPreview,
        origin: &str,
        account: &str,
    ) -> Result<bool, ProtocolFenceError> {
        let Some(record) = self.fence.read_record(origin, account)? else {
            return Ok(false);
        };
        if record.mode != ProtocolMode::V2 {
            return Ok(false);
        }
        self.ensure_reviewed_mode(origin, account, preview.is_marked())?;
        if record.format_version != if preview.is_marked() { 3 } else { 2 }
            || record.candidate_sha256 != candidate_hash(preview)?
        {
            return Err(ProtocolFenceError::StaleCutover);
        }
        Ok(true)
    }

    pub(crate) fn ensure_legacy_allowed(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        self.fence.ensure_legacy_allowed(origin, account)
    }

    pub(super) fn ensure_reviewed_mode(
        &self,
        origin: &str,
        account: &str,
        marked: bool,
    ) -> Result<(), ProtocolFenceError> {
        let actual = self.fence.product_outbox_required(origin, account)?;
        match (marked, actual) {
            (true, false) => Err(ProtocolFenceError::ProductOutboxNotEnabled),
            (false, true) => Err(ProtocolFenceError::ProductOutboxRequired),
            _ => Ok(()),
        }
    }

    pub(crate) fn ensure_v1(&self, origin: &str, account: &str) -> Result<(), ProtocolFenceError> {
        if self.fence.read_record(origin, account)?.is_some() {
            return Err(ProtocolFenceError::NotV1);
        }
        Ok(())
    }

    pub(crate) fn legacy_store(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(StateStore, PathBuf), ProtocolFenceError> {
        self.fence.legacy_store(origin, account)
    }

    /// Stop v1 for this identity before bootstrap. Repeating after a crash is
    /// allowed only for the same reviewed candidate and unchanged v1 inputs.
    /// The product must independently hold the local source/review fence.
    pub fn prepare(
        &self,
        preview: &CutoverPreview,
        origin: &str,
        account: &str,
        selections: &[PluginSelection],
    ) -> Result<(), ProtocolFenceError> {
        self.ensure_reviewed_mode(origin, account, preview.is_marked())?;
        if self.uncertain_uploads(origin, account)? != 0 {
            return Err(ProtocolFenceError::UncertainV1);
        }
        let legacy = self.fence.legacy_store(origin, account)?.0.load()?;
        preview
            .verify_inputs(origin, account, selections, legacy.as_ref())
            .map_err(|_| ProtocolFenceError::StaleCutover)?;
        let candidate_sha256 = candidate_hash(preview)?;
        if let Some(existing) = self.fence.read_record(origin, account)? {
            if existing.format_version != if preview.is_marked() { 3 } else { 2 }
                || existing.mode != ProtocolMode::Preparing
                || existing.candidate_sha256 != candidate_sha256
            {
                return Err(ProtocolFenceError::StaleCutover);
            }
            return Ok(());
        }
        self.fence.write_record(&ProtocolRecord {
            format_version: if preview.is_marked() { 3 } else { 2 },
            origin: origin.into(),
            account_id: account.into(),
            mode: ProtocolMode::Preparing,
            candidate_sha256,
        })
    }

    /// Commit protocol selection only after the exact reviewed candidate is
    /// durably bootstrapped. A crash leaves either Preparing or V2, never v1.
    pub fn finish(
        &self,
        preview: &CutoverPreview,
        origin: &str,
        account: &str,
    ) -> Result<(), ProtocolFenceError> {
        self.finish_with_hook(preview, origin, account, || {})
    }

    pub(super) fn finish_with_hook(
        &self,
        preview: &CutoverPreview,
        origin: &str,
        account: &str,
        before_commit: impl FnOnce(),
    ) -> Result<(), ProtocolFenceError> {
        let mut record = self
            .fence
            .read_record(origin, account)?
            .ok_or(ProtocolFenceError::StaleCutover)?;
        self.ensure_reviewed_mode(origin, account, preview.is_marked())?;
        if record.format_version != if preview.is_marked() { 3 } else { 2 }
            || record.mode != ProtocolMode::Preparing
            || record.candidate_sha256 != candidate_hash(preview)?
        {
            return Err(ProtocolFenceError::StaleCutover);
        }
        let legacy = self.fence.legacy_store(origin, account)?.0.load()?;
        preview
            .verify_inputs(origin, account, &preview.selections(), legacy.as_ref())
            .map_err(|_| ProtocolFenceError::StaleCutover)?;
        let store = WorkspaceStore::open(&self.fence.dir, origin, account)?;
        store
            .try_with_locked_snapshot(|snapshot| {
                let state = snapshot.ok_or(ProtocolFenceError::StaleCutover)?.state;
                if !preview.matches_bootstrap(&state) {
                    return Err(ProtocolFenceError::StaleCutover);
                }
                before_commit();
                record.mode = ProtocolMode::V2;
                self.fence.write_record(&record)
            })?
            .ok_or(ProtocolFenceError::Busy)?;
        Ok(())
    }
}
