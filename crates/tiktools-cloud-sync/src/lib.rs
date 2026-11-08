//! Optional multi-device sync of plugin selections through RemakeBackend.
//!
//! Reuses the pinned `cloud-client` sync engine (`plugin-selections`),
//! [`StateStore`](cloud_client::StateStore), and the native sync endpoints.
//! Sync is strictly opt-in and account-scoped: staging and syncing are
//! no-ops unless the user enabled sync **and** a Cloud session is live.
//! Remote records never install, enable, or execute anything by themselves;
//! the caller applies [`RemoteSelection`]s to already-installed plugins
//! through its own trusted local path, which must not re-stage them (that
//! is what prevents sync loops).

pub use cloud_client::{PluginSelection, SyncError, SyncState};

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use cloud_client::{Edit, StateStore, StoreError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tiktools_cloud_auth::CloudAuth;
use tokio::sync::Mutex;

const CONFIG_FILE: &str = "cloud-sync.json";
const STATE_FILE: &str = "sync-state.json";
const MAX_PULL_PAGES: usize = 200;

/// What [`SyncCoordinator::sync_now`] hands back for the caller to apply to
/// already-installed plugins. `selection` is `None` for a tombstone (the
/// remote side no longer selects this plugin): disable, never uninstall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSelection {
    pub key: String,
    pub selection: Option<PluginSelection>,
}

/// Everything the UI may show. No tokens, no plugin code, ever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub enabled: bool,
    pub signed_in: bool,
    pub syncing: bool,
    pub pending: usize,
    pub conflicts: Vec<String>,
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<u64>,
}

/// Outcome of one [`SyncCoordinator::sync_now`] run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncOutcome {
    pub pulled: usize,
    pub pushed: usize,
    pub conflicts: Vec<String>,
    pub remote: Vec<RemoteSelection>,
}

#[derive(Debug, Error)]
pub enum SyncCoordError {
    #[error("cloud sync is not enabled")]
    Disabled,
    #[error("cloud sync needs a signed-in account")]
    NotSignedIn,
    #[error("a sync run is already in progress")]
    AlreadyRunning,
    #[error("invalid local plugin selection: {0}")]
    Local(String),
    #[error(transparent)]
    Engine(#[from] SyncError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("cloud sync state is unreadable: {0}")]
    Io(String),
}

impl SyncCoordError {
    /// Stable short code for control responses; never carries secrets.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::NotSignedIn => "not-signed-in",
            Self::AlreadyRunning => "already-running",
            Self::Local(_) => "local",
            Self::Store(_) => "storage",
            Self::Io(_) => "storage",
            Self::Engine(error) => match error {
                SyncError::Network | SyncError::RateLimited { .. } => "network",
                SyncError::Unauthorized => "unauthorized",
                SyncError::NoPendingEdit => "no-pending-edit",
                _ => "engine",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct SyncConfig {
    enabled: bool,
}

struct Runtime {
    last_error: Option<String>,
    last_synced_at: Option<u64>,
}

/// Coordinates `plugin-selections` sync for one device. Cheap to clone;
/// runtime flags live behind one async lock, sync state on disk.
#[derive(Clone)]
pub struct SyncCoordinator {
    auth: CloudAuth,
    store: Arc<StateStore>,
    config_path: PathBuf,
    state_path: PathBuf,
    runtime: Arc<Mutex<Runtime>>,
    syncing: Arc<AtomicBool>,
}

/// Clears the `syncing` flag when dropped, including on cancellation.
struct RunningGuard<'a>(&'a AtomicBool);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl SyncCoordinator {
    /// `dir` holds the state store and the opt-in file. It must be private
    /// to the device; [`StateStore`] enforces that on open.
    pub fn new(auth: CloudAuth, dir: impl AsRef<Path>) -> Result<Self, SyncCoordError> {
        let dir = dir.as_ref().to_path_buf();
        let store = Arc::new(StateStore::open(&dir)?);
        Ok(Self {
            auth,
            store,
            config_path: dir.join(CONFIG_FILE),
            state_path: dir.join(STATE_FILE),
            runtime: Arc::new(Mutex::new(Runtime {
                last_error: None,
                last_synced_at: None,
            })),
            syncing: Arc::new(AtomicBool::new(false)),
        })
    }

