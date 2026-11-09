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
    CatalogVersionPage, CloudCatalog, WorkspaceSyncCapabilities,
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
impl SyncExport {
    fn validate(&self) -> Result<(), SyncError> {
        if self.schema_version != 1 || self.cursor < 0 || self.records.len() > 1000 {
            return Err(SyncError::Protocol);
        }
        let mut keys = BTreeSet::new();
        for record in &self.records {
            validate_record(record)?;
            if record.revision > self.cursor || !keys.insert(&record.key) {
                return Err(SyncError::Protocol);
            }
        }
        Ok(())
    }
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
impl AccountExport {
    fn validate(&self, account_id: &str) -> Result<(), SyncError> {
        if self.schema_version != 1 || self.profile.id != account_id {
            return Err(SyncError::WrongAccount);
        }
        self.sync.validate()?;
        for section in [
            &self.publisher_memberships,
            &self.publisher_keys_requested,
            &self.releases_submitted,
            &self.uploads_requested,
            &self.reviews_recorded,
            &self.publication_requests,
            &self.ownership_transfers,
            &self.audit_events_by_actor,
            &self.yanks_recorded,
            &self.advisories_recorded,
        ] {
            if section.len() > 1000 || section.iter().any(|item| !item.is_object()) {
                return Err(SyncError::Protocol);
            }
            if section.iter().any(contains_sensitive_field) {
                return Err(SyncError::Protocol);
            }
        }
        if self.cloud_roles.len() > 1000
            || self
                .cloud_roles
                .iter()
                .any(|role| !matches!(role.as_str(), "admin" | "moderator"))
        {
            return Err(SyncError::Protocol);
        }
        Ok(())
    }
}
fn contains_sensitive_field(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(fields) => fields.iter().any(|(name, child)| {
            matches!(
                name.as_str(),
                "token"
                    | "sessionToken"
                    | "accessToken"
                    | "refreshToken"
                    | "password"
                    | "passwordHash"
                    | "privateKey"
                    | "objectKey"
                    | "versionId"
                    | "signedUrl"
            ) || contains_sensitive_field(child)
        }),
        serde_json::Value::Array(items) => items.iter().any(contains_sensitive_field),
        _ => false,
    }
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
impl SyncState {
    pub fn new(origin: String, account_id: String) -> Self {
        Self {
            origin,
            account_id,
            cursor: 0,
            entries: BTreeMap::new(),
        }
    }
    /// Stage an edit locally. Repeated local changes retain the original base revision.
    pub fn stage(&mut self, key: String, edit: Edit) -> Result<(), SyncError> {
        if !valid_key(&key)
            || matches!(&edit, Edit::Put(value) if value.schema_version != 1 || value.plugin_id != key || value.version.len() > 64 || semver::Version::parse(&value.version).is_err())
        {
            return Err(SyncError::Configuration);
        }
        let entry = self.entries.entry(key).or_default();
        if let Some(pending) = &mut entry.pending {
            pending.edit = edit;
        } else {
            entry.pending = Some(Pending {
                base_revision: entry.remote.as_ref().map(|r| r.revision),
                edit,
                conflict: false,
            });
        }
        Ok(())
    }
    /// Explicit user choice to keep local data after reviewing a conflict.
    pub fn rebase(&mut self, key: &str) -> Result<(), SyncError> {
        let entry = self.entries.get_mut(key).ok_or(SyncError::NoPendingEdit)?;
        let pending = entry.pending.as_mut().ok_or(SyncError::NoPendingEdit)?;
        pending.base_revision = entry.remote.as_ref().map(|r| r.revision);
        pending.conflict = false;
        Ok(())
    }
    /// Explicit user choice to discard an unsent local edit.
    pub fn discard(&mut self, key: &str) -> Result<(), SyncError> {
        let entry = self.entries.get_mut(key).ok_or(SyncError::NoPendingEdit)?;
        if entry.pending.take().is_none() {
            return Err(SyncError::NoPendingEdit);
        }
        Ok(())
    }
    fn apply(&mut self, record: Record) -> Result<(), SyncError> {
        validate_record(&record)?;
        let entry = self.entries.entry(record.key.clone()).or_default();
        if entry
            .remote
            .as_ref()
            .is_some_and(|old| old.revision >= record.revision)
        {
            return Ok(());
        }
        if let Some(pending) = &mut entry.pending {
            if pending.base_revision != Some(record.revision) {
                pending.conflict = true;
            }
        }
        entry.remote = Some(record);
        Ok(())
    }
}
fn valid_key(key: &str) -> bool {
    key.len() <= 128
        && key.split_once('.').is_some()
        && key.split('.').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
        })
}
fn validate_record(record: &Record) -> Result<(), SyncError> {
    if record.collection != COLLECTION
        || !valid_key(&record.key)
        || record.revision <= 0
        || record.deleted != record.value.is_none()
        || record.value.as_ref().is_some_and(|v| {
            v.schema_version != 1
                || v.plugin_id != record.key
                || v.version.len() > 64
                || semver::Version::parse(&v.version).is_err()
        })
    {
        return Err(SyncError::Protocol);
    }
    Ok(())
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
impl DeletionBlockerPage {
    fn validate(&self, after: Option<Uuid>) -> Result<(), SyncError> {
        let mut roles = BTreeSet::new();
        if self
            .cloud_roles
            .iter()
            .any(|r| !matches!(r.as_str(), "admin" | "moderator") || !roles.insert(r))
            || self.memberships.len() > 100
        {
            return Err(SyncError::Protocol);
        }
        let mut previous = after;
        for member in &self.memberships {
            let name = member.namespace.as_bytes();
            if previous.is_some_and(|id| member.publisher_id <= id)
                || !(2..=63).contains(&name.len())
                || !name[0].is_ascii_lowercase()
                || name[name.len() - 1] == b'-'
                || !name
                    .iter()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
                || !matches!(
                    member.role.as_str(),
                    "owner" | "admin" | "publisher" | "viewer"
                )
            {
                return Err(SyncError::Protocol);
            }
            previous = Some(member.publisher_id);
        }
        if self.next_after.is_some()
            && (self.memberships.len() != 100 || self.next_after != previous)
        {
            return Err(SyncError::Protocol);
        }
        Ok(())
    }
}

/// A verified account and in-memory token. It is not serializable or printable.
pub struct Session {
    origin: Url,
    client: Client,
    token: SecretString,
    account_id: String,
}
impl Session {
    /// Explicitly connects and verifies the token. `origin` must be a trusted HTTPS origin;
    /// plain HTTP is accepted only for loopback IP literals for local development.
    pub async fn connect(origin: &str, token: String) -> Result<Self, SyncError> {
        Self::connect_secret(origin, SecretString::from(token)).await
    }
    pub async fn connect_secret(origin: &str, token: SecretString) -> Result<Self, SyncError> {
        let url = parse_origin(origin)?;
        if token.expose_secret().is_empty() || token.expose_secret().len() > 4096 {
            return Err(SyncError::Configuration);
        }
        let mut value = HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))
            .map_err(|_| SyncError::Configuration)?;
        value.set_sensitive(true);
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| SyncError::Configuration)?;
        let mut session = Self {
            origin: url,
            client,
            token,
            account_id: String::new(),
        };
        let profile: Profile = session.get("api/v1/native/me", &[]).await?;
        if profile.id.is_empty() || profile.id.len() > 256 {
            return Err(SyncError::Protocol);
        }
        session.account_id = profile.id;
        Ok(session)
    }
    /// Expose only to an OS credential store; never log or serialize the token.
    pub fn token_for_keychain(&self) -> &SecretString {
        &self.token
    }
    pub fn account_id(&self) -> &str {
        &self.account_id
    }
    /// Canonical operator instance identity, suitable for account-scoped storage.
    pub fn instance_origin(&self) -> String {
        self.origin.origin().ascii_serialization()
    }
    /// List Cloud-owned deletion blockers. An empty page is not a deletion guarantee.
    pub async fn deletion_blockers(
        &self,
        after: Option<Uuid>,
    ) -> Result<DeletionBlockerPage, SyncError> {
        let params = after
            .map(|id| vec![("after", id.to_string())])
            .unwrap_or_default();
        let page: DeletionBlockerPage = self
            .get("api/v1/native/me/deletion-blockers", &params)
            .await?;
        page.validate(after)?;
        Ok(page)
    }
    /// Revoke the current bearer session on the server. Remove any stored keychain
    /// entry separately after success; an offline error means revocation is unknown.
    pub async fn logout(&self) -> Result<(), SyncError> {
        let response = self
            .request(reqwest::Method::POST, "api/v1/native/logout")?
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(SyncError::Unauthorized),
            status => Err(SyncError::Http(status.as_u16())),
        }
    }
    pub fn new_state(&self) -> SyncState {
        SyncState::new(
            self.origin.origin().ascii_serialization(),
            self.account_id.clone(),
        )
    }
    fn verify_account(&self, state: &SyncState) -> Result<(), SyncError> {
        if state.account_id != self.account_id
            || state.origin != self.origin.origin().ascii_serialization()
        {
            return Err(SyncError::WrongAccount);
        }
        Ok(())
    }
    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, SyncError> {
        let url = self
            .origin
            .join(path)
            .map_err(|_| SyncError::Configuration)?;
        let mut auth = HeaderValue::from_str(&format!("Bearer {}", self.token.expose_secret()))
            .map_err(|_| SyncError::Configuration)?;
        auth.set_sensitive(true);
        Ok(self.client.request(method, url).header(AUTHORIZATION, auth))
    }
    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, SyncError> {
        let response = self
            .request(reqwest::Method::GET, path)?
            .query(params)
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        decode(response).await
    }
    /// Retrieve a point-in-time export of server-side sync records without changing local state.
    pub async fn export_sync(&self) -> Result<SyncExport, SyncError> {
        let export: SyncExport = self.get("api/v1/native/sync/export", &[]).await?;
        export.validate()?;
        Ok(export)
    }
    /// Retrieve and validate a bounded snapshot of this account's Cloud-owned data.
    pub async fn export_account(&self) -> Result<AccountExport, SyncError> {
        let response = self
            .request(reqwest::Method::GET, "api/v1/native/me/export")?
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        let export: AccountExport = decode_with_limit(response, MAX_ACCOUNT_EXPORT).await?;
        export.validate(&self.account_id)?;
        Ok(export)
    }
    /// Apply one change page atomically in memory; persist it only after this returns.
    pub async fn pull_page(&self, state: &mut SyncState) -> Result<bool, SyncError> {
        self.pull_page_with_count(state)
            .await
            .map(|(has_more, _)| has_more)
    }
    /// Apply one page and report both the continuation flag and the number
    /// of change records received, including records in the final page.
    pub async fn pull_page_with_count(
        &self,
        state: &mut SyncState,
    ) -> Result<(bool, usize), SyncError> {
        self.verify_account(state)?;
        let page: Page = self
            .get(
                "api/v1/native/sync/changes",
                &[("cursor", state.cursor.to_string())],
            )
            .await?;
        if page.cursor < state.cursor || (page.has_more && page.cursor == state.cursor) {
            return Err(SyncError::Protocol);
        }
        let mut candidate = state.clone();
        let mut last = state.cursor;
        let count = page.changes.len();
        for record in page.changes {
            if record.revision <= last || record.revision > page.cursor {
                return Err(SyncError::Protocol);
            }
            last = record.revision;
            candidate.apply(record)?;
        }
        if last != page.cursor {
            return Err(SyncError::Protocol);
        }
        candidate.cursor = page.cursor;
        *state = candidate;
        Ok((page.has_more, count))
    }
    /// Send one pending edit. A 409 leaves the edit intact for explicit resolution.
    pub async fn push(&self, state: &mut SyncState, key: &str) -> Result<(), SyncError> {
        self.verify_account(state)?;
        if !valid_key(key) {
            return Err(SyncError::Configuration);
        }
        let entry = state.entries.get(key).ok_or(SyncError::NoPendingEdit)?;
        let pending = entry.pending.as_ref().ok_or(SyncError::NoPendingEdit)?;
        if pending.conflict {
            return Err(SyncError::Conflict);
        }
        if pending.base_revision.is_none() && matches!(pending.edit, Edit::Delete) {
            // A previous write could have succeeded despite a lost response, or
            // another device could have created this record while we were offline.
            state
                .entries
                .get_mut(key)
                .unwrap()
                .pending
                .as_mut()
                .unwrap()
                .conflict = true;
            return Err(SyncError::Conflict);
        }
        let path = format!("api/v1/native/sync/collections/{COLLECTION}/{key}");
        let mut request = match &pending.edit {
            Edit::Put(value) => self.request(reqwest::Method::PUT, &path)?.json(value),
            Edit::Delete => self.request(reqwest::Method::DELETE, &path)?,
        };
        request = if let Some(revision) = pending.base_revision {
            request.header(IF_MATCH, format!("\"{revision}\""))
        } else {
            request.header(IF_NONE_MATCH, "*")
        };
        let response = request.send().await.map_err(|_| SyncError::Network)?;
        if response.status() == StatusCode::CONFLICT {
            state
                .entries
                .get_mut(key)
                .unwrap()
                .pending
                .as_mut()
                .unwrap()
                .conflict = true;
            return Err(SyncError::Conflict);
        }
        let record: Record = decode(response).await?;
        validate_record(&record)?;
        if (match &pending.edit {
            Edit::Put(value) => record.value.as_ref() != Some(value) || record.deleted,
            Edit::Delete => !record.deleted || record.value.is_some(),
        }) || record.key != key
            || pending
                .base_revision
                .is_some_and(|revision| record.revision <= revision)
        {
            return Err(SyncError::Protocol);
        }
        let entry = state.entries.get_mut(key).unwrap();
        entry.remote = Some(record);
        entry.pending = None;
        Ok(())
    }
    /// Rebuild after retention expiry without dropping pending edits. Snapshot and
    /// follow-up changes are committed to state only after every page succeeds.
    pub async fn rebuild(&self, state: &mut SyncState) -> Result<(), SyncError> {
        self.rebuild_with_count(state).await.map(|_| ())
    }
    /// Rebuild and report the number of snapshot and follow-up feed records
    /// consumed. State remains unchanged if any page fails validation.
    pub async fn rebuild_with_count(&self, state: &mut SyncState) -> Result<usize, SyncError> {
        self.verify_account(state)?;
        let head: Head = self.get("api/v1/native/sync/head", &[]).await?;
        if head.cursor < 0 {
            return Err(SyncError::Protocol);
        }
        let mut candidate = SyncState::new(state.origin.clone(), state.account_id.clone());
        candidate.cursor = head.cursor;
        let mut after: Option<String> = None;
        let mut complete = false;
        let mut count = 0usize;
        for _ in 0..MAX_PAGES {
            let params = after
                .as_ref()
                .map(|s| vec![("after", s.clone())])
                .unwrap_or_default();
            let listing: Listing = self
                .get("api/v1/native/sync/collections/plugin-selections", &params)
                .await?;
            let previous = after.clone();
            count = count
                .checked_add(listing.records.len())
                .ok_or(SyncError::Protocol)?;
            for record in listing.records {
                if after.as_ref().is_some_and(|a| record.key <= *a) {
                    return Err(SyncError::Protocol);
                }
                after = Some(record.key.clone());
                candidate.apply(record)?;
            }
            if let Some(next) = listing.next_after {
                if Some(&next) != after.as_ref() || previous.as_ref().is_some_and(|p| next <= *p) {
                    return Err(SyncError::Protocol);
                }
            } else {
                complete = true;
                break;
            }
        }
        if !complete {
            return Err(SyncError::TooManyPages);
        }
        for (key, old) in &state.entries {
            if let Some(pending) = &old.pending {
                let entry = candidate.entries.entry(key.clone()).or_default();
                let mut pending = pending.clone();
                if pending.base_revision != entry.remote.as_ref().map(|r| r.revision) {
                    pending.conflict = true;
                }
                entry.pending = Some(pending);
            }
        }
        for _ in 0..MAX_PAGES {
            let (more, received) = self.pull_page_with_count(&mut candidate).await?;
            count = count.checked_add(received).ok_or(SyncError::Protocol)?;
            if !more {
                *state = candidate;
                return Ok(count);
            }
        }
        Err(SyncError::TooManyPages)
    }
}
fn parse_origin(origin: &str) -> Result<Url, SyncError> {
    let url = Url::parse(origin).map_err(|_| SyncError::Configuration)?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || url.scheme() == "http" && loopback)
        || url.host().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(SyncError::Configuration);
    }
    Ok(url)
}

