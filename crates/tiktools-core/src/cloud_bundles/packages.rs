//! Pinned package creation, inspection, preflight and deletion.
use super::OperationError;
use super::{
    capabilities::Feature,
    errors::map_failure,
    transport::{call, deleted, gated_call},
    validation::{canonical_id, validate_item},
};
use super::{
    CloudDeleted, PackageCreateParams, PackageCreated, PackageIdParams, PackagePreflight,
    PackageRecord, MAX_TITLE_CHARS,
};
use crate::{cloud_http::decode, cloud_teams::valid_uuid};
use reqwest::Method;
use serde_json::json;
use tiktools_cloud_auth::CloudAuth;

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
