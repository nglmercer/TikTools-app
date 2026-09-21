//! Frontend JSON-RPC message routing and WebView batch emission.

use super::DesktopApp;
use crate::event::{DesktopCommand, DesktopEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tiktools_control_api::ControlApi;
use tiktools_core::ipc::IpcRouter;
use winit::event_loop::EventLoopProxy;

/// Raw inbound bound shared with local IPC. Checked before any parsing.
pub(crate) fn webview_request_too_large(raw: &str) -> bool {
    raw.len() > tiktools_control_api::MAX_REQUEST_BYTES
}

/// Allocation-free control-shape probe used only to route oversized-payload
/// errors and malformed input (full parsing happens later, on size-capped
/// input, off the UI thread).
pub(crate) fn is_probably_control_rpc(raw: &str) -> bool {
    raw.contains("\"method\"")
}

/// Allocation-free boot-handshake probe for the Winit thread: exact match
/// on the `notifyFrontendReady` payload (plus ASCII-whitespace tolerance).
/// Anything else — including handshake whitespace variants — moves to
/// Tokio, where the parsed classifier recognizes `frontend-ready`
/// robustly. The length gate alone rejects large JSON without parsing.
pub(crate) fn is_frontend_ready_fast(raw: &str) -> bool {
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
pub(crate) enum InboundRoute {
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
pub(crate) fn classify_inbound_message(raw: &str) -> InboundRoute {
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
pub(crate) async fn handle_webview_ipc_message(
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
pub(crate) fn emit_rpc_response(
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

impl DesktopApp {
    pub(crate) fn flush_host_messages(&mut self) {
        self.flush_webview_batch();
    }

    /// Two-lane enqueue: RPC responses and critical transitions land in
    /// the reliable lane (never dropped); snapshots coalesce and feed is
    /// bounded in the lossy lane. A tripped reliable safety policy fails
    /// the transport loudly instead of growing memory without bound.
    pub(crate) fn emit_to_webview(&mut self, message: String) {
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
    pub(crate) fn flush_webview_batch(&mut self) {
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
}
