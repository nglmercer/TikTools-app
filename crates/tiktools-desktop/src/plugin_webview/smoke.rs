//! Native smoke test for plugin windows: open/close cycles plus a broker
//! round-trip against the real Wry/Winit/GTK stack.
//!
//! Gated on `TIKTOOLS_TEST_NATIVE_WINDOWS=1` and a display: headless runs
//! skip silently. Run with:
//!
//! ```bash
//! TIKTOOLS_TEST_NATIVE_WINDOWS=1 cargo test -p tiktools-desktop --locked smoke -- --nocapture
//! ```
//!
//! The test page probes `window.tiktools` from real page JavaScript and
//! reports through the native IPC channel, so a passing run proves the
//! init script installed the broker surface on this platform's WebView.
//! Repeated open/close cycles — direct closes and deferred
//! `CloseRequested` closes — exercise the ordered teardown (child WebView,
//! display sync, parent window) that once crashed X11 sessions. A
//! regression fails loudly here (panic, X error abort, or timeout)
//! instead of in a user's session.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tiktools_control_api::ControlApi;
use tiktools_core::AppCore;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};

use super::{PluginUiWindows, PluginWindowState};
use crate::event::{DesktopCommand, DesktopEvent};

const PLUGIN_ID: &str = "smokeprobe";
const PAGE_ID: &str = "main";
const CYCLES: u32 = 3;
const PROBE_TIMEOUT: Duration = Duration::from_secs(25);
const GLOBAL_TIMEOUT: Duration = Duration::from_secs(120);

const PROBE_HTML: &str = r#"<!doctype html><html><head><meta charset="utf-8"></head><body><div id="app">probe</div><script src="./probe.js"></script></body></html>"#;

/// Page-side probe. The id of the single broker request encodes whether
/// the init script installed `window.tiktools`; the request itself is an
/// allowlisted `host.locale` call, so the Rust side answers through the
/// normal `PluginUiRespond` path and the driver observes the id there.
/// External script (not inline): the plugin CSP forbids inline scripts.
const PROBE_JS: &str = r#"(function () {
  var id = (typeof window.tiktools === 'object' && window.tiktools !== null)
    ? 'has-tiktools' : 'missing-tiktools';
  window.ipc.postMessage(JSON.stringify({ apiVersion: 1, id: id, method: 'host.locale', params: {} }));
})();"#;

struct NullEmitter;

impl tiktools_core::HostEmitter for NullEmitter {
    fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
}

fn probe_core(root: &std::path::Path) -> Arc<AppCore> {
    use tiktools_plugin_loader::{PluginManager, PluginRoot, PluginSource};
    let dir = root.join(PLUGIN_ID);
    std::fs::create_dir_all(dir.join("ui/dist")).unwrap();
    std::fs::write(
        dir.join("plugin.json"),
        serde_json::json!({
            "schemaVersion": 3,
            "id": PLUGIN_ID,
            "name": "smokeprobe",
            "version": "1.0.0",
            "runtime": "declarative",
            "capabilities": [],
            "permissions": [],
            "actionTypes": [],
            "ui": {
                "apiVersion": 1,
                "mode": "webview",
                "entry": "ui/dist/index.html",
                "pages": [{"id": PAGE_ID, "title": {"default": "Probe"}}],
            },
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(dir.join("ui/dist/index.html"), PROBE_HTML).unwrap();
    std::fs::write(dir.join("ui/dist/probe.js"), PROBE_JS).unwrap();
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.to_path_buf(),
        source: PluginSource::Development,
    }]);
    manager.scan().expect("smoke plugin scan");
    let mut core = AppCore::new(Arc::new(NullEmitter));
    core.plugins = Arc::new(manager);
    Arc::new(core)
}

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Open,
    WaitProbe { deadline: Instant },
    Close,
    WaitClosed,
}

struct Driver {
    core: Arc<AppCore>,
    control: Arc<ControlApi>,
    runtime: tokio::runtime::Runtime,
    proxy: EventLoopProxy<DesktopEvent>,
    windows: PluginUiWindows,
    cycle: u32,
    phase: Phase,
    global_deadline: Instant,
    failure: Option<String>,
}

