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
mod coordinator;
mod workspace;
pub use workspace::{
    WorkspaceCoordError, WorkspaceCoordinator, WorkspaceEligibility, WorkspaceOutcome,
    WorkspaceReview, WorkspaceSyncScope,
};
mod cutover_preview;
pub use cutover_preview::CutoverPreview;
mod cutover_receipt;
pub use cutover_receipt::V1CutoverReceipt;
mod cutover_execution;
pub use cutover_execution::{CutoverExecution, CutoverSource, GuardedCutoverReview};
mod protocol_fence;
pub use protocol_fence::{
    CutoverLease, ProductEmissionFence, ProductEmissionLease, ProtocolFence, ProtocolFenceError,
    V1UncertainUpload, V2Lease,
};
mod v1_recovery;
pub use v1_recovery::{V1RecoveryChoice, V1RecoveryOutcome, V1UncertainReview};
mod local_edit;
pub use local_edit::{ProductLocalEditContext, V1LocalEditContext, V2LocalEditContext};
mod v2_product_outbox;
pub use v2_product_outbox::{
    V2ProductConflictSnapshot, V2ProductDecision, V2ProductSource, V2ProductUploadSource,
};
mod product_outbox;
pub use product_outbox::{V1ConflictSourceProof, V1OutboxCandidate, V1ProductOutbox};
mod product_remote_apply;
pub use product_remote_apply::V1RemoteApplyReview;
mod v2_product_remote_apply;
pub use v2_product_remote_apply::V2RemoteApplyReview;
mod v2_reconciliation;
pub use v2_reconciliation::{V2ReconciliationAdmission, V2ReconciliationBarrier};
mod portable_adapters;
pub use portable_adapters::{
    preference_apply, preference_from_native, settings_apply, settings_from_native,
};
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
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

use cloud_client::StoreError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tiktools_cloud_auth::CloudAuth;
use tokio::sync::Mutex;

const CONFIG_FILE: &str = "cloud-sync.json";

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

#[cfg(test)]
mod sync_tests;
