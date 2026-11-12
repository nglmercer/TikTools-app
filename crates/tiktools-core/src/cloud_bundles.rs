//! Cloud assets, overlays and packages (v3). The Cloud stores and checks them.
//! A package is never applied by the Cloud: the host reads its preflight and
//! applies each item through the existing install and import flows.

use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{Method, StatusCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tiktools_cloud_auth::{CloudAuth, NativeToken};

use crate::{
    cloud_http::{self, decode, FailureEnvelope, Limits, NoContent},
    cloud_teams::valid_uuid,
    control::OperationError,
};

/// HTTP budget for overlay and package operations: 15 s, 4 MiB of reply.
const DOCUMENT_LIMITS: Limits = Limits::new(Duration::from_secs(15), 4 * 1024 * 1024);
/// HTTP budget for an owner library page: 15 s, 768 KiB of reply (the page also
/// has to fit the RPC frame it is returned in).
const LIBRARY_LIMITS: Limits = Limits::new(Duration::from_secs(15), 768 * 1024);
/// HTTP budget for an asset upload: 15 s; the reply is a small metadata record.
const ASSET_LIMITS: Limits = Limits::new(Duration::from_secs(15), 64 * 1024);
/// Input limits (what the caller may send).
const MAX_ASSET_BYTES: usize = 512 * 1024;
const MAX_DOCUMENT_BYTES: usize = 128 * 1024;
const MAX_TITLE_CHARS: usize = 120;
const MAX_LABEL_CHARS: usize = 120;
const KINDS: &[&str] = &["plugin", "template", "profile", "overlay"];

// ---- Views -------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudAsset {
    pub id: String,
    pub sha256: String,
    pub mime: String,
    pub width: i64,
    pub height: i64,
    pub size: i64,
    /// RFC 3339. Absent on a Cloud that predates asset listing.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Owner-chosen name, 1 to 120 characters. Absent when none is set.
    #[serde(default)]
    pub label: Option<String>,
}

/// One page of the owner's assets, newest first.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudAssetPage {
    pub items: Vec<CloudAsset>,
    pub next_cursor: Option<String>,
}

/// Acknowledgement of a Cloud delete (`{"id", "deleted": true}`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudDeleted {
    pub id: String,
    pub deleted: bool,
}

/// Overlay list entry. It never carries the document.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySummary {
    pub id: String,
    pub kind: String,
    /// The document title (the overlay schema has no separate name).
    pub name: String,
    /// Current revision.
    pub revision: i64,
    pub status: String,
    pub created_at: String,
    /// When the current revision was written.
    pub updated_at: String,
    pub element_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlayPage {
    pub items: Vec<OverlaySummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlayRevisionSummary {
    pub revision: i64,
    pub status: String,
    pub name: String,
    pub sha256: String,
    pub created_at: String,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlayRevisionPage {
    pub id: String,
    pub current_revision: i64,
    pub items: Vec<OverlayRevisionSummary>,
    pub next_cursor: Option<String>,
}

/// One stored overlay revision with its document.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlayRevisionRecord {
    pub id: String,
    pub revision: i64,
    pub status: String,
    pub schema_version: i64,
    pub sha256: String,
    pub created_at: String,
    pub current: bool,
    pub document: Value,
}

/// Where a forked document came from (row metadata, not part of the document).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogForkOrigin {
    pub document_id: String,
    pub revision: i64,
}

