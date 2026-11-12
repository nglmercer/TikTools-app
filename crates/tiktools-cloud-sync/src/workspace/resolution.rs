//! Explicit generation-bound keep-local and take-cloud decisions.
use super::{check_eligibility, ProductUploadAdmission, WorkspaceEligibility, WorkspaceReview};
use super::{WorkspaceCoordError, WorkspaceCoordinator};
use cloud_client::{Session, WorkspaceState, WorkspaceStateError};
use std::sync::Arc;

impl WorkspaceCoordinator {
    /// Choice is bound to the compared local generation and a new live read.
    /// Taking cloud changes only the cache/queue; local activation is separate.
    pub async fn resolve(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        keep_local: bool,
    ) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.resolve_inner(session, review, keep_local, None, None)
            .await
    }

    pub async fn resolve_guarded(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        keep_local: bool,
        eligibility: &WorkspaceEligibility,
    ) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.resolve_inner(session, review, keep_local, Some(eligibility), None)
            .await
    }

    /// An explicit keep-local choice only rebases the durable pending edit;
    /// it does not apply Cloud state or send a request. Both the exact source
    /// and session/consent publication fences are required through the save.
    pub(crate) async fn resolve_product_keep_local_guarded(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
    ) -> Result<(), WorkspaceCoordError> {
        if eligibility.publication.is_none() || review.product_proof.is_none() {
            return Err(WorkspaceCoordError::Ineligible);
        }
        self.resolve_inner(session, review, true, Some(eligibility), Some(admission))
            .await
    }

    /// An explicit take-cloud choice discards the pending edit and retains
    /// the fresh Cloud record as the cache baseline. It never applies Cloud
    /// state locally or sends a request. The same source and session/consent
    /// publication fences as keep-local are required through the save.
    pub(crate) async fn resolve_product_take_cloud_guarded(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
    ) -> Result<(), WorkspaceCoordError> {
        if eligibility.publication.is_none() || review.product_proof.is_none() {
            return Err(WorkspaceCoordError::Ineligible);
        }
        self.resolve_inner(session, review, false, Some(eligibility), Some(admission))
            .await
    }

    async fn resolve_inner(
        &self,
        session: &Session,
        review: &WorkspaceReview,
        keep_local: bool,
        eligibility: Option<&WorkspaceEligibility>,
        product: Option<&ProductUploadAdmission<'_>>,
    ) -> Result<(), WorkspaceCoordError> {
        self.identity(session)?;
        check_eligibility(eligibility, session)?;
        let same_eligibility = match (&review.eligibility, eligibility) {
            (None, None) => true,
            (Some(reviewed), Some(current)) => Arc::ptr_eq(reviewed, &current.binding),
            _ => false,
        };
        if !same_eligibility {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        if review.origin != self.origin || review.account_id != self.account_id {
            return Err(WorkspaceStateError::WrongIdentity.into());
        }
        if !Arc::ptr_eq(&review.binding, &self.binding) {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        // A product-bound review must never fall back to generic resolution,
        // which has no source or atomic session publication fence.
        if review.product_proof.is_some() != product.is_some() {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        if self.snapshot()?.state.is_product_guarded() != product.is_some() {
            return Err(crate::ProtocolFenceError::ProductOutboxRequired.into());
        }
        check_eligibility(eligibility, session)?;
        let fresh = session
            .workspace_read_for_resolution(review.target())
            .await?;
        check_eligibility(eligibility, session)?;
        if fresh.record() != review.read.record() {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        let change = |state: &mut WorkspaceState| {
            check_eligibility(eligibility, session)?;
            if let Some(proof) = &review.product_proof {
                state.verify_product_recovery(proof)?;
            }
            let current = state
                .entry(review.target())
                .ok_or(WorkspaceStateError::NoPendingEdit)?;
            if current.pending != review.entry.pending || current.next != review.entry.next {
                return Err(WorkspaceStateError::StaleReview.into());
            }
            state.resolve(
                &fresh,
                review.read.revision(),
                review.generation,
                keep_local,
            )?;
            Ok(())
        };
        if let Some(product) = product {
            let eligibility = eligibility.ok_or(WorkspaceCoordError::Ineligible)?;
            self.admit_product_proof(
                review
                    .product_proof
                    .as_ref()
                    .ok_or(WorkspaceStateError::StaleReview)?,
                eligibility,
                product,
                |_| self.update_product(eligibility, change),
            )
        } else {
            self.update(change)
        }
    }
}
