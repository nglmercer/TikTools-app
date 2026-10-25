//! Frontend JSON-RPC message routing and WebView batch emission.

use super::DesktopApp;
use crate::event::{DesktopCommand, DesktopEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tiktools_control_api::ControlApi;
use tiktools_core::ipc::IpcRouter;
use winit::event_loop::EventLoopProxy;

pub(crate) struct PendingBatch {
    pub(crate) generation: u64,
    pub(crate) sequence: u64,
    pub(crate) messages: Vec<super::outbox::QueuedWebviewMessage>,
    pub(crate) deadline: Instant,
}

/// Payload-free acknowledgment admission is handled before the main RPC
/// semaphore, so a saturated RPC lane cannot prevent transport progress.
pub(crate) fn parse_batch_ack(raw: &str) -> Option<(u64, u64)> {
    if raw.len() > 64 {
        return None;
    }
    let (generation, sequence) = raw.strip_prefix("tiktools-batch-ack:")?.split_once(':')?;
    Some((generation.parse().ok()?, sequence.parse().ok()?))
}

/// Raw inbound bound shared with local IPC. Checked before any parsing.
pub(crate) fn webview_request_too_large(raw: &str) -> bool {
    raw.len() > tiktools_control_api::MAX_REQUEST_BYTES
}

/// Allocation-free control-shape probe used only to route oversized-payload
/// errors and malformed input (full parsing happens later, on size-capped
/// input, off the UI thread).
pub(crate) fn is_probably_control_rpc(raw: &str) -> bool {
    raw.as_bytes()[..raw.len().min(4096)]
        .windows(8)
        .any(|part| part == b"\"method\"")
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
    emitter: &crate::event::EventLoopEmitter,
    admission: (u64, Arc<tokio::sync::OwnedSemaphorePermit>),
) {
    let (generation, admission) = admission;
    if emitter.generation.load(Ordering::Acquire) != generation {
        return;
    }
    // A failed transport rejects RPC loudly instead of queueing work for
    // a dead page; legacy input is diagnosed and dropped.
    if transport_failed.load(Ordering::SeqCst) {
        handle_failed_transport_message(&raw, emitter, generation);
        return;
    }
    match classify_inbound_message(&raw) {
        InboundRoute::Control(value) if !page_generation_matches(&value, generation) => {
            emit_rpc_response_for_generation(
                emitter,
                generation,
                &tiktools_control_api::RpcResponse::error(
                    tiktools_control_api::RpcId::extract(&value),
                    tiktools_control_api::ApiError::new(
                        "stale",
                        "request belongs to a retired main page",
                    ),
                ),
            );
        }
        InboundRoute::Control(value)
            if value.get("method").and_then(serde_json::Value::as_str) == Some("ui.projection") =>
        {
            let response = configure_projection(&value, emitter, generation);
            emit_rpc_response_for_generation(emitter, generation, &response);
        }
        InboundRoute::Control(value) => {
            // A deadline reports uncertain completion; it never cancels an
            // irreversible operation or releases its permit early. The owned
            // task reconciles by waiting for completion, suppressing a second
            // reply. Authoritative reads determine the final state.
            let execution = control.execute_value(&value);
            tokio::pin!(execution);
            let response = tokio::select! {
                response = &mut execution => Some(response),
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                    emitter.inbound_timeouts.fetch_add(1, Ordering::Relaxed);
                    if emitter.generation.load(Ordering::Acquire) == generation {
                        emit_rpc_response_for_generation(emitter, generation, &tiktools_control_api::RpcResponse::error(
                            tiktools_control_api::RpcId::extract(&value),
                            tiktools_control_api::ApiError::new("deadline", "RPC deadline exceeded; operation may still complete; refresh authoritative state before retrying"),
                        ));
                    }
                    execution.await;
                    None
                }
            };
            if emitter.generation.load(Ordering::Acquire) == generation {
                if let Some(response) = response {
                    emit_rpc_response_for_generation(emitter, generation, &response);
                }
            }
        }
        #[cfg(all(test, feature = "live-benchmark"))]
        InboundRoute::Legacy(value)
            if value["type"] == "benchmark-sample" && raw.len() <= 16 * 1024 =>
        {
            let _ = proxy.send_event(DesktopEvent::Command(DesktopCommand::AdmittedCommand {
                command: Box::new(DesktopCommand::BenchmarkSample {
                    data: value["data"].clone(),
                }),
                admission,
            }));
        }
        InboundRoute::Legacy(value)
            if try_plugin_ui_command(&value, proxy, admission.clone()).is_some() => {}
        InboundRoute::Legacy(value) => {
            if let Err(error) = router.dispatch_value(&value).await {
                tracing::warn!(%error, "invalid WebView IPC message");
            }
        }
        InboundRoute::FrontendReady => {
            emitter.request_ready(generation);
        }
        InboundRoute::MalformedControl(error) => {
            let response = tiktools_control_api::RpcResponse::error(
                tiktools_control_api::RpcId::extract_from_prefix(&raw),
                tiktools_control_api::ApiError::invalid_params(format!("invalid JSON: {error}")),
            );
            emit_rpc_response_for_generation(emitter, generation, &response);
        }
        InboundRoute::MalformedLegacy => {
            tracing::warn!("invalid legacy WebView message");
        }
    }
}

