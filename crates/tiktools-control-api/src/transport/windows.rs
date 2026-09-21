//! Local IPC over a Windows named pipe.

use super::security;
use super::server::serve_stream;
use super::{IPC_NAME, SHUTDOWN_POLL};
use crate::ControlApi;
use std::sync::Arc;

#[cfg(windows)]
pub(crate) fn ipc_pipe_name() -> std::io::Result<String> {
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
            return Ok(format!(r"\\.\pipe\{name}"));
        }
    }
    // Per-user production pipe: the pipe namespace is machine-wide while
    // the ownership mutex is session-scoped, so a static name would let
    // two users/sessions collide. The SID suffix isolates each user; the
    // ACL below enforces it. There is deliberately no fallback to the
    // legacy machine-wide name.
    let sid = security::current_user_sid_string()?;
    Ok(format!(r"\\.\pipe\{IPC_NAME}-{sid}"))
}

#[cfg(windows)]
pub(crate) async fn run_ipc_windows_with_ready<F>(
    api: Arc<ControlApi>,
    on_ready: F,
) -> std::io::Result<()>
where
    F: FnOnce() + Send,
{
    use tokio::net::windows::named_pipe::ServerOptions;

    let name = ipc_pipe_name()?;
    let mut on_ready = Some(on_ready);
    let mut first = true;
    loop {
        if api.core().is_shutdown() {
            return Ok(());
        }
        // The first instance claims the name (proving no live owner) and
        // locks the ACL down; later turns add instances to our own name.
        let server = if first {
            first = false;
            let claimed = security::claim_pipe_instance(&name).await?;
            tracing::info!(pipe = %name, "control IPC listening");
            claimed
        } else {
            ServerOptions::new()
                .first_pipe_instance(false)
                .create(&name)?
        };
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
