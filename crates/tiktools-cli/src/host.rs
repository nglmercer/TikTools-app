//! Headless host serving (`rpc --stdio`, `host --stdio`, `host --ipc`).

use tiktools_control_api::ControlApi;

use super::client::headless_api;

/// `host --ipc` acquires the OS control-host ownership primitive. A connect
/// probe is not a lock (it races startup); ownership failure is the single
/// authority for "another host is running".
pub async fn serve_ipc_guarded() -> i32 {
    let ownership = match tiktools_control_api::ownership::acquire_control_host_ownership() {
        Ok(ownership) => ownership,
        Err(error) => return ipc_error_exit(error),
    };
    serve_ipc(headless_api(), ownership).await
}

pub async fn serve_stdio(api: ControlApi, events: bool) -> i32 {
    // Headless hosts own the plugin lifecycle exactly like the desktop:
    // polling starts with the host, never with a UI. Idempotent.
    api.core()
        .spawn_plugin_event_poll(&tokio::runtime::Handle::current());
    if let Err(error) = run_until_shutdown(api, events, None).await {
        eprintln!("tiktools: stdio host failed: {error}");
        return 3;
    }
    0
}

async fn serve_ipc(
    api: ControlApi,
    ownership: tiktools_control_api::ownership::ControlHostGuard,
) -> i32 {
    // Headless hosts own the plugin lifecycle exactly like the desktop:
    // polling starts with the host, never with a UI. Idempotent.
    api.core()
        .spawn_plugin_event_poll(&tokio::runtime::Handle::current());
    if let Err(error) = run_until_shutdown(api, true, Some(ownership)).await {
        return ipc_error_exit(error);
    }
    0
}

fn ipc_error_exit(error: std::io::Error) -> i32 {
    if error.kind() == std::io::ErrorKind::AddrInUse {
        eprintln!("tiktools: a control host is already running on the local IPC endpoint");
        1
    } else {
        eprintln!("tiktools: IPC host failed: {error}");
        3
    }
}

/// Signals and EOF use the same AppCore shutdown path as system.shutdown.
async fn run_until_shutdown(
    api: ControlApi,
    events: bool,
    ownership: Option<tiktools_control_api::ownership::ControlHostGuard>,
) -> std::io::Result<()> {
    let ipc = ownership.is_some();
    let core = std::sync::Arc::clone(api.core());
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let signal = async {
        #[cfg(unix)]
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
        #[cfg(not(unix))]
        tokio::signal::ctrl_c().await
    };
    let serving = async {
        if let Some(ownership) = ownership {
            tiktools_control_api::transport::run_ipc_owned(
                std::sync::Arc::new(api),
                ownership,
                || {},
            )
            .await
        } else {
            tiktools_control_api::run_stdio(api, events).await
        }
    };
    tokio::pin!(serving);
    let result = tokio::select! {
        result = &mut serving => result,
        result = signal => {
            core.shutdown().await;
            result?;
            if ipc {
                // Let the listener unlink its endpoint while ownership is held.
                serving.await
            } else {
                Ok(())
            }
        }
    };
    core.shutdown().await;
    result
}