    fn read_config(&self) -> SyncConfig {
        std::fs::read(&self.config_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(SyncConfig { enabled: false })
    }

    fn write_config(&self, config: &SyncConfig) -> Result<(), SyncCoordError> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SyncCoordError::Io(e.to_string()))?;
        }
        let bytes = serde_json::to_vec(config).map_err(|e| SyncCoordError::Io(e.to_string()))?;
        std::fs::write(&self.config_path, bytes).map_err(|e| SyncCoordError::Io(e.to_string()))
    }

    pub fn is_enabled(&self) -> bool {
        self.read_config().enabled
    }

    /// Flip the opt-in flag. Disabling stops staging and syncing but keeps
    /// the stored state, so re-enabling resumes where it left off.
    pub fn set_enabled(&self, enabled: bool) -> Result<SyncStatus, SyncCoordError> {
        self.write_config(&SyncConfig { enabled })?;
        Ok(self.status_blocking())
    }

    fn status_blocking(&self) -> SyncStatus {
        let snapshot = self.store.load().ok().flatten();
        let pending = snapshot
            .as_ref()
            .map(|s| {
                s.state
                    .entries
                    .values()
                    .filter(|e| e.pending.is_some())
                    .count()
            })
            .unwrap_or(0);
        let conflicts = snapshot
            .as_ref()
            .map(|s| {
                let mut keys: Vec<String> = s
                    .state
                    .entries
                    .iter()
                    .filter(|(_, e)| e.pending.as_ref().is_some_and(|p| p.conflict))
                    .map(|(k, _)| k.clone())
                    .collect();
                keys.sort();
                keys
            })
            .unwrap_or_default();
        // Non-blocking reads: status is called off hot paths only, and the
        // async [`Self::status`] refreshes these fields authoritatively.
        let runtime = self
            .runtime
            .try_lock()
            .map(|r| (r.last_error.clone(), r.last_synced_at));
        let (last_error, last_synced_at) = runtime.unwrap_or((None, None));
        SyncStatus {
            enabled: self.is_enabled(),
            signed_in: self.auth.try_session().is_some(),
            syncing: self.syncing.load(Ordering::SeqCst),
            pending,
            conflicts,
            last_error,
            last_synced_at,
        }
    }

    /// Nonsecret status for the UI. `signed_in` reflects the live session.
    pub async fn status(&self) -> SyncStatus {
        let mut status = self.status_blocking();
        status.signed_in = self.auth.session().await.is_some();
        status.syncing = self.syncing.load(Ordering::SeqCst);
        let runtime = self.runtime.lock().await;
        status.last_error = runtime.last_error.clone();
        status.last_synced_at = runtime.last_synced_at;
        status
    }

