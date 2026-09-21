//! Headless control plane for TikTools.
//!
//! Every client speaks the same JSON-RPC-style protocol and every method
//! funnels into one [`AppCore`](tiktools_core::AppCore) operation:
//!
//! ```text
//! client -> ControlApi -> AppCore service
//! ```
//!
//! Clients: CLI, JSON stdio (`host --stdio`), local IPC (Unix socket /
//! Windows named pipe), WebView bridge, tests, AI agents.

pub mod client;
pub mod error;
pub mod modules;
pub mod ownership;
pub mod registry;
pub mod request;
pub mod response;
pub mod router;
pub mod transport;

pub use client::{ClientError, ControlClient, ControlEvent};
pub use error::ApiError;
pub use registry::MethodMeta;
pub use request::{RpcId, RpcRequest};
pub use response::RpcResponse;
pub use router::ControlRouter;
pub use transport::{
    run_ipc, run_ipc_shared, run_ipc_shared_with_ready, run_stdio, IPC_NAME, MAX_PARAMS_BYTES,
    MAX_REQUEST_BYTES,
};

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tiktools_core::AppCore;

use crate::registry::MethodRegistry;

/// Per-request execution budget. Plugin actions can legitimately take up to
/// two minutes (manifest `timeoutMs`), so the RPC budget sits above that.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(150);

/// Thin adapter over [`AppCore`]. Business logic stays in core services;
/// handlers only validate typed params and map results/errors.
pub struct ControlApi {
    core: Arc<AppCore>,
    router: ControlRouter,
}

impl ControlApi {
    pub fn new(core: Arc<AppCore>) -> Self {
        let mut router = ControlRouter::new();
        Self::register_all(&mut router);
        Self { core, router }
    }

    /// Registers every domain module plus `rpc.*` discovery into `router`.
    /// This is the single source of truth for the control surface: `new`
    /// builds hosts from it and out-of-tree parity tests enumerate it.
    pub fn register_all(router: &mut ControlRouter) {
        modules::system::register(router);
        modules::plugins::register(router);
        modules::settings::register(router);
        modules::live::register(router);
        modules::points::register(router);
        modules::automation::register(router);
        modules::processors::register(router);
        modules::media::register(router);
        modules::app::register(router);
        modules::creators::register(router);
        modules::analytics::register(router);
        modules::gifts::register(router);
        modules::workflows::register(router);
        // Agent-facing risk metadata: destructive ops delete or reset
        // persisted state, so agents should confirm before calling them.
        for name in [
            "plugins.uninstall",
            "plugins.settings.reset",
            "automation.delete",
            "workflows.delete",
            "points.reset",
            "creators.history.clear",
            "system.shutdown",
        ] {
            router.set_flags(name, true, false);
        }
        let registry = std::sync::Arc::new(std::sync::RwLock::new(MethodRegistry::default()));
        modules::rpc::register(router, std::sync::Arc::clone(&registry));
        // Snapshot after every module (including rpc.*) registered so
        // discovery lists the discovery methods themselves.
        *registry.write().expect("method registry poisoned") = MethodRegistry::snapshot(router);
    }

    pub fn core(&self) -> &Arc<AppCore> {
        &self.core
    }

    pub fn router(&self) -> &ControlRouter {
        &self.router
    }

    /// Executes one validated request.
    pub async fn execute(&self, request: RpcRequest) -> RpcResponse {
        if let Some(version) = request.jsonrpc.as_deref() {
            if version != "2.0" {
                return RpcResponse::error(
                    request.id,
                    ApiError::invalid_params("jsonrpc must be \"2.0\""),
                );
            }
        }
        let Some(entry) = self.router.get(&request.method) else {
            return RpcResponse::error(request.id, ApiError::method_not_found(&request.method));
        };
        if request.params_size() > MAX_PARAMS_BYTES {
            return RpcResponse::error(request.id, ApiError::too_large());
        }
        let params = request.params_object();
        let outcome =
            tokio::time::timeout(REQUEST_TIMEOUT, (entry.handler)(self.core.clone(), params)).await;
        match outcome {
            Ok(Ok(result)) => RpcResponse::success(request.id, result),
            Ok(Err(error)) => RpcResponse::error(request.id, error),
            Err(_) => RpcResponse::error(request.id, ApiError::timeout()),
        }
    }

