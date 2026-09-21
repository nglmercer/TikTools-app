//! NDJSON transports: stdio and persistent local IPC.
//!
//! All transports share one framing rule: one JSON request per line, one
//! JSON response per line. Event notifications interleave on the same
//! stream when event streaming is enabled.

mod framing;
#[cfg(windows)]
mod security;
mod server;
mod stdio;
#[cfg(test)]
mod tests;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use framing::{MAX_PARAMS_BYTES, MAX_REQUEST_BYTES};
pub use stdio::run_stdio;
#[cfg(unix)]
pub(crate) use unix::ipc_socket_path;
#[cfg(windows)]
pub(crate) use windows::ipc_pipe_name;

use std::sync::Arc;

use crate::ControlApi;

/// Local IPC endpoint name (`tiktools-control.sock` / pipe).
pub const IPC_NAME: &str = "tiktools-control";

#[cfg(any(unix, windows))]
/// Listener shutdown poll interval.
pub(crate) const SHUTDOWN_POLL: std::time::Duration = std::time::Duration::from_millis(250);

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
        unix::run_ipc_unix_with_ready(api, on_ready).await
    }
    #[cfg(windows)]
    {
        windows::run_ipc_windows_with_ready(api, on_ready).await
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
