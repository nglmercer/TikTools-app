use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::AppCore;

use crate::{error::ApiError, modules::Empty, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowIdParams {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSaveParams {
    pub graph: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowListResult {
    pub workflows: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDeleteResult {
    pub id: String,
    pub deleted: bool,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, WorkflowListResult, _, _>(
        "workflows.list",
        "Graph workflows for the node-canvas editor",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            crate::modules::blocking_task("workflows.list", move || core.workflow_list())
                .await?
                .map(|workflows| WorkflowListResult { workflows })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<WorkflowIdParams, Value, _, _>(
        "workflows.get",
        "One graph workflow by id",
        false,
        |core: Arc<AppCore>, params: WorkflowIdParams| async move {
            crate::modules::blocking_task("workflows.get", move || core.workflow_get(&params.id))
                .await?
                .map_err(|error| ApiError::from(error).scoped_not_found("workflow_not_found"))
        },
    );
    router.register_typed::<WorkflowSaveParams, Value, _, _>(
        "workflows.save",
        "Creates or replaces a graph workflow (graph must carry id and name)",
        true,
        |core: Arc<AppCore>, params: WorkflowSaveParams| async move {
            crate::modules::blocking_task("workflows.save", move || {
                core.workflow_save(params.graph)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<WorkflowIdParams, WorkflowDeleteResult, _, _>(
        "workflows.delete",
        "Deletes a graph workflow",
        true,
        |core: Arc<AppCore>, params: WorkflowIdParams| async move {
            let id = params.id.clone();
            crate::modules::blocking_task("workflows.delete", move || {
                core.workflow_delete(&params.id)
            })
            .await?
            .map(|deleted| WorkflowDeleteResult { id, deleted })
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<WorkflowIdParams, Value, _, _>(
        "workflows.enable",
        "Enables a graph workflow",
        true,
        |core: Arc<AppCore>, params: WorkflowIdParams| async move {
            crate::modules::blocking_task("workflows.enable", move || {
                core.workflow_set_enabled(&params.id, true)
            })
            .await?
            .map_err(|error| ApiError::from(error).scoped_not_found("workflow_not_found"))
        },
    );
    router.register_typed::<WorkflowIdParams, Value, _, _>(
        "workflows.disable",
        "Disables a graph workflow",
        true,
        |core: Arc<AppCore>, params: WorkflowIdParams| async move {
            crate::modules::blocking_task("workflows.disable", move || {
                core.workflow_set_enabled(&params.id, false)
            })
            .await?
            .map_err(|error| ApiError::from(error).scoped_not_found("workflow_not_found"))
        },
    );
}
