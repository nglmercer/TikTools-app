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

pub use cloud_client::{
    Edit as V1Edit, Entry as V1Entry, Pending as V1Pending, PluginSelection, Record as V1Record,
    SyncError, SyncState,
};
mod workspace;
pub use workspace::{
    WorkspaceCoordError, WorkspaceCoordinator, WorkspaceEligibility, WorkspaceOutcome,
    WorkspaceReview,
};
mod cutover_preview;
pub use cutover_preview::CutoverPreview;
mod cutover_receipt;
pub use cutover_receipt::V1CutoverReceipt;
mod cutover_execution;
pub use cutover_execution::{CutoverExecution, CutoverSource, GuardedCutoverReview};
mod protocol_fence;
pub use protocol_fence::{
    CutoverLease, ProtocolFence, ProtocolFenceError, V1UncertainUpload, V2Lease,
};
mod v1_recovery;
pub use v1_recovery::{V1RecoveryChoice, V1RecoveryOutcome, V1UncertainReview};
mod local_edit;
pub use local_edit::V1LocalEditContext;
mod v2_product_outbox;
pub use v2_product_outbox::{V2ProductDecision, V2ProductSource};
mod product_outbox;
pub use product_outbox::{V1ConflictSourceProof, V1OutboxCandidate, V1ProductOutbox};
mod product_remote_apply;
pub use product_remote_apply::V1RemoteApplyReview;
#[doc(hidden)]
pub mod consent_generation;
mod product_conflict;
#[allow(dead_code)] // Native review is not exposed until product integration.
mod v2_consent;
pub use product_conflict::{
    V1KeepLocalAbandonReview, V1KeepLocalIntent, V1KeepLocalRecoveryReview,
    V1PreviousConsentRebaseReview, V1ProductConflictReview, V1SupersededRebaseReview,
};

use std::{
    collections::BTreeSet,
    future::Future,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use cloud_client::{Edit, StoreError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tiktools_cloud_auth::CloudAuth;
use tokio::sync::Mutex;

const CONFIG_FILE: &str = "cloud-sync.json";
const CONSENT_MARKER_FILE: &str = "cloud-sync-consent-initialized";
const CONSENT_MARKER: &[u8] = b"tiktools-sync-consent-initialized-v1\n";
const STATE_FILE: &str = "sync-state.json";
const MAX_PULL_PAGES: usize = 200;
const MAX_STAGE_RETRIES: usize = 8;
const MAX_IDENTITY_LOCK_RETRIES: usize = 256;

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
    /// Feed and snapshot records consumed in this run, including the final
    /// page and any records read while rebuilding after cursor expiry.
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
    #[error(transparent)]
    Protocol(#[from] ProtocolFenceError),
    #[error("the uncertain write is no longer available")]
    NoUncertainWrite,
    #[error("the sync review changed; review the current values again")]
    StaleReview,
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
            Self::Protocol(ProtocolFenceError::Busy) => "already-running",
            Self::Protocol(ProtocolFenceError::Io(_) | ProtocolFenceError::Store(_)) => "storage",
            Self::Protocol(_) => "protocol",
            Self::NoUncertainWrite => "no-uncertain-write",
            Self::StaleReview => "stale-review",
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
    #[serde(
        default = "legacy_config_version",
        skip_serializing_if = "is_legacy_config_version"
    )]
    version: u8,
    enabled: bool,
    #[serde(default, with = "consent_generation")]
    generation: u128,
    /// New v2 opt-ins are private to one origin/account. Legacy settings
    /// omit this field and retain their existing device-wide semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope: Option<[u8; 32]>,
}

fn legacy_config_version() -> u8 {
    1
}
fn is_legacy_config_version(version: &u8) -> bool {
    *version == 1
}

