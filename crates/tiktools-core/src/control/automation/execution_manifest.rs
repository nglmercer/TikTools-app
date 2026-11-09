//! Owner-only execution review. A manifest is a description, never an approval
//! or an executable authority token. Source comes from one trusted DB snapshot.
use super::*;

impl AppCore {
    /// Advisory owner-side check of current reusable grants. It does not
    /// activate a definition or grant authority to a subsequent dispatch.
    pub fn automation_delegation_preflight(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> Result<crate::collaboration::Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use crate::services::automation_manifest::{
                self as manifest, ManifestSubject, ManifestSubjects,
            };
            let review = self.automation_review(kind, id)?;
            let required = manifest::required_subjects(&review).map_err(manifest_error)?;
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            let identities = service
                .execution_authorizations(&required)
                .and_then(|authorization| authorization.snapshot())
                .map_err(super::super::commands::service_error)?;
            let subjects: Vec<_> = identities
                .iter()
                .map(|identity| ManifestSubject {
                    principal_id: identity.principal_id(),
                    session_id: identity.session_id(),
                    grant_revision: identity.grant_revision(),
                    configuration_revision: identity.configuration_revision(),
                    grant: identity.grant(),
                })
                .collect();
            let candidate = manifest::build(
                &review,
                ManifestSubjects {
                    host_id: service.host_id(),
                    subjects: &subjects,
                },
            )
            .map_err(manifest_error)?;
            let result = crate::services::automation_approval_policy::reusable_preflight(
                &candidate,
                &identities,
            );
            Ok(review.map(|_| match result {
                Ok(()) => serde_json::json!({"eligible": true, "reason": null, "executionAuthorized":false}),
                Err(reason) => serde_json::json!({"eligible": false, "reason": reason, "executionAuthorized":false}),
            }))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (kind, id);
            Err(OperationError::capability_unavailable(
                "preflight requires persistence",
            ))
        }
    }

    pub fn automation_execution_manifest(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> Result<crate::collaboration::Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use crate::services::automation_manifest::{
                self as manifest, ManifestSubject, ManifestSubjects,
            };
            let review = self.automation_review(kind, id)?;
            let required = manifest::required_subjects(&review).map_err(manifest_error)?;
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            // Resolve only identities found in trusted provenance, never names
            // or grant-looking fields from the editable definition. The batch
            // is all-or-nothing and reads current persisted host authorization.
            // No activity touch means preview cannot extend a remote session.
            let identities = service
                .execution_authorizations(&required)
                .and_then(|authorization| authorization.snapshot())
                .map_err(super::super::commands::service_error)?;
            let subjects: Vec<_> = identities
                .iter()
                .map(|identity| ManifestSubject {
                    principal_id: identity.principal_id(),
                    session_id: identity.session_id(),
                    grant_revision: identity.grant_revision(),
                    configuration_revision: identity.configuration_revision(),
                    grant: identity.grant(),
                })
                .collect();
            let manifest = manifest::build(
                &review,
                ManifestSubjects {
                    host_id: service.host_id(),
                    subjects: &subjects,
                },
            )
            .map_err(manifest_error)?;
            let value = serde_json::to_value(manifest)
                .map_err(|_| OperationError::internal("execution manifest could not be encoded"))?;
            Ok(review.map(|_| value))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (kind, id);
            Err(OperationError::capability_unavailable(
                "execution manifest requires persistence",
            ))
        }
    }
}

#[cfg(feature = "persistence")]
fn manifest_error(error: crate::services::automation_manifest::ManifestError) -> OperationError {
    // Do not reflect definition/config/script text in an error or audit entry.
    use crate::services::automation_manifest::ManifestError;
    match error {
        ManifestError::TooLarge => OperationError {
            code: "too_large",
            message: "execution manifest exceeds its bounded review budget".into(),
            data: None,
        },
        ManifestError::MissingPrimary => {
            OperationError::not_found("saved automation definition does not exist")
        }
        ManifestError::InvalidReview | ManifestError::InvalidSubjects => {
            OperationError::invalid("saved execution review or authorization bindings are invalid")
        }
    }
}

#[cfg(all(test, feature = "persistence"))]
mod tests;
