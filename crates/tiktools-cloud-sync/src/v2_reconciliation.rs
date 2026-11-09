//! Experimental account-scoped barrier for explicitly reviewed local edits.
//! This does not inspect or transfer local plugin state, and is not a UI API.
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::{Duration, Instant},
};

use crate::{SyncConfigEpoch, SyncCoordinator, V2LocalEditContext, WorkspaceCoordError};
use cloud_client::{Session, WorkspaceStateSnapshot, WorkspaceStore};

const REVIEW_LIFETIME: Duration = Duration::from_secs(60);

/// Non-serializable, single-use server observation tied to an exact session,
/// consent generation and durable workspace generation. This is not user
/// consent and does not validate a local plugin's source or runtime.
pub struct V2ReconciliationBarrier {
    binding: Arc<()>,
    session: Arc<Session>,
    config: SyncConfigEpoch,
    workspace_generation: u64,
    expires: Instant,
    consumed: AtomicBool,
    source_guard: Arc<Mutex<Option<crate::ProductEmissionLease>>>,
    source_dir: PathBuf,
}

/// Borrowed evidence that the reconciliation callback owns the source lease.
/// It cannot be constructed or retained outside the synchronous callback.
pub struct V2ReconciliationAdmission<'a> {
    context: V2LocalEditContext,
    source_dir: &'a Path,
}
impl V2ReconciliationAdmission<'_> {
    pub fn context(&self) -> &V2LocalEditContext {
        &self.context
    }
    /// Confirm the native source uses the same on-disk fence. An unavailable
    /// or changed directory fails closed; noncooperating filesystem writers
    /// remain outside the advisory lock guarantee.
    pub fn matches_source_dir(&self, path: &Path) -> bool {
        std::fs::canonicalize(path).is_ok_and(|actual| actual == self.source_dir)
    }
}

// If expiry fires while final validation holds the mutex, the timer cannot
// take the lease. Release it when validation exits instead, including errors.
struct SourceUse<'a> {
    lease: MutexGuard<'a, Option<crate::ProductEmissionLease>>,
    expires: Instant,
    consumed: &'a AtomicBool,
}
impl Drop for SourceUse<'_> {
    fn drop(&mut self) {
        if self.consumed.load(Ordering::Acquire) || Instant::now() >= self.expires {
            self.lease.take();
        }
    }
}

fn ready(snapshot: Option<WorkspaceStateSnapshot>) -> Result<u64, WorkspaceCoordError> {
    let snapshot = snapshot.ok_or(WorkspaceCoordError::Ineligible)?;
    if !snapshot.state.is_product_guarded()
        || !snapshot.state.bootstrap_complete
        || snapshot.state.needs_rebuild
        || snapshot
            .state
            .entries
            .iter()
            .any(|e| e.pending.is_some() || e.next.is_some())
    {
        return Err(WorkspaceCoordError::Ineligible);
    }
    Ok(snapshot.generation)
}

