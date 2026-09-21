//! Local IPC client: connects to a running control host.
//!
//! Normal CLI commands use this to reach the desktop host (or a standalone
//! `host --ipc` host) instead of constructing a second [`AppCore`]. The
//! framing matches [`crate::transport`]: one JSON request per line, one JSON
//! response per line, with `event` notifications fanned out to subscribers.
//!
//! [`AppCore`]: tiktools_core::AppCore

use std::{
    collections::HashMap,
    pin::Pin,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tiktools_plugin_api::sync::mutex_or_recover;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};

use crate::{transport, MAX_REQUEST_BYTES, REQUEST_TIMEOUT};

/// Connected-control event broadcast capacity. Slow subscribers lag and skip,
/// never block RPC responses.
const EVENT_CHANNEL_CAPACITY: usize = 256;
/// Connection retry budget for a starting host.
const CONNECT_RETRY_BUDGET: std::time::Duration = std::time::Duration::from_secs(3);
/// Delay between connection attempts.
const CONNECT_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(40);

/// Machine-readable client failure. Operation errors from the host surface
/// with the host's own `code` so CLI exit paths stay stable.
#[derive(Debug, Clone, PartialEq)]
pub struct ClientError {
    pub code: String,
    pub message: String,
}

impl ClientError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn host_unavailable() -> Self {
        Self::new("host_unavailable", "TikTools control host is not running.")
    }

    fn transport(message: impl Into<String>) -> Self {
        Self::new("transport", message)
    }

    fn protocol(message: impl Into<String>) -> Self {
        Self::new("protocol", message)
    }
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ClientError {}

impl From<ClientError> for crate::ApiError {
    fn from(error: ClientError) -> Self {
        Self::new(error.code, error.message)
    }
}

enum ClientStream {
    #[cfg(unix)]
    Unix(tokio::net::UnixStream),
    #[cfg(windows)]
    Pipe(tokio::net::windows::named_pipe::NamedPipeClient),
}

impl AsyncRead for ClientStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            Self::Unix(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(windows)]
            Self::Pipe(pipe) => Pin::new(pipe).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ClientStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            #[cfg(unix)]
            Self::Unix(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(windows)]
            Self::Pipe(pipe) => Pin::new(pipe).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            Self::Unix(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(windows)]
            Self::Pipe(pipe) => Pin::new(pipe).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            Self::Unix(stream) => Pin::new(stream).poll_shutdown(cx),
            #[cfg(windows)]
            Self::Pipe(pipe) => Pin::new(pipe).poll_shutdown(cx),
        }
    }
}

type PendingMap =
    Arc<Mutex<HashMap<i64, tokio::sync::oneshot::Sender<Result<Value, ClientError>>>>>;

/// Maps a poisoned pending-call map to a typed client error. The
/// poisoning is logged with the shared lock-recovery target so the
/// failure stays visible in telemetry instead of crashing the caller.
fn pending_poisoned() -> ClientError {
    tracing::warn!(
        target: tiktools_plugin_api::sync::LOCK_RECOVERY_TARGET,
        lock = "control client pending",
        "control client pending lock poisoned; failing call"
    );
    ClientError::transport("control client pending lock poisoned")
}

/// One item on the client's event stream: either an authoritative domain
/// event or an explicit reliable-gap signal. A gap means the host skipped
/// authoritative events this client never saw (`lost` counts them): the
/// stream is no longer complete and the client must refresh authoritative
/// state (live status, points, plugins, creators, workflows) instead of
/// reconstructing the missing events.
#[derive(Debug, Clone, PartialEq)]
pub enum ControlEvent {
    Domain(tiktools_core::events::DomainEvent),
    Gap { lost: u64 },
}

/// Connected control client.
///
/// One background reader task demultiplexes the IPC stream: responses
/// resolve their pending RPC by id while `event` and `event.gap`
/// notifications fan out to every subscriber. Any number of RPCs may be
/// in flight concurrently and events are never discarded to unblock a call.
#[derive(Clone)]
pub struct ControlClient {
    writer: Arc<tokio::sync::Mutex<tokio::io::WriteHalf<ClientStream>>>,
    pending: PendingMap,
    events: tokio::sync::broadcast::Sender<ControlEvent>,
    next_id: Arc<AtomicI64>,
}

impl ControlClient {
    /// Connects to the running host's local IPC endpoint. Transient
    /// startup races (missing pipe/socket, pipe busy) retry within a
    /// bounded budget; only after it expires does this fail with
    /// `host_unavailable`. Callers must surface that instead of silently
    /// starting a second host.
    pub async fn connect() -> Result<Self, ClientError> {
        let stream = connect_stream()
            .await
            .map_err(|_| ClientError::host_unavailable())?;
        Ok(Self::from_stream(stream))
    }

