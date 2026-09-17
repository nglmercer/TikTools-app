//! NDJSON transports: stdio and persistent local IPC.
//!
//! All transports share one framing rule: one JSON request per line, one
//! JSON response per line. Event notifications interleave on the same
//! stream when event streaming is enabled.

use std::sync::Arc;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use crate::{event_notification, ApiError, ControlApi, RpcResponse};

/// Hard cap for one request line (bytes, including the newline).
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
/// Hard cap for the serialized `params` payload of one request.
pub const MAX_PARAMS_BYTES: usize = 256 * 1024;
/// Local IPC endpoint name (`tiktools-control.sock` / pipe).
pub const IPC_NAME: &str = "tiktools-control";
/// Listener shutdown poll interval.
const SHUTDOWN_POLL: std::time::Duration = std::time::Duration::from_millis(250);

/// Serves JSON-RPC over stdin/stdout until EOF or `system.shutdown`.
/// `stream_events` interleaves domain-event notifications on stdout.
pub async fn run_stdio(api: ControlApi, stream_events: bool) -> std::io::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    serve_stream(
        &api,
        tokio::io::BufReader::new(stdin),
        stdout,
        stream_events,
    )
    .await
}

/// Serves JSON-RPC over persistent local IPC with event streaming:
/// a Unix domain socket on Unix, a named pipe on Windows.
pub async fn run_ipc(api: ControlApi) -> std::io::Result<()> {
    run_ipc_shared(Arc::new(api)).await
}

/// Serves local IPC from a shared [`ControlApi`]. The desktop host uses this
/// so its WebView, control router, and IPC server all share one `AppCore`.
/// The loop exits once the core starts shutting down.
///
/// The server holds the OS control-host ownership primitive for its whole
/// lifetime and fails with `AddrInUse` when another host already owns the
/// production endpoint.
pub async fn run_ipc_shared(api: Arc<ControlApi>) -> std::io::Result<()> {
    run_ipc_shared_with_ready(api, || {}).await
}

/// Serves local IPC like [`run_ipc_shared`], invoking `on_ready` exactly
/// once after the endpoint is actually listening (Unix bind / first pipe
/// instance). Ownership or bind failures return before it ever fires, so
/// retry loops can clear degraded health only on genuine recovery.
pub async fn run_ipc_shared_with_ready<F>(api: Arc<ControlApi>, on_ready: F) -> std::io::Result<()>
where
    F: FnOnce() + Send,
{
    let _ownership = crate::ownership::acquire_control_host_ownership()?;
    #[cfg(unix)]
    {
        run_ipc_unix_with_ready(api, on_ready).await
    }
    #[cfg(windows)]
    {
        run_ipc_windows_with_ready(api, on_ready).await
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = api;
        let _ = on_ready;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "local IPC is only available on Unix and Windows",
        ))
    }
}

#[cfg(unix)]
pub(crate) fn ipc_socket_path() -> std::path::PathBuf {
    tiktools_core::paths::AppPaths::from_environment()
        .root
        .join(format!("{IPC_NAME}.sock"))
}

#[cfg(unix)]
async fn run_ipc_unix_with_ready<F>(api: Arc<ControlApi>, on_ready: F) -> std::io::Result<()>
where
    F: FnOnce() + Send,
{
    use tokio::net::UnixListener;

    let path = ipc_socket_path();
    if path.exists() {
        // A stale socket from a crashed host is unusable; a live host would
        // fail to bind below if the path were truly taken... best effort:
        // try connecting first, and only unlink when nobody answers.
        if tokio::net::UnixStream::connect(&path).await.is_err() {
            let _ = std::fs::remove_file(&path);
        }
    }
    let listener = UnixListener::bind(&path)?;
    // Per-user endpoint: only the owner may connect to the control socket.
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    tracing::info!(path = %path.display(), "control IPC listening");
    on_ready();
    loop {
        if api.core().is_shutdown() {
            let _ = std::fs::remove_file(&path);
            return Ok(());
        }
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                let api = Arc::clone(&api);
                tokio::spawn(async move {
                    let (read, write) = stream.into_split();
                    if let Err(error) =
                        serve_stream(&api, tokio::io::BufReader::new(read), write, true).await
                    {
                        tracing::debug!(%error, "control IPC connection ended");
                    }
                });
            }
            // Wake periodically so `system.shutdown` stops the listener
            // even while no client is connecting.
            _ = tokio::time::sleep(SHUTDOWN_POLL) => {}
        }
    }
}

