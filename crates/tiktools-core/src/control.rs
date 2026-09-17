//! Headless operations API: the single authoritative operation path.
//!
//! Every client (WebView IPC, CLI, JSON stdio, local IPC, tests, agents) is
//! expected to go through these value-returning operations. Unlike the
//! `PageMessage` handlers, these methods never emit [`HostMessage`]s; the
//! WebView layer emits UI messages after calling the same operations.
//!
//! [`HostMessage`]: crate::ipc::messages::HostMessage

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::*;
use crate::events::DomainEvent;
use crate::ipc::messages::{PartialPointsConfig, PointsConfig};
use crate::services::{media_file_ref, PointAward};
use tiktools_plugin_api::{AudioPlayOptions, MediaKind, MediaSelection};

/// Machine-readable operation failure. The control API maps this 1:1 onto
/// the JSON-RPC error envelope (`code` + `message`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationError {
    pub code: &'static str,
    pub message: String,
}

impl OperationError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found",
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_params",
            message: message.into(),
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "unavailable",
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "conflict",
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            code: "timeout",
            message: message.into(),
        }
    }

    /// Desktop-only capability (native dialogs, audio output) requested on a
    /// headless host. Callers must not emulate the capability.
    pub fn capability_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "capability_unavailable",
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal",
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for OperationError {}

