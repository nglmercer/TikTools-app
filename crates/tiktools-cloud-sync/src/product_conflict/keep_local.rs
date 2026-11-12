//! Journal-first commit of a separately reviewed keep-local decision.
use super::intent::{desired_entry, rebase_phase, state_digest};
use super::{RebasePhase, V1KeepLocalIntent, V1ProductConflictReview};
use crate::{protocol_fence::V1Lease, SyncCoordError, SyncCoordinator, V1ProductOutbox};
use crate::{V1ConflictSourceProof, V1LocalEditContext};

impl SyncCoordinator {
    /// Commit a separately confirmed keep-local decision. A new Cloud read
    /// must exactly match the reviewed value; the durable journal is created
    /// before JSON or its receipt changes and survives every later failure.
    pub async fn keep_local_product_conflict(
        &self,
        review: &V1ProductConflictReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        if !self.product_conflict_review_is_current(review, outbox)? {
            return Err(SyncCoordError::StaleReview);
        }
        let cloud = review
            .session
            .export_sync()
            .await?
            .records
            .into_iter()
            .find(|record| record.key == review.key);
        if cloud != review.cloud {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let captured = self.capture_product_conflict_locked(
                &lease,
                &review.session,
                review.config,
                &review.key,
                outbox,
            )?;
            if captured != review.captured {
                return Err(SyncCoordError::StaleReview);
            }
            let context = V1LocalEditContext {
                origin: review.session.instance_origin(),
                account_id: review.session.account_id().to_owned(),
                config_generation: review.config.generation,
            };
            let before = captured
                .snapshot
                .state
                .entries
                .get(&review.key)
                .cloned()
                .ok_or(SyncCoordError::StaleReview)?;
            let pending = before.pending.as_ref().ok_or(SyncCoordError::StaleReview)?;
            let after = desired_entry(&before, cloud.clone(), pending);
            let mut next_state = captured.snapshot.state.clone();
            next_state.entries.insert(review.key.clone(), after.clone());
            let intent = V1KeepLocalIntent {
                format_version: 1,
                origin: context.origin.clone(),
                account_id: context.account_id.clone(),
                config_generation: context.config_generation,
                source_generation: captured.source.generation(),
                initial_runtime_epoch: captured.source.runtime_epoch().to_owned(),
                key: review.key.clone(),
                before_generation: captured.snapshot.generation,
                before,
                cloud: cloud.clone(),
                after,
                before_digest: state_digest(&captured.snapshot.state)?,
                after_digest: state_digest(&next_state)?,
            };
            intent.validate(&context)?;
            if !outbox.prepare_rebase(&context, &intent, &captured.source)? {
                return Err(SyncCoordError::StaleReview);
            }
            self.finish_keep_local_locked(&lease, &context, &intent, &captured.source, outbox)
        })
    }

    pub(super) fn finish_keep_local_locked(
        &self,
        lease: &V1Lease,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        proof: &V1ConflictSourceProof,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        intent.validate(context)?;
        if proof.generation() != intent.source_generation {
            return Err(SyncCoordError::StaleReview);
        }
        lease.ensure_v1(&context.origin, &context.account_id)?;
        lease.ensure_no_uncertain(&context.origin, &context.account_id)?;
        let mut snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        if rebase_phase(intent, &snapshot)? == RebasePhase::Before {
            snapshot
                .state
                .entries
                .insert(intent.key.clone(), intent.after.clone());
            if state_digest(&snapshot.state)? != intent.after_digest {
                return Err(SyncCoordError::StaleReview);
            }
            snapshot = self.save_state(snapshot, lease)?;
        }
        if rebase_phase(intent, &snapshot)? != RebasePhase::After {
            return Err(SyncCoordError::StaleReview);
        }
        if !outbox.finish_rebase(context, intent, proof)? {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(())
    }
}
