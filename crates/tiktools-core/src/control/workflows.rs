//! Visual workflow persistence. Legacy local calls share the transactional
//! revision path; metadata-aware adapters can use the versioned operations.
use super::{clean_record_id, OperationError};
#[cfg(feature = "persistence")]
use crate::events::DomainEvent;
use crate::{collaboration::Versioned, *};

impl AppCore {
    pub fn workflow_get_scoped(
        &self,
        grant: &crate::collaboration::Grant,
        id: &str,
        bytes: usize,
    ) -> Result<Versioned<Option<Value>>, OperationError> {
        if id.trim() != id
            || !grant.allows_resource(crate::collaboration::ResourceKind::Workflow, id)
        {
            return Err(OperationError::forbidden("workflow is not available"));
        }
        let id = clean_record_id(id, "workflow")?;
        #[cfg(feature = "persistence")]
        {
            self.db
                .load_workflow_bounded(&id, bytes)
                .map_err(|error| match error {
                    crate::db::DatabaseError::SnapshotTooLarge => OperationError {
                        code: "too_large",
                        message: "workflow exceeds frame budget".into(),
                        data: None,
                    },
                    other => workflow_error(other),
                })
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (id, bytes);
            Err(workflow_unavailable())
        }
    }
    pub fn workflow_list_scoped(
        &self,
        grant: &crate::collaboration::Grant,
        bytes: usize,
    ) -> Result<Versioned<Vec<Value>>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let rows = self
                .db
                .load_workflow_snapshot(grant, bytes.min(256 * 1024), bytes)
                .map_err(|error| match error {
                    crate::db::DatabaseError::SnapshotTooLarge => OperationError {
                        code: "too_large",
                        message: "workflow list exceeds frame budget; use collaboration.snapshot"
                            .into(),
                        data: None,
                    },
                    other => workflow_error(other),
                })?;
            let (value, versions) = rows.into_iter().unzip();
            Ok(Versioned::new(value, versions))
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (grant, bytes);
            Err(workflow_unavailable())
        }
    }
    pub fn workflow_list(&self) -> Result<Vec<Value>, OperationError> {
        self.workflow_list_versioned().map(|result| result.value)
    }

    pub fn workflow_list_versioned(&self) -> Result<Versioned<Vec<Value>>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            self.db.load_workflows().map_err(workflow_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(workflow_unavailable())
        }
    }

    pub fn workflow_get(&self, id: &str) -> Result<Value, OperationError> {
        self.workflow_get_versioned(id)?
            .value
            .ok_or_else(|| OperationError::not_found(format!("workflow `{id}` does not exist")))
    }

    /// A missing value carries revision 0 (never created) or the retained
    /// tombstone revision, read in the same snapshot as the missing graph.
    pub fn workflow_get_versioned(
        &self,
        id: &str,
    ) -> Result<Versioned<Option<Value>>, OperationError> {
        let id = clean_record_id(id, "workflow")?;
        #[cfg(feature = "persistence")]
        {
            self.db.load_workflow(&id).map_err(workflow_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = id;
            Err(workflow_unavailable())
        }
    }

    pub fn workflow_save(&self, graph: Value) -> Result<Value, OperationError> {
        self.workflow_save_versioned(graph, None)
            .map(|result| result.value)
    }

    pub fn workflow_save_versioned(
        &self,
        graph: Value,
        expected: Option<&str>,
    ) -> Result<Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let _gate = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("coordination barrier is unavailable"))?;
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            let result = self
                .db
                .save_workflow(&graph, expected)
                .map_err(workflow_error)?;
            self.workflow_changed(&result.resource_versions[0].id, "saved");
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (graph, expected);
            Err(workflow_unavailable())
        }
    }

    pub fn workflow_delete(&self, id: &str) -> Result<bool, OperationError> {
        self.workflow_delete_versioned(id, None)
            .map(|result| result.value)
    }

    pub fn workflow_delete_versioned(
        &self,
        id: &str,
        expected: Option<&str>,
    ) -> Result<Versioned<bool>, OperationError> {
        let id = clean_record_id(id, "workflow")?;
        #[cfg(feature = "persistence")]
        {
            let _gate = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("coordination barrier is unavailable"))?;
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            let result = self
                .db
                .delete_workflow(&id, expected)
                .map_err(workflow_error)?;
            if result.value {
                self.workflow_changed(&id, "deleted");
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (id, expected);
            Err(workflow_unavailable())
        }
    }

    pub fn workflow_set_enabled(&self, id: &str, enabled: bool) -> Result<Value, OperationError> {
        self.workflow_set_enabled_versioned(id, enabled, None)
            .map(|result| result.value)
    }

    pub fn workflow_set_enabled_versioned(
        &self,
        id: &str,
        enabled: bool,
        expected: Option<&str>,
    ) -> Result<Versioned<Value>, OperationError> {
        let id = clean_record_id(id, "workflow")?;
        #[cfg(feature = "persistence")]
        {
            let _gate = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("coordination barrier is unavailable"))?;
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            let result = self
                .db
                .set_workflow_enabled(&id, enabled, expected)
                .map_err(workflow_error)?;
            self.workflow_changed(&id, if enabled { "enabled" } else { "disabled" });
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (id, enabled, expected);
            Err(workflow_unavailable())
        }
    }

    #[cfg(feature = "persistence")]
    fn workflow_changed(&self, id: &str, change: &str) {
        self.events.publish_domain(DomainEvent::WorkflowChanged {
            kind: "workflow".into(),
            id: id.into(),
            change: change.into(),
        });
    }
}

#[cfg(not(feature = "persistence"))]
fn workflow_unavailable() -> OperationError {
    OperationError::unavailable("graph workflows need the persistence build")
}

#[cfg(feature = "persistence")]
pub(super) fn workflow_error(error: crate::db::DatabaseError) -> OperationError {
    use crate::db::DatabaseError;
    match error {
        DatabaseError::RevisionConflict(version) => OperationError {
            code: "revision_conflict",
            message: "workflow changed; reload before saving".into(),
            data: Some(serde_json::json!({"resourceVersions":[version]})),
        },
        DatabaseError::NotFound => OperationError::not_found("workflow does not exist"),
        DatabaseError::Invalid(message) => OperationError::invalid(message),
        other => OperationError::internal(other.to_string()),
    }
}