    fn from_stream(stream: ClientStream) -> Self {
        let (read, write) = tokio::io::split(stream);
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (events, _) = tokio::sync::broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let client = Self {
            writer: Arc::new(tokio::sync::Mutex::new(write)),
            pending: Arc::clone(&pending),
            events,
            next_id: Arc::new(AtomicI64::new(1)),
        };
        tokio::spawn(reader_task(
            BufReader::new(read),
            pending,
            client.events.clone(),
        ));
        client
    }

    /// Subscribes to the host's event broadcast. The reader task
    /// forwards every `event` notification as [`ControlEvent::Domain`]
    /// and every `event.gap` notification as [`ControlEvent::Gap`]; slow
    /// receivers lag and skip, matching the host bus semantics.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ControlEvent> {
        self.events.subscribe()
    }

    /// Calls one method and deserializes the typed result. Host operation
    /// errors return with the host's error code. Concurrent calls share
    /// the connection safely.
    pub async fn call<P, R>(&self, method: &str, params: P) -> Result<R, ClientError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let result = self.call_value(method, params).await?;
        serde_json::from_value(result)
            .map_err(|error| ClientError::protocol(format!("bad result shape: {error}")))
    }

    /// Untyped call used by generic `rpc` passthroughs.
    pub async fn call_value<P>(&self, method: &str, params: P) -> Result<Value, ClientError>
    where
        P: Serialize,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let params = serde_json::to_value(params)
            .map_err(|error| ClientError::protocol(error.to_string()))?;
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let mut bytes = serde_json::to_vec(&request)
            .map_err(|error| ClientError::protocol(error.to_string()))?;
        bytes.push(b'\n');
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(ClientError::new(
                "too_large",
                "request exceeds the size limit",
            ));
        }
        let (sender, receiver) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending.lock().map_err(|_| pending_poisoned())?;
            pending.insert(id, sender);
        }
        let write_outcome = async {
            let mut writer = self.writer.lock().await;
            writer
                .write_all(&bytes)
                .await
                .map_err(|error| ClientError::transport(error.to_string()))?;
            writer
                .flush()
                .await
                .map_err(|error| ClientError::transport(error.to_string()))
        }
        .await;
        if let Err(error) = write_outcome {
            // Already failing: recover so the poison never masks the
            // original write error.
            mutex_or_recover(&self.pending, "control client pending").remove(&id);
            return Err(error);
        }
        match tokio::time::timeout(REQUEST_TIMEOUT, receiver).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err(ClientError::transport("host closed the connection")),
            Err(_) => {
                // Already failing: recover so the poison never masks the
                // original timeout.
                mutex_or_recover(&self.pending, "control client pending").remove(&id);
                Err(ClientError::new("timeout", "request timed out"))
            }
        }
    }
}

/// Bridges a direct domain subscription into the shared
/// [`ControlEvent`] broadcast shape. Reliable lag surfaces as
/// [`ControlEvent::Gap`] (the caller must resync); lossy lag only
/// skipped feed and stays silent. The forwarder exits once the bus
/// closes or no receivers remain.
pub fn bridge_subscription(
    mut subscription: tiktools_core::events::DomainSubscription,
) -> tokio::sync::broadcast::Receiver<ControlEvent> {
    let (events, receiver) = tokio::sync::broadcast::channel(EVENT_CHANNEL_CAPACITY);
    tokio::spawn(async move {
        use tiktools_core::events::DomainRecvError;
        loop {
            match subscription.recv().await {
                Ok(event) => {
                    if events.send(ControlEvent::Domain(event)).is_err() {
                        break;
                    }
                }
                Err(DomainRecvError::ReliableLagged(lost)) => {
                    if events.send(ControlEvent::Gap { lost }).is_err() {
                        break;
                    }
                }
                Err(DomainRecvError::LossyLagged(_)) => {}
                Err(DomainRecvError::Closed) => break,
            }
        }
    });
    receiver
}

async fn reader_task(
    mut reader: BufReader<tokio::io::ReadHalf<ClientStream>>,
    pending: PendingMap,
    events: tokio::sync::broadcast::Sender<ControlEvent>,
) {
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                tracing::debug!("control IPC stream closed by host");
                break;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(%error, "control IPC stream read failed");
                break;
            }
        }
        handle_client_line(&line, &pending, &events);
    }
    // The host went away: fail every still-pending call so concurrent
    // waiters never hang until their timeout.
    let senders = {
        let mut pending = mutex_or_recover(&pending, "control client pending");
        std::mem::take(&mut *pending)
    };
    for (_, sender) in senders {
        let _ = sender.send(Err(ClientError::transport("host closed the connection")));
    }
}

