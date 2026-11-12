//! Review changed source generations without exposing another account's newer edit.
use super::intent::abandon_phase;
use super::{CapturedSupersededRebase, V1SupersededRebaseReview};
use crate::{protocol_fence::V1Lease, SyncCoordError, SyncCoordinator, V1ProductOutbox};
use crate::{SyncConfig, V1LocalEditContext};
use cloud_client::Session;
use std::sync::Arc;

impl SyncCoordinator {
    fn capture_superseded_rebase_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedSupersededRebase, SyncCoordError> {
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
        let generation = outbox
            .superseded_rebase_generation(&context, &intent)?
            .filter(|generation| *generation > intent.source_generation)
            .ok_or(SyncCoordError::StaleReview)?;
        let snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        let phase = abandon_phase(&intent, &snapshot)?;
        Ok(CapturedSupersededRebase {
            intent,
            generation,
            phase,
        })
    }

    /// Review a superseded journal without retrieving the newer local value.
    /// No network request is needed: the only permitted result is the old
    /// conflict, which still requires separate review before any upload.
    pub async fn review_superseded_rebase(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1SupersededRebaseReview, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        let captured = self.with_live_session(&session, config, || {
            self.capture_superseded_rebase_locked(&lease, &session, config, key, outbox)
        })?;
        Ok(V1SupersededRebaseReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            captured,
        })
    }

    /// Restore the exact prior conflict before retiring the journal. The
    /// backend must atomically recheck the captured source generation when
    /// removing the journal. A failure retains a replayable recovery phase.
    pub fn abandon_superseded_rebase(
        &self,
        review: &V1SupersededRebaseReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let captured = self.capture_superseded_rebase_locked(
                &lease,
                &review.session,
                review.config,
                &review.captured.intent.key,
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
            self.restore_conflict_for_abandon(&lease, &context, &captured.intent, captured.phase)?;
            if !outbox.abandon_superseded_rebase(&context, &captured.intent, captured.generation)? {
                return Err(SyncCoordError::StaleReview);
            }
            Ok(())
        })
    }
}
