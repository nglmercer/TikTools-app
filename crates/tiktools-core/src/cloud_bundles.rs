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
    if let Some(cursor) = &query.cursor {
        if cursor.is_empty()
            || cursor.len() > 1024
            || !cursor
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_=".contains(&b))
        {
            return Err(OperationError::invalid("cursor is not valid"));
        }
    }
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
        Some("misconfigured" | "unsupported") => {
            OperationError::unavailable("this Cloud instance does not offer that feature")
        }
        _ => match status.as_u16() {
            401 | 403 => OperationError::forbidden("sign in to Cloud again to continue"),
            404 => OperationError::not_found("the Cloud item was not found"),
            409 => OperationError::conflict("the item changed; reload and try again"),
            _ => OperationError::unavailable("the Cloud is unavailable"),
        },
    };
    envelope.attach(error)
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
}
