//! Restore a superseded conflict before retiring its matching durable journal.
use super::intent::{abandon_phase, state_digest};
use super::{AbandonPhase, CapturedAbandon, V1KeepLocalAbandonReview, V1KeepLocalIntent};
use crate::{protocol_fence::V1Lease, SyncCoordError, SyncCoordinator, V1ProductOutbox};
use crate::{SyncConfig, V1LocalEditContext};
use cloud_client::Session;
use std::sync::Arc;

impl SyncCoordinator {
    fn capture_keep_local_abandon_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedAbandon, SyncCoordError> {
        let context = V1LocalEditContext {
            origin: session.instance_origin(),
            account_id: session.account_id().to_owned(),
            config_generation: config.generation,
        };
        lease.ensure_v1(&context.origin, &context.account_id)?;
        lease.ensure_no_uncertain(&context.origin, &context.account_id)?;
        let intent = outbox
            .load_rebase(&context, key)?
            .ok_or(SyncCoordError::StaleReview)?;
        intent.validate(&context)?;
        if intent.key != key {
            return Err(SyncCoordError::StaleReview);
        }
        let (candidate, proof) = outbox
            .newer_rebase_candidate(&context, &intent)?
            .ok_or(SyncCoordError::StaleReview)?;
        if candidate.key != key
            || candidate.generation <= intent.source_generation
            || proof.generation() != candidate.generation
        {
            return Err(SyncCoordError::StaleReview);
        }
        let snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        let phase = abandon_phase(&intent, &snapshot)?;
        Ok(CapturedAbandon {
            intent,
            candidate,
            proof,
            phase,
        })
    }

    fn capture_keep_local_abandon(
        &self,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedAbandon, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(session, config, || {
            self.capture_keep_local_abandon_locked(&lease, session, config, key, outbox)
        })
    }

    pub(super) fn restore_conflict_for_abandon(
        &self,
        lease: &V1Lease,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        phase: AbandonPhase,
    ) -> Result<(), SyncCoordError> {
        if phase != AbandonPhase::After {
            return Ok(());
        }
        let mut snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        if abandon_phase(intent, &snapshot)? != AbandonPhase::After {
            return Err(SyncCoordError::StaleReview);
        }
        snapshot
            .state
            .entries
            .insert(intent.key.clone(), intent.before.clone());
        if state_digest(&snapshot.state)? != intent.before_digest {
            return Err(SyncCoordError::StaleReview);
        }
        let saved = self.save_state(snapshot, lease)?;
        if abandon_phase(intent, &saved)? != AbandonPhase::Restored {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(())
    }

    /// Review abandonment of a superseded decision. This captures an exact
    /// source-bound newer candidate and the frozen local state. No Cloud
    /// operation is made: abandonment restores a conflict, not a resolution.
    pub async fn review_keep_local_abandon(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1KeepLocalAbandonReview, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let captured = self.capture_keep_local_abandon(&session, config, key, outbox)?;
        Ok(V1KeepLocalAbandonReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            captured,
        })
    }

    /// Confirm abandoning the saved decision. Restore the exact original
    /// conflict before clearing its journal. A crash at either step leaves
    /// one of the recognized phases and requires a fresh explicit review.
    pub fn abandon_keep_local_recovery(
        &self,
        review: &V1KeepLocalAbandonReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let captured = self.capture_keep_local_abandon_locked(
                &lease,
                &review.session,
                review.config,
                &review.captured.intent.key,
                outbox,
            )?;
            if captured != review.captured {
                return Err(SyncCoordError::StaleReview);
            }
            let intent = &captured.intent;
            let context = V1LocalEditContext {
                origin: review.session.instance_origin(),
                account_id: review.session.account_id().to_owned(),
                config_generation: review.config.generation,
            };
            self.restore_conflict_for_abandon(&lease, &context, intent, captured.phase)?;
            if !outbox.abandon_rebase(&context, intent, &captured.candidate, &captured.proof)? {
                return Err(SyncCoordError::StaleReview);
            }
            Ok(())
        })
    }
}
