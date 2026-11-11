//! Cloud integrations (v3 webhook provider): list, connect, events and disconnect.
//!
//! The webhook secret is returned by `connect` exactly once. It is passed to the
//! caller and never stored, logged or echoed by the host. Responses are bounded so
//! that an events page always fits the remote frame limit.

use std::time::Duration;

use reqwest::{Method, StatusCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tiktools_cloud_auth::{CloudAuth, NativeToken};

use crate::{
    cloud_http::{self, decode, FailureEnvelope, Limits, NoContent},
    control::OperationError,
};

/// HTTP budget for every integrations operation: 15 s, 8 MiB of reply.
const HTTP_LIMITS: Limits = Limits::new(Duration::from_secs(15), 8 * 1024 * 1024);
/// RPC budget, separate from the HTTP limit: an events reply handed back to the
/// caller stays well under the 1 MiB remote frame limit.
const RPC_EVENT_REPLY_BYTES: usize = 512 * 1024;
/// Input limit for connection labels.
const MAX_LABEL_CHARS: usize = 120;
const SECRET_HEX_CHARS: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationConnection {
    pub id: String,
    pub provider: String,
    pub label: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationConnectionList {
    pub items: Vec<IntegrationConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegrationConnectParams {
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegrationIdParams {
    pub id: String,
}

/// The only reply that carries a secret. Callers must show it once and drop it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationConnectResult {
    pub connection_id: String,
    pub provider: String,
    pub label: String,
    pub secret: String,
    pub delivery_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationEvent {
    pub delivery_id: String,
    pub received_at: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationEventList {
    pub items: Vec<IntegrationEvent>,
    /// True when older events were dropped to keep the reply within bounds.
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationDisconnectResult {
    pub id: String,
    pub status: String,
}

/// Connection identifiers are UUIDs issued by Cloud. Anything else never leaves the host.
pub fn valid_connection_id(value: &str) -> Result<(), OperationError> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| OperationError::invalid("integration connection id is not valid"))
}

/// Trims and bounds a label with the same rules the server applies.
pub fn normalize_label(value: &str) -> Result<String, OperationError> {
    let label = value.trim();
    if label.is_empty()
        || label.chars().count() > MAX_LABEL_CHARS
        || label.chars().any(char::is_control)
    {
        return Err(OperationError::invalid(
            "label must be 1 to 120 characters without control characters",
        ));
    }
    Ok(label.to_owned())
}

async fn credentials(auth: &CloudAuth) -> Result<(String, NativeToken), OperationError> {
    cloud_http::credentials(auth, "integrations").await
}

/// Maps a Cloud error envelope to a user-safe message. Raw bodies are never surfaced.
fn map_failure(status: StatusCode, body: &[u8]) -> OperationError {
    let envelope = FailureEnvelope::parse(body);
    let error = match envelope.code() {
        Some("misconfigured") => {
            OperationError::unavailable("integrations are not configured on this Cloud instance")
        }
        Some("unsupported") => {
            OperationError::unavailable("this Cloud instance does not offer that integration")
        }
        Some("notFound") => OperationError::not_found("integration connection not found"),
        Some("invalidRequest") => {
            OperationError::invalid("the Cloud rejected the integration request")
        }
        Some("unauthorized" | "forbidden") => {
            OperationError::forbidden("sign in to Cloud again to manage integrations")
        }
        Some("rateLimited") => {
            OperationError::unavailable("too many integration requests; try again later")
        }
        _ => match status.as_u16() {
            401 | 403 => OperationError::forbidden("sign in to Cloud again to manage integrations"),
            404 => OperationError::not_found("integration connection not found"),
            _ => OperationError::unavailable("cloud integrations are unavailable"),
        },
    };
    envelope.attach(error)
}

/// One request, never retried. Every integrations reply is a document, so a
/// `204` is unexpected.
async fn send(
    method: Method,
    origin: &str,
    token: &NativeToken,
    path: &str,
    body: Option<Value>,
) -> Result<Value, OperationError> {
    let mut request = cloud_http::authorized(method, format!("{origin}{path}"), token)?;
    if let Some(body) = body {
        request = request.json(&body);
    }
    cloud_http::send_json(request, HTTP_LIMITS, NoContent::Unexpected, map_failure).await
}

pub async fn list(auth: &CloudAuth) -> Result<IntegrationConnectionList, OperationError> {
    let (origin, token) = credentials(auth).await?;
    let value = send(
        Method::GET,
        &origin,
        &token,
        "/api/v3/integrations/connections",
        None,
    )
    .await?;
    decode(value)
}

pub async fn connect(
    auth: &CloudAuth,
    params: IntegrationConnectParams,
) -> Result<IntegrationConnectResult, OperationError> {
    let label = normalize_label(&params.label)?;
    let (origin, token) = credentials(auth).await?;
    let value = send(
        Method::POST,
        &origin,
        &token,
        "/api/v3/integrations/webhook/connect",
        Some(json!({ "label": label })),
    )
    .await?;
    let result: IntegrationConnectResult = decode(value)?;
    let secret_ok = result.secret.len() == SECRET_HEX_CHARS
        && result.secret.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !secret_ok || valid_connection_id(&result.connection_id).is_err() {
        return Err(OperationError::internal(
            "cloud sent an unexpected connection",
        ));
    }
    Ok(result)
}

pub async fn events(
    auth: &CloudAuth,
    params: IntegrationIdParams,
) -> Result<IntegrationEventList, OperationError> {
    valid_connection_id(&params.id)?;
    let (origin, token) = credentials(auth).await?;
    let value = send(
        Method::GET,
        &origin,
        &token,
        &format!("/api/v3/integrations/connections/{}/events", params.id),
        None,
    )
    .await?;
    let page: IntegrationEventList = decode(value)?;
    Ok(bound_events(page))
}

/// Keeps the newest events that fit the reply budget, in server order.
fn bound_events(page: IntegrationEventList) -> IntegrationEventList {
    let mut used = 0usize;
    let mut items = Vec::with_capacity(page.items.len());
    for event in page.items {
        let size = serde_json::to_vec(&event)
            .map(|bytes| bytes.len())
            .unwrap_or(usize::MAX);
        if used.saturating_add(size) > RPC_EVENT_REPLY_BYTES {
            return IntegrationEventList {
                items,
                truncated: true,
            };
        }
        used += size;
        items.push(event);
    }
    IntegrationEventList {
        items,
        truncated: page.truncated,
    }
}

pub async fn disconnect(
    auth: &CloudAuth,
    params: IntegrationIdParams,
) -> Result<IntegrationDisconnectResult, OperationError> {
    valid_connection_id(&params.id)?;
    let (origin, token) = credentials(auth).await?;
    let value = send(
        Method::DELETE,
        &origin,
        &token,
        &format!("/api/v3/integrations/connections/{}", params.id),
        None,
    )
    .await?;
    decode(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_ids_are_bounded_before_any_request() {
        assert_eq!(normalize_label("  Stream  ").unwrap(), "Stream");
        assert!(normalize_label("   ").is_err());
        assert!(normalize_label(&"x".repeat(MAX_LABEL_CHARS + 1)).is_err());
        assert!(normalize_label("bad\u{0007}label").is_err());
        assert!(valid_connection_id("3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c").is_ok());
        assert!(valid_connection_id("../admin").is_err());
    }

    #[test]
    fn failures_map_to_safe_messages_without_the_raw_body() {
        let error = map_failure(
            StatusCode::SERVICE_UNAVAILABLE,
            br#"{"code":"misconfigured","requestId":"r","internal":"db password"}"#,
        );
        assert_eq!(error.code, "unavailable");
        assert!(!error.message.contains("db password"));
        assert_eq!(map_failure(StatusCode::NOT_FOUND, b"").code, "not_found");
    }

    #[test]
    fn failures_keep_only_the_allowed_diagnostics() {
        let error = map_failure(
            StatusCode::SERVICE_UNAVAILABLE,
            br#"{"code":"misconfigured","requestId":"req-7f.2:a","retryable":true,"retryAfterMs":1500,"internal":"db password"}"#,
        );
        assert_eq!(
            error.data,
            Some(
                json!({"cloud": {"code": "misconfigured", "requestId": "req-7f.2:a", "retryable": true, "retryAfterMs": 1500}})
            )
        );
        assert_eq!(map_failure(StatusCode::BAD_GATEWAY, b"<html>").data, None);
    }

    #[test]
    fn oversized_event_pages_keep_the_newest_and_say_so() {
        let big = json!("x".repeat(200 * 1024));
        let page = IntegrationEventList {
            items: (0..5)
                .map(|i| IntegrationEvent {
                    delivery_id: format!("d{i}"),
                    received_at: "2026-11-10T00:00:00Z".into(),
                    payload: big.clone(),
                })
                .collect(),
            truncated: false,
        };
        let bounded = bound_events(page);
        assert!(bounded.truncated);
        assert!(bounded.items.len() < 5);
        assert_eq!(bounded.items[0].delivery_id, "d0");
    }
}
