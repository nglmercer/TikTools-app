//! Local v1-to-v2 migration orchestration; no network or local application.
//!
//! This is a coordinator primitive, not a product cutover. The caller must
//! retain an independently reviewed, cross-process source/package fence and
//! explicit user approval through the complete call. Existing Desktop paths
//! do not yet provide that fence and must not expose this operation.
use std::sync::Arc;

use cloud_client::{PluginSelection, Session, WorkspaceState};

use crate::{
    CutoverPreview, SyncConfigEpoch, SyncCoordError, SyncCoordinator, WorkspaceCoordError,
    WorkspaceCoordinator,
};

/// Opaque native review; UI input cannot reconstruct its identity or consent.
pub struct GuardedCutoverReview {
    session: Arc<Session>,
    consent: SyncConfigEpoch,
    preview: CutoverPreview,
}

impl GuardedCutoverReview {
    pub fn preview(&self) -> &CutoverPreview {
        &self.preview
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutoverExecution {
    Completed,
    AlreadyCompleted,
}

/// Trusted native adapter that takes its complete local-source fences before
/// invoking the synchronous operation. Implementations must not release the
/// fences until the operation returns, and must not await or perform network
/// work under them. A failed adapter may report an unknown outcome if its
/// operation ran before the failure; recovery must inspect durable state.
pub trait CutoverSource {
    fn with_guarded_source<T>(
        &self,
        operation: impl FnOnce(
            &[PluginSelection],
            &[crate::V1CutoverReceipt],
        ) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError>;
}

impl SyncCoordinator {
    /// Capture the exact live session and opt-in decision along with the
    /// informational preview. A separate product consent step is required.
    pub fn guarded_cutover_review(
        &self,
        session: &Arc<Session>,
        selections: &[PluginSelection],
    ) -> Result<GuardedCutoverReview, SyncCoordError> {
        self.guarded_cutover_review_inner(session, selections, None)
    }

    /// Review a previously migrated product outbox. Receipt metadata is
    /// frozen into the review; its original consent is retained as evidence,
    /// not reused to authorize a new upload. All migrated edits require a
    /// fresh v2 conflict decision after this local cutover.
    pub fn guarded_marked_cutover_review(
        &self,
        session: &Arc<Session>,
        selections: &[PluginSelection],
        receipts: &[crate::V1CutoverReceipt],
    ) -> Result<GuardedCutoverReview, SyncCoordError> {
        self.guarded_cutover_review_inner(session, selections, Some(receipts))
    }

    fn guarded_cutover_review_inner(
        &self,
        session: &Arc<Session>,
        selections: &[PluginSelection],
        receipts: Option<&[crate::V1CutoverReceipt]>,
    ) -> Result<GuardedCutoverReview, SyncCoordError> {
        let consent = self
            .capture_config_epoch(session)
            .ok_or(SyncCoordError::StaleReview)?;
        let preview = match self.preview_v2_cutover_inner(session, selections, receipts, || {}) {
            Ok(preview) => preview,
            Err(SyncCoordError::Protocol(crate::ProtocolFenceError::NotV1)) => {
                let lease = self.protocol.try_cutover()?;
                self.auth
                    .with_current_session_permit(session, || {
                        if !self.is_config_epoch_current(&consent, session) {
                            return Err(SyncCoordError::StaleReview);
                        }
                        Ok(lease.preview_preparing_with_receipts(
                            &session.instance_origin(),
                            session.account_id(),
                            selections,
                            receipts,
                        )?)
                    })
                    .map_err(|_| SyncCoordError::StaleReview)??
            }
            Err(error) => return Err(error),
        };
        self.auth
            .with_current_session_permit(session, || {
                if !self.is_config_epoch_current(&consent, session) {
                    return Err(SyncCoordError::StaleReview);
                }
                Ok(())
            })
            .map_err(|_| SyncCoordError::StaleReview)??;
        Ok(GuardedCutoverReview {
            session: session.clone(),
            consent,
            preview,
        })
    }

    /// Reconstruct an informational review from the current v1 snapshot and
    /// compare its complete frozen inputs, including marked outbox receipts.
    /// This is not an execution fence: writers may change state after return.
    pub fn verify_guarded_cutover_review(
        &self,
        review: &GuardedCutoverReview,
        selections: &[PluginSelection],
        receipts: &[crate::V1CutoverReceipt],
    ) -> Result<(), SyncCoordError> {
        let fresh = if review.preview.is_marked() {
            self.guarded_marked_cutover_review(&review.session, selections, receipts)?
        } else {
            if !receipts.is_empty() {
                return Err(SyncCoordError::StaleReview);
            }
            self.guarded_cutover_review(&review.session, selections)?
        };
        let original = review
            .preview
            .review_material()
            .map_err(|_| SyncCoordError::StaleReview)?;
        let current = fresh
            .preview
            .review_material()
            .map_err(|_| SyncCoordError::StaleReview)?;
        if original != current {
            return Err(SyncCoordError::StaleReview);
        }
        self.auth
            .with_current_session_permit(&review.session, || {
                if !self.is_config_epoch_current(&review.consent, &review.session) {
                    return Err(SyncCoordError::StaleReview);
                }
                Ok(())
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }

    /// Execute only through a native source adapter. A product must still
    /// obtain explicit user approval and fence every source/package writer;
    /// this coordinator cannot verify the adapter's implementation.
    pub fn execute_cutover_with_source<S: CutoverSource>(
        &self,
        review: &GuardedCutoverReview,
        source: &S,
    ) -> Result<CutoverExecution, SyncCoordError> {
        source.with_guarded_source(|selections, receipts| {
            if review.preview.receipts().unwrap_or_default() != receipts {
                return Err(SyncCoordError::StaleReview);
            }
            self.execute_guarded_cutover(review, selections)
        })
    }

    /// Commit reviewed migration under an exclusive cross-process protocol
    /// lease and this manager's session gate. `selections` must have been
    /// freshly captured and remain fenced by the caller throughout this call;
    /// the coordinator cannot enforce package or source locks by itself. Run
    /// off async executor threads: local durable writes may block. No HTTP is
    /// performed. Older binaries and other CloudAuth managers are not fenced.
    pub(crate) fn execute_guarded_cutover(
        &self,
        review: &GuardedCutoverReview,
        selections: &[PluginSelection],
    ) -> Result<CutoverExecution, SyncCoordError> {
        self.execute_guarded_cutover_with_hooks(review, selections, || Ok(()), || Ok(()))
    }

    pub(crate) fn execute_guarded_cutover_with_hooks(
        &self,
        review: &GuardedCutoverReview,
        selections: &[PluginSelection],
        after_prepare: impl FnOnce() -> Result<(), SyncCoordError>,
        after_bootstrap: impl FnOnce() -> Result<(), SyncCoordError>,
    ) -> Result<CutoverExecution, SyncCoordError> {
        let lease = self.protocol.try_cutover()?;
        self.auth
            .with_current_session_permit(&review.session, || {
                if !self.is_config_epoch_current(&review.consent, &review.session) {
                    return Err(SyncCoordError::StaleReview);
                }
                let origin = review.session.instance_origin();
                let account = review.session.account_id();
                if lease.completed_matching(&review.preview, &origin, account)? {
                    return Ok(CutoverExecution::AlreadyCompleted);
                }
                let root = self
                    .config_path
                    .parent()
                    .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
                let workspace =
                    WorkspaceCoordinator::open(root, origin.clone(), account.to_owned())
                        .map_err(|error| SyncCoordError::Local(error.to_string()))?;
                // Avoid entering Preparing when existing workspace data is
                // already incompatible. bootstrap repeats the empty-state
                // check inside its generation-checked update.
                let current = workspace
                    .snapshot()
                    .map_err(|error| SyncCoordError::Local(error.to_string()))?
                    .state;
                if current.bootstrap_complete {
                    if !review.preview.matches_bootstrap(&current) {
                        return Err(crate::ProtocolFenceError::StaleCutover.into());
                    }
                } else if current
                    != WorkspaceState::new(origin.clone(), account.to_owned())
                        .map_err(|error| SyncCoordError::Local(error.to_string()))?
                {
                    return Err(SyncCoordError::Local(
                        WorkspaceCoordError::BootstrapNeedsReview.to_string(),
                    ));
                }
                lease.prepare(&review.preview, &origin, account, selections)?;
                after_prepare()?;
                match workspace.bootstrap_reviewed(&review.preview) {
                    Ok(()) | Err(WorkspaceCoordError::AlreadyBootstrapped) => {}
                    Err(error) => return Err(SyncCoordError::Local(error.to_string())),
                }
                after_bootstrap()?;
                lease.finish(&review.preview, &origin, account)?;
                Ok(CutoverExecution::Completed)
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }
}
