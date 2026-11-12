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

mod execution;
mod notifications;
mod registration;
mod remote;
pub use notifications::{event_notification, gap_notification};

pub mod client;
pub(crate) mod command_metadata;
pub mod context;
pub mod error;
pub mod metadata;
pub mod modules;
pub mod ownership;
pub mod policy;
pub mod registry;
pub mod request;
pub mod response;
pub mod router;
pub mod transport;

pub use client::{ClientError, ControlClient, ControlEvent};
pub use context::{RemoteCapability, RemoteSession, RequestContext};
pub use error::ApiError;
pub use metadata::{CallResult, RequestMetadata, ResponseMetadata};
pub use registry::MethodMeta;
pub use request::{RpcId, RpcRequest};
pub use response::RpcResponse;
pub use router::ControlRouter;
pub use transport::{
    owner_params_bytes, owner_request_bytes, run_ipc, run_ipc_shared, run_ipc_shared_with_ready,
    run_stdio, run_stdio_shared, IPC_NAME, MAX_OWNER_LIBRARY_PARAMS_BYTES,
    MAX_OWNER_LIBRARY_REQUEST_BYTES, MAX_OWNER_RESPONSE_BYTES, MAX_PARAMS_BYTES, MAX_REQUEST_BYTES,
};

use std::sync::Arc;
use std::time::Duration;

use tiktools_core::AppCore;

/// Per-request execution budget. Plugin actions can legitimately take up to
/// two minutes (manifest `timeoutMs`), so the RPC budget sits above that.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(150);

/// Login completion waits up to 180 seconds for the browser callback before
/// redeeming it; other handlers keep the standard budget.
pub fn request_timeout_for(method: &str) -> Duration {
    if method == "cloud.login.complete" {
        Duration::from_secs(240)
    } else {
        REQUEST_TIMEOUT
    }
}

/// The local IPC client must wait past the server's login deadline so the
/// authoritative timeout response can arrive. Other methods are unchanged.
pub fn client_timeout_for(method: &str) -> Duration {
    if method == "cloud.login.complete" {
        Duration::from_secs(270)
    } else {
        REQUEST_TIMEOUT
    }
}

/// Thin adapter over [`AppCore`]. Business logic stays in core services;
/// handlers only validate typed params and map results/errors.
pub struct ControlApi {
    core: Arc<AppCore>,
    router: ControlRouter,
}

impl ControlApi {
    pub fn new(core: Arc<AppCore>) -> Self {
        core.start_competition_timer();
        core.start_coordination_worker();
        #[cfg(feature = "cloud-auth")]
        core.start_cloud_sync_worker();
        let mut router = ControlRouter::new();
        Self::register_all(&mut router);
        Self { core, router }
    }

    pub fn core(&self) -> &Arc<AppCore> {
        &self.core
    }

    pub fn router(&self) -> &ControlRouter {
        &self.router
    }
}

#[cfg(test)]
mod tests;
