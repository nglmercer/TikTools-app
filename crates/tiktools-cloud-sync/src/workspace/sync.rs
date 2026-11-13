//! Operation-scoped workspace runs composed from bounded network phases.
use super::{check_eligibility, ProductUploadAdmission, WorkspaceEligibility, WorkspaceOutcome};
use super::{WorkspaceCoordError, WorkspaceCoordinator};
use cloud_client::{Session, WorkspacePendingStatus, WorkspaceStateError};

mod absent_delete;
mod pull;
mod upload;
mod write;

impl WorkspaceCoordinator {
    /// Fetch and conditionally upload. Prepared uncertainty is saved before
    /// HTTP. Unconfirmed/conflicted requests are never automatically resent.
    /// The product must check that the captured session is still current
    /// before returning private rows to a UI or offering local application.
    pub async fn sync(&self, session: &Session) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        self.sync_scoped(session, &super::WorkspaceSyncScope::All)
            .await
    }

    /// Sync only the requested collections. Other queues and cached records
    /// remain durable and are not returned as application candidates.
    pub async fn sync_scoped(
        &self,
        session: &Session,
        scope: &super::WorkspaceSyncScope,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.sync_inner(session, None, None, scope).await
    }

    /// Product callers must use a generation-bound eligibility and recheck it
    /// when exposing or applying the returned data.
    pub async fn sync_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.sync_inner(
            session,
            Some(eligibility),
            None,
            &super::WorkspaceSyncScope::All,
        )
        .await
    }

    pub(crate) async fn sync_product_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        self.sync_product_scoped_guarded(
            session,
            eligibility,
            admission,
            &super::WorkspaceSyncScope::All,
        )
        .await
    }

    pub(crate) async fn sync_product_scoped_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
        scope: &super::WorkspaceSyncScope,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        if eligibility.publication.is_none()
            || eligibility.emission.is_none()
            || eligibility.emission_current.is_none()
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        self.sync_inner(session, Some(eligibility), Some(admission), scope)
            .await
    }

    async fn sync_inner(
        &self,
        session: &Session,
        eligibility: Option<&WorkspaceEligibility>,
        product: Option<&ProductUploadAdmission<'_>>,
        scope: &super::WorkspaceSyncScope,
    ) -> Result<WorkspaceOutcome, WorkspaceCoordError> {
        self.identity(session)?;
        check_eligibility(eligibility, session)?;
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        let initial = self.snapshot()?.state;
        if !initial.bootstrap_complete {
            return Err(WorkspaceCoordError::BootstrapRequired);
        }
        if product.is_some() != initial.is_product_guarded() {
            return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
        }
        if initial.needs_rebuild {
            return Err(WorkspaceStateError::NeedsRebuild.into());
        }
        let pulled = self
            .pull_workspace_pages(session, eligibility, product)
            .await?;
        let (pushed, absent_deletes) = self
            .upload_workspace_pending(session, eligibility, product, scope)
            .await?;
        check_eligibility(eligibility, session)?;
        let state = self.snapshot()?.state;
        check_eligibility(eligibility, session)?;
        Ok(WorkspaceOutcome {
            pulled,
            pushed,
            absent_deletes,
            pending: state
                .pending_keys()
                .iter()
                .filter(|key| scope.includes(&key.collection))
                .count(),
            conflicts: state
                .conflict_keys()
                .into_iter()
                .filter(|key| scope.includes(&key.collection))
                .collect(),
            remote: state
                .ready_remote()
                .into_iter()
                .filter(|record| scope.includes(&record.collection))
                .collect(),
            more_queued: state.entries.iter().any(|entry| {
                scope.includes(&entry.target.collection)
                    && entry
                        .pending
                        .as_ref()
                        .is_some_and(|pending| pending.status == WorkspacePendingStatus::Queued)
            }),
        })
    }
}
