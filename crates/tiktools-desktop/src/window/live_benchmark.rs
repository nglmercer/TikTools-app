//! Opt-in actual Wry + compiled Vue synthetic LIVE benchmark, one process per case.
//! Build assets with VITE_TIKTOOLS_DIAGNOSTICS=1. It never connects to TikTok.
use super::DesktopApp;
use crate::event::{DesktopCommand, DesktopEvent};
use std::io::Write;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

struct Driver {
    app: DesktopApp,
    began: Instant,
    warmup: Duration,
    load: Duration,
    cleanup: Duration,
    next_sample: Instant,
    submitted: Arc<AtomicU64>,
    output: std::fs::File,
    case: String,
    configured: bool,
    samples: u64,
    remounts: u64,
    source_task: tokio::task::JoinHandle<()>,
}
impl Driver {
    fn phase(&self) -> &'static str {
        let elapsed = self.began.elapsed();
        if elapsed < self.warmup {
            "idle"
        } else if elapsed < self.warmup + self.load {
            "load"
        } else {
            "cleanup"
        }
    }
    fn record(&mut self, ui: serde_json::Value) {
        let record = serde_json::json!({"elapsedMs": self.began.elapsed().as_millis(),
            "phase": self.phase(), "case": self.case, "sourceEvents": self.submitted.load(Ordering::Relaxed),
            "health": self.app.core.system_health(), "ui": ui, "remounts": self.remounts});
        writeln!(self.output, "{record}").unwrap();
        self.output.flush().unwrap();
        self.samples += 1;
    }
}
impl ApplicationHandler<DesktopEvent> for Driver {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.app.resumed(event_loop);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.app.window_event(event_loop, id, event);
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: DesktopEvent) {
        if let DesktopEvent::Command(DesktopCommand::AdmittedCommand { command, .. }) = &event {
            if let DesktopCommand::BenchmarkSample { data } = command.as_ref() {
                assert!(
                    !data.is_null(),
                    "build Vue assets with VITE_TIKTOOLS_DIAGNOSTICS=1"
                );
                self.record(data.clone());
                return;
            }
        }
        self.app.user_event(event_loop, event);
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.app.about_to_wait(event_loop);
        if self.app.page_ready && !self.configured {
            let tab = if self.case == "D" { "settings" } else { "feed" };
            let images = self.case != "E";
            let row_limit: usize = std::env::var("TIKTOOLS_BENCH_ROW_LIMIT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let disable_blur = std::env::var_os("TIKTOOLS_BENCH_DISABLE_ROW_BLUR").is_some();
            let override_rows = if row_limit > 0 {
                format!("window.__tiktools_benchmark_controls__.setRenderLimit({row_limit});")
            } else {
                String::new()
            };
            let script = format!("window.__tiktools_benchmark_controls__.setTab('{tab}'); window.__tiktools_benchmark_controls__.setImagesEnabled({images});{override_rows}");
            self.app
                .webview
                .as_ref()
                .unwrap()
                .evaluate_script(&script)
                .unwrap();
            if disable_blur {
                self.app.webview.as_ref().unwrap().evaluate_script("(() => {const style=document.createElement('style');style.textContent='.tt-badge-level{backdrop-filter:none!important;-webkit-backdrop-filter:none!important}';document.head.append(style);})();").unwrap();
            }
            if self.case == "B" {
                self.app.user_event(
                    event_loop,
                    DesktopEvent::Command(DesktopCommand::HideWindow),
                );
            }
            self.configured = true;
        }
        if self.app.page_ready && Instant::now() >= self.next_sample {
            self.next_sample = Instant::now() + Duration::from_secs(1);
            self.app.webview.as_ref().unwrap().evaluate_script(
                "window.ipc.postMessage(JSON.stringify({type:'benchmark-sample',data:window.__tiktools_frontend_diagnostics__?.() ?? null}));"
            ).unwrap();
            if self.case == "J" && self.remounts < 100 && self.phase() == "load" {
                match std::env::var("TIKTOOLS_BENCH_LIFECYCLE").as_deref() {
                    Ok("reload") => {
                        self.app.webview.as_ref().unwrap().reload().unwrap();
                    }
                    _ => {
                        self.app
                            .webview
                            .as_ref()
                            .unwrap()
                            .evaluate_script("window.__tiktools_remount__();")
                            .unwrap();
                    }
                }
                self.remounts += 1;
            }
        }
        if self.began.elapsed() >= self.warmup + self.load + self.cleanup {
            // Source stops before cleanup; the normal core shutdown drains all
            // accepted persistence/plugin work and releases the renderer.
            assert!(
                self.source_task.is_finished(),
                "source task must finish before cleanup ends"
            );
            self.app
                .user_event(event_loop, DesktopEvent::Command(DesktopCommand::Quit));
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(20),
        ));
    }
}
fn seconds(name: &str, default: u64) -> Duration {
    Duration::from_secs(
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default),
    )
}

