use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::Arc,
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
    pending_host_messages: VecDeque<QueuedWebviewMessage>,
    shutting_down: bool,
    startup_state: StartupState,
    startup_deadline: Option<Instant>,
    pending_activation: bool,
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
        // Forward domain events to the WebView as JSON-RPC `event`
        // notifications so the frontend control client observes the same
        // bus as CLI/IPC streaming clients.
        {
            let events = control.subscribe();
            let proxy = proxy.clone();
            runtime.spawn(forward_domain_events(events, move |notification| {
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
            pending_host_messages: VecDeque::new(),
            shutting_down: false,
            startup_state: StartupState::Initializing,
            startup_deadline: None,
            pending_activation: false,
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
                // Same transport limits as local IPC: reject oversized
                // payloads before any JSON parsing so the WebView cannot
                // bypass them. (`MAX_PARAMS_BYTES` is enforced inside
                // `ControlApi::execute`, which all WebView RPCs traverse.)
                if webview_request_too_large(&raw) {
                    if is_probably_control_rpc(&raw) {
                        let response = tiktools_control_api::RpcResponse::error(
                            tiktools_control_api::RpcId::Null,
                            tiktools_control_api::ApiError::too_large(),
                        );
                        let payload = serde_json::json!({
                            "type": "rpc-response",
                            "response": response,
                        });
                        let _ = proxy_for_ipc.send_event(DesktopEvent::Command(
                            DesktopCommand::EmitToWebview(payload.to_string()),
                        ));
                    } else {
                        tracing::warn!(
                            bytes = raw.len(),
                            "oversized WebView IPC message dropped"
                        );
                    }
                    return;
                }
                if is_frontend_ready(&raw) {
                    let _ = proxy_for_ipc.send_event(DesktopEvent::Command(
                        DesktopCommand::FrontendReady,
                    ));
                    return;
                }
                let router = router.clone();
                let control = control.clone();
                let proxy = proxy_for_ipc.clone();
                runtime.spawn(async move {
                    // JSON-RPC messages (`{"method": ...}`) go through the
                    // same ControlApi as CLI/stdio/IPC; legacy `{"type": ...}`
                    // messages keep the PageMessage path during migration.
                    if is_control_rpc(&raw) {
                        let response = match serde_json::from_str::<serde_json::Value>(&raw) {
                            Ok(raw) => control.execute_value(&raw).await,
                            Err(error) => tiktools_control_api::RpcResponse::error(
                                tiktools_control_api::RpcId::Null,
                                tiktools_control_api::ApiError::invalid_params(format!(
                                    "invalid JSON: {error}"
                                )),
                            ),
                        };
                        let payload = serde_json::json!({
                            "type": "rpc-response",
                            "response": response,
                        });
                        let _ = proxy.send_event(DesktopEvent::Command(
                            DesktopCommand::EmitToWebview(payload.to_string()),
                        ));
                        return;
                    }
                    if let Err(error) = router.dispatch(&raw).await {
                        tracing::warn!(%error, "invalid WebView IPC message");
                    }
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

    /// Bounded enqueue with coalescing. Disposable snapshots (room stats,
    /// leaderboard, analytics, processor metrics, automation context) keep
    /// only their latest value so high-rate producers cannot flood the
    /// queue; the queue itself never exceeds its bound.
    fn emit_to_webview(&mut self, message: String) {
        push_webview_message(&mut self.pending_host_messages, message);
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
        let batch = take_next_batch(&mut self.pending_host_messages);
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
        self.core.spawn_plugin_event_poll(&self.runtime);
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

/// Soft bound for queued WebView messages. Past this point the queue sheds
/// droppable feed events, then stale snapshots — never critical messages.
/// All-critical bursts may grow past the soft bound toward the hard cap.
const MAX_PENDING_WEBVIEW_MESSAGES: usize = 512;
/// Absolute memory safety cap. Only a pathological all-critical flood can
/// reach it; shedding there prefers anything that is not an RPC response.
const MAX_PENDING_WEBVIEW_MESSAGES_HARD: usize = 1024;
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

fn is_rpc_response_value(value: &serde_json::Value) -> bool {
    value.get("type").and_then(serde_json::Value::as_str) == Some("rpc-response")
}

/// One queued message with its delivery class computed once at enqueue
/// time, so saturation scans never re-parse JSON on the UI thread.
#[derive(Debug)]
struct QueuedWebviewMessage {
    body: String,
    class: WebviewMessageClass,
    coalesce_key: Option<String>,
    is_rpc_response: bool,
}

fn push_webview_message(
    queue: &mut std::collections::VecDeque<QueuedWebviewMessage>,
    body: String,
) {
    let class = classify_webview_message(&body);
    let (coalesce_key, is_rpc_response) = match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(value) => (webview_coalesce_key(&value), is_rpc_response_value(&value)),
        Err(_) => (None, false),
    };
    if class == WebviewMessageClass::Coalescable {
        // Keep only the latest snapshot per coalesce key.
        if let Some(key) = coalesce_key.as_deref() {
            queue.retain(|queued| queued.coalesce_key.as_deref() != Some(key));
        }
    }
    queue.push_back(QueuedWebviewMessage {
        body,
        class,
        coalesce_key,
        is_rpc_response,
    });
    // Soft cap: shed droppable feed first, then stale snapshots. Critical
    // messages are never evicted here; an all-critical burst grows past
    // the soft cap instead of dropping RPC responses or transitions.
    while queue.len() > MAX_PENDING_WEBVIEW_MESSAGES {
        if let Some(index) = queue
            .iter()
            .position(|queued| queued.class == WebviewMessageClass::Droppable)
        {
            queue.remove(index);
        } else if let Some(index) = queue
            .iter()
            .position(|queued| queued.class == WebviewMessageClass::Coalescable)
        {
            queue.remove(index);
        } else {
            break;
        }
    }
    // Hard safety cap: absolute memory bound. Shed anything that is not
    // an RPC response first; only a queue of nothing but RPC responses
    // sacrifices the oldest one, loudly.
    while queue.len() > MAX_PENDING_WEBVIEW_MESSAGES_HARD {
        if let Some(index) = queue.iter().position(|queued| !queued.is_rpc_response) {
            queue.remove(index);
        } else {
            tracing::error!(
                "WebView queue exceeded hard cap with RPC responses only; dropping oldest response"
            );
            queue.pop_front();
        }
    }
}

/// Takes at most one UI tick's worth of messages, preserving order.
fn take_next_batch(queue: &mut std::collections::VecDeque<QueuedWebviewMessage>) -> Vec<String> {
    let take = queue.len().min(MAX_BATCH_PER_TICK);
    queue.drain(..take).map(|queued| queued.body).collect()
}

/// Forwards domain events to a UI sink as JSON-RPC `event` notifications.
/// A lagged receiver skips the missed burst and continues: only a closed
/// channel or the shutdown event terminates the forwarder, so a temporary
/// burst never permanently disables WebView events.
async fn forward_domain_events(
    mut events: tokio::sync::broadcast::Receiver<tiktools_core::events::DomainEvent>,
    mut send: impl FnMut(String),
) {
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                tracing::warn!(
                    skipped,
                    "WebView domain event receiver lagged; skipping burst"
                );
                continue;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        };
        let shutdown = matches!(event, tiktools_core::events::DomainEvent::Shutdown);
        let notification = tiktools_control_api::event_notification(&event);
        send(notification.to_string());
        if shutdown {
            break;
        }
    }
}

fn is_frontend_ready(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .as_deref()
        == Some("frontend-ready")
}

/// Raw inbound bound shared with local IPC. Checked before any parsing.
fn webview_request_too_large(raw: &str) -> bool {
    raw.len() > tiktools_control_api::MAX_REQUEST_BYTES
}

/// Allocation-free control-shape probe used only to route oversized-payload
/// errors (full parsing happens later, on size-capped input).
fn is_probably_control_rpc(raw: &str) -> bool {
    raw.contains("\"method\"")
}

/// Control-plane messages carry `method` (JSON-RPC style); legacy WebView
/// messages carry `type` (PageMessage). The two shapes never overlap.
fn is_control_rpc(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("method")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .is_some()
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
        let mut queue = std::collections::VecDeque::new();
        push_webview_message(
            &mut queue,
            r#"{"type":"room-stats","viewers":1,"totalUsers":1,"topViewers":[]}"#.to_owned(),
        );
        push_webview_message(
            &mut queue,
            r#"{"type":"room-stats","viewers":2,"totalUsers":2,"topViewers":[]}"#.to_owned(),
        );
        push_webview_message(
            &mut queue,
            r#"{"type":"live-event","event":{"kind":"chat"}}"#.to_owned(),
        );
        assert_eq!(queue.len(), 2);
        assert!(queue[0].body.contains("\"viewers\":2"));
    }

    #[test]
    fn domain_snapshots_coalesce_but_feed_and_lifecycle_do_not() {
        let mut queue = std::collections::VecDeque::new();
        push_webview_message(&mut queue, domain_event("room.stats", r#"{"viewers":1}"#));
        push_webview_message(&mut queue, domain_event("room.stats", r#"{"viewers":2}"#));
        push_webview_message(
            &mut queue,
            domain_event("analytics.updated", r#"{"creatorUniqueId":"a"}"#),
        );
        push_webview_message(
            &mut queue,
            domain_event("analytics.updated", r#"{"creatorUniqueId":"b"}"#),
        );
        // Same plugin collapses; a different plugin is a separate stream.
        push_webview_message(
            &mut queue,
            domain_event("plugin.progress", r#"{"pluginId":"p1","state":"loading"}"#),
        );
        push_webview_message(
            &mut queue,
            domain_event("plugin.progress", r#"{"pluginId":"p1","state":"ready"}"#),
        );
        push_webview_message(
            &mut queue,
            domain_event("plugin.progress", r#"{"pluginId":"p2","state":"ready"}"#),
        );
        // Feed and lifecycle are never coalesced.
        push_webview_message(
            &mut queue,
            domain_event("live.ui-event", r#"{"event":{"n":1}}"#),
        );
        push_webview_message(
            &mut queue,
            domain_event("live.ui-event", r#"{"event":{"n":2}}"#),
        );
        push_webview_message(
            &mut queue,
            domain_event("plugin.started", r#"{"pluginId":"p1"}"#),
        );
        push_webview_message(
            &mut queue,
            domain_event("plugin.started", r#"{"pluginId":"p1"}"#),
        );
        assert_eq!(queue.len(), 8, "unexpected queue len");
        assert!(queue[0].body.contains(r#""viewers":2"#));
        assert!(queue[1].body.contains(r#""creatorUniqueId":"b""#));
        assert!(queue[2].body.contains(r#""state":"ready""#));
        assert!(queue[3].body.contains(r#""pluginId":"p2""#));
    }

    #[test]
    fn queue_never_exceeds_bound() {
        let mut queue = std::collections::VecDeque::new();
        for index in 0..(MAX_PENDING_WEBVIEW_MESSAGES + 50) {
            push_webview_message(
                &mut queue,
                format!(r#"{{"type":"live-event","n":{index}}}"#),
            );
        }
        assert_eq!(queue.len(), MAX_PENDING_WEBVIEW_MESSAGES);
    }

    #[test]
    fn rpc_response_survives_saturation() {
        let mut queue = std::collections::VecDeque::new();
        for index in 0..MAX_PENDING_WEBVIEW_MESSAGES {
            push_webview_message(
                &mut queue,
                domain_event("live.ui-event", &format!(r#"{{"event":{{"n":{index}}}}}"#)),
            );
        }
        assert_eq!(queue.len(), MAX_PENDING_WEBVIEW_MESSAGES);
        push_webview_message(
            &mut queue,
            r#"{"type":"rpc-response","response":{"id":7,"result":{}}}"#.to_owned(),
        );
        // Still capped, the response kept, and exactly the oldest feed
        // event shed to make room for it.
        assert_eq!(queue.len(), MAX_PENDING_WEBVIEW_MESSAGES);
        assert!(
            queue.iter().any(|queued| queued.body.contains(r#""id":7"#)),
            "rpc-response was evicted under saturation"
        );
        assert!(
            !queue
                .iter()
                .any(|queued| queued.body.contains(r#"{"n":0}"#)),
            "saturation must shed oldest droppable first"
        );
    }

    #[test]
    fn all_critical_burst_grows_without_drops_below_hard_cap() {
        let mut queue = std::collections::VecDeque::new();
        for index in 0..600 {
            push_webview_message(
                &mut queue,
                format!(r#"{{"type":"rpc-response","response":{{"id":{index}}}}}"#),
            );
        }
        // Past the soft cap but nothing critical is evicted there.
        assert_eq!(queue.len(), 600);
        assert!(queue[0].body.contains(r#""id":0"#));
        for index in 600..(MAX_PENDING_WEBVIEW_MESSAGES_HARD + 100) {
            push_webview_message(
                &mut queue,
                format!(r#"{{"type":"rpc-response","response":{{"id":{index}}}}}"#),
            );
        }
        // The hard safety cap still bounds memory absolutely.
        assert_eq!(queue.len(), MAX_PENDING_WEBVIEW_MESSAGES_HARD);
    }

    #[test]
    fn live_feed_keeps_fifo_order() {
        let mut queue = std::collections::VecDeque::new();
        for index in 0..50 {
            push_webview_message(
                &mut queue,
                domain_event("live.ui-event", &format!(r#"{{"event":{{"n":{index}}}}}"#)),
            );
        }
        for (position, queued) in queue.iter().enumerate() {
            assert!(
                queued.body.contains(&format!(r#"{{"n":{position}}}"#)),
                "feed reordered at {position}: {}",
                queued.body
            );
        }
    }

    #[test]
    fn oversized_bursts_drain_in_capped_batches() {
        let mut queue = std::collections::VecDeque::new();
        for index in 0..300 {
            push_webview_message(
                &mut queue,
                domain_event("live.ui-event", &format!(r#"{{"event":{{"n":{index}}}}}"#)),
            );
        }
        // Below the soft cap nothing is shed; 300 messages need three
        // capped batches, each preserving order, until the queue is empty.
        let first = take_next_batch(&mut queue);
        let second = take_next_batch(&mut queue);
        let third = take_next_batch(&mut queue);
        assert_eq!((first.len(), second.len(), third.len()), (128, 128, 44));
        assert!(first[0].contains(r#"{"n":0}"#));
        assert!(second[0].contains(r#"{"n":128}"#));
        assert!(third[0].contains(r#"{"n":256}"#));
        assert!(queue.is_empty());
        assert!(take_next_batch(&mut queue).is_empty());
    }

    #[test]
    fn oversized_webview_requests_are_rejected_at_the_boundary() {
        let limit = tiktools_control_api::MAX_REQUEST_BYTES;
        assert!(!webview_request_too_large(&"x".repeat(limit)));
        assert!(webview_request_too_large(&"x".repeat(limit + 1)));
        assert!(is_probably_control_rpc(r#"{"method":"system.ping"}"#));
        assert!(!is_probably_control_rpc(r#"{"type":"disconnect"}"#));
    }

    #[tokio::test]
    async fn lagged_burst_does_not_stop_event_forwarder() {
        let (sender, receiver) = tokio::sync::broadcast::channel(1);
        // Lag the receiver deterministically: blast a >512-event burst
        // through the capacity-1 channel before the forwarder ever reads.
        for _ in 0..600 {
            sender
                .send(tiktools_core::events::DomainEvent::LiveDisconnected)
                .expect("send fits");
        }
        let forwarded = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let task = {
            let forwarded = std::sync::Arc::clone(&forwarded);
            tokio::spawn(forward_domain_events(receiver, move |notification| {
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
                if delivered >= 1 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("forwarder must deliver after Lagged");
        sender
            .send(tiktools_core::events::DomainEvent::Shutdown)
            .expect("send fits");
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .expect("forwarder must terminate after Shutdown")
            .expect("forwarder panicked");
        let forwarded = forwarded.lock().expect("forwarded lock poisoned");
        // The burst-skipping forwarder survives Lagged: it drops the missed
        // burst but still delivers what follows, ending with Shutdown. The
        // old break-on-any-error code would have forwarded nothing here.
        assert_eq!(
            forwarded.len(),
            2,
            "unexpected forwarded batch: {forwarded:?}"
        );
        assert!(forwarded[0].contains("live.disconnected"), "{forwarded:?}");
        assert!(forwarded[1].contains("shutdown"), "{forwarded:?}");
    }
}
