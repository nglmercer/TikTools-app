use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use tiktools_core::control::ProcessorOutcomeDto;
use tiktools_core::AppCore;

use crate::{error::ApiError, modules::Empty, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorListResult {
    pub processors: Vec<Value>,
}

/// Typed health/metrics snapshot. The RPC shape is identical to the
/// frontend `ProcessorStatusEntry`: `{ processors: [...] }`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorStatusMetrics {
    pub calls: u64,
    pub successes: u64,
    pub failures: u64,
    pub timeouts: u64,
    pub skipped_circuit_open: u64,
    pub skipped_overloaded: u64,
    pub average_latency_ms: f64,
    pub max_latency_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_success_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorStatusEntry {
    pub plugin_id: String,
    pub processor_id: String,
    pub event_types: Vec<String>,
    pub timeout_ms: u64,
    pub priority: i64,
    pub status: String,
    pub metrics: ProcessorStatusMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorStatusResult {
    pub processors: Vec<ProcessorStatusEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorTestParams {
    pub plugin_id: String,
    pub processor_id: String,
    pub event: Value,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, ProcessorListResult, _, _>(
        "processors.list",
        "Indexed event processors",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<ProcessorListResult, ApiError>(ProcessorListResult {
                processors: core.processor_list(),
            })
        },
    );
    router.register_typed::<Empty, ProcessorStatusResult, _, _>(
        "processors.status",
        "Processor health and metrics snapshot",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            let mut processors = Vec::new();
            for entry in core.processor_list() {
                match serde_json::from_value::<ProcessorStatusEntry>(entry) {
                    Ok(entry) => processors.push(entry),
                    Err(error) => {
                        return Err(ApiError::internal(format!(
                            "processor status shape mismatch: {error}"
                        )));
                    }
                }
            }
            Ok::<ProcessorStatusResult, ApiError>(ProcessorStatusResult { processors })
        },
    );
    router.register_typed::<ProcessorTestParams, ProcessorOutcomeDto, _, _>(
        "processors.test",
        "Runs one processor against a sample event",
        false,
        |core: Arc<AppCore>, params: ProcessorTestParams| async move {
            core.processor_test(&params.plugin_id, &params.processor_id, params.event)
                .await
                .map_err(ApiError::from)
        },
    );
}
