//! Client-local response checks; the backend-owned interchange contract stays
//! unchanged. Catalog metadata never supplies registry trust or executable URLs.
use super::*;
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn decode<T: DeserializeOwned>(value: &Value) -> Result<T, SyncError> {
    serde_json::from_value(value.clone()).map_err(|_| SyncError::Protocol)
}
fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(deserializer)
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
}
fn document_digest(value: &Value) -> Result<String, SyncError> {
    let encoded = serde_json::to_vec(value).map_err(|_| SyncError::Protocol)?;
    Ok(Sha256::digest(encoded)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
// Catalog releases use the manifest ID grammar. V2 workspace keys also retain
// previously accepted sync IDs for backward compatibility.
pub(super) fn valid_catalog_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=128).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[1..].iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}
pub(super) fn valid_target(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"_-".contains(&v))
}
pub(super) fn valid_document_query(kind: Option<&str>, query: Option<&str>) -> bool {
    kind.is_none_or(|v| matches!(v, "template" | "profile")) && query.is_none_or(|v| v.len() <= 100)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkspaceHead {
    pub cursor: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CollectionPage {
    records: Vec<WorkspaceRecord>,
    #[serde(deserialize_with = "required_option")]
    next_after: Option<String>,
}
pub(super) fn validate_collection(
    value: &Value,
    collection: &str,
    after: Option<&str>,
) -> Result<(), SyncError> {
    let page: CollectionPage = decode(value)?;
    if page.records.len() > 50 {
        return Err(SyncError::Protocol);
    }
    let mut previous = after;
    for row in &page.records {
        if row.collection != collection
            || !row.valid(row.revision)
            || previous.is_some_and(|old| row.key.as_str() <= old)
        {
            return Err(SyncError::Protocol);
        }
        previous = Some(&row.key);
    }
    if page
        .next_after
        .as_deref()
        .is_some_and(|next| page.records.len() != 50 || Some(next) != previous)
    {
        return Err(SyncError::Protocol);
    }
    Ok(())
}
/// Provenance of a forked document: backend metadata on the caller's own
/// documents, never part of the portable value and never on public ones.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ForkOf {
    document_id: Uuid,
    revision: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentMeta {
    id: Uuid,
    kind: String,
    slug: String,
    revision: i64,
    title: String,
    sha256: String,
    status: String,
    #[serde(default, rename = "forkOf")]
    fork_of: Option<ForkOf>,
}
impl DocumentMeta {
    fn valid(&self, public: bool) -> bool {
        self.fork_of.as_ref().is_none_or(|source| {
            !public && (1..=100).contains(&source.revision) && !source.document_id.is_nil()
        }) && matches!(self.kind.as_str(), "template" | "profile")
            && portable::valid_id(&self.slug)
            && (1..=100).contains(&self.revision)
            && !self.title.trim().is_empty()
            && self.title.len() <= 120
            && hash(&self.sha256)
            && if public {
                self.status == "published"
            } else {
                matches!(
                    self.status.as_str(),
                    "draft" | "pending" | "published" | "rejected" | "withdrawn"
                )
            }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Documents {
    items: Vec<DocumentMeta>,
    #[serde(deserialize_with = "required_option")]
    next_cursor: Option<Uuid>,
}
pub(super) fn validate_documents(
    value: &Value,
    after: Option<Uuid>,
    kind: Option<&str>,
    public: bool,
) -> Result<(), SyncError> {
    let page: Documents = decode(value)?;
    if page.items.len() > 50 {
        return Err(SyncError::Protocol);
    }
    let mut previous = after;
    for item in &page.items {
        if !item.valid(public)
            || previous.is_some_and(|old| item.id <= old)
            || kind.is_some_and(|kind| item.kind != kind)
        {
            return Err(SyncError::Protocol);
        }
        previous = Some(item.id);
    }
    if page
        .next_cursor
        .is_some_and(|next| page.items.len() != 50 || Some(next) != previous)
    {
        return Err(SyncError::Protocol);
    }
    Ok(())
}
pub(super) fn validate_document(
    value: &Value,
    id: Uuid,
    revision: Option<i64>,
    public: bool,
) -> Result<(), SyncError> {
    let mut fields = value.as_object().cloned().ok_or(SyncError::Protocol)?;
    let document = fields.remove("document").ok_or(SyncError::Protocol)?;
    let meta: DocumentMeta = decode(&Value::Object(fields))?;
    if !meta.valid(public)
        || meta.id != id
        || revision.is_some_and(|revision| meta.revision != revision)
        || !portable::document(&meta.kind, &document)
        || document["id"] != meta.slug
        // Match the backend library digest algorithm, independently of the
        // registry's signature/root trust and the local Owner hash format.
        || meta.sha256 != document_digest(&document)?
    {
        return Err(SyncError::Protocol);
    }
    let title = if meta.kind == "profile" {
        &document["name"]
    } else {
        &document["title"]
    };
    if title.as_str().or_else(|| title["default"].as_str()) != Some(meta.title.as_str()) {
        return Err(SyncError::Protocol);
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryExport {
    items: Vec<Value>,
}
pub(super) fn validate_library_export(value: &Value) -> Result<(), SyncError> {
    let export: LibraryExport = decode(value)?;
    if export.items.len() > 10_000 {
        return Err(SyncError::Protocol);
    }
    let mut previous = None;
    for value in &export.items {
        let id: Uuid = decode(&value["id"])?;
        let revision = value["revision"].as_i64().ok_or(SyncError::Protocol)?;
        if previous.is_some_and(|old| old >= (id, revision)) {
            return Err(SyncError::Protocol);
        }
        validate_document(value, id, Some(revision), false)?;
        previous = Some((id, revision));
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Versions {
    items: Vec<DocumentMeta>,
    #[serde(deserialize_with = "required_option")]
    next_cursor: Option<i64>,
}
pub(super) fn validate_versions(value: &Value, id: Uuid, after: i64) -> Result<(), SyncError> {
    let page: Versions = decode(value)?;
    if page.items.len() > 50 {
        return Err(SyncError::Protocol);
    }
    let mut previous = after;
    let mut identity = None;
    for item in &page.items {
        let descriptor = (&item.kind, &item.slug);
        if !item.valid(false)
            || item.id != id
            || item.revision <= previous
            || identity.is_some_and(|old| old != descriptor)
        {
            return Err(SyncError::Protocol);
        }
        identity = Some(descriptor);
        previous = item.revision;
    }
    if page
        .next_cursor
        .is_some_and(|next| page.items.len() != 50 || next != previous)
    {
        return Err(SyncError::Protocol);
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryAck {
    id: Uuid,
    revision: i64,
    status: String,
}
pub(super) fn validate_library_ack(
    value: &Value,
    id: Option<Uuid>,
    revision: i64,
) -> Result<(), SyncError> {
    let ack: LibraryAck = decode(value)?;
    if id.is_some_and(|id| ack.id != id)
        || ack.revision != revision
        || !(1..=100).contains(&ack.revision)
        || ack.status != "draft"
    {
        return Err(SyncError::Protocol);
    }
    Ok(())
}
pub(super) async fn expect_no_content(response: reqwest::Response) -> Result<(), SyncError> {
    if response.status() == StatusCode::NO_CONTENT {
        return Ok(());
    }
    decode_with_limit::<Value>(response, MAX_V2_RESPONSE).await?;
    Err(SyncError::Protocol)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Advisory {
    severity: String,
    #[serde(deserialize_with = "required_option")]
    message: Option<String>,
    #[serde(deserialize_with = "required_option")]
    url: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PluginVersion {
    version: String,
    target: String,
    sha256: String,
    size: i64,
    yanked: bool,
    key_revoked: bool,
    publisher_approved: bool,
    #[serde(deserialize_with = "required_option")]
    advisory: Option<Advisory>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Plugin {
    id: String,
    #[serde(deserialize_with = "required_option")]
    latest_stable_version: Option<String>,
    metadata_path: String,
    versions: Vec<PluginVersion>,
}
impl Plugin {
    fn valid(&self, generation: i64, target: Option<&str>) -> bool {
        if !valid_catalog_id(&self.id) || self.versions.is_empty() {
            return false;
        }
        let prefix = format!("/api/v1/registry/generations/{generation}/files/");
        let Some(path) = self.metadata_path.strip_prefix(&prefix) else {
            return false;
        };
        let chars: Vec<_> = self.id.chars().collect();
        let shard = |start| {
            (start..start + 2)
                .map(|index| chars.get(index).copied().unwrap_or('_'))
                .collect::<String>()
        };
        // Never accept a literal parent segment; use the canonical index
        // location for the legacy shard whose second component is `..`.
        let expected = if shard(2) == ".." {
            format!("index/{}.json", self.id)
        } else {
            format!("index/{}/{}/{}.json", shard(0), shard(2), self.id)
        };
        if path != expected {
            return false;
        }
        let mut previous = None;
        let mut stable = None;
        let mut seen = BTreeSet::new();
        for row in &self.versions {
            let Ok(version) = semver::Version::parse(&row.version) else {
                return false;
            };
            if row.version.len() > 128
                || !valid_target(&row.target)
                || !hash(&row.sha256)
                || row.size < 1
                || target.is_some_and(|target| row.target != target && row.target != "any")
                || !seen.insert((&row.version, &row.target))
                || previous.as_ref().is_some_and(|old| old < &version)
            {
                return false;
            }
            if let Some(advisory) = &row.advisory {
                if !matches!(
                    advisory.severity.as_str(),
                    "low" | "moderate" | "high" | "critical"
                ) || advisory.message.as_ref().is_none_or(|m| {
                    m.trim().is_empty() || m.len() > 1024 || m.chars().any(char::is_control)
                }) || advisory.url.as_ref().is_some_and(|u| {
                    url::Url::parse(u).map_or(true, |url| {
                        url.scheme() != "https"
                            || url.host().is_none()
                            || !url.username().is_empty()
                            || url.password().is_some()
                            || url.fragment().is_some()
                    })
                }) {
                    return false;
                }
            }
            if stable.is_none()
                && !row.yanked
                && !row.key_revoked
                && row.publisher_approved
                && version.pre.is_empty()
            {
                stable = Some(row.version.as_str());
            }
            previous = Some(version);
        }
        self.latest_stable_version.as_deref() == stable
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Plugins {
    #[serde(deserialize_with = "required_option")]
    generation: Option<i64>,
    items: Vec<Plugin>,
    #[serde(deserialize_with = "required_option")]
    next_cursor: Option<String>,
}
pub(super) fn validate_plugins(
    value: &Value,
    after: Option<&str>,
    generation: Option<i64>,
    target: Option<&str>,
) -> Result<(), SyncError> {
    let page: Plugins = decode(value)?;
    let Some(actual) = page.generation else {
        return if generation.is_none() && page.items.is_empty() && page.next_cursor.is_none() {
            Ok(())
        } else {
            Err(SyncError::Protocol)
        };
    };
    if actual < 1 || generation.is_some_and(|expected| expected != actual) || page.items.len() > 50
    {
        return Err(SyncError::Protocol);
    }
    let mut previous = after;
    for item in &page.items {
        if !item.valid(actual, target) || previous.is_some_and(|old| item.id.as_str() <= old) {
            return Err(SyncError::Protocol);
        }
        previous = Some(&item.id);
    }
    if page
        .next_cursor
        .as_deref()
        .is_some_and(|next| page.items.len() != 50 || Some(next) != previous)
    {
        return Err(SyncError::Protocol);
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginDetail {
    generation: i64,
    plugin: Plugin,
}
pub(super) fn validate_plugin(
    value: &Value,
    id: &str,
    generation: Option<i64>,
    target: Option<&str>,
) -> Result<(), SyncError> {
    let detail: PluginDetail = decode(value)?;
    if detail.generation < 1
        || generation.is_some_and(|expected| expected != detail.generation)
        || detail.plugin.id != id
        || !detail.plugin.valid(detail.generation, target)
    {
        return Err(SyncError::Protocol);
    }
    Ok(())
}
