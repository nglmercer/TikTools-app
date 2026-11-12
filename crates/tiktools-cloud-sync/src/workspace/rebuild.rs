//! Explicit cursor-expiry recovery of the downloaded baseline.
use super::{check_eligibility, WorkspaceEligibility};
use super::{WorkspaceCoordError, WorkspaceCoordinator};
use cloud_client::Session;

impl WorkspaceCoordinator {
    /// Explicit 410 recovery reads a consistent snapshot and atomically
    /// replaces only the fetched cloud baseline. No application source changes.
    pub async fn rebuild_remote_cache(&self, session: &Session) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.rebuild_inner(session, None).await
    }

    pub async fn rebuild_remote_cache_guarded(
        &self,
        session: &Session,
        eligibility: &WorkspaceEligibility,
    ) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.rebuild_inner(session, Some(eligibility)).await
    }

    async fn rebuild_inner(
        &self,
        session: &Session,
        eligibility: Option<&WorkspaceEligibility>,
    ) -> Result<(), WorkspaceCoordError> {
        self.identity(session)?;
        check_eligibility(eligibility, session)?;
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        if !self.snapshot()?.state.bootstrap_complete {
            return Err(WorkspaceCoordError::BootstrapRequired);
        }
        check_eligibility(eligibility, session)?;
        let snapshot = session.workspace_snapshot().await?;
        check_eligibility(eligibility, session)?;
        if self.snapshot()?.state.is_product_guarded() {
            let eligibility = eligibility.ok_or(WorkspaceCoordError::Ineligible)?;
            self.update_product(eligibility, |state| {
                state.install_rebuild(snapshot.clone())?;
                Ok(())
            })
        } else {
            self.update(|state| {
                state.install_rebuild(snapshot.clone())?;
                Ok(())
            })
        }
    }
}
