//! Identity-scoped snapshots, CAS saves and checked legacy resets.
use crate::{protocol_fence, SyncState};
use crate::{SyncCoordError, SyncCoordinator};
use cloud_client::StoreError;
use std::sync::Arc;

const STATE_FILE: &str = "sync-state.json";

impl SyncCoordinator {
    /// Forget device sync state (logout, account switch, or support reset).
    /// Never touches the OS credential store or the server.
    pub fn reset(&self) {
        let _ = self.try_reset();
    }

    /// Checked form for product callers; reports an active run, a selected v2
    /// protocol or uncertain upload instead of silently discarding work.
    pub fn try_reset(&self) -> Result<(), SyncCoordError> {
        let lease = self.protocol.try_exclusive_v1()?;
        self.reset_locked(&lease)
    }

    pub(crate) fn reset_locked(
        &self,
        lease: &protocol_fence::V1Lease,
    ) -> Result<(), SyncCoordError> {
        let (origin, account) = self
            .current_identity()?
            .ok_or(SyncCoordError::NotSignedIn)?;
        lease.ensure_v1(&origin, &account)?;
        // A staged product receipt can be the only durable link to a pending
        // JSON edit after its outbox entry has been acknowledged. The legacy
        // reset cannot reconcile those stores and must not remove that state.
        lease.ensure_legacy_allowed(&origin, &account)?;
        lease.ensure_no_uncertain(&origin, &account)?;
        let (store, dir) = lease.legacy_store(&origin, &account)?;
        if let Some(snapshot) = store.load()? {
            if snapshot.state.origin != origin || snapshot.state.account_id != account {
                return Err(SyncCoordError::Store(StoreError::WrongIdentity));
            }
        }
        match std::fs::remove_file(dir.join(STATE_FILE)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(SyncCoordError::Io(error.to_string())),
        }
    }

    /// Load this identity's legacy state without replacing another account.
    pub(crate) fn load_or_reset(
        &self,
        session: &Arc<cloud_client::Session>,
        lease: &protocol_fence::V1Lease,
    ) -> Result<cloud_client::StateSnapshot, SyncCoordError> {
        let fresh = session.new_state();
        let (store, _) = lease.legacy_store(&fresh.origin, &fresh.account_id)?;
        let snapshot = store.load()?;
        if snapshot.is_none() {
            return Ok(cloud_client::StateSnapshot {
                generation: 0,
                state: fresh,
            });
        }
        let snapshot = snapshot.expect("checked above");
        if snapshot.state.origin != fresh.origin || snapshot.state.account_id != fresh.account_id {
            return Err(SyncCoordError::Store(StoreError::WrongIdentity));
        }
        // The actual guarded pull immediately following this load verifies
        // the account. A separate probe would consume changes (and can hide a
        // one-shot cursor-expiry response) before the sync loop sees them.
        Ok(snapshot)
    }

    pub(crate) fn load_for_session(
        &self,
        lease: &protocol_fence::V1Lease,
    ) -> Result<cloud_client::StateSnapshot, SyncCoordError> {
        if let Some((origin, account_id)) = self.current_identity()? {
            return self.load_for_identity(lease, &origin, &account_id);
        }
        Err(SyncCoordError::NotSignedIn)
    }

    pub(crate) fn load_for_identity(
        &self,
        lease: &protocol_fence::V1Lease,
        origin: &str,
        account: &str,
    ) -> Result<cloud_client::StateSnapshot, SyncCoordError> {
        lease.ensure_v1(origin, account)?;
        let (store, _) = lease.legacy_store(origin, account)?;
        if let Some(snapshot) = store.load()? {
            if snapshot.state.origin != origin || snapshot.state.account_id != account {
                return Err(SyncCoordError::Store(StoreError::WrongIdentity));
            }
            return Ok(snapshot);
        }
        Ok(cloud_client::StateSnapshot {
            generation: 0,
            state: SyncState::new(origin.into(), account.into()),
        })
    }

    pub(crate) fn save_state(
        &self,
        snapshot: cloud_client::StateSnapshot,
        lease: &protocol_fence::V1Lease,
    ) -> Result<cloud_client::StateSnapshot, SyncCoordError> {
        let generation = if snapshot.generation == 0 {
            None
        } else {
            Some(snapshot.generation)
        };
        let (store, _) = lease.legacy_store(&snapshot.state.origin, &snapshot.state.account_id)?;
        Ok(store.save(generation, &snapshot.state)?)
    }
}
