//! Native source admission for queued and reviewed product proofs.
use super::{ProductUploadAdmission, WorkspaceEligibility};
use super::{WorkspaceCoordError, WorkspaceCoordinator};
use cloud_client::{WorkspaceKey, WorkspaceProductProof, WorkspaceStateError};

impl WorkspaceCoordinator {
    /// The short source fence encloses only the local decision and durable
    /// preparation, never the HTTP request. A later source mutation can be
    /// staged separately while the original request is in flight.
    pub(super) fn admit_product<T>(
        &self,
        target: &WorkspaceKey,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
        operation: impl FnOnce(&WorkspaceProductProof) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        let proof = self
            .snapshot()?
            .state
            .product_queued_proof(target)
            .ok_or(WorkspaceStateError::StaleReview)?;
        self.admit_product_proof(&proof, eligibility, admission, operation)
    }

    pub(super) fn admit_product_proof<T>(
        &self,
        proof: &WorkspaceProductProof,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
        operation: impl FnOnce(&WorkspaceProductProof) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        self.admit_product_proof_mode(proof, eligibility, admission, false, operation)
    }

    pub(super) fn discard_product_proof<T>(
        &self,
        proof: &WorkspaceProductProof,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
        operation: impl FnOnce(&WorkspaceProductProof) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        self.admit_product_proof_mode(proof, eligibility, admission, true, operation)
    }

    fn admit_product_proof_mode<T>(
        &self,
        proof: &WorkspaceProductProof,
        eligibility: &WorkspaceEligibility,
        admission: &ProductUploadAdmission<'_>,
        discard: bool,
        operation: impl FnOnce(&WorkspaceProductProof) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        if crate::v2_product_outbox::OWNED_EDIT_COLLECTIONS
            .contains(&proof.target().collection.as_str())
        {
            // Owner-owned edit (no native source row): the digest must still be
            // the one staged under this account's consent generation.
            let expected = crate::v2_product_outbox::owned_edit_digest(
                proof.target(),
                proof.edit(),
                admission.config_generation,
            );
            if expected.as_deref() != Some(proof.decision_digest()) {
                return Err(WorkspaceStateError::StaleReview.into());
            }
            let mut operation = Some(operation);
            let mut result = None;
            let mut admit = || {
                let operation = operation.take().ok_or(WorkspaceStateError::StaleReview)?;
                eligibility.check()?;
                result = Some(operation(proof)?);
                Ok(())
            };
            if discard {
                admission.source.with_fenced_owned_discard(
                    proof.target(),
                    proof.edit(),
                    &mut admit,
                )?;
            } else {
                admission.source.with_fenced_owned_edit(
                    proof.target(),
                    proof.edit(),
                    &mut admit,
                )?;
            }
            return result.ok_or_else(|| WorkspaceStateError::StaleReview.into());
        }
        let decision = crate::V2ProductDecision::from_proof(
            &self.origin,
            &self.account_id,
            admission.config_generation,
            proof,
        )?;
        let mut operation = Some(operation);
        let mut result = None;
        let mut admit = || {
            let operation = operation.take().ok_or(WorkspaceStateError::StaleReview)?;
            eligibility.check()?;
            result = Some(operation(proof)?);
            Ok(())
        };
        admission.source.with_fenced_upload(&decision, &mut admit)?;
        result.ok_or_else(|| WorkspaceStateError::StaleReview.into())
    }
}