pub(crate) fn page_generation_matches(value: &serde_json::Value, generation: u64) -> bool {
    match value.get("pageGeneration") {
        None => true, // Bootstrap and existing clients retain their contract.
        Some(declared) => declared.as_u64() == Some(generation),
    }
}

/// This capability is main-WebView-only; CLI and native plugin RPC keep
/// the complete shared API/stream. Request revisions fence task reordering.
fn configure_projection(
    value: &serde_json::Value,
    emitter: &crate::event::EventLoopEmitter,
    generation: u64,
) -> tiktools_control_api::RpcResponse {
    let id = tiktools_control_api::RpcId::extract(value);
    let params = &value["params"];
    let parsed = (
        params["version"].as_u64(),
        params["revision"].as_u64(),
        params["feed"].as_bool(),
        params["behavior"].as_bool(),
        params["points"].as_bool(),
        params["visible"].as_bool(),
    );
    let (Some(1), Some(revision), Some(feed), Some(behavior), Some(points), Some(visible)) = parsed
    else {
        return tiktools_control_api::RpcResponse::error(id,
            tiktools_control_api::ApiError::invalid_params("ui.projection requires version 1, revision and boolean feed/behavior/points/visible"));
    };
    if !emitter
        .projection
        .update(generation, revision, feed, behavior, points, visible)
    {
        let current = emitter.projection.view();
        return tiktools_control_api::RpcResponse::error(
            id,
            tiktools_control_api::ApiError {
                code: "stale".to_owned(),
                message: "presentation request belongs to a retired page or revision".to_owned(),
                data: Some(
                    serde_json::json!({"generation": current.generation, "revision": current.revision}),
                ),
            },
        );
    }
    tiktools_control_api::RpcResponse::success(
        id,
        serde_json::json!({"version": 1, "generation": generation, "revision": revision, "stats": emitter.projection.diagnostics()}),
    )
}

