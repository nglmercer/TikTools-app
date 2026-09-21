//! Event processor testing and status operations.

use super::{clean_plugin_id, OperationError};
use crate::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorOutcomeDto {
    pub plugin_id: String,
    pub processor_id: String,
    pub ok: bool,
    pub duration_ms: u64,
    pub result: Value,
    pub error: Option<String>,
}

impl AppCore {
    // ------------------------------------------------------------------
    // Processors.
    // ------------------------------------------------------------------

    pub async fn processor_test(
        &self,
        plugin_id: &str,
        processor_id: &str,
        event: Value,
    ) -> Result<ProcessorOutcomeDto, OperationError> {
        let plugin_id = clean_plugin_id(plugin_id)?;
        let processor_id = processor_id.trim();
        if processor_id.is_empty() || processor_id.len() > 128 {
            return Err(OperationError::invalid(
                "processor id must be 1..=128 characters",
            ));
        }
        if !event.is_object() {
            return Err(OperationError::invalid("event must be an object"));
        }
        let outcome = self.test_processor(&plugin_id, processor_id, event).await;
        Ok(ProcessorOutcomeDto {
            plugin_id,
            processor_id: processor_id.to_owned(),
            ok: outcome.ok,
            duration_ms: outcome.duration_ms,
            result: outcome.result,
            error: outcome.error,
        })
    }

    pub fn processor_status(&self) -> Value {
        self.processor_status_snapshot()
    }

    pub fn processor_list(&self) -> Vec<Value> {
        match self.processor_status_snapshot() {
            Value::Array(entries) => entries,
            snapshot => snapshot
                .get("processors")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        }
    }
}
