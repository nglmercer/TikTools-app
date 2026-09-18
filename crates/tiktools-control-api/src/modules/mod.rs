pub mod analytics;
pub mod app;
pub mod automation;
pub mod creators;
pub mod gifts;
pub mod live;
pub mod media;
pub mod plugins;
pub mod points;
pub mod processors;
pub mod rpc;
pub mod settings;
pub mod system;
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
