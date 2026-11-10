//! Cloud assets, overlays and packages (v3). The Cloud stores and checks them.
//! A package is never applied by the Cloud: the host reads its preflight and
//! applies each item through the existing install and import flows.

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{header::AUTHORIZATION, Method};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tiktools_cloud_auth::CloudAuth;

use crate::{
    cloud_teams::{credentials, send, valid_uuid},
    control::OperationError,
};

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

async fn call(
    auth: &CloudAuth,
    method: Method,
    path: String,
    body: Option<Value>,
) -> Result<Value, OperationError> {
    let (origin, token) = credentials(auth).await?;
    send(method, &origin, &token, &path, body).await
}

fn decode<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, OperationError> {
    serde_json::from_value(value)
        .map_err(|_| OperationError::internal("cloud sent an unexpected reply"))
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
    let response = crate::cloud_teams::http()
        .post(format!("{origin}/api/v3/assets"))
        .header(AUTHORIZATION, format!("Bearer {}", token.expose()))
        .body(bytes)
        .send()
        .await
        .map_err(|_| OperationError::unavailable("cloud request failed"))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|_| OperationError::unavailable("cloud reply could not be read"))?;
    if !status.is_success() {
        return Err(OperationError::invalid("the Cloud refused this image"));
    }
    let value: Value = serde_json::from_slice(&body)
        .map_err(|_| OperationError::internal("cloud sent an unreadable reply"))?;
    decode(value)
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
}
