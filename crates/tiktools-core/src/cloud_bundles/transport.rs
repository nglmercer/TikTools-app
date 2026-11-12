//! Credential acquisition and single-attempt document requests/acknowledgements.
use super::OperationError;
use super::{
    capabilities::{announced, Feature},
    errors::map_failure,
    CloudDeleted, DOCUMENT_LIMITS,
};
use crate::cloud_http::{self, decode, NoContent};
use reqwest::{Method, StatusCode};
use serde_json::Value;
use tiktools_cloud_auth::CloudAuth;
use tiktools_cloud_auth::NativeToken;

pub(super) async fn credentials(auth: &CloudAuth) -> Result<(String, NativeToken), OperationError> {
    cloud_http::credentials(auth, "this").await
}

/// Credentials plus proof that the instance announced `feature`. When it did
/// not, no request for the feature is ever made.
pub(super) async fn gated(
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
pub(super) async fn gated_call(
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
pub(super) async fn call(
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

/// The Cloud answers `{"id", "deleted": true}` for the id that was asked for.
pub(super) fn deleted(value: Value, id: &str) -> Result<CloudDeleted, OperationError> {
    let ack: CloudDeleted = decode(value)?;
    if !ack.deleted || !ack.id.eq_ignore_ascii_case(id) {
        return Err(OperationError::internal(
            "cloud sent an unexpected delete reply",
        ));
    }
    Ok(ack)
}
