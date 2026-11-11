use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::{collaboration::Versioned, control::WorkflowMutation, AppCore};

use crate::{
    command_metadata::{versioned_command, CommandDispatch},
    error::ApiError,
    modules::Empty,
    router::ControlRouter,
};

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
    router.register_versioned::<Empty, WorkflowListResult, _, _>(
        "workflows.list",
        "Graph workflows for the node-canvas editor",
        false,
        |core: Arc<AppCore>, context, _params: Empty, _metadata| async move {
            let scope = match context {
                crate::RequestContext::Remote(session) => Some((
                    session.grant()?,
                    session
                        .connection_limits()
                        .map_or(512 * 1024, |l| l.frame_bytes / 2),
                )),
                _ => None,
            };
            crate::modules::blocking_task("workflows.list", move || match scope {
                Some((grant, bytes)) => core.workflow_list_scoped(&grant, bytes),
                None => core.workflow_list_versioned(),
            })
            .await?
            .map(|result| result.map(|workflows| WorkflowListResult { workflows }))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<WorkflowIdParams, Value, _, _>(
        "workflows.get",
        "One graph workflow by id",
        false,
        |core: Arc<AppCore>, context, params: WorkflowIdParams, _metadata| async move {
            let scope = if let crate::RequestContext::Remote(session) = context {
                if params.id.trim() != params.id
                    || !session.grant()?.allows_resource(
                        tiktools_core::collaboration::ResourceKind::Workflow,
                        &params.id,
                    )
                {
                    return Err(crate::context::forbidden());
                }
                Some((
                    session.grant()?,
                    session
                        .connection_limits()
                        .map_or(512 * 1024, |l| l.frame_bytes / 2),
                ))
            } else {
                None
            };
            let result = crate::modules::blocking_task("workflows.get", move || match scope {
                Some((grant, bytes)) => core.workflow_get_scoped(&grant, &params.id, bytes),
                None => core.workflow_get_versioned(&params.id),
            })
            .await??;
            match result.value {
                Some(value) => Ok(Versioned::new(value, result.resource_versions)),
                None => {
                    let mut error = ApiError::new("workflow_not_found", "workflow does not exist");
                    error.data =
                        Some(serde_json::json!({"resourceVersions":result.resource_versions}));
                    Err(error)
                }
            }
        },
    );
    router.register_versioned::<WorkflowSaveParams, Value, _, _>(
        "workflows.save",
        "Creates or replaces a graph workflow",
        true,
        |core, context, params, metadata| async move {
            mutate(
                core,
                context,
                metadata,
                WorkflowMutation::Save(params.graph),
            )
            .await
        },
    );
    router.register_versioned::<WorkflowIdParams, WorkflowDeleteResult, _, _>(
        "workflows.delete",
        "Deletes a graph workflow",
        true,
        |core, context, params, metadata| async move {
            let id = params.id;
            let result = mutate(
                core,
                context,
                metadata,
                WorkflowMutation::Delete(id.clone()),
            )
            .await?;
            let deleted = result
                .value
                .get("deleted")
                .and_then(Value::as_bool)
                .ok_or_else(|| ApiError::internal("invalid stored workflow command result"))?;
            Ok(result.map(|_| WorkflowDeleteResult { id, deleted }))
        },
    );
    router.register_versioned::<WorkflowIdParams, Value, _, _>(
        "workflows.enable",
        "Enables a graph workflow",
        true,
        |core, context, params, metadata| async move {
            mutate(
                core,
                context,
                metadata,
                WorkflowMutation::SetEnabled {
                    id: params.id,
                    enabled: true,
                },
            )
            .await
        },
    );
    router.register_versioned::<WorkflowIdParams, Value, _, _>(
        "workflows.disable",
        "Disables a graph workflow",
        true,
        |core, context, params, metadata| async move {
            mutate(
                core,
                context,
                metadata,
                WorkflowMutation::SetEnabled {
                    id: params.id,
                    enabled: false,
                },
            )
            .await
        },
    );
    for method in [
        "workflows.save",
        "workflows.delete",
        "workflows.enable",
        "workflows.disable",
    ] {
        router.set_durable_commands(method, true);
    }
}

async fn mutate(
    core: Arc<AppCore>,
    context: crate::RequestContext,
    metadata: Option<crate::RequestMetadata>,
    operation: WorkflowMutation,
) -> Result<Versioned<Value>, ApiError> {
    crate::modules::blocking_task("workflows.mutate", move || {
        match versioned_command(context, metadata, || {
            ApiError::new(
                "forbidden",
                "remote mutations require durable command metadata",
            )
        })? {
            CommandDispatch::Durable { caller, request } => core
                .workflow_command(caller, request, operation)
                .map_err(ApiError::from),
            CommandDispatch::Local { expected_revision } => {
                let expected = expected_revision.as_deref();
                match operation {
                    WorkflowMutation::Save(graph) => core.workflow_save_versioned(graph, expected),
                    WorkflowMutation::Delete(id) => {
                        core.workflow_delete_versioned(&id, expected).map(|result| {
                            result.map(|deleted| serde_json::json!({"id":id,"deleted":deleted}))
                        })
                    }
                    WorkflowMutation::SetEnabled { id, enabled } => {
                        core.workflow_set_enabled_versioned(&id, enabled, expected)
                    }
                }
                .map_err(ApiError::from)
            }
        }
    })
    .await?
}
