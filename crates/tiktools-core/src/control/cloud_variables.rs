//! Runtime variables as the sixth v2 sync collection (`variables`).
//!
//! The collection is credential-free by contract: only the types `text`,
//! `number`, `boolean` and `list` exist on the wire. A `secret` variable is
//! excluded here, in the host, before anything is staged; its value is never
//! copied into a pending edit, a request body, a log line or an error.
//! Staging also requires the instance to announce the collection
//! (`sync.optionalCollections` contains `variables`); an older backend gets
//! `capability_unavailable` and nothing is staged.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::OperationError;
use crate::AppCore;

/// Local variable types. `Secret` exists only so the host can recognise and
/// exclude it; it has no portable representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum VariableKind {
    Text,
    Number,
    Boolean,
    List,
    Secret,
}

impl VariableKind {
    fn wire(self) -> Option<&'static str> {
        match self {
            Self::Text => Some("text"),
            Self::Number => Some("number"),
            Self::Boolean => Some("boolean"),
            Self::List => Some("list"),
            Self::Secret => None,
        }
    }
}

/// One local variable offered for sync.
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariableInput {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: VariableKind,
    /// `text`: string; `number`: finite number; `boolean`; `list`: array of
    /// strings. For `secret` the value is ignored and dropped.
    pub value: Value,
}

// The value is never printed, whatever the type.
impl std::fmt::Debug for VariableInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VariableInput")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("value", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncV2StageVariablesParams {
    /// Variables to create or update in the Cloud workspace.
    #[serde(default)]
    pub variables: Vec<VariableInput>,
    /// Names whose Cloud record should be deleted (also how a variable that
    /// became secret is withdrawn from the Cloud).
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncV2StageVariablesResult {
    /// Variables with a pending put now.
    pub staged: usize,
    /// Names with a pending delete now.
    pub removed: usize,
    /// Names of `secret` variables that were left out. Nothing about them was
    /// staged or sent.
    pub excluded_secrets: Vec<String>,
    /// Names the portable contract refuses (bad name, wrong value type, too
    /// large, or a value that looks like a credential). Never staged.
    pub rejected: Vec<String>,
}

/// What a request resolves to before any state is touched.
#[derive(Debug, PartialEq)]
pub(crate) struct VariablePlan {
    pub(crate) puts: Vec<(String, Value)>,
    pub(crate) removes: Vec<String>,
    pub(crate) excluded_secrets: Vec<String>,
    pub(crate) rejected: Vec<String>,
}

/// Pure planning: secrets are dropped here and nothing of them survives.
pub(crate) fn plan(params: SyncV2StageVariablesParams) -> Result<VariablePlan, OperationError> {
    use cloud_client::{WorkspaceEdit, WorkspaceKey};
    let mut seen = std::collections::BTreeSet::new();
    let mut plan = VariablePlan {
        puts: Vec::new(),
        removes: Vec::new(),
        excluded_secrets: Vec::new(),
        rejected: Vec::new(),
    };
    for variable in params.variables {
        if !seen.insert(variable.name.clone()) {
            return Err(OperationError::invalid(
                "a variable name appears more than once",
            ));
        }
        let Some(kind) = variable.kind.wire() else {
            // The value is dropped without being read.
            plan.excluded_secrets.push(variable.name);
            continue;
        };
        let document = json!({
            "schemaVersion": 1,
            "name": variable.name,
            "type": kind,
            "value": variable.value,
        });
        let valid = WorkspaceKey::new("variables", variable.name.clone())
            .map(|key| WorkspaceEdit::Put(document.clone()).valid(&key))
            .unwrap_or(false);
        if valid {
            plan.puts.push((variable.name, document));
        } else {
            plan.rejected.push(variable.name);
        }
    }
    for name in params.remove {
        if !seen.insert(name.clone()) {
            return Err(OperationError::invalid(
                "a variable name appears more than once",
            ));
        }
        if WorkspaceKey::new("variables", name.clone()).is_err() {
            plan.rejected.push(name);
        } else {
            plan.removes.push(name);
        }
    }
    Ok(plan)
}

fn unsupported(reason: &str) -> OperationError {
    OperationError {
        code: "capability_unavailable",
        message: "this Cloud instance does not sync variables".into(),
        data: Some(json!({
            "reason": reason,
            "capability": "sync.optionalCollections",
            "collection": "variables",
        })),
    }
}

/// `Ok` only when the instance announces the `variables` collection.
pub(crate) async fn require_variables(origin: &str) -> Result<(), OperationError> {
    let client = cloud_client::CloudCatalog::new(origin)
        .map_err(|_| OperationError::unavailable("Cloud origin is unavailable"))?;
    match client.workspace_sync_capabilities().await {
        Ok(capabilities) if capabilities.variables => Ok(()),
        Ok(_) => Err(unsupported("variables_unsupported")),
        Err(cloud_client::SyncError::Http(404)) => Err(unsupported("capabilities_missing")),
        Err(_) => Err(OperationError::unavailable(
            "Cloud v2 capabilities could not be verified",
        )),
    }
}

