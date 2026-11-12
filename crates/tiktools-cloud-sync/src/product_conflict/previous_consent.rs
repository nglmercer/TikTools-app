//! Explicit review and abandonment of a decision from an earlier consent epoch.
use super::intent::abandon_phase;
use super::{AbandonPhase, V1KeepLocalIntent, V1PreviousConsentRebaseReview};
use crate::{protocol_fence::V1Lease, SyncCoordError, SyncCoordinator, V1ProductOutbox};
use crate::{SyncConfig, V1LocalEditContext};
use cloud_client::Session;
use std::sync::Arc;

impl SyncCoordinator {
    fn capture_previous_consent_rebase_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(V1KeepLocalIntent, AbandonPhase), SyncCoordError> {
        let context = V1LocalEditContext {
            origin: session.instance_origin(),
            account_id: session.account_id().to_owned(),
            config_generation: config.generation,
        };
        lease.ensure_v1(&context.origin, &context.account_id)?;
        lease.ensure_no_uncertain(&context.origin, &context.account_id)?;
        let intent = outbox
            .load_rebase_with_previous_consent(&context, key)?
            .ok_or(SyncCoordError::StaleReview)?;
        if intent.origin != context.origin
            || intent.account_id != context.account_id
            || intent.key != key
            || intent.config_generation == context.config_generation
        {
            return Err(SyncCoordError::StaleReview);
        }
        let old_context = V1LocalEditContext {
            origin: context.origin.clone(),
            account_id: context.account_id.clone(),
            config_generation: intent.config_generation,
        };
        intent.validate(&old_context)?;
        let snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        let phase = abandon_phase(&intent, &snapshot)?;
        Ok((intent, phase))
    }

    /// Inspect a previous-consent journal without sending a request or
    /// exposing another account's data. A separate confirmation is required.
    pub async fn review_previous_consent_rebase(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1PreviousConsentRebaseReview, SyncCoordError> {
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
        let (intent, phase) = self.with_live_session(&session, config, || {
            self.capture_previous_consent_rebase_locked(&lease, &session, config, key, outbox)
        })?;
        Ok(V1PreviousConsentRebaseReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            intent,
            phase,
        })
    }

    /// Restore the exact conflict first, then retire only the matching old
    /// journal. A crash leaves a recognized phase for a new explicit review.
    pub fn abandon_previous_consent_rebase(
        &self,
        review: &V1PreviousConsentRebaseReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(), SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let captured = self.capture_previous_consent_rebase_locked(
                &lease,
                &review.session,
                review.config,
                &review.intent.key,
                outbox,
            )?;
            if captured != (review.intent.clone(), review.phase) {
                return Err(SyncCoordError::StaleReview);
            }
            let context = V1LocalEditContext {
                origin: review.session.instance_origin(),
                account_id: review.session.account_id().to_owned(),
                config_generation: review.config.generation,
            };
            self.restore_conflict_for_abandon(&lease, &context, &review.intent, review.phase)?;
            if !outbox.abandon_rebase_with_previous_consent(&context, &review.intent)? {
                return Err(SyncCoordError::StaleReview);
            }
            Ok(())
        })
    }
}
