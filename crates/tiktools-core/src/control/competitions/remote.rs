//! Revision-bound remote lifecycle commands. Coordination, ranking and competition gates
//! retain their original order through admission, durable commit and fresh-event publication.

use super::CompetitionRemoteTransition;
use crate::{control::OperationError, AppCore};
#[cfg(feature = "persistence")]
use crate::{
    control::{clean_record_id, commands},
    recover_mutex,
};

impl AppCore {
    /// Atomic, recoverable stop transition for a scoped remote controller.
    pub fn competition_remote_command(
        self: &std::sync::Arc<Self>,
        auth: crate::collaboration::Authorization,
        request: crate::collaboration::CommandRequest,
        id: &str,
        generation: &str,
        transition: CompetitionRemoteTransition,
    ) -> Result<crate::collaboration::Versioned<serde_json::Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use crate::collaboration::{Capability, ResourceKind};
            let clean = clean_record_id(id, "competition")?;
            if clean != id || id.contains('*') || id.chars().any(char::is_control) {
                return Err(OperationError::invalid("invalid competition ID"));
            }
            request
                .validate()
                .map_err(|_| OperationError::invalid("invalid command metadata"))?;
            let generation = generation
                .parse::<u64>()
                .ok()
                .filter(|n| n.to_string() == generation && *n <= i64::MAX as u64)
                .ok_or_else(|| OperationError::invalid("invalid competition generation"))?;
            let _gate = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
            if self.is_shutdown() {
                return Err(OperationError::unavailable("host is shutting down"));
            }
            let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
            let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
            if self.competitions.load_failed {
                return Err(OperationError::unavailable(
                    "competition definitions unavailable",
                ));
            }
            let identity = auth.snapshot().map_err(commands::service_error)?;
            if !identity
                .grant
                .capabilities
                .contains(&Capability::CompetitionsRead)
                || !identity
                    .grant
                    .capabilities
                    .contains(&Capability::CompetitionsManage)
                || identity.grant.resources.contains_key(&ResourceKind::Live)
                || !identity
                    .grant
                    .allows_resource(ResourceKind::Competition, id)
            {
                return Err(OperationError::forbidden("competition command unavailable"));
            }
            let service = self.collaboration().map_err(commands::service_error)?;
            let command = crate::db::CommandTransaction {
                principal_id: identity.principal_id,
                session_id: identity.session_id,
                authorization: Some(auth),
                request,
                current_instance_id: service.instance_id().into(),
                max_ids_per_principal: service
                    .configuration()
                    .map_err(commands::service_error)?
                    .limits
                    .command_ids_per_principal,
            };
            self.db
                .check_command_admission(
                    crate::db::CommandDatabase::Automation,
                    &command.principal_id,
                    &command.request.command_id,
                    command.max_ids_per_principal,
                )
                .map_err(|e| commands::command_error(e, &command.request.command_id))?;
            let operation = match transition {
                CompetitionRemoteTransition::Start => Some(crate::db::CompetitionActivation::Start),
                CompetitionRemoteTransition::Resume => {
                    Some(crate::db::CompetitionActivation::Resume)
                }
                _ => None,
            };
            let outcome = if let Some(operation) = operation {
                let definition = self.competition_get(id)?;
                let ranking_paused = self.ranking_snapshot(&definition.ranking_id)?.paused;
                let owner = self
                    .aggregations
                    .checkpoint_owner
                    .as_deref()
                    .ok_or_else(|| {
                        OperationError::unavailable("competition integrity owner unavailable")
                    })?;
                self.db.execute_competition_activation(
                    id,
                    generation,
                    operation,
                    &command,
                    &service,
                    owner,
                    ranking_paused,
                )
            } else {
                self.db.execute_competition_stop(
                    id,
                    generation,
                    transition,
                    &command,
                    &service,
                    self.aggregations.checkpoint_owner.as_deref(),
                )
            };
            let (result, fresh) = outcome.map_err(|e| match e {
                crate::db::DatabaseError::RevisionConflict(version) => OperationError {
                    code: "revision_conflict",
                    message: "competition changed; refresh its status".into(),
                    data: Some(serde_json::json!({"resourceVersions":[version]})),
                },
                crate::db::DatabaseError::Invalid(reason)
                    if reason == "competition ranking is paused" =>
                {
                    OperationError::conflict("resume the competition ranking first")
                }
                other => commands::command_error(other, &command.request.command_id),
            })?;
            if fresh {
                let resulting_generation = result
                    .value
                    .get("generation")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(generation);
                self.publish_remote_competition_transition(
                    id,
                    resulting_generation,
                    transition,
                    &command.principal_id,
                    &command.session_id,
                );
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (auth, request, id, generation, transition);
            Err(OperationError::unavailable(
                "competition commands require persistence",
            ))
        }
    }
    #[cfg(feature = "persistence")]
    fn publish_remote_competition_transition(
        self: &std::sync::Arc<Self>,
        id: &str,
        generation: u64,
        transition: CompetitionRemoteTransition,
        principal_id: &str,
        session_id: &str,
    ) {
        if let Some(data) = self.competition_context(id, generation) {
            match transition {
                CompetitionRemoteTransition::Start => {
                    self.publish_competition_event("competition.started", data)
                }
                CompetitionRemoteTransition::Resume => {
                    self.publish_competition_event("competition.resumed", data)
                }
                CompetitionRemoteTransition::Pause | CompetitionRemoteTransition::Cancel => {
                    let retained = self
                        .db
                        .competition_retains_operator(id, generation, principal_id, session_id)
                        .unwrap_or(false);
                    if retained {
                        self.publish_competition_event(
                            if matches!(transition, CompetitionRemoteTransition::Pause) {
                                "competition.paused"
                            } else {
                                "competition.cancelled"
                            },
                            data,
                        );
                    } else {
                        self.events.publish_presentation_domain(
                            if matches!(transition, CompetitionRemoteTransition::Pause) {
                                crate::events::DomainEvent::CompetitionPaused(data)
                            } else {
                                crate::events::DomainEvent::CompetitionCancelled(data)
                            },
                        );
                    }
                }
            }
        }
    }
}
