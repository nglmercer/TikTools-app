//! Cooperative cross-process protocol fencing for a private device directory.
//!
//! Older binaries and code that bypasses this coordinator cannot participate.
//! Local application mutations and session eligibility need their own product
//! fence; a protocol lease alone is not user consent to activate cloud data.
use crate::CutoverPreview;
use cloud_client::{Pending, Record, StoreError};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{fs::File, io, path::PathBuf};
use thiserror::Error;

mod cutover;
mod files;
mod leases;
mod legacy;
mod product;
mod records;
mod uncertain;
mod v1;

use files::{checked_file, marker_bytes, sync_dir};
use records::{candidate_hash, identity_hash};

const MAX_PROTOCOL_MARKER_BYTES: usize = 4096;
const MAX_UNCERTAIN_MARKER_BYTES: usize = 8192;

#[derive(Debug, Error)]
pub enum ProtocolFenceError {
    #[error("another sync or cutover operation is active")]
    Busy,
    #[error("v1 is unavailable while this account uses or is switching to v2")]
    NotV1,
    #[error("v1 has an uncertain prior upload; review its outcome before cutover")]
    UncertainV1,
    #[error("v2 workspace has pending local work; review it before enabling the product outbox")]
    UnsettledV2,
    #[error("this account requires the product outbox; legacy operations are unavailable")]
    ProductOutboxRequired,
    #[error("product sync is unavailable until the account has a reviewed outbox migration")]
    ProductOutboxNotEnabled,
    #[error("cutover needs the same reviewed inputs and a complete v2 bootstrap")]
    StaleCutover,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("protocol fence storage failed")]
    Io(#[from] io::Error),
}

#[derive(Clone)]
pub struct ProtocolFence {
    dir: PathBuf,
}

/// Shared leases let normal v1 staging continue while v1 network work runs.
pub(crate) struct V1Lease {
    fence: ProtocolFence,
    _file: File,
}

/// Exclusive across cooperating processes, held through durable cutover.
pub struct CutoverLease {
    fence: ProtocolFence,
    _file: File,
}

/// Held by product v2 operations while accessing this identity. A raw
/// WorkspaceCoordinator alone does not enforce product protocol selection.
pub struct V2Lease {
    _file: File,
}

/// Cross-process boundary between a product network emission and a local
/// edit that cannot be attributed to Cloud. This directory is independent
/// of the optional Cloud coordinator state directory.
#[derive(Clone)]
pub struct ProductEmissionFence {
    fence: ProtocolFence,
}

/// Owns the advisory file lock until the request completes or is cancelled.
pub struct ProductEmissionLease {
    _file: File,
}
pub(crate) struct V1NetworkLease {
    _file: File,
}

/// Serializes short product mutation and upload-admission transactions across
/// cooperating processes. It must never be held during an HTTP await.
pub(crate) struct V1AdmissionLease {
    _file: File,
}

// A concurrently forked child can briefly inherit an open lock descriptor
// before close-on-exec takes effect. Explicitly unlock at the end of the
// lease rather than waiting for every inherited descriptor to close. This also
// makes the lifetime of a lease independent of a child's pre-exec delay.
macro_rules! unlock_on_drop {
    ($($lease:ty),+ $(,)?) => {$ (
        impl Drop for $lease {
            fn drop(&mut self) {
                // Closing the file remains a fallback if explicit unlock fails.
                let _ = FileExt::unlock(&self._file);
            }
        }
    )+};
}
unlock_on_drop!(
    V1Lease,
    CutoverLease,
    V2Lease,
    ProductEmissionLease,
    V1NetworkLease,
    V1AdmissionLease
);

pub(crate) struct UncertainUpload {
    path: PathBuf,
}

/// Frozen original candidate and base, without credentials or a replay action.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct V1UncertainUpload {
    format_version: u32,
    origin: String,
    account_id: String,
    source_generation: u64,
    key: String,
    pending: Pending,
    base_record: Option<Record>,
}
impl V1UncertainUpload {
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn pending(&self) -> &Pending {
        &self.pending
    }
    pub fn base_record(&self) -> Option<&Record> {
        self.base_record.as_ref()
    }
    pub fn source_generation(&self) -> u64 {
        self.source_generation
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProtocolRecord {
    format_version: u32,
    origin: String,
    account_id: String,
    mode: ProtocolMode,
    candidate_sha256: String,
}

/// Opaque exact marker used only to revalidate a native consent review.
#[allow(dead_code)] // Used by the not-yet-exposed native v2 consent review.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ResumeProtocolRecord(ProtocolRecord);

#[allow(dead_code)]
impl ResumeProtocolRecord {
    pub(crate) fn is_preparing(&self) -> bool {
        self.0.mode == ProtocolMode::Preparing
    }

    pub(crate) fn is_marked(&self) -> bool {
        self.0.format_version == 3
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProductOutboxRecord {
    format_version: u32,
    origin: String,
    account_id: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum ProtocolMode {
    Preparing,
    V2,
}

#[cfg(test)]
mod tests;
