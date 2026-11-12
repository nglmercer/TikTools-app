//! Explicitly opt-in sync client. No network is used until a session is opened.
//! Persist `SyncState` in application storage; keep session tokens in an OS keychain.
#![forbid(unsafe_code)]
#[cfg(all(
    feature = "os-keychain",
    not(any(target_os = "linux", target_os = "macos", target_os = "windows"))
))]
compile_error!("os-keychain is supported only on Linux, macOS, and Windows");
#[cfg(feature = "os-keychain")]
mod keychain;
#[cfg(feature = "os-keychain")]
pub use keychain::{Keychain, KeychainError};
mod account;
mod exports;
mod selection_state;
mod selection_sync;
mod transport;

use selection_state::{valid_key, validate_record};
#[cfg(test)]
use transport::retry_delay;
use transport::{decode, decode_with_limit, parse_origin};

mod login;
mod store;
mod workspace;
mod workspace_state;
mod workspace_store;
pub use cloud_contracts::{WorkspaceChanges, WorkspaceRecord, WorkspaceSnapshot};
pub use login::{LoginAttempt, LoginResult};
use reqwest::{
    header::{HeaderValue, AUTHORIZATION, IF_MATCH, IF_NONE_MATCH},
    Client, Response, StatusCode,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
pub use store::{StateSnapshot, StateStore, StoreError};
use thiserror::Error;
use url::Url;
use uuid::Uuid;
pub use workspace::{
    CatalogAdvisory, CatalogPageCapabilities, CatalogSummary, CatalogSummaryPage, CatalogVersion,
    CatalogVersionPage, CloudCatalog, WorkspaceSyncCapabilities, WorkspaceWriteEpoch,
};
pub use workspace_state::{
    WorkspaceEdit, WorkspaceEntry, WorkspaceKey, WorkspaceLocalEdit, WorkspacePending,
    WorkspacePendingStatus, WorkspaceProductProof, WorkspacePush, WorkspaceRead, WorkspaceState,
    WorkspaceStateError,
};
pub use workspace_store::{WorkspaceOperationGuard, WorkspaceStateSnapshot, WorkspaceStore};

const COLLECTION: &str = "plugin-selections";
const MAX_RESPONSE: usize = 1024 * 1024;
const MAX_ACCOUNT_EXPORT: usize = 16 * 1024 * 1024;
const MAX_PAGES: usize = 200;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("invalid cloud origin or token")]
    Configuration,
    #[error("network request failed")]
    Network,
    #[error("cloud session is not authorized")]
    Unauthorized,
    #[error("sync state belongs to a different account")]
    WrongAccount,
    #[error("remote change conflicts with a local edit")]
    Conflict,
    #[error("sync cursor expired; rebuild is required")]
    CursorExpired,
    #[error("cloud rate limit reached; retry after {retry_after_seconds:?} seconds")]
    RateLimited { retry_after_seconds: Option<u64> },
    #[error("cloud rejected the request ({0})")]
    Http(u16),
    #[error("invalid cloud response")]
    Protocol,
    #[error("too many pages; retry the operation")]
    TooManyPages,
    #[error("no local edit exists")]
    NoPendingEdit,
    #[error("desktop login was cancelled or expired")]
    LoginCancelled,
    #[error("unable to open the system browser")]
    Browser,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginSelection {
    pub schema_version: u8,
    pub plugin_id: String,
    pub version: String,
    pub enabled: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    pub collection: String,
    pub key: String,
    pub revision: i64,
    pub deleted: bool,
    pub value: Option<PluginSelection>,
}
/// Current server-side sync records, including tombstones; pending local edits are excluded.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncExport {
    pub schema_version: u8,
    pub cursor: i64,
    pub records: Vec<Record>,
}

/// Bounded Cloud-owned account data. Authentication secrets and object-storage
/// capabilities are intentionally absent; callers must still protect the export.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountExport {
    pub schema_version: u8,
    pub profile: ExportProfile,
    pub cloud_roles: Vec<String>,
    pub publisher_memberships: Vec<serde_json::Value>,
    pub publisher_keys_requested: Vec<serde_json::Value>,
    pub releases_submitted: Vec<serde_json::Value>,
    pub uploads_requested: Vec<serde_json::Value>,
    pub reviews_recorded: Vec<serde_json::Value>,
    pub publication_requests: Vec<serde_json::Value>,
    pub ownership_transfers: Vec<serde_json::Value>,
    pub audit_events_by_actor: Vec<serde_json::Value>,
    pub yanks_recorded: Vec<serde_json::Value>,
    pub advisories_recorded: Vec<serde_json::Value>,
    pub sync: SyncExport,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportProfile {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "operation",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Edit {
    Put(PluginSelection),
    Delete,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    pub base_revision: Option<i64>,
    pub edit: Edit,
    pub conflict: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub remote: Option<Record>,
    pub pending: Option<Pending>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncState {
    pub origin: String,
    pub account_id: String,
    pub cursor: i64,
    pub entries: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
struct Profile {
    id: String,
}
#[derive(Deserialize)]
struct Page {
    changes: Vec<Record>,
    cursor: i64,
    has_more: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Listing {
    records: Vec<Record>,
    next_after: Option<String>,
}
#[derive(Deserialize)]
struct Head {
    cursor: i64,
}

/// Cloud-owned deletion blockers. Each page is a separate point-in-time view.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeletionBlockerPage {
    pub cloud_roles: Vec<String>,
    pub memberships: Vec<DeletionMembership>,
    pub next_after: Option<Uuid>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeletionMembership {
    pub publisher_id: Uuid,
    pub namespace: String,
    pub role: String,
}

/// A verified account and in-memory token. It is not serializable or printable.
pub struct Session {
    origin: Url,
    client: Client,
    token: SecretString,
    account_id: String,
}

#[cfg(test)]
mod tests;