/// Demultiplexes one host line: `event` and `event.gap` notifications
/// fan out to subscribers while responses resolve their pending RPC by
/// id. Malformed input is diagnosed (never with line contents, which may
/// carry secrets) and a response matching a live call but carrying
/// neither result nor error fails fast instead of hanging to timeout.
fn handle_client_line(
    line: &str,
    pending: &PendingMap,
    events: &tokio::sync::broadcast::Sender<ControlEvent>,
) {
    let value: Value = match serde_json::from_str(line.trim()) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(
                bytes = line.len(),
                %error,
                "dropping invalid JSON line from control host"
            );
            return;
        }
    };
    if value.get("method").and_then(Value::as_str) == Some("event.gap") {
        // A gap always means resync, even when the count itself is
        // malformed: failing loud (lost: 0) beats silently assuming a
        // complete stream.
        let lost = value
            .get("params")
            .and_then(|params| params.get("lost"))
            .and_then(Value::as_u64)
            .unwrap_or_else(|| {
                tracing::warn!("event.gap notification without a lost count; assuming resync");
                0
            });
        let _ = events.send(ControlEvent::Gap { lost });
        return;
    }
    if value.get("method").and_then(Value::as_str) == Some("event") {
        match value.get("params") {
            Some(params) => {
                match serde_json::from_value::<tiktools_core::events::DomainEvent>(params.clone()) {
                    Ok(event) => {
                        let _ = events.send(ControlEvent::Domain(event));
                    }
                    Err(error) => {
                        let topic = params.get("topic").and_then(Value::as_str).unwrap_or("?");
                        tracing::warn!(
                            topic,
                            %error,
                            "dropping unparseable domain event from control host"
                        );
                    }
                }
            }
            None => tracing::warn!("dropping event notification without params"),
        }
        return;
    }
    let response_id = value.get("id").and_then(Value::as_i64).unwrap_or(-1);
    let sender = {
        let mut pending = mutex_or_recover(pending, "control client pending");
        pending.remove(&response_id)
    };
    let Some(sender) = sender else {
        tracing::debug!(response_id, "response matches no pending request");
        return;
    };
    if let Some(error) = value.get("error") {
        let code = error
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("error")
            .to_owned();
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("request failed")
            .to_owned();
        let _ = sender.send(Err(ClientError::new(code, message)));
    } else if let Some(result) = value.get("result").cloned() {
        let _ = sender.send(Ok(result));
    } else {
        tracing::warn!(
            response_id,
            "control host sent a response without result or error"
        );
        let _ = sender.send(Err(ClientError::protocol(
            "malformed response: missing result and error",
        )));
    }
}

