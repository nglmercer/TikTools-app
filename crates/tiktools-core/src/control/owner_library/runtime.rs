use super::*;
use crate::AppCore;

#[cfg(feature = "persistence")]
pub(crate) fn library_now() -> i64 {
    super::super::owner_approval::approval_now()
}

#[cfg(feature = "persistence")]
pub(crate) fn library_rejection(code: &str) -> bool {
    super::super::owner_definition::definition_rejection(code)
}

#[cfg(feature = "persistence")]
fn failure(error: crate::db::DatabaseError) -> OperationError {
    let (code, message) = match error {
        crate::db::DatabaseError::OwnerAdoptionPending => (
            "library_pending",
            "another Owner change has an unresolved outcome",
        ),
        crate::db::DatabaseError::SnapshotTooLarge => (
            "too_large",
            "automation library exceeds its bounded snapshot or result budget",
        ),
        _ => (
            "library_journal_unavailable",
            "automation library recovery could not be read or persisted",
        ),
    };
    OperationError {
        code,
        message: message.into(),
        data: None,
    }
}

#[cfg(feature = "persistence")]
fn crash(stage: &str) {
    #[cfg(test)]
    if std::env::var("TIKTOOLS_OWNER_LIBRARY_CRASH").as_deref() == Ok(stage) {
        std::process::exit(90);
    }
    #[cfg(not(test))]
    let _ = stage;
}

#[cfg(feature = "persistence")]
fn fresh_acknowledgement(
    intent: &OwnerLibraryIntent,
    state: &OwnerLibraryState,
    result: crate::collaboration::Versioned<Value>,
) -> Result<OwnerLibraryAcknowledgement, OperationError> {
    let invalid = || {
        failure(crate::db::DatabaseError::Invalid(
            "unverified library acknowledgement".into(),
        ))
    };
    let row = state
        .recent
        .first()
        .filter(|row| row.intent == *intent)
        .ok_or_else(invalid)?;
    let receipt = row.receipt.as_ref().ok_or_else(invalid)?;
    let value: OwnerLibraryReceipt = serde_json::from_value(result.value).map_err(|_| invalid())?;
    let command = result.command.ok_or_else(invalid)?;
    let primary = result.resource_versions.first().ok_or_else(invalid)?;
    if state.pending.is_some()
        || row.outcome != OwnerLibraryOutcome::Succeeded
        || row.error_code.is_some()
        || value != *receipt
        || command.command_id != intent.command_id
        || command.state != crate::collaboration::CommandState::Succeeded
        || !command
            .committed_at
            .is_some_and(|at| (1..=9_007_199_254_740_991).contains(&at))
        || !command.resource.as_ref().is_some_and(|resource| {
            resource.kind == crate::collaboration::ResourceKind::Automation
                && resource.scope == "library"
                && resource.id == "catalog"
        })
        || primary.kind != crate::collaboration::ResourceKind::Automation
        || primary.scope != "library"
        || primary.id != "catalog"
        || primary.revision != receipt.revision
        || result.host_instance_id.as_deref() != Some(intent.host_instance_id.as_str())
        || usize::try_from(receipt.affected_resources).ok() != Some(result.resource_versions.len())
        || receipt.versions_digest != owner_library_versions_digest(&result.resource_versions)
    {
        return Err(invalid());
    }
    Ok(OwnerLibraryAcknowledgement {
        value,
        resource_versions: result.resource_versions,
        command,
        host_instance_id: intent.host_instance_id.clone(),
    })
}

impl AppCore {
    #[cfg(feature = "persistence")]
    pub(crate) fn library_app_state_projection(
        &self,
        keys: Option<&[String]>,
        mut state: std::collections::BTreeMap<String, String>,
    ) -> Result<std::collections::BTreeMap<String, String>, OperationError> {
        let all = keys.is_none_or(|keys| keys.is_empty());
        let explicit = keys.is_some_and(|keys| {
            keys.iter()
                .any(|key| LIBRARY_STATE_KEYS.contains(&key.as_str()))
        });
        if !all && !explicit {
            return Ok(state);
        }
        let initialized = self.db.automation_library_exists().map_err(failure)?;
        if !explicit
            && !initialized
            && !self
                .db
                .automation_library_legacy_exists()
                .map_err(failure)?
        {
            return Ok(state);
        }
        if !initialized {
            let _owner = self
                .owner_adoption_gate
                .lock()
                .map_err(|_| OperationError::internal("Owner coordinator is unavailable"))?;
            let _coordination = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
            let _writer = self.automation_gate()?;
            self.db.ensure_automation_library().map_err(failure)?;
        }
        for (key, value) in self.db.automation_library_legacy_state().map_err(failure)? {
            if all || keys.is_some_and(|keys| keys.contains(&key)) {
                state.insert(key, value);
            }
        }
        Ok(state)
    }

