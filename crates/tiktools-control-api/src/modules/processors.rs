use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::{control::ProcessorOutcomeDto, AppCore};

use crate::{error::ApiError, modules::Empty, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorListResult {
    pub processors: Vec<Value>,
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
    router.register_typed::<Empty, Value, _, _>(
        "processors.status",
        "Processor health and metrics snapshot",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<Value, ApiError>(core.processor_status())
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
