//! Host-owned command coordination. No caller identity can be deserialized.
use crate::{collaboration::*, control::OperationError, AppCore};
#[cfg(feature = "persistence")]
use serde_json::json;
use serde_json::Value;
use std::sync::Arc;

#[cfg(feature = "persistence")]
#[derive(Default)]
pub(crate) struct CoordinationWorker {
    stopping: tokio_util::sync::CancellationToken,
    task: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

#[derive(Clone)]
pub enum CommandCaller {
    LocalOwner,
    Remote(Authorization),
}

pub enum WorkflowMutation {
    Save(Value),
    Delete(String),
    SetEnabled { id: String, enabled: bool },
}
impl WorkflowMutation {
    #[cfg(feature = "persistence")]
    pub(crate) fn authorize_removal(&self, auth: &Authorization) -> Result<Authenticated, Error> {
        let identity = auth.snapshot()?;
        let (id, capability) = match self {
            Self::Delete(id) => (id, Capability::WorkflowsEdit),
            Self::SetEnabled { id, enabled: false } => (id, Capability::WorkflowsToggle),
            _ => return Err(Error::Forbidden),
        };
        if !identity.grant.capabilities.contains(&capability)
            || !identity
                .grant
                .capabilities
                .contains(&Capability::WorkflowsRead)
            || !identity.grant.allows_resource(ResourceKind::Workflow, id)
        {
            return Err(Error::Forbidden);
        }
        Ok(identity)
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn method(&self) -> &'static str {
        match self {
            Self::Save(_) => "workflows.save",
            Self::Delete(_) => "workflows.delete",
            Self::SetEnabled { enabled: true, .. } => "workflows.enable",
            Self::SetEnabled { enabled: false, .. } => "workflows.disable",
        }
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn params(&self) -> Value {
        match self {
            Self::Save(graph) => json!({"graph":graph}),
            Self::Delete(id) | Self::SetEnabled { id, .. } => json!({"id":id}),
        }
    }
    #[cfg(feature = "persistence")]
    fn resource_id(&self) -> Result<String, OperationError> {
        let id = match self {
            Self::Save(graph) => {
                if !graph
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| !name.trim().is_empty())
                {
                    return Err(OperationError::invalid("workflow name is required"));
                }
                graph
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| OperationError::invalid("workflow ID is required"))?
            }
            Self::Delete(id) | Self::SetEnabled { id, .. } => id,
        };
        let canonical = super::clean_record_id(id, "workflow")?;
        if canonical != id || id.contains('*') || id.chars().any(char::is_control) {
            return Err(OperationError::invalid(
                "workflow command ID must be canonical",
            ));
        }
        Ok(canonical)
    }
}

impl AppCore {
    /// Remote removal/deactivation cannot introduce executable effects. Saving
    /// or activating definitions still requires the delegated-effect policy.
    pub fn workflow_command(
        &self,
        caller: CommandCaller,
        request: CommandRequest,
        operation: WorkflowMutation,
    ) -> Result<Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            if matches!(&caller, CommandCaller::Remote(_))
                && matches!(
                    &operation,
                    WorkflowMutation::Save(_) | WorkflowMutation::SetEnabled { enabled: true, .. }
                )
            {
                return Err(OperationError {
                    code: "unsupported_remote",
                    message: "delegated workflow command policy is unavailable".into(),
                    data: None,
                });
            }
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            request
                .validate()
                .map_err(|_| OperationError::invalid("invalid command metadata"))?;
            let id = operation.resource_id()?;
            let _gate = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            // Opening the identity service also acquires its exclusive workspace
            // lease before inspecting a retained command or accepting a new one.
            let service = self.collaboration().map_err(service_error)?;
            let remote = matches!(&caller, CommandCaller::Remote(_));
            let (principal_id, session_id) = match &caller {
                CommandCaller::LocalOwner => ("local-owner".into(), "local-owner".into()),
                CommandCaller::Remote(auth) => {
                    // Recheck after obtaining the writer gate. Queued work may
                    // have lost its capability, resource scope or session.
                    let identity = operation.authorize_removal(auth).map_err(service_error)?;
                    (identity.principal_id, identity.session_id)
                }
            };
            let command = crate::db::CommandTransaction {
                principal_id,
                session_id,
                authorization: match &caller {
                    CommandCaller::LocalOwner => None,
                    CommandCaller::Remote(auth) => Some(auth.clone()),
                },
                request,
                current_instance_id: service.instance_id().into(),
                max_ids_per_principal: service
                    .configuration()
                    .map_err(service_error)?
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
                .map_err(|error| command_error(error, &command.request.command_id))?;
            let (result, fresh) = self
                .db
                .execute_workflow_command(&operation, &command)
                .map_err(|error| command_error(error, &command.request.command_id))?;
            if fresh {
                let change = match operation {
                    WorkflowMutation::Save(_) => Some("saved"),
                    WorkflowMutation::Delete(_)
                        if result.value.get("deleted") == Some(&Value::Bool(true)) =>
                    {
                        Some("deleted")
                    }
                    WorkflowMutation::Delete(_) => None,
                    WorkflowMutation::SetEnabled { enabled: true, .. } => Some("enabled"),
                    WorkflowMutation::SetEnabled { enabled: false, .. } => Some("disabled"),
                };
                if let Some(change) = change {
                    let event = crate::events::DomainEvent::WorkflowChanged {
                        kind: "workflow".into(),
                        id,
                        change: change.into(),
                    };
                    if remote {
                        self.events.publish_presentation_domain(event);
                    } else {
                        self.events.publish_domain(event);
                    }
                }
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (caller, request, operation);
            Err(OperationError::capability_unavailable(
                "commands require persistence",
            ))
        }
    }