impl SyncCoordinator {
    /// Perform an authenticated policy check and account-wide server barrier.
    /// This does not activate the server's irreversible epoch policy. Older
    /// servers or an account without that policy fail closed. Product remains
    /// unavailable until every server instance enforces the policy.
    #[doc(hidden)]
    pub async fn prepare_v2_reconciliation_barrier(
        &self,
        session: Arc<Session>,
    ) -> Result<V2ReconciliationBarrier, WorkspaceCoordError> {
        let sync = self.clone();
        crate::v2_product_outbox::run_product_worker(move |cancelled, deadline| async move {
            let config = sync
                .capture_v2_config_epoch(&session)
                .ok_or(WorkspaceCoordError::Ineligible)?;
            let origin = session.instance_origin();
            let account = session.account_id().to_owned();
            let _protocol = sync.protocol.try_v2(&origin, &account)?;
            let marker = sync.protocol.try_v1()?;
            marker.ensure_product_outbox(&origin, &account)?;
            let root = sync
                .config_path
                .parent()
                .ok_or(WorkspaceCoordError::Ineligible)?;
            // Keep the same native source gate from before the barrier until
            // the reviewed local commit. This prevents cooperative emitters or
            // source writers from starting in the gap between those steps.
            let source_root = root.parent().ok_or(WorkspaceCoordError::Ineligible)?;
            let source_dir = source_root.join("plugin-sync-emission");
            let source_guard =
                crate::ProductEmissionFence::open(&source_dir)?.try_untracked_edit()?;
            let source_dir =
                std::fs::canonicalize(source_dir).map_err(|_| WorkspaceCoordError::Ineligible)?;
            let store = WorkspaceStore::open(root, &origin, &account)?;
            let _operation = store
                .try_operation_lock()?
                .ok_or(WorkspaceCoordError::Busy)?;
            let generation = ready(store.load()?)?;
            let eligible = || {
                crate::v2_product_outbox::product_eligible(&cancelled, deadline, || {
                    sync.is_v2_config_epoch_current(&config, &session)
                        && sync.auth.is_current_session(&session)
                })
            };
            if !eligible() {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let before = session.workspace_write_epoch().await?;
            if !eligible() || before.epoch_required() != Some(true) {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let session_guard = sync
                .auth
                .try_session_emission_permit(&session)
                .map_err(|_| WorkspaceCoordError::Ineligible)?;
            if cancelled.load(Ordering::Acquire)
                || Instant::now() >= deadline
                || !sync.auth.is_current_session(&session)
            {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let barrier = session.workspace_write_barrier().await?;
            drop(session_guard);
            if !eligible() || !barrier.is_after(&before)? || ready(store.load()?)? != generation {
                return Err(WorkspaceCoordError::Ineligible);
            }
            let expires = Instant::now() + REVIEW_LIFETIME;
            let source_guard = Arc::new(Mutex::new(Some(source_guard)));
            schedule_source_release(&source_guard, expires);
            Ok(V2ReconciliationBarrier {
                binding: sync.recovery_binding.clone(),
                session,
                config,
                workspace_generation: generation,
                expires,
                consumed: AtomicBool::new(false),
                source_guard,
                source_dir,
            })
        })
        .await
    }

    /// Invoke one short native commit under the captured session/consent and
    /// workspace fences. The proof owns the exclusive product source lease;
    /// caller must hold the exact plugin runtime/source fence. It must obtain an explicit
    /// user decision, verify native outboxes/journals and atomically revalidate
    /// the local marker in `operation`. The workspace check alone says nothing
    /// about pending SQLite candidates or legacy Cloud state.
    /// No network or asynchronous work may occur in the callback. Run it on a
    /// bounded blocking worker: native disk calls cannot be forcibly cancelled.
    /// A failed callback consumes the proof; retry requires a new barrier.
    #[doc(hidden)]
    pub fn with_v2_reconciliation_barrier<T>(
        &self,
        proof: &V2ReconciliationBarrier,
        operation: impl FnOnce(&V2ReconciliationAdmission<'_>) -> Result<T, WorkspaceCoordError>,
    ) -> Result<T, WorkspaceCoordError> {
        if !Arc::ptr_eq(&proof.binding, &self.recovery_binding)
            || Instant::now() >= proof.expires
            || proof.consumed.load(Ordering::Acquire)
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        let mut source_guard = SourceUse {
            lease: proof
                .source_guard
                .try_lock()
                .map_err(|_| WorkspaceCoordError::Busy)?,
            expires: proof.expires,
            consumed: &proof.consumed,
        };
        if source_guard.lease.is_none() {
            return Err(WorkspaceCoordError::Ineligible);
        }
        let session = &proof.session;
        let origin = session.instance_origin();
        let account = session.account_id();
        let _protocol = self.protocol.try_v2(&origin, account)?;
        let marker = self.protocol.try_v1()?;
        marker.ensure_product_outbox(&origin, account)?;
        let root = self
            .config_path
            .parent()
            .ok_or(WorkspaceCoordError::Ineligible)?;
        let store = WorkspaceStore::open(root, &origin, account)?;
        let _workspace = store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        let result = self
            .auth
            .with_current_session_permit(session, || {
                if Instant::now() >= proof.expires
                    || !self.is_v2_config_epoch_current(&proof.config, session)
                    || ready(store.load()?)? != proof.workspace_generation
                    || proof.consumed.swap(true, Ordering::AcqRel)
                {
                    return Err(WorkspaceCoordError::Ineligible);
                }
                operation(&V2ReconciliationAdmission {
                    context: V2LocalEditContext {
                        origin,
                        account_id: account.to_owned(),
                        config_generation: proof.config.config.generation,
                    },
                    source_dir: &proof.source_dir,
                })
            })
            .map_err(|_| WorkspaceCoordError::Ineligible)?;
        if proof.consumed.load(Ordering::Acquire) {
            source_guard.lease.take();
        }
        result
    }
}

fn schedule_source_release(
    guard: &Arc<Mutex<Option<crate::ProductEmissionLease>>>,
    expires: Instant,
) {
    let guard = Arc::downgrade(guard);
    tokio::spawn(async move {
        tokio::time::sleep_until(expires.into()).await;
        if let Some(guard) = guard.upgrade() {
            if let Ok(mut guard) = guard.try_lock() {
                guard.take();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn expired_reconciliation_releases_the_source_lease() {
        let dir = tempfile::tempdir().unwrap();
        let fence = crate::ProductEmissionFence::open(dir.path().join("source")).unwrap();
        let guard = Arc::new(Mutex::new(Some(fence.try_untracked_edit().unwrap())));
        schedule_source_release(&guard, Instant::now() + Duration::from_millis(20));
        assert!(fence.try_emission().is_err());
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if fence.try_emission().is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert!(guard.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn expiry_during_final_validation_releases_after_validation_returns() {
        let dir = tempfile::tempdir().unwrap();
        let fence = crate::ProductEmissionFence::open(dir.path().join("source")).unwrap();
        let guard = Arc::new(Mutex::new(Some(fence.try_untracked_edit().unwrap())));
        let expires = Instant::now() + Duration::from_millis(20);
        schedule_source_release(&guard, expires);
        let consumed = AtomicBool::new(false);
        let held = SourceUse {
            lease: guard.lock().unwrap(),
            expires,
            consumed: &consumed,
        };
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(fence.try_emission().is_err());
        drop(held);
        assert!(fence.try_emission().is_ok());
        assert!(guard.lock().unwrap().is_none());
    }
}