    #[cfg(feature = "persistence")]
    pub(crate) fn library_app_state_set(
        &self,
        key: &str,
        value: &str,
    ) -> Result<std::collections::BTreeMap<String, String>, OperationError> {
        let _owner = self
            .owner_adoption_gate
            .lock()
            .map_err(|_| OperationError::internal("Owner coordinator is unavailable"))?;
        let _coordination = self
            .coordination_gate
            .lock()
            .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
        let _writer = self.automation_gate()?;
        self.db.ensure_automation_library().map_err(failure)?;
        let service = self
            .collaboration()
            .map_err(super::super::commands::service_error)?;
        self.db
            .automation_library_legacy_set(service.host_id(), key, value)
            .map_err(|error| match error {
                crate::db::DatabaseError::Invalid(_) | crate::db::DatabaseError::Json(_) => {
                    OperationError::invalid("invalid automation library state")
                }
                other => failure(other),
            })?;
        let stored = self
            .db
            .automation_library_legacy_state()
            .map_err(failure)?
            .remove(key)
            .ok_or_else(|| OperationError::internal("library value is unavailable"))?;
        self.app_state.set(key.to_owned(), stored.clone());
        self.events
            .publish_presentation_domain(crate::events::DomainEvent::WorkflowChanged {
                kind: "library".into(),
                id: "catalog".into(),
                change: "updated".into(),
            });
        Ok(std::collections::BTreeMap::from([(key.to_owned(), stored)]))
    }

