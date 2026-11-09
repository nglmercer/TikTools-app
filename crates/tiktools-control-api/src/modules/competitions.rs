//! Competition definitions exposed through the shared control registry.
use crate::{
    error::ApiError,
    modules::{Empty, OkResult},
    router::ControlRouter,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tiktools_competitions::Transition;
pub use tiktools_competitions::{
    ActionExecution, ActionResolution, Definition, Lifecycle, LifecycleState, ResultRecord,
};
pub use tiktools_core::control::{CompetitionSummary, CompetitionSummaryPage};
use tiktools_core::AppCore;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompetitionId {
    pub id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompetitionStopParams {
    pub id: String,
    #[serde(default)]
    pub expected_generation: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompetitionList {
    pub competitions: Vec<Definition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompetitionSummaryParams {
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_revision: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionResultsRequest {
    pub id: String,
    #[serde(default)]
    pub before_generation: Option<u64>,
    #[serde(default)]
    pub limit: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionResults {
    pub results: Vec<ResultRecord>,
    pub next_before_generation: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompetitionActionId {
    pub id: String,
    pub generation: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResolveCompetitionAction {
    pub id: String,
    pub generation: u64,
    pub resolution: ActionResolution,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompetitionActionResult {
    pub action: Option<ActionExecution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelectWinner {
    pub id: String,
    pub winner_id: String,
    pub generation: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ManualCandidates {
    pub generation: u64,
    pub candidates: Vec<tiktools_aggregation::Entry>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_contextual::<CompetitionSummaryParams, tiktools_core::control::CompetitionSummaryPage, _, _>(
        "competitions.summary",
        "Read bounded authorized competition status without definitions or participants",
        |core, context, params| async move {
            let grant = match &context {
                crate::RequestContext::LocalOwner => tiktools_core::collaboration::Grant {
                    capabilities: [tiktools_core::collaboration::Capability::CompetitionsRead].into(),
                    ..Default::default()
                },
                crate::RequestContext::Remote(session) => session.grant()?,
            };
            let budget = match &context {
                crate::RequestContext::Remote(session) => session.connection_limits().map_or(512 * 1024, |v| v.frame_bytes / 2),
                _ => 512 * 1024,
            };
            crate::modules::blocking_task("competitions.summary", move || {
                core.competition_summary_page(&grant, params.after.as_deref(), params.limit.unwrap_or(20), budget, params.include_revision)
            }).await?.map_err(ApiError::from)
        },
    );
    router.register_typed::<Definition, OkResult, _, _>(
        "competitions.create",
        "Create a competition definition",
        true,
        |core: Arc<AppCore>, definition| async move {
            crate::modules::blocking_task("competitions.create", move || {
                core.competition_create(definition)
            })
            .await?
            .map(|_| OkResult::ok())
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<Definition, OkResult, _, _>(
        "competitions.update",
        "Update a competition definition without results",
        true,
        |core: Arc<AppCore>, definition| async move {
            crate::modules::blocking_task("competitions.update", move || {
                core.competition_update(definition)
            })
            .await?
            .map(|_| OkResult::ok())
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<CompetitionId, OkResult, _, _>(
        "competitions.delete",
        "Delete a competition definition without results",
        true,
        |core: Arc<AppCore>, params: CompetitionId| async move {
            crate::modules::blocking_task("competitions.delete", move || {
                core.competition_delete(&params.id)
            })
            .await?
            .map(|_| OkResult::ok())
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<CompetitionActionId, CompetitionActionResult, _, _>(
        "competitions.action",
        "Read winner action status",
        false,
        |core: Arc<AppCore>, params: CompetitionActionId| async move {
            crate::modules::blocking_task("competitions.action", move || {
                core.competition_action(&params.id, params.generation)
            })
            .await?
            .map(|action| CompetitionActionResult { action })
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<CompetitionActionId, OkResult, _, _>(
        "competitions.markActionUncertain",
        "Mark a stranded running winner action uncertain",
        true,
        |core: Arc<AppCore>, params: CompetitionActionId| async move {
            crate::modules::blocking_task("competitions.markActionUncertain", move || {
                core.competition_mark_action_uncertain(&params.id, params.generation)
            })
            .await?
            .map(|_| OkResult::ok())
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<ResolveCompetitionAction, OkResult, _, _>(
        "competitions.resolveAction",
        "Resolve an uncertain winner action without dispatching it",
        true,
        |core: Arc<AppCore>, params: ResolveCompetitionAction| async move {
            crate::modules::blocking_task("competitions.resolveAction", move || {
                core.competition_resolve_action(&params.id, params.generation, params.resolution)
            })
            .await?
            .map(|_| OkResult::ok())
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<CompetitionResultsRequest, CompetitionResults, _, _>(
        "competitions.results",
        "Read committed competition result history",
        false,
        |core: Arc<AppCore>, params: CompetitionResultsRequest| async move {
            crate::modules::blocking_task("competitions.results", move || {
                core.competition_results(
                    &params.id,
                    params.before_generation,
                    params.limit.unwrap_or(10),
                )
            })
            .await?
            .map(|(results, next_before_generation)| CompetitionResults {
                results,
                next_before_generation,
            })
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<CompetitionId, ManualCandidates, _, _>(
        "competitions.candidates",
        "Read tied leaders awaiting manual selection",
        false,
        |core: Arc<AppCore>, params: CompetitionId| async move {
            crate::modules::blocking_task("competitions.candidates", move || {
                core.competition_manual_candidates(&params.id)
            })
            .await?
            .map(|(generation, candidates)| ManualCandidates {
                generation,
                candidates,
            })
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<SelectWinner, ResultRecord, _, _>(
        "competitions.selectWinner",
        "Resolve a closed manual highest-score tie",
        true,
        |core: Arc<AppCore>, params: SelectWinner| async move {
            crate::modules::blocking_task("competitions.selectWinner", move || {
                core.competition_select_winner(&params.id, &params.winner_id, params.generation)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<CompetitionId, ResultRecord, _, _>(
        "competitions.finish",
        "Close and finalize an eligible competition",
        true,
        |core: Arc<AppCore>, params: CompetitionId| async move {
            crate::modules::blocking_task("competitions.finish", move || {
                core.competition_finish(&params.id)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<CompetitionId, LifecycleState, _, _>(
        "competitions.state",
        "Read durable competition lifecycle state",
        false,
        |core: Arc<AppCore>, params: CompetitionId| async move {
            crate::modules::blocking_task("competitions.state", move || {
                core.competition_state(&params.id)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    for (method, transition, local_transition) in [
        (
            "competitions.start",
            tiktools_core::control::CompetitionRemoteTransition::Start,
            Transition::Start,
        ),
        (
            "competitions.resume",
            tiktools_core::control::CompetitionRemoteTransition::Resume,
            Transition::Resume,
        ),
        (
            "competitions.pause",
            tiktools_core::control::CompetitionRemoteTransition::Pause,
            Transition::Pause,
        ),
        (
            "competitions.cancel",
            tiktools_core::control::CompetitionRemoteTransition::Cancel,
            Transition::Cancel,
        ),
    ] {
        router.register_versioned::<CompetitionStopParams, serde_json::Value, _, _>(
            method,
            "Transition a competition with a revision-bound command",
            true,
            move |core: Arc<AppCore>, context, params, metadata| async move {
                crate::modules::blocking_task("competition stop", move || match context {
                    crate::RequestContext::Remote(session) => {
                        if matches!(
                            transition,
                            tiktools_core::control::CompetitionRemoteTransition::Start
                                | tiktools_core::control::CompetitionRemoteTransition::Resume
                        ) {
                            core.start_competition_timer();
                        }
                        let meta = metadata.ok_or_else(|| {
                            ApiError::invalid_params("command metadata is required")
                        })?;
                        let request = tiktools_core::collaboration::CommandRequest {
                            command_id: meta
                                .command_id
                                .ok_or_else(|| ApiError::invalid_params("commandId is required"))?,
                            host_instance_id: meta.host_instance_id.ok_or_else(|| {
                                ApiError::invalid_params("hostInstanceId is required")
                            })?,
                            expected_revision: meta.expected_revision.ok_or_else(|| {
                                ApiError::invalid_params("expectedRevision is required")
                            })?,
                        };
                        let generation = params.expected_generation.ok_or_else(|| {
                            ApiError::invalid_params("expectedGeneration is required")
                        })?;
                        core.competition_remote_command(
                            session.authorization()?,
                            request,
                            &params.id,
                            &generation,
                            transition,
                        )
                        .map_err(ApiError::from)
                    }
                    crate::RequestContext::LocalOwner => {
                        core.start_competition_timer();
                        core.competition_transition(&params.id, local_transition)
                            .and_then(|state| {
                                serde_json::to_value(state).map_err(|_| {
                                    tiktools_core::control::OperationError::internal(
                                        "competition state serialization failed",
                                    )
                                })
                            })
                            .map(|value| {
                                tiktools_core::collaboration::Versioned::new(value, Vec::new())
                            })
                            .map_err(ApiError::from)
                    }
                })
                .await?
            },
        );
        router.set_durable_commands(method, true);
    }
    router.register_typed::<Empty, CompetitionList, _, _>(
        "competitions.list",
        "List competition definitions",
        false,
        |core: Arc<AppCore>, _| async move {
            core.competition_list()
                .map(|competitions| CompetitionList { competitions })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<CompetitionId, Definition, _, _>(
        "competitions.get",
        "Get a competition definition",
        false,
        |core: Arc<AppCore>, params: CompetitionId| async move {
            core.competition_get(&params.id).map_err(ApiError::from)
        },
    );
}
