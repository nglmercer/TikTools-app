//! V2 workspace synchronization state. Transport never applies application
//! source, and uncertainty never becomes permission to repeat a write.
mod bootstrap;
mod local;
mod product;
mod queries;
mod remote;
mod resolution;
mod upload;
mod validation;

use validation::{identity_valid, remote_revision, valid_record, value_bytes};

use crate::{
    Session, SyncError, SyncState, WorkspaceChanges, WorkspaceRecord, WorkspaceSnapshot,
    WorkspaceWriteEpoch,
};
use cloud_contracts::portable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub(crate) const MAX_WORKSPACE_STATE_BYTES: usize = 48 * 1024 * 1024;
const VALUE_BYTES: usize = 8 * 1024 * 1024;
const MAX_ENTRIES: usize = 1000;

#[derive(Debug, Error)]
pub enum WorkspaceStateError {
    #[error("workspace state or response is invalid")]
    Invalid,
    #[error("workspace state belongs to a different origin or account")]
    WrongIdentity,
    #[error("workspace snapshot rebuild requires explicit review")]
    NeedsRebuild,
    #[error("a current read is required before resolving this workspace edit")]
    NeedsRead,
    #[error("workspace edit requires explicit conflict or unknown-outcome review")]
    Conflict,
    #[error("workspace review changed; read and review again")]
    StaleReview,
    #[error("no pending workspace edit exists")]
    NoPendingEdit,
    #[error("workspace state exceeds its bounded capacity")]
    TooLarge,
}
type Result<T> = std::result::Result<T, WorkspaceStateError>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceKey {
    pub collection: String,
    pub key: String,
}
impl WorkspaceKey {
    pub fn new(collection: impl Into<String>, key: impl Into<String>) -> Result<Self> {
        let value = Self {
            collection: collection.into(),
            key: key.into(),
        };
        if !value.valid() {
            return Err(WorkspaceStateError::Invalid);
        }
        Ok(value)
    }
    fn valid(&self) -> bool {
        portable::valid_key(&self.collection, &self.key)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum WorkspaceEdit {
    Put(Value),
    Delete,
}
impl WorkspaceEdit {
    /// Whether this edit satisfies the portable contract of `key`'s collection.
    pub fn valid(&self, key: &WorkspaceKey) -> bool {
        key.valid()
            && match self {
                Self::Put(value) => portable::validate(&key.collection, &key.key, value),
                Self::Delete => true,
            }
    }
    fn bytes(&self) -> Result<usize> {
        match self {
            Self::Put(value) => value_bytes(value),
            Self::Delete => Ok(0),
        }
    }
    /// Compare an edit with a record using portable JSON number equivalence.
    pub fn matches(&self, record: &WorkspaceRecord) -> bool {
        match self {
            Self::Put(value) => {
                !record.deleted
                    && record
                        .value
                        .as_ref()
                        .is_some_and(|remote| portable::values_equivalent(value, remote))
            }
            Self::Delete => record.deleted && record.value.is_none(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspacePendingStatus {
    Queued,
    Unconfirmed,
    Conflict,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePending {
    pub base_revision: Option<i64>,
    pub edit: WorkspaceEdit,
    pub local_generation: u64,
    pub base_record: Option<WorkspaceRecord>,
    pub status: WorkspacePendingStatus,
    /// Exact account-bound epoch prepared before a fenced product request.
    /// Its presence is not proof that the request reached the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_epoch: Option<WorkspaceWriteEpoch>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceLocalEdit {
    pub edit: WorkspaceEdit,
    pub local_generation: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceEntry {
    pub target: WorkspaceKey,
    pub remote: Option<WorkspaceRecord>,
    pub pending: Option<WorkspacePending>,
    /// A newer local decision cannot replace the frozen uncertain request.
    pub next: Option<WorkspaceLocalEdit>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceState {
    pub origin: String,
    pub account_id: String,
    pub cursor: i64,
    pub needs_rebuild: bool,
    pub bootstrap_complete: bool,
    pub next_local_generation: u64,
    pub entries: Vec<WorkspaceEntry>,
    /// Private local product-source receipts; never uploaded to Cloud.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    product_receipts: Vec<WorkspaceProductReceipt>,
    /// Once set, this partition must never be downgraded to generic mode.
    #[serde(default, skip_serializing_if = "is_false")]
    product_guarded: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkspaceProductReceipt {
    target: WorkspaceKey,
    source_generation: u64,
    decision_digest: String,
    edit: WorkspaceEdit,
    workspace_generation: u64,
}

/// Non-deserializable snapshot of the source receipt for one product edit.
/// This is evidence to compare under the product source fence, not authority
/// to send or apply anything by itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceProductProof(WorkspaceProductReceipt);
impl WorkspaceProductProof {
    pub fn target(&self) -> &WorkspaceKey {
        &self.0.target
    }
    pub fn source_generation(&self) -> u64 {
        self.0.source_generation
    }
    pub fn decision_digest(&self) -> &str {
        &self.0.decision_digest
    }
    pub fn edit(&self) -> &WorkspaceEdit {
        &self.0.edit
    }
    pub fn local_generation(&self) -> u64 {
        self.0.workspace_generation
    }
}

/// Non-deserializable proof of exactly the request durably prepared for HTTP.
#[derive(Clone)]
pub struct WorkspacePush {
    origin: String,
    account_id: String,
    target: WorkspaceKey,
    pending: WorkspacePending,
}
impl WorkspacePush {
    pub fn target(&self) -> &WorkspaceKey {
        &self.target
    }
    pub fn base_revision(&self) -> Option<i64> {
        self.pending.base_revision
    }
    pub fn edit(&self) -> &WorkspaceEdit {
        &self.pending.edit
    }
    pub fn local_generation(&self) -> u64 {
        self.pending.local_generation
    }
    pub fn write_epoch(&self) -> Option<&WorkspaceWriteEpoch> {
        self.pending.write_epoch.as_ref()
    }
}
/// Only an actual authenticated target GET (including verified 404) creates a
/// resolution read. A cached equal body is never an original-write receipt.
pub struct WorkspaceRead {
    origin: String,
    account_id: String,
    target: WorkspaceKey,
    record: Option<WorkspaceRecord>,
}
impl WorkspaceRead {
    pub fn target(&self) -> &WorkspaceKey {
        &self.target
    }
    pub fn record(&self) -> Option<&WorkspaceRecord> {
        self.record.as_ref()
    }
    pub fn revision(&self) -> Option<i64> {
        self.record.as_ref().map(|r| r.revision)
    }
}

#[cfg(test)]
mod tests;
