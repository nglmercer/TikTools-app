//! Host services and initial hidden window creation. Runtime workers, subscriptions and the
//! frontend handshake keep their original startup ordering.

use super::super::{
    domain_events::forward_domain_events, outbox::WebviewOutbox, DesktopApp, StartupState,
    FRONTEND_STARTUP_TIMEOUT,
};
use crate::{
    event::{CoalescedSignals, DesktopCommand, DesktopEvent},
    tray::TrayController,
    webview::FrontendSource,
};
#[cfg(target_os = "linux")]
use gtk::prelude::*;
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};
use tiktools_control_api::ControlApi;
use tiktools_core::{ipc::IpcRouter, AppCore};
use tokio::runtime::Handle;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
#[cfg(not(target_os = "linux"))]
use winit::{
    dpi::LogicalSize,
    window::{Icon as WindowIcon, Window},
};

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
        // Give plugins a capability-gated path back into the host. This is
        // what lets a user replace remote control with their own plugin: the
        // plugin owns its listener, the host keeps authorization and tokens.
        control.install_plugin_host_calls(runtime.clone());
        // Winit constructs this host outside an entered Tokio context.
        // Durable metadata delivery still belongs to the host lifecycle.
        core.start_coordination_worker_on(&runtime);
        #[cfg(feature = "cloud-auth")]
        core.start_cloud_sync_worker_on(&runtime);
        // Remote control is owned exclusively by the installable
        // `tiktools.remote-control` plugin. It starts itself through the
        // events.subscribe auto-start path and executes host operations via
        // the capability-gated `host.call` channel installed above.
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
            reload: super::super::reload::MainReloadSupervisor::new(),
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
            restart_requested: None,
            native_menu_model: None,
            #[cfg(target_os = "windows")]
            windows_menu: None,
        }
    }

    fn finish_window_creation(&mut self, webview: wry::WebView) {
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
    }

    #[cfg(not(target_os = "linux"))]
    pub(super) fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
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

        self.window = Some(crate::shell::ShellWindow::Winit(window));
        self.finish_window_creation(webview);
        #[cfg(target_os = "windows")]
        if let Some(model) = self.native_menu_model.clone() {
            self.rebuild_windows_menu(&model);
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(super) fn create_window(&mut self, _event_loop: &ActiveEventLoop) -> Result<(), String> {
        tracing::debug!("creating desktop window (native GTK shell)");
        let (window, content, menubar) =
            crate::shell::create_gtk_shell_window("TikTools", 900, 680, self.emitter.clone());
        // Close button replicates the Winit CloseRequested policy through
        // the command channel: hide to tray when available, shut down else.
        {
            let proxy = self.proxy.clone();
            window.connect_delete_event(move |_, _| {
                let _ = proxy.send_coalesced(crate::event::DesktopEvent::Command(
                    crate::event::DesktopCommand::MainWindowCloseRequested,
                ));
                // Always inhibit the default destroy: the command handler
                // decides whether the window hides or the process exits.
                gtk::glib::Propagation::Stop
            });
        }
        let webview = self.build_main_webview_gtk(&content)?;
        tracing::info!("Wry WebView created successfully");
        for child in content.children() {
            child.set_hexpand(true);
            child.set_vexpand(true);
        }
        if let Some(model) = self.native_menu_model.clone() {
            crate::shell::rebuild_native_menu(&menubar, &model, &self.emitter);
        }
        self.window = Some(crate::shell::ShellWindow::Gtk {
            window,
            content,
            menubar,
        });
        self.finish_window_creation(webview);
        Ok(())
    }

    pub(super) fn fail_startup(&mut self, event_loop: &ActiveEventLoop, reason: impl Into<String>) {
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
