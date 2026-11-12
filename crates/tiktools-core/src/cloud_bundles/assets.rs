//! Bounded image upload and capability-gated asset lifecycle operations.
use super::OperationError;
use super::{
    capabilities::Feature,
    errors::{map_asset_delete_failure, map_failure},
    transport::{credentials, deleted, gated_call},
    validation::{canonical_id, clean_label, page_params},
};
use super::{
    AssetIdParams, AssetSetLabelParams, AssetUploadParams, CloudAsset, CloudAssetPage,
    CloudDeleted, CloudPageQuery, ASSET_LIMITS, MAX_ASSET_BYTES,
};
use crate::cloud_http::{self, decode, NoContent};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Method;
use serde_json::json;
use tiktools_cloud_auth::CloudAuth;

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
