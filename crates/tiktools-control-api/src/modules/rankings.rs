//! Ranking operations; state is owned by the headless AppCore.
use crate::{
    error::ApiError,
    modules::{Empty, OkResult},
    router::ControlRouter,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub use tiktools_aggregation::{metrics::MetricDescriptor, model::Definition, Snapshot};
pub use tiktools_core::control::{ContributionOutcome, ExternalContribution};
use tiktools_core::AppCore;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RankingId {
    pub id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RankingList {
    pub rankings: Vec<Definition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetricList {
    pub metrics: Vec<MetricDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RankingUpdate {
    pub definition: Definition,
    #[serde(default)]
    pub reset: bool,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, MetricList, _, _>(
        "rankings.metrics",
        "List available native delta metrics",
        false,
        |_core: Arc<AppCore>, _| async move {
            Ok::<_, ApiError>(MetricList {
                metrics: tiktools_aggregation::metrics::descriptors(),
            })
        },
    );
    router.register_typed::<Definition, OkResult, _, _>(
        "rankings.create",
        "Create a ranking definition",
        true,
        |core: Arc<AppCore>, params| async move {
            crate::modules::blocking_task("rankings.create", move || core.ranking_create(params))
                .await?
                .map(|_| OkResult::ok())
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<RankingUpdate, OkResult, _, _>(
        "rankings.update",
        "Update a paused ranking; score changes require reset",
        true,
        |core: Arc<AppCore>, params: RankingUpdate| async move {
            crate::modules::blocking_task("rankings.update", move || {
                core.ranking_update(params.definition, params.reset)
            })
            .await?
            .map(|_| OkResult::ok())
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, RankingList, _, _>(
        "rankings.list",
        "List rankings",
        false,
        |core: Arc<AppCore>, _| async move {
            Ok::<_, ApiError>(RankingList {
                rankings: core.ranking_list(),
            })
        },
    );
    router.register_typed::<RankingId, Definition, _, _>(
        "rankings.get",
        "Get a ranking definition",
        false,
        |core: Arc<AppCore>, params: RankingId| async move {
            core.ranking_get(&params.id).map_err(ApiError::from)
        },
    );
    router.register_typed::<RankingId, Snapshot, _, _>(
        "rankings.snapshot",
        "Get a ranking snapshot",
        false,
        |core: Arc<AppCore>, params: RankingId| async move {
            core.ranking_snapshot(&params.id).map_err(ApiError::from)
        },
    );
    router.register_typed::<RankingId, OkResult, _, _>(
        "rankings.delete",
        "Delete a ranking",
        true,
        |core: Arc<AppCore>, params: RankingId| async move {
            crate::modules::blocking_task("rankings.delete", move || {
                core.ranking_delete(&params.id)
            })
            .await?
            .map(|_| OkResult::ok())
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<ExternalContribution, ContributionOutcome, _, _>(
        "rankings.contribute",
        "Apply an idempotent plugin or custom ranking contribution",
        true,
        |core: Arc<AppCore>, params| async move {
            crate::modules::blocking_task("rankings.contribute", move || {
                core.ranking_contribute(params)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<RankingId, OkResult, _, _>(
        "rankings.reset",
        "Reset a ranking",
        true,
        |core: Arc<AppCore>, params: RankingId| async move {
            crate::modules::blocking_task("rankings.reset", move || core.ranking_reset(&params.id))
                .await?
                .map(|_| OkResult::ok())
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<RankingId, OkResult, _, _>(
        "rankings.pause",
        "Pause a ranking",
        true,
        |core: Arc<AppCore>, params: RankingId| async move {
            crate::modules::blocking_task("rankings.pause", move || {
                core.ranking_pause(&params.id, true)
            })
            .await?
            .map(|_| OkResult::ok())
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<RankingId, OkResult, _, _>(
        "rankings.resume",
        "Resume a ranking",
        true,
        |core: Arc<AppCore>, params: RankingId| async move {
            crate::modules::blocking_task("rankings.resume", move || {
                core.ranking_pause(&params.id, false)
            })
            .await?
            .map(|_| OkResult::ok())
            .map_err(ApiError::from)
        },
    );
}
