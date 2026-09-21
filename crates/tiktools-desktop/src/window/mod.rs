mod domain_events;
mod ipc;
mod lifecycle;
mod outbox;
#[cfg(test)]
mod tests;

use crate::event::DesktopEvent;
use crate::tray::TrayController;
use crate::webview::FrontendSource;
use outbox::WebviewOutbox;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tiktools_control_api::ControlApi;
use tiktools_core::ipc::IpcRouter;
use tiktools_core::AppCore;
use tokio::runtime::Handle;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;
use wry::WebView;

pub struct DesktopApp {
    window: Option<Window>,
    webview: Option<WebView>,
    core: Arc<AppCore>,
    router: Arc<IpcRouter>,
    control: Arc<ControlApi>,
    frontend: FrontendSource,
    runtime: Handle,
    proxy: EventLoopProxy<DesktopEvent>,
    tray: Option<TrayController>,
    pending_host_messages: WebviewOutbox,
    shutting_down: bool,
    startup_state: StartupState,
    startup_deadline: Option<Instant>,
    pending_activation: bool,
    /// Shared with the Wry IPC handler: while set, inbound RPC is
    /// rejected with a transport failure instead of queueing work for a
    /// dead page. Cleared when the reloaded page handshakes.
    webview_failed: Arc<AtomicBool>,
    /// A reload/recreate is waiting for the fresh page's handshake, which
    /// recovers the failed transport (see `frontend_ready`).
    reload_pending: bool,
    log_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupState {
    Initializing,
    WebViewLoading,
    Ready,
    Failed,
    ShuttingDown,
}

/// How long the hidden window waits for the `frontend-ready` IPC before
/// reporting a startup failure. The deadline bounds total startup time; on
/// Linux the loop still wakes every pump interval inside it (see
/// `platform::prepare_for_startup_wait`) so GTK/WebKit keeps progressing.
const FRONTEND_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
