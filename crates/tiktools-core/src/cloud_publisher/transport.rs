use super::*;

impl CloudPublisher {
    pub(super) async fn credentials(
        &self,
        auth: &CloudAuth,
    ) -> Result<(String, NativeToken), crate::control::OperationError> {
        cloud_http::credentials(auth, "cloud publisher").await
    }

    pub(super) async fn post_json<Req: Serialize, Res: for<'de> Deserialize<'de>>(
        &self,
        origin: &str,
        token: &NativeToken,
        path: &str,
        idempotency_key: Option<&str>,
        body: &Req,
    ) -> Result<Res, crate::control::OperationError> {
        let mut request =
            cloud_http::authorized(Method::POST, format!("{origin}{path}"), token)?.json(body);
        if let Some(key) = idempotency_key {
            request = request.header("idempotency-key", key);
        }
        decode_reply(cloud_http::execute(request, HTTP_LIMITS).await?)
    }

    pub(super) async fn get_json<Res: for<'de> Deserialize<'de>>(
        &self,
        origin: &str,
        token: &NativeToken,
        path: &str,
    ) -> Result<Res, crate::control::OperationError> {
        let request = cloud_http::authorized(Method::GET, format!("{origin}{path}"), token)?;
        decode_reply(cloud_http::execute(request, HTTP_LIMITS).await?)
    }

    /// DELETE with no response body. A 2xx status is success. A 409 means the
    /// backend still sees members, keys, uploads, releases, transfers or
    /// step-up records, so nothing was removed.
    pub(super) async fn delete_empty(
        &self,
        origin: &str,
        token: &NativeToken,
        path: &str,
    ) -> Result<(), crate::control::OperationError> {
        use crate::control::OperationError;
        let request = cloud_http::authorized(Method::DELETE, format!("{origin}{path}"), token)?;
        let reply = cloud_http::execute(request, HTTP_LIMITS).await?;
        if reply.status.is_success() {
            return Ok(());
        }
        if reply.status == StatusCode::CONFLICT {
            return Err(OperationError::conflict(
                "publisher still has other members, keys, uploads, releases, transfers or step-up records; nothing was deleted",
            ));
        }
        Err(backend_failure(&reply))
    }
}

/// Typed JSON of a success reply, or the backend failure of any other status.
pub(super) fn decode_reply<Res: for<'de> Deserialize<'de>>(
    reply: Reply,
) -> Result<Res, crate::control::OperationError> {
    use crate::control::OperationError;
    if !reply.status.is_success() {
        return Err(backend_failure(&reply));
    }
    if reply.status == StatusCode::NO_CONTENT {
        // Every typed publisher reply is a document.
        cloud_http::success_json(&reply, NoContent::Unexpected)?;
    }
    serde_json::from_slice(&reply.body)
        .map_err(|_| OperationError::internal("cloud sent an unreadable reply"))
}

/// The backend's own `{"error": "..."}` text, trimmed to a short single line,
/// or the status when there is none. The body was already size-bounded.
pub(super) fn backend_failure(reply: &Reply) -> crate::control::OperationError {
    use crate::control::OperationError;
    let status = reply.status;
    let message = serde_json::from_slice::<BackendError>(&reply.body)
        .ok()
        .and_then(|body| body.error)
        .map(|text| {
            text.chars()
                .filter(|character| !character.is_control())
                .take(MAX_BACKEND_MESSAGE_CHARS)
                .collect::<String>()
        })
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| format!("cloud rejected the request ({status})"));
    match status {
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
            OperationError::invalid(message)
        }
        StatusCode::UNAUTHORIZED => OperationError::unavailable(format!(
            "{message}; the stored session may have expired, sign in again"
        )),
        StatusCode::FORBIDDEN => OperationError::forbidden(message),
        StatusCode::NOT_FOUND => OperationError::not_found(message),
        StatusCode::CONFLICT => OperationError::conflict(message),
        StatusCode::TOO_MANY_REQUESTS => {
            OperationError::unavailable(format!("cloud rate limit: {message}"))
        }
        _ => OperationError::internal(message),
    }
}

#[derive(Debug, Clone, Deserialize)]
struct BackendError {
    #[serde(default)]
    error: Option<String>,
}
