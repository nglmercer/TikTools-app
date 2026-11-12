//! Authenticated comparisons and recovery barriers for original requests.
use super::{check_eligibility, WorkspaceEligibility, WorkspaceReview};
use super::{WorkspaceCoordError, WorkspaceCoordinator};
use cloud_client::{
    Session, WorkspaceKey, WorkspaceProductProof, WorkspaceState, WorkspaceStateError,
};

impl WorkspaceCoordinator {
    /// Read-only cloud comparison. It cannot authorize replay or local apply.
    pub async fn review(
        &self,
        session: &Session,
        target: &WorkspaceKey,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.review_inner(session, target, None, None).await
    }

    pub async fn review_guarded(
        &self,
        session: &Session,
        target: &WorkspaceKey,
        eligibility: &WorkspaceEligibility,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.review_inner(session, target, Some(eligibility), None)
            .await
    }

    /// Capture a private product recovery comparison. It can only authorize a
    /// later explicit keep-local or take-cloud resolution under the matching
    /// native source fence; it never applies a remote value or resends a
    /// request itself.
    pub(crate) async fn review_product_guarded(
        &self,
        session: &Session,
        target: &WorkspaceKey,
        eligibility: &WorkspaceEligibility,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        if eligibility.publication.is_none()
            || eligibility.emission.is_none()
            || eligibility.emission_current.is_none()
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        let proof = self
            .snapshot()?
            .state
            .product_recovery_proof(target)
            .ok_or(WorkspaceStateError::StaleReview)?;
        self.review_inner(session, target, Some(eligibility), Some(proof))
            .await
    }

    async fn review_inner(
        &self,
        session: &Session,
        target: &WorkspaceKey,
        eligibility: Option<&WorkspaceEligibility>,
        product_proof: Option<WorkspaceProductProof>,
    ) -> Result<WorkspaceReview, WorkspaceCoordError> {
        self.identity(session)?;
        check_eligibility(eligibility, session)?;
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        if self.snapshot()?.state.is_product_guarded() != product_proof.is_some() {
            return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
        }
        if product_proof.is_some() {
            // Fence the original request, not a newer staged-next decision.
            // A legacy uncertain request without a recorded epoch cannot be
            // given one retroactively and must remain pending for recovery.
            let original_epoch = self
                .snapshot()?
                .state
                .entry(target)
                .and_then(|entry| entry.pending.as_ref())
                .and_then(|pending| pending.write_epoch.clone())
                .ok_or(WorkspaceCoordError::Ineligible)?;
            check_eligibility(eligibility, session)?;
            let policy = session.workspace_write_epoch().await?;
            check_eligibility(eligibility, session)?;
            if policy.epoch_required() != Some(true) {
                return Err(WorkspaceCoordError::Ineligible);
            }
            // Advancing the server epoch affects all devices on this account.
            // Admit the captured session before sending the barrier, just as
            // for a product write. A queued logout must win before admission
            // or wait for the request future to finish or be cancelled.
            let eligibility = eligibility.ok_or(WorkspaceCoordError::Ineligible)?;
            let barrier_guard = eligibility
                .emission
                .as_ref()
                .ok_or(WorkspaceCoordError::Ineligible)?()?;
            if !eligibility
                .emission_current
                .as_ref()
                .ok_or(WorkspaceCoordError::Ineligible)?()
            {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let barrier = session.workspace_write_barrier().await?;
            drop(barrier_guard);
            check_eligibility(Some(eligibility), session)?;
            if !barrier.is_after(&original_epoch)? {
                return Err(WorkspaceCoordError::Ineligible);
            }
        }
        check_eligibility(eligibility, session)?;
        let read = session.workspace_read_for_resolution(target).await?;
        check_eligibility(eligibility, session)?;
        let observe = |state: &mut WorkspaceState| {
            if let Some(proof) = &product_proof {
                state.verify_product_recovery(proof)?;
            }
            state.observe_read(&read)?;
            Ok(())
        };
        if product_proof.is_some() {
            self.update_product(eligibility.ok_or(WorkspaceCoordError::Ineligible)?, observe)?;
        } else {
            self.update(observe)?;
        }
        let state = self.snapshot()?.state;
        if let Some(proof) = &product_proof {
            state.verify_product_recovery(proof)?;
        }
        let generation = state
            .local_generation(target)
            .ok_or(WorkspaceStateError::NoPendingEdit)?;
        let entry = state
            .entry(target)
            .cloned()
            .ok_or(WorkspaceStateError::NoPendingEdit)?;
        check_eligibility(eligibility, session)?;
        Ok(WorkspaceReview {
            binding: self.binding.clone(),
            eligibility: eligibility.map(|v| v.binding.clone()),
            origin: self.origin.clone(),
            account_id: self.account_id.clone(),
            read,
            generation,
            entry,
            product_proof,
        })
    }
}
