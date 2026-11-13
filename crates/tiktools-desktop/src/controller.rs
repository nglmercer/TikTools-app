//! A separate controller window, deliberately without AppCore, IpcRouter,
//! local IPC, plugins, media services, tray or TikTok runtime construction.
use crate::{platform, webview::FrontendSource};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tiktools_client::{
    remote::{controller::Controller, profiles::ProfileManager},
    ClientError,
};
use tiktools_control_api::{ApiError, RpcId, RpcRequest, RpcResponse};
use tokio::{
    runtime::{Builder, Handle},
    sync::{mpsc, watch, Semaphore},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use wry::{PageLoadEvent, WebView, WebViewBuilder};

pub fn run(profile: Option<String>, log_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let source = FrontendSource::from_environment()?;
    let profiles = ProfileManager::system()?;
    #[cfg(feature = "cloud-auth")]
    let controller = Arc::new(Controller::with_cloud_environment(profiles)?);
    #[cfg(not(feature = "cloud-auth"))]
    let controller = Arc::new(Controller::new(profiles));
    platform::initialize()?;
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    let mut builder = EventLoop::<()>::with_user_event();
    platform::configure_event_loop(&mut builder);
    let event_loop = builder.build()?;
    let (tx, rx) = mpsc::channel(32);
    let changed = controller.subscribe();
    let mut app = ControllerWindow {
        source,
        controller,
        runtime: runtime.handle().clone(),
        profile,
        window: None,
        webview: None,
        tx,
        rx,
        changed,
        generation: Arc::new(AtomicU64::new(0)),
        ready: Arc::new(AtomicU64::new(u64::MAX)),
        deadline: None,
        observed_generation: 0,
        log_path,
    };
    event_loop.run_app(&mut app)?;
    runtime.block_on(app.controller.disconnect());
    Ok(())
}
struct Packet {
    generation: u64,
    response: RpcResponse,
}
struct ControllerWindow {
    source: FrontendSource,
    controller: Arc<Controller>,
    runtime: Handle,
    profile: Option<String>,
    window: Option<Window>,
    webview: Option<WebView>,
    tx: mpsc::Sender<Packet>,
    rx: mpsc::Receiver<Packet>,
    changed: watch::Receiver<u64>,
    generation: Arc<AtomicU64>,
    ready: Arc<AtomicU64>,
    deadline: Option<Instant>,
    observed_generation: u64,
    log_path: PathBuf,
}
impl ControllerWindow {
    fn create(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        let window = event_loop.create_window(
            Window::default_attributes()
                .with_title("TikTools — Remote controller")
                .with_visible(false)
                .with_inner_size(winit::dpi::LogicalSize::new(1000, 740)),
        )?;
        let initial = json!({"profile":self.profile}).to_string();
        let script=format!("Object.defineProperty(window,'__tiktools_controller__',{{value:{initial},writable:false,configurable:false}});");
        let source = self.source.clone();
        let controller = self.controller.clone();
        let runtime = self.runtime.clone();
        let tx = self.tx.clone();
        let generation = self.generation.clone();
        let ready = self.ready.clone();
        let page_generation = generation.clone();
        let admission = Arc::new(Semaphore::new(32));
        let mut builder = WebViewBuilder::new()
            .with_initialization_script(&script)
            .with_initialization_script(crate::webview::MAIN_INIT_SCRIPT)
            .with_navigation_handler(move |url| source.allows_navigation(&url))
            .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
            .with_on_page_load_handler(move |event, _| {
                if matches!(event, PageLoadEvent::Started) {
                    page_generation.fetch_add(1, Ordering::AcqRel);
                }
            })
            .with_ipc_handler(move |request| {
                let raw = request.body();
                let epoch = generation.load(Ordering::Acquire);
                if raw == r#"{"type":"frontend-ready"}"# {
                    ready.store(epoch, Ordering::Release);
                    return;
                }
                let permit = admission.clone().try_acquire_owned();
                if raw.len() > 1024 * 1024 || permit.is_err() {
                    let code = if raw.len() > 1024 * 1024 {
                        "too_large"
                    } else {
                        "overloaded"
                    };
                    let response = RpcResponse::error(
                        RpcId::extract_from_prefix(raw),
                        ApiError::new(code, "controller bridge admission rejected"),
                    );
                    let _ = tx.try_send(Packet {
                        generation: epoch,
                        response,
                    });
                    return;
                }
                let permit = permit.unwrap();
                let raw = raw.clone();
                let tx = tx.clone();
                let controller = controller.clone();
                let generation = generation.clone();
                runtime.spawn(async move {
                    let _permit = permit;
                    if generation.load(Ordering::Acquire) != epoch {
                        return;
                    }
                    let response = bridge(&controller, &raw).await;
                    if generation.load(Ordering::Acquire) == epoch {
                        let _ = tx
                            .send(Packet {
                                generation: epoch,
                                response,
                            })
                            .await;
                    }
                });
            });
        if let Some(assets) = self.source.asset_server() {
            builder =
                builder.with_custom_protocol("tiktools".into(), move |_, req| assets.respond(req));
        }
        let size = window.inner_size();
        builder = builder
            .with_bounds(wry::Rect {
                position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                size: wry::dpi::PhysicalSize::new(size.width.max(1), size.height.max(1)).into(),
            })
            .with_url(self.source.url().as_str());
        self.webview = Some(platform::build_webview(builder, &window)?);
        self.window = Some(window);
        self.deadline = Some(Instant::now() + Duration::from_secs(15));
        Ok(())
    }
    fn emit(&self, value: Value) {
        let Some(webview) = &self.webview else {
            return;
        };
        if let Ok(raw) = serde_json::to_string(&value) {
            if let Ok(quoted) = serde_json::to_string(&raw) {
                let _ =
                    webview.evaluate_script(&format!("window.__webview_on_message__?.({quoted});"));
            }
        }
    }
}
impl ApplicationHandler for ControllerWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(e) = self.create(event_loop) {
                crate::show_startup_failure(&e.to_string(), &self.log_path);
                event_loop.exit();
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(webview) = &self.webview {
                    let _ = webview.set_bounds(wry::Rect {
                        position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                        size: wry::dpi::PhysicalSize::new(size.width.max(1), size.height.max(1))
                            .into(),
                    });
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        platform::pump();
        let generation = self.generation.load(Ordering::Acquire);
        if self.observed_generation != generation {
            self.observed_generation = generation;
            self.deadline = Some(Instant::now() + Duration::from_secs(15));
        }
        let ready = self.ready.load(Ordering::Acquire) == generation;
        // RPC replies can be needed by component setup before frontend-ready.
        for _ in 0..8 {
            match self.rx.try_recv() {
                Ok(packet) if packet.generation == generation => {
                    self.emit(json!({"type":"rpc-response","response":packet.response}))
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        if self.changed.has_changed().unwrap_or(false) {
            self.changed.borrow_and_update();
            self.emit(json!({"jsonrpc":"2.0","method":"event","params":{"topic":"controller.changed","data":{}}}));
        }
        if ready {
            if let Some(window) = &self.window {
                window.set_visible(true);
            }
            self.deadline = None;
        } else if self.deadline.is_some_and(|d| Instant::now() >= d) {
            crate::show_startup_failure("Controller frontend did not become ready", &self.log_path);
            event_loop.exit();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }
}
async fn bridge(controller: &Controller, raw: &str) -> RpcResponse {
    let request: RpcRequest = match serde_json::from_str(raw) {
        Ok(r) => r,
        Err(_) => {
            return RpcResponse::error(
                RpcId::extract_from_prefix(raw),
                ApiError::invalid_params("invalid controller request"),
            )
        }
    };
    let id = request.id.clone();
    let result = async {
        if request.jsonrpc.as_deref() != Some("2.0") || matches!(id, RpcId::Null) {
            return Err(ClientError::new(
                "invalid_params",
                "controller requires a correlated JSON-RPC request",
            ));
        }
        if request.method == "controller.forward" {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Forward {
                epoch: String,
                method: String,
                params: Value,
                meta: Option<tiktools_client::RequestMetadata>,
            }
            let p: Forward = serde_json::from_value(request.params)
                .map_err(|_| ClientError::new("invalid_params", "invalid forward parameters"))?;
            controller
                .forward(&p.epoch, &p.method, p.params, p.meta)
                .await
        } else if request.method == "controller.workflows" {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase", deny_unknown_fields)]
            struct Page {
                epoch: String,
                view_revision: String,
                offset: usize,
            }
            let p: Page = serde_json::from_value(request.params)
                .map_err(|_| ClientError::new("invalid_params", "invalid page parameters"))?;
            Ok(tiktools_client::CallResult {
                result: controller.workflows(&p.epoch, &p.view_revision, p.offset)?,
                meta: None,
            })
        } else {
            Ok(tiktools_client::CallResult {
                result: controller.manage(&request.method, request.params).await?,
                meta: None,
            })
        }
    }
    .await;
    match result {
        Ok(answer) => {
            let mut response = RpcResponse::success(id, answer.result);
            response.meta = answer.meta;
            response
        }
        Err(error) => RpcResponse::error(
            id,
            ApiError {
                code: error.code,
                message: error.message,
                data: error.data,
            },
        ),
    }
}

#[cfg(all(test, target_os = "linux"))]
mod smoke;
