//! Explicit consent recovery after v2 or an interrupted v2 cutover.
//!
//! This is native-only groundwork, not a product RPC or permission to send
//! pending work. The caller must separately review the local source and all
//! uncertain uploads before enabling any v2 networking or application.
use std::sync::Arc;

use cloud_client::{
    PluginSelection, Session, WorkspaceState, WorkspaceStateSnapshot, WorkspaceStore,
};

use crate::{protocol_fence::ResumeProtocolRecord, SyncConfig, SyncCoordError, SyncCoordinator};

/// An exact, private review of a disabled preference, account and workspace.
pub(crate) struct V2ResumeReview {
    binding: Arc<()>,
    session: Arc<Session>,
    config: SyncConfig,
    record: ResumeProtocolRecord,
    selections: Option<Vec<PluginSelection>>,
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
        let lease = self.protocol.try_cutover()?;
        self.auth
            .with_current_session_permit(session, || {
                let config = self.read_disabled_consent_for_review()?;
                let origin = session.instance_origin();
                let account = session.account_id();
                let record = lease.resume_record(&origin, account)?;
                let preview = match (record.is_preparing(), selections) {
                    (true, Some(selections)) => {
                        Some(lease.preview_preparing(&origin, account, selections)?)
                    }
                    (false, None) => None,
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
                    selections: selections.map(|items| items.to_vec()),
                    workspace,
                })
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }

    /// Confirm a fresh opt-in without changing the selected protocol or any
    /// pending data. Rechecks every reviewed input under the same locks.
    pub(crate) fn confirm_v2_resume(&self, review: &V2ResumeReview) -> Result<(), SyncCoordError> {
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
                    lease.preview_preparing(&origin, account, selections)?;
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