impl Driver {
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        if self.failure.is_none() {
            self.failure = Some(message);
        }
        event_loop.exit();
    }

    fn window_id(&self) -> Option<winit::window::WindowId> {
        self.windows
            .windows
            .values()
            .next()
            .and_then(|entry| entry.window.as_ref().map(|handle| handle.id()))
    }

    fn drive(&mut self, event_loop: &ActiveEventLoop) {
        if self.failure.is_some() {
            return;
        }
        if Instant::now() > self.global_deadline {
            self.fail(
                event_loop,
                format!("native smoke test exceeded {GLOBAL_TIMEOUT:?}"),
            );
            return;
        }
        match &self.phase {
            Phase::Open => {
                let handle = self.runtime.handle().clone();
                match self.windows.open(
                    event_loop,
                    &self.core,
                    &self.control,
                    &handle,
                    &self.proxy,
                    PLUGIN_ID,
                    PAGE_ID,
                ) {
                    Ok(true) => {
                        self.phase = Phase::WaitProbe {
                            deadline: Instant::now() + PROBE_TIMEOUT,
                        };
                    }
                    Ok(false) => self.fail(
                        event_loop,
                        format!("cycle {}: window already open after close", self.cycle),
                    ),
                    Err(error) => self.fail(
                        event_loop,
                        format!("cycle {}: open failed: {error}", self.cycle),
                    ),
                }
            }
            Phase::WaitProbe { deadline } => {
                if Instant::now() > *deadline {
                    self.fail(
                        event_loop,
                        format!(
                            "cycle {}: no broker probe within {PROBE_TIMEOUT:?} \
                             (page did not load or window.tiktools is missing)",
                            self.cycle
                        ),
                    );
                }
            }
            Phase::Close => {
                if self.cycle.is_multiple_of(2) {
                    // Deferred native close: synthetic `CloseRequested`
                    // must mark the entry closing and queue exactly one
                    // `ClosePluginUi`, which `user_event` below runs.
                    let Some(id) = self.window_id() else {
                        self.fail(
                            event_loop,
                            format!("cycle {}: window vanished before close", self.cycle),
                        );
                        return;
                    };
                    let proxy = self.proxy.clone();
                    if !self
                        .windows
                        .on_window_event(&proxy, id, &WindowEvent::CloseRequested)
                    {
                        self.fail(
                            event_loop,
                            format!("cycle {}: CloseRequested not consumed", self.cycle),
                        );
                        return;
                    }
                    let closing = self
                        .windows
                        .windows
                        .values()
                        .next()
                        .is_some_and(|entry| entry.state == PluginWindowState::Closing);
                    if !closing {
                        self.fail(
                            event_loop,
                            format!("cycle {}: CloseRequested did not mark closing", self.cycle),
                        );
                        return;
                    }
                } else {
                    // Direct API close (frontend close requests and
                    // teardown topics take this path).
                    self.windows.close(PLUGIN_ID, PAGE_ID);
                }
                self.phase = Phase::WaitClosed;
            }
            Phase::WaitClosed => {
                if self.windows.windows.is_empty() {
                    self.cycle += 1;
                    if self.cycle >= CYCLES {
                        event_loop.exit();
                    } else {
                        self.phase = Phase::Open;
                    }
                }
            }
        }
    }

    fn on_respond(&mut self, event_loop: &ActiveEventLoop, response: &str) {
        let value: serde_json::Value = match serde_json::from_str(response) {
            Ok(value) => value,
            Err(_) => return,
        };
        let id = value
            .get("id")
            .and_then(|id| id.as_str())
            .unwrap_or_default();
        if id == "missing-tiktools" {
            self.fail(
                event_loop,
                "page reports window.tiktools is missing (init script failed on this WebView)"
                    .to_owned(),
            );
            return;
        }
        if id != "has-tiktools" {
            return;
        }
        if value.get("ok") != Some(&serde_json::Value::Bool(true)) {
            self.fail(event_loop, format!("broker probe rejected: {response}"));
            return;
        }
        if matches!(self.phase, Phase::WaitProbe { .. }) {
            self.phase = Phase::Close;
        }
    }
}

impl ApplicationHandler<DesktopEvent> for Driver {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Real window-manager events for the probe window flow through the
        // same production path as the app (a user-driven close races the
        // driver harmlessly: states make every close idempotent).
        let proxy = self.proxy.clone();
        self.windows.on_window_event(&proxy, window_id, &event);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: DesktopEvent) {
        let DesktopEvent::Command(command) = event;
        match command {
            DesktopCommand::ClosePluginUi { plugin_id, page_id } => {
                self.windows.close(&plugin_id, &page_id);
            }
            DesktopCommand::PluginUiRespond { response, .. } => {
                self.on_respond(event_loop, &response);
            }
            DesktopCommand::PluginUiSubscribe {
                plugin_id,
                page_id,
                effect,
            } => {
                self.windows
                    .apply_subscription(&plugin_id, &page_id, effect);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        crate::platform::pump();
        self.drive(event_loop);
        // Gentle poll: the phases are deadline-driven, not frame-driven.
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn native_open_close_cycles() {
    if std::env::var_os("TIKTOOLS_TEST_NATIVE_WINDOWS").is_none() {
        eprintln!("skipped: set TIKTOOLS_TEST_NATIVE_WINDOWS=1 to run the native smoke test");
        return;
    }
    if std::env::var_os("DISPLAY").is_none() {
        eprintln!("skipped: native smoke test needs an X11 display (DISPLAY is unset)");
        return;
    }
    crate::platform::initialize().expect("platform init for native smoke test");
    let root = std::env::temp_dir().join(format!(
        "tiktools-native-smoke-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let core = probe_core(&root);
    let control = Arc::new(ControlApi::new(core.clone()));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("smoke test tokio runtime");
    // Test runners execute on a worker thread; Winit only allows that
    // through the explicit X11 opt-out (the `DISPLAY` gate above plus
    // `configure_event_loop` guarantee the X11 backend here, matching the
    // app's own Linux setup).
    let mut builder = EventLoop::<DesktopEvent>::with_user_event();
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_any_thread(true);
    }
    crate::platform::configure_event_loop(&mut builder);
    let event_loop = builder.build().expect("smoke test event loop");
    let proxy = event_loop.create_proxy();
    let mut driver = Driver {
        core,
        control,
        runtime,
        proxy,
        windows: PluginUiWindows::default(),
        cycle: 0,
        phase: Phase::Open,
        global_deadline: Instant::now() + GLOBAL_TIMEOUT,
        failure: None,
    };
    // A teardown crash aborts the process here instead of passing: that is
    // the regression signal this test exists to catch.
    let _ = event_loop.run_app(&mut driver);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(driver.cycle, CYCLES, "smoke test did not finish all cycles");
    if let Some(failure) = driver.failure {
        panic!("native smoke test failed: {failure}");
    }
}
