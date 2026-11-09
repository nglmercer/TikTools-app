//! Private admission of a cached v2 plugin selection for local review.
//!
//! This does not fetch, apply, activate, or acknowledge any plugin state.
//! Product activation additionally needs source/runtime/package admission,
//! durable application recovery, and a server barrier for uncertain writes.
use std::sync::Arc;

use cloud_client::{
    Session, WorkspaceKey, WorkspaceRecord, WorkspaceStateSnapshot, WorkspaceStore,
};

use crate::{SyncConfigEpoch, SyncCoordError, SyncCoordinator, V2LocalEditContext};

/// Opaque evidence of one exact cached, non-pending plugin selection.
/// The record is not by itself permission to change the local application.
pub struct V2RemoteApplyReview {
    binding: Arc<()>,
    session: Arc<Session>,
    epoch: SyncConfigEpoch,
    snapshot_generation: u64,
    record: WorkspaceRecord,
}

fn ready_record(
    snapshot: Option<WorkspaceStateSnapshot>,
    target: &WorkspaceKey,
) -> Result<(u64, WorkspaceRecord), SyncCoordError> {
    let snapshot = snapshot.ok_or(SyncCoordError::StaleReview)?;
    if snapshot.generation == 0
        || !snapshot.state.bootstrap_complete
        || snapshot.state.needs_rebuild
    {
        return Err(SyncCoordError::StaleReview);
    }
    let record = snapshot
        .state
        .entry(target)
        .filter(|entry| entry.pending.is_none() && entry.next.is_none())
        .and_then(|entry| entry.remote.as_ref())
        .filter(|record| record.collection == target.collection && record.key == target.key)
        .ok_or(SyncCoordError::StaleReview)?;
    Ok((snapshot.generation, record.clone()))
}

impl SyncCoordinator {
    fn v2_product_remote_store(&self, session: &Session) -> Result<WorkspaceStore, SyncCoordError> {
        let root = self
            .config_path
            .parent()
            .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
        Ok(WorkspaceStore::open(
            root,
            session.instance_origin(),
            session.account_id(),
        )?)
    }

    /// Capture a local cache observation under the exact live session,
    /// account-scoped v2 consent and protocol. This performs no HTTP request.
    /// Call from a bounded blocking worker: private files may stall.
    #[doc(hidden)]
    pub fn review_v2_product_remote(
        &self,
        session: &Arc<Session>,
        plugin_id: &str,
    ) -> Result<V2RemoteApplyReview, SyncCoordError> {
        let target = WorkspaceKey::new("plugin-selections", plugin_id)
            .map_err(|_| SyncCoordError::StaleReview)?;
        let epoch = self
            .capture_v2_config_epoch(session)
            .ok_or(SyncCoordError::StaleReview)?;
        let origin = session.instance_origin();
        let account = session.account_id();
        let _lease = self.protocol.try_v2(&origin, account)?;
        let marker_lease = self.protocol.try_v1()?;
        marker_lease.ensure_product_outbox(&origin, account)?;
        let store = self.v2_product_remote_store(session)?;
        let (snapshot_generation, record) = self
            .auth
            .with_current_session_permit(session, || {
                if !self.is_v2_config_epoch_current(&epoch, session) {
                    return Err(SyncCoordError::StaleReview);
                }
                store
                    .try_with_locked_snapshot(|snapshot| ready_record(snapshot, &target))?
                    .ok_or(crate::ProtocolFenceError::Busy.into())
            })
            .map_err(|_| SyncCoordError::StaleReview)??;
        Ok(V2RemoteApplyReview {
            binding: self.recovery_binding.clone(),
            session: session.clone(),
            epoch,
            snapshot_generation,
            record,
        })
    }

    /// Revalidate the exact session, scoped consent, selected protocol and
    /// cache under their cooperative locks before a short native transaction.
    /// The caller must already hold its local plugin mutation fence and must
    /// atomically revalidate the source generation, runtime, installed package,
    /// pending outboxes, journals and untracked marker in `operation`. This
    /// callback must not await, do network I/O, or re-enter sync/auth. It must
    /// write a durable intent before any application effect and must not stage
    /// the remote record back into the local outbox. The lock and account permit
    /// can be delayed by blocked native I/O; run on a bounded blocking worker.
    /// A server-backed barrier is additionally required before enabling this
    /// for product use; a cached record cannot prove an old HTTP write ended.
    /// Tombstones do not authorize removing an installed package. If a caller
    /// exposes the callback's record asynchronously, it must fence delivery to
    /// the original account/session as well; return from this function alone
    /// does not protect a later UI response.
    #[doc(hidden)]
    pub fn with_reviewed_v2_product_remote<R>(
        &self,
        review: &V2RemoteApplyReview,
        operation: impl FnOnce(&V2LocalEditContext, &WorkspaceRecord) -> Result<R, SyncCoordError>,
    ) -> Result<R, SyncCoordError> {
        if !Arc::ptr_eq(&review.binding, &self.recovery_binding) {
            return Err(SyncCoordError::StaleReview);
        }
        let session = &review.session;
        let origin = session.instance_origin();
        let account = session.account_id();
        let target = WorkspaceKey::new("plugin-selections", &review.record.key)
            .map_err(|_| SyncCoordError::StaleReview)?;
        let _lease = self.protocol.try_v2(&origin, account)?;
        let marker_lease = self.protocol.try_v1()?;
        marker_lease.ensure_product_outbox(&origin, account)?;
        let store = self.v2_product_remote_store(session)?;
        self.auth
            .with_current_session_permit(session, || {
                if !self.is_v2_config_epoch_current(&review.epoch, session) {
                    return Err(SyncCoordError::StaleReview);
                }
                store
                    .try_with_locked_snapshot(|snapshot| {
                        let (generation, record) = ready_record(snapshot, &target)?;
                        if generation != review.snapshot_generation || record != review.record {
                            return Err(SyncCoordError::StaleReview);
                        }
                        let context = V2LocalEditContext {
                            origin: origin.clone(),
                            account_id: account.to_owned(),
                            config_generation: review.epoch.config.generation,
                        };
                        operation(&context, &record)
                    })?
                    .ok_or(crate::ProtocolFenceError::Busy.into())
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }
}
