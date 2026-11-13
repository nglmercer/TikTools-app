//! Durable v2 workspace orchestration. This module never applies downloaded
//! source to the application. The product adapter owns opt-in, captured-session
//! eligibility and explicit local reviews; the v1 coordinator does not call it.
use std::{path::Path, sync::Arc};

use cloud_client::{
    Session, StoreError, SyncError, WorkspaceEntry, WorkspaceKey, WorkspaceProductProof,
    WorkspaceRead, WorkspaceRecord, WorkspaceStateError, WorkspaceStore,
};
use thiserror::Error;

mod admission;
mod bootstrap;
mod eligibility;
mod local;
mod rebuild;
mod resolution;
mod review;
mod storage;
mod sync;

use eligibility::check_eligibility;

const MAX_LOCAL_RETRIES: usize = 8;

pub(crate) struct ProductUploadAdmission<'a> {
    pub source: &'a dyn crate::V2ProductUploadSource,
    pub config_generation: u128,
}

#[derive(Debug, Error)]
pub enum WorkspaceCoordError {
    #[error("workspace cutover requires explicit review")]
    BootstrapRequired,
    #[error("workspace cutover was already completed; existing edits were preserved")]
    AlreadyBootstrapped,
    #[error("unfinished workspace contains data; review it before cutover")]
    BootstrapNeedsReview,
    #[error("another workspace network or review operation is in progress")]
    Busy,
    #[error("workspace session or opt-in changed during the operation")]
    Ineligible,
    #[error("workspace operation exceeded its time limit")]
    TimedOut,
    #[error("workspace background worker failed")]
    WorkerFailed,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    State(#[from] WorkspaceStateError),
    #[error(transparent)]
    Transport(#[from] SyncError),
    #[error(transparent)]
    Protocol(#[from] crate::ProtocolFenceError),
}

/// Captures one product session and opt-in generation. The callback must remain
/// false after either changes, even if the same account signs in or opts in
/// again. It must be non-blocking and must not expose credentials.
#[derive(Clone)]
pub struct WorkspaceEligibility {
    binding: Arc<()>,
    session: Arc<Session>,
    current: Arc<dyn Fn() -> bool + Send + Sync>,
    publication: Option<Arc<PublicationGuard>>,
    emission: Option<Arc<EmissionGuard>>,
    emission_current: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}
type PublicationGuard = dyn Fn(&mut dyn FnMut() -> Result<(), WorkspaceCoordError>) -> Result<(), WorkspaceCoordError>
    + Send
    + Sync;
type EmissionGuard = dyn Fn() -> Result<Box<dyn Send>, WorkspaceCoordError> + Send + Sync;

/// Fetched data is not evidence of local application or execution approval.
pub struct WorkspaceOutcome {
    pub pulled: usize,
    pub pushed: usize,
    pub absent_deletes: usize,
    pub pending: usize,
    pub conflicts: Vec<WorkspaceKey>,
    pub remote: Vec<WorkspaceRecord>,
    pub more_queued: bool,
}

/// Limits publication and returned application candidates. Downloads still
/// cache every collection before advancing the account's shared cursor.
#[derive(Clone, Default)]
pub enum WorkspaceSyncScope {
    #[default]
    All,
    Collections(std::collections::BTreeSet<String>),
}

impl WorkspaceSyncScope {
    pub fn includes(&self, collection: &str) -> bool {
        match self {
            Self::All => true,
            Self::Collections(collections) => collections.contains(collection),
        }
    }
}

/// Native, non-deserializable comparison captured by a real authenticated GET.
/// A UI adapter must retain this object behind its own review identity rather
/// than reconstructing consent from a key and a bare boolean.
pub struct WorkspaceReview {
    binding: Arc<()>,
    eligibility: Option<Arc<()>>,
    origin: String,
    account_id: String,
    read: WorkspaceRead,
    generation: u64,
    entry: WorkspaceEntry,
    product_proof: Option<WorkspaceProductProof>,
}
impl WorkspaceReview {
    pub fn target(&self) -> &WorkspaceKey {
        self.read.target()
    }
    pub fn remote(&self) -> Option<&WorkspaceRecord> {
        self.read.record()
    }
    pub fn entry(&self) -> &WorkspaceEntry {
        &self.entry
    }
    pub fn local_generation(&self) -> u64 {
        self.generation
    }
}

/// One account on one operator-provisioned origin. Separate identities retain
/// their own files and pending work. No credential is stored in this handle.
pub struct WorkspaceCoordinator {
    binding: Arc<()>,
    store: WorkspaceStore,
    protocol: crate::ProtocolFence,
    origin: String,
    account_id: String,
}

impl WorkspaceCoordinator {
    pub fn open(
        root: impl AsRef<Path>,
        origin: String,
        account_id: String,
    ) -> Result<Self, WorkspaceCoordError> {
        let store = WorkspaceStore::open(&root, origin.clone(), account_id.clone())?;
        let protocol = crate::ProtocolFence::open(root)?;
        Ok(Self {
            binding: Arc::new(()),
            store,
            protocol,
            origin,
            account_id,
        })
    }
}

#[cfg(test)]
mod tests;
