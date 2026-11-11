//! Neutral HTTP plumbing shared by the Cloud domains (teams, integrations,
//! bundles, publisher). It owns the client policy, credentials, bounded body
//! reads and the safe part of the Cloud error envelope. Operations, DTOs and
//! user-facing messages stay in each domain.
//!
//! Three kinds of limit are kept apart on purpose:
//! * HTTP limits ([`Limits`]): how long a request may take and how many bytes
//!   of its reply are ever buffered. Declared per operation by the domain.
//! * RPC limits: how large a value handed back to the caller may be (for
//!   example the integration events budget). They live in the domain.
//! * Input limits: validation of what the caller sends. Also in the domain.
//!
//! Mutations are never retried here. The client disables reqwest's protocol
//! retries and nothing in this module re-sends a request; `retryable` in the
//! Cloud envelope is recovery information for the caller only.

use std::{sync::OnceLock, time::Duration};

use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use tiktools_cloud_auth::{CloudAuth, NativeToken};

use crate::control::OperationError;

/// Largest error body ever buffered, whatever the operation's success limit.
pub(crate) const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;
const MAX_RETRY_AFTER_MS: u64 = 3_600_000;

/// Per-operation HTTP budget.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    /// Whole-request deadline, including reading the body.
    pub(crate) timeout: Duration,
    /// Most bytes of a successful reply that are buffered.
    pub(crate) max_response_bytes: usize,
}

impl Limits {
    pub(crate) const fn new(timeout: Duration, max_response_bytes: usize) -> Self {
        Self {
            timeout,
            max_response_bytes,
        }
    }
}

/// What a `204 No Content` means for one operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoContent {
    /// The operation has no result body: `204` is success and yields `null`.
    Null,
    /// The operation always returns a document: `204` is an unexpected reply.
    Unexpected,
}

/// The shared client: no redirects, no automatic retries, pooled across every
/// Cloud domain. It has no default timeout, so each request must go through
/// [`execute`], which applies the operation's own deadline.
///
/// A failed build is reported and retried on the next call; it never falls back
/// to a client with default (redirecting) policies.
pub(crate) fn client() -> Result<&'static Client, OperationError> {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let built = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .build()
        .map_err(|_| OperationError::unavailable("the Cloud HTTP client could not be started"))?;
    Ok(CLIENT.get_or_init(|| built))
}

/// Origin and session token of the signed-in account. The token is borrowed
/// per operation from [`CloudAuth`], which keeps ownership; callers must not
/// store it. `subject` names the feature in the sign-in message.
pub(crate) async fn credentials(
    auth: &CloudAuth,
    subject: &str,
) -> Result<(String, NativeToken), OperationError> {
    let origin = auth
        .last_identity()
        .map(|(origin, _)| origin)
        .filter(|origin| !origin.is_empty())
        .ok_or_else(|| {
            OperationError::unavailable(format!(
                "{subject} needs a signed-in Cloud account; sign in first"
            ))
        })?;
    let token = auth.native_token().await.ok_or_else(|| {
        OperationError::unavailable(format!("{subject} needs a signed-in Cloud account"))
    })?;
    Ok((origin, token))
}

/// A request to the Cloud with the session token attached. reqwest marks the
/// `Authorization` header sensitive, so it is redacted from debug output.
pub(crate) fn authorized(
    method: Method,
    url: String,
    token: &NativeToken,
) -> Result<RequestBuilder, OperationError> {
    Ok(with_bearer(client()?.request(method, url), token.expose()))
}

fn with_bearer(request: RequestBuilder, secret: &str) -> RequestBuilder {
    request.bearer_auth(secret)
}

/// A reply whose body has already been bounded.
#[derive(Debug)]
pub(crate) struct Reply {
    pub(crate) status: StatusCode,
    /// Whole body for a success, at most [`MAX_ERROR_BODY_BYTES`] for a failure
    /// (empty when the failure body was too large or unreadable).
    pub(crate) body: Vec<u8>,
}

enum BodyFailure {
    TooLarge,
    Unreadable,
}

