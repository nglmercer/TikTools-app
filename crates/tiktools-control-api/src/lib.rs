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

pub use client::{ClientError, ControlClient};
pub use error::ApiError;
pub use registry::MethodMeta;
pub use request::{RpcId, RpcRequest};
pub use response::RpcResponse;
pub use router::ControlRouter;
pub use transport::{
    run_ipc, run_ipc_shared, run_stdio, IPC_NAME, MAX_PARAMS_BYTES, MAX_REQUEST_BYTES,
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
        modules::system::register(&mut router);
        modules::plugins::register(&mut router);
        modules::settings::register(&mut router);
        modules::live::register(&mut router);
        modules::points::register(&mut router);
        modules::automation::register(&mut router);
        modules::processors::register(&mut router);
        modules::media::register(&mut router);
        modules::app::register(&mut router);
        modules::creators::register(&mut router);
        modules::analytics::register(&mut router);
        modules::gifts::register(&mut router);
        modules::workflows::register(&mut router);
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
        modules::rpc::register(&mut router, std::sync::Arc::clone(&registry));
        // Snapshot after every module (including rpc.*) registered so
        // discovery lists the discovery methods themselves.
        *registry.write().expect("method registry poisoned") = MethodRegistry::snapshot(&router);
        Self { core, router }
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

    /// Subscribes to the domain event bus. Each event maps to one
    /// JSON-RPC notification via [`event_notification`].
    pub fn subscribe(
        &self,
    ) -> tokio::sync::broadcast::Receiver<tiktools_core::events::DomainEvent> {
        self.core.events.subscribe_domain()
    }
}

/// Maps one domain event onto the JSON-RPC notification envelope:
/// `{ jsonrpc, method: "event", params: { topic, data } }`.
pub fn event_notification(event: &tiktools_core::events::DomainEvent) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "event",
        "params": serde_json::to_value(event).unwrap_or(Value::Null),
    })
}
