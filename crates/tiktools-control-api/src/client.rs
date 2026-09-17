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

/// Connected control client.
///
/// One background reader task demultiplexes the IPC stream: responses
/// resolve their pending RPC by id while `event` notifications fan out to
/// every subscriber. Any number of RPCs may be in flight concurrently and
/// events are never discarded to unblock a call.
#[derive(Clone)]
pub struct ControlClient {
    writer: Arc<tokio::sync::Mutex<tokio::io::WriteHalf<ClientStream>>>,
    pending: PendingMap,
    events: tokio::sync::broadcast::Sender<tiktools_core::events::DomainEvent>,
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

    /// Subscribes to the host's domain-event broadcast. The reader task
    /// forwards every `event` notification here; slow receivers lag and
    /// skip, matching the host bus semantics.
    pub fn subscribe(
        &self,
    ) -> tokio::sync::broadcast::Receiver<tiktools_core::events::DomainEvent> {
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
            let mut pending = self.pending.lock().expect("client pending lock poisoned");
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
            let mut pending = self.pending.lock().expect("client pending lock poisoned");
            pending.remove(&id);
            return Err(error);
        }
        match tokio::time::timeout(REQUEST_TIMEOUT, receiver).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err(ClientError::transport("host closed the connection")),
            Err(_) => {
                let mut pending = self.pending.lock().expect("client pending lock poisoned");
                pending.remove(&id);
                Err(ClientError::new("timeout", "request timed out"))
            }
        }
    }
}

async fn reader_task(
    mut reader: BufReader<tokio::io::ReadHalf<ClientStream>>,
    pending: PendingMap,
    events: tokio::sync::broadcast::Sender<tiktools_core::events::DomainEvent>,
) {
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let value: Value = match serde_json::from_str(line.trim()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if value.get("method").and_then(Value::as_str) == Some("event") {
            if let Some(params) = value.get("params") {
                if let Ok(event) =
                    serde_json::from_value::<tiktools_core::events::DomainEvent>(params.clone())
                {
                    let _ = events.send(event);
                }
            }
            continue;
        }
        let response_id = value.get("id").and_then(Value::as_i64).unwrap_or(-1);
        let sender = {
            let mut pending = pending.lock().expect("client pending lock poisoned");
            pending.remove(&response_id)
        };
        let Some(sender) = sender else {
            continue;
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
        } else {
            let result = value.get("result").cloned().unwrap_or(Value::Null);
            let _ = sender.send(Ok(result));
        }
    }
    // The host went away: fail every still-pending call so concurrent
    // waiters never hang until their timeout.
    let senders = {
        let mut pending = pending.lock().expect("client pending lock poisoned");
        std::mem::take(&mut *pending)
    };
    for (_, sender) in senders {
        let _ = sender.send(Err(ClientError::transport("host closed the connection")));
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
        tokio::net::windows::named_pipe::ClientOptions::new()
            .open(transport::ipc_pipe_name())
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
