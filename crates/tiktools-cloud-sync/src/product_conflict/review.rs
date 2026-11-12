//! Source-fenced conflict capture and fresh remote comparison.
use super::{CapturedConflict, V1ProductConflictReview};
use crate::{protocol_fence::V1Lease, SyncCoordError, SyncCoordinator, V1ProductOutbox};
use crate::{SyncConfig, V1LocalEditContext};
use cloud_client::{Session, SyncError};
use std::sync::Arc;

impl SyncCoordinator {
    pub(super) fn capture_product_conflict_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedConflict, SyncCoordError> {
        let origin = session.instance_origin();
        let account = session.account_id();
        lease.ensure_v1(&origin, account)?;
        lease.ensure_no_uncertain(&origin, account)?;
        let snapshot = self.load_for_identity(lease, &origin, account)?;
        let pending = snapshot
            .state
            .entries
            .get(key)
            .and_then(|entry| entry.pending.as_ref())
            .filter(|pending| pending.conflict)
            .ok_or(SyncCoordError::StaleReview)?;
        let context = V1LocalEditContext {
            origin,
            account_id: account.to_owned(),
            config_generation: config.generation,
        };
        if outbox.has_pending_rebase(&context)? {
            return Err(SyncCoordError::StaleReview);
        }
        let source = outbox
            .conflict_source_proof(&context, key, pending)?
            .ok_or(SyncCoordError::StaleReview)?;
        Ok(CapturedConflict { snapshot, source })
    }

    fn capture_product_conflict(
        &self,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<CapturedConflict, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(session, config, || {
            self.capture_product_conflict_locked(&lease, session, config, key, outbox)
        })
    }

    /// Capture a conflict only after validating its durable source, fetch a
    /// fresh Cloud snapshot without holding the admission lock, then capture
    /// again. Any local change, session switch or opt-out invalidates it.
    pub async fn review_product_conflict(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1ProductConflictReview, SyncCoordError> {
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let before = self.capture_product_conflict(&session, config, key, outbox)?;
        let cloud = session
            .export_sync()
            .await?
            .records
            .into_iter()
            .find(|record| record.key == key);
        let after = self.capture_product_conflict(&session, config, key, outbox)?;
        if before != after {
            return Err(SyncCoordError::StaleReview);
        }
        let entry = &after.snapshot.state.entries[key];
        if let Some(base) = entry.remote.as_ref() {
            let Some(current) = cloud.as_ref() else {
                return Err(SyncError::Protocol.into());
            };
            if current.revision < base.revision
                || (current.revision == base.revision && current != base)
            {
                return Err(SyncError::Protocol.into());
            }
        }
        if entry
            .pending
            .as_ref()
            .and_then(|pending| pending.base_revision)
            .is_some_and(|base| cloud.as_ref().is_none_or(|current| current.revision < base))
        {
            return Err(SyncError::Protocol.into());
        }
        Ok(V1ProductConflictReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            key: key.to_owned(),
            captured: after,
            cloud,
        })
    }

    /// Check only that the local inputs of a prior review remain unchanged.
    /// A later decision must also re-fetch Cloud and use a crash-safe journal.
    pub fn product_conflict_review_is_current(
        &self,
        review: &V1ProductConflictReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<bool, SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Ok(false);
        }
        match self.capture_product_conflict(&review.session, review.config, &review.key, outbox) {
            Ok(current) => Ok(current == review.captured),
            Err(SyncCoordError::StaleReview) => Ok(false),
            Err(error) => Err(error),
        }
    }
}
