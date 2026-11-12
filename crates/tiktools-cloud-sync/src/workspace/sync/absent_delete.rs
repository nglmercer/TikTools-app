//! Unbased deletion observation with product epoch and source admission.
use super::super::{
    check_eligibility, ProductUploadAdmission, WorkspaceCoordError, WorkspaceCoordinator,
    WorkspaceEligibility,
};
use cloud_client::{Session, WorkspaceKey, WorkspaceProductProof, WorkspaceState};

impl WorkspaceCoordinator {
    pub(super) async fn complete_workspace_absent_delete(
        &self,
        session: &Session,
        key: &WorkspaceKey,
        generation: u64,
        eligibility: Option<&WorkspaceEligibility>,
        product: Option<&ProductUploadAdmission<'_>>,
    ) -> Result<bool, WorkspaceCoordError> {
        check_eligibility(eligibility, session)?;
        let read = if let Some(product) = product {
            // An absence read alone cannot fence an older in-flight
            // creation. First require server epoch enforcement and
            // advance it while the exact session and source lease hold.
            let before = session.workspace_write_epoch().await?;
            check_eligibility(eligibility, session)?;
            if before.epoch_required() != Some(true) {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let source_guard = product.source.acquire_emission_guard()?;
            let eligible = eligibility.ok_or(WorkspaceCoordError::Ineligible)?;
            let admit_session = eligible
                .emission
                .as_ref()
                .ok_or(WorkspaceCoordError::Ineligible)?;
            let session_guard = admit_session()?;
            if !eligible
                .emission_current
                .as_ref()
                .ok_or(WorkspaceCoordError::Ineligible)?()
            {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let barrier = session.workspace_write_barrier().await?;
            if !barrier.is_after(&before)? {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let read = session.workspace_read_for_resolution(key).await?;
            drop(session_guard);
            drop(source_guard);
            read
        } else {
            session.workspace_read_for_resolution(key).await?
        };
        check_eligibility(eligibility, session)?;
        let complete = |proof: Option<&WorkspaceProductProof>| {
            let change = |state: &mut WorkspaceState| {
                if let Some(proof) = proof {
                    state.verify_product_queued(proof)?;
                }
                if read.record().is_none() {
                    state.complete_absent_delete(&read, generation)?;
                } else {
                    state.observe_read(&read)?;
                }
                Ok(())
            };
            if product.is_some() {
                self.update_product(eligibility.ok_or(WorkspaceCoordError::Ineligible)?, change)
            } else {
                self.update(change)
            }
        };
        if let Some(product) = product {
            self.admit_product(
                key,
                eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                product,
                |proof| complete(Some(proof)),
            )?;
        } else {
            complete(None)?;
        }
        Ok(self
            .snapshot()?
            .state
            .entry(key)
            .and_then(|entry| entry.pending.as_ref())
            .is_none())
    }
}
