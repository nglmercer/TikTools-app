//! Atomic saved-source review for the native Owner approval workflow. These
//! serializable descriptions never construct a trusted execution permit.
use super::{AutomationKind, OperationError};
use crate::{
    collaboration::{Grant, Versioned},
    AppCore,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalReviewParams {
    pub kind: AutomationKind,
    pub id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerApprovalBlockReason {
    DefinitionMissing,
    NoDelegatedSubjects,
    SubjectsUnavailable,
    PolicyUnavailable,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalEligibility {
    pub eligible: bool,
    pub reason_code: Option<OwnerApprovalBlockReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum OwnerApprovalStatusKind {
    None,
    Active,
    Invalidated,
    Revoked,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalStatus {
    pub state: OwnerApprovalStatusKind,
    pub generation: Option<String>,
    pub policy_version: Option<u32>,
    pub manifest_digest: Option<String>,
    pub current: bool,
}
impl Default for OwnerApprovalStatus {
    fn default() -> Self {
        Self {
            state: OwnerApprovalStatusKind::None,
            generation: None,
            policy_version: None,
            manifest_digest: None,
            current: false,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalSubject {
    pub principal_id: String,
    pub session_id: String,
    pub grant_revision: String,
    pub configuration_revision: String,
    pub grant: Grant,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerApprovalReviewResult {
    pub host_id: String,
    pub host_instance_id: String,
    pub kind: AutomationKind,
    pub id: String,
    pub source: Value,
    pub manifest: Option<Value>,
    pub subjects: Vec<OwnerApprovalSubject>,
    pub eligibility: OwnerApprovalEligibility,
    pub approval: OwnerApprovalStatus,
}

impl AppCore {
    pub fn owner_approval_review(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> Result<Versioned<OwnerApprovalReviewResult>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use crate::services::automation_manifest::{
                self as manifest, ManifestSubject, ManifestSubjects,
            };
            use serde_json::json;
            let canonical = super::clean_record_id(id, "automation")?;
            if canonical != id || id.contains('*') || id.chars().any(char::is_control) {
                return Err(OperationError::invalid(
                    "review requires a canonical saved automation ID",
                ));
            }
            let _guard = self.automation_gate()?;
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            let service = self
                .collaboration()
                .map_err(super::commands::service_error)?;
            let (source, saved) = self
                .db
                .behavior_approval_review_versioned(kind.table(), id)
                .map_err(|error| match error {
                    crate::db::DatabaseError::SnapshotTooLarge => review_too_large(),
                    _ => {
                        OperationError::internal("saved automation approval review is unavailable")
                    }
                })?;
            let mut status = saved
                .as_ref()
                .map(|saved| saved.status.clone())
                .unwrap_or_default();
            let mut reason = None;
            let mut preview = None;
            let mut subjects = Vec::new();
            if source.value["record"].is_null() {
                reason = Some(OwnerApprovalBlockReason::DefinitionMissing);
            } else {
                let required =
                    manifest::required_subjects(&source).map_err(|error| match error {
                        manifest::ManifestError::TooLarge => review_too_large(),
                        _ => OperationError::internal("saved automation review is inconsistent"),
                    })?;
                match service
                    .execution_authorizations(&required)
                    .and_then(|batch| batch.snapshot())
                {
                    Err(_) => reason = Some(OwnerApprovalBlockReason::SubjectsUnavailable),
                    Ok(identities) => {
                        let pins: Vec<_> = identities
                            .iter()
                            .map(|identity| ManifestSubject {
                                principal_id: identity.principal_id(),
                                session_id: identity.session_id(),
                                grant_revision: identity.grant_revision(),
                                configuration_revision: identity.configuration_revision(),
                                grant: identity.grant(),
                            })
                            .collect();
                        let value = manifest::build(
                            &source,
                            ManifestSubjects {
                                host_id: service.host_id(),
                                subjects: &pins,
                            },
                        )
                        .map_err(|error| match error {
                            manifest::ManifestError::TooLarge => review_too_large(),
                            _ => OperationError::internal(
                                "saved automation manifest is inconsistent",
                            ),
                        })?;
                        let policy = crate::services::automation_approval_policy::bindings(
                            &value,
                            &identities,
                        );
                        subjects = identities
                            .iter()
                            .map(|identity| OwnerApprovalSubject {
                                principal_id: identity.principal_id().into(),
                                session_id: identity.session_id().into(),
                                grant_revision: identity.grant_revision().into(),
                                configuration_revision: identity.configuration_revision().into(),
                                grant: identity.grant().clone(),
                            })
                            .collect();
                        let value = serde_json::to_value(value).map_err(|_| {
                            OperationError::internal("approval preview could not be encoded")
                        })?;
                        if required.is_empty() {
                            reason = Some(OwnerApprovalBlockReason::NoDelegatedSubjects);
                        } else if let Ok(policy) = policy {
                            status.current = status.state == OwnerApprovalStatusKind::Active
                                && saved.as_ref().is_some_and(|saved| {
                                    saved.binding == json!({"manifest":value,"bindings":policy})
                                });
                        } else {
                            reason = Some(OwnerApprovalBlockReason::PolicyUnavailable);
                        }
                        preview = Some(value);
                    }
                }
            }
            let mut result = source.map(|source| OwnerApprovalReviewResult {
                host_id: service.host_id().into(),
                host_instance_id: service.instance_id().into(),
                kind,
                id: id.into(),
                source,
                manifest: preview,
                subjects,
                eligibility: OwnerApprovalEligibility {
                    eligible: reason.is_none(),
                    reason_code: reason,
                },
                approval: status,
            });
            result.host_instance_id = Some(service.instance_id().into());
            if serde_json::to_vec(&result)
                .map_err(|_| OperationError::internal("approval review could not be encoded"))?
                .len()
                > 4 * 1024 * 1024
            {
                return Err(review_too_large());
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (kind, id);
            Err(OperationError::capability_unavailable(
                "owner approval review requires persistence",
            ))
        }
    }
}
#[cfg(feature = "persistence")]
fn review_too_large() -> OperationError {
    OperationError {
        code: "too_large",
        message: "owner approval review exceeds its source and authorization budget".into(),
        data: None,
    }
}

#[cfg(all(test, feature = "persistence"))]
mod tests;
