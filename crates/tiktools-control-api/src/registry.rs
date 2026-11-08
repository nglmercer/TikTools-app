use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ConcurrencyStrategy {
    CompareAndSet,
    Additive,
    ExclusiveTransition,
}

/// Discoverable method metadata (see `rpc.discover` / `rpc.schema`).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MethodMeta {
    pub name: String,
    pub description: String,
    pub side_effect: bool,
    /// Deletes or resets persisted state (agents should confirm first).
    #[serde(default)]
    pub destructive: bool,
    /// Needs a desktop host (native dialogs); headless hosts refuse it.
    #[serde(default)]
    pub requires_desktop: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<ConcurrencyStrategy>,
    #[serde(default)]
    pub durable_commands: bool,
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
