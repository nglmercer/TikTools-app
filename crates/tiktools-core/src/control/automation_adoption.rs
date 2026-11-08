//! Host-owned metadata journal for the owner's explicit authority transfers.
//! Requests never choose IDs and recovery never dispatches an adoption again.
use super::{AutomationKind, OperationError};
use crate::{collaboration::ResourceVersion, AppCore};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerAdoptionIntent {
    pub command_id: String,
    pub kind: AutomationKind,
    pub id: String,
    pub expected_revision: String,
    pub host_instance_id: String,
    pub created_at: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerAdoptionOutcome {
    Succeeded,
    Rejected,
    ReviewedUnknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerAdoptionRecord {
    pub intent: OwnerAdoptionIntent,
    pub outcome: OwnerAdoptionOutcome,
    pub revision: Option<String>,
    pub error_code: Option<String>,
    pub recorded_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerAdoptionState {
    pub host_id: String,
    pub host_instance_id: String,
    pub pending: Option<OwnerAdoptionIntent>,
    pub recent: Vec<OwnerAdoptionRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerAdoptionSubmit {
    pub kind: AutomationKind,
    pub id: String,
    pub expected_revision: String,
    pub host_id: String,
    pub host_instance_id: String,
    pub reviewed_versions: Vec<ResourceVersion>,
    pub acknowledge_authority: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerAdoptionReview {
    pub command_id: String,
    pub acknowledge_unknown: bool,
}

#[cfg(feature = "persistence")]
fn adoption_error(code: &'static str, message: &str) -> OperationError {
    OperationError {
        code,
        message: message.into(),
        data: None,
    }
}
#[cfg(feature = "persistence")]
pub(crate) fn adoption_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(9_007_199_254_740_991) as i64
}
#[cfg(feature = "persistence")]
pub(crate) fn adoption_failure(error: crate::db::DatabaseError) -> OperationError {
    match error {
        crate::db::DatabaseError::OwnerAdoptionPending => adoption_error(
            "command_pending",
            "resolve or explicitly review the pending owner authority transition first",
        ),
        crate::db::DatabaseError::Invalid(_) | crate::db::DatabaseError::Json(_) => adoption_error(
            "adoption_journal_invalid",
            "owner adoption journal requires review",
        ),
        _ => adoption_error(
            "adoption_journal_unavailable",
            "owner adoption journal could not be read or persisted",
        ),
    }
}
#[cfg(feature = "persistence")]
pub(crate) fn adoption_rejection(code: &str) -> bool {
    matches!(
        code,
        "revision_conflict"
            | "automation_not_found"
            | "not_found"
            | "invalid_params"
            | "forbidden"
            | "capability_unavailable"
            | "host_restarted"
            | "command_capacity"
            | "overloaded"
    )
}
#[cfg(all(test, feature = "persistence"))]
fn crash(stage: &str) {
    if std::env::var("TIKTOOLS_OWNER_ADOPTION_CRASH").as_deref() == Ok(stage) {
        std::process::exit(88);
    }
}
#[cfg(all(not(test), feature = "persistence"))]
fn crash(_: &str) {}

impl AppCore {
    pub fn owner_adoption_state(&self) -> Result<OwnerAdoptionState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner adoption coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            self.db
                .owner_adoption_state(service.host_id(), service.instance_id())
                .map_err(adoption_failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::capability_unavailable(
                "owner adoption requires persistence",
            ))
        }
    }
    pub fn owner_adoption_submit(
        &self,
        input: OwnerAdoptionSubmit,
    ) -> Result<OwnerAdoptionState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use super::{AutomationMutation, CommandCaller};
            use crate::collaboration::CommandRequest;
            if !input.acknowledge_authority {
                return Err(adoption_error(
                    "confirmation_required",
                    "review and acknowledge owner authority transfer",
                ));
            }
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner adoption coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            if input.host_id != service.host_id() || input.host_instance_id != service.instance_id()
            {
                return Err(adoption_error(
                    "host_restarted",
                    "owner host identity changed; review again",
                ));
            }
            if super::clean_record_id(&input.id, "automation")? != input.id
                || input.id.contains('*')
                || input.id.chars().any(char::is_control)
                || input.reviewed_versions.is_empty()
                || input.reviewed_versions.len() > 513
            {
                return Err(OperationError::invalid(
                    "invalid reviewed automation identity",
                ));
            }
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| OperationError::internal("command identity is unavailable"))?;
            let command_id = format!(
                "owner-adopt-{}",
                bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
            );
            let request = CommandRequest {
                command_id: command_id.clone(),
                host_instance_id: input.host_instance_id.clone(),
                expected_revision: input.expected_revision.clone(),
            };
            request
                .validate()
                .map_err(|_| OperationError::invalid("invalid owner adoption revision"))?;
            let intent = OwnerAdoptionIntent {
                command_id,
                kind: input.kind,
                id: input.id.clone(),
                expected_revision: input.expected_revision,
                host_instance_id: input.host_instance_id,
                created_at: adoption_now(),
            };
            let operation = AutomationMutation::Adopt {
                kind: input.kind,
                id: input.id,
                reviewed_versions: Some(input.reviewed_versions),
            };
            crash("before_intent");
            self.db
                .owner_adoption_begin(service.host_id(), &intent, &operation)
                .map_err(adoption_failure)?;
            crash("after_intent");
            crash("before_dispatch");
            let result = self.automation_command(CommandCaller::LocalOwner, request, operation);
            crash("after_effect");
            match result {
                Ok(_) => {
                    self.db
                        .owner_adoption_resolve(service.host_id(), false, None)
                        .map_err(adoption_failure)?;
                }
                Err(error) if adoption_rejection(error.code) => {
                    self.db
                        .owner_adoption_resolve(service.host_id(), false, Some(error.code))
                        .map_err(adoption_failure)?;
                }
                Err(error) => return Err(error),
            }
            crash("after_record");
            self.db
                .owner_adoption_state(service.host_id(), service.instance_id())
                .map_err(adoption_failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = input;
            Err(OperationError::capability_unavailable(
                "owner adoption requires persistence",
            ))
        }
    }
    pub fn owner_adoption_resolve(&self) -> Result<OwnerAdoptionState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner adoption coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            self.db
                .owner_adoption_resolve(service.host_id(), false, None)
                .map_err(adoption_failure)?;
            self.db
                .owner_adoption_state(service.host_id(), service.instance_id())
                .map_err(adoption_failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::capability_unavailable(
                "owner adoption requires persistence",
            ))
        }
    }
    pub fn owner_adoption_review_unknown(
        &self,
        input: OwnerAdoptionReview,
    ) -> Result<OwnerAdoptionState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            if !input.acknowledge_unknown {
                return Err(adoption_error(
                    "confirmation_required",
                    "review the unresolved original adoption outcome",
                ));
            }
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner adoption coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            let state = self
                .db
                .owner_adoption_state(service.host_id(), service.instance_id())
                .map_err(adoption_failure)?;
            if state.pending.as_ref().map(|i| i.command_id.as_str())
                != Some(input.command_id.as_str())
            {
                return Err(adoption_error(
                    "stale_view",
                    "pending owner adoption changed",
                ));
            }
            // A final lookup wins over a stale client assertion that the result
            // is unknown. Even this path never dispatches the old operation.
            self.db
                .owner_adoption_resolve(service.host_id(), true, None)
                .map_err(adoption_failure)?;
            self.db
                .owner_adoption_state(service.host_id(), service.instance_id())
                .map_err(adoption_failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = input;
            Err(OperationError::capability_unavailable(
                "owner adoption requires persistence",
            ))
        }
    }
}

#[cfg(all(test, feature = "persistence"))]
mod tests;
