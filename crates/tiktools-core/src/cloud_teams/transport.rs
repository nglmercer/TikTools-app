use super::*;

pub(crate) async fn credentials(auth: &CloudAuth) -> Result<(String, NativeToken), OperationError> {
    cloud_http::credentials(auth, "this").await
}

/// Teams messages for a Cloud failure. Only used for the `/api/v3/remote/*`
/// operations in this module.
pub(super) fn map_failure(status: StatusCode, body: &[u8]) -> OperationError {
    let envelope = FailureEnvelope::parse(body);
    let error = match envelope.code() {
        Some("notFound") => OperationError::not_found("team item not found"),
        Some("conflict") => OperationError::conflict("the item changed; reload and try again"),
        Some("expired") => {
            OperationError::invalid("the challenge or invitation expired; try again")
        }
        Some("invalidRequest") => OperationError::invalid("the Cloud rejected the request"),
        Some("unauthorized" | "forbidden") => {
            OperationError::forbidden("sign in to Cloud again to manage teams")
        }
        Some("offline") => OperationError::unavailable("the host is offline"),
        Some("rateLimited") => OperationError::unavailable("too many requests; try again later"),
        Some("misconfigured" | "unsupported") => {
            OperationError::unavailable("remote control is not available on this Cloud instance")
        }
        _ => match status.as_u16() {
            401 | 403 => OperationError::forbidden("sign in to Cloud again to manage teams"),
            404 => OperationError::not_found("team item not found"),
            409 => OperationError::conflict("the item changed; reload and try again"),
            _ => OperationError::unavailable("team management is unavailable"),
        },
    };
    envelope.attach(error)
}

/// One request, never retried. `204` is a valid empty result for every teams
/// operation (the revoke calls return nothing).
pub(crate) async fn send(
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
    cloud_http::send_json(request, HTTP_LIMITS, NoContent::Null, map_failure).await
}

pub(crate) fn str_field(value: &Value, key: &str) -> Result<String, OperationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| OperationError::internal("cloud sent an unexpected reply"))
}
