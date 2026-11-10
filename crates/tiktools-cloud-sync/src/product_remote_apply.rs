//! Session- and state-fenced admission of a pulled v1 plugin selection.
//!
//! This native boundary does not alter a plugin. It exposes an exact durable
//! Cloud record only while the live session, consent and protocol admission
//! are held. The product must additionally hold its plugin mutation gate and
//! atomically revalidate its local source, package version and durable apply
//! intent before changing a plugin or acknowledging any outcome.
use std::sync::Arc;

use cloud_client::{Record, Session};

use crate::{
    protocol_fence::V1Lease, SyncConfig, SyncCoordError, SyncCoordinator, V1LocalEditContext,
    V1ProductOutbox,
};

/// An opaque native observation of one exact, durably pulled selection.
/// It is neither serializable nor permission to modify a plugin by itself.
pub struct V1RemoteApplyReview {
    binding: Arc<()>,
    session: Arc<Session>,
    config: SyncConfig,
    snapshot_generation: u64,
    record: Record,
}

impl V1RemoteApplyReview {
    /// Nonsecret record from the persisted account partition. Its value can
    /// be a tombstone; a tombstone disables, never uninstalls, locally.
    pub fn record(&self) -> &Record {
        &self.record
    }
}

impl SyncCoordinator {
    fn capture_product_remote_locked(
        &self,
        lease: &V1Lease,
        session: &Arc<Session>,
        config: SyncConfig,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<(u64, Record), SyncCoordError> {
        let context = V1LocalEditContext {
            origin: session.instance_origin(),
            account_id: session.account_id().to_owned(),
            config_generation: config.generation,
        };
        lease.ensure_v1(&context.origin, &context.account_id)?;
        lease.ensure_no_uncertain(&context.origin, &context.account_id)?;
        if outbox.has_pending_rebase(&context)? {
            return Err(SyncCoordError::StaleReview);
        }
        let snapshot = self.load_for_identity(lease, &context.origin, &context.account_id)?;
        let entry = snapshot
            .state
            .entries
            .get(key)
            .filter(|entry| entry.pending.is_none())
            .ok_or(SyncCoordError::StaleReview)?;
        let record = entry.remote.as_ref().ok_or(SyncCoordError::StaleReview)?;
        // Only durable observations count; a fresh in-memory state does not.
        if snapshot.generation == 0 || record.key != key {
            return Err(SyncCoordError::StaleReview);
        }
        Ok((snapshot.generation, record.clone()))
    }

    /// Capture a pulled selection for a native product adapter. This observes
    /// the last committed local Cloud snapshot; it does not fetch the network.
    pub async fn review_product_remote_apply(
        &self,
        key: &str,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<V1RemoteApplyReview, SyncCoordError> {
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
        let (snapshot_generation, record) = self.with_live_session(&session, config, || {
            self.capture_product_remote_locked(&lease, &session, config, key, outbox)
        })?;
        Ok(V1RemoteApplyReview {
            binding: self.recovery_binding.clone(),
            session,
            config,
            snapshot_generation,
            record,
        })
    }

    /// Execute a short product-side transaction for an exact pulled record.
    /// The caller must already hold its plugin mutation gate; inside this
    /// callback it must verify source generation, installed version and any
    /// unresolved local candidate atomically with its durable intent. It may
    /// not await, perform network I/O, or re-enter sync/authentication. A
    /// callback result alone does not acknowledge a plugin runtime outcome.
    pub fn with_reviewed_product_remote<R>(
        &self,
        review: &V1RemoteApplyReview,
        outbox: &mut dyn V1ProductOutbox,
        operation: impl FnOnce(&V1LocalEditContext, &Record) -> Result<R, SyncCoordError>,
    ) -> Result<R, SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let lease = self.protocol.try_v1()?;
        let _admission = lease.try_admission()?;
        self.with_live_session(&review.session, review.config, || {
            let (generation, record) = self.capture_product_remote_locked(
                &lease,
                &review.session,
                review.config,
                &review.record.key,
                outbox,
            )?;
            if generation != review.snapshot_generation || record != review.record {
                return Err(SyncCoordError::StaleReview);
            }
            let context = V1LocalEditContext {
                origin: review.session.instance_origin(),
                account_id: review.session.account_id().to_owned(),
                config_generation: review.config.generation,
            };
            operation(&context, &record)
        })
    }
}
