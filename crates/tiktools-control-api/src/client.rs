//! Local IPC client: connects to a running control host.
//!
//! Normal CLI commands use this to reach the desktop host (or a standalone
//! `host --ipc` host) instead of constructing a second [`AppCore`]. The
//! framing matches [`crate::transport`]: one JSON request per line, one JSON
//! response per line, with `event` notifications skipped by [`call`].
//!
//! [`AppCore`]: tiktools_core::AppCore
//! [`call`]: ControlClient::call

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};

use crate::{transport, MAX_REQUEST_BYTES, REQUEST_TIMEOUT};

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

/// Connected control client. Calls are sequential: one in-flight request at
/// a time per connection.
pub struct ControlClient {
    reader: BufReader<tokio::io::ReadHalf<ClientStream>>,
    writer: tokio::io::WriteHalf<ClientStream>,
    next_id: i64,
}

impl ControlClient {
    /// Connects to the running host's local IPC endpoint. Fails with
    /// `host_unavailable` when nothing is listening; callers must surface
    /// that instead of silently starting a second host.
    pub async fn connect() -> Result<Self, ClientError> {
        let stream = connect_stream()
            .await
            .map_err(|_| ClientError::host_unavailable())?;
        let (read, write) = tokio::io::split(stream);
        Ok(Self {
            reader: BufReader::new(read),
            writer: write,
            next_id: 1,
        })
    }

    /// Calls one method and deserializes the typed result. Host operation
    /// errors return with the host's error code; interleaved `event`
    /// notifications are skipped.
    pub async fn call<P, R>(&mut self, method: &str, params: P) -> Result<R, ClientError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let result = self.call_value(method, params).await?;
        serde_json::from_value(result)
            .map_err(|error| ClientError::protocol(format!("bad result shape: {error}")))
    }

    /// Untyped call used by generic `rpc` passthroughs.
    pub async fn call_value<P>(&mut self, method: &str, params: P) -> Result<Value, ClientError>
    where
        P: Serialize,
    {
        let id = self.next_id;
        self.next_id += 1;
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
        self.writer
            .write_all(&bytes)
            .await
            .map_err(|error| ClientError::transport(error.to_string()))?;
        self.writer
            .flush()
            .await
            .map_err(|error| ClientError::transport(error.to_string()))?;
        loop {
            let mut line = String::new();
            let read =
                tokio::time::timeout(REQUEST_TIMEOUT, self.reader.read_line(&mut line)).await;
            let bytes_read = match read {
                Ok(Ok(bytes_read)) => bytes_read,
                Ok(Err(error)) => return Err(ClientError::transport(error.to_string())),
                Err(_) => return Err(ClientError::new("timeout", "request timed out")),
            };
            if bytes_read == 0 {
                return Err(ClientError::transport("host closed the connection"));
            }
            let value: Value = serde_json::from_str(line.trim()).map_err(|error| {
                ClientError::protocol(format!("invalid response JSON: {error}"))
            })?;
            // Event notifications interleave on IPC streams; they carry no
            // id and must not resolve a pending call.
            if value.get("method").and_then(Value::as_str) == Some("event") {
                continue;
            }
            let response_id = value.get("id").and_then(Value::as_i64).unwrap_or(-1);
            if response_id != id {
                continue;
            }
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
                return Err(ClientError::new(code, message));
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

async fn connect_stream() -> std::io::Result<ClientStream> {
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