#[test]
fn native_live_matrix() {
    // Test binaries do not run main(), where the application installs the provider.
    tiktools_tls::ensure_default_crypto_provider();
    let Ok(case) = std::env::var("TIKTOOLS_BENCH_CASE") else {
        eprintln!("skipped: set TIKTOOLS_BENCH_CASE and isolated TIKTOOLS_HOME");
        return;
    };
    assert!(["A", "B", "C", "D", "E", "F", "G", "H", "J"].contains(&case.as_str()));
    let home = std::path::PathBuf::from(std::env::var_os("TIKTOOLS_HOME").unwrap());
    assert!(
        home.starts_with(std::env::temp_dir()),
        "benchmark data must be isolated in temp"
    );
    std::fs::create_dir_all(&home).unwrap();
    let warmup = seconds("TIKTOOLS_BENCH_WARMUP", 30);
    let load = seconds("TIKTOOLS_BENCH_LOAD", 600);
    let cleanup = seconds("TIKTOOLS_BENCH_CLEANUP", 120);
    let rate: u64 = std::env::var("TIKTOOLS_BENCH_RATE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    assert!((10..=10_000).contains(&rate) && rate.is_multiple_of(10));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let began = Instant::now();
    let submitted = Arc::new(AtomicU64::new(0));
    let output_path = std::env::var_os("TIKTOOLS_BENCH_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join("benchmark.ndjson"));
    let mut output = std::fs::File::create(output_path).unwrap();
    writeln!(output, "{}", serde_json::json!({"metadata": {"revision": std::env::var("TIKTOOLS_BENCH_REVISION").expect("record exact revision / dirty patch hash"), "pid": std::process::id(), "os": std::env::consts::OS, "case": case, "sourceRate": rate, "synthetic": true, "rowBlurOverride": if std::env::var_os("TIKTOOLS_BENCH_DISABLE_ROW_BLUR").is_some() { "disabled" } else { "stylesheet-default" }, "lifecycle": std::env::var("TIKTOOLS_BENCH_LIFECYCLE").unwrap_or_else(|_| "root".to_owned())}})).unwrap();
    let assets = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dist/web")
        .canonicalize()
        .unwrap();
    std::fs::write(
        assets.join("benchmark-avatar.png"),
        include_bytes!("../../../../tests/fixtures/live-benchmark/avatar.png"),
    )
    .unwrap();
    let configure_actions =
        |core: &Arc<tiktools_core::AppCore>| {
            if case == "G" {
                return;
            }
            use tiktools_core::control::AutomationKind;
            core.automation_save(AutomationKind::Action, &serde_json::json!({
            "id": "benchmark-log", "name": "Benchmark log", "typeId": "core.log", "enabled": true,
            "config": {"message": "synthetic completion"}
        })).unwrap();
            core.automation_save(AutomationKind::Event, &serde_json::json!({
            "id": "benchmark-chat", "name": "Benchmark chat", "enabled": true,
            "trigger": "tiktok.chat", "filters": [], "cooldownMs": 0, "cooldownScope": "user",
            "actionIds": ["benchmark-log"], "runMode": "all"
        })).unwrap();
        };
    let start_source = |core: Arc<tiktools_core::AppCore>| {
        let submitted = submitted.clone();
        let case = case.clone();
        runtime.spawn(async move {
            let mut input = core.start_live_benchmark().await.unwrap();
            tokio::time::sleep(warmup).await;
            let mut clock = tokio::time::interval(Duration::from_millis(100));
            clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut index = 0;
            if case == "H" {
                for _ in 0..10_000 {
                    let image = format!("./benchmark-avatar.png?viewer={}", index % 100);
                    input.submit(index, 100, true, Some(&image)).await;
                    index += 1;
                }
                submitted.store(index, Ordering::Relaxed);
            }
            let target = index + rate * load.as_secs();
            while index < target {
                clock.tick().await;
                for _ in 0..rate / 10 {
                    let image = format!("./benchmark-avatar.png?viewer={}", index % 100);
                    input.submit(index, 100, true, Some(&image)).await;
                    index += 1;
                }
                submitted.store(index, Ordering::Relaxed);
            }
            drop(input); // closes admission so the worker can drain normally
        })
    };
    if case == "A" {
        struct Noop;
        impl tiktools_core::HostEmitter for Noop {
            fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
            fn wants_automation_history(&self) -> bool {
                false
            }
            fn wants_leaderboard_snapshots(&self) -> bool {
                false
            }
        }
        let core = Arc::new(tiktools_core::AppCore::new(Arc::new(Noop)));
        configure_actions(&core);
        core.spawn_plugin_event_poll(runtime.handle());
        let source = start_source(core.clone());
        runtime.block_on(async {
            while began.elapsed() < warmup + load + cleanup {
                let phase = if began.elapsed() < warmup { "idle" } else if began.elapsed() < warmup + load { "load" } else { "cleanup" };
                writeln!(output, "{}", serde_json::json!({"phase": phase, "sourceEvents": submitted.load(Ordering::Relaxed), "health": core.system_health()})).unwrap();
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            source.await.unwrap(); core.shutdown().await;
        });
        return;
    }
    crate::platform::initialize().unwrap();
    let mut builder = EventLoop::<DesktopEvent>::with_user_event();
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_any_thread(true);
    }
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        builder.with_any_thread(true);
    }
    crate::platform::configure_event_loop(&mut builder);
    let event_loop = builder.build().unwrap();
    let proxy = event_loop.create_proxy();
    let emitter = crate::event::shared_emitter(proxy.clone());
    if case == "F" {
        emitter
            .projection
            .disable_points
            .store(true, Ordering::Relaxed);
    }
    let core = Arc::new(tiktools_core::AppCore::new(emitter.clone()));
    configure_actions(&core);
    let router = Arc::new(tiktools_core::ipc::IpcRouter::new(core.clone()));
    let source_task = start_source(core.clone());
    let root = assets;
    let app = DesktopApp::new(
        core,
        router,
        crate::webview::FrontendSource::EmbeddedAssets {
            root: Arc::new(root),
        },
        runtime.handle().clone(),
        proxy,
        home.join("benchmark.log"),
        emitter,
    );
    let mut driver = Driver {
        app,
        began,
        warmup,
        load,
        cleanup,
        next_sample: Instant::now(),
        submitted,
        output,
        case,
        configured: false,
        samples: 0,
        remounts: 0,
        source_task,
    };
    event_loop.run_app(&mut driver).unwrap();
    runtime.block_on(driver.source_task).unwrap();
    assert!(
        driver.samples > 0,
        "actual Vue page must report diagnostics"
    );
    assert!(
        driver.app.core.system_health()["liveIngress"]["outstanding"]
            .as_i64()
            .unwrap()
            == 0
    );
}