#[cfg(windows)]
pub(crate) fn ipc_pipe_name() -> String {
    // Test override so integration tests isolate their endpoint instead
    // of touching the well-known production pipe. Unix isolates through
    // TIKTOOLS_HOME already, so no override is needed there.
    if let Ok(name) = std::env::var("TIKTOOLS_IPC_NAME") {
        if !name.is_empty()
            && name.len() <= 64
            && name.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '-' || character == '_'
            })
        {
            return format!(r"\\.\pipe\{name}");
        }
    }
    format!(r"\\.\pipe\{IPC_NAME}")
}

#[cfg(windows)]
async fn run_ipc_windows_with_ready<F>(api: Arc<ControlApi>, on_ready: F) -> std::io::Result<()>
where
    F: FnOnce() + Send,
{
    use tokio::net::windows::named_pipe::ServerOptions;

    let name = ipc_pipe_name();
    tracing::info!(pipe = %name, "control IPC listening");
    let mut on_ready = Some(on_ready);
    loop {
        if api.core().is_shutdown() {
            return Ok(());
        }
        let server = ServerOptions::new().create(&name)?;
        // Ready exactly once, after the first pipe instance exists: each
        // loop turn creates a fresh instance, so only the first counts.
        if let Some(ready) = on_ready.take() {
            ready();
        }
        tokio::select! {
            result = server.connect() => {
                result?;
                let api = Arc::clone(&api);
                tokio::spawn(async move {
                    let (read, write) = tokio::io::split(server);
                    if let Err(error) =
                        serve_stream(&api, tokio::io::BufReader::new(read), write, true).await
                    {
                        tracing::debug!(%error, "control IPC connection ended");
                    }
                });
            }
            // Wake periodically so `system.shutdown` stops the listener
            // even while no client is connecting.
            _ = tokio::time::sleep(SHUTDOWN_POLL) => {}
        }
    }
}

async fn serve_stream<R, W>(
    api: &ControlApi,
    mut reader: R,
    mut writer: W,
    stream_events: bool,
) -> std::io::Result<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut events = stream_events.then(|| api.subscribe());
    let mut line = Vec::with_capacity(4096);
    loop {
        tokio::select! {
            biased;
            result = read_capped_line(&mut reader, &mut line) => {
                match result? {
                    None => return Ok(()),
                    Some(LineOutcome::Ok) => {
                        let response = execute_line(api, &line).await;
                        line.clear();
                        write_response(&mut writer, &response).await?;
                        // Flush events the request published (a biased select
                        // would otherwise starve them behind piped requests).
                        drain_events(&mut writer, &mut events).await?;
                        if api.core().is_shutdown() {
                            writer.flush().await?;
                            return Ok(());
                        }
                    }
                    Some(LineOutcome::TooLarge) => {
                        line.clear();
                        let response = RpcResponse::error(
                            crate::RpcId::Null,
                            ApiError::too_large(),
                        );
                        write_response(&mut writer, &response).await?;
                    }
                }
            }
            event = async {
                match events.as_mut() {
                    Some(receiver) => receiver.recv().await.ok(),
                    None => std::future::pending().await,
                }
            } => {
                if let Some(event) = event {
                    let notification = event_notification(&event);
                    let mut bytes = serde_json::to_vec(&notification)
                        .unwrap_or_else(|_| b"{}".to_vec());
                    bytes.push(b'\n');
                    writer.write_all(&bytes).await?;
                    writer.flush().await?;
                }
            }
        }
    }
}

enum LineOutcome {
    Ok,
    TooLarge,
}

/// Reads one `\n`-terminated line capped at [`MAX_REQUEST_BYTES`]. Overlong
/// lines are discarded through the terminator so framing stays aligned.
async fn read_capped_line<R>(
    reader: &mut R,
    line: &mut Vec<u8>,
) -> std::io::Result<Option<LineOutcome>>
where
    R: AsyncBufRead + Unpin,
{
    line.clear();
    let mut total = 0usize;
    let mut too_large = false;
    loop {
        let chunk = reader.fill_buf().await?;
        if chunk.is_empty() {
            return Ok((!too_large && total > 0).then_some(LineOutcome::Ok));
        }
        let end = chunk.iter().position(|byte| *byte == b'\n');
        match end {
            Some(index) => {
                let take = index + 1;
                if !too_large {
                    total += take;
                    if total > MAX_REQUEST_BYTES {
                        too_large = true;
                        line.clear();
                    } else {
                        line.extend_from_slice(&chunk[..take]);
                    }
                }
                reader.consume(take);
                return Ok(Some(if too_large {
                    LineOutcome::TooLarge
                } else {
                    LineOutcome::Ok
                }));
            }
            None => {
                if !too_large {
                    total += chunk.len();
                    if total > MAX_REQUEST_BYTES {
                        too_large = true;
                        line.clear();
                    } else {
                        line.extend_from_slice(chunk);
                    }
                }
                let len = chunk.len();
                reader.consume(len);
            }
        }
    }
}