/// Desktop-level plugin UI commands from the main frontend
/// (`{"type":"plugin-ui-open","pluginId","pageId"}` and the `-close`
/// twin). These open native windows, so the desktop layer owns them —
/// they never reach the control plane or the legacy page router.
/// Returns `Some(())` when the value was a plugin UI command.
fn try_plugin_ui_command(
    value: &serde_json::Value,
    proxy: &EventLoopProxy<DesktopEvent>,
    admission: Arc<tokio::sync::OwnedSemaphorePermit>,
) -> Option<()> {
    let command = value.get("type")?.as_str()?;
    if command != "plugin-ui-open" && command != "plugin-ui-close" {
        return None;
    }
    let plugin_id = value.get("pluginId")?.as_str()?;
    let page_id = value.get("pageId")?.as_str()?;
    if !crate::plugin_webview::is_valid_ui_id(plugin_id)
        || !crate::plugin_webview::is_valid_ui_id(page_id)
    {
        tracing::warn!("ignoring plugin UI command with invalid ids");
        return Some(());
    }
    let command = if command == "plugin-ui-open" {
        DesktopCommand::OpenPluginUi {
            plugin_id: plugin_id.to_owned(),
            page_id: page_id.to_owned(),
        }
    } else {
        DesktopCommand::ClosePluginUi {
            plugin_id: plugin_id.to_owned(),
            page_id: page_id.to_owned(),
            window_id: None,
            owner: None,
        }
    };
    let _ = proxy.send_event(DesktopEvent::Command(DesktopCommand::AdmittedCommand {
        command: Box::new(command),
        admission,
    }));
    Some(())
}

/// Inbound dispatch while the transport is failed: control calls are
/// rejected with a transport failure (the id lets the rebooting frontend
/// fail the call instead of hanging it), the boot handshake still flows
/// so recovery can complete, and everything else is diagnosed.
fn handle_failed_transport_message(
    raw: &str,
    emitter: &crate::event::EventLoopEmitter,
    generation: u64,
) {
    match classify_inbound_message(raw) {
        InboundRoute::Control(value) => {
            let response = tiktools_control_api::RpcResponse::error(
                tiktools_control_api::RpcId::extract(&value),
                tiktools_control_api::ApiError::new(
                    "transport",
                    "WebView transport failed; the UI is reloading",
                ),
            );
            emit_rpc_response_for_generation(emitter, generation, &response);
        }
        InboundRoute::MalformedControl(error) => {
            let response = tiktools_control_api::RpcResponse::error(
                tiktools_control_api::RpcId::extract_from_prefix(raw),
                tiktools_control_api::ApiError::new(
                    "transport",
                    format!("WebView transport failed; dropping malformed input (invalid JSON: {error})"),
                ),
            );
            emit_rpc_response_for_generation(emitter, generation, &response);
        }
        InboundRoute::FrontendReady => {
            emitter.request_ready(generation);
        }
        InboundRoute::Legacy(_) | InboundRoute::MalformedLegacy => {
            tracing::warn!("dropping WebView IPC while the transport is failed");
        }
    }
}

#[derive(serde::Serialize)]
struct RpcEnvelope<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    response: &'a tiktools_control_api::RpcResponse,
}

/// Sends one RPC response to the frontend through the host-message batch
/// path (reliable lane: never shed).
pub(crate) fn emit_rpc_response(
    emitter: &crate::event::EventLoopEmitter,
    response: &tiktools_control_api::RpcResponse,
) {
    let payload = RpcEnvelope {
        kind: "rpc-response",
        response,
    };
    match super::outbox::encode_bounded(&payload) {
        Ok(payload) => emitter.send(payload),
        Err(error) => {
            tracing::error!(%error, "RPC response exceeds transport limit");
            emitter.fail();
        }
    }
}

fn emit_rpc_response_for_generation(
    emitter: &crate::event::EventLoopEmitter,
    generation: u64,
    response: &tiktools_control_api::RpcResponse,
) {
    let payload = RpcEnvelope {
        kind: "rpc-response",
        response,
    };
    match super::outbox::encode_bounded(&payload) {
        Ok(payload) => emitter.send_generation(payload, Some(generation)),
        Err(_) if emitter.generation.load(Ordering::Acquire) == generation => emitter.fail(),
        Err(_) => {}
    }
}

impl DesktopApp {
    pub(crate) fn flush_host_messages(&mut self) {
        self.flush_webview_batch();
    }

