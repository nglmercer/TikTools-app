use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Discoverable method metadata (see `rpc.discover` / `rpc.schema`).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MethodMeta {
    pub name: String,
    pub description: String,
    pub side_effect: bool,
    pub params_schema: Value,
    pub result_schema: Value,
}

/// Immutable snapshot of the registered methods, captured for the
/// `rpc.*` handlers after all domain modules registered.
#[derive(Debug, Clone, Default)]
pub struct MethodRegistry {
    pub methods: Vec<MethodMeta>,
}

impl MethodRegistry {
    pub fn snapshot(router: &crate::router::ControlRouter) -> Self {
        Self {
            methods: router.metadata(),
        }
    }

    pub fn get(&self, method: &str) -> Option<&MethodMeta> {
        self.methods.iter().find(|meta| meta.name == method)
    }
}
