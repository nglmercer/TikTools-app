//! Behavior commands share IDs/capacity with workflows and points. Remote
//! callers may remove/deactivate complete events, author disabled drafts, or
//! create, edit and activate definitions under a current reusable delegation.
use super::*;
use crate::{collaboration::*, control::CommandCaller};

pub enum AutomationMutation {
    Adopt {
        kind: AutomationKind,
        id: String,
        reviewed_versions: Option<Vec<ResourceVersion>>,
    },
    Create {
        kind: AutomationKind,
        record: Value,
    },
    Update {
        kind: AutomationKind,
        id: String,
        record: Value,
    },
    Delete {
        kind: AutomationKind,
        id: String,
    },
    SetEnabled {
        kind: AutomationKind,
        id: String,
        enabled: bool,
    },
}
impl AutomationMutation {
    #[cfg(feature = "persistence")]
    pub(crate) fn is_remote_deactivation(&self) -> bool {
        matches!(
            self,
            Self::Delete {
                kind: AutomationKind::Event,
                ..
            } | Self::SetEnabled {
                kind: AutomationKind::Event,
                enabled: false,
                ..
            }
        )
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn is_remote_activation(&self) -> bool {
        matches!(self, Self::SetEnabled { enabled: true, .. })
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn is_reusable_write(&self) -> bool {
        matches!(self, Self::Create { record, .. } | Self::Update { record, .. } if record.get("enabled") == Some(&Value::Bool(true)))
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn is_remote_draft(&self) -> bool {
        matches!(self,Self::Create{record,..}|Self::Update{record,..} if record.get("enabled")==Some(&Value::Bool(false)))
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn is_remote_draft_update(&self) -> bool {
        matches!(self, Self::Update { record, .. } if record.get("enabled") == Some(&Value::Bool(false)))
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn authorize_remote(&self, auth: &Authorization) -> Result<Authenticated, Error> {
        if !self.is_remote_deactivation()
            && !self.is_remote_draft()
            && !self.is_remote_activation()
            && !self.is_reusable_write()
        {
            return Err(Error::Forbidden);
        }
        let identity = auth.snapshot()?;
        let (kind, id) = self.target();
        let capability = if matches!(
            self,
            Self::Delete { .. } | Self::Create { .. } | Self::Update { .. }
        ) {
            Capability::AutomationEdit
        } else {
            Capability::AutomationToggle
        };
        if !identity
            .grant
            .capabilities
            .contains(&Capability::AutomationRead)
            || !identity.grant.capabilities.contains(&capability)
            || !identity.grant.allows_automation(kind.family(), id)
            || identity.grant.resources.contains_key(&ResourceKind::Live)
        {
            return Err(Error::Forbidden);
        }
        Ok(identity)
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn normalize(mut self) -> Result<Self, OperationError> {
        match &mut self {
            Self::Create { kind, record } => {
                let object = record.as_object_mut().ok_or_else(|| {
                    OperationError::invalid("automation record must be an object")
                })?;
                if object
                    .get("name")
                    .and_then(Value::as_str)
                    .is_none_or(|s| s.trim().is_empty())
                {
                    object.insert(
                        "name".into(),
                        match kind {
                            AutomationKind::Action => "Untitled action",
                            AutomationKind::Event => "Untitled event",
                        }
                        .into(),
                    );
                }
            }
            Self::Update { id, record, .. } => {
                record
                    .as_object_mut()
                    .ok_or_else(|| OperationError::invalid("automation record must be an object"))?
                    .insert("id".into(), id.clone().into());
            }
            _ => {}
        }
        let (kind, id) = self.target();
        if clean_record_id(id, kind.label())? != id
            || id.contains('*')
            || id.chars().any(char::is_control)
        {
            return Err(OperationError::invalid(
                "automation command ID must be explicit and canonical",
            ));
        }
        Ok(self)
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn target(&self) -> (AutomationKind, &str) {
        match self {
            Self::Create { kind, record } => (
                *kind,
                record.get("id").and_then(Value::as_str).unwrap_or_default(),
            ),
            Self::Update { kind, id, .. }
            | Self::Delete { kind, id }
            | Self::SetEnabled { kind, id, .. }
            | Self::Adopt { kind, id, .. } => (*kind, id),
        }
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn method(&self) -> &'static str {
        match self {
            Self::Adopt { .. } => "automation.adopt",
            Self::Create { .. } => "automation.create",
            Self::Update { .. } => "automation.update",
            Self::Delete { .. } => "automation.delete",
            Self::SetEnabled { enabled: true, .. } => "automation.enable",
            Self::SetEnabled { enabled: false, .. } => "automation.disable",
        }
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn params(&self) -> Value {
        let (kind, id) = self.target();
        match self {
            Self::Adopt {
                reviewed_versions: Some(versions),
                ..
            } => json!({"kind":kind,"id":id,"reviewedVersions":versions}),
            Self::Create { record, .. } => json!({"kind":kind,"record":record}),
            Self::Update { record, .. } => json!({"kind":kind,"id":id,"record":record}),
            _ => json!({"kind":kind,"id":id}),
        }
    }
}

impl AppCore {
    pub fn automation_review(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> Result<Versioned<Value>, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        #[cfg(feature = "persistence")]
        {
            self.db
                .behavior_review_versioned(kind.table(), &id)
                .map_err(|error| match error {
                    crate::db::DatabaseError::SnapshotTooLarge => OperationError {
                        code: "too_large",
                        message: "automation review exceeds its definition or dependency budget"
                            .into(),
                        data: None,
                    },
                    other => versioned::automation_error(other),
                })
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (kind, id);
            Err(OperationError::capability_unavailable(
                "review requires persistence",
            ))
        }
    }
    pub fn automation_authority(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> Result<Versioned<Value>, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        #[cfg(feature = "persistence")]
        {
            self.db
                .behavior_provenance_versioned(kind.table(), &id)
                .map(|v| v.map(|provenance| json!({"kind":kind,"id":id,"provenance":provenance.map(|p|json!({"authority":p.authority,"generation":p.generation.to_string()}))})))
                .map_err(versioned::automation_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (kind, id);
            Err(OperationError::capability_unavailable(
                "authority requires persistence",
            ))
        }
    }
    pub fn automation_command(
        &self,
        caller: CommandCaller,
        request: CommandRequest,
        operation: AutomationMutation,
    ) -> Result<Versioned<Value>, OperationError> {
        self.automation_command_with_budget(caller, request, operation, false)
    }

    #[cfg(feature = "persistence")]
    pub(crate) fn automation_definition_command(
        &self,
        request: CommandRequest,
        operation: AutomationMutation,
    ) -> Result<Versioned<Value>, OperationError> {
        self.automation_command_with_budget(CommandCaller::LocalOwner, request, operation, true)
    }

    fn automation_command_with_budget(
        &self,
        caller: CommandCaller,
        request: CommandRequest,
        operation: AutomationMutation,
        native_definition: bool,
    ) -> Result<Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            if matches!(&caller, CommandCaller::Remote(_))
                && !operation.is_remote_deactivation()
                && !operation.is_remote_draft()
                && !operation.is_remote_activation()
                && !operation.is_reusable_write()
            {
                return Err(OperationError::forbidden(
                    "remote action/definition execution policy is unavailable",
                ));
            }
            request
                .validate()
                .map_err(|_| OperationError::invalid("invalid command metadata"))?;
            let operation = operation.normalize()?;
            let (kind, target) = operation.target();
            let id = target.to_owned();
            // Fixed order: shared command namespace/publication, then behavior
            // writer/index admission. Legacy behavior writers take only the latter.
            let _coordination = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
            let _writer = self.automation_gate()?;
            self.ensure_automation_writable(Some(&request.expected_revision))?;
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            let remote = matches!(&caller, CommandCaller::Remote(_));
            let (principal_id, session_id, authorization) = match caller {
                CommandCaller::LocalOwner => ("local-owner".into(), "local-owner".into(), None),
                CommandCaller::Remote(auth) => {
                    let identity = operation
                        .authorize_remote(&auth)
                        .map_err(super::super::commands::service_error)?;
                    (identity.principal_id, identity.session_id, Some(auth))
                }
            };
            let command = crate::db::CommandTransaction {
                principal_id,
                session_id,
                authorization,
                request,
                current_instance_id: service.instance_id().into(),
                max_ids_per_principal: service
                    .configuration()
                    .map_err(super::super::commands::service_error)?
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
                .map_err(|e| behavior_command_error(e, &command.request.command_id))?;
            if native_definition {
                // Bound the acknowledgement/recovery before changing source.
                // Both command admission and the behavior writer remain held.
                self.db
                    .owner_definition_validate_dispatch(
                        &operation,
                        &command.request.expected_revision,
                    )
                    .map_err(|error| match error {
                        crate::db::DatabaseError::SnapshotTooLarge => OperationError {
                            code: "too_large",
                            message: "definition result exceeds the native recovery budget".into(),
                            data: None,
                        },
                        other => behavior_command_error(other, &command.request.command_id),
                    })?;
            }
            let owner_draft = if remote && operation.is_remote_draft_update() {
                let (kind, id) = operation.target();
                let permitted = command
                    .authorization
                    .as_ref()
                    .and_then(|auth| auth.snapshot().ok())
                    .and_then(|identity| identity.grant.automation_delegation)
                    .is_some_and(|policy| policy.allow_owner_edits);
                permitted
                    && self.db.is_owner_behavior(kind, id).map_err(|error| {
                        behavior_command_error(error, &command.request.command_id)
                    })?
            } else {
                false
            };
            let (result, fresh) = if remote
                && (operation.is_remote_activation()
                    || operation.is_reusable_write()
                    || owner_draft)
            {
                self.db
                    .execute_reusable_behavior_activation(&operation, &command, &service)
            } else {
                self.db.execute_behavior_command(&operation, &command)
            }
            .map_err(|e| behavior_command_error(e, &command.request.command_id))?;
            if fresh {
                // Commit + runtime index installation is one admission barrier;
                // metadata replay never reinstalls a historical definition.
                self.refresh_automation_snapshot_locked()?;
                let change = match &operation {
                    AutomationMutation::Adopt { .. } => "adopted",
                    AutomationMutation::Create { .. } => "created",
                    AutomationMutation::Update { .. } => "updated",
                    AutomationMutation::Delete { .. } => "deleted",
                    AutomationMutation::SetEnabled { enabled: true, .. } => "enabled",
                    AutomationMutation::SetEnabled { enabled: false, .. } => "disabled",
                };
                if remote {
                    self.events
                        .publish_presentation_domain(DomainEvent::WorkflowChanged {
                            kind: kind.label().into(),
                            id: id.clone(),
                            change: change.into(),
                        });
                } else {
                    self.automation_changed(kind, &id, change);
                }
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (caller, request, operation, native_definition);
            Err(OperationError::capability_unavailable(
                "automation commands require persistence",
            ))
        }
    }
}

#[cfg(feature = "persistence")]
pub(super) fn behavior_command_error(error: crate::db::DatabaseError, id: &str) -> OperationError {
    match error {
        crate::db::DatabaseError::Invalid(_)
        | crate::db::DatabaseError::NotFound
        | crate::db::DatabaseError::RevisionConflict(_) => versioned::automation_error(error),
        other => super::super::commands::command_error(other, id),
    }
}

impl AppCore {
    /// Check that a shared source response still names the current authorized revision.
    pub fn automation_shared_source_current(
        &self,
        kind: AutomationKind,
        id: &str,
        revision: &str,
        auth: &Authorization,
    ) -> Result<bool, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let id = clean_record_id(id, kind.label())?;
            self.db
                .behavior_shared_source_current(kind.family(), &id, revision, auth)
                .map_err(|error| behavior_command_error(error, "source-read"))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (kind, id, revision, auth);
            Err(OperationError::capability_unavailable(
                "source sharing requires persistence",
            ))
        }
    }

    /// Fetch only a definition explicitly shared with this authenticated identity.
    pub fn automation_shared_source(
        &self,
        kind: AutomationKind,
        id: &str,
        auth: &Authorization,
        budget: usize,
    ) -> Result<Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let id = clean_record_id(id, kind.label())?;
            self.db
                .behavior_shared_source(kind.family(), &id, auth, budget)
                .map_err(|error| behavior_command_error(error, "source-read"))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (kind, id, auth, budget);
            Err(OperationError::capability_unavailable(
                "source sharing requires persistence",
            ))
        }
    }
}