/// Reads at most `max` bytes: checks `Content-Length` first, then counts every
/// chunk before it is added so a chunked reply is cut during the read.
async fn read_bounded(response: &mut Response, max: usize) -> Result<Vec<u8>, BodyFailure> {
    if response
        .content_length()
        .is_some_and(|length| length > max as u64)
    {
        return Err(BodyFailure::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| BodyFailure::Unreadable)?
    {
        if body.len().saturating_add(chunk.len()) > max {
            return Err(BodyFailure::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Sends the request once, under the operation's deadline, and buffers the
/// reply within its limit. Success bodies over the limit are an error; failure
/// bodies are capped separately and only lose their envelope when oversized.
pub(crate) async fn execute(
    request: RequestBuilder,
    limits: Limits,
) -> Result<Reply, OperationError> {
    let mut response = request
        .timeout(limits.timeout)
        .send()
        .await
        .map_err(|_| OperationError::unavailable("cloud request failed"))?;
    let status = response.status();
    if status.is_success() {
        let body = read_bounded(&mut response, limits.max_response_bytes)
            .await
            .map_err(|failure| match failure {
                BodyFailure::TooLarge => OperationError::unavailable("cloud reply is too large"),
                BodyFailure::Unreadable => {
                    OperationError::unavailable("cloud reply could not be read")
                }
            })?;
        return Ok(Reply { status, body });
    }
    let cap = MAX_ERROR_BODY_BYTES.min(limits.max_response_bytes);
    let body = read_bounded(&mut response, cap).await.unwrap_or_default();
    Ok(Reply { status, body })
}

/// Parses a success body as JSON. An empty body is never silently accepted:
/// `204` follows `no_content`; a zero-length `200` is an unreadable reply.
pub(crate) fn success_json(reply: &Reply, no_content: NoContent) -> Result<Value, OperationError> {
    if reply.status == StatusCode::NO_CONTENT {
        return match no_content {
            NoContent::Null => Ok(Value::Null),
            NoContent::Unexpected => Err(OperationError::internal(
                "cloud sent an empty reply where one was expected",
            )),
        };
    }
    serde_json::from_slice(&reply.body)
        .map_err(|_| OperationError::internal("cloud sent an unreadable reply"))
}

/// Send once and return the JSON reply. A non-success status goes through the
/// domain's `map_failure`, which only ever sees a bounded body.
pub(crate) async fn send_json(
    request: RequestBuilder,
    limits: Limits,
    no_content: NoContent,
    map_failure: fn(StatusCode, &[u8]) -> OperationError,
) -> Result<Value, OperationError> {
    let reply = execute(request, limits).await?;
    if !reply.status.is_success() {
        return Err(map_failure(reply.status, &reply.body));
    }
    success_json(&reply, no_content)
}

/// The Cloud error envelope, parsed without trusting it. Only `code` (for
/// matching known values) and a validated, bounded diagnostics block survive.
#[derive(Debug, Default)]
pub(crate) struct FailureEnvelope {
    code: Option<String>,
    diagnostics: Option<Value>,
}

impl FailureEnvelope {
    pub(crate) fn parse(body: &[u8]) -> Self {
        let Ok(envelope) = serde_json::from_slice::<Value>(body) else {
            return Self::default();
        };
        Self {
            code: envelope
                .get("code")
                .and_then(Value::as_str)
                .map(str::to_owned),
            diagnostics: diagnostics(&envelope),
        }
    }

    pub(crate) fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Adds the allowed diagnostics to a domain error under `data.cloud`.
    pub(crate) fn attach(self, mut error: OperationError) -> OperationError {
        error.data = self.diagnostics;
        error
    }
}

/// The parts of the Cloud error envelope the UI may use, each validated and
/// bounded. Nothing else from the body is kept: a reply may carry internal
/// detail.
fn diagnostics(envelope: &Value) -> Option<Value> {
    let token = |value: Option<&Value>, max: usize, extra: &[u8]| -> Option<String> {
        let text = value?.as_str()?;
        (!text.is_empty()
            && text.len() <= max
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || extra.contains(&byte)))
        .then(|| text.to_owned())
    };
    let code = token(envelope.get("code"), 32, b"")?;
    let mut cloud = serde_json::Map::new();
    cloud.insert("code".into(), Value::String(code));
    if let Some(request_id) = token(envelope.get("requestId"), 80, b"-_.:") {
        cloud.insert("requestId".into(), Value::String(request_id));
    }
    if let Some(retryable) = envelope.get("retryable").and_then(Value::as_bool) {
        cloud.insert("retryable".into(), Value::Bool(retryable));
    }
    if let Some(after) = envelope
        .get("retryAfterMs")
        .and_then(Value::as_u64)
        .filter(|after| *after <= MAX_RETRY_AFTER_MS)
    {
        cloud.insert("retryAfterMs".into(), Value::from(after));
    }
    Some(json!({ "cloud": cloud }))
}

/// Typed decode of an already-validated JSON value.
pub(crate) fn decode<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, OperationError> {
    serde_json::from_value(value)
        .map_err(|_| OperationError::internal("cloud sent an unexpected reply"))
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    use super::*;

    const LIMITS: Limits = Limits::new(Duration::from_secs(5), 1024);

    async fn read_head(stream: &mut TcpStream) {
        let mut head = Vec::new();
        let mut byte = [0_u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            if stream.read_exact(&mut byte).await.is_err() {
                return;
            }
            head.push(byte[0]);
        }
    }

    /// Serves `handler` for every connection and counts the requests it saw.
    async fn serve<F, Fut>(handler: F) -> (String, Arc<AtomicUsize>)
    where
        F: Fn(TcpStream) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = seen.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                read_head(&mut stream).await;
                counter.fetch_add(1, Ordering::SeqCst);
                let work = handler(stream);
                tokio::spawn(work);
            }
        });
        (origin, seen)
    }

    async fn respond(mut stream: TcpStream, status: &str, headers: &str, body: &str) {
        let head = format!(
            "HTTP/1.1 {status}\r\n{headers}content-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes()).await;
        let _ = stream.write_all(body.as_bytes()).await;
    }

    fn get(origin: &str) -> RequestBuilder {
        client().unwrap().get(format!("{origin}/x"))
    }

    fn neutral(status: StatusCode, body: &[u8]) -> OperationError {
        let envelope = FailureEnvelope::parse(body);
        let error = OperationError::unavailable(format!("failed {}", status.as_u16()));
        envelope.attach(error)
    }

    #[tokio::test]
    async fn a_chunked_reply_over_the_limit_is_cut_during_the_read() {
        let written = Arc::new(AtomicUsize::new(0));
        let progress = written.clone();
        let (origin, _) = serve(move |mut stream| {
            let progress = progress.clone();
            async move {
                let _ = stream
                    .write_all(b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n")
                    .await;
                let chunk = vec![b'a'; 16 * 1024];
                for _ in 0..4096 {
                    let mut frame = format!("{:x}\r\n", chunk.len()).into_bytes();
                    frame.extend_from_slice(&chunk);
                    frame.extend_from_slice(b"\r\n");
                    if stream.write_all(&frame).await.is_err() {
                        return;
                    }
                    progress.fetch_add(chunk.len(), Ordering::SeqCst);
                }
            }
        })
        .await;
        let error = execute(
            get(&origin),
            Limits::new(Duration::from_secs(10), 40 * 1024),
        )
        .await
        .unwrap_err();
        assert_eq!(error.message, "cloud reply is too large");
        tokio::time::sleep(Duration::from_millis(300)).await;
        // 64 MiB were on offer; the client stopped reading and closed.
        assert!(written.load(Ordering::SeqCst) < 32 * 1024 * 1024);
    }

    #[tokio::test]
    async fn a_declared_length_over_the_limit_is_refused_before_any_body_is_read() {
        let (origin, _) = serve(|mut stream| async move {
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 10485760\r\n\r\nabc")
                .await;
            tokio::time::sleep(Duration::from_secs(30)).await;
        })
        .await;
        let started = std::time::Instant::now();
        let error = execute(get(&origin), LIMITS).await.unwrap_err();
        assert_eq!(error.message, "cloud reply is too large");
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[tokio::test]
    async fn error_bodies_are_bounded_and_only_lose_their_envelope() {
        let huge = format!(
            r#"{{"code":"offline","padding":"{}"}}"#,
            "x".repeat(MAX_ERROR_BODY_BYTES * 2)
        );
        let (origin, _) = serve(move |stream| {
            let huge = huge.clone();
            async move { respond(stream, "503 Service Unavailable", "", &huge).await }
        })
        .await;
        // The success limit is larger than the error cap on purpose.
        let reply = execute(
            get(&origin),
            Limits::new(Duration::from_secs(5), 4 * 1024 * 1024),
        )
        .await
        .unwrap();
        assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(reply.body.is_empty());
        assert_eq!(neutral(reply.status, &reply.body).data, None);

        let (small_origin, _) = serve(|stream| async move {
            respond(
                stream,
                "503 Service Unavailable",
                "",
                r#"{"code":"offline","requestId":"r-1"}"#,
            )
            .await
        })
        .await;
        let reply = execute(get(&small_origin), LIMITS).await.unwrap();
        assert_eq!(
            neutral(reply.status, &reply.body).data,
            Some(json!({"cloud": {"code": "offline", "requestId": "r-1"}}))
        );
    }

    #[tokio::test]
    async fn invalid_json_and_empty_bodies_are_explicit() {
        let (origin, _) =
            serve(|stream| async move { respond(stream, "200 OK", "", "not json").await }).await;
        let bad = execute(get(&origin), LIMITS).await.unwrap();
        assert_eq!(
            success_json(&bad, NoContent::Null).unwrap_err().message,
            "cloud sent an unreadable reply"
        );

        let (origin, _) =
            serve(|stream| async move { respond(stream, "200 OK", "", "").await }).await;
        let empty = execute(get(&origin), LIMITS).await.unwrap();
        assert!(
            success_json(&empty, NoContent::Null).is_err(),
            "a zero-length 200 is not a result"
        );

        let (origin, _) =
            serve(|stream| async move { respond(stream, "204 No Content", "", "").await }).await;
        let none = execute(get(&origin), LIMITS).await.unwrap();
        assert_eq!(success_json(&none, NoContent::Null).unwrap(), Value::Null);
        assert!(success_json(&none, NoContent::Unexpected).is_err());
    }

    #[tokio::test]
    async fn redirects_are_not_followed() {
        let (origin, seen) = serve(|stream| async move {
            respond(
                stream,
                "302 Found",
                "location: http://127.0.0.1:9/elsewhere\r\n",
                "",
            )
            .await
        })
        .await;
        let reply = execute(get(&origin), LIMITS).await.unwrap();
        assert_eq!(reply.status, StatusCode::FOUND);
        assert_eq!(seen.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_mutation_that_times_out_is_sent_exactly_once() {
        let (origin, seen) = serve(|_stream| async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
        })
        .await;
        let request = client()
            .unwrap()
            .post(format!("{origin}/mutate"))
            .json(&json!({"a": 1}));
        let error = execute(request, Limits::new(Duration::from_millis(150), 1024))
            .await
            .unwrap_err();
        assert_eq!(error.message, "cloud request failed");
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(seen.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn authorization_is_marked_sensitive() {
        let request = with_bearer(
            client().unwrap().get("http://127.0.0.1:1/x"),
            "secret-token",
        )
        .build()
        .unwrap();
        let header = request
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .unwrap();
        assert!(header.is_sensitive());
        assert!(!format!("{header:?}").contains("secret-token"));
    }

    #[test]
    fn diagnostics_keep_only_validated_parts_of_the_envelope() {
        let error = neutral(
            StatusCode::SERVICE_UNAVAILABLE,
            br#"{"code":"offline","requestId":"req-7f.2:a","retryable":true,"retryAfterMs":1500,"internal":"db password","stack":"at x"}"#,
        );
        assert_eq!(
            error.data,
            Some(
                json!({"cloud": {"code": "offline", "requestId": "req-7f.2:a", "retryable": true, "retryAfterMs": 1500}})
            )
        );
        assert!(!error.data.unwrap().to_string().contains("db password"));
        let forged = neutral(
            StatusCode::BAD_GATEWAY,
            br#"{"code":"temporaryFailure","requestId":"has spaces <script>","retryable":"yes","retryAfterMs":99999999999}"#,
        );
        assert_eq!(
            forged.data,
            Some(json!({"cloud": {"code": "temporaryFailure"}}))
        );
        assert_eq!(neutral(StatusCode::BAD_GATEWAY, b"<html>").data, None);
        assert_eq!(
            neutral(StatusCode::BAD_GATEWAY, br#"{"code":"not a code!"}"#).data,
            None
        );
    }
}
