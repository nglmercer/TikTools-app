//! Bounded queued-write traversal with separate absent-delete handling.
use super::super::{
    check_eligibility, ProductUploadAdmission, WorkspaceCoordError, WorkspaceCoordinator,
    WorkspaceEligibility,
};
use cloud_client::{Session, WorkspaceEdit, WorkspacePendingStatus};

const MAX_WRITES: usize = 50;

impl WorkspaceCoordinator {
    pub(super) async fn upload_workspace_pending(
        &self,
        session: &Session,
        eligibility: Option<&WorkspaceEligibility>,
        product: Option<&ProductUploadAdmission<'_>>,
        scope: &super::super::WorkspaceSyncScope,
    ) -> Result<(usize, usize), WorkspaceCoordError> {
        let keys = self.snapshot()?.state.pending_keys();
        let mut pushed = 0;
        let mut absent_deletes = 0;
        let mut attempts = 0;
        for key in keys {
            if !scope.includes_target(&key.collection, &key.key) {
                continue;
            }
            check_eligibility(eligibility, session)?;
            if attempts >= MAX_WRITES {
                break;
            }
            let state = self.snapshot()?.state;
            let Some(pending) = state.entry(&key).and_then(|entry| entry.pending.as_ref()) else {
                continue;
            };
            if pending.status != WorkspacePendingStatus::Queued {
                continue;
            }
            if pending.base_revision.is_none() && matches!(pending.edit, WorkspaceEdit::Delete) {
                let generation = pending.local_generation;
                attempts += 1;
                if self
                    .complete_workspace_absent_delete(
                        session,
                        &key,
                        generation,
                        eligibility,
                        product,
                    )
                    .await?
                {
                    absent_deletes += 1;
                }
                continue;
            }
            if self
                .push_workspace_edit(session, &key, eligibility, product, &mut attempts)
                .await?
            {
                pushed += 1;
            }
        }
        Ok((pushed, absent_deletes))
    }
}