impl From<tiktools_plugin_loader::PluginLoaderError> for OperationError {
    fn from(error: tiktools_plugin_loader::PluginLoaderError) -> Self {
        match &error {
            tiktools_plugin_loader::PluginLoaderError::NotFound(_) => {
                Self::not_found(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::RuntimeUnavailable(_) => {
                Self::unavailable(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::Timeout(_) => {
                Self::timeout(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::Manifest(_)
            | tiktools_plugin_loader::PluginLoaderError::InvalidDirectory(_)
            | tiktools_plugin_loader::PluginLoaderError::Runtime(_) => {
                Self::internal(error.to_string())
            }
        }
    }
}

/// Behavior record family. The storage table is a closed allowlist: table
/// names are never derived from caller input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AutomationKind {
    Event,
    Action,
}

impl AutomationKind {
    fn table(self) -> &'static str {
        match self {
            Self::Event => "behavior_events",
            Self::Action => "behavior_actions",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Action => "action",
        }
    }
}

impl std::str::FromStr for AutomationKind {
    type Err = OperationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "event" | "events" => Ok(Self::Event),
            "action" | "actions" => Ok(Self::Action),
            other => Err(OperationError::invalid(format!(
                "unknown automation kind `{other}` (expected `event` or `action`)"
            ))),
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionOutcome {
    pub action_type: String,
    pub ok: bool,
    pub summary: String,
    pub logs: Vec<String>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorOutcomeDto {
    pub plugin_id: String,
    pub processor_id: String,
    pub ok: bool,
    pub duration_ms: u64,
    pub result: Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveStatus {
    pub connected: bool,
    pub unique_id: Option<String>,
    pub room_id: Option<String>,
    pub connection_id: Option<String>,
    pub native: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoctorCheck {
    pub id: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallResult {
    pub id: String,
    pub version: String,
    pub directory: String,
    pub replaced: bool,
}

fn clean_record_id(value: &str, what: &str) -> Result<String, OperationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return Err(OperationError::invalid(format!(
            "{what} id must be 1..=128 characters"
        )));
    }
    Ok(trimmed.to_owned())
}

fn clean_plugin_id(value: &str) -> Result<String, OperationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 128 || !is_identifier(trimmed) {
        return Err(OperationError::invalid(format!(
            "plugin id `{value}` is not a valid identifier"
        )));
    }
    Ok(trimmed.to_owned())
}

fn fresh_record_id(prefix: &str) -> String {
    format!("{prefix}-{}-{:08x}", now_millis(), fastrand::u32(..))
}

impl AppCore {
    /// Returns whether [`AppCore::shutdown`] has started.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_started.load(Ordering::Acquire)
    }

    /// Records a control IPC failure so `system.health` reports degraded
    /// instead of silently losing CLI/agent connectivity while the GUI
    /// looks healthy. `None` clears the degraded state after a retry.
    pub fn set_ipc_error(&self, message: Option<String>) {
        *self.ipc_error.write().expect("ipc error lock poisoned") = message;
    }

    pub fn ipc_error(&self) -> Option<String> {
        self.ipc_error
            .read()
            .expect("ipc error lock poisoned")
            .clone()
    }

    /// Loads the merged behavior snapshot (persisted records plus the live
    /// runtime catalog) and refreshes the in-memory automation projection.
    /// This is the value-returning twin of the `get-behavior` emit path.
    pub fn behavior_snapshot(&self) -> Value {
        let snapshot = self.load_merged_behavior_snapshot();
        self.automation.replace_snapshot(&snapshot);
        self.request_hotkey_sync();
        snapshot
    }

    fn load_merged_behavior_snapshot(&self) -> Value {
        #[cfg(feature = "persistence")]
        let mut snapshot = self.db.load_behavior_snapshot().unwrap_or_else(|error| {
            tracing::warn!(%error, "could not load behavior snapshot");
            empty_behavior_snapshot()
        });
        #[cfg(not(feature = "persistence"))]
        let mut snapshot = empty_behavior_snapshot();
        self.merge_runtime_catalog(&mut snapshot);
        snapshot
    }

    fn refresh_automation_snapshot(&self) {
        let snapshot = self.load_merged_behavior_snapshot();
        self.automation.replace_snapshot(&snapshot);
        self.request_hotkey_sync();
    }

    fn behavior_records(&self, kind: AutomationKind) -> Vec<Value> {
        let snapshot = self.load_merged_behavior_snapshot();
        let key = match kind {
            AutomationKind::Event => "events",
            AutomationKind::Action => "actions",
        };
        snapshot
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    pub fn automation_list(&self, kind: AutomationKind) -> Vec<Value> {
        self.behavior_records(kind)
    }

    pub fn automation_get(&self, kind: AutomationKind, id: &str) -> Result<Value, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        self.behavior_records(kind)
            .into_iter()
            .find(|record| record.get("id").and_then(Value::as_str) == Some(id.as_str()))
            .ok_or_else(|| {
                OperationError::not_found(format!("{} `{id}` does not exist", kind.label()))
            })
    }

    /// Creates a behavior record, generating an id when the caller omits one.
    pub fn automation_create(
        &self,
        kind: AutomationKind,
        mut record: Value,
    ) -> Result<Value, OperationError> {
        let object = record
            .as_object_mut()
            .ok_or_else(|| OperationError::invalid("automation record must be an object"))?;
        if object
            .get("id")
            .and_then(Value::as_str)
            .is_none_or(|id| id.trim().is_empty())
        {
            object.insert(
                "id".to_owned(),
                Value::String(fresh_record_id(kind.label())),
            );
        }
        if object
            .get("name")
            .and_then(Value::as_str)
            .is_none_or(|name| name.trim().is_empty())
        {
            let fallback = match kind {
                AutomationKind::Event => "Untitled event",
                AutomationKind::Action => "Untitled action",
            };
            object.insert("name".to_owned(), Value::String(fallback.to_owned()));
        }
        let _ = object.get("id");
        let saved = self.save_automation_record(kind, &record)?;
        self.events.publish_domain(DomainEvent::WorkflowChanged {
            kind: kind.label().to_owned(),
            id: saved
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            change: "created".to_owned(),
        });
        Ok(saved)
    }

    /// Replaces a behavior record. The record id must already exist.
    pub fn automation_update(
        &self,
        kind: AutomationKind,
        id: &str,
        record: Value,
    ) -> Result<Value, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        let mut object = record
            .as_object()
            .cloned()
            .ok_or_else(|| OperationError::invalid("automation record must be an object"))?;
        object.insert("id".to_owned(), Value::String(id.clone()));
        let record = Value::Object(object);
        // Update (not upsert): refuse to resurrect deleted records silently.
        self.automation_get(kind, &id)?;
        let saved = self.save_automation_record(kind, &record)?;
        self.events.publish_domain(DomainEvent::WorkflowChanged {
            kind: kind.label().to_owned(),
            id,
            change: "updated".to_owned(),
        });
        Ok(saved)
    }

    pub fn automation_delete(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> Result<bool, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        let removed = self.delete_automation_record(kind, &id)?;
        if removed {
            self.events.publish_domain(DomainEvent::WorkflowChanged {
                kind: kind.label().to_owned(),
                id,
                change: "deleted".to_owned(),
            });
        }
        Ok(removed)
    }

    pub fn automation_set_enabled(
        &self,
        kind: AutomationKind,
        id: &str,
        enabled: bool,
    ) -> Result<Value, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        let saved = self.set_automation_enabled_record(kind, &id, enabled)?;
        self.events.publish_domain(DomainEvent::WorkflowChanged {
            kind: kind.label().to_owned(),
            id,
            change: if enabled { "enabled" } else { "disabled" }.to_owned(),
        });
        Ok(saved)
    }

    fn save_automation_record(
        &self,
        kind: AutomationKind,
        value: &Value,
    ) -> Result<Value, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let saved = self
                .db
                .save_behavior(kind.table(), value)
                .map_err(|error| OperationError::internal(error.to_string()))?;
            self.refresh_automation_snapshot();
            Ok(saved)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let object = value
                .as_object()
                .ok_or_else(|| OperationError::invalid("automation record must be an object"))?;
            if object
                .get("id")
                .and_then(Value::as_str)
                .is_none_or(|id| id.trim().is_empty())
                || object
                    .get("name")
                    .and_then(Value::as_str)
                    .is_none_or(|name| name.trim().is_empty())
            {
                return Err(OperationError::invalid(
                    "automation record needs an `id` and a `name`",
                ));
            }
            match kind {
                AutomationKind::Event => self.automation.upsert_event(value.clone()),
                AutomationKind::Action => self.automation.upsert_action(value.clone()),
            }
            Ok(value.clone())
        }
    }

    fn delete_automation_record(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> Result<bool, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let removed = self
                .db
                .delete_behavior(kind.table(), id)
                .map_err(|error| OperationError::internal(error.to_string()))?;
            self.refresh_automation_snapshot();
            Ok(removed)
        }
        #[cfg(not(feature = "persistence"))]
        {
            match kind {
                AutomationKind::Event => self.automation.remove_event(id),
                AutomationKind::Action => self.automation.remove_action(id),
            }
            Ok(true)
        }
    }