/// A private draft created by forking a public template or profile.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogForkResult {
    pub id: String,
    pub revision: i64,
    pub status: String,
    pub fork_of: CatalogForkOrigin,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlayRecord {
    pub id: String,
    pub revision: i64,
    pub status: String,
    pub schema_version: i64,
    pub document: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlayWriteResult {
    pub id: String,
    pub revision: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackageRecord {
    pub id: String,
    pub revision: i64,
    pub document: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackageCreated {
    pub id: String,
    pub revision: i64,
    pub status: String,
    pub package_version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreflightItem {
    pub kind: String,
    pub id: String,
    pub version: Option<String>,
    pub revision: Option<i64>,
    pub required: bool,
    /// `ready`, `missing`, `notInstallable`, `revisionChanged` or `noActiveGeneration`.
    pub status: String,
    /// `reviewRequired` for plugins (the host consent step shows them) or `none`.
    pub permissions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PackagePreflight {
    pub package_version: i64,
    pub items: Vec<PreflightItem>,
    pub blocking: Vec<Value>,
    pub can_apply: bool,
    pub apply_mode: String,
}

// ---- Parameters --------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetUploadParams {
    /// Standard base64 of raw PNG, JPEG or WebP bytes, at most 512 KiB decoded.
    pub data_base64: String,
}

/// Paging for the owner's assets and overlays. Cursors come only from the
/// previous page's `nextCursor`.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudPageQuery {
    pub cursor: Option<String>,
    /// 1 to 50; the Cloud default (50) when absent.
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetIdParams {
    pub id: String,
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetSetLabelParams {
    pub id: String,
    /// The key is required. `null` clears the label; otherwise 1 to 120
    /// characters after trimming.
    #[serde(deserialize_with = "required_option")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverlayRevisionsQuery {
    pub id: String,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverlayRevisionParams {
    pub id: String,
    /// Positive revision number.
    pub revision: i64,
}

/// Fork a document that is visible in the public catalog.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogForkParams {
    /// The public document id.
    pub id: String,
    /// A revision that is still published; the current one when absent.
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverlayCreateParams {
    pub document: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverlayIdParams {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverlayReviseParams {
    pub id: String,
    pub document: Value,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageItemParams {
    /// `plugin`, `template`, `profile` or `overlay`.
    pub kind: String,
    pub id: String,
    /// Required for plugins: the pinned semantic version.
    pub version: Option<String>,
    /// Required for templates, profiles and overlays: the pinned revision.
    pub revision: Option<i64>,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageCreateParams {
    pub title: String,
    pub items: Vec<PackageItemParams>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageIdParams {
    pub id: String,
}

// ---- Owner library (Míos) --------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryItemsQuery {
    #[serde(default)]
    pub query: String,
    /// `template` or `profile`. Plugins are not library documents.
    pub kind: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemPage {
    /// Same item shape as the public catalog, in any publication status.
    pub items: Vec<Value>,
    pub next_cursor: Option<String>,
}

/// Lists the signed-in owner's templates and profiles, in any status.
pub async fn list_library_items(
    auth: &CloudAuth,
    query: LibraryItemsQuery,
) -> Result<LibraryItemPage, OperationError> {
    if query.query.chars().count() > 100 || query.query.chars().any(char::is_control) {
        return Err(OperationError::invalid("search text is not valid"));
    }
    if let Some(kind) = &query.kind {
        if kind != "template" && kind != "profile" {
            return Err(OperationError::invalid(
                "library kind must be template or profile",
            ));
        }
    }
    check_cursor(query.cursor.as_deref())?;
    let limit = query.limit.unwrap_or(25);
    if !(1..=50).contains(&limit) {
        return Err(OperationError::invalid("limit must be between 1 and 50"));
    }
    let (origin, token) = credentials(auth).await?;
    let mut params: Vec<(&str, String)> = vec![("limit", limit.to_string())];
    if !query.query.is_empty() {
        params.push(("q", query.query.clone()));
    }
    if let Some(kind) = &query.kind {
        params.push(("kind", kind.clone()));
    }
    if let Some(cursor) = &query.cursor {
        params.push(("cursor", cursor.clone()));
    }
    let request = cloud_http::authorized(
        Method::GET,
        format!("{origin}/api/v3/library/items"),
        &token,
    )?
    .query(&params);
    let value =
        cloud_http::send_json(request, LIBRARY_LIMITS, NoContent::Unexpected, map_failure).await?;
    let items = value
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| OperationError::internal("cloud sent an unexpected library page"))?;
    let next_cursor = match value.get("nextCursor") {
        Some(Value::String(cursor)) => Some(cursor.clone()),
        Some(Value::Null) | None => None,
        _ => return Err(OperationError::internal("cloud sent an unexpected cursor")),
    };
    Ok(LibraryItemPage { items, next_cursor })
}

// ---- Validation --------------------------------------------------------------

/// The hyphenated lowercase form of an identifier, which is what goes in a path.
fn canonical_id(value: &str) -> Result<String, OperationError> {
    uuid::Uuid::parse_str(value)
        .map(|id| id.hyphenated().to_string())
        .map_err(|_| OperationError::invalid("identifier is not valid"))
}

/// Opaque cursors are base64url JSON; anything else is refused before it is sent.
fn check_cursor(cursor: Option<&str>) -> Result<(), OperationError> {
    if let Some(cursor) = cursor {
        if cursor.is_empty()
            || cursor.len() > 1024
            || !cursor
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_=".contains(&b))
        {
            return Err(OperationError::invalid("cursor is not valid"));
        }
    }
    Ok(())
}

/// Query pairs for a paged list, validated. Unset values are not sent.
fn page_params(
    cursor: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<(&'static str, String)>, OperationError> {
    check_cursor(cursor)?;
    let mut params = Vec::new();
    if let Some(limit) = limit {
        if !(1..=50).contains(&limit) {
            return Err(OperationError::invalid("limit must be between 1 and 50"));
        }
        params.push(("limit", limit.to_string()));
    }
    if let Some(cursor) = cursor {
        params.push(("cursor", cursor.to_owned()));
    }
    Ok(params)
}

/// Label rule of the Cloud: trimmed, 1 to 120 characters, no control
/// characters and no line or paragraph separators.
fn clean_label(label: &str) -> Result<String, OperationError> {
    let label = label.trim();
    let count = label.chars().count();
    if count == 0
        || count > MAX_LABEL_CHARS
        || label
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(OperationError::invalid(
            "the label must be 1 to 120 characters without line breaks",
        ));
    }
    Ok(label.to_owned())
}

fn bounded_document(value: &Value) -> Result<(), OperationError> {
    if !value.is_object() {
        return Err(OperationError::invalid("the document must be an object"));
    }
    let size = serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_DOCUMENT_BYTES {
        return Err(OperationError::invalid("the document is too large"));
    }
    Ok(())
}

fn validate_item(item: &PackageItemParams) -> Result<(), OperationError> {
    if !KINDS.contains(&item.kind.as_str()) {
        return Err(OperationError::invalid(
            "package item kind is not supported",
        ));
    }
    if item.kind == "plugin" {
        let version = item
            .version
            .as_deref()
            .ok_or_else(|| OperationError::invalid("a plugin item pins a version"))?;
        if version.is_empty() || version.len() > 64 || version.chars().any(char::is_control) {
            return Err(OperationError::invalid("plugin version is not valid"));
        }
        if item.revision.is_some() {
            return Err(OperationError::invalid(
                "a plugin item pins a version, not a revision",
            ));
        }
    } else {
        valid_uuid(&item.id)?;
        let revision = item
            .revision
            .ok_or_else(|| OperationError::invalid("this item pins a revision"))?;
        if revision < 1 {
            return Err(OperationError::invalid("revision must be positive"));
        }
        if item.version.is_some() {
            return Err(OperationError::invalid("only plugins pin a version"));
        }
    }
    Ok(())
}

async fn credentials(auth: &CloudAuth) -> Result<(String, NativeToken), OperationError> {
    cloud_http::credentials(auth, "this").await
}

/// Cloud failure wording for overlays, packages, assets and the owner library.
/// It names none of them in particular, so every operation here reads sensibly.
fn map_failure(status: StatusCode, body: &[u8]) -> OperationError {
    let envelope = FailureEnvelope::parse(body);
    let error = match envelope.code() {
        Some("notFound") => OperationError::not_found("the Cloud item was not found"),
        Some("conflict") => OperationError::conflict("the item changed; reload and try again"),
        Some("invalidRequest") => OperationError::invalid("the Cloud rejected the request"),
        Some("unauthorized" | "forbidden") => {
            OperationError::forbidden("sign in to Cloud again to continue")
        }
        Some("rateLimited") => OperationError::unavailable("too many requests; try again later"),
        Some("quotaExceeded") => quota_exceeded(),
        Some("misconfigured" | "unsupported") => {
            OperationError::unavailable("this Cloud instance does not offer that feature")
        }
        _ => match status.as_u16() {
            401 | 403 => OperationError::forbidden("sign in to Cloud again to continue"),
            404 => OperationError::not_found("the Cloud item was not found"),
            409 => OperationError::conflict("the item changed; reload and try again"),
            507 => quota_exceeded(),
            _ => OperationError::unavailable("the Cloud is unavailable"),
        },
    };
    envelope.attach(error)
}

/// An account limit (assets, overlays, revisions, library) is full.
fn quota_exceeded() -> OperationError {
    OperationError {
        code: "quota_exceeded",
        message: "the Cloud storage limit for this account is reached; delete something first"
            .into(),
        data: None,
    }
}

/// What a `409` means for one delete, so the UI can explain it.
#[derive(Clone, Copy)]
enum Blocked {
    /// An overlay revision still names the asset.
    AssetReferenced,
    /// A package still pins the overlay.
    OverlayPinned,
}

fn map_blocked(status: StatusCode, body: &[u8], blocked: Blocked) -> OperationError {
    let envelope = FailureEnvelope::parse(body);
    if status.as_u16() != 409 && envelope.code() != Some("conflict") {
        return map_failure(status, body);
    }
    let (reason, message) = match blocked {
        Blocked::AssetReferenced => (
            "assetReferenced",
            "an overlay still uses this image; remove it from the overlay first",
        ),
        Blocked::OverlayPinned => (
            "overlayPinned",
            "a package still pins this overlay; delete the package first",
        ),
    };
    let mut error = envelope.attach(OperationError::conflict(message));
    let mut data = error.data.take().unwrap_or_else(|| json!({}));
    data["reason"] = json!(reason);
    error.data = Some(data);
    error
}

fn map_asset_delete_failure(status: StatusCode, body: &[u8]) -> OperationError {
    map_blocked(status, body, Blocked::AssetReferenced)
}

fn map_overlay_delete_failure(status: StatusCode, body: &[u8]) -> OperationError {
    map_blocked(status, body, Blocked::OverlayPinned)
}

/// Fork answers with bare status codes (the v2 library routes).
fn map_fork_failure(status: StatusCode, body: &[u8]) -> OperationError {
    let envelope = FailureEnvelope::parse(body);
    let reason = |error: OperationError, reason: &str| {
        let mut error = envelope_attach(error, body);
        let mut data = error.data.take().unwrap_or_else(|| json!({}));
        data["reason"] = json!(reason);
        error.data = Some(data);
        error
    };
    match status.as_u16() {
        404 => reason(
            OperationError::not_found("that document is not published, or it no longer exists"),
            "notPublished",
        ),
        409 => reason(
            OperationError::conflict("your library already has a document with this id"),
            "alreadyInLibrary",
        ),
        507 => reason(quota_exceeded(), "libraryQuota"),
        429 => reason(
            OperationError::unavailable("too many requests; try again later"),
            "rateLimited",
        ),
        _ => envelope.attach(match status.as_u16() {
            400 => OperationError::invalid("the Cloud rejected the request"),
            401 | 403 => OperationError::forbidden("sign in to Cloud again to continue"),
            _ => OperationError::unavailable("the Cloud is unavailable"),
        }),
    }
}

fn envelope_attach(error: OperationError, body: &[u8]) -> OperationError {
    FailureEnvelope::parse(body).attach(error)
}

// ---- Capability gate -----------------------------------------------------------

/// A Cloud feature a host method depends on. The instance announces these in
/// `GET /api/v2/capabilities`; an absent key means "not available".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Feature {
    Media(&'static str),
    Overlays(&'static str),
    Packages(&'static str),
    Fork,
}

impl Feature {
    fn capability(self) -> &'static str {
        match self {
            Self::Media(_) => "media.operations",
            Self::Overlays(_) => "overlays.operations",
            Self::Packages(_) => "packages.operations",
            Self::Fork => "library.fork",
        }
    }

    fn operation(self) -> &'static str {
        match self {
            Self::Media(op) | Self::Overlays(op) | Self::Packages(op) => op,
            Self::Fork => "fork",
        }
    }

    fn announced_by(self, document: &Value) -> bool {
        let operations = |pointer: &str, op: &str| {
            document
                .pointer(pointer)
                .and_then(Value::as_array)
                .is_some_and(|list| list.iter().any(|entry| entry.as_str() == Some(op)))
        };
        match self {
            Self::Media(op) => operations("/media/operations", op),
            Self::Overlays(op) => operations("/overlays/operations", op),
            Self::Packages(op) => operations("/packages/operations", op),
            Self::Fork => document.pointer("/library/fork/available") == Some(&Value::Bool(true)),
        }
    }

    fn unavailable(self) -> OperationError {
        OperationError {
            code: "capability_unavailable",
            message: "this Cloud instance does not offer that feature".into(),
            data: Some(json!({
                "reason": "cloud_feature_unsupported",
                "capability": self.capability(),
                "operation": self.operation(),
            })),
        }
    }
}

/// 15 s and 64 KiB, like the SDK's own capability discovery.
const CAPABILITY_LIMITS: Limits = Limits::new(Duration::from_secs(15), 64 * 1024);

fn map_capability_failure(status: StatusCode, _body: &[u8]) -> OperationError {
    match status.as_u16() {
        // A Cloud without the capability document is an older backend.
        404 => OperationError {
            code: "capability_unavailable",
            message: "this Cloud instance does not announce its features".into(),
            data: Some(json!({"reason": "capabilities_missing"})),
        },
        _ => OperationError::unavailable("the Cloud features could not be checked"),
    }
}

/// Reads the anonymous capability document. No credential is sent.
async fn announced(origin: &str) -> Result<Value, OperationError> {
    let request = cloud_http::client()?.get(format!("{origin}/api/v2/capabilities"));
    cloud_http::send_json(
        request,
        CAPABILITY_LIMITS,
        NoContent::Unexpected,
        map_capability_failure,
    )
    .await
}

/// Credentials plus proof that the instance announced `feature`. When it did
/// not, no request for the feature is ever made.
async fn gated(
    auth: &CloudAuth,
    feature: Feature,
) -> Result<(String, NativeToken), OperationError> {
    let (origin, token) = credentials(auth).await?;
    if !feature.announced_by(&announced(&origin).await?) {
        return Err(feature.unavailable());
    }
    Ok((origin, token))
}

/// One gated request, never retried. Every reply is a document.
async fn gated_call(
    auth: &CloudAuth,
    feature: Feature,
    method: Method,
    path: String,
    query: &[(&str, String)],
    body: Option<Value>,
    map: fn(StatusCode, &[u8]) -> OperationError,
) -> Result<Value, OperationError> {
    let (origin, token) = gated(auth, feature).await?;
    let mut request = cloud_http::authorized(method, format!("{origin}{path}"), &token)?;
    if !query.is_empty() {
        request = request.query(query);
    }
    if let Some(body) = body {
        request = request.json(&body);
    }
    cloud_http::send_json(request, DOCUMENT_LIMITS, NoContent::Unexpected, map).await
}

/// One request, never retried. Every reply here is a document.
async fn call(
    auth: &CloudAuth,
    method: Method,
    path: String,
    body: Option<Value>,
) -> Result<Value, OperationError> {
    let (origin, token) = credentials(auth).await?;
    let mut request = cloud_http::authorized(method, format!("{origin}{path}"), &token)?;
    if let Some(body) = body {
        request = request.json(&body);
    }
    cloud_http::send_json(request, DOCUMENT_LIMITS, NoContent::Unexpected, map_failure).await
}

// ---- Assets ------------------------------------------------------------------

/// Uploads raw image bytes. The Cloud reads the format from the signature bytes.
pub async fn upload_asset(
    auth: &CloudAuth,
    params: AssetUploadParams,
) -> Result<CloudAsset, OperationError> {
    if params.data_base64.len() > MAX_ASSET_BYTES * 4 / 3 + 8 {
        return Err(OperationError::invalid("the image is larger than 512 KiB"));
    }
    let bytes = STANDARD
        .decode(params.data_base64.as_bytes())
        .map_err(|_| OperationError::invalid("the image data is not valid base64"))?;
    if bytes.is_empty() || bytes.len() > MAX_ASSET_BYTES {
        return Err(OperationError::invalid(
            "the image must be between 1 byte and 512 KiB",
        ));
    }
    let (origin, token) = credentials(auth).await?;
    let request = cloud_http::authorized(Method::POST, format!("{origin}/api/v3/assets"), &token)?
        .body(bytes);
    // Not retried: an upload whose outcome is unknown is reported, never resent.
    let reply = cloud_http::execute(request, ASSET_LIMITS).await?;
    if !reply.status.is_success() {
        return Err(OperationError::invalid("the Cloud refused this image"));
    }
    decode(cloud_http::success_json(&reply, NoContent::Unexpected)?)
}

// ---- Overlays ----------------------------------------------------------------

pub async fn create_overlay(
    auth: &CloudAuth,
    params: OverlayCreateParams,
) -> Result<OverlayWriteResult, OperationError> {
    bounded_document(&params.document)?;
    let value = call(
        auth,
        Method::POST,
        "/api/v3/overlays".into(),
        Some(json!({"document": params.document})),
    )
    .await?;
    decode(value)
}

pub async fn read_overlay(
    auth: &CloudAuth,
    params: OverlayIdParams,
) -> Result<OverlayRecord, OperationError> {
    valid_uuid(&params.id)?;
    let value = call(
        auth,
        Method::GET,
        format!("/api/v3/overlays/{}", params.id),
        None,
    )
    .await?;
    decode(value)
}

pub async fn revise_overlay(
    auth: &CloudAuth,
    params: OverlayReviseParams,
) -> Result<OverlayWriteResult, OperationError> {
    valid_uuid(&params.id)?;
    bounded_document(&params.document)?;
    if params.expected_revision < 1 {
        return Err(OperationError::invalid(
            "expected revision must be positive",
        ));
    }
    let value = call(
        auth,
        Method::POST,
        format!("/api/v3/overlays/{}/revisions", params.id),
        Some(json!({"document": params.document, "expectedRevision": params.expected_revision})),
    )
    .await?;
    decode(value)
}

// ---- Asset lifecycle ------------------------------------------------------------

pub async fn list_assets(
    auth: &CloudAuth,
    query: CloudPageQuery,
) -> Result<CloudAssetPage, OperationError> {
    let params = page_params(query.cursor.as_deref(), query.limit)?;
    let value = gated_call(
        auth,
        Feature::Media("list"),
        Method::GET,
        "/api/v3/assets".into(),
        &params,
        None,
        map_failure,
    )
    .await?;
    decode(value)
}

/// Renames an asset, or clears its label with `None`. Idempotent.
pub async fn set_asset_label(
    auth: &CloudAuth,
    params: AssetSetLabelParams,
) -> Result<CloudAsset, OperationError> {
    let id = canonical_id(&params.id)?;
    let label = params.label.as_deref().map(clean_label).transpose()?;
    let value = gated_call(
        auth,
        Feature::Media("rename"),
        Method::PATCH,
        format!("/api/v3/assets/{id}"),
        &[],
        Some(json!({ "label": label })),
        map_failure,
    )
    .await?;
    decode(value)
}

/// Deletes an asset. `conflict` (reason `assetReferenced`) while an overlay
/// revision still names it.
pub async fn delete_asset(
    auth: &CloudAuth,
    params: AssetIdParams,
) -> Result<CloudDeleted, OperationError> {
    let id = canonical_id(&params.id)?;
    let value = gated_call(
        auth,
        Feature::Media("delete"),
        Method::DELETE,
        format!("/api/v3/assets/{id}"),
        &[],
        None,
        map_asset_delete_failure,
    )
    .await?;
    deleted(value, &id)
}

/// The Cloud answers `{"id", "deleted": true}` for the id that was asked for.
fn deleted(value: Value, id: &str) -> Result<CloudDeleted, OperationError> {
    let ack: CloudDeleted = decode(value)?;
    if !ack.deleted || !ack.id.eq_ignore_ascii_case(id) {
        return Err(OperationError::internal(
            "cloud sent an unexpected delete reply",
        ));
    }
    Ok(ack)
}

// ---- Overlay history and delete ---------------------------------------------------

pub async fn list_overlays(
    auth: &CloudAuth,
    query: CloudPageQuery,
) -> Result<OverlayPage, OperationError> {
    let params = page_params(query.cursor.as_deref(), query.limit)?;
    let value = gated_call(
        auth,
        Feature::Overlays("list"),
        Method::GET,
        "/api/v3/overlays".into(),
        &params,
        None,
        map_failure,
    )
    .await?;
    decode(value)
}

pub async fn list_overlay_revisions(
    auth: &CloudAuth,
    query: OverlayRevisionsQuery,
) -> Result<OverlayRevisionPage, OperationError> {
    let id = canonical_id(&query.id)?;
    let params = page_params(query.cursor.as_deref(), query.limit)?;
    let value = gated_call(
        auth,
        Feature::Overlays("revisions"),
        Method::GET,
        format!("/api/v3/overlays/{}/revisions", id),
        &params,
        None,
        map_failure,
    )
    .await?;
    decode(value)
}

pub async fn read_overlay_revision(
    auth: &CloudAuth,
    params: OverlayRevisionParams,
) -> Result<OverlayRevisionRecord, OperationError> {
    let id = canonical_id(&params.id)?;
    if params.revision < 1 {
        return Err(OperationError::invalid("revision must be positive"));
    }
    let value = gated_call(
        auth,
        Feature::Overlays("revision"),
        Method::GET,
        format!("/api/v3/overlays/{}/revisions/{}", id, params.revision),
        &[],
        None,
        map_failure,
    )
    .await?;
    decode(value)
}

/// Deletes an overlay and every revision. `conflict` (reason `overlayPinned`)
/// while a package pins it.
pub async fn delete_overlay(
    auth: &CloudAuth,
    params: OverlayIdParams,
) -> Result<CloudDeleted, OperationError> {
    let id = canonical_id(&params.id)?;
    let value = gated_call(
        auth,
        Feature::Overlays("delete"),
        Method::DELETE,
        format!("/api/v3/overlays/{}", id),
        &[],
        None,
        map_overlay_delete_failure,
    )
    .await?;
    deleted(value, &id)
}

/// Deletes a package. A package is only a list of pins, so this also releases
/// the overlays it pinned.
pub async fn delete_package(
    auth: &CloudAuth,
    params: PackageIdParams,
) -> Result<CloudDeleted, OperationError> {
    let id = canonical_id(&params.id)?;
    let value = gated_call(
        auth,
        Feature::Packages("delete"),
        Method::DELETE,
        format!("/api/v3/packages/{}", id),
        &[],
        None,
        map_failure,
    )
    .await?;
    deleted(value, &id)
}

// ---- Fork -------------------------------------------------------------------------

/// Copies a public template or profile into the caller's library as a private
/// draft. Additive: nothing existing is changed.
pub async fn fork_catalog_document(
    auth: &CloudAuth,
    params: CatalogForkParams,
) -> Result<CatalogForkResult, OperationError> {
    let id = canonical_id(&params.id)?;
    if params.revision.is_some_and(|revision| revision < 1) {
        return Err(OperationError::invalid("revision must be positive"));
    }
    let body = params
        .revision
        .map(|revision| json!({ "revision": revision }));
    let value = gated_call(
        auth,
        Feature::Fork,
        Method::POST,
        format!("/api/v2/catalog/documents/{}/fork", id),
        &[],
        body,
        map_fork_failure,
    )
    .await?;
    let result: CatalogForkResult = decode(value)?;
    if result.status != "draft" || result.revision < 1 || result.fork_of.revision < 1 {
        return Err(OperationError::internal(
            "cloud sent an unexpected fork reply",
        ));
    }
    Ok(result)
}

// ---- Packages ----------------------------------------------------------------

pub async fn create_package(
    auth: &CloudAuth,
    params: PackageCreateParams,
) -> Result<PackageCreated, OperationError> {
    let title = params.title.trim();
    if title.is_empty()
        || title.chars().count() > MAX_TITLE_CHARS
        || title.chars().any(char::is_control)
    {
        return Err(OperationError::invalid(
            "package title must be 1 to 120 characters",
        ));
    }
    if params.items.is_empty() {
        return Err(OperationError::invalid("a package needs at least one item"));
    }
    for item in &params.items {
        validate_item(item)?;
    }
    let value = call(
        auth,
        Method::POST,
        "/api/v3/packages".into(),
        Some(json!({"title": title, "items": params.items})),
    )
    .await?;
    decode(value)
}

pub async fn read_package(
    auth: &CloudAuth,
    params: PackageIdParams,
) -> Result<PackageRecord, OperationError> {
    valid_uuid(&params.id)?;
    let value = call(
        auth,
        Method::GET,
        format!("/api/v3/packages/{}", params.id),
        None,
    )
    .await?;
    decode(value)
}

/// Read-only report of what applying the package would need. Nothing is applied here.
pub async fn preflight_package(
    auth: &CloudAuth,
    params: PackageIdParams,
) -> Result<PackagePreflight, OperationError> {
    valid_uuid(&params.id)?;
    let value = call(
        auth,
        Method::GET,
        format!("/api/v3/packages/{}/preflight", params.id),
        None,
    )
    .await?;
    let report: PackagePreflight = decode(value)?;
    if report.apply_mode != "clientJournal" {
        return Err(OperationError::internal(
            "cloud sent an unexpected apply mode",
        ));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: &str, version: Option<&str>, revision: Option<i64>) -> PackageItemParams {
        PackageItemParams {
            kind: kind.into(),
            id: "3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c".into(),
            version: version.map(str::to_owned),
            revision,
            required: true,
        }
    }

    #[test]
    fn plugins_pin_versions_and_documents_pin_revisions() {
        assert!(validate_item(&item("plugin", Some("1.2.0"), None)).is_ok());
        assert!(validate_item(&item("plugin", None, None)).is_err());
        assert!(validate_item(&item("plugin", Some("1.2.0"), Some(3))).is_err());
        assert!(validate_item(&item("overlay", None, Some(2))).is_ok());
        assert!(validate_item(&item("overlay", None, None)).is_err());
        assert!(validate_item(&item("overlay", None, Some(0))).is_err());
        assert!(validate_item(&item("script", None, Some(1))).is_err());
    }

    #[test]
    fn documents_must_be_objects_within_the_size_budget() {
        assert!(bounded_document(&json!({"overlayVersion": 1})).is_ok());
        assert!(bounded_document(&json!([1, 2])).is_err());
        let big = json!({"text": "x".repeat(MAX_DOCUMENT_BYTES)});
        assert!(bounded_document(&big).is_err());
    }

    #[test]
    fn overlay_package_and_library_failures_never_mention_teams() {
        for (status, body, code) in [
            (
                404,
                &br#"{"code":"notFound","requestId":"r-1"}"#[..],
                "not_found",
            ),
            (404, b"", "not_found"),
            (409, br#"{"code":"conflict"}"#, "conflict"),
            (403, br#"{"code":"forbidden"}"#, "forbidden"),
            (503, br#"{"code":"misconfigured"}"#, "unavailable"),
            (500, b"<html>", "unavailable"),
        ] {
            let error = map_failure(StatusCode::from_u16(status).unwrap(), body);
            assert_eq!(error.code, code);
            assert!(
                !error.message.to_lowercase().contains("team"),
                "{}",
                error.message
            );
        }
        let kept = map_failure(
            StatusCode::NOT_FOUND,
            br#"{"code":"notFound","requestId":"r-1","internal":"x"}"#,
        );
        assert_eq!(
            kept.data,
            Some(json!({"cloud": {"code": "notFound", "requestId": "r-1"}}))
        );
    }

    // ---- Capability gating and lifecycle, against a recording server ----------

    use std::sync::{Arc, Mutex};
    use tiktools_cloud_auth::{Persistence, TokenStore, TokenStoreError};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    struct Store;
    impl TokenStore for Store {
        fn persistence(&self) -> Persistence {
            Persistence::MemoryOnly
        }
        fn load(&self, _: &str, _: &str) -> Result<Option<secrecy::SecretString>, TokenStoreError> {
            Ok(Some(secrecy::SecretString::from("test-token".to_owned())))
        }
        fn store(
            &self,
            _: &str,
            _: &str,
            _: &cloud_client::Session,
        ) -> Result<(), TokenStoreError> {
            Ok(())
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), TokenStoreError> {
            Ok(())
        }
    }

    type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

    /// Serves `handler(method, target, body) -> (status, json)` and records
    /// every request as `(method, target, body)`.
    async fn serve<F>(handler: F) -> (CloudAuth, Seen, std::path::PathBuf)
    where
        F: Fn(&str, &str, &str) -> (u16, Value) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let seen: Seen = Arc::default();
        let log = seen.clone();
        let handler = Arc::new(handler);
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let (log, handler) = (log.clone(), handler.clone());
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let (head_end, length) = loop {
                        let mut buf = [0u8; 4096];
                        let Ok(n) = socket.read(&mut buf).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buf[..n]);
                        if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                            let head = String::from_utf8_lossy(&bytes[..at]).to_ascii_lowercase();
                            let length = head
                                .lines()
                                .find_map(|l| l.strip_prefix("content-length: "))
                                .and_then(|v| v.trim().parse::<usize>().ok())
                                .unwrap_or(0);
                            break (at + 4, length);
                        }
                    };
                    while bytes.len() < head_end + length {
                        let mut buf = [0u8; 4096];
                        match socket.read(&mut buf).await {
                            Ok(n) if n > 0 => bytes.extend_from_slice(&buf[..n]),
                            _ => return,
                        }
                    }
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let mut first = text.lines().next().unwrap_or_default().split_whitespace();
                    let method = first.next().unwrap_or_default().to_owned();
                    let target = first.next().unwrap_or_default().to_owned();
                    let body = String::from_utf8_lossy(&bytes[head_end..]).into_owned();
                    log.lock()
                        .unwrap()
                        .push((method.clone(), target.clone(), body.clone()));
                    let (status, value) = if method == "GET" && target == "/api/v1/native/me" {
                        (200, json!({"id": "alice"}))
                    } else {
                        handler(&method, &target, &body)
                    };
                    let payload = serde_json::to_vec(&value).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        payload.len()
                    );
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&payload).await;
                });
            }
        });
        let dir = std::env::temp_dir().join(format!(
            "bundles-gate-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let sidecar = dir.join("account.json");
        std::fs::write(
            &sidecar,
            serde_json::to_vec(&json!({"origin": origin, "account_id": "alice"})).unwrap(),
        )
        .unwrap();
        let auth = CloudAuth::new(Some(origin), sidecar, Arc::new(Store));
        auth.restore().await.unwrap();
        (auth, seen, dir)
    }

    const ID: &str = "3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c";

    fn capabilities() -> Value {
        json!({
            "media": {"operations": ["upload", "list", "read", "rename", "delete"]},
            "overlays": {"operations": ["create", "list", "read", "revise", "revisions", "revision", "delete"]},
            "packages": {"operations": ["create", "read", "preflight", "delete"]},
            "library": {"fork": {"available": true}},
        })
    }

    fn paths(seen: &Seen) -> Vec<String> {
        seen.lock()
            .unwrap()
            .iter()
            .map(|(m, t, _)| format!("{m} {t}"))
            .filter(|p| !p.contains("/api/v1/native/me"))
            .collect()
    }

    #[tokio::test]
    async fn an_unannounced_capability_never_reaches_the_feature_endpoint() {
        for document in [
            json!({}),
            json!({"media": {"operations": ["upload"]}}),
            json!({"media": {"operations": "list"}}),
            json!({"overlays": {"operations": ["list"]}, "library": {"fork": {"available": false}}}),
        ] {
            let doc = document.clone();
            let (auth, seen, dir) = serve(move |_, target, _| {
                if target == "/api/v2/capabilities" {
                    (200, doc.clone())
                } else {
                    (500, json!({}))
                }
            })
            .await;
            let fail = |error: OperationError, capability: &str| {
                assert_eq!(error.code, "capability_unavailable");
                assert_eq!(error.data.unwrap()["capability"], capability);
            };
            fail(
                list_assets(&auth, CloudPageQuery::default())
                    .await
                    .unwrap_err(),
                "media.operations",
            );
            fail(
                set_asset_label(
                    &auth,
                    AssetSetLabelParams {
                        id: ID.into(),
                        label: None,
                    },
                )
                .await
                .unwrap_err(),
                "media.operations",
            );
            fail(
                delete_asset(&auth, AssetIdParams { id: ID.into() })
                    .await
                    .unwrap_err(),
                "media.operations",
            );
            if document.pointer("/overlays/operations/0").is_none() {
                fail(
                    list_overlays(&auth, CloudPageQuery::default())
                        .await
                        .unwrap_err(),
                    "overlays.operations",
                );
            }
            fail(
                delete_package(&auth, PackageIdParams { id: ID.into() })
                    .await
                    .unwrap_err(),
                "packages.operations",
            );
            fail(
                fork_catalog_document(
                    &auth,
                    CatalogForkParams {
                        id: ID.into(),
                        revision: None,
                    },
                )
                .await
                .unwrap_err(),
                "library.fork",
            );
            assert!(
                paths(&seen).iter().all(|p| p == "GET /api/v2/capabilities"),
                "{:?}",
                paths(&seen)
            );
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[tokio::test]
    async fn an_instance_without_a_capability_document_is_an_older_backend() {
        let (auth, seen, dir) = serve(|_, _, _| (404, json!({}))).await;
        let error = list_assets(&auth, CloudPageQuery::default())
            .await
            .unwrap_err();
        assert_eq!(error.code, "capability_unavailable");
        assert_eq!(error.data.unwrap()["reason"], "capabilities_missing");
        assert_eq!(paths(&seen), vec!["GET /api/v2/capabilities"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn invalid_input_is_refused_before_any_network_use() {
        let (auth, seen, dir) = serve(|_, _, _| (500, json!({}))).await;
        let bad = |e: OperationError| assert_eq!(e.code, "invalid_params");
        bad(list_assets(
            &auth,
            CloudPageQuery {
                cursor: Some("a b".into()),
                limit: None,
            },
        )
        .await
        .unwrap_err());
        bad(list_assets(
            &auth,
            CloudPageQuery {
                cursor: None,
                limit: Some(51),
            },
        )
        .await
        .unwrap_err());
        bad(list_overlays(
            &auth,
            CloudPageQuery {
                cursor: None,
                limit: Some(0),
            },
        )
        .await
        .unwrap_err());
        bad(delete_asset(&auth, AssetIdParams { id: "../x".into() })
            .await
            .unwrap_err());
        for label in ["", "   ", "a\nb", "x\u{2028}y", &"l".repeat(121)] {
            bad(set_asset_label(
                &auth,
                AssetSetLabelParams {
                    id: ID.into(),
                    label: Some(label.into()),
                },
            )
            .await
            .unwrap_err());
        }
        bad(read_overlay_revision(
            &auth,
            OverlayRevisionParams {
                id: ID.into(),
                revision: 0,
            },
        )
        .await
        .unwrap_err());
        bad(fork_catalog_document(
            &auth,
            CatalogForkParams {
                id: ID.into(),
                revision: Some(0),
            },
        )
        .await
        .unwrap_err());
        assert!(paths(&seen).is_empty(), "{:?}", paths(&seen));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn announced_operations_send_the_documented_requests() {
        let (auth, seen, dir) = serve(|method, target, body| match (method, target) {
            (_, "/api/v2/capabilities") => (200, capabilities()),
            ("GET", t) if t.starts_with("/api/v3/assets?") => (
                200,
                json!({"items":[{"id":ID,"sha256":"a".repeat(64),"mime":"image/png","width":1,"height":1,"size":9,"createdAt":"2026-11-11T00:00:00Z","label":"Logo"}],"nextCursor":"abc_-="}),
            ),
            ("PATCH", t) if t == format!("/api/v3/assets/{ID}") => {
                let label: Value = serde_json::from_str(body).unwrap();
                (200, json!({"id":ID,"sha256":"a".repeat(64),"mime":"image/png","width":1,"height":1,"size":9,"createdAt":"2026-11-11T00:00:00Z","label":label["label"]}))
            }
            ("GET", t) if t == "/api/v3/overlays" || t.starts_with("/api/v3/overlays?") => (
                200,
                json!({"items":[{"id":ID,"kind":"follow","name":"Hi","revision":2,"status":"draft","createdAt":"c","updatedAt":"u","elementCount":3}],"nextCursor":null}),
            ),
            ("GET", t) if t.starts_with(&format!("/api/v3/overlays/{ID}/revisions?")) => (
                200,
                json!({"id":ID,"currentRevision":2,"items":[{"revision":2,"status":"draft","name":"Hi","sha256":"b".repeat(64),"createdAt":"c","current":true}],"nextCursor":null}),
            ),
            ("GET", t) if t == format!("/api/v3/overlays/{ID}/revisions/2") => (
                200,
                json!({"id":ID,"revision":2,"status":"draft","schemaVersion":1,"sha256":"b".repeat(64),"createdAt":"c","current":true,"document":{"overlayVersion":1}}),
            ),
            ("DELETE", t) if t.starts_with("/api/v3/") => (200, json!({"id":ID,"deleted":true})),
            ("POST", t) if t == format!("/api/v2/catalog/documents/{ID}/fork") => (
                201,
                json!({"id":"11111111-1111-4111-8111-111111111111","revision":1,"status":"draft","forkOf":{"documentId":ID,"revision":4}}),
            ),
            _ => (404, json!({})),
        })
        .await;
        let page = list_assets(
            &auth,
            CloudPageQuery {
                cursor: Some("c-1_=".into()),
                limit: Some(10),
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items[0].label.as_deref(), Some("Logo"));
        assert_eq!(page.next_cursor.as_deref(), Some("abc_-="));
        let renamed = set_asset_label(
            &auth,
            AssetSetLabelParams {
                id: ID.to_uppercase(),
                label: Some("  New  ".into()),
            },
        )
        .await
        .unwrap();
        assert_eq!(renamed.label.as_deref(), Some("New"));
        let cleared = set_asset_label(
            &auth,
            AssetSetLabelParams {
                id: ID.into(),
                label: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(cleared.label, None);
        assert_eq!(
            list_overlays(&auth, CloudPageQuery::default())
                .await
                .unwrap()
                .items[0]
                .element_count,
            3
        );
        let revisions = list_overlay_revisions(
            &auth,
            OverlayRevisionsQuery {
                id: ID.into(),
                cursor: None,
                limit: Some(5),
            },
        )
        .await
        .unwrap();
        assert!(revisions.items[0].current);
        let record = read_overlay_revision(
            &auth,
            OverlayRevisionParams {
                id: ID.into(),
                revision: 2,
            },
        )
        .await
        .unwrap();
        assert_eq!(record.document["overlayVersion"], 1);
        assert!(
            delete_asset(&auth, AssetIdParams { id: ID.into() })
                .await
                .unwrap()
                .deleted
        );
        assert!(
            delete_overlay(&auth, OverlayIdParams { id: ID.into() })
                .await
                .unwrap()
                .deleted
        );
        assert!(
            delete_package(&auth, PackageIdParams { id: ID.into() })
                .await
                .unwrap()
                .deleted
        );
        let fork = fork_catalog_document(
            &auth,
            CatalogForkParams {
                id: ID.into(),
                revision: Some(4),
            },
        )
        .await
        .unwrap();
        assert_eq!(fork.fork_of.revision, 4);
        let fork_no_revision = fork_catalog_document(
            &auth,
            CatalogForkParams {
                id: ID.into(),
                revision: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(fork_no_revision.status, "draft");
        let log = seen.lock().unwrap().clone();
        let find = |m: &str, prefix: &str| {
            log.iter()
                .filter(|(rm, t, _)| rm == m && t.starts_with(prefix))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(
            find("GET", "/api/v3/assets?")[0].1,
            "/api/v3/assets?limit=10&cursor=c-1_%3D"
        );
        let patches = find("PATCH", "/api/v3/assets/");
        assert_eq!(
            patches[0].1,
            format!("/api/v3/assets/{ID}"),
            "ids are sent canonical"
        );
        assert_eq!(
            serde_json::from_str::<Value>(&patches[0].2).unwrap(),
            json!({"label": "New"})
        );
        assert_eq!(
            serde_json::from_str::<Value>(&patches[1].2).unwrap(),
            json!({"label": null})
        );
        let forks = find("POST", "/api/v2/catalog/documents/");
        assert_eq!(
            serde_json::from_str::<Value>(&forks[0].2).unwrap(),
            json!({"revision": 4})
        );
        assert_eq!(forks[1].2, "", "no revision sends an empty body");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn blocked_deletes_and_fork_failures_are_distinct_typed_errors() {
        let (auth, _seen, dir) = serve(|method, target, _| match (method, target) {
            (_, "/api/v2/capabilities") => (200, capabilities()),
            ("DELETE", t) if t.starts_with("/api/v3/assets/") => {
                (409, json!({"code":"conflict","requestId":"r-1"}))
            }
            ("DELETE", t) if t.starts_with("/api/v3/overlays/") => {
                (409, json!({"code":"conflict"}))
            }
            ("DELETE", t) if t.starts_with("/api/v3/packages/") => {
                (404, json!({"code":"notFound"}))
            }
            ("POST", t) if t.ends_with("/fork") && t.contains("22222222") => (409, json!({})),
            ("POST", t) if t.ends_with("/fork") && t.contains("33333333") => (507, json!({})),
            ("POST", t) if t.ends_with("/fork") && t.contains("44444444") => (429, json!({})),
            ("POST", t) if t.ends_with("/fork") => (404, json!({})),
            _ => (500, json!({})),
        })
        .await;
        let asset = delete_asset(&auth, AssetIdParams { id: ID.into() })
            .await
            .unwrap_err();
        assert_eq!(asset.code, "conflict");
        assert_eq!(asset.data.as_ref().unwrap()["reason"], "assetReferenced");
        assert_eq!(asset.data.unwrap()["cloud"]["requestId"], "r-1");
        let overlay = delete_overlay(&auth, OverlayIdParams { id: ID.into() })
            .await
            .unwrap_err();
        assert_eq!(overlay.code, "conflict");
        assert_eq!(overlay.data.unwrap()["reason"], "overlayPinned");
        assert_eq!(
            delete_package(&auth, PackageIdParams { id: ID.into() })
                .await
                .unwrap_err()
                .code,
            "not_found"
        );
        let fork = |id: &str| CatalogForkParams {
            id: id.into(),
            revision: None,
        };
        let missing = fork_catalog_document(&auth, fork(ID)).await.unwrap_err();
        assert_eq!(
            (missing.code, missing.data.unwrap()["reason"].clone()),
            ("not_found", json!("notPublished"))
        );
        let owned = fork_catalog_document(&auth, fork("22222222-2222-4222-8222-222222222222"))
            .await
            .unwrap_err();
        assert_eq!(
            (owned.code, owned.data.unwrap()["reason"].clone()),
            ("conflict", json!("alreadyInLibrary"))
        );
        let full = fork_catalog_document(&auth, fork("33333333-3333-4333-8333-333333333333"))
            .await
            .unwrap_err();
        assert_eq!(
            (full.code, full.data.unwrap()["reason"].clone()),
            ("quota_exceeded", json!("libraryQuota"))
        );
        let limited = fork_catalog_document(&auth, fork("44444444-4444-4444-8444-444444444444"))
            .await
            .unwrap_err();
        assert_eq!(
            (limited.code, limited.data.unwrap()["reason"].clone()),
            ("unavailable", json!("rateLimited"))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn quota_and_set_label_wire_shapes() {
        let error = map_failure(
            StatusCode::INSUFFICIENT_STORAGE,
            br#"{"code":"quotaExceeded"}"#,
        );
        assert_eq!(error.code, "quota_exceeded");
        // The label key is required: omitting it must not silently clear.
        assert!(serde_json::from_value::<AssetSetLabelParams>(json!({"id": ID})).is_err());
        assert!(
            serde_json::from_value::<AssetSetLabelParams>(json!({"id": ID, "label": null})).is_ok()
        );
    }
}
