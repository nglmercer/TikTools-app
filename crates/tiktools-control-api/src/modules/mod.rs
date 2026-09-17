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
