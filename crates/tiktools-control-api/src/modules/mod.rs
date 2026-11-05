pub mod analytics;
pub mod app;
pub mod automation;
#[cfg(feature = "cloud-auth")]
pub mod cloud;
pub mod competitions;
pub mod creators;
pub mod gifts;
pub mod globals;
pub mod live;
pub mod media;
#[cfg(feature = "plugin-install")]
pub mod packages;
pub mod plugins;
pub mod points;
pub mod processors;
#[cfg(all(feature = "cloud-auth", feature = "http", feature = "plugin-install"))]
pub mod publisher;
pub mod rankings;
#[cfg(feature = "plugin-install")]
pub mod registry;
pub mod rpc;
pub mod settings;
#[cfg(feature = "cloud-auth")]
pub mod sync;
pub mod system;
pub mod ui;
pub mod widgets;
pub mod workflows;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Empty `{}` params shared by read-only methods.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Empty {}

/// `{ ok: true }` result shared by fire-and-forget mutations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OkResult {
    pub ok: bool,
}

impl OkResult {
    pub fn ok() -> Self {
        Self { ok: true }
    }
}

/// Runs synchronous SQLite/filesystem work on the blocking pool so RPC
/// handlers never stall Tokio workers. Pure in-memory reads must NOT use
/// this; the pool hop would only add latency.
pub(crate) async fn blocking_task<T, F>(
    what: &'static str,
    task: F,
) -> Result<T, super::error::ApiError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|error| super::error::ApiError::internal(format!("{what} worker failed: {error}")))
}
