use super::*;
use crate::control::{OwnerLibraryIntent, OwnerLibrarySubmit};

impl AppCore {
    pub(crate) fn execute_owner_library(
        &self,
        input: &OwnerLibrarySubmit,
        intent: &OwnerLibraryIntent,
    ) -> Result<crate::collaboration::Versioned<Value>, OperationError> {
        let _coordination = self
            .coordination_gate
            .lock()
            .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
        let _writer = self.automation_gate()?;
        self.ensure_automation_writable(Some(&input.expected_revision))?;
        let service = self
            .collaboration()
            .map_err(super::super::commands::service_error)?;
        let command = crate::db::CommandTransaction {
            principal_id: "local-owner".into(),
            session_id: "local-owner".into(),
            authorization: None,
            request: crate::collaboration::CommandRequest {
                command_id: intent.command_id.clone(),
                host_instance_id: intent.host_instance_id.clone(),
                expected_revision: intent.expected_revision.clone(),
            },
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
                "local-owner",
                &intent.command_id,
                command.max_ids_per_principal,
            )
            .map_err(|error| super::commands::behavior_command_error(error, &intent.command_id))?;
        // Runtime descriptors do not belong to the source transaction. Capture
        // their current exact byte overhead under the same writer, then leave
        // that headroom in the pure SQLite precommit snapshot check.
        let (catalog, revision, mut current) =
            self.db
                .automation_library_snapshot()
                .map_err(|error| match error {
                    crate::db::DatabaseError::SnapshotTooLarge => OperationError {
                        code: "too_large",
                        message: "automation library exceeds its bounded snapshot budget".into(),
                        data: None,
                    },
                    other => super::commands::behavior_command_error(other, &intent.command_id),
                })?;
        let size = |definitions: &Value| {
            serde_json::to_vec(&serde_json::json!({
            "catalog":catalog,"revision":revision,"definitions":definitions,"resourceVersions":current.resource_versions,
        })).map(|value|value.len()).map_err(|_|OperationError::internal("library budget is unavailable"))
        };
        let stored_bytes = size(&current.value)?;
        self.merge_runtime_catalog(&mut current.value);
        let merged_bytes = size(&current.value)?;
        let too_large = || OperationError {
            code: "too_large",
            message: "automation library exceeds its merged snapshot budget".into(),
            data: None,
        };
        if merged_bytes > crate::control::OWNER_LIBRARY_MAX_BYTES - 8192 {
            return Err(too_large());
        }
        let snapshot_budget = (crate::control::OWNER_LIBRARY_MAX_BYTES - 8192)
            .checked_sub(merged_bytes.saturating_sub(stored_bytes))
            .ok_or_else(too_large)?;
        let (result, fresh) = self
            .db
            .execute_automation_library_command_with_budget(input, &command, snapshot_budget)
            .map_err(|error| match error {
                crate::db::DatabaseError::SnapshotTooLarge => OperationError {
                    code: "too_large",
                    message: "automation library exceeds its bounded result budget".into(),
                    data: None,
                },
                crate::db::DatabaseError::LibraryDefinitionConflict => OperationError {
                    code: "revision_conflict",
                    message: "automation definitions changed; review the current library again"
                        .into(),
                    data: None,
                },
                other => super::commands::behavior_command_error(other, &intent.command_id),
            })?;
        if fresh {
            if result.resource_versions.len() > 1 {
                self.refresh_automation_snapshot_locked()?;
            }
            self.events
                .publish_presentation_domain(DomainEvent::WorkflowChanged {
                    kind: "library".into(),
                    id: "catalog".into(),
                    change: "updated".into(),
                });
        }
        Ok(result)
    }
}
