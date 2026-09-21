//! Parsed plugin action descriptors and manifest field queries.

use crate::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PluginActionDescriptor {
    pub(crate) id: String,
    #[serde(default, rename = "requiredCapabilities")]
    pub(crate) required_capabilities: Vec<String>,
    #[serde(default, rename = "timeoutMs")]
    pub(crate) timeout_ms: Option<u64>,
    /// Declarative HTTP block (schema v3 only). Present means the host
    /// executes this action itself instead of calling into the plugin.
    #[serde(default)]
    pub(crate) http: Option<Value>,
}

impl PluginActionDescriptor {
    pub(crate) fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(
            self.timeout_ms
                .unwrap_or(tiktools_plugin_api::manifest::DEFAULT_PLUGIN_ACTION_TIMEOUT_MS),
        )
    }
}

/// True when the named action descriptor in this manifest declares a config
/// field with this key. Pure over the manifest so the immediate-execution
/// text rule stays unit-testable without a plugin registry.
pub(crate) fn manifest_action_declares_field(
    manifest: &tiktools_plugin_api::PluginManifest,
    type_id: &str,
    key: &str,
) -> bool {
    manifest.action_types.iter().any(|descriptor| {
        descriptor.get("id").and_then(Value::as_str) == Some(type_id)
            && descriptor
                .get("fields")
                .and_then(Value::as_array)
                .is_some_and(|fields| {
                    fields
                        .iter()
                        .any(|field| field.get("key").and_then(Value::as_str) == Some(key))
                })
    })
}