    pub fn collaboration_command_get(
        &self,
        caller: CommandCaller,
        params: CommandGet,
    ) -> Result<CommandOutcome, OperationError> {
        if !valid_command_id(&params.command_id) {
            return Err(OperationError::invalid("invalid command ID"));
        }
        #[cfg(feature = "persistence")]
        {
            let principal = match &caller {
                CommandCaller::LocalOwner => params
                    .principal_id
                    .as_deref()
                    .unwrap_or("local-owner")
                    .to_owned(),
                CommandCaller::Remote(auth) => {
                    let identity = auth.snapshot().map_err(service_error)?;
                    if !identity
                        .grant
                        .capabilities
                        .contains(&Capability::CommandsRead)
                        || !(identity
                            .grant
                            .capabilities
                            .contains(&Capability::WorkflowsRead)
                            || identity
                                .grant
                                .capabilities
                                .contains(&Capability::PointsRead)
                            || identity
                                .grant
                                .capabilities
                                .contains(&Capability::CompetitionsRead)
                                && !identity.grant.resources.contains_key(&ResourceKind::Live)
                            || super::automation::automation_read_allowed(&identity.grant))
                        || params
                            .principal_id
                            .as_ref()
                            .is_some_and(|id| *id != identity.principal_id)
                    {
                        return Err(OperationError::forbidden("command is not available"));
                    }
                    identity.principal_id
                }
            };
            if principal.is_empty()
                || principal.len() > 256
                || principal.chars().any(char::is_control)
            {
                return Err(OperationError::invalid("invalid command principal"));
            }
            let _gate = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
            let _service = self.collaboration().map_err(service_error)?;
            let missing = || {
                if matches!(&caller, CommandCaller::LocalOwner) {
                    OperationError::not_found("command not found")
                } else {
                    OperationError::forbidden("command is not available")
                }
            };
            let target = self
                .db
                .command_resource(&principal, &params.command_id)
                .map_err(|error| command_error(error, &params.command_id))?
                .ok_or_else(missing)?;
            authorize_query(&caller, &target)?;
            let outcome = self.db.command_outcome(&principal, &params.command_id);
            // Expired outcomes and storage errors are also reauthorized. No
            // historical body is read before current resource scope is checked.
            authorize_query(&caller, &target)?;
            let outcome = outcome
                .map_err(|error| {
                    let mut error = command_error(error, &params.command_id);
                    let mut data = error.data.take().unwrap_or_else(|| json!({}));
                    data["principalId"] = json!(principal);
                    data["command"] = json!({"resource":target});
                    error.data = Some(data);
                    error
                })?
                .ok_or_else(missing)?;
            Ok(outcome)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (caller, params);
            Err(OperationError::capability_unavailable(
                "commands require persistence",
            ))
        }
    }

