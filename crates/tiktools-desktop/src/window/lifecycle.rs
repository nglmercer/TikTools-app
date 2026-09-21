//! Window startup, shutdown, visibility, and event-loop integration.

use super::domain_events::forward_domain_events;
use super::ipc::{
    emit_rpc_response, handle_webview_ipc_message, is_frontend_ready_fast, is_probably_control_rpc,
    webview_request_too_large,
};
use super::outbox::WebviewOutbox;
use super::{DesktopApp, StartupState, FRONTEND_STARTUP_TIMEOUT};
use crate::event::{DesktopCommand, DesktopEvent};
use crate::platform;
use crate::tray::TrayController;
use crate::webview::FrontendSource;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tiktools_control_api::ControlApi;
use tiktools_core::ipc::IpcRouter;
use tiktools_core::AppCore;
use tokio::runtime::Handle;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::{Icon as WindowIcon, Window, WindowId};
use wry::dpi::{PhysicalPosition as WryPhysicalPosition, PhysicalSize as WryPhysicalSize};
use wry::PageLoadEvent;
use wry::Rect;
use wry::WebViewBuilder;

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
            plugin_ui: crate::plugin_webview::PluginUiWindows::default(),
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
        if self.plugin_ui.on_window_event(window_id, &event) {
            return;
        }
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
            DesktopEvent::Command(DesktopCommand::OpenPluginUi { plugin_id, page_id }) => {
                if self.shutting_down {
                    return;
                }
                if let Err(error) = self.plugin_ui.open(
                    event_loop,
                    &self.core,
                    &self.control,
                    &self.runtime,
                    &self.proxy,
                    &plugin_id,
                    &page_id,
                ) {
                    tracing::warn!(%error, plugin = %plugin_id, page = %page_id, "could not open plugin UI");
                }
            }
            DesktopEvent::Command(DesktopCommand::ClosePluginUi { plugin_id, page_id }) => {
                self.plugin_ui.close(&plugin_id, &page_id);
            }
            DesktopEvent::Command(DesktopCommand::PluginUiRespond {
                plugin_id,
                page_id,
                response,
            }) => {
                self.plugin_ui.respond(&plugin_id, &page_id, &response);
            }
            DesktopEvent::Command(DesktopCommand::PluginUiSubscribe {
                plugin_id,
                page_id,
                effect,
            }) => {
                self.plugin_ui
                    .apply_subscription(&plugin_id, &page_id, effect);
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
