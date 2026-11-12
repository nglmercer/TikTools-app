//! Cloud plugin-selection sync through the shared control plane.
//!
//! Everything here is nonsecret: the opt-in flag, pending/conflict counts,
//! and error text. V2 downloaded plugin selections can be applied after the
//! native account, server, source, package and runtime checks succeed.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_cloud_sync::{SyncCoordError, V1UncertainReview};

use super::OperationError;

mod conflicts;
mod identity;
mod legacy;
mod network;
mod projection;
mod recovery;
mod staging;

// Keep the internal projection path available to non-persistent consumers.
#[cfg_attr(not(feature = "persistence"), allow(unused_imports))]
pub(crate) use projection::v2_change_entry;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncNowResult {
    pub pushed: usize,
    pub pulled: usize,
    pub applied: usize,
    pub skipped: usize,
    pub conflicts: Vec<String>,
    /// At least one remote selection was retained without automatic local
    /// application. This is not a count of local differences or failures.
    #[serde(default)]
    pub application_deferred: bool,
}

/// A v2 network pass persists its cache and attempts supported plugin state
/// changes; other Cloud records are not applied by this operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceV2SyncResult {
    pub pushed: usize,
    pub pulled: usize,
    /// Pending edits in the v2 workspace; additional SQLite intents may be
    /// indicated by `more_queued`.
    pub workspace_pending: usize,
    /// Cached records available for application or review.
    pub available_records: usize,
    pub conflicts: Vec<String>,
    pub more_queued: bool,
}

/// A nonsecret projection of a single selection shown during explicit review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncRecoveryValue {
    pub state: String,
    pub version: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncRecoveryReview {
    pub handle: String,
    pub key: String,
    pub base: SyncRecoveryValue,
    pub original: SyncRecoveryValue,
    pub local: SyncRecoveryValue,
    pub cloud: SyncRecoveryValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncRecoveryChoice {
    KeepLocal,
    TakeCloud,
    TakeCloudIncludingNewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncRecoveryOutcome {
    Resolved,
    NewerEditNeedsReview,
}

pub(crate) struct RecoveryHandle {
    token: String,
    review: V1UncertainReview,
    newer_confirmation_required: bool,
}

/// One v2 conflict key for the current account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct V2ConflictTarget {
    pub collection: String,
    pub key: String,
}

/// A generic nonsecret projection for UI comparison. `revision` is present
/// for Cloud-backed values; `value` carries the portable JSON when present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct V2ConflictValue {
    pub state: String,
    pub revision: Option<i64>,
    pub value: Option<serde_json::Value>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
}

/// Reviewed v2 conflict comparison bound to a one-time handle. Resolution
/// re-reads Cloud and rejects the handle unless the fresh comparison matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct V2ConflictReview {
    pub handle: String,
    pub collection: String,
    pub key: String,
    pub base: V2ConflictValue,
    pub local: V2ConflictValue,
    pub cloud: V2ConflictValue,
    pub has_newer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum V2ConflictChoice {
    KeepLocal,
    TakeCloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum V2ConflictOutcome {
    Resolved,
}

/// One pending local edit of a v2 workspace entry, with the frozen Cloud base it
/// was made against. `status` is `queued`, `unconfirmed` or `conflict`.
///
/// For `plugin-settings` entries `base` and `local` carry no value (their
/// `value` is always null): settings are arbitrary plugin data that may be
/// sensitive, so only `changed_keys`, the sorted top-level setting names whose
/// value differs from the base (or that were added or removed), is projected.
/// It is `null` for every other collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct V2PendingChange {
    pub status: String,
    pub base: V2ConflictValue,
    pub local: V2ConflictValue,
    pub changed_keys: Option<Vec<String>>,
}

/// Read-only projection of one durable v2 workspace entry: the last fetched
/// Cloud record, the pending local edit when there is one, and whether a newer
/// local decision waits behind it. Nothing here contacts Cloud. Values are
/// projected only for the credential-free `plugin-selections` and
/// `preferences` and `variables` collections; every other collection (notably
/// `plugin-settings`) is reduced to metadata and never leaves the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct V2ChangeEntry {
    pub collection: String,
    pub key: String,
    pub remote: V2ConflictValue,
    pub pending: Option<V2PendingChange>,
    pub has_newer: bool,
}

pub(crate) struct V2ConflictHandle {
    token: String,
    session: std::sync::Arc<cloud_client::Session>,
    snapshot: tiktools_cloud_sync::V2ProductConflictSnapshot,
}

fn sync_error(error: SyncCoordError) -> OperationError {
    let message = error.to_string();
    let reason = error.code();
    let mut operation = match reason {
        "disabled" | "not-signed-in" | "network" | "unauthorized" => {
            OperationError::unavailable(message)
        }
        "already-running" | "stale-review" => OperationError::conflict(message),
        "no-pending-edit" | "no-uncertain-write" => OperationError::not_found(message),
        _ => OperationError::internal(message),
    };
    operation.data = Some(serde_json::json!({ "reason": reason }));
    operation
}

#[cfg(test)]
mod change_entry_tests;