/// Writes every queued domain event as a notification. Stops at the
/// first empty/lagged/closed drain so a hot bus cannot stall responses.
async fn drain_events<W>(
    writer: &mut W,
    events: &mut Option<tokio::sync::broadcast::Receiver<tiktools_core::events::DomainEvent>>,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let Some(receiver) = events.as_mut() else {
        return Ok(());
    };
    while let Ok(event) = receiver.try_recv() {
        let notification = event_notification(&event);
        let mut bytes = serde_json::to_vec(&notification).unwrap_or_else(|_| b"{}".to_vec());
        bytes.push(b'\n');
        writer.write_all(&bytes).await?;
    }
    writer.flush().await
}

async fn execute_line(api: &ControlApi, line: &[u8]) -> RpcResponse {
    let text = String::from_utf8_lossy(line);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return RpcResponse::error(
            crate::RpcId::Null,
            ApiError::invalid_params("empty request"),
        );
    }
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(raw) => api.execute_value(&raw).await,
        Err(error) => RpcResponse::error(
            crate::RpcId::Null,
            ApiError::invalid_params(format!("invalid JSON: {error}")),
        ),
    }
}

async fn write_response<W>(writer: &mut W, response: &RpcResponse) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut bytes = serde_json::to_vec(response)
        .unwrap_or_else(|_| b"{\"jsonrpc\":\"2.0\",\"id\":null}".to_vec());
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives one request plus one domain event through the NDJSON framing
    /// over an in-memory duplex (no sockets needed).
    #[tokio::test]
    async fn ndjson_framing_carries_responses_and_events() {
        struct Emitter;
        impl tiktools_core::HostEmitter for Emitter {
            fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
        }
        // Isolated home so the core never touches real user data.
        let home = std::env::temp_dir().join(format!(
            "tiktools-transport-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        std::env::set_var("TIKTOOLS_HOME", &home);
        let core = std::sync::Arc::new(tiktools_core::AppCore::new(std::sync::Arc::new(Emitter)));
        let api = ControlApi::new(std::sync::Arc::clone(&core));
        let (client, server) = tokio::io::duplex(64 * 1024);
        let (server_read, server_write) = tokio::io::split(server);
        let server_task = tokio::spawn(async move {
            serve_stream(
                &api,
                tokio::io::BufReader::new(server_read),
                server_write,
                true,
            )
            .await
        });
        let (client_read, mut client_write) = tokio::io::split(client);
        client_write
            .write_all(b"{\"id\":1,\"method\":\"system.ping\"}\n")
            .await
            .unwrap();
        let mut lines = tokio::io::BufReader::new(client_read).lines();
        // Every wait below is bounded: a stuck server must fail the test,
        // never hang the suite.
        let response = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("response line timed out")
            .unwrap()
            .expect("response line");
        assert!(
            response.contains("\"id\":1"),
            "unexpected response: {response}"
        );
        assert!(
            response.contains("\"ok\":true"),
            "unexpected response: {response}"
        );
        // The server is now inside its select loop (hence subscribed), so a
        // domain event published here interleaves as a JSON-RPC notification.
        core.events
            .publish_domain(tiktools_core::events::DomainEvent::LiveDisconnected);
        let event = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("event line timed out")
            .unwrap()
            .expect("event line");
        assert!(
            event.contains("\"method\":\"event\""),
            "unexpected event: {event}"
        );
        assert!(
            event.contains("live.disconnected"),
            "unexpected event: {event}"
        );
        // Split halves share the duplex endpoint, so a bare drop would not
        // deliver EOF; an explicit shutdown closes the write side.
        client_write.shutdown().await.unwrap();
        let eof = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("EOF timed out")
            .unwrap();
        assert!(eof.is_none());
        tokio::time::timeout(std::time::Duration::from_secs(10), server_task)
            .await
            .expect("server task hung after EOF")
            .expect("server task panicked")
            .unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn overlong_lines_error_without_breaking_framing() {
        let input = format!(
            "{{\"id\":1,\"method\":\"{}}}\n{{\"id\":2,\"method\":\"system.ping\"}}\n",
            "x".repeat(MAX_REQUEST_BYTES)
        );
        let mut reader = tokio::io::BufReader::new(input.as_bytes());
        let mut line = Vec::new();
        assert!(matches!(
            read_capped_line(&mut reader, &mut line).await.unwrap(),
            Some(LineOutcome::TooLarge)
        ));
        assert!(matches!(
            read_capped_line(&mut reader, &mut line).await.unwrap(),
            Some(LineOutcome::Ok)
        ));
        assert!(String::from_utf8_lossy(&line).contains("\"id\":2"));
    }
}
