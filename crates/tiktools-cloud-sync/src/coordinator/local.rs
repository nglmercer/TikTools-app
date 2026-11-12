//! Offline v1 staging and explicit legacy conflict decisions.
use crate::{PluginSelection, ProtocolFenceError};
use crate::{SyncCoordError, SyncCoordinator};
use cloud_client::{Edit, StoreError};

const MAX_STAGE_RETRIES: usize = 8;
const MAX_IDENTITY_LOCK_RETRIES: usize = 256;

impl SyncCoordinator {
    /// Stage a local install/enable change for upload. No network happens
    /// here, so this is safe offline; the edit waits in the store until the
    /// next [`SyncCoordinator::sync_now`]. Returns `false` (not staged) when
    /// sync is disabled or no account is signed in. Never fails the local
    /// plugin operation: invalid selections are reported, not applied.
    pub fn record_local_change(
        &self,
        plugin_id: &str,
        version: &str,
        enabled: bool,
    ) -> Result<bool, SyncCoordError> {
        let selection = PluginSelection {
            schema_version: 1,
            plugin_id: plugin_id.to_owned(),
            version: version.to_owned(),
            enabled,
        };
        self.stage_local(plugin_id, Edit::Put(selection))
    }

    /// Stage a local uninstall for upload (tombstone). Same offline and
    /// opt-in semantics as [`SyncCoordinator::record_local_change`].
    pub fn record_local_removal(&self, plugin_id: &str) -> Result<bool, SyncCoordError> {
        self.stage_local(plugin_id, Edit::Delete)
    }

    fn stage_local(&self, plugin_id: &str, edit: Edit) -> Result<bool, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let _admission = {
            let mut acquired = None;
            for attempt in 0..MAX_IDENTITY_LOCK_RETRIES {
                match lease.try_admission() {
                    Ok(guard) => {
                        acquired = Some(guard);
                        break;
                    }
                    Err(ProtocolFenceError::Busy) if attempt + 1 < MAX_IDENTITY_LOCK_RETRIES => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            acquired.ok_or(ProtocolFenceError::Busy)?
        };
        // The admission lease excludes cooperating consent changes. Read and
        // durably initialize its marker before taking the account mutex; the
        // product marker is still checked first inside the identity callback.
        let config = self.read_staging_config();
        for attempt in 0..MAX_IDENTITY_LOCK_RETRIES {
            match self.auth.with_sync_identity(|identity| {
                let Some((origin, account)) = identity else {
                    return Ok(false);
                };
                // Product mode fences every legacy staging attempt for this
                // identity, including disabled or damaged configuration. A
                // false opt-out must not masquerade as successful staging.
                lease.ensure_legacy_allowed(&origin, &account)?;
                let config = match &config {
                    Ok(config) => *config,
                    Err(error) => return Err(SyncCoordError::Io(error.to_string())),
                };
                if !config.enabled || !config.allows(&origin, &account) {
                    return Ok(false);
                }
                for _ in 0..MAX_STAGE_RETRIES {
                    let mut state = self.load_for_identity(&lease, &origin, &account)?;
                    state
                        .state
                        .stage(plugin_id.to_owned(), edit.clone())
                        .map_err(|_| {
                            SyncCoordError::Local(format!(
                                "invalid plugin selection for `{plugin_id}`"
                            ))
                        })?;
                    match self.save_state(state, &lease) {
                        Ok(_) => return Ok(true),
                        Err(SyncCoordError::Store(StoreError::ConcurrentChange)) => continue,
                        Err(error) => return Err(error),
                    }
                }
                Err(StoreError::ConcurrentChange.into())
            }) {
                Ok(result) => return result,
                Err(_) if attempt + 1 < MAX_IDENTITY_LOCK_RETRIES => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(_) => return Err(ProtocolFenceError::Busy.into()),
            }
        }
        Err(ProtocolFenceError::Busy.into())
    }

    /// Keep the local edit after reviewing a conflict; the next
    /// [`SyncCoordinator::sync_now`] uploads it over the remote record.
    pub fn keep_local(&self, key: &str) -> Result<(), SyncCoordError> {
        let lease = self.protocol.try_exclusive_v1()?;
        let mut state = self.load_for_session(&lease)?;
        lease.ensure_legacy_allowed(&state.state.origin, &state.state.account_id)?;
        state.state.rebase(key)?;
        self.save_state(state, &lease)?;
        Ok(())
    }

    /// Drop the local edit and accept the remote record. Returns the remote
    /// selection for the caller to apply locally (`None` = tombstone).
    pub fn take_remote(&self, key: &str) -> Result<Option<PluginSelection>, SyncCoordError> {
        let lease = self.protocol.try_exclusive_v1()?;
        let mut state = self.load_for_session(&lease)?;
        lease.ensure_legacy_allowed(&state.state.origin, &state.state.account_id)?;
        state.state.discard(key)?;
        let remote = state
            .state
            .entries
            .get(key)
            .and_then(|e| e.remote.clone())
            .and_then(|r| r.value.clone());
        self.save_state(state, &lease)?;
        Ok(remote)
    }
}