    /// Parses one raw JSON value (a stdio/IPC line) and executes it.
    pub async fn execute_value(&self, raw: &Value) -> RpcResponse {
        let id = RpcId::extract(raw);
        let Some(object) = raw.as_object() else {
            return RpcResponse::error(id, ApiError::invalid_params("request must be an object"));
        };
        if let Some(version) = object.get("jsonrpc").and_then(Value::as_str) {
            if version != "2.0" {
                return RpcResponse::error(id, ApiError::invalid_params("jsonrpc must be \"2.0\""));
            }
        }
        let method = object
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if method.is_empty() || method.len() > 128 {
            return RpcResponse::error(
                id,
                ApiError::invalid_params("method must be 1..=128 characters"),
            );
        }
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        self.execute(RpcRequest {
            jsonrpc: None,
            id,
            method: method.to_owned(),
            params,
        })
        .await
    }

    /// Subscribes to the domain event bus (reliable + lossy lanes,
    /// reliable drained first). Each event maps to one JSON-RPC
    /// notification via [`event_notification`].
    pub fn subscribe(&self) -> tiktools_core::events::DomainSubscription {
        self.core.events.subscribe_domain()
    }

    /// Subscribes to the domain event bus through the shared
    /// [`ControlEvent`] broadcast shape, so direct (in-process) and
    /// connected (IPC) subscribers consume one stream type.
    pub fn subscribe_broadcast(&self) -> tokio::sync::broadcast::Receiver<ControlEvent> {
        crate::client::bridge_subscription(self.subscribe())
    }
}

/// Maps one domain event onto the JSON-RPC notification envelope:
/// `{ jsonrpc, method: "event", params: { topic, data } }`.
pub fn event_notification(event: &tiktools_core::events::DomainEvent) -> Value {
    // The notification params carry the event's own `{topic, data}` serde
    // form (unit variants serialize without a `data` key); only a
    // conversion failure substitutes the explicit error marker below.
    match serde_json::to_value(event) {
        Ok(params) => serde_json::json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": params,
        }),
        Err(error) => {
            tracing::error!(topic = event.topic(), %error, "domain event failed envelope conversion");
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "event",
                "params": {
                    "topic": event.topic(),
                    "data": { "error": "event_serialization_failed" },
                },
            })
        }
    }
}

/// Maps a reliable-lane lag onto the JSON-RPC gap notification envelope:
/// `{ jsonrpc, method: "event.gap", params: { lost, resync: true } }`.
/// `lost` counts the skipped authoritative events. The stream is no
/// longer complete: the client must refresh authoritative state instead
/// of reconstructing the missing events.
pub fn gap_notification(lost: u64) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "event.gap",
        "params": {
            "lost": lost,
            "resync": true,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_and_automation_topics_map_to_event_notifications() {
        // The stdio/IPC/WebView transports all send `event_notification`
        // verbatim, so this envelope is what an IPC subscriber receives.
        for event in [
            tiktools_core::events::DomainEvent::PluginEvent {
                plugin_id: "hotkeys".to_owned(),
                event_type: "hotkey.pressed".to_owned(),
                event: serde_json::json!({"type": "hotkey.pressed"}),
            },
            tiktools_core::events::DomainEvent::PluginStatus {
                plugin_id: "hotkeys".to_owned(),
                status: serde_json::json!({}),
            },
            tiktools_core::events::DomainEvent::AutomationRunsChanged { runs: vec![] },
            tiktools_core::events::DomainEvent::AutomationRunCompleted {
                run: serde_json::json!({}),
            },
        ] {
            let notification = event_notification(&event);
            assert_eq!(notification["method"], "event");
            assert_eq!(notification["params"]["topic"], event.topic());
        }
        let notification = event_notification(&tiktools_core::events::DomainEvent::PluginEvent {
            plugin_id: "hotkeys".to_owned(),
            event_type: "hotkey.pressed".to_owned(),
            event: serde_json::json!({"type": "hotkey.pressed"}),
        });
        assert_eq!(notification["params"]["data"]["pluginId"], "hotkeys");
        assert_eq!(
            notification["params"]["data"]["eventType"],
            "hotkey.pressed"
        );
    }
}
