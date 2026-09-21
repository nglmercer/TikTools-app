//! Plugin settings, connection checks, and token provisioning.

use super::{clean_plugin_id, OperationError};
use crate::events::DomainEvent;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingsResult {
    pub plugin_id: String,
    pub schema: Value,
    pub ui_hints: Option<Value>,
    pub values: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginConnectionResult {
    pub plugin_id: String,
    pub ok: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

// ------------------------------------------------------------------
// WebView parity: app state, creators, analytics, gifts, workflow
// lookup, automation nodes/scripts, token provisioning, media picker.
// ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginProvisionResult {
    pub plugin_id: String,
    pub ok: bool,
    pub error: Option<String>,
}

impl AppCore {
    // ------------------------------------------------------------------
    // Plugin settings (secrets stay redacted outside execution paths).
    // ------------------------------------------------------------------

    pub fn plugin_settings(&self, id: &str) -> Result<PluginSettingsResult, OperationError> {
        let plugin = self.require_discovered(id)?;
        let schema = plugin.manifest.settings_schema.clone().ok_or_else(|| {
            OperationError::invalid(format!(
                "Plugin settings are not declared by `{}`.",
                plugin.manifest.id
            ))
        })?;
        let values = self
            .capabilities
            .load_plugin_settings_for_display(&plugin.manifest)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        Ok(PluginSettingsResult {
            plugin_id: plugin.manifest.id.clone(),
            schema,
            ui_hints: plugin.manifest.settings_ui_hints.clone(),
            values: values
                .as_object()
                .cloned()
                .map(|object| object.into_iter().collect())
                .unwrap_or_default(),
        })
    }

    pub fn plugin_settings_save(
        &self,
        id: &str,
        values: BTreeMap<String, Value>,
    ) -> Result<PluginSettingsResult, OperationError> {
        let plugin = self.require_discovered(id)?;
        if plugin.manifest.settings_schema.is_none() {
            return Err(OperationError::invalid(format!(
                "Plugin settings are not declared by `{}`.",
                plugin.manifest.id
            )));
        }
        if values.len() > 256 {
            return Err(OperationError::invalid(
                "too many settings values (max 256)",
            ));
        }
        let stored = self
            .capabilities
            .save_plugin_settings(&plugin.manifest, &values)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        self.bump_processor_settings_revision(&plugin.manifest.id);
        self.option_sources.clear();
        self.events
            .publish_domain(DomainEvent::PluginSettingsChanged {
                plugin_id: plugin.manifest.id.clone(),
            });
        Ok(PluginSettingsResult {
            plugin_id: plugin.manifest.id.clone(),
            schema: plugin
                .manifest
                .settings_schema
                .clone()
                .unwrap_or(Value::Null),
            ui_hints: plugin.manifest.settings_ui_hints.clone(),
            values: stored
                .as_object()
                .cloned()
                .map(|object| object.into_iter().collect())
                .unwrap_or_default(),
        })
    }

    /// Deletes stored settings so schema defaults apply again. Secrets are
    /// dropped with the file; the result echoes redacted defaults.
    pub fn plugin_settings_reset(&self, id: &str) -> Result<PluginSettingsResult, OperationError> {
        let plugin = self.require_discovered(id)?;
        if plugin.manifest.settings_schema.is_none() {
            return Err(OperationError::invalid(format!(
                "Plugin settings are not declared by `{}`.",
                plugin.manifest.id
            )));
        }
        let directory = self
            .capabilities
            .ensure_plugin_data_dir(&plugin.manifest)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let path = directory.join("settings.json");
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(OperationError::internal(format!(
                    "could not reset plugin settings: {error}"
                )))
            }
        }
        self.bump_processor_settings_revision(&plugin.manifest.id);
        self.option_sources.clear();
        self.events
            .publish_domain(DomainEvent::PluginSettingsChanged {
                plugin_id: plugin.manifest.id.clone(),
            });
        self.plugin_settings(&plugin.manifest.id)
    }

    pub async fn plugin_connection_check(
        self: &Arc<Self>,
        id: &str,
    ) -> Result<PluginConnectionResult, OperationError> {
        let id = clean_plugin_id(id)?;
        if self.plugins.get(&id).is_none() {
            return Err(OperationError::not_found(format!(
                "plugin `{id}` is not installed"
            )));
        }
        let check = self.check_plugin_connection(&id).await;
        Ok(PluginConnectionResult {
            plugin_id: id,
            ok: check.ok,
            latency_ms: check.latency_ms,
            error: check.error,
        })
    }

    /// One-click token provisioning. Operational failures (bad credentials,
    /// unreachable server, unsupported flow) return `ok: false` with a
    /// message, mirroring plugin action execution; only malformed input and
    /// unknown plugins are hard errors. The password never appears in the
    /// result, logs, or events.
    pub async fn plugin_token_provision(
        self: &Arc<Self>,
        id: &str,
        username: &str,
        password: &str,
    ) -> Result<PluginProvisionResult, OperationError> {
        let id = clean_plugin_id(id)?;
        if username.trim().is_empty() || username.len() > 128 {
            return Err(OperationError::invalid(
                "username must be 1..=128 characters",
            ));
        }
        if password.is_empty() || password.len() > 4096 {
            return Err(OperationError::invalid(
                "password must be 1..=4096 characters",
            ));
        }
        if self.plugins.get(&id).is_none() {
            return Err(OperationError::not_found(format!(
                "Plugin `{id}` is not installed."
            )));
        }
        match self
            .provision_plugin_token_inner(&id, username, password)
            .await
        {
            Ok(()) => Ok(PluginProvisionResult {
                plugin_id: id,
                ok: true,
                error: None,
            }),
            Err(message) => Ok(PluginProvisionResult {
                plugin_id: id,
                ok: false,
                error: Some(message),
            }),
        }
    }
}