    pub fn owner_library_snapshot(&self) -> Result<OwnerLibrarySnapshot, OperationError> {
        #[cfg(feature = "persistence")]
        {
            if !self.db.automation_library_exists().map_err(failure)? {
                let _owner = self
                    .owner_adoption_gate
                    .lock()
                    .map_err(|_| OperationError::internal("Owner coordinator is unavailable"))?;
                let _coordination = self
                    .coordination_gate
                    .lock()
                    .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
                let _writer = self.automation_gate()?;
                self.db.ensure_automation_library().map_err(failure)?;
            }
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            let (catalog, revision, mut definitions) =
                self.db.automation_library_snapshot().map_err(failure)?;
            self.merge_runtime_catalog(&mut definitions.value);
            let result = OwnerLibrarySnapshot {
                host_id: service.host_id().into(),
                host_instance_id: service.instance_id().into(),
                revision,
                definition_digest: owner_library_definition_digest(
                    &definitions.value,
                    &definitions.resource_versions,
                ),
                catalog,
                definitions: definitions.value,
                resource_versions: definitions.resource_versions,
            };
            if serde_json::to_vec(&result)
                .map_err(|_| OperationError::internal("library snapshot is unavailable"))?
                .len()
                > OWNER_LIBRARY_MAX_BYTES
            {
                return Err(failure(crate::db::DatabaseError::SnapshotTooLarge));
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::capability_unavailable(
            "atomic automation library changes require persistence",
        ))
    }

    pub fn owner_library_state(&self) -> Result<OwnerLibraryState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _owner = self
                .owner_adoption_gate
                .lock()
                .map_err(|_| OperationError::internal("Owner coordinator is unavailable"))?;
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            self.db
                .owner_library_state(service.host_id(), service.instance_id())
                .map_err(failure)
        }
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::capability_unavailable(
            "automation library recovery requires persistence",
        ))
    }

    pub fn owner_library_submit(
        &self,
        input: OwnerLibrarySubmit,
    ) -> Result<OwnerLibrarySubmission, OperationError> {
        #[cfg(feature = "persistence")]
        {
            validate_owner_library_submit(&input)?;
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            let _owner = self
                .owner_adoption_gate
                .lock()
                .map_err(|_| OperationError::internal("Owner coordinator is unavailable"))?;
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            if input.host_id != service.host_id() || input.host_instance_id != service.instance_id()
            {
                return Err(OperationError {
                    code: "host_restarted",
                    message: "Owner host identity changed; review again".into(),
                    data: None,
                });
            }
            // Migration is independent of the requested effect and finishes
            // before the native intent. It never rewrites the legacy points DB.
            {
                let _coordination = self
                    .coordination_gate
                    .lock()
                    .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
                let _writer = self.automation_gate()?;
                self.db.ensure_automation_library().map_err(failure)?;
            }
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| OperationError::internal("command identity is unavailable"))?;
            let intent = OwnerLibraryIntent {
                command_id: format!(
                    "owner-library-{}",
                    bytes
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                ),
                operation: input.mutation.operation(),
                target: input.mutation.target().into(),
                expected_revision: input.expected_revision.clone(),
                definition_digest: input.definition_digest.clone(),
                command_hash: owner_library_command_hash(&input),
                host_instance_id: input.host_instance_id.clone(),
                created_at: library_now(),
                provenance: input.provenance.clone(),
            };
            crash("before_intent");
            self.db
                .owner_library_begin(&input.host_id, &intent, &input)
                .map_err(failure)?;
            crash("after_intent");
            crash("before_dispatch");
            let original = self.execute_owner_library(&input, &intent);
            crash("after_effect");
            let original = match original {
                Ok(result) => {
                    self.db
                        .owner_library_resolve(&input.host_id, false, None)
                        .map_err(failure)?;
                    Some(result)
                }
                Err(error) if library_rejection(error.code) => {
                    self.db
                        .owner_library_resolve(&input.host_id, false, Some(error.code))
                        .map_err(failure)?;
                    None
                }
                Err(error) => return Err(error),
            };
            crash("after_record");
            let state = self
                .db
                .owner_library_state(&input.host_id, service.instance_id())
                .map_err(failure)?;
            let acknowledgement = original
                .map(|result| fresh_acknowledgement(&intent, &state, result))
                .transpose()?;
            Ok(OwnerLibrarySubmission {
                state,
                acknowledgement,
            })
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = input;
            Err(OperationError::capability_unavailable(
                "atomic automation library changes require persistence",
            ))
        }
    }

    pub fn owner_library_resolve(&self) -> Result<OwnerLibraryState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _owner = self
                .owner_adoption_gate
                .lock()
                .map_err(|_| OperationError::internal("Owner coordinator is unavailable"))?;
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            self.db
                .owner_library_resolve(service.host_id(), false, None)
                .map_err(failure)?;
            self.db
                .owner_library_state(service.host_id(), service.instance_id())
                .map_err(failure)
        }
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::capability_unavailable(
            "automation library recovery requires persistence",
        ))
    }

    pub fn owner_library_review_unknown(
        &self,
        input: OwnerLibraryReviewUnknown,
    ) -> Result<OwnerLibraryState, OperationError> {
        #[cfg(feature = "persistence")]
        {
            if !input.acknowledge_unknown {
                return Err(OperationError {
                    code: "confirmation_required",
                    message: "review the unresolved original library outcome".into(),
                    data: None,
                });
            }
            let _owner = self
                .owner_adoption_gate
                .lock()
                .map_err(|_| OperationError::internal("Owner coordinator is unavailable"))?;
            let service = self
                .collaboration()
                .map_err(super::super::commands::service_error)?;
            let state = self
                .db
                .owner_library_state(service.host_id(), service.instance_id())
                .map_err(failure)?;
            if state
                .pending
                .as_ref()
                .map(|intent| intent.command_id.as_str())
                != Some(input.command_id.as_str())
            {
                return Err(OperationError {
                    code: "stale_view",
                    message: "pending Owner library change was replaced".into(),
                    data: None,
                });
            }
            self.db
                .owner_library_resolve(service.host_id(), true, None)
                .map_err(failure)?;
            self.db
                .owner_library_state(service.host_id(), service.instance_id())
                .map_err(failure)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = input;
            Err(OperationError::capability_unavailable(
                "automation library recovery requires persistence",
            ))
        }
    }
}
