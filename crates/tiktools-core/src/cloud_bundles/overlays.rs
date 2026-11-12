//! Overlay documents, revision history and guarded deletion requests.
use super::OperationError;
use super::{
    capabilities::Feature,
    errors::{map_failure, map_overlay_delete_failure},
    transport::{call, deleted, gated_call},
    validation::{bounded_document, canonical_id, page_params},
};
use super::{
    CloudDeleted, CloudPageQuery, OverlayCreateParams, OverlayIdParams, OverlayPage, OverlayRecord,
    OverlayReviseParams, OverlayRevisionPage, OverlayRevisionParams, OverlayRevisionRecord,
    OverlayRevisionsQuery, OverlayWriteResult,
};
use crate::{cloud_http::decode, cloud_teams::valid_uuid};
use reqwest::Method;
use serde_json::json;
use tiktools_cloud_auth::CloudAuth;

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