    /// Retention does not require opening the private identity database on a
    /// local-only startup. A weak reference never keeps a stopped core alive.
    pub fn start_coordination_worker(self: &Arc<Self>) {
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            self.start_coordination_worker_on(&runtime);
        }
    }

    /// GUI construction runs outside Tokio's entered context. Use its owned
    /// runtime explicitly; do not silently omit delivery on the desktop host.
    pub fn start_coordination_worker_on(self: &Arc<Self>, runtime: &tokio::runtime::Handle) {
        #[cfg(feature = "persistence")]
        {
            let mut task = tiktools_plugin_api::sync::recover_mutex(
                &self.coordination_worker.task,
                "coordination worker",
            );
            if self.is_shutdown()
                || task.is_some()
                || self.coordination_worker.stopping.is_cancelled()
            {
                return;
            }
            let weak = Arc::downgrade(self);
            let stopping = self.coordination_worker.stopping.clone();
            *task = Some(runtime.spawn(async move {
                let mut clock = tokio::time::interval(std::time::Duration::from_millis(80));
                clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                let mut last_prune = std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_secs(60))
                    .unwrap_or_else(std::time::Instant::now);
                loop {
                    tokio::select! { _=stopping.cancelled()=>break, _=clock.tick()=>{} }
                    let Some(core) = weak.upgrade() else {
                        break;
                    };
                    if core.is_shutdown() {
                        break;
                    }
                    let prune = last_prune.elapsed() >= std::time::Duration::from_secs(60);
                    if prune {
                        last_prune = std::time::Instant::now();
                    }
                    let outcome = tokio::task::spawn_blocking(move || {
                        if prune {
                            core.db.prune_command_outcomes().map_err(|_| {
                                OperationError::internal("command retention is unavailable")
                            })?;
                            core.prune_coordination_audit()?;
                        }
                        core.deliver_coordination()
                    })
                    .await;
                    if !matches!(outcome, Ok(Ok(_))) {
                        tracing::warn!("collaboration delivery or retention is unavailable");
                        // An unavailable database must not turn an 80-ms idle
                        // clock into a tight failure/logging loop.
                        tokio::select! {
                            _ = stopping.cancelled() => break,
                            _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
                        }
                    }
                }
            }));
        }
        #[cfg(not(feature = "persistence"))]
        let _ = runtime;
    }
    #[cfg(feature = "persistence")]
    pub(crate) async fn stop_coordination(self: &Arc<Self>) {
        self.coordination_worker.stopping.cancel();
        let task = tiktools_plugin_api::sync::recover_mutex(
            &self.coordination_worker.task,
            "coordination worker",
        )
        .take();
        if let Some(task) = task {
            let _ = task.await;
        }
        // Commands already executing on the blocking pool finish their atomic
        // commit before the core reports shutdown complete. New ones recheck
        // shutdown after obtaining this same gate.
        let core = Arc::clone(self);
        if tokio::task::spawn_blocking(move || {
            drop(tiktools_plugin_api::sync::recover_mutex(
                &core.coordination_gate,
                "command coordination",
            ))
        })
        .await
        .is_err()
        {
            self.shutdown_update_safe
                .store(false, std::sync::atomic::Ordering::Release);
        }
    }
}

#[cfg(feature = "persistence")]
fn authorize_query(caller: &CommandCaller, target: &CommandResource) -> Result<(), OperationError> {
    if let CommandCaller::Remote(auth) = caller {
        let identity = auth.snapshot().map_err(service_error)?;
        if !identity
            .grant
            .capabilities
            .contains(&Capability::CommandsRead)
            || !match target.kind {
                ResourceKind::Workflow => identity
                    .grant
                    .capabilities
                    .contains(&Capability::WorkflowsRead),
                ResourceKind::Viewer => super::points::points_read_allowed(&identity.grant),
                ResourceKind::Automation => {
                    crate::collaboration::AutomationFamily::from_scope(&target.scope).is_some()
                        && super::automation::automation_read_allowed(&identity.grant)
                }
                ResourceKind::Competition => {
                    identity
                        .grant
                        .capabilities
                        .contains(&Capability::CompetitionsRead)
                        && !identity.grant.resources.contains_key(&ResourceKind::Live)
                }
                _ => false,
            }
            || !identity
                .grant
                .allows_scoped_resource(target.kind, &target.scope, &target.id)
        {
            return Err(OperationError::forbidden("command is not available"));
        }
    }
    Ok(())
}

#[cfg(feature = "persistence")]
pub(super) fn service_error(error: crate::collaboration::Error) -> OperationError {
    OperationError {
        code: error.code(),
        message: error.to_string(),
        data: None,
    }
}
#[cfg(feature = "persistence")]
pub(super) fn command_error(error: crate::db::DatabaseError, id: &str) -> OperationError {
    use crate::db::DatabaseError;
    let code = match error {
        DatabaseError::CommandAuthorization(error) => return service_error(error),
        DatabaseError::CommandIdReused => "command_id_reused",
        DatabaseError::CommandExpired => "command_expired",
        DatabaseError::HostRestarted => "host_restarted",
        DatabaseError::CommandCapacity => "overloaded",
        DatabaseError::CurrencyChanged => "currency_changed",
        DatabaseError::PointsWriterUnavailable => {
            return OperationError {
                code: "overloaded",
                message: "points writer is not ready to accept a command".into(),
                data: Some(json!({"commandId":id})),
            }
        }
        DatabaseError::SnapshotTooLarge => {
            return OperationError {
                code: "too_large",
                message: "points record exceeds its bounded budget".into(),
                data: None,
            }
        }
        other => return super::workflows::workflow_error(other),
    };
    OperationError {
        code,
        message: match code {
            "command_id_reused" => "command ID was used for another operation",
            "command_expired" => "historical command outcome is unknown",
            "host_restarted" => "host restarted; obtain a fresh baseline",
            "currency_changed" => "points currency changed; review it before adjusting",
            _ => "command history capacity reached",
        }
        .into(),
        data: Some(if code == "command_expired" {
            json!({"commandId":id,"state":CommandState::Unknown,"outcomeExpired":true})
        } else {
            json!({"commandId":id})
        }),
    }
}
