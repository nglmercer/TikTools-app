//! Native ownership of single-record write identities. Recovery reads the
//! original ledger only; historical results never write definitions again.
use super::{AutomationKind, OperationError};
use crate::{
    collaboration::{CommandReceipt, ResourceVersion},
    AppCore,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerDefinitionOperation {
    Create,
    Update,
    Delete,
    Enable,
    Disable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerDefinitionIntent {
    pub command_id: String,
    pub operation: OwnerDefinitionOperation,
    pub kind: AutomationKind,
    pub id: String,
    pub expected_revision: String,
    pub host_instance_id: String,
    pub record_digest: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerDefinitionOutcome {
    Succeeded,
    Rejected,
    ReviewedUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerDefinitionReceipt {
    pub revision: String,
    pub record_digest: Option<String>,
    pub enabled: Option<bool>,
    pub deleted: Option<bool>,
    pub affected_resources: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerDefinitionRecord {
    pub intent: OwnerDefinitionIntent,
    pub outcome: OwnerDefinitionOutcome,
    pub receipt: Option<OwnerDefinitionReceipt>,
    pub error_code: Option<String>,
    pub recorded_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerDefinitionState {
    pub host_id: String,
    pub host_instance_id: String,
    pub pending: Option<OwnerDefinitionIntent>,
    pub recent: Vec<OwnerDefinitionRecord>,
}

/// Returned only by the original successful submission. Recovery surfaces
/// contain metadata, so they cannot silently replace an editor's newer draft.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerDefinitionAcknowledgement {
    pub value: Value,
    pub resource_versions: Vec<ResourceVersion>,
    pub command: CommandReceipt,
    pub host_instance_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerDefinitionSubmission {
    pub state: OwnerDefinitionState,
    pub acknowledgement: Option<OwnerDefinitionAcknowledgement>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "operation",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OwnerDefinitionSubmit {
    Create {
        kind: AutomationKind,
        record: Value,
        expected_revision: String,
        host_id: String,
        host_instance_id: String,
    },
    Update {
        kind: AutomationKind,
        id: String,
        record: Value,
        expected_revision: String,
        host_id: String,
        host_instance_id: String,
    },
    Delete {
        kind: AutomationKind,
        id: String,
        expected_revision: String,
        host_id: String,
        host_instance_id: String,
    },
    Enable {
        kind: AutomationKind,
        id: String,
        expected_revision: String,
        host_id: String,
        host_instance_id: String,
    },
    Disable {
        kind: AutomationKind,
        id: String,
        expected_revision: String,
        host_id: String,
        host_instance_id: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerDefinitionReviewUnknown {
    pub command_id: String,
    pub acknowledge_unknown: bool,
}

#[cfg(feature = "persistence")]
pub(crate) fn definition_now() -> i64 {
    super::owner_approval::approval_now()
}

#[cfg(feature = "persistence")]
pub(crate) fn definition_rejection(code: &str) -> bool {
    matches!(
        code,
        "revision_conflict"
            | "not_found"
            | "invalid_params"
            | "forbidden"
            | "capability_unavailable"
            | "host_restarted"
            | "too_large"
    )
}

#[cfg(feature = "persistence")]
pub(crate) fn definition_record_digest(value: &Value) -> String {
    fn canonical(value: &Value) -> Value {
        match value {
            Value::Object(values) => {
                let sorted: std::collections::BTreeMap<_, _> = values.iter().collect();
                Value::Object(
                    sorted
                        .into_iter()
                        .map(|(k, v)| (k.clone(), canonical(v)))
                        .collect(),
                )
            }
            Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
            other => other.clone(),
        }
    }
    crate::collaboration::hash_secret(&canonical(value).to_string())
}

#[cfg(feature = "persistence")]
fn definition_failure(error: crate::db::DatabaseError) -> OperationError {
    let (code, message) = match error {
        crate::db::DatabaseError::OwnerAdoptionPending => (
            "command_pending",
            "resolve or explicitly review the pending owner operation first",
        ),
        crate::db::DatabaseError::Invalid(_) | crate::db::DatabaseError::Json(_) => (
            "definition_journal_invalid",
            "owner definition journal requires review",
        ),
        _ => (
            "definition_journal_unavailable",
            "owner definition journal could not be read or persisted",
        ),
    };
    OperationError {
        code,
        message: message.into(),
        data: None,
    }
}

#[cfg(feature = "persistence")]
fn validate_source(value: &Value) -> Result<(), OperationError> {
    let mut stack = vec![(value, 0usize)];
    let mut nodes = 0usize;
    while let Some((value, depth)) = stack.pop() {
        nodes += 1;
        if depth > 64 || nodes > 65_536 {
            return Err(OperationError::invalid(
                "definition source exceeds its structure budget",
            ));
        }
        let children: Box<dyn Iterator<Item = &Value> + '_> = match value {
            Value::Object(values) => Box::new(values.values()),
            Value::Array(values) => Box::new(values.iter()),
            _ => continue,
        };
        for child in children {
            if stack.len() + nodes >= 65_536 {
                return Err(OperationError::invalid(
                    "definition source exceeds its structure budget",
                ));
            }
            stack.push((child, depth + 1));
        }
    }
    if value.to_string().len() > 256 * 1024 {
        return Err(OperationError {
            code: "too_large",
            message: "definition source exceeds its byte budget".into(),
            data: None,
        });
    }
    Ok(())
}

#[cfg(feature = "persistence")]
fn prepare(
    input: OwnerDefinitionSubmit,
) -> Result<(String, String, String, super::AutomationMutation), OperationError> {
    use super::AutomationMutation;
    let (host, instance, expected, operation) = match input {
        OwnerDefinitionSubmit::Create {
            kind,
            record,
            expected_revision,
            host_id,
            host_instance_id,
        } => {
            validate_source(&record)?;
            (
                host_id,
                host_instance_id,
                expected_revision,
                AutomationMutation::Create { kind, record },
            )
        }
        OwnerDefinitionSubmit::Update {
            kind,
            id,
            record,
            expected_revision,
            host_id,
            host_instance_id,
        } => {
            validate_source(&record)?;
            if record.get("id").and_then(Value::as_str) != Some(id.as_str()) {
                return Err(OperationError::invalid(
                    "definition target must match its saved source identity",
                ));
            }
            (
                host_id,
                host_instance_id,
                expected_revision,
                AutomationMutation::Update { kind, id, record },
            )
        }
        OwnerDefinitionSubmit::Delete {
            kind,
            id,
            expected_revision,
            host_id,
            host_instance_id,
        } => (
            host_id,
            host_instance_id,
            expected_revision,
            AutomationMutation::Delete { kind, id },
        ),
        OwnerDefinitionSubmit::Enable {
            kind,
            id,
            expected_revision,
            host_id,
            host_instance_id,
        } => (
            host_id,
            host_instance_id,
            expected_revision,
            AutomationMutation::SetEnabled {
                kind,
                id,
                enabled: true,
            },
        ),
        OwnerDefinitionSubmit::Disable {
            kind,
            id,
            expected_revision,
            host_id,
            host_instance_id,
        } => (
            host_id,
            host_instance_id,
            expected_revision,
            AutomationMutation::SetEnabled {
                kind,
                id,
                enabled: false,
            },
        ),
    };
    let operation = operation.normalize()?;
    crate::db::validate_owner_definition_input(&operation, &expected).map_err(|_| {
        OperationError::invalid("invalid definition target, source or reviewed revision")
    })?;
    Ok((host, instance, expected, operation))
}

#[cfg(feature = "persistence")]
fn fresh_acknowledgement(
    intent: &OwnerDefinitionIntent,
    state: &OwnerDefinitionState,
    result: crate::collaboration::Versioned<Value>,
) -> Result<OwnerDefinitionAcknowledgement, OperationError> {
    let invalid = || {
        definition_failure(crate::db::DatabaseError::Invalid(
            "unverified fresh definition acknowledgement".into(),
        ))
    };
    let record = state
        .recent
        .first()
        .filter(|r| r.intent.command_id == intent.command_id)
        .ok_or_else(invalid)?;
    let receipt = record.receipt.as_ref().ok_or_else(invalid)?;
    let command = result.command.ok_or_else(invalid)?;
    let primary = result.resource_versions.first().ok_or_else(invalid)?;
    let scope = match intent.kind {
        AutomationKind::Action => "action",
        AutomationKind::Event => "event",
    };
    if state.pending.is_some()
        || record.intent != *intent
        || record.outcome != OwnerDefinitionOutcome::Succeeded
        || record.error_code.is_some()
        || command.command_id != intent.command_id
        || command.state != crate::collaboration::CommandState::Succeeded
        || !command
            .committed_at
            .is_some_and(|n| (1..=9_007_199_254_740_991).contains(&n))
        || !command.resource.as_ref().is_some_and(|r| {
            r.kind == crate::collaboration::ResourceKind::Automation
                && r.scope == scope
                && r.id == intent.id
        })
        || result.host_instance_id.as_deref() != Some(intent.host_instance_id.as_str())
        || primary.kind != crate::collaboration::ResourceKind::Automation
        || primary.scope != scope
        || primary.id != intent.id
        || primary.revision != receipt.revision
        || usize::try_from(receipt.affected_resources).ok() != Some(result.resource_versions.len())
        || result.value.get("id").and_then(Value::as_str) != Some(intent.id.as_str())
    {
        return Err(invalid());
    }
    match intent.operation {
        OwnerDefinitionOperation::Create
        | OwnerDefinitionOperation::Update
        | OwnerDefinitionOperation::Enable
        | OwnerDefinitionOperation::Disable => {
            if receipt.record_digest.as_deref()
                != Some(definition_record_digest(&result.value).as_str())
                || receipt.enabled != result.value.get("enabled").and_then(Value::as_bool)
                || receipt.deleted.is_some()
            {
                return Err(invalid());
            }
        }
        OwnerDefinitionOperation::Delete => {
            if receipt.deleted != result.value.get("deleted").and_then(Value::as_bool)
                || receipt.record_digest.is_some()
                || receipt.enabled.is_some()
            {
                return Err(invalid());
            }
        }
    }
    Ok(OwnerDefinitionAcknowledgement {
        value: result.value,
        resource_versions: result.resource_versions,
        command,
        host_instance_id: intent.host_instance_id.clone(),
    })
}

#[cfg(feature = "persistence")]
fn definition_crash(stage: &str) {
    #[cfg(test)]
    if std::env::var("TIKTOOLS_OWNER_DEFINITION_CRASH").as_deref() == Ok(stage) {
        std::process::exit(90);
    }
    #[cfg(not(test))]
    let _ = stage;
}

impl AppCore {
    pub fn owner_definition_state(&self) -> Result<OwnerDefinitionState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner operation coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            self.db
                .owner_definition_state(service.host_id(), service.instance_id())
                .map_err(definition_failure)
        }
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::capability_unavailable(
            "owner definition recovery requires persistence",
        ))
    }

    pub fn owner_definition_submit(
        &self,
        input: OwnerDefinitionSubmit,
    ) -> Result<OwnerDefinitionSubmission, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use super::AutomationMutation;
            let (host, instance, expected, operation) = prepare(input)?;
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner operation coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            if host != service.host_id() || instance != service.instance_id() {
                return Err(OperationError {
                    code: "host_restarted",
                    message: "owner host identity changed; review again".into(),
                    data: None,
                });
            }
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| OperationError::internal("command identity is unavailable"))?;
            let (kind, target) = operation.target();
            let (variant, record_digest) = match &operation {
                AutomationMutation::Create { record, .. } => (
                    OwnerDefinitionOperation::Create,
                    Some(definition_record_digest(record)),
                ),
                AutomationMutation::Update { record, .. } => (
                    OwnerDefinitionOperation::Update,
                    Some(definition_record_digest(record)),
                ),
                AutomationMutation::Delete { .. } => (OwnerDefinitionOperation::Delete, None),
                AutomationMutation::SetEnabled { enabled: true, .. } => {
                    (OwnerDefinitionOperation::Enable, None)
                }
                AutomationMutation::SetEnabled { enabled: false, .. } => {
                    (OwnerDefinitionOperation::Disable, None)
                }
                AutomationMutation::Adopt { .. } => {
                    unreachable!("the closed native definition union cannot adopt")
                }
            };
            let intent = OwnerDefinitionIntent {
                command_id: format!(
                    "owner-definition-{}",
                    bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
                ),
                operation: variant,
                kind,
                id: target.into(),
                expected_revision: expected.clone(),
                host_instance_id: instance.clone(),
                record_digest,
                created_at: definition_now(),
            };
            let request = crate::collaboration::CommandRequest {
                command_id: intent.command_id.clone(),
                host_instance_id: instance,
                expected_revision: expected,
            };
            request
                .validate()
                .map_err(|_| OperationError::invalid("invalid owner definition revision"))?;
            definition_crash("before_intent");
            self.db
                .owner_definition_begin(&host, &intent, &operation)
                .map_err(definition_failure)?;
            definition_crash("after_intent");
            definition_crash("before_dispatch");
            let result = self.automation_definition_command(request, operation);
            definition_crash("after_effect");
            let original = match result {
                Ok(result) => {
                    self.db
                        .owner_definition_resolve(&host, false, None)
                        .map_err(definition_failure)?;
                    Some(result)
                }
                Err(error) if definition_rejection(error.code) => {
                    self.db
                        .owner_definition_resolve(&host, false, Some(error.code))
                        .map_err(definition_failure)?;
                    None
                }
                Err(error) => return Err(error),
            };
            definition_crash("after_record");
            let state = self
                .db
                .owner_definition_state(&host, service.instance_id())
                .map_err(definition_failure)?;
            let acknowledgement = original
                .map(|result| fresh_acknowledgement(&intent, &state, result))
                .transpose()?;
            Ok(OwnerDefinitionSubmission {
                state,
                acknowledgement,
            })
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = input;
            Err(OperationError::capability_unavailable(
                "owner definition recovery requires persistence",
            ))
        }
    }

    pub fn owner_definition_resolve(&self) -> Result<OwnerDefinitionState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner operation coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            self.db
                .owner_definition_resolve(service.host_id(), false, None)
                .map_err(definition_failure)?;
            self.db
                .owner_definition_state(service.host_id(), service.instance_id())
                .map_err(definition_failure)
        }
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::capability_unavailable(
            "owner definition recovery requires persistence",
        ))
    }

    pub fn owner_definition_review_unknown(
        &self,
        input: OwnerDefinitionReviewUnknown,
    ) -> Result<OwnerDefinitionState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            if !input.acknowledge_unknown {
                return Err(OperationError {
                    code: "confirmation_required",
                    message: "review the unresolved original definition outcome".into(),
                    data: None,
                });
            }
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner operation coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            let state = self
                .db
                .owner_definition_state(service.host_id(), service.instance_id())
                .map_err(definition_failure)?;
            if state.pending.as_ref().map(|i| i.command_id.as_str())
                != Some(input.command_id.as_str())
            {
                return Err(OperationError {
                    code: "stale_view",
                    message: "pending owner definition changed".into(),
                    data: None,
                });
            }
            self.db
                .owner_definition_resolve(service.host_id(), true, None)
                .map_err(definition_failure)?;
            self.db
                .owner_definition_state(service.host_id(), service.instance_id())
                .map_err(definition_failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = input;
            Err(OperationError::capability_unavailable(
                "owner definition recovery requires persistence",
            ))
        }
    }
}

#[cfg(all(test, feature = "persistence"))]
mod tests;
