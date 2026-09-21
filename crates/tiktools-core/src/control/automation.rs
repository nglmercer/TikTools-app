//! Behavior/automation records and script analysis.

use super::{clean_record_id, fresh_record_id, OperationError};
use crate::events::DomainEvent;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

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
pub struct ScriptAnalysisResult {
    pub node_id: String,
    pub source: String,
    pub diagnostics: Vec<Value>,
    pub completions: Vec<Value>,
    pub hover: Option<Value>,
}

impl AppCore {
    /// Loads the merged behavior snapshot (persisted records plus the live
    /// runtime catalog) and refreshes the in-memory automation projection.
    /// This is the value-returning twin of the `get-behavior` emit path.
    pub fn behavior_snapshot(&self) -> Value {
        let snapshot = self.load_merged_behavior_snapshot();
        self.automation.replace_snapshot(&snapshot);
        self.request_hotkey_sync();
        snapshot
    }

    pub(crate) fn load_merged_behavior_snapshot(&self) -> Value {
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

    pub(crate) fn refresh_automation_snapshot(&self) {
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
}
