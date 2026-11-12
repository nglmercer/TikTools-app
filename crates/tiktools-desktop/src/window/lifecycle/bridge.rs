//! Synchronous UI-thread IPC admission. Payload bounds and boot messages precede owned
//! semaphore admission; decoding stays on the runtime with the captured generation.

use super::super::{
    ipc::{
        emit_rpc_response, handle_webview_ipc_message, is_frontend_ready_fast,
        is_probably_control_rpc, parse_batch_ack, webview_request_too_large,
    },
    DesktopApp,
};
use crate::event::DesktopEvent;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tiktools_control_api::ControlApi;
use tiktools_core::ipc::IpcRouter;
use tokio::runtime::Handle;
use winit::event_loop::EventLoopProxy;

pub(super) struct MainIpcBridge {
    router: Arc<IpcRouter>,
    control: Arc<ControlApi>,
    runtime: Handle,
    proxy: EventLoopProxy<DesktopEvent>,
    transport_failed: Arc<AtomicBool>,
    emitter: Arc<crate::event::EventLoopEmitter>,
    admission: Arc<tokio::sync::Semaphore>,
}

impl MainIpcBridge {
    pub(super) fn new(app: &DesktopApp) -> Self {
        Self {
            router: app.router.clone(),
            control: app.control.clone(),
            runtime: app.runtime.clone(),
            proxy: app.proxy.clone(),
            transport_failed: app.webview_failed.clone(),
            emitter: app.emitter.clone(),
            admission: app.emitter.ipc_slots.clone(),
        }
    }

    pub(super) fn handle(&self, raw: &str) {
        if let Some((generation, sequence)) = parse_batch_ack(raw) {
            self.emitter.request_batch_ack(generation, sequence);
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
                emit_rpc_response(&self.emitter, &response);
            } else {
                tracing::warn!(bytes = raw.len(), "oversized WebView IPC message dropped");
            }
            return;
        }
        if is_frontend_ready_fast(raw) {
            self.emitter
                .request_ready(self.emitter.generation.load(Ordering::Acquire));
            return;
        }
        let permit = match self.admission.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                self.emitter
                    .inbound_rejected
                    .fetch_add(1, Ordering::Relaxed);
                if is_probably_control_rpc(raw) {
                    let response = tiktools_control_api::RpcResponse::error(
                        tiktools_control_api::RpcId::extract_from_prefix(raw),
                        tiktools_control_api::ApiError::new(
                            "overloaded",
                            "Main WebView IPC capacity exhausted",
                        ),
                    );
                    emit_rpc_response(&self.emitter, &response);
                }
                return;
            }
        };
        let raw = raw.to_owned();
        let generation = self.emitter.generation.load(Ordering::Acquire);
        let emitter = self.emitter.clone();
        let router = self.router.clone();
        let control = self.control.clone();
        let proxy = self.proxy.clone();
        let transport_failed = self.transport_failed.clone();
        self.runtime.spawn(async move {
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
    }
}
