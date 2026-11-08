//! Native metadata journal for explicit Owner approvals. Browser callers never
//! choose command identities, and recovery only resolves the original ledger.
#[cfg(feature = "persistence")]
use super::{ApprovalActivateParams, ApprovalRevokeParams};
use super::{ApprovalEnablement, AutomationKind, OperationError};
use crate::{collaboration::ResourceVersion, AppCore};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerApprovalOperation {
    Activate,
    Revoke,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalIntent {
    pub command_id: String,
    pub operation: OwnerApprovalOperation,
    pub kind: AutomationKind,
    pub id: String,
    pub expected_revision: String,
    pub host_instance_id: String,
    pub created_at: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerApprovalOutcome {
    Succeeded,
    Rejected,
    ReviewedUnknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalReceipt {
    pub revision: String,
    pub approval_generation: String,
    pub policy_version: Option<u32>,
    pub manifest_digest: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalRecord {
    pub intent: OwnerApprovalIntent,
    pub outcome: OwnerApprovalOutcome,
    pub receipt: Option<OwnerApprovalReceipt>,
    pub error_code: Option<String>,
    pub recorded_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalState {
    pub host_id: String,
    pub host_instance_id: String,
    pub pending: Option<OwnerApprovalIntent>,
    pub recent: Vec<OwnerApprovalRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "operation",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OwnerApprovalSubmit {
    Activate {
        kind: AutomationKind,
        id: String,
        expected_revision: String,
        host_id: String,
        host_instance_id: String,
        reviewed_versions: Vec<ResourceVersion>,
        manifest_digest: String,
        enable_plan: Vec<ApprovalEnablement>,
        acknowledge_effects: bool,
    },
    Revoke {
        kind: AutomationKind,
        id: String,
        expected_revision: String,
        host_id: String,
        host_instance_id: String,
        acknowledge_revocation: bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalReviewUnknown {
    pub command_id: String,
    pub acknowledge_unknown: bool,
}

#[cfg(feature = "persistence")]
pub(crate) enum ApprovalCommand {
    Activate(ApprovalActivateParams),
    Revoke(ApprovalRevokeParams),
}
#[cfg(feature = "persistence")]
impl ApprovalCommand {
    pub(crate) fn method(&self) -> &'static str {
        match self {
            Self::Activate(_) => "automation.approval.activate",
            Self::Revoke(_) => "automation.approval.revoke",
        }
    }
    pub(crate) fn target(&self) -> (AutomationKind, &str) {
        match self {
            Self::Activate(p) => (p.kind, &p.id),
            Self::Revoke(p) => (p.kind, &p.id),
        }
    }
    pub(crate) fn operation(&self) -> OwnerApprovalOperation {
        match self {
            Self::Activate(_) => OwnerApprovalOperation::Activate,
            Self::Revoke(_) => OwnerApprovalOperation::Revoke,
        }
    }
    pub(crate) fn params(&self) -> serde_json::Value {
        match self {
            Self::Activate(p) => {
                // The business ledger uses its closed internal scope spelling,
                // rather than the public enable-plan `kind` field name.
                let plan: Vec<_> = p.enable_plan.iter().map(|e| serde_json::json!({"scope":e.kind.family().scope(),"id":e.id,"enabled":e.enabled})).collect();
                serde_json::json!({"kind":p.kind,"id":p.id,"reviewedVersions":p.reviewed_versions,"manifestDigest":p.manifest_digest,"enablePlan":plan,"acknowledgeEffects":p.acknowledge_effects})
            }
            Self::Revoke(p) => {
                serde_json::json!({"kind":p.kind,"id":p.id,"acknowledgeRevocation":p.acknowledge_revocation})
            }
        }
    }
}
#[cfg(feature = "persistence")]
fn approval_error(code: &'static str, message: &str) -> OperationError {
    OperationError {
        code,
        message: message.into(),
        data: None,
    }
}
#[cfg(feature = "persistence")]
pub(crate) fn approval_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(9_007_199_254_740_991) as i64
}
#[cfg(feature = "persistence")]
pub(crate) fn approval_failure(error: crate::db::DatabaseError) -> OperationError {
    match error {
        crate::db::DatabaseError::OwnerAdoptionPending => approval_error(
            "command_pending",
            "resolve or explicitly review the pending owner authority transition first",
        ),
        crate::db::DatabaseError::Invalid(_) | crate::db::DatabaseError::Json(_) => approval_error(
            "approval_journal_invalid",
            "owner approval journal requires review",
        ),
        _ => approval_error(
            "approval_journal_unavailable",
            "owner approval journal could not be read or persisted",
        ),
    }
}
#[cfg(feature = "persistence")]
pub(crate) fn approval_rejection(code: &str) -> bool {
    // Ambiguous internal/storage/busy/capacity errors retain the pending intent.
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
#[cfg(all(test, feature = "persistence"))]
pub(crate) fn owner_approval_crash(stage: &str) {
    if std::env::var("TIKTOOLS_OWNER_APPROVAL_CRASH").as_deref() == Ok(stage) {
        std::process::exit(89);
    }
}
#[cfg(all(not(test), feature = "persistence"))]
pub(crate) fn owner_approval_crash(_: &str) {}

impl AppCore {
    pub fn owner_approval_state(&self) -> Result<OwnerApprovalState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner authority coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            self.db
                .owner_approval_state(service.host_id(), service.instance_id())
                .map_err(approval_failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::capability_unavailable(
                "owner approval requires persistence",
            ))
        }
    }
    pub fn owner_approval_submit(
        &self,
        input: OwnerApprovalSubmit,
    ) -> Result<OwnerApprovalState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use crate::collaboration::CommandRequest;
            let (host, instance, expected, operation) = match input {
                OwnerApprovalSubmit::Activate {
                    kind,
                    id,
                    expected_revision,
                    host_id,
                    host_instance_id,
                    reviewed_versions,
                    manifest_digest,
                    enable_plan,
                    acknowledge_effects,
                } => {
                    if !acknowledge_effects {
                        return Err(approval_error(
                            "confirmation_required",
                            "review and acknowledge the delegated effect and enable-state plan",
                        ));
                    }
                    (
                        host_id,
                        host_instance_id,
                        expected_revision,
                        ApprovalCommand::Activate(ApprovalActivateParams {
                            kind,
                            id,
                            reviewed_versions,
                            manifest_digest,
                            enable_plan,
                            acknowledge_effects,
                        }),
                    )
                }
                OwnerApprovalSubmit::Revoke {
                    kind,
                    id,
                    expected_revision,
                    host_id,
                    host_instance_id,
                    acknowledge_revocation,
                } => {
                    if !acknowledge_revocation {
                        return Err(approval_error(
                            "confirmation_required",
                            "acknowledge delegated execution revocation",
                        ));
                    }
                    (
                        host_id,
                        host_instance_id,
                        expected_revision,
                        ApprovalCommand::Revoke(ApprovalRevokeParams {
                            kind,
                            id,
                            acknowledge_revocation,
                        }),
                    )
                }
            };
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner authority coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            if host != service.host_id() || instance != service.instance_id() {
                return Err(approval_error(
                    "host_restarted",
                    "owner host identity changed; review again",
                ));
            }
            crate::db::validate_owner_approval_input(&operation, &expected).map_err(|_| {
                OperationError::invalid("invalid reviewed approval identity or enable-state plan")
            })?;
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| OperationError::internal("command identity is unavailable"))?;
            let command_id = format!(
                "owner-approval-{}",
                bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
            );
            let (kind, id) = operation.target();
            let intent = OwnerApprovalIntent {
                command_id: command_id.clone(),
                operation: operation.operation(),
                kind,
                id: id.into(),
                expected_revision: expected.clone(),
                host_instance_id: instance.clone(),
                created_at: approval_now(),
            };
            let request = CommandRequest {
                command_id,
                host_instance_id: instance,
                expected_revision: expected,
            };
            request
                .validate()
                .map_err(|_| OperationError::invalid("invalid owner approval revision"))?;
            owner_approval_crash("before_intent");
            self.db
                .owner_approval_begin(&host, &intent, &operation)
                .map_err(approval_failure)?;
            owner_approval_crash("after_intent");
            owner_approval_crash("before_dispatch");
            let result = match operation {
                ApprovalCommand::Activate(p) => self.automation_approval_activate(request, p),
                ApprovalCommand::Revoke(p) => self.automation_approval_revoke(request, p),
            };
            owner_approval_crash("after_effect");
            match result {
                Ok(_) => self
                    .db
                    .owner_approval_resolve(&host, false, None)
                    .map_err(approval_failure)?,
                Err(error) if approval_rejection(error.code) => self
                    .db
                    .owner_approval_resolve(&host, false, Some(error.code))
                    .map_err(approval_failure)?,
                Err(error) => return Err(error),
            }
            owner_approval_crash("after_record");
            self.db
                .owner_approval_state(&host, service.instance_id())
                .map_err(approval_failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = input;
            Err(OperationError::capability_unavailable(
                "owner approval requires persistence",
            ))
        }
    }
    pub fn owner_approval_resolve(&self) -> Result<OwnerApprovalState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner authority coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            self.db
                .owner_approval_resolve(service.host_id(), false, None)
                .map_err(approval_failure)?;
            self.db
                .owner_approval_state(service.host_id(), service.instance_id())
                .map_err(approval_failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::capability_unavailable(
                "owner approval requires persistence",
            ))
        }
    }
    pub fn owner_approval_review_unknown(
        &self,
        input: OwnerApprovalReviewUnknown,
    ) -> Result<OwnerApprovalState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            if !input.acknowledge_unknown {
                return Err(approval_error(
                    "confirmation_required",
                    "review the unresolved original approval outcome",
                ));
            }
            let _guard = self.owner_adoption_gate.lock().map_err(|_| {
                OperationError::internal("owner authority coordinator is unavailable")
            })?;
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            let state = self
                .db
                .owner_approval_state(service.host_id(), service.instance_id())
                .map_err(approval_failure)?;
            if state.pending.as_ref().map(|i| i.command_id.as_str())
                != Some(input.command_id.as_str())
            {
                return Err(approval_error(
                    "stale_view",
                    "pending owner approval changed",
                ));
            }
            self.db
                .owner_approval_resolve(service.host_id(), true, None)
                .map_err(approval_failure)?;
            self.db
                .owner_approval_state(service.host_id(), service.instance_id())
                .map_err(approval_failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = input;
            Err(OperationError::capability_unavailable(
                "owner approval requires persistence",
            ))
        }
    }
}
#[cfg(all(test, feature = "persistence"))]
mod tests;
