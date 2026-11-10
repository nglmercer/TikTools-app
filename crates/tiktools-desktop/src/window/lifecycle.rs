//! Window startup, shutdown, visibility, and event-loop integration.

use super::domain_events::forward_domain_events;
use super::ipc::{
    emit_rpc_response, handle_webview_ipc_message, is_frontend_ready_fast, is_probably_control_rpc,
    parse_batch_ack, webview_request_too_large,
};
use super::outbox::WebviewOutbox;
use super::reload::{failure_reasons, RecoveryDecision};
use super::{DesktopApp, StartupState, FRONTEND_STARTUP_TIMEOUT};
use crate::event::CoalescedSignals;
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

const UPDATE_RELEASES_URL: &str = "https://github.com/nglmercer/TikTools-app/releases";

fn is_update_releases_url(url: &str) -> bool {
    url == UPDATE_RELEASES_URL
}

impl DesktopApp {
    pub fn new(
        core: Arc<AppCore>,
        router: Arc<IpcRouter>,
        frontend: FrontendSource,
        runtime: Handle,
        proxy: EventLoopProxy<DesktopEvent>,
        log_path: PathBuf,
        emitter: Arc<crate::event::EventLoopEmitter>,
    ) -> Self {
        let control = Arc::new(ControlApi::new(core.clone()));
        // Winit constructs this host outside an entered Tokio context.
        // Durable metadata delivery still belongs to the host lifecycle.
        core.start_coordination_worker_on(&runtime);
        #[cfg(feature = "cloud-auth")]
        core.start_cloud_sync_worker_on(&runtime);
        #[cfg(feature = "remote-control")]
        {
            core.set_collaboration_gateway_available(true);
            runtime.spawn(tiktools_remote::supervise(Arc::clone(&control)));
        }
        let domain_mailbox = emitter.mailbox.clone();
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
                            let _ = proxy.send_coalesced(DesktopEvent::Command(
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
            let events = core.events.subscribe_projection();
            let core = core.clone();
            let emitter = emitter.clone();
            let projection = emitter.projection.clone();
            runtime.spawn(forward_domain_events(
                events,
                core,
                projection,
                move |notification, generation| {
                    emitter.send_value_generation(notification, generation);
                },
            ));
        }
        Self {
            window: None,
            webview: None,
            #[cfg(all(test, feature = "live-benchmark"))]
            benchmark_context: std::env::var_os("TIKTOOLS_BENCH_CASE")
                .and_then(|_| std::env::var_os("TIKTOOLS_HOME"))
                .map(|home| wry::WebContext::new(Some(PathBuf::from(home).join("webview-data")))),
            core,
            router,
            control,
            frontend,
            runtime,
            proxy,
            tray: None,
            pending_host_messages: WebviewOutbox::new(),
            pending_batch: None,
            page_ready: false,
            reload: super::reload::MainReloadSupervisor::new(),
            domain_mailbox,
            emitter,
            shutting_down: false,
            startup_state: StartupState::Initializing,
            startup_deadline: None,
            pending_activation: false,
            webview_failed: Arc::new(AtomicBool::new(false)),
            reload_pending: false,
            log_path,
            ui_generation: 0,
            diagnostics_at: Instant::now(),
            diagnostic_logging: std::env::var_os("TIKTOOLS_DIAGNOSTICS").is_some(),
            plugin_ui: crate::plugin_webview::PluginUiWindows::default(),
            pending_update: None,
            update_ready_for_handoff: false,
        }
    }

    pub(crate) fn take_update_plan(
        &mut self,
    ) -> Result<Option<(tiktools_core::app_update::InstallPlan, bool)>, bool> {
        let plan = self.pending_update.take();
        match plan {
            Some((_, spanish)) if !self.update_ready_for_handoff => Err(spanish),
            _ => Ok(plan),
        }
    }

    /// Builds the main frontend WebView as a child of `window` on the UI
    /// thread. The frontend URL, navigation policy, IPC bridge and asset
    /// protocols are identical for the first load and every recreate, so a
    /// recovered WebView never escapes the allowed origin and never touches
    /// `AppCore` or user data (only the WebView handle is rebuilt).
    fn build_main_webview(&mut self, window: &Window) -> Result<wry::WebView, String> {
        let router = self.router.clone();
        let control = self.control.clone();
        let runtime = self.runtime.clone();
        let proxy_for_ipc = self.proxy.clone();
        let transport_failed = self.webview_failed.clone();
        let ipc_emitter = self.emitter.clone();
        let admission = self.emitter.ipc_slots.clone();
        let page_emitter = self.emitter.clone();
        let navigation_frontend = self.frontend.clone();
        #[cfg(all(test, feature = "live-benchmark"))]
        let base_builder = match self.benchmark_context.as_mut() {
            // Isolate benchmark cookies/localStorage/cache from the user's app.
            Some(context) => WebViewBuilder::new_with_web_context(context),
            None => WebViewBuilder::new(),
        };
        #[cfg(not(all(test, feature = "live-benchmark")))]
        let base_builder = WebViewBuilder::new();
        let mut builder = base_builder
            .with_devtools(cfg!(debug_assertions) || cfg!(feature = "devtools"))
            .with_focused(false)
            .with_autoplay(true)
            // Page-focused shortcuts (F5/Ctrl+R reload, F12/Ctrl+Shift+I
            // inspector): on Linux the child WebView holds keyboard focus,
            // so the Winit window never sees these keys. The script reloads
            // in-page and asks the host for the inspector.
            .with_initialization_script(crate::webview::MAIN_INIT_SCRIPT)
            .with_initialization_script(if cfg!(feature = "remote-client") { "window.__tiktools_remote_client_available__ = true;" } else { "window.__tiktools_remote_client_available__ = false;" })
            .with_navigation_handler(move |url| {
                if is_update_releases_url(&url) {
                    if let Err(error) = open::that_detached(UPDATE_RELEASES_URL) {
                        tracing::warn!(%error, "could not open the application release page");
                    }
                    return false;
                }
                // Plugin asset URLs load inline frames in the plugin tabs;
                // installation is enforced per request by the asset server.
                let allowed = navigation_frontend.allows_navigation(&url)
                    || crate::plugin_webview::assets::allows_plugin_navigation(&url);
                if !allowed {
                    tracing::warn!(url = %url, "blocked WebView navigation outside the application frontend");
                }
                allowed
            })
            .with_new_window_req_handler(move |url, _features| {
                tracing::debug!(url = %url, "blocked WebView new-window request");
                wry::NewWindowResponse::Deny
            })
            // Anchor downloads (the rule-profile export, plugin asset
            // dumps) surface through WebKit's download signal, which
            // wry only wires up when a handler is registered. Without
            // this, `download`-attribute clicks in the WebView fail
            // silently: the blob URL is never written anywhere.
            .with_download_started_handler(move |url, destination| {
                tracing::debug!(
                    url = %url,
                    destination = %destination.display(),
                    "WebView download started"
                );
                true
            })
            .with_on_page_load_handler(move |event, url| {
                match event {
                    PageLoadEvent::Started => {
                        page_emitter.advance_generation();
                        tracing::info!(url = %url, generation = page_emitter.generation.load(Ordering::Acquire), "WebView navigation Started");
                    }
                    PageLoadEvent::Finished => {
                        tracing::info!(url = %url, "WebView navigation Finished; waiting for frontend-ready");
                    }
                }
            })
            .with_ipc_handler(move |request| {
                let raw = request.body();
                if let Some((generation, sequence)) = parse_batch_ack(raw) {
                    ipc_emitter.request_batch_ack(generation, sequence);
                    return;
                }
                // The Winit callback never parses JSON: size-check, match
                // the tiny boot handshake, then move the payload to Tokio,
                // where it is parsed exactly once and classified on the
                // value. This keeps malicious or very large valid JSON off
                // the UI loop.
                // Same transport limits as local IPC: reject oversized
                // payloads before any JSON parsing so the WebView cannot
                // bypass them. (`MAX_PARAMS_BYTES` is enforced inside
                // `ControlApi::execute`, which all WebView RPCs traverse.)
                if webview_request_too_large(raw) {
                    if is_probably_control_rpc(raw) {
                        // Preserve the request id so the frontend resolves
                        // (rejects) the pending call instead of hanging it
                        // until timeout; the id sits in the bounded prefix.
                        let response = tiktools_control_api::RpcResponse::error(
                            tiktools_control_api::RpcId::extract_from_prefix(raw),
                            tiktools_control_api::ApiError::too_large(),
                        );
                        emit_rpc_response(&ipc_emitter, &response);
                    } else {
                        tracing::warn!(
                            bytes = raw.len(),
                            "oversized WebView IPC message dropped"
                        );
                    }
                    return;
                }
                if is_frontend_ready_fast(raw) {
                    ipc_emitter.request_ready(ipc_emitter.generation.load(Ordering::Acquire));
                    return;
                }
                let permit = match admission.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => {
                        ipc_emitter.inbound_rejected.fetch_add(1, Ordering::Relaxed);
                        if is_probably_control_rpc(raw) {
                            let response = tiktools_control_api::RpcResponse::error(
                                tiktools_control_api::RpcId::extract_from_prefix(raw),
                                tiktools_control_api::ApiError::new("overloaded", "Main WebView IPC capacity exhausted"),
                            );
                            emit_rpc_response(&ipc_emitter, &response);
                        }
                        return;
                    }
                };
                let raw = raw.clone();
                let generation = ipc_emitter.generation.load(Ordering::Acquire);
                let emitter = ipc_emitter.clone();
                let router = router.clone();
                let control = control.clone();
                let proxy = proxy_for_ipc.clone();
                let transport_failed = transport_failed.clone();
                runtime.spawn(async move {
                    let permit = Arc::new(permit);
                    handle_webview_ipc_message(
                        raw,
                        &router,
                        &control,
                        &proxy,
                        &transport_failed,
                        &emitter,
                        (generation, permit),
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
        // Plugin UI assets for the inline tab frames, in dev and packaged
        // builds alike (the main document URL is irrelevant: the scheme is
        // registered on the WebView itself).
        let plugin_assets =
            crate::plugin_webview::assets::SharedPluginAssetServer::new(self.core.clone());
        builder = builder.with_custom_protocol(
            crate::plugin_webview::assets::PLUGIN_ASSET_SCHEME.to_owned(),
            move |_id, request| plugin_assets.respond(request),
        );
        builder = builder.with_url(self.frontend.url().as_str());
        tracing::debug!(
            url = %self.frontend.url(),
            "creating TikTools frontend WebView"
        );
        platform::build_webview(builder, window)
            .map_err(|error| format!("could not create Wry WebView: {error}"))
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

        let webview = self.build_main_webview(&window)?;
        tracing::info!("Wry WebView created successfully");

        self.window = Some(window);
        self.webview = Some(webview);
        self.startup_state = StartupState::WebViewLoading;
        let now = Instant::now();
        self.startup_deadline = Some(now + FRONTEND_STARTUP_TIMEOUT);
        // The first navigation shares the reload watchdog: a missing
        // `frontend-ready` on the very first document times out the same
        // way as any later reload (plus the startup dialog path below).
        self.reload.on_navigation_started(now);
        tracing::info!("WebView lifecycle Loading: waiting for frontend-ready");
        if self.tray.is_none() {
            match TrayController::create(self.proxy.clone()) {
                Ok(tray) => self.tray = Some(tray),
                Err(error) => {
                    tracing::warn!(%error, "system tray is unavailable; window remains usable")
                }
            }
        }
        self.flush_host_messages();
        tracing::debug!("main window created, waiting for frontend-ready");
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

    /// User-initiated reload on the UI thread: halt delivery to the current
    /// document immediately, then reload. The resulting Started event
    /// advances the generation and restarts the bounded watchdog, so a
    /// manual refresh flows through the same lifecycle as a programmatic
    /// one. Never touches `AppCore` or user data.
    fn reload_webview(&mut self) {
        if self.shutting_down {
            return;
        }
        let Some(webview) = self.webview.as_ref() else {
            return;
        };
        self.page_ready = false;
        self.pending_batch = None;
        self.emitter.clear_batch();
        if let Err(error) = webview.reload() {
            tracing::debug!(%error, "manual WebView reload failed");
        }
    }

    fn on_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        use winit::event::ElementState;
        use winit::keyboard::{KeyCode, PhysicalKey};
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }
        // Parent-focused shortcuts. When the page itself holds keyboard
        // focus (the common Linux case: the child WebView owns focus), the
        // Winit window never sees the key and the init script handles it
        // instead (F5/Ctrl+R in-page reload, F12/Ctrl+Shift+I inspector
        // request). F12 is the conventional inspector shortcut. WebView2
        // also handles it natively when devtools are enabled; this covers
        // the other backends and guarantees the shortcut exists.
        match event.physical_key {
            PhysicalKey::Code(KeyCode::F12) => self.open_devtools(),
            PhysicalKey::Code(KeyCode::F5) => self.reload_webview(),
            _ => {}
        }
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
        self.emitter.projection.set_host_visible(visible);
        self.emitter.send_value_generation(
            &serde_json::json!({"jsonrpc": "2.0", "method": "event", "params": {
                "topic": "ui.host.visibility", "data": {"visible": visible},
            }}),
            self.emitter.generation.load(Ordering::Acquire),
        );
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
        tracing::debug!("shutdown started");
        self.shutting_down = true;
        self.emitter.ipc_slots.close();
        self.emitter.generation.fetch_add(1, Ordering::AcqRel);
        self.domain_mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .shutdown();
        self.startup_state = StartupState::ShuttingDown;
        self.startup_deadline = None;
        let core = Arc::clone(&self.core);
        let proxy = self.proxy.clone();
        self.runtime.spawn(async move {
            core.shutdown().await;
            let _ = proxy.send_coalesced(DesktopEvent::Command(DesktopCommand::ShutdownComplete));
        });
    }

    fn finalize_shutdown(&mut self, event_loop: &ActiveEventLoop) {
        self.startup_state = StartupState::ShuttingDown;
        self.tray.take();
        if let Some(webview) = self.webview.take() {
            platform::prepare_webview_teardown(&webview);
            drop(webview);
        }
        // Container destroy ahead of the parent destroy (see
        // `platform::sync_gtk_display`); the parent handle drops normally.
        platform::sync_gtk_display();
        self.window.take();
        self.update_ready_for_handoff =
            self.pending_update.is_some() && self.core.update_shutdown_safe();
        tracing::debug!("shutdown complete, exiting");
        event_loop.exit();
    }

    fn sync_page_generation(&mut self, generation: u64) {
        if generation == self.ui_generation {
            return;
        }
        let replacing_page = self.ui_generation != 0;
        self.ui_generation = generation;
        // Navigation Started: advance the generation (done by the caller),
        // invalidate pending batches, and stop delivery to the previous
        // document before a single new message can cross documents. The
        // stale backlog is dropped without replay: recovery never retries
        // non-idempotent RPCs, the fresh page resynchronizes through its
        // mount reads instead.
        self.page_ready = false;
        self.pending_batch = None;
        self.emitter.clear_batch();
        // (Re)start the bounded reload watchdog for this document.
        let now = Instant::now();
        self.reload.on_navigation_started(now);
        tracing::info!(
            generation,
            replacing_page,
            lifecycle = ?self.reload.state(),
            "WebView navigation Started: batches invalidated, delivery halted, watchdog started"
        );
        if !self.pending_host_messages.is_empty() {
            // Initial bootstrap snapshots precede the first page's full
            // authoritative fetch; only replacing an existing page creates
            // a discontinuity for a client that was already consuming.
            if replacing_page {
                self.core.record_event_gap();
            }
            self.pending_host_messages.clear_for_reload();
        }
        // A navigation during startup restarts the startup budget as well:
        // the fresh document gets the full handshake window instead of
        // inheriting the previous document's deadline.
        if self.startup_state == StartupState::WebViewLoading {
            self.startup_deadline = Some(now + FRONTEND_STARTUP_TIMEOUT);
        }
    }

    fn frontend_ready(&mut self) {
        // Frontend-ready: validate the current generation before marking
        // Ready. A stale handshake from the previous document can never
        // restore delivery or clear a newer document's watchdog.
        let generation = self.emitter.generation.load(Ordering::Acquire);
        if !self
            .reload
            .on_frontend_ready(self.ui_generation, generation)
        {
            tracing::warn!(
                ui_generation = self.ui_generation,
                emitter_generation = generation,
                "stale frontend-ready ignored: generation mismatch"
            );
            return;
        }
        tracing::info!(
            generation = self.ui_generation,
            lifecycle = ?self.reload.state(),
            "WebView frontend-ready: page Ready, IPC delivery restored"
        );
        // A reload/recreate handshake recovers a failed transport only
        // once the fresh page is actually running: stale responses can
        // never mis-resolve the new page's calls, and the mount reads
        // resync authoritative state. Recovery runs before the startup
        // gate below so a mid-startup reload still completes startup.
        // Authoritative resync: the rebooted frontend refetches through
        // its mount reads; the gap marker makes any discontinuity
        // explicit instead of replaying the dropped backlog.
        if self.reload_pending && !self.shutting_down {
            self.reload_pending = false;
            self.pending_host_messages.recover_transport();
            self.domain_mailbox
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .recover();
            self.webview_failed.store(false, Ordering::SeqCst);
            self.core.set_webview_error(None);
            tracing::info!(
                "WebView transport recovered after reload; authoritative resync via mount reads"
            );
        }
        if !self.shutting_down {
            self.page_ready = true;
        }
        if self.shutting_down || self.startup_state == StartupState::Ready {
            return;
        }
        if self.startup_state != StartupState::WebViewLoading {
            tracing::debug!(state = ?self.startup_state, "ignoring frontend-ready outside WebView startup");
            return;
        }
        tracing::debug!("frontend-ready IPC received, main window shown");
        self.startup_state = StartupState::Ready;
        self.startup_deadline = None;
        // Plugin polling is owned by host startup (see `DesktopApp::new`),
        // never by WebView readiness: no lifecycle call here.
        self.set_window_visible(true);
        if let Some(window) = self.window.as_ref() {
            window.focus_window();
        }
    }

    /// Recreates the main WebView on the UI thread after a failed
    /// navigation recovery. Only the WebView handle is rebuilt with the
    /// same URL, navigation policy and bridges: `AppCore`, user data and
    /// the parent window are preserved.
    fn recreate_main_webview(&mut self) -> Result<(), String> {
        let window = self
            .window
            .take()
            .ok_or_else(|| "no window to recreate the WebView in".to_owned())?;
        if let Some(old) = self.webview.take() {
            platform::prepare_webview_teardown(&old);
            drop(old);
        }
        platform::sync_gtk_display();
        let result = self.build_main_webview(&window);
        self.window = Some(window);
        match result {
            Ok(webview) => {
                self.webview = Some(webview);
                self.page_ready = false;
                self.pending_batch = None;
                self.emitter.clear_batch();
                self.pending_host_messages.clear_for_reload();
                self.reload_pending = true;
                tracing::info!("WebView recreated on the UI thread; waiting for frontend-ready");
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Polls the bounded reload watchdog on the UI thread. Runs only after
    /// startup completed: the first load keeps the startup dialog path.
    /// Recovery is bounded (one navigation reload, one WebView recreate,
    /// then `Failed`): no infinite reload loops, no unbounded memory, no
    /// replay of non-idempotent RPCs, no `AppCore` restart.
    fn poll_reload_watchdog(&mut self) {
        if self.shutting_down || self.startup_state != StartupState::Ready {
            return;
        }
        // Batch-ack timeouts already fail the transport explicitly below;
        // the watchdog here covers the missing `frontend-ready` case.
        let now = Instant::now();
        match self.reload.check_timeout(now) {
            RecoveryDecision::Wait => {}
            RecoveryDecision::ReloadOnce => {
                tracing::warn!(
                    reason = failure_reasons::RELOAD_TIMEOUT,
                    "WebView reload timeout: attempting one controlled navigation recovery"
                );
                self.core.set_webview_error(Some(format!(
                    "WebView reload timed out ({}); attempting navigation recovery",
                    failure_reasons::RELOAD_TIMEOUT
                )));
                self.page_ready = false;
                self.pending_batch = None;
                self.emitter.clear_batch();
                self.pending_host_messages.clear_for_reload();
                match self.webview.as_ref() {
                    Some(webview) => match webview.reload() {
                        Ok(()) => {
                            self.reload_pending = true;
                            tracing::info!("WebView navigation recovery reload issued");
                        }
                        Err(error) => {
                            tracing::error!(%error, "controlled navigation recovery reload failed");
                        }
                    },
                    None => {
                        tracing::error!("WebView reload timeout with no WebView to reload");
                    }
                }
            }
            RecoveryDecision::Recreate => {
                tracing::warn!(
                    reason = failure_reasons::RELOAD_TIMEOUT,
                    "WebView reload timeout persists: recreating the WebView on the UI thread"
                );
                self.core.set_webview_error(Some(format!(
                    "WebView reload timed out ({}); recreating the WebView",
                    failure_reasons::RELOAD_TIMEOUT
                )));
                if let Err(error) = self.recreate_main_webview() {
                    tracing::error!(%error, "WebView recreate failed; transport stays failed");
                    self.reload.record_failure(failure_reasons::RELOAD_TIMEOUT);
                }
            }
            RecoveryDecision::GiveUp => {
                tracing::error!(
                    reason = ?self.reload.failure_reason(),
                    lifecycle = ?self.reload.state(),
                    "WebView reload recovery exhausted; parking in Failed without further retries"
                );
                self.page_ready = false;
                self.pending_batch = None;
                self.emitter.clear_batch();
                self.core.set_webview_error(Some(format!(
                    "WebView reload failed ({:?}); reload the window to retry",
                    self.reload.failure_reason().unwrap_or("unknown")
                )));
                self.webview_failed.store(true, Ordering::SeqCst);
            }
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
            .plugin_ui
            .on_window_event(&self.proxy, window_id, &event)
        {
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
            WindowEvent::Focused(gained) => {
                tracing::debug!(gained, "desktop window focus changed");
                if platform::window_focus_action(gained)
                    == platform::WindowFocusAction::FocusWebview
                {
                    if let Some(webview) = self.webview.as_ref() {
                        if let Err(error) = webview.focus() {
                            tracing::debug!(%error, "could not focus WebView");
                        }
                    }
                }
            }
            WindowEvent::Destroyed => {
                tracing::debug!(tray = self.tray.is_some(), "window was destroyed");
                if let Some(webview) = self.webview.take() {
                    platform::prepare_webview_teardown(&webview);
                    drop(webview);
                }
                platform::sync_gtk_display();
                // The server already destroyed this window: dropping the
                // handle would queue a second destroy for a dead id (stale
                // BadWindow filed where Winit's IME path panics on it).
                // Leak the handle; the tray path below recreates on demand.
                self.emitter.advance_generation();
                if let Some(window) = self.window.take() {
                    platform::record_external_destroy();
                    std::mem::forget(window);
                }
                if self.tray.is_none() {
                    self.shutdown(event_loop);
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: DesktopEvent) {
        match crate::event::consume_signal(event) {
            DesktopEvent::Command(DesktopCommand::AdmittedCommand {
                command,
                admission: _admission,
            }) => {
                self.user_event(event_loop, DesktopEvent::Command(*command));
            }
            // Wake-only chaining for multi-batch bursts; the flush runs in
            // `about_to_wait`, so this arm intentionally does nothing.
            DesktopEvent::Command(DesktopCommand::FlushWebviewBatch) => {}
            DesktopEvent::Command(DesktopCommand::HostBatchAcknowledged) => {
                if self.emitter.consume_batch_ack()
                    && self.pending_batch.as_ref().is_some_and(|batch| {
                        batch.generation == self.emitter.generation.load(Ordering::Acquire)
                            && batch.sequence == self.emitter.batch_sequence.load(Ordering::Acquire)
                    })
                {
                    if let Some(batch) = self.pending_batch.as_ref() {
                        let latency = Instant::now()
                            .saturating_duration_since(
                                batch.deadline - std::time::Duration::from_secs(5),
                            )
                            .as_micros() as u64;
                        self.emitter
                            .ack_latency_us
                            .store(latency, Ordering::Relaxed);
                        self.emitter
                            .ack_latency_max_us
                            .fetch_max(latency, Ordering::Relaxed);
                        self.emitter
                            .ack_latency_total_us
                            .fetch_add(latency, Ordering::Relaxed);
                    }
                    self.pending_batch = None;
                    self.emitter.clear_batch();
                    self.emitter
                        .acknowledged_batches
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
            #[cfg(all(test, feature = "live-benchmark"))]
            DesktopEvent::Command(DesktopCommand::BenchmarkSample { .. }) => {}
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
            DesktopEvent::Command(DesktopCommand::FrontendReady { generation: _ }) => {
                let generation = self.emitter.consume_ready();
                if generation == self.emitter.generation.load(Ordering::Acquire) {
                    self.sync_page_generation(generation);
                    self.frontend_ready();
                }
            }
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
            DesktopEvent::Command(DesktopCommand::OpenPluginDevtools { window_id }) => {
                self.plugin_ui.open_devtools_for_window(window_id)
            }
            DesktopEvent::Command(DesktopCommand::Quit) => self.shutdown(event_loop),
            DesktopEvent::Command(DesktopCommand::RequestAppUpdateInstall {
                generation,
                spanish,
            }) => {
                struct ResetPrompt(Arc<crate::event::EventLoopEmitter>);
                impl Drop for ResetPrompt {
                    fn drop(&mut self) {
                        self.0.update_prompt_pending.store(false, Ordering::Release);
                    }
                }
                let _reset = ResetPrompt(self.emitter.clone());
                if self.shutting_down
                    || generation != self.emitter.generation.load(Ordering::Acquire)
                {
                    return;
                }
                let Some(updater) = self.core.app_updater() else {
                    return;
                };
                let status = updater.status();
                if !status.install_supported
                    || status.state != tiktools_core::app_update::Phase::ReadyToInstall
                {
                    return;
                }
                let Some(version) = status.available_version.as_deref() else {
                    return;
                };
                let Some(location) = updater.installation_location() else {
                    return;
                };
                let location = location
                    .to_string_lossy()
                    .chars()
                    .map(|c| if c.is_control() { '�' } else { c })
                    .collect::<String>();
                let (title, description) = if spanish {
                    ("Instalar actualización de TikTools", format!("¿Instalar TikTools {version} y reiniciar? Las sesiones LIVE y automatizaciones activas se detendrán.\n\nUbicación: {location}"))
                } else {
                    ("Install TikTools update", format!("Install TikTools {version} and restart? Active LIVE sessions and automations will stop.\n\nLocation: {location}"))
                };
                let dialog = rfd::MessageDialog::new()
                    .set_title(title)
                    .set_description(description)
                    .set_level(rfd::MessageLevel::Warning)
                    .set_buttons(rfd::MessageButtons::YesNo);
                let dialog = if let Some(window) = self.window.as_ref() {
                    dialog.set_parent(window)
                } else {
                    dialog
                };
                if dialog.show() != rfd::MessageDialogResult::Yes {
                    return;
                }
                match updater.prepare_install() {
                    Ok(plan) => {
                        self.pending_update = Some((plan, spanish));
                        // Allow the Installing event to reach the WebView before
                        // closing its outbound mailbox and beginning shutdown.
                        let proxy = self.proxy.clone();
                        self.runtime.spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                            let _ = proxy.send_event(DesktopEvent::Command(
                                DesktopCommand::BeginAppUpdateShutdown,
                            ));
                        });
                    }
                    Err(error) => {
                        tracing::warn!(%error, "could not prepare application update");
                        let message = if spanish {
                            "No se pudo preparar la actualización. Comprueba el estado e inténtalo de nuevo."
                        } else {
                            "The update could not be prepared. Check its status and try again."
                        };
                        let _ = rfd::MessageDialog::new()
                            .set_title(if spanish {
                                "Actualización de TikTools"
                            } else {
                                "TikTools update"
                            })
                            .set_description(message)
                            .set_level(rfd::MessageLevel::Error)
                            .set_buttons(rfd::MessageButtons::Ok)
                            .show();
                    }
                }
            }
            DesktopEvent::Command(DesktopCommand::BeginAppUpdateShutdown) => {
                if self.pending_update.is_some() {
                    self.shutdown(event_loop);
                }
            }
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
            DesktopEvent::Command(DesktopCommand::ClosePluginUi {
                plugin_id,
                page_id,
                window_id,
                owner,
            }) => {
                self.plugin_ui
                    .close_if_current(&plugin_id, &page_id, window_id, owner);
            }
            DesktopEvent::Command(DesktopCommand::PluginUiRespond {
                window_id,
                plugin_id,
                page_id,
                response,
                effect,
                generation,
                admission: _admission,
            }) => {
                self.plugin_ui.respond(
                    &plugin_id, &page_id, response, window_id, generation, effect,
                );
            }
            DesktopEvent::Command(DesktopCommand::PluginUiSubscribe {
                window_id,
                plugin_id,
                page_id,
                effect,
                generation,
                admission: _admission,
            }) => {
                self.plugin_ui
                    .apply_subscription(&plugin_id, &page_id, effect, window_id, generation);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        platform::pump();
        let generation = self.emitter.generation.load(Ordering::Acquire);
        self.sync_page_generation(generation);
        // Bounded reload watchdog: missing `frontend-ready` records a
        // reason and attempts one controlled reload, then one recreate,
        // then parks in Failed. Never restarts AppCore, never replays RPCs.
        self.poll_reload_watchdog();
        // Plugin pop-outs share the same bound: timed-out bridge checks
        // reload once, then close/recreate, then park without looping.
        self.plugin_ui.poll_recovery();
        // Automatic recreates run on the UI thread through the same `open`
        // path (same origin policy, same broker, same AppCore): at most one
        // per failure sequence, so a permanently broken page parks instead
        // of reopening forever.
        for (plugin_id, page_id) in self.plugin_ui.drain_pending_recreates() {
            if self.shutting_down {
                continue;
            }
            tracing::info!(plugin = %plugin_id, page = %page_id, "recreating plugin WebView on the UI thread");
            if let Err(error) = self.plugin_ui.open(
                event_loop,
                &self.core,
                &self.control,
                &self.runtime,
                &self.proxy,
                &plugin_id,
                &page_id,
            ) {
                tracing::error!(%error, plugin = %plugin_id, page = %page_id, "plugin WebView recreate failed");
            }
        }
        if self
            .pending_batch
            .as_ref()
            .is_some_and(|batch| Instant::now() >= batch.deadline)
        {
            self.core.record_event_gap();
            self.fail_webview_transport(failure_reasons::BATCH_TIMEOUT);
        }
        if self.diagnostics_at.elapsed() >= std::time::Duration::from_secs(10) {
            self.diagnostics_at = Instant::now();
            *self
                .emitter
                .ui_snapshot
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = serde_json::json!({
                "outbox": self.pending_host_messages.diagnostics(),
                "retainedBatchCount": self.pending_batch.as_ref().map(|batch| batch.messages.len()).unwrap_or(0),
                "plugins": self.plugin_ui.diagnostics(),
                "nativeLifecycle": platform::native_lifecycle_diagnostics(),
                "webviewLifecycle": {
                    "state": format!("{:?}", self.reload.state()),
                    "failureReason": self.reload.failure_reason(),
                    "navigationRecoveries": self.reload.recovery_counts().0,
                    "webviewRecreates": self.reload.recovery_counts().1,
                    "pageReady": self.page_ready,
                },
            });
            if self.diagnostic_logging {
                use tiktools_core::HostEmitter;
                tracing::info!(desktop = %self.emitter.diagnostics(), "aggregate desktop diagnostics");
            }
        }
        let (messages, failed, wake, dropped) = {
            let mut mailbox = self
                .domain_mailbox
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let (messages, failed, wake) = mailbox.drain();
            (messages, failed, wake, mailbox.take_lossy_drops())
        };
        if dropped > 0 && !failed {
            self.pending_host_messages
                .push_value(&tiktools_control_api::gap_notification(dropped));
        }
        if failed {
            self.core.record_event_gap();
            self.fail_webview_transport("producer mailbox admission exhausted");
        }
        for message in messages {
            self.emit_to_webview(message);
        }
        if wake {
            let _ = self
                .proxy
                .send_coalesced(DesktopEvent::Command(DesktopCommand::FlushWebviewBatch));
        }
        // One batch per UI tick: all host messages queued since the last
        // tick share a single `evaluate_script`.
        self.flush_webview_batch();
        let plugin_pending = self.plugin_ui.flush_batches();
        // A batch is capped, so a burst longer than one batch must chain
        // exactly one wake per remaining batch instead of stalling under
        // ControlFlow::Wait with no further events. The wake event itself
        // is a no-op; the next turn's flush does the work. No busy loop:
        // one wake is scheduled per turn that still has backlog.
        if (self.page_ready
            && self.pending_batch.is_none()
            && !self.pending_host_messages.is_empty())
            || plugin_pending
        {
            let _ = self
                .proxy
                .send_coalesced(DesktopEvent::Command(DesktopCommand::FlushWebviewBatch));
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
        if let Some(batch) = self.pending_batch.as_ref() {
            platform::prepare_for_startup_wait(event_loop, batch.deadline);
        } else {
            platform::prepare_for_wait(event_loop);
        }
    }
}

impl Drop for DesktopApp {
    fn drop(&mut self) {
        self.emitter.ipc_slots.close();
        self.domain_mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .shutdown();
        if let Some(webview) = self.webview.take() {
            platform::prepare_webview_teardown(&webview);
            drop(webview);
            platform::sync_gtk_display();
        }
        self.window.take();
    }
}

#[cfg(test)]
mod update_release_link_tests {
    use super::*;

    #[test]
    fn only_the_fixed_release_page_opens_externally() {
        assert!(is_update_releases_url(UPDATE_RELEASES_URL));
        for url in [
            "http://github.com/nglmercer/TikTools-app/releases",
            "https://github.com/nglmercer/TikTools-app/releases?next=evil",
            "https://github.com.evil.example/nglmercer/TikTools-app/releases",
            "https://github.com/nglmercer/TikTools-app/releases/other",
        ] {
            assert!(!is_update_releases_url(url));
        }
    }
}
