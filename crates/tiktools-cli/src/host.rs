//! Headless host serving (`rpc --stdio`, `host --stdio`, `host --ipc`).

use tiktools_control_api::ControlApi;

use super::client::headless_api;

/// `host --ipc` acquires the OS control-host ownership primitive. A connect
/// probe is not a lock (it races startup); ownership failure is the single
/// authority for "another host is running".
pub async fn serve_ipc_guarded() -> i32 {
    serve_ipc(headless_api()).await
}

pub async fn serve_stdio(api: ControlApi, events: bool) -> i32 {
    // Headless hosts own the plugin lifecycle exactly like the desktop:
    // polling starts with the host, never with a UI. Idempotent.
    api.core()
        .spawn_plugin_event_poll(&tokio::runtime::Handle::current());
    if let Err(error) = tiktools_control_api::run_stdio(api, events).await {
        eprintln!("tiktools: stdio host failed: {error}");
        return 3;
    }
    0
}

pub async fn serve_ipc(api: ControlApi) -> i32 {
    // Headless hosts own the plugin lifecycle exactly like the desktop:
    // polling starts with the host, never with a UI. Idempotent.
    api.core()
        .spawn_plugin_event_poll(&tokio::runtime::Handle::current());
    if let Err(error) = tiktools_control_api::run_ipc(api).await {
        if error.kind() == std::io::ErrorKind::AddrInUse {
            eprintln!("tiktools: a control host is already running on the local IPC endpoint");
            return 1;
        }
        eprintln!("tiktools: IPC host failed: {error}");
        return 3;
    }
    0
}