    /// Forget device sync state (logout, account switch, or support reset).
    /// Never touches the OS credential store or the server.
    pub fn reset(&self) {
        let _ = std::fs::remove_file(&self.state_path);
    }

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
        if !self.is_enabled() {
            return Ok(false);
        }
        let mut state = match self.load_for_session() {
            Ok(state) => state,
            Err(SyncCoordError::NotSignedIn) => return Ok(false),
            Err(error) => return Err(error),
        };
        let selection = PluginSelection {
            schema_version: 1,
            plugin_id: plugin_id.to_owned(),
            version: version.to_owned(),
            enabled,
        };
        state
            .state
            .stage(plugin_id.to_owned(), Edit::Put(selection))
            .map_err(|_| {
                SyncCoordError::Local(format!("invalid plugin selection for `{plugin_id}`"))
            })?;
        self.save_state(state)?;
        Ok(true)
    }

    /// Stage a local uninstall for upload (tombstone). Same offline and
    /// opt-in semantics as [`SyncCoordinator::record_local_change`].
    pub fn record_local_removal(&self, plugin_id: &str) -> Result<bool, SyncCoordError> {
        if !self.is_enabled() {
            return Ok(false);
        }
        let mut state = match self.load_for_session() {
            Ok(state) => state,
            Err(SyncCoordError::NotSignedIn) => return Ok(false),
            Err(error) => return Err(error),
        };
        state
            .state
            .stage(plugin_id.to_owned(), Edit::Delete)
            .map_err(|_| {
                SyncCoordError::Local(format!("invalid plugin selection for `{plugin_id}`"))
            })?;
        self.save_state(state)?;
        Ok(true)
    }

    /// Keep the local edit after reviewing a conflict; the next
    /// [`SyncCoordinator::sync_now`] uploads it over the remote record.
    pub fn keep_local(&self, key: &str) -> Result<(), SyncCoordError> {
        let mut state = self.load_for_session()?;
        state.state.rebase(key)?;
        self.save_state(state)?;
        Ok(())
    }

    /// Drop the local edit and accept the remote record. Returns the remote
    /// selection for the caller to apply locally (`None` = tombstone).
    pub fn take_remote(&self, key: &str) -> Result<Option<PluginSelection>, SyncCoordError> {
        let mut state = self.load_for_session()?;
        state.state.discard(key)?;
        let remote = state
            .state
            .entries
            .get(key)
            .and_then(|e| e.remote.clone())
            .and_then(|r| r.value.clone());
        self.save_state(state)?;
        Ok(remote)
    }

    /// Pull remote changes, push staged edits, persist, and report what the
    /// caller must apply to already-installed plugins. Expired cursors
    /// rebuild from a snapshot automatically; conflicts are collected (not
    /// fatal) for explicit resolution via [`SyncCoordinator::keep_local`]
    /// or [`SyncCoordinator::take_remote`].
    pub async fn sync_now(&self) -> Result<SyncOutcome, SyncCoordError> {
        if !self.is_enabled() {
            return Err(SyncCoordError::Disabled);
        }
        if !self
            .syncing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return Err(SyncCoordError::AlreadyRunning);
        }
        // Release the run flag even when the caller drops this future
        // mid-sync (the control plane enforces a per-request timeout);
        // otherwise every later run reports `AlreadyRunning` until restart.
        let running = RunningGuard(&self.syncing);
        let outcome = self.sync_inner().await;
        drop(running);
        {
            let mut runtime = self.runtime.lock().await;
            match &outcome {
                Ok(_) => {
                    runtime.last_error = None;
                    runtime.last_synced_at = Some(now_secs());
                }
                Err(error) => {
                    runtime.last_error = Some(error.to_string());
                }
            }
        }
        outcome
    }

    async fn sync_inner(&self) -> Result<SyncOutcome, SyncCoordError> {
        // A fresh process holds no in-memory session; reconnect from the
        // store first so one-shot callers (CLI, startup) sync without a
        // separate restore step. Failures fall through to NotSignedIn.
        if self.auth.session().await.is_none() {
            let _ = self.auth.restore().await;
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let mut snapshot = self.load_or_reset(&session).await?;
        let mut pulled = 0usize;
        // Pull until the server reports no more changes, rebuilding from a
        // snapshot when the cursor expired mid-stream.
        for _ in 0..MAX_PULL_PAGES {
            match session.pull_page(&mut snapshot.state).await {
                Ok(false) => break,
                Ok(true) => {
                    pulled += 1;
                    continue;
                }
                Err(SyncError::CursorExpired) => {
                    session.rebuild(&mut snapshot.state).await?;
                    continue;
                }
                Err(SyncError::WrongAccount) => {
                    // Signed in as someone else since the state was loaded.
                    snapshot = self.reset_to_session(&session).await?;
                    continue;
                }
                Err(error) => {
                    self.save_state(snapshot)?;
                    return Err(error.into());
                }
            }
        }
        // Push staged edits in key order for deterministic behavior.
        // Conflicts stay staged for explicit resolution; anything else
        // aborts after persisting progress so far.
        let mut pushed = 0usize;
        let keys: Vec<String> = snapshot
            .state
            .entries
            .iter()
            .filter(|(_, e)| e.pending.as_ref().is_some_and(|p| !p.conflict))
            .map(|(k, _)| k.clone())
            .collect();
        for key in keys {
            match session.push(&mut snapshot.state, &key).await {
                Ok(()) => pushed += 1,
                Err(SyncError::Conflict) => continue,
                Err(SyncError::WrongAccount) => {
                    self.save_state(snapshot)?;
                    return Err(SyncError::WrongAccount.into());
                }
                Err(error) => {
                    self.save_state(snapshot)?;
                    return Err(error.into());
                }
            }
        }
        let outcome = SyncOutcome {
            pulled,
            pushed,
            conflicts: snapshot
                .state
                .entries
                .iter()
                .filter(|(_, e)| e.pending.as_ref().is_some_and(|p| p.conflict))
                .map(|(k, _)| k.clone())
                .collect::<Vec<_>>(),
            remote: snapshot
                .state
                .entries
                .iter()
                .filter_map(|(key, entry)| {
                    entry.remote.as_ref().map(|record| RemoteSelection {
                        key: key.clone(),
                        selection: record.value.clone(),
                    })
                })
                .collect(),
        };
        self.save_state(snapshot)?;
        Ok(outcome)
    }

    /// Load stored state, or start fresh from the live session. A store file
    /// left by another account/origin is replaced, never reused: the server
    /// records for the old identity stay untouched on the server.
    async fn load_or_reset(
        &self,
        session: &cloud_client::Session,
    ) -> Result<cloud_client::StateSnapshot, SyncCoordError> {
        let snapshot = self.store.load()?;
        if snapshot.is_none() {
            return Ok(cloud_client::StateSnapshot {
                generation: 0,
                state: session.new_state(),
            });
        }
        let mut snapshot = snapshot.expect("checked above");
        // Probe identity through the engine: a cheap no-op pull fails with
        // WrongAccount when the file belongs to someone else. A fresh
        // cursor-0 state pulls everything, so only probe when work exists.
        let probe_needed = snapshot.state.cursor > 0 || !snapshot.state.entries.is_empty();
        if probe_needed {
            let mut probe = snapshot.state.clone();
            // A zero-change pull still verifies the account server-side.
            match session.pull_page(&mut probe).await {
                Ok(_) => {
                    snapshot.state = probe;
                    return Ok(snapshot);
                }
                Err(SyncError::WrongAccount) => {}
                Err(_) => return Ok(snapshot),
            }
            return self.reset_to_session(session).await;
        }
        Ok(snapshot)
    }

    async fn reset_to_session(
        &self,
        session: &cloud_client::Session,
    ) -> Result<cloud_client::StateSnapshot, SyncCoordError> {
        self.reset();
        Ok(cloud_client::StateSnapshot {
            generation: 0,
            state: session.new_state(),
        })
    }

    fn load_for_session(&self) -> Result<cloud_client::StateSnapshot, SyncCoordError> {
        // Prefer the live session identity; a store file left by another
        // account/origin is replaced, never staged into.
        if let Some(session) = self.auth.try_session() {
            let fresh = session.new_state();
            if let Some(snapshot) = self.store.load()? {
                if snapshot.state.origin == fresh.origin
                    && snapshot.state.account_id == fresh.account_id
                {
                    return Ok(snapshot);
                }
                self.reset();
            }
            return Ok(cloud_client::StateSnapshot {
                generation: 0,
                state: fresh,
            });
        }
        if let Some(snapshot) = self.store.load()? {
            return Ok(snapshot);
        }
        // No stored state: target the last known identity so offline staging
        // keeps working across restarts. The engine re-validates on use.
        if let Some((origin, account_id)) = self.auth.last_identity() {
            return Ok(cloud_client::StateSnapshot {
                generation: 0,
                state: SyncState {
                    origin,
                    account_id,
                    cursor: 0,
                    entries: Default::default(),
                },
            });
        }
        Err(SyncCoordError::NotSignedIn)
    }

    fn save_state(&self, snapshot: cloud_client::StateSnapshot) -> Result<(), SyncCoordError> {
        let generation = if snapshot.generation == 0 {
            None
        } else {
            Some(snapshot.generation)
        };
        self.store.save(generation, &snapshot.state)?;
        Ok(())
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod sync_tests;