    fn set_automation_enabled_record(
        &self,
        kind: AutomationKind,
        id: &str,
        enabled: bool,
    ) -> Result<Value, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let saved = self
                .db
                .set_behavior_enabled(kind.table(), id, enabled)
                .map_err(|error| match error {
                    crate::db::DatabaseError::Invalid(message) => {
                        OperationError::not_found(message)
                    }
                    other => OperationError::internal(other.to_string()),
                })?;
            self.refresh_automation_snapshot();
            Ok(saved)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let mut record = self.automation_get(kind, id)?;
            record["enabled"] = Value::Bool(enabled);
            match kind {
                AutomationKind::Event => self.automation.upsert_event(record.clone()),
                AutomationKind::Action => self.automation.upsert_action(record.clone()),
            }
            Ok(record)
        }
    }

    pub async fn test_automation_event(self: &Arc<Self>, record: &Value) -> Value {
        self.test_event(record).await
    }

    pub async fn test_automation_action(
        self: &Arc<Self>,
        action: &Value,
        trigger: Option<&str>,
    ) -> Value {
        self.test_action(action, trigger).await
    }

    pub fn automation_runs(&self) -> Vec<Value> {
        self.automation.recent_runs()
    }

    // ------------------------------------------------------------------
    // Graph workflows (the node-canvas persistence behind the UI).
    // ------------------------------------------------------------------

    pub fn workflow_list(&self) -> Result<Vec<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            self.db
                .load_workflows()
                .map_err(|error| OperationError::internal(error.to_string()))
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "graph workflows need the persistence build",
            ))
        }
    }

    pub fn workflow_save(&self, graph: Value) -> Result<Value, OperationError> {
        #[cfg(feature = "persistence")]
        {
            if !graph.is_object() {
                return Err(OperationError::invalid("workflow graph must be an object"));
            }
            let saved = self
                .db
                .save_workflow(&graph)
                .map_err(|error| OperationError::internal(error.to_string()))?;
            if let Some(id) = saved.get("id").and_then(Value::as_str) {
                self.events.publish_domain(DomainEvent::WorkflowChanged {
                    kind: "workflow".to_owned(),
                    id: id.to_owned(),
                    change: "saved".to_owned(),
                });
            }
            Ok(saved)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = graph;
            Err(OperationError::unavailable(
                "graph workflows need the persistence build",
            ))
        }
    }

    pub fn workflow_delete(&self, id: &str) -> Result<bool, OperationError> {
        let id = clean_record_id(id, "workflow")?;
        #[cfg(feature = "persistence")]
        {
            let removed = self
                .db
                .delete_workflow(&id)
                .map_err(|error| OperationError::internal(error.to_string()))?;
            if removed {
                self.events.publish_domain(DomainEvent::WorkflowChanged {
                    kind: "workflow".to_owned(),
                    id,
                    change: "deleted".to_owned(),
                });
            }
            Ok(removed)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = id;
            Err(OperationError::unavailable(
                "graph workflows need the persistence build",
            ))
        }
    }

    pub fn workflow_set_enabled(&self, id: &str, enabled: bool) -> Result<Value, OperationError> {
        let id = clean_record_id(id, "workflow")?;
        #[cfg(feature = "persistence")]
        {
            let saved =
                self.db
                    .set_workflow_enabled(&id, enabled)
                    .map_err(|error| match error {
                        crate::db::DatabaseError::Invalid(message) => {
                            OperationError::not_found(message)
                        }
                        other => OperationError::internal(other.to_string()),
                    })?;
            self.events.publish_domain(DomainEvent::WorkflowChanged {
                kind: "workflow".to_owned(),
                id,
                change: if enabled { "enabled" } else { "disabled" }.to_owned(),
            });
            Ok(saved)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (id, enabled);
            Err(OperationError::unavailable(
                "graph workflows need the persistence build",
            ))
        }
    }

    // ------------------------------------------------------------------
    // Plugins.
    // ------------------------------------------------------------------

    /// Safe plugin summaries. Never includes settings values.
    pub fn plugin_list(&self) -> Vec<Value> {
        let snapshot = self.load_merged_behavior_snapshot();
        snapshot
            .get("plugins")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    pub fn plugin_get(&self, id: &str) -> Result<Value, OperationError> {
        // Canonical existence check first so every plugin op shares one
        // not-found message.
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id;
        self.plugin_list()
            .into_iter()
            .find(|plugin| {
                plugin
                    .get("descriptor")
                    .and_then(|descriptor| descriptor.get("id"))
                    .and_then(Value::as_str)
                    == Some(id.as_str())
                    || plugin.get("id").and_then(Value::as_str) == Some(id.as_str())
            })
            .ok_or_else(|| OperationError::not_found(format!("Plugin `{id}` is not installed.")))
    }

    fn require_discovered(
        &self,
        id: &str,
    ) -> Result<tiktools_plugin_loader::DiscoveredPlugin, OperationError> {
        let id = clean_plugin_id(id)?;
        self.plugins
            .get(&id)
            .ok_or_else(|| OperationError::not_found(format!("Plugin `{id}` is not installed.")))
    }

    pub fn plugin_set_installed(&self, id: &str, installed: bool) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        #[cfg(feature = "persistence")]
        self.db
            .set_plugin_state(&id, installed, true)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        if !installed {
            // Stopping a non-running plugin is a no-op success; surface real
            // shutdown failures.
            self.plugins.stop(&id)?;
        }
        self.set_plugin_activation(&id, installed, true);
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        Ok(())
    }

    pub fn plugin_set_enabled(&self, id: &str, enabled: bool) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        #[cfg(feature = "persistence")]
        self.db
            .set_plugin_state(&id, true, enabled)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let result = if enabled {
            self.plugins.start(&id)
        } else {
            self.plugins.stop(&id)
        };
        result?;
        self.set_plugin_activation(&id, true, enabled);
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        self.events.publish_domain(if enabled {
            DomainEvent::PluginStarted { plugin_id: id }
        } else {
            DomainEvent::PluginStopped { plugin_id: id }
        });
        Ok(())
    }

    pub fn plugin_start(&self, id: &str) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.plugins.start(&id)?;
        self.events
            .publish_domain(DomainEvent::PluginStarted { plugin_id: id });
        Ok(())
    }

    pub fn plugin_stop(&self, id: &str) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.plugins.stop(&id)?;
        self.events
            .publish_domain(DomainEvent::PluginStopped { plugin_id: id });
        Ok(())
    }

    #[cfg(feature = "plugin-install")]
    pub fn plugin_install(
        &self,
        archive: &str,
        replace_existing: bool,
    ) -> Result<PluginInstallResult, OperationError> {
        let trimmed = archive.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 {
            return Err(OperationError::invalid(
                "plugin archive path must be 1..=4096 characters",
            ));
        }
        let installed = self
            .install_plugin(std::path::Path::new(trimmed), replace_existing)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        self.refresh_automation_snapshot();
        let id = installed.manifest.id.clone();
        self.events.publish_domain(DomainEvent::PluginInstalled {
            plugin_id: id.clone(),
        });
        Ok(PluginInstallResult {
            id,
            version: installed.manifest.version.clone(),
            directory: installed.directory.display().to_string(),
            replaced: replace_existing,
        })
    }

    #[cfg(not(feature = "plugin-install"))]
    pub fn plugin_install(
        &self,
        _archive: &str,
        _replace_existing: bool,
    ) -> Result<PluginInstallResult, OperationError> {
        Err(OperationError::unavailable(
            "plugin installation was disabled in this build",
        ))
    }

    #[cfg(feature = "plugin-install")]
    pub fn plugin_uninstall(&self, id: &str) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.uninstall_plugin(&id)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        self.refresh_automation_snapshot();
        self.events
            .publish_domain(DomainEvent::PluginUninstalled { plugin_id: id });
        Ok(())
    }

    #[cfg(not(feature = "plugin-install"))]
    pub fn plugin_uninstall(&self, _id: &str) -> Result<(), OperationError> {
        Err(OperationError::unavailable(
            "plugin installation was disabled in this build",
        ))
    }

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

    /// Resolves `action-type/field` option documents for action configs.
    /// Returns the items plus the server-reported selection, if any.
    pub async fn plugin_action_options(
        self: &Arc<Self>,
        source: &str,
    ) -> Result<(Vec<Value>, Option<String>), OperationError> {
        let source = source.trim();
        if source.is_empty() || source.len() > 256 {
            return Err(OperationError::invalid(
                "option source must be 1..=256 characters",
            ));
        }
        let (options, selected, error) = self.resolve_action_options(source).await;
        if let Some(error) = error {
            return Err(OperationError::internal(error));
        }
        Ok((options, selected))
    }

    /// Executes one plugin action. `live = false` is a dry run (no HTTP is
    /// sent, no side effects); `live = true` performs the real execution.
    pub async fn plugin_action_execute(
        self: &Arc<Self>,
        action_type: &str,
        config: BTreeMap<String, Value>,
        live: bool,
    ) -> Result<PluginActionOutcome, OperationError> {
        let action_type = action_type.trim();
        if action_type.is_empty() || action_type.len() > 128 || !is_identifier(action_type) {
            return Err(OperationError::invalid(
                "action type is not a valid identifier",
            ));
        }
        if config.len() > 64 {
            return Err(OperationError::invalid("too many config values (max 64)"));
        }
        // Actions declaring a `text` field (TTS speak) require bounded
        // spoken text, mirroring the WebView voice-tester path.
        if self.action_declares_field(action_type, "text") {
            let text = config
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if text.trim().is_empty() {
                return Err(OperationError::invalid(
                    "config.text must not be empty for this action type",
                ));
            }
            if text.len() > 4_096 {
                return Err(OperationError::invalid(
                    "Text is too long (4,096 character limit).",
                ));
            }
        }
        let started = now_millis();
        let action = json!({
            "typeId": action_type,
            "config": Value::Object(config.into_iter().collect()),
        });
        let event = json!({
            "id": format!("control-{}", started),
            "type": "control.execute",
            "timestamp": started,
            "data": {},
        });
        let mut logs = Vec::new();
        match self
            .execute_plugin_action(action_type, &action, &event, &mut logs, !live)
            .await
        {
            Ok(summary) => Ok(PluginActionOutcome {
                action_type: action_type.to_owned(),
                ok: true,
                summary,
                logs: logs.into_iter().take(20).collect(),
                duration_ms: now_millis().saturating_sub(started),
                error: None,
            }),
            Err(error) => Ok(PluginActionOutcome {
                action_type: action_type.to_owned(),
                ok: false,
                summary: error.clone(),
                logs: logs.into_iter().take(20).collect(),
                duration_ms: now_millis().saturating_sub(started),
                error: Some(error),
            }),
        }
    }

    // ------------------------------------------------------------------
    // Live.
    // ------------------------------------------------------------------

    pub fn live_status(&self) -> LiveStatus {
        let connected = self.live.is_connected();
        #[cfg(feature = "native-tiktok")]
        let native = true;
        #[cfg(not(feature = "native-tiktok"))]
        let native = false;
        #[cfg(feature = "persistence")]
        let context = self
            .connection_context
            .read()
            .expect("connection context lock poisoned")
            .clone();
        #[cfg(not(feature = "persistence"))]
        let context: Option<LiveContext> = self
            .connection_context
            .read()
            .expect("connection context lock poisoned")
            .clone();
        LiveStatus {
            connected,
            unique_id: context.as_ref().map(|context| context.unique_id.clone()),
            room_id: context.as_ref().map(|context| context.room_id.clone()),
            connection_id: context.map(|context| context.connection_id),
            native,
        }
    }

    #[cfg(feature = "native-tiktok")]
    pub async fn live_connect(
        self: &Arc<Self>,
        unique_id: String,
        session_cookie: String,
        room_id: Option<String>,
    ) -> Result<LiveStatus, OperationError> {
        let unique_id = clean_unique_id(&unique_id)
            .ok_or_else(|| OperationError::invalid("uniqueId must not be empty"))?;
        if session_cookie.trim().is_empty() || session_cookie.len() > 16_384 {
            return Err(OperationError::invalid(
                "sessionCookie must be 1..=16384 characters",
            ));
        }
        if room_id.as_ref().is_some_and(|room| room.len() > 64) {
            return Err(OperationError::invalid("roomId is too long (max 64)"));
        }
        self.start_live_event_pump();
        self.publish_disconnected_event().await;
        self.live.disconnect().await;
        let info = self
            .live
            .connect(ConnectRequest {
                unique_id,
                session_cookie,
                room_id,
            })
            .await
            .map_err(|error| OperationError::unavailable(error.to_string()))?;
        self.events.publish_domain(DomainEvent::LiveConnected {
            unique_id: Some(info.unique_id.clone()),
            room_id: Some(info.room_id.clone()),
        });
        self.events.publish_domain(DomainEvent::CreatorChanged {
            unique_id: Some(info.unique_id.clone()),
        });
        Ok(self.live_status())
    }

    #[cfg(not(feature = "native-tiktok"))]
    pub async fn live_connect(
        self: &Arc<Self>,
        _unique_id: String,
        _session_cookie: String,
        _room_id: Option<String>,
    ) -> Result<LiveStatus, OperationError> {
        Err(OperationError::unavailable(
            "the native TikTok client is disabled in this build",
        ))
    }

    pub async fn live_disconnect(self: &Arc<Self>) -> LiveStatus {
        self.publish_disconnected_event().await;
        self.live.disconnect().await;
        self.events.publish_domain(DomainEvent::LiveDisconnected);
        self.events
            .publish_domain(DomainEvent::CreatorChanged { unique_id: None });
        self.live_status()
    }

    // ------------------------------------------------------------------
    // Points.
    // ------------------------------------------------------------------

    pub fn points_config(&self) -> PointsConfig {
        self.points.config()
    }

    pub fn points_update_config(&self, update: PartialPointsConfig) -> PointsConfig {
        self.points.update_config(update)
    }

    pub fn points_leaderboard(&self, limit: Option<i64>) -> Vec<Value> {
        self.points.leaderboard(limit)
    }

    pub fn points_viewer(&self, unique_id: &str) -> Result<Value, OperationError> {
        let needle = unique_id.trim().trim_start_matches('@');
        if needle.is_empty() {
            return Err(OperationError::invalid("uniqueId must not be empty"));
        }
        self.points
            .leaderboard(Some(1_000))
            .into_iter()
            .find(|viewer| viewer.get("uniqueId").and_then(Value::as_str) == Some(needle))
            .ok_or_else(|| {
                OperationError::not_found(format!("viewer `{needle}` has no points record"))
            })
    }

    /// Manual adjustment, matching the WebView adjust-points path: creates
    /// the viewer record when it does not exist yet.
    pub fn points_adjust(&self, unique_id: &str, delta: f64) -> Result<PointAward, OperationError> {
        if !delta.is_finite() || delta.abs() > 1_000_000_000.0 {
            return Err(OperationError::invalid("delta must be a finite number"));
        }
        let award = self
            .points
            .award_points(
                unique_id,
                PointAction::Manual,
                AwardOptions {
                    custom_amount: Some(delta),
                    ..AwardOptions::default()
                },
            )
            .ok_or_else(|| OperationError::invalid("uniqueId must not be empty"))?;
        self.events.publish_domain(DomainEvent::PointsChanged {
            unique_id: award.unique_id.clone(),
            delta: award.delta,
            total_points: award.total_points,
            level: award.level,
        });
        Ok(award)
    }

    pub fn points_reset(&self, unique_id: Option<&str>) {
        let cleaned = unique_id
            .map(|value| value.trim().trim_start_matches('@').to_owned())
            .filter(|value| !value.is_empty());
        self.points.reset(cleaned.as_deref());
    }

    // ------------------------------------------------------------------
    // Processors.
    // ------------------------------------------------------------------

    pub async fn processor_test(
        &self,
        plugin_id: &str,
        processor_id: &str,
        event: Value,
    ) -> Result<ProcessorOutcomeDto, OperationError> {
        let plugin_id = clean_plugin_id(plugin_id)?;
        let processor_id = processor_id.trim();
        if processor_id.is_empty() || processor_id.len() > 128 {
            return Err(OperationError::invalid(
                "processor id must be 1..=128 characters",
            ));
        }
        if !event.is_object() {
            return Err(OperationError::invalid("event must be an object"));
        }
        let outcome = self.test_processor(&plugin_id, processor_id, event).await;
        Ok(ProcessorOutcomeDto {
            plugin_id,
            processor_id: processor_id.to_owned(),
            ok: outcome.ok,
            duration_ms: outcome.duration_ms,
            result: outcome.result,
            error: outcome.error,
        })
    }

    pub fn processor_status(&self) -> Value {
        self.processor_status_snapshot()
    }

    pub fn processor_list(&self) -> Vec<Value> {
        match self.processor_status_snapshot() {
            Value::Array(entries) => entries,
            snapshot => snapshot
                .get("processors")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        }
    }

    // ------------------------------------------------------------------
    // Media.
    // ------------------------------------------------------------------

    pub fn media_validate(
        &self,
        path: &str,
        kind: MediaKind,
    ) -> Result<MediaSelection, OperationError> {
        let trimmed = path.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 {
            return Err(OperationError::invalid("path must be 1..=4096 characters"));
        }
        let file = media_file_ref(std::path::Path::new(trimmed), kind)
            .map_err(|error| OperationError::invalid(error.to_string()))?;
        Ok(MediaSelection::File { file })
    }

    pub async fn media_play(
        &self,
        path: &str,
        kind: Option<MediaKind>,
        volume: Option<f32>,
    ) -> Result<AudioPlaybackResult, OperationError> {
        let trimmed = path.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 {
            return Err(OperationError::invalid("path must be 1..=4096 characters"));
        }
        let volume = volume.unwrap_or(1.0);
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(OperationError::invalid("volume must be within 0.0..=1.0"));
        }
        let file = media_file_ref(std::path::Path::new(trimmed), kind.unwrap_or_default())
            .map_err(|error| OperationError::invalid(error.to_string()))?;
        self.play_audio(
            file,
            AudioPlayOptions {
                volume,
                ..AudioPlayOptions::default()
            },
        )
        .await
        .map_err(|error| OperationError::unavailable(error.to_string()))
    }

    // ------------------------------------------------------------------
    // System: info, health, snapshot, doctor.
    // ------------------------------------------------------------------

    pub fn system_info(&self) -> Value {
        json!({
            "name": "tiktools",
            "version": env!("CARGO_PKG_VERSION"),
            "features": {
                "nativeTikTok": cfg!(feature = "native-tiktok"),
                "persistence": cfg!(feature = "persistence"),
                "pluginInstall": cfg!(feature = "plugin-install"),
                "http": cfg!(feature = "http"),
            },
            "home": self.db.paths().root.display().to_string(),
            "dataDir": self.db.paths().data.display().to_string(),
            "pid": std::process::id(),
        })
    }

    pub fn system_health(&self) -> Value {
        let plugins = self.plugins.list();
        let running = plugins.iter().filter(|plugin| plugin.running).count();
        let unavailable = plugins.iter().filter(|plugin| !plugin.available).count();
        let mut degraded: Vec<String> = Vec::new();
        if unavailable > 0 {
            degraded.push(format!("{unavailable} plugin(s) unavailable"));
        }
        #[cfg(feature = "http")]
        if let Some(message) = self.http_client_error.as_ref() {
            degraded.push(message.clone());
        }
        if let Some(message) = self.ipc_error() {
            degraded.push(message);
        }
        json!({
            "status": if degraded.is_empty() { "ok" } else { "degraded" },
            "reasons": degraded,
            "liveConnected": self.live.is_connected(),
            "plugins": {
                "total": plugins.len(),
                "running": running,
                "unavailable": unavailable,
            },
            "processors": self.processor_list().len(),
            "shutdown": self.is_shutdown(),
        })
    }

    /// Safe observable state. Never includes settings values or secrets.
    pub fn system_snapshot(&self) -> Value {
        let behavior = self.load_merged_behavior_snapshot();
        json!({
            "live": serde_json::to_value(self.live_status()).unwrap_or(Value::Null),
            "plugins": behavior.get("plugins").cloned().unwrap_or(Value::Array(vec![])),
            "processors": self.processor_status(),
            "automations": {
                "events": behavior.get("events").cloned().unwrap_or(Value::Array(vec![])),
                "actions": behavior.get("actions").cloned().unwrap_or(Value::Array(vec![])),
            },
            "pointsConfig": serde_json::to_value(self.points.config()).unwrap_or(Value::Null),
            "health": self.system_health(),
        })
    }

    pub fn system_doctor(&self) -> DoctorReport {
        let mut checks = Vec::new();
        let paths = self.db.paths();
        let writable = paths.temp.exists()
            && std::fs::write(paths.temp.join(".doctor-write-test"), b"ok")
                .and_then(|()| std::fs::remove_file(paths.temp.join(".doctor-write-test")))
                .is_ok();
        checks.push(DoctorCheck {
            id: "storage.temp-writable".to_owned(),
            status: if writable { "ok" } else { "failed" }.to_owned(),
            message: Some(paths.temp.display().to_string()),
        });
        #[cfg(feature = "persistence")]
        {
            match self.db.load_behavior_snapshot() {
                Ok(_) => checks.push(DoctorCheck {
                    id: "database.behavior".to_owned(),
                    status: "ok".to_owned(),
                    message: None,
                }),
                Err(error) => checks.push(DoctorCheck {
                    id: "database.behavior".to_owned(),
                    status: "failed".to_owned(),
                    message: Some(error.to_string()),
                }),
            }
        }
        #[cfg(not(feature = "persistence"))]
        checks.push(DoctorCheck {
            id: "database.behavior".to_owned(),
            status: "skipped".to_owned(),
            message: Some("persistence is disabled in this build".to_owned()),
        });
        for plugin in self.plugins.list() {
            if plugin.available {
                checks.push(DoctorCheck {
                    id: format!("plugin.{}.available", plugin.manifest.id),
                    status: "ok".to_owned(),
                    message: None,
                });
            } else {
                checks.push(DoctorCheck {
                    id: format!("plugin.{}.available", plugin.manifest.id),
                    status: "failed".to_owned(),
                    message: plugin
                        .reason
                        .clone()
                        .or_else(|| Some("plugin is unavailable".to_owned())),
                });
            }
        }
        if cfg!(feature = "native-tiktok") {
            let connected = self.live.is_connected();
            checks.push(DoctorCheck {
                id: "live.transport".to_owned(),
                status: "ok".to_owned(),
                message: Some(
                    if connected {
                        "connected"
                    } else {
                        "disconnected"
                    }
                    .to_owned(),
                ),
            });
        } else {
            checks.push(DoctorCheck {
                id: "live.transport".to_owned(),
                status: "skipped".to_owned(),
                message: Some("native TikTok client is disabled in this build".to_owned()),
            });
        }
        let processors = self.processor_list().len();
        checks.push(DoctorCheck {
            id: "processors.index".to_owned(),
            status: "ok".to_owned(),
            message: Some(format!("{processors} processor(s) indexed")),
        });
        let ok = !checks.iter().any(|check| check.status == "failed");
        DoctorReport { ok, checks }
    }
}

