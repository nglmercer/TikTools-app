//! Fresh review and explicit resumption of an interrupted keep-local intent.
use super::intent::rebase_phase;
use super::{RebasePhase, V1KeepLocalIntent, V1KeepLocalRecoveryReview};
use crate::{protocol_fence::V1Lease, SyncCoordError, SyncCoordinator, V1ProductOutbox};
use crate::{SyncConfig, V1ConflictSourceProof, V1LocalEditContext};
use cloud_client::Session;
use std::sync::Arc;

impl SyncCoordinator {
    fn capture_keep_local_recovery_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(V1KeepLocalIntent, V1ConflictSourceProof, RebasePhase), SyncCoordError> {
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
        let proof = outbox
            .rebase_source_proof(&context, &intent)?
            .ok_or(SyncCoordError::StaleReview)?;
        if proof.generation() != intent.source_generation {
            return Err(SyncCoordError::StaleReview);
        }
        let snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        let phase = rebase_phase(&intent, &snapshot)?;
        Ok((intent, proof, phase))
    }

    fn capture_keep_local_recovery(
        &self,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(V1KeepLocalIntent, V1ConflictSourceProof, RebasePhase), SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(session, config, || {
            self.capture_keep_local_recovery_locked(&lease, session, config, key, outbox)
        })
    }

    /// Inspect a saved decision after interruption. This rechecks current
    /// Cloud and source and returns an opaque native review; it never resumes
    /// the decision automatically. If a newer local edit changed the source,
    /// the journal and edit remain intact and sync stays blocked. The
    /// separate explicit abandonment flow restores the original conflict
    /// before retiring the journal; deleting it alone is unsafe.
    pub async fn review_keep_local_recovery(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1KeepLocalRecoveryReview, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let before = self.capture_keep_local_recovery(&session, config, key, outbox)?;
        let cloud = session
            .export_sync()
            .await?
            .records
            .into_iter()
            .find(|record| record.key == key);
        if cloud != before.0.cloud {
            return Err(SyncCoordError::StaleReview);
        }
        let after = self.capture_keep_local_recovery(&session, config, key, outbox)?;
        if before != after {
            return Err(SyncCoordError::StaleReview);
        }
        Ok(V1KeepLocalRecoveryReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            intent: after.0,
            proof: after.1,
            phase: after.2,
        })
    }

    /// Resume only after a new explicit recovery review. Re-fetch Cloud and
    /// compare all frozen local inputs again; no remote write is sent here.
    pub async fn resume_keep_local_recovery(
        &self,
        review: &V1KeepLocalRecoveryReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let before = self.capture_keep_local_recovery(
            &review.session,
            review.config,
            &review.intent.key,
            outbox,
        )?;
        if before != (review.intent.clone(), review.proof.clone(), review.phase) {
            return Err(SyncCoordError::StaleReview);
        }
        let cloud = review
            .session
            .export_sync()
            .await?
            .records
            .into_iter()
            .find(|record| record.key == review.intent.key);
        if cloud != review.intent.cloud {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let current = self.capture_keep_local_recovery_locked(
                &lease,
                &review.session,
                review.config,
                &review.intent.key,
                outbox,
            )?;
            if current != (review.intent.clone(), review.proof.clone(), review.phase) {
                return Err(SyncCoordError::StaleReview);
            }
            let context = V1LocalEditContext {
                origin: review.session.instance_origin(),
                account_id: review.session.account_id().to_owned(),
                config_generation: review.config.generation,
            };
            self.finish_keep_local_locked(&lease, &context, &review.intent, &review.proof, outbox)
        })
    }
}
