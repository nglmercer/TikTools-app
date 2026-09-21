//! Local IPC over a Unix domain socket.

use super::server::serve_stream;
use super::{IPC_NAME, SHUTDOWN_POLL};
use crate::ControlApi;
use std::sync::Arc;

#[cfg(unix)]
pub(crate) fn ipc_socket_path() -> std::path::PathBuf {
    tiktools_core::paths::AppPaths::from_environment()
        .root
        .join(format!("{IPC_NAME}.sock"))
}

#[cfg(unix)]
pub(crate) async fn run_ipc_unix_with_ready<F>(
    api: Arc<ControlApi>,
    on_ready: F,
) -> std::io::Result<()>
where
    F: FnOnce() + Send,
{
    use tokio::net::UnixListener;

    let path = ipc_socket_path();
    if path.exists() {
        // The ownership lock is already held here (acquired before this
        // function runs), so a live owner cannot exist: a connect probe
        // distinguishes its stale socket (unlink) from a foreign bind,
        // which still fails below with `AddrInUse`.
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
