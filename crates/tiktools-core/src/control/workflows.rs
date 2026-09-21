//! Visual workflow graph persistence operations.

use super::{clean_record_id, OperationError};
#[cfg(feature = "persistence")]
use crate::events::DomainEvent;
use crate::*;

impl AppCore {
    // ------------------------------------------------------------------
    // Graph workflows (the node-canvas persistence behind the UI).
    // ------------------------------------------------------------------

    pub fn workflow_list(&self) -> Result<Vec<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            self.db
                .load_workflows()
                .map_err(|error| OperationError::internal(error.to_string()))
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "graph workflows need the persistence build",
            ))
        }
    }

    pub fn workflow_save(&self, graph: Value) -> Result<Value, OperationError> {
        #[cfg(feature = "persistence")]
        {
            if !graph.is_object() {
                return Err(OperationError::invalid("workflow graph must be an object"));
            }
            let saved = self
                .db
                .save_workflow(&graph)
                .map_err(|error| OperationError::internal(error.to_string()))?;
            if let Some(id) = saved.get("id").and_then(Value::as_str) {
                self.events.publish_domain(DomainEvent::WorkflowChanged {
                    kind: "workflow".to_owned(),
                    id: id.to_owned(),
                    change: "saved".to_owned(),
                });
            }
            Ok(saved)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = graph;
            Err(OperationError::unavailable(
                "graph workflows need the persistence build",
            ))
        }
    }

    pub fn workflow_delete(&self, id: &str) -> Result<bool, OperationError> {
        let id = clean_record_id(id, "workflow")?;
        #[cfg(feature = "persistence")]
        {
            let removed = self
                .db
                .delete_workflow(&id)
                .map_err(|error| OperationError::internal(error.to_string()))?;
            if removed {
                self.events.publish_domain(DomainEvent::WorkflowChanged {
                    kind: "workflow".to_owned(),
                    id,
                    change: "deleted".to_owned(),
                });
            }
            Ok(removed)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = id;
            Err(OperationError::unavailable(
                "graph workflows need the persistence build",
            ))
        }
    }

    pub fn workflow_set_enabled(&self, id: &str, enabled: bool) -> Result<Value, OperationError> {
        let id = clean_record_id(id, "workflow")?;
        #[cfg(feature = "persistence")]
        {
            let saved =
                self.db
                    .set_workflow_enabled(&id, enabled)
                    .map_err(|error| match error {
                        crate::db::DatabaseError::Invalid(message) => {
                            OperationError::not_found(message)
                        }
                        other => OperationError::internal(other.to_string()),
                    })?;
            self.events.publish_domain(DomainEvent::WorkflowChanged {
                kind: "workflow".to_owned(),
                id,
                change: if enabled { "enabled" } else { "disabled" }.to_owned(),
            });
            Ok(saved)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (id, enabled);
            Err(OperationError::unavailable(
                "graph workflows need the persistence build",
            ))
        }
    }

    pub fn workflow_get(&self, id: &str) -> Result<Value, OperationError> {
        let id = clean_record_id(id, "workflow")?;
        self.workflow_list()?
            .into_iter()
            .find(|workflow| workflow.get("id").and_then(Value::as_str) == Some(id.as_str()))
            .ok_or_else(|| OperationError::not_found(format!("workflow `{id}` does not exist")))
    }
}
