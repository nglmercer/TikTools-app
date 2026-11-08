//! Explicit Owner approval preserves delegated provenance and binds the final
//! post-activation closure. Durable replay never reinstalls an older permit.
use super::*;
use crate::collaboration::{CommandRequest, Versioned};
use crate::control::{ApprovalActivateParams, ApprovalRevokeParams};

impl AppCore {
    pub fn automation_approval_activate(
        &self,
        request: CommandRequest,
        params: ApprovalActivateParams,
    ) -> Result<Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use crate::db::{
                BehaviorApprovalActivation, BehaviorApprovalBinding, BehaviorApprovalEnableState,
                DatabaseError,
            };
            use crate::services::automation_manifest::{
                self as manifest, ManifestSubject, ManifestSubjects,
            };
            request
                .validate()
                .map_err(|_| OperationError::invalid("invalid command metadata"))?;
            let _coordination = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
            let _writer = self.automation_gate()?;
            self.ensure_automation_writable(Some(&request.expected_revision))?;
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            let command = self.approval_owner_command(request, &service)?;
            self.db
                .check_command_admission(
                    crate::db::CommandDatabase::Automation,
                    &command.principal_id,
                    &command.request.command_id,
                    command.max_ids_per_principal,
                )
                .map_err(|error| {
                    commands::behavior_command_error(error, &command.request.command_id)
                })?;
            let states: Vec<_> = params
                .enable_plan
                .iter()
                .map(|plan| BehaviorApprovalEnableState {
                    scope: plan.kind.family().scope().into(),
                    id: plan.id.clone(),
                    enabled: plan.enabled,
                })
                .collect();
            let activation = BehaviorApprovalActivation {
                table: params.kind.table(),
                id: &params.id,
                reviewed_versions: &params.reviewed_versions,
                preview_digest: &params.manifest_digest,
                enable_states: &states,
                acknowledged: params.acknowledge_effects,
            };
            let (result, fresh) = self
                .db
                .execute_behavior_approval_activation(
                    &activation,
                    &command,
                    |review| {
                        let pairs = manifest::required_subjects(review).map_err(|_| {
                            DatabaseError::Invalid("invalid execution review".into())
                        })?;
                        service
                            .execution_authorizations(&pairs)
                            .map_err(DatabaseError::CommandAuthorization)
                    },
                    |review, identities| {
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
                            review,
                            ManifestSubjects {
                                host_id: service.host_id(),
                                subjects: &subjects,
                            },
                        )
                        .map_err(|_| DatabaseError::Invalid("invalid execution review".into()))?;
                        let bindings = crate::services::automation_approval_policy::bindings(
                            &manifest, identities,
                        )
                        .map_err(|_| {
                            DatabaseError::Invalid(
                                "execution approval policy is unavailable for this closure".into(),
                            )
                        })?;
                        Ok(BehaviorApprovalBinding {
                            manifest: serde_json::to_value(manifest)?,
                            bindings,
                        })
                    },
                )
                .map_err(|error| {
                    commands::behavior_command_error(error, &command.request.command_id)
                })?;
            if fresh {
                self.refresh_automation_snapshot_locked()?;
                self.automation_changed(params.kind, &params.id, "approved");
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (request, params);
            Err(OperationError::capability_unavailable(
                "execution approval requires persistence",
            ))
        }
    }

    pub fn automation_approval_revoke(
        &self,
        request: CommandRequest,
        params: ApprovalRevokeParams,
    ) -> Result<Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            request
                .validate()
                .map_err(|_| OperationError::invalid("invalid command metadata"))?;
            let _coordination = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
            let _writer = self.automation_gate()?;
            self.ensure_automation_writable(Some(&request.expected_revision))?;
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            let command = self.approval_owner_command(request, &service)?;
            self.db
                .check_command_admission(
                    crate::db::CommandDatabase::Automation,
                    &command.principal_id,
                    &command.request.command_id,
                    command.max_ids_per_principal,
                )
                .map_err(|error| {
                    commands::behavior_command_error(error, &command.request.command_id)
                })?;
            let (result, fresh) = self
                .db
                .execute_behavior_approval_revocation(
                    params.kind.table(),
                    &params.id,
                    params.acknowledge_revocation,
                    &command,
                )
                .map_err(|error| {
                    commands::behavior_command_error(error, &command.request.command_id)
                })?;
            if fresh {
                self.refresh_automation_snapshot_locked()?;
                self.automation_changed(params.kind, &params.id, "approval-revoked");
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (request, params);
            Err(OperationError::capability_unavailable(
                "execution approval requires persistence",
            ))
        }
    }

    #[cfg(feature = "persistence")]
    fn approval_owner_command(
        &self,
        request: CommandRequest,
        service: &crate::collaboration::CollaborationService,
    ) -> Result<crate::db::CommandTransaction, OperationError> {
        Ok(crate::db::CommandTransaction {
            principal_id: "local-owner".into(),
            session_id: "local-owner".into(),
            authorization: None,
            request,
            current_instance_id: service.instance_id().into(),
            max_ids_per_principal: service
                .configuration()
                .map_err(super::super::commands::service_error)?
                .limits
                .command_ids_per_principal,
        })
    }
}

#[cfg(all(test, feature = "persistence"))]
mod tests;