    /// Two-lane enqueue: RPC responses and critical transitions land in
    /// the reliable lane (never dropped); snapshots coalesce and feed is
    /// bounded in the lossy lane. A tripped reliable safety policy fails
    /// the transport loudly instead of growing memory without bound.
    pub(crate) fn emit_to_webview(&mut self, message: super::outbox::QueuedWebviewMessage) {
        // Plugin-scoped domain events fan out to subscribed plugin
        // windows; anything else is ignored by the forwarder.
        self.plugin_ui.forward_prepared(&message);
        self.pending_host_messages.push_prepared(message);
        if self.pending_host_messages.take_transport_failure() {
            self.fail_webview_transport("reliable admission exhausted");
        }
    }

    /// Fails the WebView transport loudly after the reliable outbox
    /// exceeded its safety policy: health degrades, stale queued messages
    /// are cleared for the rebooting page, new RPC is rejected, and the
    /// WebView reloads. The latch releases when the fresh page completes
    /// its handshake (see `frontend_ready`); without that handshake the
    /// transport stays failed rather than delivering to a dead page.
    pub(crate) fn fail_webview_transport(&mut self, reason: &str) {
        self.page_ready = false;
        self.pending_batch = None;
        self.emitter.clear_batch();
        self.webview_failed.store(true, Ordering::SeqCst);
        self.emitter.advance_generation();
        self.core.set_webview_error(Some(format!(
            "WebView transport failed: {reason}; reloading"
        )));
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
        if !self.page_ready || self.pending_batch.is_some() || self.shutting_down {
            return;
        }
        let Some(webview) = self.webview.as_ref() else {
            return;
        };
        if self.pending_host_messages.is_empty() {
            return;
        }
        let batch = self.pending_host_messages.take_prepared_batch();
        let literals: Option<Vec<&str>> = batch
            .iter()
            .map(|message| message.javascript_literal.as_deref())
            .collect();
        let Some(literals) = literals else {
            self.fail_webview_transport("missing prepared batch literal");
            return;
        };
        let argument = format!("[{}]", literals.join(","));
        tracing::debug!(
            count = batch.len(),
            bytes = argument.len(),
            "delivering host message batch to WebView"
        );
        let generation = self.emitter.generation.load(Ordering::Acquire);
        let sequence = self.emitter.batch_sequence.fetch_add(1, Ordering::AcqRel) + 1;
        let script = format!(
            "(() => {{ const batch = {argument}; if (typeof window.__tiktools_receive_batch__ === 'function') {{ window.__tiktools_receive_batch__(batch, {generation}); }} else if (typeof window.__webview_on_message__ === 'function') {{ for (const item of batch) window.__webview_on_message__(item); }} else {{ return; }} window.ipc.postMessage('tiktools-batch-ack:{generation}:{sequence}'); }})();"
        );
        self.emitter
            .batch_messages
            .store(batch.len() as u64, Ordering::Relaxed);
        self.emitter.batch_bytes.store(
            batch
                .iter()
                .map(|message| message.body.len())
                .sum::<usize>() as u64,
            Ordering::Relaxed,
        );
        self.pending_batch = Some(PendingBatch {
            generation,
            sequence,
            messages: batch,
            deadline: Instant::now() + Duration::from_secs(5),
        });
        self.emitter.evaluations.fetch_add(1, Ordering::Relaxed);
        self.emitter
            .evaluation_bytes
            .fetch_add(script.len() as u64, Ordering::Relaxed);
        if let Err(error) = webview.evaluate_script(&script) {
            self.emitter
                .evaluation_errors
                .fetch_add(1, Ordering::Relaxed);
            if !self.shutting_down {
                tracing::error!(%error, "host batch evaluation failed; retiring transport and resyncing");
                self.core.record_event_gap();
                self.fail_webview_transport("script evaluation failed");
            }
        }
    }
}