impl AppCore {
    /// Upserts a behavior record (create-or-replace). Unlike
    /// [`AppCore::automation_create`] / [`AppCore::automation_update`], the
    /// record must already carry an `id` and a `name`; this matches the
    /// legacy WebView save path exactly.
    pub fn automation_save(
        &self,
        kind: AutomationKind,
        value: &Value,
    ) -> Result<Value, OperationError> {
        let object = value
            .as_object()
            .ok_or_else(|| OperationError::invalid("automation record must be an object"))?;
        let id = object
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| OperationError::invalid("automation record needs an `id`"))?;
        if object
            .get("name")
            .and_then(Value::as_str)
            .is_none_or(|name| name.trim().is_empty())
        {
            return Err(OperationError::invalid("automation record needs a `name`"));
        }
        let saved = self.save_automation_record(kind, value)?;
        self.events.publish_domain(DomainEvent::WorkflowChanged {
            kind: kind.label().to_owned(),
            id: id.to_owned(),
            change: "saved".to_owned(),
        });
        Ok(saved)
    }

    /// Last automation event observed, for context panels and test previews.
    pub fn automation_context(&self) -> (Option<Value>, Option<u64>) {
        let event = self
            .last_automation_event
            .read()
            .expect("automation event lock poisoned")
            .clone();
        let captured_at = *self
            .last_automation_event_at
            .read()
            .expect("automation timestamp lock poisoned");
        (event, captured_at)
    }

    /// Picks the top live room for a session cookie and connects to it.
    /// Room choice is deterministic (first of the viewer-ordered rooms).
    #[cfg(feature = "native-tiktok")]
    pub async fn live_pick(
        self: &Arc<Self>,
        session_cookie: String,
    ) -> Result<LiveStatus, OperationError> {
        if session_cookie.trim().is_empty() || session_cookie.len() > 16_384 {
            return Err(OperationError::invalid(
                "sessionCookie must be 1..=16384 characters",
            ));
        }
        self.start_live_event_pump();
        let mut rooms = self
            .live
            .live_channels(&session_cookie)
            .await
            .map_err(|error| OperationError::unavailable(error.to_string()))?;
        if rooms.is_empty() {
            return Err(OperationError::unavailable(
                "TikTok returned no live rooms.",
            ));
        }
        let room = rooms.remove(0);
        self.live_connect(room.unique_id, session_cookie, Some(room.room_id))
            .await
    }

    #[cfg(not(feature = "native-tiktok"))]
    pub async fn live_pick(
        self: &Arc<Self>,
        _session_cookie: String,
    ) -> Result<LiveStatus, OperationError> {
        Err(OperationError::unavailable(
            "the native TikTok client is disabled in this build",
        ))
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GiftDebugResult {
    pub gift_id: Option<String>,
    pub icon_url: Option<String>,
    pub has_icon: bool,
    pub total_gifts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptAnalysisResult {
    pub node_id: String,
    pub source: String,
    pub diagnostics: Vec<Value>,
    pub completions: Vec<Value>,
    pub hover: Option<Value>,
}

impl AppCore {
    pub fn app_state_get(
        &self,
        keys: Option<&[String]>,
    ) -> Result<BTreeMap<String, String>, OperationError> {
        if let Some(keys) = keys {
            if keys.len() > 256 {
                return Err(OperationError::invalid("at most 256 keys per request"));
            }
            if keys.iter().any(|key| key.is_empty() || key.len() > 256) {
                return Err(OperationError::invalid(
                    "app state keys must be 1..=256 characters",
                ));
            }
        }
        let state = self.app_state.read(keys);
        #[cfg(feature = "persistence")]
        let state = {
            let mut state = state;
            match self.db.load_app_state() {
                Ok(persisted) => {
                    state = persisted
                        .into_iter()
                        .filter_map(|(key, value)| {
                            value.as_str().map(|value| (key, value.to_owned()))
                        })
                        .filter(|(key, _)| {
                            keys.is_none_or(|keys| keys.is_empty() || keys.contains(key))
                        })
                        .collect();
                }
                Err(error) => tracing::warn!(%error, "could not load app state"),
            }
            state
        };
        Ok(state)
    }

    pub fn app_state_set(
        &self,
        key: &str,
        value: &str,
    ) -> Result<BTreeMap<String, String>, OperationError> {
        if key.is_empty() || key.len() > 256 {
            return Err(OperationError::invalid(
                "app state key must be 1..=256 characters",
            ));
        }
        if value.len() > 65_536 {
            return Err(OperationError::invalid(
                "app state value exceeds 65536 characters",
            ));
        }
        self.app_state.set(key.to_owned(), value.to_owned());
        #[cfg(feature = "persistence")]
        if let Err(error) = self.db.save_app_state(key, value) {
            tracing::warn!(%error, "could not persist app state");
        }
        Ok([(key.to_owned(), value.to_owned())].into_iter().collect())
    }

    pub fn creator_get(&self, unique_id: Option<&str>) -> Option<Value> {
        #[cfg(feature = "persistence")]
        {
            match self.db.load_creator(unique_id) {
                Ok(creator) => creator,
                Err(error) => {
                    tracing::warn!(%error, "could not load creator state");
                    None
                }
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = unique_id;
            None
        }
    }

    pub fn creator_recent(&self, limit: Option<i64>) -> Vec<Value> {
        #[cfg(feature = "persistence")]
        {
            match self
                .db
                .load_recent_creators(limit.unwrap_or(10).clamp(0, 1000))
            {
                Ok(creators) => creators,
                Err(error) => {
                    tracing::warn!(%error, "could not load creator history");
                    Vec::new()
                }
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = limit;
            Vec::new()
        }
    }

    pub fn creator_history_clear(&self) {
        #[cfg(feature = "persistence")]
        if let Err(error) = self.db.clear_creator_history() {
            tracing::warn!(%error, "could not clear creator history");
        }
        self.events
            .publish_domain(DomainEvent::CreatorChanged { unique_id: None });
    }

    pub fn analytics_summary(
        &self,
        creator_unique_id: Option<String>,
        start_day: Option<i64>,
        end_day: Option<i64>,
        limit: Option<i64>,
    ) -> Option<Value> {
        #[cfg(feature = "persistence")]
        {
            let now_unix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0);
            let today = crate::db::utc_day(now_unix);
            let end = end_day.unwrap_or(today);
            let start = start_day.unwrap_or(end - 6).min(end);
            let creator = creator_unique_id
                .filter(|value| !value.trim().is_empty())
                .or_else(|| self.current_creator_unique_id())
                .unwrap_or_default();
            if creator.is_empty() {
                return None;
            }
            match self
                .db
                .analytics_summary(&creator, start, end, limit.unwrap_or(10))
            {
                Ok(summary) => match serde_json::to_value(summary) {
                    Ok(summary) => Some(summary),
                    Err(error) => {
                        tracing::warn!(%error, "could not serialize analytics summary");
                        None
                    }
                },
                Err(error) => {
                    tracing::warn!(%error, "could not load analytics summary");
                    None
                }
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (creator_unique_id, start_day, end_day, limit);
            None
        }
    }

    pub fn gift_catalog(&self) -> Vec<Value> {
        #[cfg(feature = "persistence")]
        {
            match self.db.load_gift_catalog() {
                Ok(gifts) => gifts,
                Err(error) => {
                    tracing::warn!(%error, "could not load gift catalog");
                    Vec::new()
                }
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            Vec::new()
        }
    }

    pub fn gift_debug(&self, gift_id: Option<&str>) -> GiftDebugResult {
        let catalog = self.gift_catalog();
        let found = gift_id.and_then(|id| {
            let id = id.trim();
            catalog
                .iter()
                .find(|gift| gift.get("id").and_then(Value::as_str) == Some(id))
        });
        let icon_url = found
            .and_then(|gift| gift.get("iconUrl").and_then(Value::as_str))
            .map(str::to_owned);
        GiftDebugResult {
            gift_id: gift_id.map(str::to_owned),
            icon_url: icon_url.clone(),
            has_icon: icon_url.is_some(),
            total_gifts: catalog.len() as u64,
        }
    }

    pub fn workflow_get(&self, id: &str) -> Result<Value, OperationError> {
        let id = clean_record_id(id, "workflow")?;
        self.workflow_list()?
            .into_iter()
            .find(|workflow| workflow.get("id").and_then(Value::as_str) == Some(id.as_str()))
            .ok_or_else(|| OperationError::not_found(format!("workflow `{id}` does not exist")))
    }

    pub fn automation_nodes(&self) -> Vec<Value> {
        builtin_node_catalog()
    }

    pub fn automation_script_analyze(
        &self,
        node_id: &str,
        source: &str,
        offset: u64,
        event_type: Option<&str>,
    ) -> Result<ScriptAnalysisResult, OperationError> {
        if node_id.is_empty() || node_id.len() > 256 {
            return Err(OperationError::invalid("nodeId must be 1..=256 characters"));
        }
        if source.len() > 128 * 1024 || offset > 128 * 1024 {
            return Err(OperationError::invalid(
                "source/offset exceeds the 128 KiB limit",
            ));
        }
        if event_type.is_some_and(|event| event.len() > 256) {
            return Err(OperationError::invalid("eventType is too long (max 256)"));
        }
        let diagnostics = self
            .automation
            .validate_script(source)
            .err()
            .map(|message| {
                vec![json!({
                    "line": 1,
                    "column": 1,
                    "message": message,
                    "severity": "error"
                })]
            })
            .unwrap_or_default();
        Ok(ScriptAnalysisResult {
            node_id: node_id.to_owned(),
            source: source.to_owned(),
            diagnostics,
            completions: Vec::new(),
            hover: None,
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

    /// Desktop-only native file dialog. Headless hosts return
    /// `capability_unavailable`; callers must not emulate a dialog.
    pub async fn media_pick(
        &self,
        options: MediaPickerOptions,
    ) -> Result<Option<MediaSelection>, OperationError> {
        if options
            .title
            .as_ref()
            .is_some_and(|title| title.len() > 256)
        {
            return Err(OperationError::invalid("title is too long (max 256)"));
        }
        if options
            .initial_directory
            .as_ref()
            .is_some_and(|directory| directory.len() > 4096)
        {
            return Err(OperationError::invalid(
                "initialDirectory is too long (max 4096)",
            ));
        }
        if options.extensions.len() > 32
            || options.extensions.iter().any(|extension| {
                extension.is_empty()
                    || extension.len() > 16
                    || !extension.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '_')
                    })
            })
        {
            return Err(OperationError::invalid(
                "extensions must be at most 32 alphanumeric tokens",
            ));
        }
        match self.open_media_picker(options).await {
            Ok(selection) => Ok(selection),
            Err(MediaApiError::Validation(error)) => {
                Err(OperationError::invalid(error.to_string()))
            }
            Err(MediaApiError::Host(MediaHostError::Unavailable(message))) => {
                Err(OperationError::capability_unavailable(message))
            }
            Err(MediaApiError::Host(MediaHostError::Failed(message))) => {
                Err(OperationError::unavailable(message))
            }
        }
    }
}
