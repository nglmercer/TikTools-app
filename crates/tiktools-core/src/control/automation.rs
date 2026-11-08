//! Behavior/automation records and script analysis.
mod approval_commands;
mod commands;
mod execution_manifest;
mod remote;
mod versioned;
pub use commands::AutomationMutation;

use super::{clean_record_id, fresh_record_id, OperationError};
use crate::events::DomainEvent;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

/// Events followed by actions, preserving the legacy automation list shape.
pub type AutomationRecords = (Vec<Value>, Vec<Value>);

pub(super) fn automation_read_allowed(grant: &crate::collaboration::Grant) -> bool {
    grant
        .capabilities
        .contains(&crate::collaboration::Capability::AutomationRead)
        && !grant
            .resources
            .contains_key(&crate::collaboration::ResourceKind::Live)
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
    pub(crate) fn family(self) -> crate::collaboration::AutomationFamily {
        match self {
            Self::Event => crate::collaboration::AutomationFamily::Event,
            Self::Action => crate::collaboration::AutomationFamily::Action,
        }
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn table(self) -> &'static str {
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

/// One record that no longer exists, reported by the bulk
/// enablement pass so callers can prune it from their packs.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationEnablementMiss {
    /// `event` or `action`.
    pub kind: String,
    /// Record id that no longer exists.
    pub id: String,
}

/// Outcome of one bulk enablement pass: applied count plus
/// the records that were already gone.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationSetEnablementResult {
    pub updated: usize,
    pub missing: Vec<AutomationEnablementMiss>,
}

impl AppCore {
    /// Loads the merged behavior snapshot (persisted records plus the live
    /// runtime catalog) and refreshes the in-memory automation projection.
    /// This is the value-returning twin of the `get-behavior` emit path.
    pub fn behavior_snapshot(&self) -> Value {
        self.behavior_snapshot_versioned()
            .map(|v| v.value)
            .unwrap_or_else(|error| {
                tracing::warn!(%error,"could not load behavior snapshot");
                empty_behavior_snapshot()
            })
    }
    pub fn behavior_snapshot_versioned(
        &self,
    ) -> Result<crate::collaboration::Versioned<Value>, OperationError> {
        let _guard = self.automation_gate()?;
        self.refresh_automation_snapshot_locked()
    }
    fn load_merged_behavior_versioned(
        &self,
    ) -> Result<crate::collaboration::Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        let mut snapshot = self
            .db
            .load_behavior_snapshot_versioned()
            .map_err(|e| OperationError::internal(e.to_string()))?;
        #[cfg(not(feature = "persistence"))]
        let mut snapshot =
            crate::collaboration::Versioned::new(self.automation.records_snapshot(), Vec::new());
        self.merge_runtime_catalog(&mut snapshot.value);
        Ok(snapshot)
    }
    pub(crate) fn load_merged_behavior_snapshot(&self) -> Value {
        self.load_merged_behavior_versioned()
            .map(|v| v.value)
            .unwrap_or_else(|_| empty_behavior_snapshot())
    }
    pub(crate) fn refresh_automation_snapshot(&self) {
        if let Ok(_guard) = self.automation_gate() {
            if self.refresh_automation_snapshot_locked().is_err() {
                self.automation.quarantine();
            }
        }
    }
    fn refresh_automation_snapshot_locked(
        &self,
    ) -> Result<crate::collaboration::Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        let loaded = self
            .db
            .load_behavior_runtime_approval_snapshot()
            .map(|((snapshot, provenance), approvals)| (snapshot, provenance, approvals))
            .map_err(|e| OperationError::internal(e.to_string()));
        #[cfg(not(feature = "persistence"))]
        let loaded = self.load_merged_behavior_versioned().map(|v| (v, (), ()));
        let (mut snapshot, provenance, approvals) = match loaded {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.automation.quarantine();
                self.request_hotkey_sync();
                return Err(error);
            }
        };
        self.merge_runtime_catalog(&mut snapshot.value);
        #[cfg(feature = "persistence")]
        self.automation.replace_snapshot_trusted_with_approvals(
            &snapshot.value,
            &snapshot.resource_versions,
            provenance
                .into_iter()
                .map(|(scope, id, p)| ((scope, id), p.into()))
                .collect(),
            approvals,
        );
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (provenance, approvals);
            self.automation
                .replace_snapshot_versioned(&snapshot.value, &snapshot.resource_versions);
        }
        self.request_hotkey_sync();
        Ok(snapshot)
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
        self.automation_get_versioned(kind, id)?
            .value
            .ok_or_else(|| OperationError::not_found("automation record does not exist"))
    }
    pub fn automation_create(
        &self,
        kind: AutomationKind,
        record: Value,
    ) -> Result<Value, OperationError> {
        self.automation_create_versioned(kind, record, None)
            .map(|v| v.value)
    }
    pub fn automation_update(
        &self,
        kind: AutomationKind,
        id: &str,
        record: Value,
    ) -> Result<Value, OperationError> {
        self.automation_update_versioned(kind, id, record, None)
            .map(|v| v.value)
    }
    pub fn automation_delete(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> Result<bool, OperationError> {
        self.automation_delete_versioned(kind, id, None)
            .map(|v| v.value)
    }
    pub fn automation_set_enabled(
        &self,
        kind: AutomationKind,
        id: &str,
        enabled: bool,
    ) -> Result<Value, OperationError> {
        self.automation_set_enabled_versioned(kind, id, enabled, None)
            .map(|v| v.value)
    }
    pub fn automation_set_enablement_bulk(
        &self,
        updates: &[(AutomationKind, String, bool)],
    ) -> Result<AutomationSetEnablementResult, OperationError> {
        self.automation_set_enablement_versioned(updates)
            .map(|v| v.value)
    }
    pub async fn test_automation_event(self: &Arc<Self>, record: &Value) -> Value {
        self.test_event(record).await
    }

    pub async fn fire_automation_event(
        self: &Arc<Self>,
        trigger: &str,
        event: Option<&Value>,
        record: Option<&Value>,
    ) -> Value {
        self.fire_synthetic_event(trigger, event, record).await
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
        self.automation_save_versioned(kind, value, None)
            .map(|v| v.value)
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