fn read_regular_file(
    path: &Path,
    max: usize,
) -> Result<(Vec<u8>, std::fs::Metadata), SyncCoordError> {
    use std::io::Read;
    let invalid = || SyncCoordError::Io("sync preference file is unreadable".into());
    let before = std::fs::symlink_metadata(path).map_err(|_| invalid())?;
    if !before.file_type().is_file() {
        return Err(invalid());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|_| invalid())?;
    let opened = file.metadata().map_err(|_| invalid())?;
    if !opened.file_type().is_file() || opened.len() > max as u64 {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() > max {
        return Err(invalid());
    }
    Ok((bytes, opened))
}

fn consent_scope(origin: &str, account: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let material = serde_json::to_vec(&("tiktools-v2-consent", origin, account))
        .expect("serializing strings cannot fail");
    Sha256::digest(material).into()
}

impl SyncConfig {
    fn valid(&self) -> bool {
        matches!((self.version, self.scope), (1, None) | (2, Some(_)))
    }

    fn allows(&self, origin: &str, account: &str) -> bool {
        self.scope
            .is_none_or(|scope| scope == consent_scope(origin, account))
    }

    fn allows_session(&self, session: &cloud_client::Session) -> bool {
        self.allows(&session.instance_origin(), session.account_id())
    }
}

/// Opaque opt-in snapshot bound to one coordinator and exact login session.
/// It only supports revalidation; it is not a lease or authority to apply data.
#[derive(Clone)]
pub struct SyncConfigEpoch {
    binding: Arc<()>,
    session: Arc<cloud_client::Session>,
    config: SyncConfig,
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
    protocol: ProtocolFence,
    config_path: PathBuf,
    runtime: Arc<Mutex<Runtime>>,
    syncing: Arc<AtomicBool>,
    recovery_binding: Arc<()>,
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
    /// to the device; [`cloud_client::StateStore`] enforces that on open.
    pub fn new(auth: CloudAuth, dir: impl AsRef<Path>) -> Result<Self, SyncCoordError> {
        let dir = dir.as_ref().to_path_buf();
        let protocol = ProtocolFence::open(&dir)?;
        let coordinator = Self {
            auth,
            protocol,
            config_path: dir.join(CONFIG_FILE),
            runtime: Arc::new(Mutex::new(Runtime {
                last_error: None,
                last_synced_at: None,
            })),
            syncing: Arc::new(AtomicBool::new(false)),
            recovery_binding: Arc::new(()),
        };
        // Upgraded installations may already have a preference file. Record
        // that fact durably before admitting operations, so a later loss of
        // the file cannot be mistaken for a never-configured installation.
        if !coordinator.consent_marker_present()?
            && match std::fs::symlink_metadata(&coordinator.config_path) {
                Ok(_) => true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => return Err(SyncCoordError::Io(error.to_string())),
            }
        {
            let lease = coordinator.protocol.try_v1()?;
            let _admission = lease.try_admission()?;
            coordinator.ensure_consent_marker()?;
        }
        Ok(coordinator)
    }

    fn consent_marker_path(&self) -> PathBuf {
        self.config_path.with_file_name(CONSENT_MARKER_FILE)
    }

    fn consent_marker_present(&self) -> Result<bool, SyncCoordError> {
        let path = self.consent_marker_path();
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(SyncCoordError::Io(error.to_string())),
        };
        if !metadata.file_type().is_file() || metadata.len() != CONSENT_MARKER.len() as u64 {
            return Err(SyncCoordError::Io("invalid sync consent marker".into()));
        }
        let (bytes, opened) = read_regular_file(&path, CONSENT_MARKER.len())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if opened.permissions().mode() & 0o077 != 0 {
                return Err(SyncCoordError::Io("invalid sync consent marker".into()));
            }
        }
        #[cfg(not(unix))]
        let _ = opened;
        if bytes != CONSENT_MARKER {
            return Err(SyncCoordError::Io("invalid sync consent marker".into()));
        }
        Ok(true)
    }

    /// Caller holds the exclusive protocol lease, or shared protocol plus
    /// admission. The marker is installed and directory-synced before a
    /// preference is written or a legacy preference is used for mutation.
    fn ensure_consent_marker(&self) -> Result<(), SyncCoordError> {
        let dir = self
            .config_path
            .parent()
            .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
        if !self.consent_marker_present()? {
            let mut temp = tempfile::NamedTempFile::new_in(dir)
                .map_err(|error| SyncCoordError::Io(error.to_string()))?;
            use std::io::Write;
            temp.write_all(CONSENT_MARKER)
                .map_err(|error| SyncCoordError::Io(error.to_string()))?;
            temp.as_file()
                .sync_all()
                .map_err(|error| SyncCoordError::Io(error.to_string()))?;
            if let Err(error) = temp.persist_noclobber(self.consent_marker_path()) {
                if error.error.kind() != std::io::ErrorKind::AlreadyExists
                    || !self.consent_marker_present()?
                {
                    return Err(SyncCoordError::Io(error.error.to_string()));
                }
            }
        }
        #[cfg(unix)]
        std::fs::File::open(dir)
            .and_then(|dir| dir.sync_all())
            .map_err(|error| SyncCoordError::Io(error.to_string()))?;
        Ok(())
    }

    /// Read only a bounded regular file. On Unix the open itself rejects
    /// symlinks and is nonblocking, including if a FIFO replaces the file.
    fn read_config_bytes(&self) -> Result<Vec<u8>, SyncCoordError> {
        Ok(read_regular_file(&self.config_path, 4096)?.0)
    }

    fn read_config(&self) -> SyncConfig {
        let mut config = self
            .read_config_bytes()
            .ok()
            .and_then(|bytes| serde_json::from_slice::<SyncConfig>(&bytes).ok())
            .filter(SyncConfig::valid)
            .unwrap_or(SyncConfig {
                version: 1,
                enabled: false,
                generation: 0,
                scope: None,
            });
        // Files predating consent generations do not establish an identity.
        // Keep their state intact, but require explicit opt-in before sync.
        if config.generation == 0 {
            config.enabled = false;
        }
        config
    }

    fn write_config(&self, config: &SyncConfig) -> Result<(), SyncCoordError> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SyncCoordError::Io(e.to_string()))?;
        }
        let bytes = serde_json::to_vec(config).map_err(|e| SyncCoordError::Io(e.to_string()))?;
        let dir = self
            .config_path
            .parent()
            .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
        let mut temp =
            tempfile::NamedTempFile::new_in(dir).map_err(|e| SyncCoordError::Io(e.to_string()))?;
        use std::io::Write;
        temp.write_all(&bytes)
            .map_err(|e| SyncCoordError::Io(e.to_string()))?;
        temp.as_file()
            .sync_all()
            .map_err(|e| SyncCoordError::Io(e.to_string()))?;
        temp.persist(&self.config_path)
            .map_err(|e| SyncCoordError::Io(e.error.to_string()))?;
        #[cfg(unix)]
        std::fs::File::open(dir)
            .and_then(|dir| dir.sync_all())
            .map_err(|e| SyncCoordError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        let config = self.read_config();
        config.enabled
            && (config.scope.is_none()
                || self
                    .current_identity()
                    .ok()
                    .flatten()
                    .is_some_and(|(origin, account)| config.allows(&origin, &account)))
    }

    /// Capture opt-in only for an exact current session. A later guarded commit
    /// must recheck both this epoch and the live account session.
    pub fn capture_config_epoch(
        &self,
        session: &Arc<cloud_client::Session>,
    ) -> Option<SyncConfigEpoch> {
        let config = self.read_config();
        (config.enabled && config.allows_session(session) && self.auth.is_current_session(session))
            .then(|| SyncConfigEpoch {
                binding: self.recovery_binding.clone(),
                session: session.clone(),
                config,
            })
    }

    /// Compare only the captured inputs. Call within `CloudAuth::with_current_session`
    /// so account logout or replacement cannot race the local observation.
    pub fn is_config_epoch_current(
        &self,
        epoch: &SyncConfigEpoch,
        session: &Arc<cloud_client::Session>,
    ) -> bool {
        Arc::ptr_eq(&epoch.binding, &self.recovery_binding)
            && Arc::ptr_eq(&epoch.session, session)
            && epoch.config.enabled
            && epoch.config.allows_session(session)
            && self.read_config() == epoch.config
    }

    /// Capture consent specifically bound to this v2 origin/account. Legacy
    /// device-wide v1 opt-in is never enough to authorize v2 networking.
    /// Callers must additionally hold and recheck the selected v2 protocol.
    pub fn capture_v2_config_epoch(
        &self,
        session: &Arc<cloud_client::Session>,
    ) -> Option<SyncConfigEpoch> {
        self.capture_config_epoch(session)
            .filter(|epoch| epoch.config.version == 2 && epoch.config.scope.is_some())
    }

    /// Call within the same current-session guard as `is_config_epoch_current`.
    pub fn is_v2_config_epoch_current(
        &self,
        epoch: &SyncConfigEpoch,
        session: &Arc<cloud_client::Session>,
    ) -> bool {
        epoch.config.version == 2
            && epoch.config.scope.is_some()
            && self.is_config_epoch_current(epoch, session)
    }

    /// Build an informational v2 migration candidate from the exact current
    /// v1 snapshot and the supplied local selections. This does not authorize
    /// or start a cutover: the product must capture and recheck local source,
    /// account and consent under its cutover fence before accepting a review.
    pub fn preview_v2_cutover(
        &self,
        session: &Arc<cloud_client::Session>,
        selections: &[PluginSelection],
    ) -> Result<CutoverPreview, SyncCoordError> {
        self.preview_v2_cutover_with_snapshot_hook(session, selections, || {})
    }

    fn preview_v2_cutover_with_snapshot_hook(
        &self,
        session: &Arc<cloud_client::Session>,
        selections: &[PluginSelection],
        after_snapshot: impl FnOnce(),
    ) -> Result<CutoverPreview, SyncCoordError> {
        self.preview_v2_cutover_inner(session, selections, None, after_snapshot)
    }

    fn preview_v2_cutover_inner(
        &self,
        session: &Arc<cloud_client::Session>,
        selections: &[PluginSelection],
        receipts: Option<&[V1CutoverReceipt]>,
        after_snapshot: impl FnOnce(),
    ) -> Result<CutoverPreview, SyncCoordError> {
        let epoch = self
            .capture_config_epoch(session)
            .ok_or(SyncCoordError::StaleReview)?;
        let origin = session.instance_origin();
        let account = session.account_id().to_owned();
        let snapshot = || -> Result<_, SyncCoordError> {
            let lease = self.protocol.try_v1()?;
            let _admission = lease.try_admission()?;
            lease.ensure_v1(&origin, &account)?;
            // These durable journals can contain work not represented by the
            // v1 snapshot and prevent a complete migration candidate.
            lease.ensure_outbox_mode(&origin, &account, receipts.is_some())?;
            lease.ensure_no_uncertain(&origin, &account)?;
            self.auth
                .with_current_session_permit(session, || {
                    if !self.is_config_epoch_current(&epoch, session) {
                        return Err(SyncCoordError::StaleReview);
                    }
                    Ok(lease.legacy_store(&origin, &account)?.0.load()?)
                })
                .map_err(|_| SyncCoordError::StaleReview)?
        };
        let legacy = snapshot()?;
        after_snapshot();
        // Validation can be expensive for large candidates. Do not block
        // logout, local admission, or protocol cutover while it runs.
        let preview = match receipts {
            None => {
                CutoverPreview::new(origin.clone(), account.clone(), selections, legacy.as_ref())
            }
            Some(receipts) => CutoverPreview::new_marked(
                origin.clone(),
                account.clone(),
                selections,
                legacy.as_ref(),
                receipts,
            ),
        }
        .map_err(|error| SyncCoordError::Local(error.to_string()))?;
        let current = snapshot()?;
        preview
            .verify_inputs(&origin, &account, selections, current.as_ref())
            .map_err(|_| SyncCoordError::StaleReview)?;
        Ok(preview)
    }

    /// Flip the opt-in flag. Disabling stops staging and syncing but keeps
    /// the stored state. Enabling currently supports the v1 product path;
    /// enabling an account already switched to v2 requires product wiring.
    pub fn set_enabled(&self, enabled: bool) -> Result<SyncStatus, SyncCoordError> {
        let lease = self.protocol.try_exclusive_v1()?;
        if enabled {
            if let Some((origin, account)) = self.current_identity()? {
                lease.ensure_v1(&origin, &account)?;
            }
        }
        self.write_new_consent(enabled, self.read_config().generation, None)?;
        drop(lease);
        Ok(self.status_blocking())
    }

    /// The caller must hold exclusive protocol admission and any required
    /// current-session fence. Every explicit decision gets a new identity.
    fn write_new_consent(
        &self,
        enabled: bool,
        previous: u128,
        scope: Option<[u8; 32]>,
    ) -> Result<(), SyncCoordError> {
        self.ensure_consent_marker()?;
        // The high bit keeps new identities outside legacy u64 generations.
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes)
            .map_err(|_| SyncCoordError::Io("cannot create sync consent identity".into()))?;
        bytes[0] |= 0x80;
        let generation = u128::from_be_bytes(bytes);
        if generation == previous {
            return Err(SyncCoordError::Io("sync consent identity collision".into()));
        }
        self.write_config(&SyncConfig {
            version: if scope.is_some() { 2 } else { 1 },
            enabled,
            generation,
            scope,
        })
    }

    fn status_blocking(&self) -> SyncStatus {
        let snapshot = match self.current_identity() {
            Ok(Some((origin, account))) => self
                .protocol
                .legacy_snapshot(&origin, &account)
                .ok()
                .flatten(),
            Ok(None) => None,
            Err(_) => None,
        };
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
        let _ = self.try_reset();
    }

    /// Checked form for product callers; reports an active run, a selected v2
    /// protocol or uncertain upload instead of silently discarding work.
    pub fn try_reset(&self) -> Result<(), SyncCoordError> {
        let lease = self.protocol.try_exclusive_v1()?;
        self.reset_locked(&lease)
    }

    fn reset_locked(&self, lease: &protocol_fence::V1Lease) -> Result<(), SyncCoordError> {
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

    /// Pull remote changes, push staged edits, persist, and report what the
    /// caller must apply to already-installed plugins. Expired cursors
    /// rebuild from a snapshot automatically; conflicts are collected (not
    /// fatal) for explicit resolution via [`SyncCoordinator::keep_local`]
    /// or [`SyncCoordinator::take_remote`].
    pub async fn sync_now(&self) -> Result<SyncOutcome, SyncCoordError> {
        // Compatibility entry point for applications without a product
        // outbox. A product that records candidates in a separate durable
        // store must use `sync_now_with_outbox` instead.
        self.sync_now_inner(&mut product_outbox::NoProductOutbox, false)
            .await
    }

    /// Sync with a product-owned durable outbox. Every cooperating local
    /// mutation must use `with_v1_local_edit`; direct callers of `sync_now`
    /// cannot provide the product outbox's cross-store safety guarantee.
    /// This low-level compatibility entry point does not require a migration
    /// marker or a runtime guard and rejects migrated accounts; product integrations must use
    /// `sync_now_with_marked_outbox` after explicit source migration instead.
    pub async fn sync_now_with_outbox(
        &self,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<SyncOutcome, SyncCoordError> {
        self.sync_now_inner(outbox, false).await
    }

    /// Product drain for a previously reviewed v1 migration. The durable
    /// per-account marker must already fence cooperating legacy writers, and
    /// the native outbox must supply a runtime gate. This method never creates
    /// the marker or authorizes migration on its own. Older binaries and
    /// direct filesystem writers remain outside the cooperative protocol.
    pub async fn sync_now_with_marked_outbox(
        &self,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<SyncOutcome, SyncCoordError> {
        self.sync_now_inner(outbox, true).await
    }

    async fn sync_now_inner(
        &self,
        outbox: &mut dyn V1ProductOutbox,
        require_product_marker: bool,
    ) -> Result<SyncOutcome, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let config = self.read_config();
        if !config.enabled {
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
        let _network = lease.try_network()?;
        if self.read_config() != config {
            return Err(SyncCoordError::Disabled);
        }
        let outcome = self
            .sync_inner(&lease, config, outbox, require_product_marker)
            .await;
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

    async fn sync_inner(
        &self,
        lease: &protocol_fence::V1Lease,
        config: SyncConfig,
        outbox: &mut dyn V1ProductOutbox,
        require_product_marker: bool,
    ) -> Result<SyncOutcome, SyncCoordError> {
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
        let _marker_admission = lease.try_admission()?;
        self.with_live_session(&session, config, || {
            lease.ensure_v1(&session.instance_origin(), session.account_id())?;
            lease.ensure_outbox_mode(
                &session.instance_origin(),
                session.account_id(),
                require_product_marker,
            )?;
            lease.ensure_no_uncertain(&session.instance_origin(), session.account_id())?;
            Ok(())
        })?;
        drop(_marker_admission);
        let mut snapshot = self.load_or_reset(&session, lease)?;
        let mut pulled = 0usize;
        // Pull until the server reports no more changes, rebuilding from a
        // snapshot when the cursor expired mid-stream.
        let mut complete = false;
        for _ in 0..MAX_PULL_PAGES {
            match self
                .guarded_sync_io(
                    &session,
                    config,
                    session.pull_page_with_count(&mut snapshot.state),
                )
                .await
            {
                Ok((more, count)) => {
                    pulled = pulled.checked_add(count).ok_or(SyncError::Protocol)?;
                    if !more {
                        complete = true;
                        break;
                    }
                }
                Err(SyncCoordError::Engine(SyncError::CursorExpired)) => {
                    let count = self
                        .guarded_sync_io(
                            &session,
                            config,
                            session.rebuild_with_count(&mut snapshot.state),
                        )
                        .await?;
                    pulled = pulled.checked_add(count).ok_or(SyncError::Protocol)?;
                    // `rebuild_with_count` already pulls its own follow-up
                    // changes until the server says the feed is complete.
                    complete = true;
                    break;
                }
                Err(SyncCoordError::Engine(SyncError::WrongAccount)) => {
                    // Never delete another identity's durable state.
                    return Err(SyncError::WrongAccount.into());
                }
                Err(error) => {
                    if matches!(error, SyncCoordError::StaleReview) {
                        return Err(error);
                    }
                    self.save_product_checkpoint(
                        lease,
                        &session,
                        config,
                        snapshot,
                        outbox,
                        None,
                        require_product_marker,
                    )?;
                    return Err(error);
                }
            }
        }
        if !complete {
            // Preserve safely validated pages so a backlog larger than the
            // per-run bound can make progress without admitting any upload.
            self.save_product_checkpoint(
                lease,
                &session,
                config,
                snapshot,
                outbox,
                None,
                require_product_marker,
            )?;
            return Err(SyncError::TooManyPages.into());
        }
        // Push staged edits in key order for deterministic behavior.
        // Conflicts stay staged for explicit resolution; anything else
        // aborts after persisting progress so far.
        let mut pushed = 0usize;
        let mut attempted = BTreeSet::new();
        loop {
            let (updated, admitted) = if require_product_marker {
                let mut input = Some(snapshot);
                let mut output = None;
                let mut operation = |guarded: &mut dyn V1ProductOutbox| {
                    let snapshot = input.take().ok_or_else(|| {
                        SyncCoordError::Local("runtime admission was repeated".into())
                    })?;
                    output = Some(self.admit_product_upload(
                        lease, &session, config, snapshot, guarded, &attempted, true,
                    )?);
                    Ok(())
                };
                outbox.with_runtime_guard(&mut operation)?;
                drop(operation);
                output.ok_or_else(|| {
                    SyncCoordError::Local("runtime admission was not performed".into())
                })?
            } else {
                self.admit_product_upload(
                    lease, &session, config, snapshot, outbox, &attempted, false,
                )?
            };
            snapshot = updated;
            let Some((key, marker)) = admitted else { break };
            attempted.insert(key.clone());
            match self
                .guarded_sync_io(&session, config, session.push(&mut snapshot.state, &key))
                .await
            {
                Ok(()) => pushed += 1,
                Err(SyncCoordError::Engine(SyncError::Conflict)) => {}
                Err(SyncCoordError::Engine(SyncError::WrongAccount)) => {
                    return Err(SyncError::WrongAccount.into());
                }
                Err(error) => {
                    // Keep the persisted source generation unchanged so an
                    // explicit review can distinguish the original request
                    // from any newer local decision. Pulled cache is refetched.
                    return Err(error);
                }
            }
            // Do not erase the original candidate before the known outcome is
            // durably saved. A concurrent local stage may win the CAS; in that
            // case its newer value survives and this marker remains for review.
            snapshot = self.save_product_checkpoint(
                lease,
                &session,
                config,
                snapshot,
                outbox,
                Some(marker),
                require_product_marker,
            )?;
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
                // A remote value is not a local decision while an edit is
                // pending. In particular, conflicted values must wait for
                // keep_local/take_remote instead of changing a running plugin.
                .filter(|(_, entry)| entry.pending.is_none())
                .filter_map(|(key, entry)| {
                    entry.remote.as_ref().map(|record| RemoteSelection {
                        key: key.clone(),
                        selection: record.value.clone(),
                    })
                })
                .collect(),
        };
        self.save_product_checkpoint(
            lease,
            &session,
            config,
            snapshot,
            outbox,
            None,
            require_product_marker,
        )?;
        Ok(outcome)
    }

    /// Load this identity's legacy state without replacing another account.
    fn load_or_reset(
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

    /// Run a bounded synchronous state transition while the exact session is
    /// installed. The protocol lease held by sync separately protects opt-in.
    fn with_live_session<R>(
        &self,
        session: &Arc<cloud_client::Session>,
        config: SyncConfig,
        operation: impl FnOnce() -> Result<R, SyncCoordError>,
    ) -> Result<R, SyncCoordError> {
        self.auth
            .with_current_session(session, || {
                if self.read_config() != config || !config.allows_session(session) {
                    return Err(SyncCoordError::StaleReview);
                }
                operation()
            })
            .map_err(|_| SyncCoordError::StaleReview)?
    }

    /// Revalidate immediately before and after network work. There remains a
    /// dispatch race between the first check and polling the future; a proper
    /// revocable request gate is required for strict cancellation. Writes must
    /// retain their uncertain marker whenever a reply cannot be committed.
    async fn guarded_sync_io<F, T>(
        &self,
        session: &Arc<cloud_client::Session>,
        config: SyncConfig,
        future: F,
    ) -> Result<T, SyncCoordError>
    where
        F: Future<Output = Result<T, SyncError>>,
    {
        self.with_live_session(session, config, || Ok(()))?;
        let result = future.await;
        self.with_live_session(session, config, || Ok(()))?;
        result.map_err(SyncCoordError::from)
    }

    fn current_identity(&self) -> Result<Option<(String, String)>, SyncCoordError> {
        self.auth
            .try_sync_identity()
            .map_err(|_| ProtocolFenceError::Busy.into())
    }

    fn load_for_session(
        &self,
        lease: &protocol_fence::V1Lease,
    ) -> Result<cloud_client::StateSnapshot, SyncCoordError> {
        if let Some((origin, account_id)) = self.current_identity()? {
            return self.load_for_identity(lease, &origin, &account_id);
        }
        Err(SyncCoordError::NotSignedIn)
    }

    fn load_for_identity(
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

    fn save_state(
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

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod sync_tests;
