use std::{str::FromStr, sync::Arc};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use tiktools_core::control::{
    AutomationEnablementMiss, AutomationSetEnablementResult, ScriptAnalysisResult,
};
use tiktools_core::{
    collaboration::Versioned,
    control::{AutomationKind, AutomationMutation},
    AppCore,
};

use crate::{
    command_metadata::{versioned_command, CommandDispatch},
    error::ApiError,
    modules::Empty,
    router::ControlRouter,
};

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

/// A saved definition only; caller JSON cannot provide authority or a candidate.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationManifestParams {
    pub kind: AutomationKind,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AutomationReviewAuthority {
    Owner,
    Delegated {
        #[serde(rename = "principalId")]
        principal_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationReviewProvenance {
    pub authority: AutomationReviewAuthority,
    pub generation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationReviewDependency {
    pub id: String,
    pub record: Option<Value>,
    pub provenance: Option<AutomationReviewProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationReviewResult {
    pub kind: AutomationKind,
    pub id: String,
    pub record: Option<Value>,
    pub provenance: Option<AutomationReviewProvenance>,
    pub dependencies: Vec<AutomationReviewDependency>,
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

/// One record in a bulk enablement pass.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationEnablementUpdate {
    /// `event` or `action`.
    pub kind: String,
    /// Record id.
    pub id: String,
    /// Desired enabled state.
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationSetEnablementParams {
    /// Batch of enable/disable updates (at most 512).
    pub updates: Vec<AutomationEnablementUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationContextParams {
    /// Return the last envelope of this type instead of the global last
    /// event. Callers emulating one trigger pass it so previews replay real
    /// data of the right shape.
    #[serde(default)]
    pub event_type: Option<String>,
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
pub struct AutomationFireParams {
    /// Trigger to fire (`tiktok.gift`, ...). Required: firing needs a type.
    pub trigger: String,
    /// Optional custom envelope used as-is (fresh id/timestamp, type forced
    /// to `trigger`). Absent: last live envelope of the trigger, else the
    /// per-type sample.
    #[serde(default)]
    pub event: Option<Value>,
    /// Optional editor draft to test-fire: runs when its trigger and
    /// filters match, even when unsaved or disabled. Its saved twin, if
    /// any, is skipped so one fire never executes the same event twice.
    #[serde(default)]
    pub record: Option<Value>,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationAdoptParams {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    pub acknowledge_authority: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_versions: Option<Vec<tiktools_core::collaboration::ResourceVersion>>,
}
pub fn register(router: &mut ControlRouter) {
    router.register_versioned::<AutomationGetParams, Value, _, _>(
        "automation.source.get",
        "Reads an explicitly shared automation definition",
        false,
        |core, context, params, meta| async move {
            owner_metadata(crate::RequestContext::LocalOwner, meta, false)?;
            let crate::RequestContext::Remote(session) = context else {
                return Err(crate::context::forbidden());
            };
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            let auth = session.authorization()?;
            let budget = session.connection_limits().map_or(128 * 1024, |limits| {
                (limits.frame_bytes / 4).min(128 * 1024)
            });
            crate::modules::blocking_task("automation.source.get", move || {
                core.automation_shared_source(kind, &params.id, &auth, budget)
            })
            .await?
            .map_err(ApiError::from)
            .map_err(|e| e.scoped_not_found("automation_not_found"))
        },
    );
    router.register_versioned::<AutomationManifestParams, Value, _, _>(
        "automation.delegation.preflight",
        "Owner-only advisory reusable-grant check; never an execution permit",
        false,
        |core, context, params, meta| async move {
            owner_metadata(context, meta, false)?;
            crate::modules::blocking_task("automation.delegation.preflight", move || {
                core.automation_delegation_preflight(params.kind, &params.id)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<AutomationManifestParams, Value, _, _>(
        "automation.manifest.get",
        "Owner-only exact definition and delegated effect manifest preview; never approval",
        false,
        |core, context, params, meta| async move {
            owner_metadata(context, meta, false)?;
            crate::modules::blocking_task("automation.manifest.get", move || {
                core.automation_execution_manifest(params.kind, &params.id)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<AutomationGetParams, AutomationReviewResult, _, _>(
        "automation.review",
        "Owner-only atomic definition, provenance and dependency review",
        false,
        |core, context, params, meta| async move {
            owner_metadata(context, meta, false)?;
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            let result = crate::modules::blocking_task("automation.review", move || {
                core.automation_review(kind, &params.id)
            })
            .await??;
            let value = serde_json::from_value(result.value)
                .map_err(|_| ApiError::new("internal", "invalid behavior review response"))?;
            Ok(tiktools_core::collaboration::Versioned::new(
                value,
                result.resource_versions,
            ))
        },
    );
    router.register_versioned::<AutomationAdoptParams, Value, _, _>(
        "automation.adopt",
        "Explicit owner authority transfer; always leaves delegated record disabled",
        true,
        |core, context, params, meta| async move {
            if !matches!(context, crate::RequestContext::LocalOwner) {
                return Err(crate::context::forbidden());
            }
            if !params.acknowledge_authority {
                return Err(ApiError::new(
                    "confirmation_required",
                    "review and acknowledge owner authority transfer",
                ));
            }
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            mutate(
                core,
                context,
                meta,
                AutomationMutation::Adopt {
                    kind,
                    id: params.id,
                    reviewed_versions: params.reviewed_versions,
                },
            )
            .await
        },
    );
    router.set_durable_commands("automation.adopt", true);
    router.register_versioned::<AutomationGetParams, Value, _, _>(
        "automation.authority.get",
        "Owner-only retained behavior provenance and generation",
        false,
        |core, context, params, meta| async move {
            owner_metadata(context, meta, false)?;
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            crate::modules::blocking_task("automation.authority.get", move || {
                core.automation_authority(kind, &params.id)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<AutomationListParams, AutomationListResult, _, _>(
        "automation.list",
        "Behavior events and actions (kind: event, action, all)",
        false,
        |core, context, params, meta| async move {
            let scope = read_scope(context, meta)?;
            let kind = match params
                .kind
                .as_deref()
                .unwrap_or("all")
                .trim()
                .to_ascii_lowercase()
                .as_str()
            {
                "event" | "events" => Some(AutomationKind::Event),
                "action" | "actions" => Some(AutomationKind::Action),
                "all" | "" => None,
                _ => return Err(ApiError::invalid_params("unknown automation kind")),
            };
            Ok(
                crate::modules::blocking_task("automation.list", move || match scope {
                    Some((grant, bytes)) => core.automation_list_scoped(&grant, kind, bytes),
                    None => core.automation_list_versioned(kind),
                })
                .await??
                .map(|(events, actions)| AutomationListResult { events, actions }),
            )
        },
    );
    router.register_versioned::<AutomationGetParams, Value, _, _>(
        "automation.get",
        "One behavior event or action by id",
        false,
        |core, context, params, meta| async move {
            let scope = read_scope(context, meta)?;
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            let result = crate::modules::blocking_task("automation.get", move || match scope {
                Some((grant, bytes)) => core.automation_get_scoped(&grant, kind, &params.id, bytes),
                None => core.automation_get_versioned(kind, &params.id),
            })
            .await??;
            match result.value {
                Some(value) => Ok(tiktools_core::collaboration::Versioned::new(
                    value,
                    result.resource_versions,
                )),
                None => {
                    let mut error =
                        ApiError::new("automation_not_found", "automation record does not exist");
                    error.data =
                        Some(serde_json::json!({"resourceVersions":result.resource_versions}));
                    Err(error)
                }
            }
        },
    );
    router.register_versioned::<AutomationCreateParams, Value, _, _>(
        "automation.create",
        "Creates a behavior event or action (id generated when omitted)",
        true,
        |core, context, params, meta| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            mutate(
                core,
                context,
                meta,
                AutomationMutation::Create {
                    kind,
                    record: params.record,
                },
            )
            .await
        },
    );
    router.register_versioned::<AutomationUpdateParams, Value, _, _>(
        "automation.update",
        "Replaces a behavior event or action (must already exist)",
        true,
        |core, context, params, meta| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            mutate(
                core,
                context,
                meta,
                AutomationMutation::Update {
                    kind,
                    id: params.id,
                    record: params.record,
                },
            )
            .await
        },
    );
    router.register_versioned::<AutomationGetParams, AutomationDeleteResult, _, _>(
        "automation.delete",
        "Deletes a behavior event or action",
        true,
        |core, context, params, meta| async move {
            let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
            let id = params.id;
            mutate(
                core,
                context,
                meta,
                AutomationMutation::Delete {
                    kind,
                    id: id.clone(),
                },
            )
            .await
            .map(|value| {
                value.map(|result| AutomationDeleteResult {
                    id,
                    deleted: result
                        .get("deleted")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                })
            })
        },
    );
    for (method, enabled) in [("automation.enable", true), ("automation.disable", false)] {
        router.register_versioned::<AutomationGetParams, Value, _, _>(
            method,
            "Sets behavior event or action enablement",
            true,
            move |core, context, params, meta| async move {
                let kind = parse_kind(params.kind.as_deref(), AutomationKind::Event)?;
                mutate(
                    core,
                    context,
                    meta,
                    AutomationMutation::SetEnabled {
                        kind,
                        id: params.id,
                        enabled,
                    },
                )
                .await
            },
        );
    }
    for method in [
        "automation.create",
        "automation.update",
        "automation.adopt",
        "automation.delete",
        "automation.enable",
        "automation.disable",
    ] {
        router.set_durable_commands(method, true);
    }
    router
        .register_versioned::<AutomationSetEnablementParams, AutomationSetEnablementResult, _, _>(
            "automation.set_enablement",
            "Atomic owner bulk enablement across behavior families",
            true,
            |core, context, params, meta| async move {
                owner_metadata(context, meta, false)?;
                if params.updates.len() > 512 {
                    return Err(ApiError::invalid_params(
                        "at most 512 enablement updates per request",
                    ));
                }
                let updates = params
                    .updates
                    .into_iter()
                    .map(|u| {
                        AutomationKind::from_str(&u.kind)
                            .map(|kind| (kind, u.id, u.enabled))
                            .map_err(ApiError::from)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                crate::modules::blocking_task("automation.set_enablement", move || {
                    core.automation_set_enablement_versioned(&updates)
                })
                .await?
                .map_err(ApiError::from)
            },
        );
    router.set_concurrency(
        "automation.set_enablement",
        crate::registry::ConcurrencyStrategy::ExclusiveTransition,
    );
    router.register_typed::<AutomationContextParams, AutomationContextResult, _, _>(
        "automation.context",
        "Last automation event observed, for context panels and previews (eventType narrows to one trigger)",
        false,
        |core: Arc<AppCore>, params: AutomationContextParams| async move {
            let (event, captured_at) = match params.event_type.as_deref() {
                Some(event_type) if !event_type.trim().is_empty() => {
                    core.automation_context_for(event_type)
                }
                _ => core.automation_context(),
            };
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
    router.register_typed::<AutomationFireParams, Value, _, _>(
        "automation.fire",
        "Fires a synthetic event through the live pipeline: really triggers matching events and executes their actions (no dry run)",
        true,
        |core: Arc<AppCore>, params: AutomationFireParams| async move {
            let trigger = params.trigger.trim();
            if trigger.is_empty() {
                return Err::<Value, ApiError>(ApiError::invalid_params(
                    "automation.fire needs a `trigger`",
                ));
            }
            if params.event.as_ref().is_some_and(|event| !event.is_object()) {
                return Err::<Value, ApiError>(ApiError::invalid_params(
                    "automation.fire `event` must be an object",
                ));
            }
            if params.record.as_ref().is_some_and(|record| !record.is_object()) {
                return Err::<Value, ApiError>(ApiError::invalid_params(
                    "automation.fire `record` must be an object",
                ));
            }
            Ok::<Value, ApiError>(
                core.fire_automation_event(trigger, params.event.as_ref(), params.record.as_ref())
                    .await,
            )
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
    router.register_versioned::<Empty, Value, _, _>(
        "automation.snapshot",
        "Merged owner behavior snapshot and runtime catalog",
        false,
        |core, context, _, meta| async move {
            owner_metadata(context, meta, false)?;
            crate::modules::blocking_task("automation.snapshot", move || {
                core.behavior_snapshot_versioned()
            })
            .await?
            .map_err(ApiError::from)
        },
    );
}

fn owner_metadata(
    context: crate::RequestContext,
    metadata: Option<crate::RequestMetadata>,
    allow_revision: bool,
) -> Result<Option<String>, ApiError> {
    if !matches!(context, crate::RequestContext::LocalOwner) {
        return Err(crate::context::forbidden());
    }
    let metadata = metadata.unwrap_or_default();
    if metadata.command_id.is_some() || metadata.host_instance_id.is_some() {
        return Err(ApiError::invalid_params(
            "this automation operation does not accept durable command metadata",
        ));
    }
    if !allow_revision && metadata.expected_revision.is_some() {
        return Err(ApiError::invalid_params(
            "this automation operation does not accept a single expectedRevision",
        ));
    }
    Ok(metadata.expected_revision)
}

fn read_scope(
    context: crate::RequestContext,
    metadata: Option<crate::RequestMetadata>,
) -> Result<Option<(tiktools_core::collaboration::Grant, usize)>, ApiError> {
    owner_metadata(crate::RequestContext::LocalOwner, metadata, false)?;
    match context {
        crate::RequestContext::LocalOwner => Ok(None),
        crate::RequestContext::Remote(session) => Ok(Some((
            session.grant()?,
            session
                .connection_limits()
                .map_or(256 * 1024, |l| l.frame_bytes / 2),
        ))),
    }
}

async fn mutate(
    core: Arc<AppCore>,
    context: crate::RequestContext,
    metadata: Option<crate::RequestMetadata>,
    operation: AutomationMutation,
) -> Result<Versioned<Value>, ApiError> {
    crate::modules::blocking_task("automation.mutate", move || {
        match versioned_command(context, metadata, crate::context::forbidden)? {
            CommandDispatch::Durable { caller, request } => core
                .automation_command(caller, request, operation)
                .map_err(ApiError::from)
                .map_err(|e| e.scoped_not_found("automation_not_found")),
            CommandDispatch::Local { expected_revision } => {
                let expected = expected_revision.as_deref();
                match operation {
                    AutomationMutation::Adopt { .. } => {
                        return Err(ApiError::invalid_params(
                            "adoption requires durable command metadata",
                        ))
                    }
                    AutomationMutation::Create { kind, record } => {
                        core.automation_create_versioned(kind, record, expected)
                    }
                    AutomationMutation::Update { kind, id, record } => {
                        core.automation_update_versioned(kind, &id, record, expected)
                    }
                    AutomationMutation::Delete { kind, id } => core
                        .automation_delete_versioned(kind, &id, expected)
                        .map(|v| v.map(|deleted| serde_json::json!({"id":id,"deleted":deleted}))),
                    AutomationMutation::SetEnabled { kind, id, enabled } => {
                        core.automation_set_enabled_versioned(kind, &id, enabled, expected)
                    }
                }
                .map_err(ApiError::from)
                .map_err(|e| e.scoped_not_found("automation_not_found"))
            }
        }
    })
    .await?
}
