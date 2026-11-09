//! Explicit consent recovery after v2 or an interrupted v2 cutover.
//!
//! This is native-only groundwork, not a product RPC or permission to send
//! pending work. The caller must separately review the local source and all
//! uncertain uploads before enabling any v2 networking or application.
use std::sync::Arc;

use cloud_client::{
    PluginSelection, Session, WorkspaceState, WorkspaceStateSnapshot, WorkspaceStore,
};

use crate::{
    protocol_fence::ResumeProtocolRecord, CutoverSource, SyncConfig, SyncCoordError,
    SyncCoordinator, V1CutoverReceipt,
};

/// An exact, private review of a disabled preference, account and workspace.
pub(crate) struct V2ResumeReview {
    binding: Arc<()>,
    session: Arc<Session>,
    config: SyncConfig,
    record: ResumeProtocolRecord,
    selections: Option<Vec<PluginSelection>>,
    marked_receipts: Option<Vec<V1CutoverReceipt>>,
    workspace: Option<WorkspaceStateSnapshot>,
}

fn same_workspace(a: &Option<WorkspaceStateSnapshot>, b: &Option<WorkspaceStateSnapshot>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.generation == b.generation && a.state == b.state,
        _ => false,
    }
}

impl SyncCoordinator {
    /// Recover the narrow crash window after a marked cutover was committed
    /// but its original enabled v1 preference was not yet scoped to v2.
    /// The caller must have the user's one-time migration consent; this never
    /// re-enables a disabled preference or alters pending workspace records.
    #[doc(hidden)]
    pub fn complete_interrupted_v2_scope(
        &self,
        session: &Arc<Session>,
    ) -> Result<(), SyncCoordError> {
        let lease = self.protocol.try_cutover()?;
        self.auth
            .with_current_session_permit(session, || {
                let config = self.read_staging_config()?;
                if !config.enabled
                    || config.version != 1
                    || config.scope.is_some()
                    || !config.allows_session(session)
                {
                    return Err(SyncCoordError::StaleReview);
                }
                let origin = session.instance_origin();
                let account = session.account_id();
                let record = lease.resume_record(&origin, account)?;
                if record.is_preparing() || !record.is_marked() {
                    return Err(SyncCoordError::StaleReview);
                }
                let root = self
                    .config_path
                    .parent()
                    .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
                let store = WorkspaceStore::open(root, &origin, account)?;
                let _operation = store
                    .try_operation_lock()?
                    .ok_or(crate::ProtocolFenceError::Busy)?;
                store
                    .try_with_locked_snapshot(|snapshot| {
                        if snapshot.as_ref().is_none_or(|snapshot| {
                            !snapshot.state.bootstrap_complete
                                || !snapshot.state.is_product_guarded()
                        }) {
                            return Err(SyncCoordError::StaleReview);
                        }
                        self.write_new_consent(
                            true,
                            config.generation,
                            Some(crate::consent_scope(&origin, account)),
                        )
                    })?
                    .ok_or(crate::ProtocolFenceError::Busy)?;
                Ok(())
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }

    fn read_disabled_consent_for_review(&self) -> Result<SyncConfig, SyncCoordError> {
        if !self.consent_marker_present()? {
            return Err(SyncCoordError::StaleReview);
        }
        let bytes = self
            .read_config_bytes()
            .map_err(|_| SyncCoordError::StaleReview)?;
        let config: SyncConfig =
            serde_json::from_slice(&bytes).map_err(|_| SyncCoordError::StaleReview)?;
        if !config.valid() || config.enabled || config.generation == 0 {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(config)
    }

    /// Inspect the exact disabled state before separately confirming consent.
    /// For Preparing, selections must reproduce the original frozen review;
    /// for completed v2, `selections` must be None. This does no network I/O.
    pub(crate) fn review_v2_resume(
        &self,
        session: &Arc<Session>,
        selections: Option<&[PluginSelection]>,
    ) -> Result<V2ResumeReview, SyncCoordError> {
        self.review_v2_resume_inner(session, selections, None)
    }

    /// Review an interrupted marked cutover against its frozen receipt set.
    /// Confirmation additionally requires a fresh guarded native source.
    pub(crate) fn review_marked_v2_resume(
        &self,
        session: &Arc<Session>,
        selections: &[PluginSelection],
        receipts: &[V1CutoverReceipt],
    ) -> Result<V2ResumeReview, SyncCoordError> {
        self.review_v2_resume_inner(session, Some(selections), Some(receipts))
    }

    fn review_v2_resume_inner(
        &self,
        session: &Arc<Session>,
        selections: Option<&[PluginSelection]>,
        receipts: Option<&[V1CutoverReceipt]>,
    ) -> Result<V2ResumeReview, SyncCoordError> {
        let lease = self.protocol.try_cutover()?;
        self.auth
            .with_current_session_permit(session, || {
                let config = self.read_disabled_consent_for_review()?;
                let origin = session.instance_origin();
                let account = session.account_id();
                let record = lease.resume_record(&origin, account)?;
                let preview =
                    match (record.is_preparing(), selections) {
                        (true, Some(selections)) => Some(lease.preview_preparing_with_receipts(
                            &origin, account, selections, receipts,
                        )?),
                        (false, None) if receipts.is_none() => None,
                        _ => return Err(SyncCoordError::StaleReview),
                    };
                let root = self
                    .config_path
                    .parent()
                    .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
                let store = WorkspaceStore::open(root, &origin, account)?;
                let workspace = store
                    .try_with_locked_snapshot(|snapshot| {
                        if let Some(preview) = &preview {
                            if let Some(snapshot) = &snapshot {
                                let empty = WorkspaceState::new(origin.clone(), account.to_owned())
                                    .map_err(|_| SyncCoordError::StaleReview)?;
                                if snapshot.state != empty
                                    && !preview.matches_bootstrap(&snapshot.state)
                                {
                                    return Err(SyncCoordError::StaleReview);
                                }
                            }
                        } else if snapshot
                            .as_ref()
                            .is_none_or(|snapshot| !snapshot.state.bootstrap_complete)
                        {
                            return Err(SyncCoordError::StaleReview);
                        }
                        Ok::<_, SyncCoordError>(snapshot)
                    })?
                    .ok_or(crate::ProtocolFenceError::Busy)?;
                Ok(V2ResumeReview {
                    binding: self.recovery_binding.clone(),
                    session: session.clone(),
                    config,
                    record,
                    selections: match (&preview, receipts) {
                        (Some(preview), Some(_)) => Some(preview.selections()),
                        _ => selections.map(|items| items.to_vec()),
                    },
                    marked_receipts: preview
                        .as_ref()
                        .and_then(|p| p.receipts().map(|items| items.to_vec())),
                    workspace,
                })
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }

    /// Confirm a fresh opt-in without changing the selected protocol or any
    /// pending data. Rechecks every reviewed input under the same locks.
    pub(crate) fn confirm_v2_resume(&self, review: &V2ResumeReview) -> Result<(), SyncCoordError> {
        if review.marked_receipts.is_some() {
            return Err(SyncCoordError::StaleReview);
        }
        self.confirm_v2_resume_inner(review)
    }

    /// Confirm marked Preparing only under a trusted source fence; a receipt
    /// or selection changed after review must not authorize a new consent.
    pub(crate) fn confirm_marked_v2_resume_with_source<S: CutoverSource>(
        &self,
        review: &V2ResumeReview,
        source: &S,
    ) -> Result<(), SyncCoordError> {
        let expected_receipts = review
            .marked_receipts
            .as_ref()
            .ok_or(SyncCoordError::StaleReview)?;
        let expected_selections = review
            .selections
            .as_ref()
            .ok_or(SyncCoordError::StaleReview)?;
        source.with_guarded_source(|selections, receipts| {
            let mut selections = selections.to_vec();
            selections.sort_by(|a, b| a.plugin_id.cmp(&b.plugin_id));
            let mut receipts = receipts.to_vec();
            receipts.sort_by(|a, b| a.key().cmp(b.key()));
            if &selections != expected_selections || &receipts != expected_receipts {
                return Err(SyncCoordError::StaleReview);
            }
            self.confirm_v2_resume_inner(review)
        })
    }

    fn confirm_v2_resume_inner(&self, review: &V2ResumeReview) -> Result<(), SyncCoordError> {
        if !Arc::ptr_eq(&self.recovery_binding, &review.binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_cutover()?;
        self.auth
            .with_current_session_permit(&review.session, || {
                let config = self.read_disabled_consent_for_review()?;
                let origin = review.session.instance_origin();
                let account = review.session.account_id();
                if config != review.config
                    || lease.resume_record(&origin, account)? != review.record
                {
                    return Err(SyncCoordError::StaleReview);
                }
                if let Some(selections) = &review.selections {
                    lease.preview_preparing_with_receipts(
                        &origin,
                        account,
                        selections,
                        review.marked_receipts.as_deref(),
                    )?;
                }
                let root = self
                    .config_path
                    .parent()
                    .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
                let store = WorkspaceStore::open(root, &origin, account)?;
                store
                    .try_with_locked_snapshot(|snapshot| {
                        if !same_workspace(&snapshot, &review.workspace) {
                            return Err(SyncCoordError::StaleReview);
                        }
                        self.write_new_consent(
                            true,
                            config.generation,
                            Some(crate::consent_scope(&origin, account)),
                        )
                    })?
                    .ok_or(crate::ProtocolFenceError::Busy)?;
                Ok(())
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }
}
