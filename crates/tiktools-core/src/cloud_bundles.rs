//! Cloud assets, overlays and packages (v3). The Cloud stores and checks them.
//! A package is never applied by the Cloud: the host reads its preflight and
//! applies each item through the existing install and import flows.

use crate::{cloud_http::Limits, control::OperationError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

mod assets;
mod capabilities;
mod errors;
mod library;
mod overlays;
mod packages;
mod transport;
mod validation;

pub use assets::{delete_asset, list_assets, set_asset_label, upload_asset};
pub use library::{fork_catalog_document, list_library_items};
pub use overlays::{
    create_overlay, delete_overlay, list_overlay_revisions, list_overlays, read_overlay,
    read_overlay_revision, revise_overlay,
};
pub use packages::{create_package, delete_package, preflight_package, read_package};

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

#[cfg(test)]
mod tests;
