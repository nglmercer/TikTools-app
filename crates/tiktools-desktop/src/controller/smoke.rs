//! Real packaged Vue + Wry bridge, run alone because Winit owns one loop.
use super::*;
use std::sync::atomic::AtomicBool;
use tiktools_client::remote::profiles::OsCredentialStore;
use winit::platform::x11::EventLoopBuilderExtX11;
#[cfg(feature = "remote-control")]
mod commands;

struct Driver {
    app: ControllerWindow,
    passed: Arc<AtomicBool>,
    probing: Arc<AtomicBool>,
    cycles: u32,
    expected_generation: u64,
    deadline: Instant,
}
impl ApplicationHandler for Driver {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.app.resumed(event_loop);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.app.window_event(event_loop, id, event);
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.app.about_to_wait(event_loop);
        if let Some(window) = &self.app.window {
            window.set_visible(false);
        }
        assert!(
            Instant::now() < self.deadline,
            "packaged controller frontend did not become ready"
        );
        if self.passed.swap(false, Ordering::AcqRel) {
            self.cycles += 1;
            if self.cycles == 3 {
                event_loop.exit();
                return;
            }
            self.expected_generation = self.app.generation.load(Ordering::Acquire) + 1;
            self.app.webview.as_ref().unwrap().reload().unwrap();
            return;
        }
        if self.app.generation.load(Ordering::Acquire) >= self.expected_generation
            && self.app.ready.load(Ordering::Acquire) == self.app.generation.load(Ordering::Acquire)
            && !self.probing.swap(true, Ordering::AcqRel)
        {
            let passed = self.passed.clone();
            let probing = self.probing.clone();
            self.app.webview.as_ref().unwrap().evaluate_script_with_callback(
                r#"(() => {
                  const root = document.querySelector('.controller');
                  const form = root?.querySelector('details');
                  if (!window.__tiktools_controller__ || !window.ipc || !form || document.querySelector('.app-shell')) return false;
                  form.open = true;
                  root.scrollTop = root.scrollHeight;
                  const submit = root.querySelector('button[type=submit]')?.getBoundingClientRect();
                  const bounds = root.getBoundingClientRect();
                  return !!(submit && getComputedStyle(root).overflowY === 'auto' && submit.top >= bounds.top && submit.bottom <= bounds.bottom + 1);
                })()"#,
                move|result|{if result=="true"{passed.store(true,Ordering::Release);}probing.store(false,Ordering::Release);}
            ).unwrap();
        }
    }
}
#[test]
#[ignore = "requires a Linux display and built dist/web; run this test alone"]
fn packaged_controller_reloads_without_starting_a_local_host() {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dist/web")
        .canonicalize()
        .unwrap();
    assert!(assets.join("index.html").is_file());
    let home = tempfile::tempdir().unwrap();
    let controller = Arc::new(Controller::new(
        ProfileManager::new(home.path().join("profiles"), Arc::new(OsCredentialStore)).unwrap(),
    ));
    platform::initialize().unwrap();
    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    let mut builder = EventLoop::<()>::with_user_event();
    builder.with_any_thread(true);
    platform::configure_event_loop(&mut builder);
    let event_loop = builder.build().unwrap();
    let (tx, rx) = mpsc::channel(32);
    let changed = controller.subscribe();
    let app = ControllerWindow {
        source: FrontendSource::EmbeddedAssets {
            root: Arc::new(assets),
        },
        controller: controller.clone(),
        runtime: runtime.handle().clone(),
        profile: None,
        window: None,
        webview: None,
        tx,
        rx,
        changed,
        generation: Arc::new(AtomicU64::new(0)),
        ready: Arc::new(AtomicU64::new(u64::MAX)),
        deadline: None,
        observed_generation: 0,
        log_path: home.path().join("controller.log"),
    };
    let mut driver = Driver {
        app,
        passed: Arc::new(AtomicBool::new(false)),
        probing: Arc::new(AtomicBool::new(false)),
        cycles: 0,
        expected_generation: 1,
        deadline: Instant::now() + Duration::from_secs(30),
    };
    event_loop.run_app(&mut driver).unwrap();
    assert_eq!(driver.cycles, 3);
    assert_eq!(controller.status().phase, "idle");
    assert!(driver.app.generation.load(Ordering::Acquire) >= 3);
    for entry in std::fs::read_dir(home.path()).unwrap() {
        assert_eq!(entry.unwrap().file_name(), "profiles");
    }
    runtime.block_on(controller.disconnect());
}
