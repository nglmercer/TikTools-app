use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use tiktools_control_api::ControlApi;
use tiktools_core::{ipc::IpcRouter, AppCore};
use tokio::runtime::Handle;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy},
    window::{Icon as WindowIcon, Window, WindowId},
};
use wry::{
    dpi::{PhysicalPosition as WryPhysicalPosition, PhysicalSize as WryPhysicalSize},
    PageLoadEvent, Rect, WebView, WebViewBuilder,
};

use crate::{
    event::{DesktopCommand, DesktopEvent},
    platform,
    tray::TrayController,
    webview::FrontendSource,
};

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

impl DesktopApp {
    pub fn new(
        core: Arc<AppCore>,
        router: Arc<IpcRouter>,
        frontend: FrontendSource,
        runtime: Handle,
        proxy: EventLoopProxy<DesktopEvent>,
        log_path: PathBuf,
    ) -> Self {
        let control = Arc::new(ControlApi::new(core.clone()));
        // Persistent local IPC shares the desktop's one AppCore: the CLI
        // and agents observe the same runtime the WebView drives. The
        // server runs on Tokio without blocking Winit and exits on core
        // shutdown; a second desktop instance never gets here because the
        // single-instance guard exits it first. Failures are never silent:
        // the core health degrades, the UI thread is notified (startup
        // fails fast when IPC is already owned), and the server retries.
        {
            let control = control.clone();
            let core = core.clone();
            let proxy = proxy.clone();
            runtime.spawn(async move {
                let mut first = true;
                loop {
                    if core.is_shutdown() {
                        break;
                    }
                    // The ready callback clears degraded health only once the
                    // endpoint is actually listening again, so a recovered
                    // retry genuinely restores CLI/agent connectivity.
                    let ready_core = core.clone();
                    match tiktools_control_api::run_ipc_shared_with_ready(
                        control.clone(),
                        move || ready_core.set_ipc_error(None),
                    )
                    .await
                    {
                        Ok(()) => {
                            core.set_ipc_error(None);
                            break;
                        }
                        Err(error) => {
                            let owned_elsewhere =
                                error.kind() == std::io::ErrorKind::AddrInUse;
                            let message = if owned_elsewhere {
                                "control IPC is owned by another host; CLI/agents cannot reach this desktop"
                                    .to_owned()
                            } else {
                                format!("control IPC unavailable ({error}); CLI/agents cannot reach this desktop")
                            };
                            tracing::error!(%error, "control IPC server failed");
                            core.set_ipc_error(Some(message.clone()));
                            let _ = proxy.send_event(DesktopEvent::Command(
                                DesktopCommand::IpcFailed(message),
                            ));
                            if core.is_shutdown() {
                                break;
                            }
                            // Ownership conflicts retry slowly (the owner may
                            // be a stale host that is shutting down); other
                            // errors retry on the same cadence.
                            let _ = first;
                            first = false;
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            });
        }
        // Plugin polling belongs to the host/runtime lifecycle, not the
        // WebView: spontaneous plugin events (hotkeys, timers) must flow
        // even before the frontend handshakes, across reloads, and while
        // the window is hidden. Idempotent; the frontend-ready path must
        // never own it.
        core.spawn_plugin_event_poll(&runtime);
        // Forward domain events to the WebView as JSON-RPC `event`
        // notifications so the frontend control client observes the same
        // bus as CLI/IPC streaming clients.
        {
            let events = control.subscribe();
            let core = core.clone();
            let proxy = proxy.clone();
            runtime.spawn(forward_domain_events(events, core, move |notification| {
                let _ = proxy.send_event(DesktopEvent::Command(DesktopCommand::EmitToWebview(
                    notification,
                )));
            }));
        }
        Self {
            window: None,
            webview: None,
            core,
            router,
            control,
            frontend,
            runtime,
            proxy,
            tray: None,
            pending_host_messages: WebviewOutbox::new(),
            shutting_down: false,
            startup_state: StartupState::Initializing,
            startup_deadline: None,
            pending_activation: false,
            webview_failed: Arc::new(AtomicBool::new(false)),
            reload_pending: false,
            log_path,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        tracing::debug!("creating desktop window");
        let window_icon =
            WindowIcon::from_rgba(crate::icon::rgba(), crate::icon::SIZE, crate::icon::SIZE)
                .map_err(|error| format!("could not create window icon: {error}"))?;
        let attributes = Window::default_attributes()
            .with_title("TikTools")
            .with_inner_size(LogicalSize::new(900_u32, 680_u32))
            .with_resizable(true)
            .with_window_icon(Some(window_icon))
            .with_visible(false);
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| format!("could not create window: {error}"))?;

        let router = self.router.clone();
        let control = self.control.clone();
        let runtime = self.runtime.clone();
        let proxy_for_ipc = self.proxy.clone();
        let transport_failed = self.webview_failed.clone();
        let navigation_frontend = self.frontend.clone();
        let mut builder = WebViewBuilder::new()
            .with_devtools(cfg!(debug_assertions) || cfg!(feature = "devtools"))
            .with_focused(false)
            .with_autoplay(true)
            .with_navigation_handler(move |url| {
                let allowed = navigation_frontend.allows_navigation(&url);
                if !allowed {
                    tracing::warn!(url = %url, "blocked WebView navigation outside the application frontend");
                }
                allowed
            })
            .with_new_window_req_handler(move |url, _features| {
                tracing::debug!(url = %url, "blocked WebView new-window request");
                wry::NewWindowResponse::Deny
            })
            .with_on_page_load_handler(|event, url| {
                match event {
                    PageLoadEvent::Started => {
                        tracing::debug!(url = %url, "frontend page load started");
                    }
                    PageLoadEvent::Finished => {
                        tracing::debug!(url = %url, "frontend page load finished; waiting for frontend-ready");
                    }
                }
            })
            .with_ipc_handler(move |request| {
                let raw = request.body().clone();
                // The Winit callback never parses JSON: size-check, match
                // the tiny boot handshake, then move the payload to Tokio,
                // where it is parsed exactly once and classified on the
                // value. This keeps malicious or very large valid JSON off
                // the UI loop.
                // Same transport limits as local IPC: reject oversized
                // payloads before any JSON parsing so the WebView cannot
                // bypass them. (`MAX_PARAMS_BYTES` is enforced inside
                // `ControlApi::execute`, which all WebView RPCs traverse.)
                if webview_request_too_large(&raw) {
                    if is_probably_control_rpc(&raw) {
                        // Preserve the request id so the frontend resolves
                        // (rejects) the pending call instead of hanging it
                        // until timeout; the id sits in the bounded prefix.
                        let response = tiktools_control_api::RpcResponse::error(
                            tiktools_control_api::RpcId::extract_from_prefix(&raw),
                            tiktools_control_api::ApiError::too_large(),
                        );
                        emit_rpc_response(&proxy_for_ipc, &response);
                    } else {
                        tracing::warn!(
                            bytes = raw.len(),
                            "oversized WebView IPC message dropped"
                        );
                    }
                    return;
                }
                if is_frontend_ready_fast(&raw) {
                    let _ = proxy_for_ipc.send_event(DesktopEvent::Command(
                        DesktopCommand::FrontendReady,
                    ));
                    return;
                }
                let router = router.clone();
                let control = control.clone();
                let proxy = proxy_for_ipc.clone();
                let transport_failed = transport_failed.clone();
                runtime.spawn(async move {
                    handle_webview_ipc_message(
                        raw,
                        &router,
                        &control,
                        &proxy,
                        &transport_failed,
                    )
                    .await;
                });
            });

        // Wry's Linux/X11 child-window path converts logical default bounds
        // using the X11 screen millimeter dimensions. Some XWayland/KDE
        // sessions report those dimensions as zero, which produces an invalid
        // scale factor before the WebView is even attached. The window resize
        // event already gives us physical pixels, so keep this boundary
        // physical and avoid that conversion entirely.
        let initial_size = window.inner_size();
        builder = builder.with_bounds(Rect {
            position: WryPhysicalPosition::new(0, 0).into(),
            size: WryPhysicalSize::new(initial_size.width.max(1), initial_size.height.max(1))
                .into(),
        });

        if let Some(assets) = self.frontend.asset_server() {
            builder = builder.with_custom_protocol("tiktools".to_owned(), move |_id, request| {
                assets.respond(request)
            });
        }
        builder = builder.with_url(self.frontend.url().as_str());
        tracing::debug!(
            url = %self.frontend.url(),
            "creating TikTools frontend WebView"
        );
        let webview = platform::build_webview(builder, &window)
            .map_err(|error| format!("could not create Wry WebView: {error}"))?;
        tracing::debug!("Wry WebView created successfully");

        self.window = Some(window);
        self.webview = Some(webview);
        self.startup_state = StartupState::WebViewLoading;
        self.startup_deadline = Some(Instant::now() + FRONTEND_STARTUP_TIMEOUT);
        tracing::debug!("waiting for frontend-ready");
        if self.tray.is_none() {
            match TrayController::create(self.proxy.clone()) {
                Ok(tray) => self.tray = Some(tray),
                Err(error) => {
                    tracing::warn!(%error, "system tray is unavailable; window remains usable")
                }
            }
        }
        self.flush_host_messages();
        Ok(())
    }

    fn flush_host_messages(&mut self) {
        self.flush_webview_batch();
    }

    /// Two-lane enqueue: RPC responses and critical transitions land in
    /// the reliable lane (never dropped); snapshots coalesce and feed is
    /// bounded in the lossy lane. A tripped reliable safety policy fails
    /// the transport loudly instead of growing memory without bound.
    fn emit_to_webview(&mut self, message: String) {
        self.pending_host_messages.push(message);
        if self.pending_host_messages.take_transport_failure() {
            self.fail_webview_transport();
        }
    }

    /// Fails the WebView transport loudly after the reliable outbox
    /// exceeded its safety policy: health degrades, stale queued messages
    /// are cleared for the rebooting page, new RPC is rejected, and the
    /// WebView reloads. The latch releases when the fresh page completes
    /// its handshake (see `frontend_ready`); without that handshake the
    /// transport stays failed rather than delivering to a dead page.
    fn fail_webview_transport(&mut self) {
        self.webview_failed.store(true, Ordering::SeqCst);
        self.core.set_webview_error(Some(
            "WebView transport failed: reliable backlog overflow; reloading".to_owned(),
        ));
        self.pending_host_messages.clear_for_reload();
        match self.webview.as_ref() {
            Some(webview) => match webview.reload() {
                Ok(()) => {
                    tracing::error!("WebView reloaded after reliable backlog overflow");
                    self.reload_pending = true;
                }
                Err(error) => {
                    tracing::error!(%error, "WebView reload failed after reliable backlog overflow; RPC stays rejected");
                }
            },
            None => {
                tracing::error!(
                    "WebView transport failed with no WebView to reload; RPC stays rejected"
                );
            }
        }
    }

    /// Drains up to one batch per UI tick through a single
    /// `evaluate_script` call. The frontend fans the batch out to its
    /// normal per-message dispatch.
    fn flush_webview_batch(&mut self) {
        let Some(webview) = self.webview.as_ref() else {
            return;
        };
        if self.pending_host_messages.is_empty() {
            return;
        }
        let batch = self.pending_host_messages.take_batch();
        let argument = match serde_json::to_string(&batch) {
            Ok(argument) => argument,
            Err(error) => {
                tracing::error!(%error, "could not encode host message batch for JavaScript");
                return;
            }
        };
        tracing::debug!(
            count = batch.len(),
            bytes = argument.len(),
            "delivering host message batch to WebView"
        );
        let script = format!(
            "if (typeof window.__tiktools_receive_batch__ === 'function') {{ window.__tiktools_receive_batch__({argument}); }} else {{ const batch = {argument}; for (const item of batch) {{ if (typeof window.__webview_on_message__ === 'function') {{ window.__webview_on_message__(item); }} else {{ const queue = window.__tiktools_host_message_queue__ || (window.__tiktools_host_message_queue__ = []); if (queue.length < 512) queue.push(item); }} }} }}"
        );
        if let Err(error) = webview.evaluate_script(&script) {
            if !self.shutting_down {
                tracing::debug!(%error, "could not deliver host message batch to WebView");
            }
        }
    }

    fn resize_webview(&self, size: PhysicalSize<u32>) {
        let Some(webview) = self.webview.as_ref() else {
            return;
        };
        let bounds = Rect {
            position: WryPhysicalPosition::new(0, 0).into(),
            size: WryPhysicalSize::new(size.width.max(1), size.height.max(1)).into(),
        };
        if let Err(error) = webview.set_bounds(bounds) {
            tracing::debug!(%error, "could not resize WebView");
        }
    }

    fn on_keyboard_input(&self, event: winit::event::KeyEvent) {
        use winit::event::ElementState;
        use winit::keyboard::{KeyCode, PhysicalKey};
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }
        // F12 is the conventional inspector shortcut. WebView2 also handles
        // it natively when devtools are enabled; this covers the other
        // backends and guarantees the shortcut exists.
        if !matches!(event.physical_key, PhysicalKey::Code(KeyCode::F12)) {
            return;
        }
        self.open_devtools();
    }

    #[cfg(any(debug_assertions, feature = "devtools"))]
    fn open_devtools(&self) {
        if let Some(webview) = self.webview.as_ref() {
            webview.open_devtools();
        }
    }

    /// Release builds without the `devtools` feature have no inspector API;
    /// the shortcut and tray item stay compiled but are intentional no-ops.
    #[cfg(not(any(debug_assertions, feature = "devtools")))]
    fn open_devtools(&self) {}

    fn set_window_visible(&self, visible: bool) {
        if let Some(webview) = self.webview.as_ref() {
            if let Err(error) = webview.set_visible(visible) {
                tracing::debug!(%error, visible, "could not change WebView visibility");
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.set_visible(visible);
        }
    }

    fn shutdown(&mut self, _event_loop: &ActiveEventLoop) {
        if self.shutting_down {
            return;
        }
        self.shutting_down = true;
        self.startup_state = StartupState::ShuttingDown;
        self.startup_deadline = None;
        let core = Arc::clone(&self.core);
        let proxy = self.proxy.clone();
        self.runtime.spawn(async move {
            core.shutdown().await;
            let _ = proxy.send_event(DesktopEvent::Command(DesktopCommand::ShutdownComplete));
        });
    }

    fn finalize_shutdown(&mut self, event_loop: &ActiveEventLoop) {
        self.startup_state = StartupState::ShuttingDown;
        self.tray.take();
        self.webview.take();
        self.window.take();
        event_loop.exit();
    }

    fn frontend_ready(&mut self) {
        // A reload/recreate handshake recovers a failed transport only
        // once the fresh page is actually running: stale responses can
        // never mis-resolve the new page's calls, and the mount reads
        // resync authoritative state. Recovery runs before the startup
        // gate below so a mid-startup reload still completes startup.
        if self.reload_pending && !self.shutting_down {
            self.reload_pending = false;
            self.pending_host_messages.recover_transport();
            self.webview_failed.store(false, Ordering::SeqCst);
            self.core.set_webview_error(None);
            tracing::error!("WebView transport recovered after reload");
        }
        if self.shutting_down || self.startup_state == StartupState::Ready {
            return;
        }
        if self.startup_state != StartupState::WebViewLoading {
            tracing::debug!(state = ?self.startup_state, "ignoring frontend-ready outside WebView startup");
            return;
        }
        tracing::debug!("frontend-ready IPC received");
        self.startup_state = StartupState::Ready;
        self.startup_deadline = None;
        // Plugin polling is owned by host startup (see `DesktopApp::new`),
        // never by WebView readiness: no lifecycle call here.
        self.set_window_visible(true);
        if let Some(window) = self.window.as_ref() {
            window.focus_window();
        }
    }

    fn restore_window(&mut self) {
        if self.shutting_down {
            return;
        }
        if self.startup_state != StartupState::Ready {
            self.pending_activation = true;
            return;
        }
        self.pending_activation = false;
        if let Some(window) = self.window.as_ref() {
            window.set_minimized(false);
        }
        self.set_window_visible(true);
        if let Some(window) = self.window.as_ref() {
            window.focus_window();
        }
    }

    fn fail_startup(&mut self, event_loop: &ActiveEventLoop, reason: impl Into<String>) {
        if self.startup_state == StartupState::Failed || self.shutting_down {
            return;
        }
        let reason = reason.into();
        self.startup_state = StartupState::Failed;
        self.startup_deadline = None;
        tracing::error!(%reason, "TikTools frontend did not become ready");
        crate::show_startup_failure(&reason, &self.log_path);
        event_loop.exit();
    }
}

impl ApplicationHandler<DesktopEvent> for DesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        if self.window.is_some() {
            return;
        }
        if let Err(error) = self.create_window(event_loop) {
            tracing::error!(%error, "Rust desktop host could not start");
            crate::show_startup_failure(&error, &self.log_path);
            self.startup_state = StartupState::Failed;
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_some_and(|window| window.id() != window_id)
        {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.tray.is_some() {
                    tracing::debug!("window close requested; hiding TikTools in the tray");
                    self.set_window_visible(false);
                } else {
                    tracing::debug!("window close requested without a tray; shutting down");
                    self.shutdown(event_loop);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.on_keyboard_input(event),
            WindowEvent::Resized(size) => self.resize_webview(size),
            WindowEvent::Destroyed => {
                tracing::debug!(tray = self.tray.is_some(), "window was destroyed");
                self.webview.take();
                self.window.take();
                if self.tray.is_none() {
                    self.shutdown(event_loop);
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: DesktopEvent) {
        match event {
            DesktopEvent::Command(DesktopCommand::EmitToWebview(message)) => {
                self.emit_to_webview(message);
            }
            // Wake-only chaining for multi-batch bursts; the flush runs in
            // `about_to_wait`, so this arm intentionally does nothing.
            DesktopEvent::Command(DesktopCommand::FlushWebviewBatch) => {}
            DesktopEvent::Command(DesktopCommand::IpcFailed(message)) => {
                // A second owner at startup means a stale host is still
                // running: fail fast instead of mixing runtimes. After
                // startup the degraded `system.health` plus retry loop
                // keeps CLI/agent loss visible without killing the GUI.
                if self.startup_state != StartupState::Ready && !self.shutting_down {
                    self.fail_startup(
                        event_loop,
                        format!(
                            "The control IPC endpoint is already owned ({message}). Close the other TikTools host and restart."
                        ),
                    );
                } else {
                    tracing::error!(message, "control IPC unavailable; CLI/agents degraded");
                }
            }
            DesktopEvent::Command(DesktopCommand::FrontendReady) => self.frontend_ready(),
            DesktopEvent::Command(DesktopCommand::ShowWindow) => {
                if self.window.is_none() {
                    if let Err(error) = self.create_window(event_loop) {
                        tracing::error!(%error, "could not recreate TikTools window from tray");
                        return;
                    }
                    // A failed transport recreates with a fresh page: drop
                    // the stale backlog now; the mount handshake recovers
                    // the latch (see `frontend_ready`).
                    if self.pending_host_messages.transport_failed() {
                        self.pending_host_messages.clear_for_reload();
                        self.reload_pending = true;
                    }
                }
                self.restore_window();
            }
            DesktopEvent::Command(DesktopCommand::HideWindow) => {
                self.set_window_visible(false);
            }
            DesktopEvent::Command(DesktopCommand::OpenDevtools) => self.open_devtools(),
            DesktopEvent::Command(DesktopCommand::Quit) => self.shutdown(event_loop),
            DesktopEvent::Command(DesktopCommand::ShutdownComplete) => {
                self.finalize_shutdown(event_loop)
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        platform::pump();
        // One batch per UI tick: all host messages queued since the last
        // tick share a single `evaluate_script`.
        self.flush_webview_batch();
        // A batch is capped, so a burst longer than one batch must chain
        // exactly one wake per remaining batch instead of stalling under
        // ControlFlow::Wait with no further events. The wake event itself
        // is a no-op; the next turn's flush does the work. No busy loop:
        // one wake is scheduled per turn that still has backlog.
        if !self.pending_host_messages.is_empty() {
            let _ = self
                .proxy
                .send_event(DesktopEvent::Command(DesktopCommand::FlushWebviewBatch));
        }
        if self.startup_state == StartupState::WebViewLoading {
            if let Some(deadline) = self.startup_deadline {
                if Instant::now() >= deadline {
                    tracing::error!(
                        state = ?self.startup_state,
                        "frontend startup timeout"
                    );
                    self.fail_startup(
                        event_loop,
                        "The packaged web application did not become ready within 10 seconds.",
                    );
                    return;
                }
                // On Linux this wakes at the next GTK pump interval (capped by
                // the deadline) instead of sleeping through the whole startup
                // timeout; other platforms wait for the deadline directly.
                platform::prepare_for_startup_wait(event_loop, deadline);
                return;
            }
        }
        platform::prepare_for_wait(event_loop);
    }
}

/// Bound for the lossy lane (snapshots + feed). Past this point the
/// oldest lossy message is shed — never a reliable one.
const MAX_PENDING_WEBVIEW_MESSAGES: usize = 512;
/// Backlog size at which the reliable lane starts warning loudly. The
/// reliable lane is never dropped: a stuck/slow frontend shows up as a
/// growing backlog in logs instead of silently lost RPC responses.
const MAX_RELIABLE_WEBVIEW_BACKLOG: usize = 1024;
/// Reliable-lane safety policy: past either bound the transport is
/// broken (a stuck page that never drains), so it fails loudly —
/// health degrades, RPC is rejected, the WebView reloads — instead of
/// consuming unlimited memory. Reliable messages are never silently
/// evicted to stay under these bounds.
const MAX_RELIABLE_MESSAGES: usize = 4096;
const MAX_RELIABLE_BYTES: usize = 16 * 1024 * 1024;
/// Maximum messages delivered in one UI tick through one `evaluate_script`.
const MAX_BATCH_PER_TICK: usize = 128;

/// Delivery class for one queued WebView message. This is the single
/// classification shared by the queue policy and the tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebviewMessageClass {
    /// Never evicted under saturation: RPC responses, lifecycle and
    /// connection transitions, errors, shutdown, and anything unrecognized
    /// (the core rule forbids silently dropping a possible state change).
    Critical,
    /// Snapshots where only the latest per coalesce key is kept.
    Coalescable,
    /// High-rate feed where shedding the oldest under saturation is safe.
    Droppable,
}

/// Coalesce identity for snapshots, e.g. `legacy:room-stats`,
/// `domain:room.stats`, `domain:plugin.progress:<plugin-id>`.
fn webview_coalesce_key(value: &serde_json::Value) -> Option<String> {
    if classify_value(value) != WebviewMessageClass::Coalescable {
        return None;
    }
    if let Some(kind) = value.get("type").and_then(serde_json::Value::as_str) {
        return Some(format!("legacy:{kind}"));
    }
    let topic = value
        .get("params")
        .and_then(|params| params.get("topic"))
        .and_then(serde_json::Value::as_str)?;
    if topic == "plugin.progress" {
        let plugin = value
            .get("params")
            .and_then(|params| params.get("data"))
            .and_then(|data| data.get("pluginId"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
        return Some(format!("domain:{topic}:{plugin}"));
    }
    Some(format!("domain:{topic}"))
}

fn classify_value(value: &serde_json::Value) -> WebviewMessageClass {
    if let Some(kind) = value.get("type").and_then(serde_json::Value::as_str) {
        return match kind {
            // Compat duplicates of authoritative domain twins: safe to shed.
            "live-event" | "points-awarded" | "plugin-progress" => WebviewMessageClass::Droppable,
            "room-stats" | "leaderboard" | "analytics-summary" | "processor-status"
            | "automation-context" | "gift-catalog" => WebviewMessageClass::Coalescable,
            // rpc-response, connection/error/reconnecting transitions, and
            // every other rare state/result message: never evicted.
            _ => WebviewMessageClass::Critical,
        };
    }
    if value.get("method").and_then(serde_json::Value::as_str) == Some("event") {
        let topic = value
            .get("params")
            .and_then(|params| params.get("topic"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        return match topic {
            "room.stats" | "analytics.updated" | "plugin.progress" | "gifts.catalog" => {
                WebviewMessageClass::Coalescable
            }
            "live.event" | "live.ui-event" => WebviewMessageClass::Droppable,
            // Connection/lifecycle/state transitions, errors, shutdown,
            // and unknown topics: never evicted.
            _ => WebviewMessageClass::Critical,
        };
    }
    // Unrecognized shapes default to Critical per the core rule.
    WebviewMessageClass::Critical
}

fn classify_webview_message(message: &str) -> WebviewMessageClass {
    match serde_json::from_str::<serde_json::Value>(message) {
        Ok(value) => classify_value(&value),
        Err(_) => WebviewMessageClass::Critical,
    }
}

/// One lossy-lane message with its delivery class computed once at
/// enqueue time, so saturation scans never re-parse JSON on the UI thread.
#[derive(Debug)]
struct QueuedWebviewMessage {
    body: String,
    class: WebviewMessageClass,
    coalesce_key: Option<String>,
}

/// Two-lane outbox enforcing the core invariant: RPC responses and
/// critical state transitions must never silently disappear.
///
/// * `reliable` holds Critical messages (RPC responses, connection and
///   lifecycle transitions, errors, shutdown). It is never shed: a slow
///   frontend builds a loud backlog instead of losing results, up to the
///   `MAX_RELIABLE_*` safety policy, past which the whole transport
///   fails loudly (health, RPC rejection, reload) instead of growing
///   memory without bound.
/// * `lossy` holds Coalescable snapshots (latest per key wins) and
///   Droppable feed, bounded by `MAX_PENDING_WEBVIEW_MESSAGES` with
///   oldest-first shedding.
///
/// Ordering contract: FIFO within each lane; each batch drains reliable
/// first, then fills the remainder of the tick from lossy. A snapshot or
/// feed item may therefore be delivered after a reliable message that was
/// enqueued later — acceptable because snapshots supersede and feed is
/// explicitly lossy, while results and transitions stay prompt.
#[derive(Debug, Default)]
struct WebviewOutbox {
    reliable: std::collections::VecDeque<String>,
    lossy: std::collections::VecDeque<QueuedWebviewMessage>,
    /// Serialized bytes currently held in the reliable lane.
    reliable_bytes: usize,
    /// Latched when the reliable lane exceeds its safety policy. While
    /// set, new reliable messages are counted (never queued) and inbound
    /// RPC is rejected, until a clean reload recovers the transport.
    transport_failed: bool,
    /// Edge flag: set on the trip, taken by the UI loop to run the
    /// fail-loud handling exactly once per trip.
    failure_pending: bool,
    /// Reliable messages refused while the transport was failed. Counted
    /// and logged, never silent — but the page is dead, so queueing more
    /// would only pin memory no reader can consume.
    dropped_while_failed: u64,
}

impl WebviewOutbox {
    fn new() -> Self {
        Self::default()
    }

    fn is_empty(&self) -> bool {
        self.reliable.is_empty() && self.lossy.is_empty()
    }

    fn transport_failed(&self) -> bool {
        self.transport_failed
    }

    #[cfg(test)]
    fn reliable_bytes(&self) -> usize {
        self.reliable_bytes
    }

    /// Takes the failure edge exactly once per trip so the UI loop runs
    /// the fail-loud handling (health + reload) a single time.
    fn take_transport_failure(&mut self) -> bool {
        std::mem::take(&mut self.failure_pending)
    }

    /// Clears both lanes after the transport failed. The page is dead or
    /// rebooting, so retained messages are undeliverable; the rebooted
    /// frontend resyncs authoritative state through its mount reads.
    fn clear_for_reload(&mut self) {
        let reliable = self.reliable.len();
        let lossy = self.lossy.len();
        self.reliable.clear();
        self.lossy.clear();
        self.reliable_bytes = 0;
        tracing::error!(
            reliable,
            lossy,
            "cleared WebView outbox for transport reload"
        );
    }

    /// Releases the failure latch after the reloaded page handshakes. New
    /// messages queue again; the rebooted frontend resyncs through its
    /// mount reads.
    fn recover_transport(&mut self) {
        self.transport_failed = false;
        self.dropped_while_failed = 0;
    }

    fn push(&mut self, body: String) {
        if classify_webview_message(&body) == WebviewMessageClass::Critical {
            if self.transport_failed {
                self.dropped_while_failed += 1;
                if self.dropped_while_failed == 1 || self.dropped_while_failed.is_multiple_of(256) {
                    tracing::error!(
                        dropped = self.dropped_while_failed,
                        "WebView transport failed; refusing reliable backlog for a dead page"
                    );
                }
                return;
            }
            self.reliable_bytes += body.len();
            self.reliable.push_back(body);
            // Loud backlog instead of silent drops: warn once when the
            // budget is crossed, then once per additional budget of lag.
            let backlog = self.reliable.len();
            if backlog == MAX_RELIABLE_WEBVIEW_BACKLOG + 1
                || (backlog > MAX_RELIABLE_WEBVIEW_BACKLOG
                    && backlog.is_multiple_of(MAX_RELIABLE_WEBVIEW_BACKLOG))
            {
                tracing::error!(
                    backlog,
                    "WebView reliable backlog growing; RPC responses and transitions are held, not dropped"
                );
            }
            // Safety policy: never evict to stay under the bounds — fail
            // the whole transport loudly instead.
            if self.reliable.len() > MAX_RELIABLE_MESSAGES
                || self.reliable_bytes > MAX_RELIABLE_BYTES
            {
                self.transport_failed = true;
                self.failure_pending = true;
                tracing::error!(
                    messages = self.reliable.len(),
                    bytes = self.reliable_bytes,
                    "WebView reliable backlog exceeded the safety policy; failing the transport loudly"
                );
            }
            return;
        }
        let coalesce_key = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| webview_coalesce_key(&value));
        let class = classify_webview_message(&body);
        if class == WebviewMessageClass::Coalescable {
            // Keep only the latest snapshot per coalesce key.
            if let Some(key) = coalesce_key.as_deref() {
                self.lossy
                    .retain(|queued| queued.coalesce_key.as_deref() != Some(key));
            }
        }
        self.lossy.push_back(QueuedWebviewMessage {
            body,
            class,
            coalesce_key,
        });
        // Bounded lossy lane: shed oldest droppable feed first, then the
        // oldest snapshot. Reliable messages live in the other lane and
        // are never candidates here.
        while self.lossy.len() > MAX_PENDING_WEBVIEW_MESSAGES {
            if let Some(index) = self
                .lossy
                .iter()
                .position(|queued| queued.class == WebviewMessageClass::Droppable)
            {
                self.lossy.remove(index);
            } else {
                self.lossy.pop_front();
            }
        }
    }

    /// Takes at most one UI tick's worth of messages: reliable first,
    /// then lossy fill. FIFO within each lane.
    fn take_batch(&mut self) -> Vec<String> {
        let mut batch = Vec::new();
        while batch.len() < MAX_BATCH_PER_TICK {
            if let Some(body) = self.reliable.pop_front() {
                self.reliable_bytes = self.reliable_bytes.saturating_sub(body.len());
                batch.push(body);
            } else {
                break;
            }
        }
        while batch.len() < MAX_BATCH_PER_TICK {
            if let Some(queued) = self.lossy.pop_front() {
                batch.push(queued.body);
            } else {
                break;
            }
        }
        batch
    }
}

/// Forwards domain events to a UI sink as JSON-RPC `event` notifications.
/// Reliable lag emits an explicit `event.gap` notification (and records
/// it for `system.health`) so the frontend resyncs instead of assuming a
/// complete stream; lossy lag just skips. Only a closed channel or the
/// shutdown event terminates the forwarder, so a temporary burst never
/// permanently disables WebView events.
async fn forward_domain_events(
    mut events: tiktools_core::events::DomainSubscription,
    core: Arc<AppCore>,
    mut send: impl FnMut(String),
) {
    use tiktools_core::events::DomainRecvError;
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(DomainRecvError::ReliableLagged(lost)) => {
                tracing::warn!(
                    lost,
                    "WebView domain event receiver lagged on the reliable lane"
                );
                core.record_event_gap();
                send(tiktools_control_api::gap_notification(lost).to_string());
                continue;
            }
            Err(DomainRecvError::LossyLagged(skipped)) => {
                tracing::debug!(
                    skipped,
                    "WebView domain event receiver skipped a lossy burst"
                );
                continue;
            }
            Err(DomainRecvError::Closed) => break,
        };
        let shutdown = matches!(event, tiktools_core::events::DomainEvent::Shutdown);
        let notification = tiktools_control_api::event_notification(&event);
        send(notification.to_string());
        if shutdown {
            break;
        }
    }
}

/// Raw inbound bound shared with local IPC. Checked before any parsing.
fn webview_request_too_large(raw: &str) -> bool {
    raw.len() > tiktools_control_api::MAX_REQUEST_BYTES
}

/// Allocation-free control-shape probe used only to route oversized-payload
/// errors and malformed input (full parsing happens later, on size-capped
/// input, off the UI thread).
fn is_probably_control_rpc(raw: &str) -> bool {
    raw.contains("\"method\"")
}

/// Allocation-free boot-handshake probe for the Winit thread: exact match
/// on the `notifyFrontendReady` payload (plus ASCII-whitespace tolerance).
/// Anything else — including handshake whitespace variants — moves to
/// Tokio, where the parsed classifier recognizes `frontend-ready`
/// robustly. The length gate alone rejects large JSON without parsing.
fn is_frontend_ready_fast(raw: &str) -> bool {
    if raw.len() > 128 {
        return false;
    }
    raw.trim_matches(|character: char| character.is_ascii_whitespace())
        == r#"{"type":"frontend-ready"}"#
}

/// One parsed inbound route. The payload is parsed exactly once, off the
/// UI thread; classification runs on the value, never by re-scanning the
/// raw text.
#[derive(Debug, PartialEq, Eq)]
enum InboundRoute {
    /// JSON-RPC control call: execute through the shared ControlApi.
    Control(serde_json::Value),
    /// Legacy `{"type": ...}` page message.
    Legacy(serde_json::Value),
    /// Boot handshake in a shape the Winit fast path did not match
    /// (whitespace variants): complete startup without touching routers.
    FrontendReady,
    /// Unparseable but control-shaped: answer with a correlated RPC error
    /// carrying the parse failure.
    MalformedControl(String),
    /// Unparseable legacy-shaped input: diagnose, never route to ControlApi.
    MalformedLegacy,
}

/// Parses once and classifies on the value. Control-plane messages carry
/// `method` (JSON-RPC style); legacy WebView messages carry `type`
/// (PageMessage). The two shapes never overlap.
fn classify_inbound_message(raw: &str) -> InboundRoute {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(value) => {
            if value
                .get("method")
                .and_then(serde_json::Value::as_str)
                .is_some()
            {
                InboundRoute::Control(value)
            } else if value.get("type").and_then(serde_json::Value::as_str)
                == Some("frontend-ready")
            {
                InboundRoute::FrontendReady
            } else {
                InboundRoute::Legacy(value)
            }
        }
        Err(error) => {
            if is_probably_control_rpc(raw) {
                InboundRoute::MalformedControl(error.to_string())
            } else {
                InboundRoute::MalformedLegacy
            }
        }
    }
}

/// Tokio-side inbound dispatch: parse once, classify on the value,
/// execute. Control calls share the ControlApi with CLI/stdio/IPC;
/// legacy messages keep the PageMessage path during migration.
async fn handle_webview_ipc_message(
    raw: String,
    router: &Arc<IpcRouter>,
    control: &Arc<ControlApi>,
    proxy: &EventLoopProxy<DesktopEvent>,
    transport_failed: &AtomicBool,
) {
    // A failed transport rejects RPC loudly instead of queueing work for
    // a dead page; legacy input is diagnosed and dropped.
    if transport_failed.load(Ordering::SeqCst) {
        handle_failed_transport_message(&raw, proxy);
        return;
    }
    match classify_inbound_message(&raw) {
        InboundRoute::Control(value) => {
            let response = control.execute_value(&value).await;
            emit_rpc_response(proxy, &response);
        }
        InboundRoute::Legacy(value) => {
            if let Err(error) = router.dispatch_value(&value).await {
                tracing::warn!(%error, "invalid WebView IPC message");
            }
        }
        InboundRoute::FrontendReady => {
            let _ = proxy.send_event(DesktopEvent::Command(DesktopCommand::FrontendReady));
        }
        InboundRoute::MalformedControl(error) => {
            let response = tiktools_control_api::RpcResponse::error(
                tiktools_control_api::RpcId::extract_from_prefix(&raw),
                tiktools_control_api::ApiError::invalid_params(format!("invalid JSON: {error}")),
            );
            emit_rpc_response(proxy, &response);
        }
        InboundRoute::MalformedLegacy => {
            tracing::warn!("invalid legacy WebView message");
        }
    }
}

/// Inbound dispatch while the transport is failed: control calls are
/// rejected with a transport failure (the id lets the rebooting frontend
/// fail the call instead of hanging it), the boot handshake still flows
/// so recovery can complete, and everything else is diagnosed.
fn handle_failed_transport_message(raw: &str, proxy: &EventLoopProxy<DesktopEvent>) {
    match classify_inbound_message(raw) {
        InboundRoute::Control(value) => {
            let response = tiktools_control_api::RpcResponse::error(
                tiktools_control_api::RpcId::extract(&value),
                tiktools_control_api::ApiError::new(
                    "transport",
                    "WebView transport failed; the UI is reloading",
                ),
            );
            emit_rpc_response(proxy, &response);
        }
        InboundRoute::MalformedControl(error) => {
            let response = tiktools_control_api::RpcResponse::error(
                tiktools_control_api::RpcId::extract_from_prefix(raw),
                tiktools_control_api::ApiError::new(
                    "transport",
                    format!("WebView transport failed; dropping malformed input (invalid JSON: {error})"),
                ),
            );
            emit_rpc_response(proxy, &response);
        }
        InboundRoute::FrontendReady => {
            let _ = proxy.send_event(DesktopEvent::Command(DesktopCommand::FrontendReady));
        }
        InboundRoute::Legacy(_) | InboundRoute::MalformedLegacy => {
            tracing::warn!("dropping WebView IPC while the transport is failed");
        }
    }
}

/// Sends one RPC response to the frontend through the host-message batch
/// path (reliable lane: never shed).
fn emit_rpc_response(
    proxy: &EventLoopProxy<DesktopEvent>,
    response: &tiktools_control_api::RpcResponse,
) {
    let payload = serde_json::json!({
        "type": "rpc-response",
        "response": response,
    });
    let _ = proxy.send_event(DesktopEvent::Command(DesktopCommand::EmitToWebview(
        payload.to_string(),
    )));
}

#[cfg(test)]
mod webview_queue_tests {
    use super::*;

    fn domain_event(topic: &str, data: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","method":"event","params":{{"topic":"{topic}","data":{data}}}}}"#
        )
    }

    #[test]
    fn classifier_assigns_plan_classes() {
        use WebviewMessageClass::{Coalescable, Critical, Droppable};
        // Critical: RPC responses, transitions, errors, shutdown, unknowns.
        assert_eq!(
            classify_webview_message(r#"{"type":"rpc-response","response":{}}"#),
            Critical
        );
        assert_eq!(
            classify_webview_message(r#"{"type":"error","message":"x"}"#),
            Critical
        );
        assert_eq!(
            classify_webview_message(&domain_event("live.connected", "{}")),
            Critical
        );
        assert_eq!(
            classify_webview_message(&domain_event("live.disconnected", "{}")),
            Critical
        );
        assert_eq!(
            classify_webview_message(&domain_event("plugin.started", "{}")),
            Critical
        );
        assert_eq!(
            classify_webview_message(&domain_event("points.changed", "{}")),
            Critical
        );
        assert_eq!(
            classify_webview_message(&domain_event("shutdown", "{}")),
            Critical
        );
        // The gap/resync signal is authoritative: it travels the reliable
        // lane, never shed under saturation.
        assert_eq!(
            classify_webview_message(
                r#"{"jsonrpc":"2.0","method":"event.gap","params":{"lost":12,"resync":true}}"#
            ),
            Critical
        );
        assert_eq!(
            classify_webview_message(&domain_event("future.unknown", "{}")),
            Critical
        );
        assert_eq!(classify_webview_message("not json {{{"), Critical);
        // Coalescable: legacy and domain snapshots.
        assert_eq!(
            classify_webview_message(r#"{"type":"room-stats"}"#),
            Coalescable
        );
        assert_eq!(
            classify_webview_message(r#"{"type":"leaderboard"}"#),
            Coalescable
        );
        assert_eq!(
            classify_webview_message(&domain_event("room.stats", "{}")),
            Coalescable
        );
        assert_eq!(
            classify_webview_message(&domain_event("analytics.updated", "{}")),
            Coalescable
        );
        assert_eq!(
            classify_webview_message(&domain_event("gifts.catalog", "{}")),
            Coalescable
        );
        assert_eq!(
            classify_webview_message(&domain_event("plugin.progress", "{}")),
            Coalescable
        );
        // Droppable: high-rate feed plus compat duplicates of domain twins.
        assert_eq!(
            classify_webview_message(&domain_event("live.ui-event", "{}")),
            Droppable
        );
        assert_eq!(
            classify_webview_message(&domain_event("live.event", "{}")),
            Droppable
        );
        assert_eq!(
            classify_webview_message(r#"{"type":"live-event"}"#),
            Droppable
        );
        assert_eq!(
            classify_webview_message(r#"{"type":"points-awarded"}"#),
            Droppable
        );
    }

    #[test]
    fn coalesces_disposable_snapshots() {
        let mut outbox = WebviewOutbox::new();
        outbox
            .push(r#"{"type":"room-stats","viewers":1,"totalUsers":1,"topViewers":[]}"#.to_owned());
        outbox
            .push(r#"{"type":"room-stats","viewers":2,"totalUsers":2,"topViewers":[]}"#.to_owned());
        outbox.push(r#"{"type":"live-event","event":{"kind":"chat"}}"#.to_owned());
        assert_eq!(outbox.lossy.len(), 2);
        assert!(outbox.lossy[0].body.contains("\"viewers\":2"));
        assert!(outbox.reliable.is_empty());
    }

    #[test]
    fn domain_snapshots_coalesce_but_feed_and_lifecycle_do_not() {
        let mut outbox = WebviewOutbox::new();
        outbox.push(domain_event("room.stats", r#"{"viewers":1}"#));
        outbox.push(domain_event("room.stats", r#"{"viewers":2}"#));
        outbox.push(domain_event(
            "analytics.updated",
            r#"{"creatorUniqueId":"a"}"#,
        ));
        outbox.push(domain_event(
            "analytics.updated",
            r#"{"creatorUniqueId":"b"}"#,
        ));
        // Same plugin collapses; a different plugin is a separate stream.
        outbox.push(domain_event(
            "plugin.progress",
            r#"{"pluginId":"p1","state":"loading"}"#,
        ));
        outbox.push(domain_event(
            "plugin.progress",
            r#"{"pluginId":"p1","state":"ready"}"#,
        ));
        outbox.push(domain_event(
            "plugin.progress",
            r#"{"pluginId":"p2","state":"ready"}"#,
        ));
        // Feed is never coalesced.
        outbox.push(domain_event("live.ui-event", r#"{"event":{"n":1}}"#));
        outbox.push(domain_event("live.ui-event", r#"{"event":{"n":2}}"#));
        // Lifecycle is reliable: never coalesced, never shed.
        outbox.push(domain_event("plugin.started", r#"{"pluginId":"p1"}"#));
        outbox.push(domain_event("plugin.started", r#"{"pluginId":"p1"}"#));
        assert_eq!(outbox.lossy.len(), 6, "unexpected lossy len");
        assert_eq!(outbox.reliable.len(), 2, "unexpected reliable len");
        assert!(outbox.lossy[0].body.contains(r#""viewers":2"#));
        assert!(outbox.lossy[1].body.contains(r#""creatorUniqueId":"b""#));
        assert!(outbox.lossy[2].body.contains(r#""state":"ready""#));
        assert!(outbox.lossy[3].body.contains(r#""pluginId":"p2""#));
    }

    #[test]
    fn lossy_lane_never_exceeds_bound() {
        let mut outbox = WebviewOutbox::new();
        for index in 0..(MAX_PENDING_WEBVIEW_MESSAGES + 50) {
            outbox.push(format!(r#"{{"type":"live-event","n":{index}}}"#));
        }
        assert_eq!(outbox.lossy.len(), MAX_PENDING_WEBVIEW_MESSAGES);
        // Oldest-first shedding: n:0..50 are gone, n:50 survives.
        assert!(outbox.lossy[0].body.contains(r#""n":50"#));
        assert!(outbox.reliable.is_empty());
    }

    #[test]
    fn rpc_response_survives_saturation() {
        let mut outbox = WebviewOutbox::new();
        for index in 0..MAX_PENDING_WEBVIEW_MESSAGES {
            outbox.push(domain_event(
                "live.ui-event",
                &format!(r#"{{"event":{{"n":{index}}}}}"#),
            ));
        }
        assert_eq!(outbox.lossy.len(), MAX_PENDING_WEBVIEW_MESSAGES);
        outbox.push(r#"{"type":"rpc-response","response":{"id":7,"result":{}}}"#.to_owned());
        // The response lands in the reliable lane, untouched by the full
        // lossy lane, and heads the very next batch.
        assert_eq!(outbox.reliable.len(), 1);
        assert!(outbox.reliable[0].contains(r#""id":7"#));
        let batch = outbox.take_batch();
        assert!(batch[0].contains(r#""id":7"#));
    }

    #[test]
    fn reliable_lane_never_drops_under_massive_burst() {
        let mut outbox = WebviewOutbox::new();
        // 1500 critical messages blow past the old 1024 hard cap that used
        // to shed them; every one must be retained and drain in order.
        for index in 0..1500 {
            outbox.push(format!(
                r#"{{"type":"rpc-response","response":{{"id":{index}}}}}"#
            ));
        }
        assert_eq!(outbox.reliable.len(), 1500);
        assert!(outbox.reliable[0].contains(r#""id":0"#));
        assert!(outbox.reliable[1499].contains(r#""id":1499"#));
        let mut drained = 0;
        while !outbox.is_empty() {
            let batch = outbox.take_batch();
            assert!(!batch.is_empty());
            assert!(batch.len() <= MAX_BATCH_PER_TICK);
            drained += batch.len();
        }
        assert_eq!(drained, 1500);
    }

    #[test]
    fn critical_transitions_survive_lossy_flood() {
        let mut outbox = WebviewOutbox::new();
        for index in 0..(MAX_PENDING_WEBVIEW_MESSAGES + 100) {
            outbox.push(domain_event(
                "live.ui-event",
                &format!(r#"{{"event":{{"n":{index}}}}}"#),
            ));
        }
        for topic in [
            "live.disconnected",
            "live.error",
            "plugin.stopped",
            "points.changed",
            "shutdown",
        ] {
            outbox.push(domain_event(topic, "{}"));
        }
        assert_eq!(outbox.reliable.len(), 5);
        assert_eq!(outbox.lossy.len(), MAX_PENDING_WEBVIEW_MESSAGES);
        let batch = outbox.take_batch();
        for topic in [
            "live.disconnected",
            "live.error",
            "plugin.stopped",
            "points.changed",
            "shutdown",
        ] {
            assert!(
                batch.iter().any(|body| body.contains(topic)),
                "lost {topic} under lossy flood"
            );
        }
    }

    #[test]
    fn live_feed_keeps_fifo_order() {
        let mut outbox = WebviewOutbox::new();
        for index in 0..50 {
            outbox.push(domain_event(
                "live.ui-event",
                &format!(r#"{{"event":{{"n":{index}}}}}"#),
            ));
        }
        for (position, queued) in outbox.lossy.iter().enumerate() {
            assert!(
                queued.body.contains(&format!(r#"{{"n":{position}}}"#)),
                "feed reordered at {position}: {}",
                queued.body
            );
        }
    }

    #[test]
    fn oversized_bursts_drain_in_capped_batches() {
        let mut outbox = WebviewOutbox::new();
        for index in 0..300 {
            outbox.push(domain_event(
                "live.ui-event",
                &format!(r#"{{"event":{{"n":{index}}}}}"#),
            ));
        }
        // Below the lossy cap nothing is shed; 300 messages need three
        // capped batches, each preserving order, until the outbox is empty.
        let first = outbox.take_batch();
        let second = outbox.take_batch();
        let third = outbox.take_batch();
        assert_eq!((first.len(), second.len(), third.len()), (128, 128, 44));
        assert!(first[0].contains(r#"{"n":0}"#));
        assert!(second[0].contains(r#"{"n":128}"#));
        assert!(third[0].contains(r#"{"n":256}"#));
        assert!(outbox.is_empty());
        assert!(outbox.take_batch().is_empty());
    }

    #[test]
    fn oversized_webview_requests_are_rejected_at_the_boundary() {
        let limit = tiktools_control_api::MAX_REQUEST_BYTES;
        assert!(!webview_request_too_large(&"x".repeat(limit)));
        assert!(webview_request_too_large(&"x".repeat(limit + 1)));
        assert!(is_probably_control_rpc(r#"{"method":"system.ping"}"#));
        assert!(!is_probably_control_rpc(r#"{"type":"disconnect"}"#));
    }

    fn forwarder_test_core(tag: &str) -> (Arc<AppCore>, std::path::PathBuf) {
        struct Emitter;
        impl tiktools_core::HostEmitter for Emitter {
            fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
        }
        let home = std::env::temp_dir().join(format!(
            "tiktools-webview-gap-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        std::env::set_var("TIKTOOLS_HOME", &home);
        (Arc::new(AppCore::new(Arc::new(Emitter))), home)
    }

    #[tokio::test]
    async fn lagged_burst_emits_gap_and_forwarder_survives() {
        let (core, home) = forwarder_test_core("reliable");
        let bus = tiktools_core::events::EventBus::new(1);
        let receiver = bus.subscribe_domain();
        // Lag the receiver deterministically: blast a >512-event burst
        // through the capacity-1 reliable lane before the forwarder reads.
        for _ in 0..600 {
            bus.publish_domain(tiktools_core::events::DomainEvent::LiveDisconnected);
        }
        let forwarded = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let task = {
            let forwarded = std::sync::Arc::clone(&forwarded);
            let core = Arc::clone(&core);
            tokio::spawn(forward_domain_events(receiver, core, move |notification| {
                forwarded
                    .lock()
                    .expect("forwarded lock poisoned")
                    .push(notification);
            }))
        };
        // Only terminate once the post-burst message is through, so the
        // Shutdown cannot collapse into the lagged burst itself.
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let delivered = forwarded.lock().expect("forwarded lock poisoned").len();
                if delivered >= 2 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("forwarder must deliver gap plus backlog after lag");
        bus.publish_domain(tiktools_core::events::DomainEvent::Shutdown);
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .expect("forwarder must terminate after Shutdown")
            .expect("forwarder panicked");
        let forwarded = forwarded.lock().expect("forwarded lock poisoned");
        // Reliable lag is explicit, never silent: a gap notification heads
        // the retained backlog, later events still arrive, and Shutdown
        // still terminates the forwarder.
        assert_eq!(
            forwarded.len(),
            3,
            "unexpected forwarded batch: {forwarded:?}"
        );
        let gap: serde_json::Value = serde_json::from_str(&forwarded[0]).expect("gap is JSON");
        assert_eq!(gap["method"], serde_json::json!("event.gap"));
        assert_eq!(gap["params"]["resync"], serde_json::json!(true));
        assert!(
            gap["params"]["lost"].as_u64().unwrap_or_default() > 0,
            "gap must count the lost events: {gap}"
        );
        assert!(forwarded[1].contains("live.disconnected"), "{forwarded:?}");
        assert!(forwarded[2].contains("shutdown"), "{forwarded:?}");
        assert_eq!(core.event_gap_stats().0, 1);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn lossy_burst_forwards_without_gap_signal() {
        let (core, home) = forwarder_test_core("lossy");
        let bus = tiktools_core::events::EventBus::new(1);
        let receiver = bus.subscribe_domain();
        for index in 0..600 {
            bus.publish_domain(tiktools_core::events::DomainEvent::LiveUiEvent {
                event: serde_json::json!({"n": index}),
            });
        }
        let forwarded = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let task = {
            let forwarded = std::sync::Arc::clone(&forwarded);
            let core = Arc::clone(&core);
            tokio::spawn(forward_domain_events(receiver, core, move |notification| {
                forwarded
                    .lock()
                    .expect("forwarded lock poisoned")
                    .push(notification);
            }))
        };
        // Wait for the retained lossy backlog, then terminate cleanly.
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let delivered = forwarded.lock().expect("forwarded lock poisoned").len();
                if delivered >= 1 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("forwarder must deliver the retained backlog");
        bus.publish_domain(tiktools_core::events::DomainEvent::Shutdown);
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .expect("forwarder must terminate after Shutdown")
            .expect("forwarder panicked");
        let forwarded = forwarded.lock().expect("forwarded lock poisoned");
        assert!(
            !forwarded.iter().any(|line| line.contains("event.gap")),
            "a lossy flood must never cause a gap signal: {forwarded:?}"
        );
        assert!(
            forwarded
                .last()
                .is_some_and(|line| line.contains("shutdown")),
            "shutdown still terminates after a lossy flood: {forwarded:?}"
        );
        assert_eq!(core.event_gap_stats().0, 0);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn reliable_lane_trips_safety_policy_by_count() {
        let mut outbox = WebviewOutbox::new();
        for index in 0..MAX_RELIABLE_MESSAGES {
            outbox.push(format!(
                r#"{{"type":"rpc-response","response":{{"id":{index}}}}}"#
            ));
        }
        assert!(!outbox.transport_failed());
        assert!(!outbox.take_transport_failure());
        // The 4097th message trips the policy: the failure edge fires
        // exactly once and every queued message is still retained.
        outbox.push(r#"{"type":"rpc-response","response":{"id":"trip"}}"#.to_owned());
        assert!(outbox.transport_failed());
        assert!(outbox.take_transport_failure());
        assert!(!outbox.take_transport_failure());
        assert_eq!(outbox.reliable.len(), MAX_RELIABLE_MESSAGES + 1);
        assert!(outbox.reliable_bytes() > 0);
        // While failed, new reliable messages are counted, never queued:
        // the queue cannot grow without bound for a dead page.
        outbox.push(r#"{"type":"rpc-response","response":{"id":"refused"}}"#.to_owned());
        assert_eq!(outbox.reliable.len(), MAX_RELIABLE_MESSAGES + 1);
        assert_eq!(outbox.dropped_while_failed, 1);
        // Recovery rearms the transport: new messages queue again.
        outbox.clear_for_reload();
        assert!(outbox.is_empty());
        assert_eq!(outbox.reliable_bytes(), 0);
        outbox.recover_transport();
        assert!(!outbox.transport_failed());
        outbox.push(r#"{"type":"rpc-response","response":{"id":"recovered"}}"#.to_owned());
        assert_eq!(outbox.reliable.len(), 1);
    }

    #[test]
    fn reliable_lane_trips_safety_policy_by_bytes() {
        let mut outbox = WebviewOutbox::new();
        // 17 one-megabyte critical messages exceed the 16 MiB policy with
        // far fewer than 4096 messages.
        let big = format!(
            r#"{{"type":"rpc-response","response":{{"blob":"{}"}}}}"#,
            "x".repeat(1024 * 1024)
        );
        for _ in 0..17 {
            outbox.push(big.clone());
        }
        assert!(outbox.transport_failed());
        assert!(outbox.reliable_bytes() > MAX_RELIABLE_BYTES);
        assert!(outbox.take_transport_failure());
    }

    #[test]
    fn take_batch_accounts_reliable_bytes() {
        let mut outbox = WebviewOutbox::new();
        outbox.push(r#"{"type":"rpc-response","response":{"id":1}}"#.to_owned());
        outbox.push(r#"{"type":"rpc-response","response":{"id":2}}"#.to_owned());
        let held = outbox.reliable_bytes();
        assert!(held > 0);
        let batch = outbox.take_batch();
        assert_eq!(batch.len(), 2);
        assert_eq!(outbox.reliable_bytes(), 0);
        assert!(outbox.is_empty());
    }

    #[test]
    fn inbound_routing_parses_once_and_correlates_errors() {
        // Valid shapes route by value, never by re-scanning text.
        assert!(matches!(
            classify_inbound_message(r#"{"jsonrpc":"2.0","id":1,"method":"system.ping"}"#),
            InboundRoute::Control(_)
        ));
        assert!(matches!(
            classify_inbound_message(r#"{"type":"disconnect"}"#),
            InboundRoute::Legacy(_)
        ));
        // The exact handshake and its whitespace variants both complete
        // startup without touching either router.
        assert_eq!(
            classify_inbound_message(r#"{"type":"frontend-ready"}"#),
            InboundRoute::FrontendReady
        );
        assert_eq!(
            classify_inbound_message("  { \"type\" : \"frontend-ready\" }  "),
            InboundRoute::FrontendReady
        );
        // Malformed control-shaped input answers with a correlated RPC
        // error instead of falling into the legacy router.
        assert!(matches!(
            classify_inbound_message(r#"{"id":7,"method":"system.ping","params":{broken"#),
            InboundRoute::MalformedControl(_)
        ));
        assert_eq!(
            tiktools_control_api::RpcId::extract_from_prefix(
                r#"{"id":7,"method":"system.ping","params":{broken"#
            ),
            tiktools_control_api::RpcId::Number(7)
        );
        // Malformed legacy-shaped input never enters the ControlApi.
        assert_eq!(
            classify_inbound_message(r#"{"type":"disconnect",oops"#),
            InboundRoute::MalformedLegacy
        );
        assert_eq!(
            classify_inbound_message("not json at all"),
            InboundRoute::MalformedLegacy
        );
    }

    #[test]
    fn frontend_ready_fast_path_rejects_large_json_without_parsing() {
        // Exact handshake (plus padding) matches on the Winit thread.
        assert!(is_frontend_ready_fast(r#"{"type":"frontend-ready"}"#));
        assert!(is_frontend_ready_fast("  {\"type\":\"frontend-ready\"}\n"));
        assert!(!is_frontend_ready_fast(r#"{"type":"disconnect"}"#));
        assert!(!is_frontend_ready_fast(r#"{"method":"system.ping"}"#));
        // The length gate alone rejects large payloads: a 1 MiB valid
        // JSON document never reaches a parser on the UI thread.
        let large = format!(
            r#"{{"method":"system.ping","params":{{"blob":"{}"}}}}"#,
            "y".repeat(1024 * 1024)
        );
        assert!(!is_frontend_ready_fast(&large));
        // Even a large payload smuggling the handshake string is rejected
        // here and classified off-thread instead.
        let smuggled = format!(
            r#"{{"note":"frontend-ready","blob":"{}"}}"#,
            "z".repeat(1024)
        );
        assert!(!is_frontend_ready_fast(&smuggled));
        assert!(matches!(
            classify_inbound_message(&smuggled),
            InboundRoute::Legacy(_)
        ));
    }
}