async fn connect_stream() -> std::io::Result<ClientStream> {
    let deadline = tokio::time::Instant::now() + CONNECT_RETRY_BUDGET;
    loop {
        match try_connect_stream().await {
            Ok(stream) => return Ok(stream),
            Err(error) if is_transient_connect_error(&error) => {
                if tokio::time::Instant::now() >= deadline {
                    return Err(error);
                }
                tokio::time::sleep(CONNECT_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
}

fn is_transient_connect_error(error: &std::io::Error) -> bool {
    // Windows named-pipe startup races: ERROR_FILE_NOT_FOUND (2) while the
    // server instance is being created, ERROR_PIPE_BUSY (231) while all
    // instances are connected.
    #[cfg(windows)]
    if let Some(code) = error.raw_os_error() {
        if code == 2 || code == 231 {
            return true;
        }
    }
    matches!(
        error.kind(),
        std::io::ErrorKind::NotFound
            | std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::Interrupted
    )
}

async fn try_connect_stream() -> std::io::Result<ClientStream> {
    #[cfg(unix)]
    {
        tokio::net::UnixStream::connect(transport::ipc_socket_path())
            .await
            .map(ClientStream::Unix)
    }
    #[cfg(windows)]
    {
        let name = transport::ipc_pipe_name()?;
        tokio::net::windows::named_pipe::ClientOptions::new()
            .open(name)
            .map(ClientStream::Pipe)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "local IPC is only available on Unix and Windows",
        ))
    }
}

#[cfg(test)]
mod client_line_tests {
    use std::time::Duration;

    use super::*;

    fn harness() -> (PendingMap, tokio::sync::broadcast::Sender<ControlEvent>) {
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (events, _) = tokio::sync::broadcast::channel(16);
        (pending, events)
    }

    fn listen(
        pending: &PendingMap,
        id: i64,
    ) -> tokio::sync::oneshot::Receiver<Result<Value, ClientError>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        pending
            .lock()
            .expect("pending lock poisoned")
            .insert(id, sender);
        receiver
    }

    fn poison(pending: &PendingMap) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = pending.lock().unwrap();
            panic!("test poison");
        }));
    }

    #[tokio::test]
    async fn malformed_response_fails_pending_call_without_timeout() {
        let (pending, events) = harness();
        let receiver = listen(&pending, 7);
        handle_client_line(r#"{"jsonrpc":"2.0","id":7}"#, &pending, &events);
        let outcome = tokio::time::timeout(Duration::from_secs(5), receiver)
            .await
            .expect("must resolve without waiting for timeout")
            .expect("sender alive");
        let error = outcome.expect_err("neither result nor error must fail");
        assert_eq!(error.code, "protocol");
    }

    #[tokio::test]
    async fn garbage_and_bad_events_never_break_the_stream() {
        let (pending, events) = harness();
        let mut subscriber = events.subscribe();
        let receiver = listen(&pending, 9);
        // Invalid JSON, an event without params, and an unparseable event
        // payload are diagnosed and skipped; the pending call still
        // resolves normally afterwards.
        handle_client_line("{{{ not json", &pending, &events);
        handle_client_line(r#"{"method":"event"}"#, &pending, &events);
        handle_client_line(
            r#"{"method":"event","params":{"topic":"points.changed","data":{}}}"#,
            &pending,
            &events,
        );
        handle_client_line(
            r#"{"jsonrpc":"2.0","id":9,"result":{"ok":true}}"#,
            &pending,
            &events,
        );
        let outcome = tokio::time::timeout(Duration::from_secs(5), receiver)
            .await
            .expect("resolves promptly")
            .expect("sender alive")
            .expect("valid response resolves");
        assert_eq!(outcome, serde_json::json!({"ok": true}));
        assert!(subscriber.try_recv().is_err());
    }

    #[tokio::test]
    async fn valid_event_still_fans_out() {
        let (pending, events) = harness();
        let mut subscriber = events.subscribe();
        handle_client_line(
            r#"{"method":"event","params":{"topic":"live.disconnected"}}"#,
            &pending,
            &events,
        );
        let event = subscriber.try_recv().expect("event fans out");
        assert!(matches!(
            event,
            ControlEvent::Domain(tiktools_core::events::DomainEvent::LiveDisconnected)
        ));
    }

    #[tokio::test]
    async fn gap_notification_fans_out_as_gap() {
        let (pending, events) = harness();
        let mut subscriber = events.subscribe();
        handle_client_line(
            r#"{"jsonrpc":"2.0","method":"event.gap","params":{"lost":12,"resync":true}}"#,
            &pending,
            &events,
        );
        assert_eq!(
            subscriber.try_recv().expect("gap fans out"),
            ControlEvent::Gap { lost: 12 }
        );
        // A malformed gap still forces resync rather than silently
        // assuming a complete stream.
        handle_client_line(
            r#"{"method":"event.gap","params":{"resync":true}}"#,
            &pending,
            &events,
        );
        assert_eq!(
            subscriber.try_recv().expect("malformed gap still fans out"),
            ControlEvent::Gap { lost: 0 }
        );
        // Later domain events still arrive after the gap.
        handle_client_line(
            r#"{"method":"event","params":{"topic":"live.disconnected"}}"#,
            &pending,
            &events,
        );
        assert!(matches!(
            subscriber.try_recv().expect("post-gap event fans out"),
            ControlEvent::Domain(_)
        ));
    }

    #[tokio::test]
    async fn response_still_resolves_when_the_pending_map_is_poisoned() {
        let (pending, events) = harness();
        let receiver = listen(&pending, 11);
        poison(&pending);
        handle_client_line(
            r#"{"jsonrpc":"2.0","id":11,"result":{"ok":true}}"#,
            &pending,
            &events,
        );
        let outcome = tokio::time::timeout(Duration::from_secs(5), receiver)
            .await
            .expect("resolves promptly")
            .expect("sender alive")
            .expect("valid response resolves");
        assert_eq!(outcome, serde_json::json!({"ok": true}));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn poisoned_pending_map_fails_new_calls_with_a_typed_error() {
        let (ours, _peer) = tokio::net::UnixStream::pair().expect("test socket pair");
        let client = ControlClient::from_stream(ClientStream::Unix(ours));
        poison(&client.pending);
        let outcome: Result<Value, ClientError> = client
            .call_value("system.ping", serde_json::json!({}))
            .await;
        let error = outcome.expect_err("poisoned pending map must fail the call");
        assert_eq!(error.code, "transport");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn closed_stream_drains_poisoned_pending_calls() {
        let (pending, events) = harness();
        let receiver = listen(&pending, 3);
        poison(&pending);
        let (ours, peer) = tokio::net::UnixStream::pair().expect("test socket pair");
        let (read, write) = tokio::io::split(ClientStream::Unix(ours));
        drop(peer);
        drop(write);
        reader_task(BufReader::new(read), pending, events).await;
        let outcome = tokio::time::timeout(Duration::from_secs(5), receiver)
            .await
            .expect("drain resolves")
            .expect("sender alive");
        let error = outcome.expect_err("closed host must fail pending calls");
        assert_eq!(error.code, "transport");
    }
}
