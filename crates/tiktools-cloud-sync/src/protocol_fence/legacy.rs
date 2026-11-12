//! Account-partitioned legacy snapshots without replacing another identity.
use super::ProtocolFenceError;
use super::{identity_hash, sync_dir, ProtocolFence};
use cloud_client::{StateSnapshot, StateStore, StoreError};
use std::{fs, io, path::PathBuf};

impl ProtocolFence {
    /// Existing legacy bytes remain at their original path. A different
    /// identity gets a separate private partition rather than replacing them.
    pub(super) fn legacy_store(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<(StateStore, PathBuf), ProtocolFenceError> {
        let hash = identity_hash(origin, account)?;
        let root = StateStore::open(&self.dir)?;
        let namespace = self.dir.join("legacy-v1");
        let partition = namespace.join(hash);
        let namespace_exists = match fs::symlink_metadata(&namespace) {
            Ok(meta) if meta.file_type().is_dir() => {
                StateStore::open(&namespace)?;
                true
            }
            Ok(_) => return Err(StoreError::UnsafeDirectory.into()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if namespace_exists {
            match fs::symlink_metadata(&partition) {
                Ok(meta) if meta.file_type().is_dir() => {
                    return Ok((StateStore::open(&partition)?, partition))
                }
                Ok(_) => return Err(StoreError::UnsafeDirectory.into()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        if root.load()?.is_some_and(|snapshot| {
            snapshot.state.origin != origin || snapshot.state.account_id != account
        }) {
            StateStore::open(&namespace)?;
            let store = StateStore::open(&partition)?;
            sync_dir(&namespace)?;
            sync_dir(&self.dir)?;
            return Ok((store, partition));
        }
        Ok((root, self.dir.clone()))
    }

    /// Captures the legacy state for a preview without exposing another
    /// account's partition. The product must still recheck under cutover.
    pub fn legacy_snapshot(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<Option<StateSnapshot>, ProtocolFenceError> {
        let _lease = self.try_v1()?;
        let (store, _) = self.legacy_store(origin, account)?;
        Ok(store.load()?)
    }
}
