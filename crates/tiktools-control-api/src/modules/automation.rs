use std::{str::FromStr, sync::Arc};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::{
    control::{AutomationKind, ScriptAnalysisResult},
    AppCore,
};

use crate::{error::ApiError, modules::Empty, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationListParams {
    /// `event`, `action`, or `all` (default).
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationListResult {
    pub events: Vec<Value>,
    pub actions: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationGetParams {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationCreateParams {
    #[serde(default)]
    pub kind: Option<String>,
    pub record: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationUpdateParams {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    pub record: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationDeleteResult {
    pub id: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationContextResult {
    pub event: Option<Value>,
    pub captured_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationScriptParams {
    pub node_id: String,
    pub source: String,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub event_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationNodesResult {
    pub nodes: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunsResult {
    pub runs: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationTestParams {
    /// Test a saved record by id...
    #[serde(default)]
    pub id: Option<String>,
    /// ...or an inline record. One of the two is required.
    #[serde(default)]
    pub record: Option<Value>,
    #[serde(default)]
    pub kind: Option<String>,
    /// Sample trigger for inline action tests (default `tiktok.chat`).
    #[serde(default)]
    pub trigger: Option<String>,
}

fn parse_kind(raw: Option<&str>, default: AutomationKind) -> Result<AutomationKind, ApiError> {
    match raw {
        None => Ok(default),
        Some(value) => AutomationKind::from_str(value).map_err(ApiError::from),
    }
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<AutomationListParams, AutomationListResult, _, _>(
        "automation.list",
        "Behavior events and actions (kind: event, action, all)",
        false,
        |core: Arc<AppCore>, params: AutomationListParams| async move {
            let kind = params
                .kind
                .as_deref()
                .unwrap_or("all")
                .trim()
                .to_ascii_lowercase();
            let (events, actions) = match kind.as_str() {
                "event" | "events" => (core.automation_list(AutomationKind::Event), Vec::new()),
                "action" | "actions" => (Vec::new(), core.automation_list(AutomationKind::Action)),
                "all" | "" => (
                    core.automation_list(AutomationKind::Event),
                    core.automation_list(AutomationKind::Action),
                ),
                other => {
                    return Err::<AutomationListResult, ApiError>(ApiError::invalid_params(
                        format!("unknown automation kind `{other}`"),
                    ))
                }
            };
            Ok(AutomationListResult { events, actions })
        },
    );
    router.register_typed::<AutomationGetParams, Value, _, _>(
        "automation.get",
        "One behavior event or action by id",
        false,
        |core: Arc<AppCore>, params: AutomationGetParams| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            core.automation_get(kind, &params.id)
                .map_err(|error| ApiError::from(error).scoped_not_found("automation_not_found"))
        },
    );
    router.register_typed::<AutomationCreateParams, Value, _, _>(
        "automation.create",
        "Creates a behavior event or action (id generated when omitted)",
        true,
        |core: Arc<AppCore>, params: AutomationCreateParams| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            core.automation_create(kind, params.record)
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<AutomationUpdateParams, Value, _, _>(
        "automation.update",
        "Replaces a behavior event or action (must already exist)",
        true,
        |core: Arc<AppCore>, params: AutomationUpdateParams| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            core.automation_update(kind, &params.id, params.record)
                .map_err(|error| ApiError::from(error).scoped_not_found("automation_not_found"))
        },
    );
    router.register_typed::<AutomationGetParams, AutomationDeleteResult, _, _>(
        "automation.delete",
        "Deletes a behavior event or action",
        true,
        |core: Arc<AppCore>, params: AutomationGetParams| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            core.automation_delete(kind, &params.id)
                .map(|deleted| AutomationDeleteResult {
                    id: params.id,
                    deleted,
                })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<AutomationGetParams, Value, _, _>(
        "automation.enable",
        "Enables a behavior event or action",
        true,
        |core: Arc<AppCore>, params: AutomationGetParams| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            core.automation_set_enabled(kind, &params.id, true)
                .map_err(|error| ApiError::from(error).scoped_not_found("automation_not_found"))
        },
    );
    router.register_typed::<AutomationGetParams, Value, _, _>(
        "automation.disable",
        "Disables a behavior event or action",
        true,
        |core: Arc<AppCore>, params: AutomationGetParams| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            core.automation_set_enabled(kind, &params.id, false)
                .map_err(|error| ApiError::from(error).scoped_not_found("automation_not_found"))
        },
    );
    router.register_typed::<Empty, AutomationContextResult, _, _>(
        "automation.context",
        "Last automation event observed, for context panels and previews",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            let (event, captured_at) = core.automation_context();
            Ok::<AutomationContextResult, ApiError>(AutomationContextResult { event, captured_at })
        },
    );
    router.register_typed::<AutomationTestParams, Value, _, _>(
        "automation.test",
        "Dry-runs a saved (id) or inline (record) event or action",
        false,
        |core: Arc<AppCore>, params: AutomationTestParams| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            let record = match (params.id, params.record) {
                (Some(id), _) => core.automation_get(kind, &id).map_err(|error| {
                    ApiError::from(error).scoped_not_found("automation_not_found")
                })?,
                (None, Some(record)) => record,
                (None, None) => {
                    return Err::<Value, ApiError>(ApiError::invalid_params(
                        "automation.test needs `id` or `record`",
                    ))
                }
            };
            if !record.is_object() {
                return Err::<Value, ApiError>(ApiError::invalid_params(
                    "automation record must be an object",
                ));
            }
            Ok::<Value, ApiError>(match kind {
                AutomationKind::Event => core.test_automation_event(&record).await,
                AutomationKind::Action => {
                    core.test_automation_action(&record, params.trigger.as_deref())
                        .await
                }
            })
        },
    );
    router.register_typed::<Empty, AutomationNodesResult, _, _>(
        "automation.nodes.list",
        "Built-in node catalog for the workflow graph editor",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<AutomationNodesResult, ApiError>(AutomationNodesResult {
                nodes: core.automation_nodes(),
            })
        },
    );
    router.register_typed::<AutomationScriptParams, ScriptAnalysisResult, _, _>(
        "automation.script.analyze",
        "Validates an automation script and returns diagnostics",
        false,
        |core: Arc<AppCore>, params: AutomationScriptParams| async move {
            core.automation_script_analyze(
                &params.node_id,
                &params.source,
                params.offset.unwrap_or(0),
                params.event_type.as_deref(),
            )
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, AutomationRunsResult, _, _>(
        "automation.runs",
        "Recent behavior execution runs",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<AutomationRunsResult, ApiError>(AutomationRunsResult {
                runs: core.automation_runs(),
            })
        },
    );
    router.register_typed::<Empty, Value, _, _>(
        "automation.snapshot",
        "Merged behavior snapshot: records plus the live runtime catalog",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            // Snapshot load plus catalog merge stays off Tokio workers.
            let snapshot =
                crate::modules::blocking_task("automation.snapshot", move || {
                    core.behavior_snapshot()
                })
                .await?;
            Ok::<Value, ApiError>(snapshot)
        },
    );
}