impl AppCore {
    /// Stage variables for v2 sync under the same fences as preferences.
    /// Secrets are excluded in the host; an instance that does not announce
    /// `variables` gets `capability_unavailable` and nothing is staged. Only
    /// the staging happens here: the next `sync.v2.now` uploads.
    pub async fn sync_v2_stage_variables(
        self: &std::sync::Arc<Self>,
        params: SyncV2StageVariablesParams,
    ) -> Result<SyncV2StageVariablesResult, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let plan = plan(params)?;
            let mut result = SyncV2StageVariablesResult {
                staged: 0,
                removed: 0,
                excluded_secrets: plan.excluded_secrets.clone(),
                rejected: plan.rejected.clone(),
            };
            if plan.puts.is_empty() && plan.removes.is_empty() {
                return Ok(result);
            }
            let (session, _) = self.v2_identity().await?;
            require_variables(&session.instance_origin()).await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let mut edits = Vec::new();
            for (name, document) in plan.puts {
                let key = cloud_client::WorkspaceKey::new("variables", name)
                    .map_err(|_| OperationError::invalid("invalid variable name"))?;
                edits.push((key, cloud_client::WorkspaceEdit::Put(document)));
            }
            for name in plan.removes {
                let key = cloud_client::WorkspaceKey::new("variables", name)
                    .map_err(|_| OperationError::invalid("invalid variable name"))?;
                edits.push((key, cloud_client::WorkspaceEdit::Delete));
            }
            let puts = edits
                .iter()
                .filter(|(_, edit)| matches!(edit, cloud_client::WorkspaceEdit::Put(_)))
                .count();
            let removes = edits.len() - puts;
            let core = self.clone();
            let core_session = session.clone();
            tokio::task::spawn_blocking(move || {
                let coordinator = core.sync.coordinator().map_err(|_| {
                    OperationError::unavailable("Cloud sync storage is unavailable")
                })?;
                for (key, edit) in edits {
                    coordinator
                        .stage_v2_product_edit(core_session.clone(), key, edit)
                        .map_err(|_| OperationError::conflict("Cloud v2 staging was rejected"))?;
                }
                Ok::<(), OperationError>(())
            })
            .await
            .map_err(|_| OperationError::unavailable("Cloud sync worker is unavailable"))??;
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            result.staged = puts;
            result.removed = removes;
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = params;
            Err(OperationError::unavailable(
                "Cloud v2 variables require persistence",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, kind: VariableKind, value: Value) -> VariableInput {
        VariableInput {
            name: name.into(),
            kind,
            value,
        }
    }

    #[test]
    fn secrets_are_excluded_and_leave_no_trace_in_the_plan() {
        let planned = plan(SyncV2StageVariablesParams {
            variables: vec![
                input("apiKey", VariableKind::Secret, json!("hunter2-very-secret")),
                input("commandPort", VariableKind::Number, json!(46665)),
                input("tags", VariableKind::List, json!(["a", "b"])),
                input("greeting", VariableKind::Text, json!("hola")),
                input("loud", VariableKind::Boolean, json!(true)),
            ],
            remove: vec!["old".into()],
        })
        .unwrap();
        assert_eq!(planned.excluded_secrets, vec!["apiKey".to_owned()]);
        assert_eq!(planned.puts.len(), 4);
        assert_eq!(planned.removes, vec!["old".to_owned()]);
        assert!(planned.rejected.is_empty());
        let text = format!("{:?}", planned.puts);
        assert!(!text.contains("hunter2") && !text.contains("apiKey"));
        assert_eq!(
            planned.puts[0].1,
            json!({"schemaVersion":1,"name":"commandPort","type":"number","value":46665})
        );
        // Debug output of the input never shows the value.
        let shown = format!("{:?}", input("k", VariableKind::Text, json!("visible?")));
        assert!(!shown.contains("visible?"));
    }

    #[test]
    fn contract_violations_are_rejected_by_name_without_failing_the_rest() {
        let planned = plan(SyncV2StageVariablesParams {
            variables: vec![
                input("ok", VariableKind::Text, json!("fine")),
                input("9bad", VariableKind::Text, json!("x")),
                input("wrongType", VariableKind::Number, json!("1")),
                input("token", VariableKind::Text, json!("Bearer abcdef")),
                input("big", VariableKind::Text, json!("x".repeat(4097))),
                input("mixed", VariableKind::List, json!(["a", 1])),
            ],
            remove: vec!["not valid".into()],
        })
        .unwrap();
        assert_eq!(planned.puts.len(), 1);
        assert_eq!(
            planned.rejected,
            vec!["9bad", "wrongType", "token", "big", "mixed", "not valid"]
        );
        assert!(planned.removes.is_empty());
    }

    #[test]
    fn duplicate_names_are_refused() {
        let duplicate = SyncV2StageVariablesParams {
            variables: vec![
                input("a", VariableKind::Text, json!("1")),
                input("a", VariableKind::Secret, json!("2")),
            ],
            remove: vec![],
        };
        assert_eq!(plan(duplicate).unwrap_err().code, "invalid_params");
        let both = SyncV2StageVariablesParams {
            variables: vec![input("a", VariableKind::Text, json!("1"))],
            remove: vec!["a".into()],
        };
        assert_eq!(plan(both).unwrap_err().code, "invalid_params");
    }

    #[test]
    fn unknown_fields_and_types_do_not_deserialize() {
        assert!(serde_json::from_value::<SyncV2StageVariablesParams>(
            json!({"variables":[{"name":"a","type":"password","value":"x"}]})
        )
        .is_err());
        assert!(serde_json::from_value::<SyncV2StageVariablesParams>(
            json!({"variables":[{"name":"a","type":"text","value":"x","secret":true}]})
        )
        .is_err());
        assert!(serde_json::from_value::<SyncV2StageVariablesParams>(json!({})).is_ok());
    }
}