fn retry_delay(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let values: Vec<_> = headers
        .get_all(reqwest::header::RETRY_AFTER)
        .iter()
        .collect();
    if values.len() != 1 {
        return None;
    }
    values[0]
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()
        .filter(|value| (1..=86_400).contains(value))
}

async fn decode<T: for<'de> Deserialize<'de>>(response: Response) -> Result<T, SyncError> {
    decode_with_limit(response, MAX_RESPONSE).await
}

async fn decode_with_limit<T: for<'de> Deserialize<'de>>(
    mut response: Response,
    max_bytes: usize,
) -> Result<T, SyncError> {
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(SyncError::Unauthorized),
        StatusCode::CONFLICT => return Err(SyncError::Conflict),
        StatusCode::GONE => return Err(SyncError::CursorExpired),
        StatusCode::TOO_MANY_REQUESTS => {
            return Err(SyncError::RateLimited {
                retry_after_seconds: retry_delay(response.headers()),
            })
        }
        s if !s.is_success() => return Err(SyncError::Http(s.as_u16())),
        _ => (),
    }
    if response
        .content_length()
        .is_some_and(|n| n > max_bytes as u64)
    {
        return Err(SyncError::Protocol);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| SyncError::Network)? {
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(SyncError::Protocol);
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| SyncError::Protocol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn pull_count_includes_final_and_empty_pages_and_preserves_legacy_api() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let first = serde_json::json!({"changes":[
            {"collection":"plugin-selections","key":"acme.a","revision":1,"deleted":false,
             "value":{"schemaVersion":1,"pluginId":"acme.a","version":"1.0.0","enabled":true}},
            {"collection":"plugin-selections","key":"acme.b","revision":2,"deleted":false,
             "value":{"schemaVersion":1,"pluginId":"acme.b","version":"1.0.0","enabled":false}}
        ],"cursor":2,"has_more":false})
        .to_string();
        let empty = serde_json::json!({"changes":[],"cursor":2,"has_more":false}).to_string();
        let invalid = serde_json::json!({"changes":[],"cursor":1,"has_more":false}).to_string();
        let task = tokio::spawn(async move {
            for body in [
                r#"{"id":"alice"}"#.to_owned(),
                first,
                empty.clone(),
                invalid,
                empty,
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 4096];
                let _ = socket.read(&mut request).await.unwrap();
                let head = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                socket.write_all(head.as_bytes()).await.unwrap();
                socket.write_all(body.as_bytes()).await.unwrap();
            }
        });
        let session = Session::connect(&origin, "fixture".into()).await.unwrap();
        let mut state = session.new_state();
        assert_eq!(
            session.pull_page_with_count(&mut state).await.unwrap(),
            (false, 2)
        );
        let saved = state.clone();
        assert_eq!(
            session.pull_page_with_count(&mut state).await.unwrap(),
            (false, 0)
        );
        assert!(matches!(
            session.pull_page_with_count(&mut state).await,
            Err(SyncError::Protocol)
        ));
        assert_eq!(state, saved);
        assert!(!session.pull_page(&mut state).await.unwrap());
        assert_eq!(state, saved);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn account_export_response_limit_allows_large_bounded_payloads() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let body = serde_json::json!({"payload": "x".repeat(MAX_RESPONSE)}).to_string();
        let task = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 4096];
                let _ = socket.read(&mut request).await;
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(header.as_bytes()).await;
                let _ = socket.write_all(body.as_bytes()).await;
            }
        });
        let url = format!("http://{address}/");
        let response = reqwest::get(&url).await.unwrap();
        let value: serde_json::Value = decode_with_limit(response, MAX_ACCOUNT_EXPORT)
            .await
            .unwrap();
        assert_eq!(value["payload"].as_str().unwrap().len(), MAX_RESPONSE);
        let response = reqwest::get(&url).await.unwrap();
        assert!(matches!(
            decode::<serde_json::Value>(response).await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }

    #[tokio::test]
    async fn refuses_redirects_before_forwarding_bearer() {
        let destination = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = destination.local_addr().unwrap();
        let source = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", source.local_addr().unwrap());
        let handler = tokio::spawn(async move {
            let (mut socket, _) = source.accept().await.unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request).await.unwrap();
            socket.write_all(format!("HTTP/1.1 302 Found\r\nLocation: http://{target}/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        });
        assert!(matches!(
            Session::connect(&origin, "test-token".into()).await,
            Err(SyncError::Http(302))
        ));
        handler.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), destination.accept())
                .await
                .is_err()
        );
    }

    #[test]
    fn deletion_blocker_pages_reject_invalid_roles_and_cursors() {
        let member = DeletionMembership {
            publisher_id: Uuid::from_u128(1),
            namespace: "example".into(),
            role: "owner".into(),
        };
        let mut page = DeletionBlockerPage {
            cloud_roles: vec!["admin".into()],
            memberships: vec![member.clone()],
            next_after: None,
        };
        assert!(page.validate(None).is_ok());
        page.memberships.push(member);
        assert!(page.validate(None).is_err());
        page.memberships.pop();
        page.next_after = Some(Uuid::from_u128(1));
        assert!(page.validate(None).is_err());
        page.next_after = None;
        page.cloud_roles.push("admin".into());
        assert!(page.validate(None).is_err());
        page.cloud_roles.clear();
        assert!(page.validate(Some(Uuid::from_u128(1))).is_err());
    }

    #[test]
    fn rate_limit_retry_delay_is_bounded_and_unambiguous() {
        let mut headers = reqwest::header::HeaderMap::new();
        assert_eq!(retry_delay(&headers), None);
        headers.insert(reqwest::header::RETRY_AFTER, "60".parse().unwrap());
        assert_eq!(retry_delay(&headers), Some(60));
        headers.insert(reqwest::header::RETRY_AFTER, "86401".parse().unwrap());
        assert_eq!(retry_delay(&headers), None);
        headers.insert(reqwest::header::RETRY_AFTER, "0".parse().unwrap());
        assert_eq!(retry_delay(&headers), None);
        headers.append(reqwest::header::RETRY_AFTER, "60".parse().unwrap());
        assert_eq!(retry_delay(&headers), None);
    }

    #[test]
    fn sync_export_rejects_inconsistent_or_unrecognized_records() {
        let good = Record {
            collection: COLLECTION.into(),
            key: "acme.plugin".into(),
            revision: 1,
            deleted: true,
            value: None,
        };
        let mut export = SyncExport {
            schema_version: 1,
            cursor: 1,
            records: vec![good.clone()],
        };
        assert!(export.validate().is_ok());
        export.records.push(good);
        assert!(matches!(export.validate(), Err(SyncError::Protocol)));
        export.records.pop();
        export.cursor = 0;
        assert!(matches!(export.validate(), Err(SyncError::Protocol)));
        export.cursor = 1;
        export.records[0].collection = "other".into();
        assert!(matches!(export.validate(), Err(SyncError::Protocol)));
        assert!(serde_json::from_str::<SyncExport>(
            r#"{"schemaVersion":1,"cursor":0,"records":[],"token":"secret"}"#
        )
        .is_err());
    }

    #[test]
    fn account_export_is_account_bound_bounded_and_secret_free() {
        let raw = serde_json::json!({
            "schemaVersion": 1,
            "profile": {"id":"user-1","email":"user@example.test","name":"User"},
            "cloudRoles": ["moderator"],
            "publisherMemberships": [{"namespace":"acme","role":"owner"}],
            "publisherKeysRequested": [],
            "releasesSubmitted": [],
            "uploadsRequested": [],
            "reviewsRecorded": [],
            "publicationRequests": [],
            "ownershipTransfers": [],
            "auditEventsByActor": [],
            "yanksRecorded": [],
            "advisoriesRecorded": [],
            "sync": {"schemaVersion":1,"cursor":0,"records":[]}
        });
        let mut export: AccountExport = serde_json::from_value(raw).unwrap();
        assert!(export.validate("user-1").is_ok());
        assert!(matches!(
            export.validate("user-2"),
            Err(SyncError::WrongAccount)
        ));
        export.publisher_memberships[0]["sessionToken"] = serde_json::json!("secret");
        assert!(matches!(
            export.validate("user-1"),
            Err(SyncError::Protocol)
        ));
        export.publisher_memberships.clear();
        export.cloud_roles.push("owner".into());
        assert!(matches!(
            export.validate("user-1"),
            Err(SyncError::Protocol)
        ));
    }

    #[test]
    fn offline_edits_require_explicit_resolution() {
        let mut state = SyncState::new("https://cloud.example".into(), "user".into());
        state.stage("acme.plugin".into(), Edit::Delete).unwrap();
        assert!(state.entries["acme.plugin"].pending.is_some());
        assert!(state
            .stage(
                "acme.plugin".into(),
                Edit::Put(PluginSelection {
                    schema_version: 1,
                    plugin_id: "acme.plugin".into(),
                    version: "not-a-version".into(),
                    enabled: true
                })
            )
            .is_err());
        assert!(state.entries["acme.plugin"].pending.is_some());
        state.discard("acme.plugin").unwrap();
        assert!(state.entries["acme.plugin"].pending.is_none());
    }
}
