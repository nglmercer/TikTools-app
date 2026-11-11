use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::AppCore;
use tiktools_core::{collaboration::Versioned, control::PointsAdjustment};
pub use tiktools_core::{
    ipc::messages::{PartialPointsConfig, PointsConfig},
    services::PointAward,
};

use crate::{
    command_metadata::{additive_command, CommandDispatch},
    error::ApiError,
    modules::{Empty, OkResult},
    router::ControlRouter,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsViewerParams {
    pub unique_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsAdjustParams {
    pub unique_id: String,
    pub delta: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsInfo {
    pub currency_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PointsViewParams {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub unique_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsViewResult {
    pub currency_name: String,
    pub viewers: Vec<Value>,
    pub selected_id: Option<String>,
    pub selected: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsLeaderboardParams {
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsLeaderboardResult {
    pub viewers: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsViewerResult {
    pub viewer: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointsResetParams {
    #[serde(default)]
    pub unique_id: Option<String>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_versioned::<PointsViewParams, PointsViewResult, _, _>(
        "points.view",
        "Committed bounded leaderboard and selected viewer",
        false,
        |core, context, params, _| async move {
            let grant = match &context {
                crate::RequestContext::LocalOwner => tiktools_core::collaboration::Grant {
                    capabilities: [tiktools_core::collaboration::Capability::PointsRead].into(),
                    ..Default::default()
                },
                crate::RequestContext::Remote(session) => session.grant()?,
            };
            let bytes = match &context {
                crate::RequestContext::Remote(s) => s
                    .connection_limits()
                    .map_or(512 * 1024, |l| l.frame_bytes / 2),
                _ => 512 * 1024,
            };
            let result = super::blocking_task("points.view", move || {
                core.points_display_scoped(&grant, params.limit, params.unique_id.as_deref(), bytes)
            })
            .await??;
            let value: PointsViewResult = serde_json::from_value(result.value.clone())
                .map_err(|_| ApiError::internal("invalid points projection"))?;
            Ok(result.map(|_| value))
        },
    );
    router.register_contextual::<Empty, PointsInfo, _, _>(
        "points.info",
        "Points unit label for confirmation; no earning configuration",
        |core, _, _| async move {
            let currency_name = core.points_config().currency_name;
            if !tiktools_core::control::valid_currency(&currency_name) {
                return Err(ApiError::internal("points unit label is unavailable"));
            }
            Ok(PointsInfo { currency_name })
        },
    );
    router.register_typed::<Empty, PointsConfig, _, _>(
        "points.config.get",
        "Current points configuration",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<PointsConfig, ApiError>(core.points_config())
        },
    );
    // Params ARE the partial config object, e.g. {"pointsPerChat": 2}.
    router.register_typed::<PartialPointsConfig, PointsConfig, _, _>(
        "points.config.set",
        "Updates points configuration (partial object)",
        true,
        |core: Arc<AppCore>, params: PartialPointsConfig| async move {
            // Synchronous SQLite write: off the Tokio workers.
            crate::modules::blocking_task("points.config.set", move || {
                core.points_update_config(params)
            })
            .await
        },
    );
    router.register_versioned::<PointsViewerParams, PointsViewerResult, _, _>(
        "points.viewer.get",
        "One viewer points record by uniqueId",
        false,
        |core: Arc<AppCore>, context, params: PointsViewerParams, _metadata| async move {
            if let crate::RequestContext::Remote(session) = context {
                let grant = session.grant()?;
                let bytes = session
                    .connection_limits()
                    .map_or(512 * 1024, |l| l.frame_bytes / 2);
                let result = crate::modules::blocking_task("points.viewer.get", move || {
                    core.points_viewer_scoped(&grant, &params.unique_id, bytes)
                })
                .await??;
                return match result.value {
                    Some(viewer) => Ok(Versioned::new(
                        PointsViewerResult { viewer },
                        result.resource_versions,
                    )),
                    None => {
                        let mut error =
                            ApiError::new("viewer_not_found", "viewer has no points record");
                        error.data =
                            Some(serde_json::json!({"resourceVersions":result.resource_versions}));
                        Err(error)
                    }
                };
            }
            // A cold lookup or pending reset can wait for the owned writer.
            // Never hold a Tokio network worker while waiting on that gate.
            crate::modules::blocking_task("points.viewer.get", move || {
                core.points_viewer(&params.unique_id)
            })
            .await?
            .map(|viewer| Versioned::new(PointsViewerResult { viewer }, vec![]))
            .map_err(|error| ApiError::from(error).scoped_not_found("viewer_not_found"))
        },
    );
    router.register_versioned::<PointsAdjustParams, PointAward, _, _>(
        "points.adjust",
        "Manual points adjustment (creates the viewer record if missing)",
        true,
        |core: Arc<AppCore>, context, params: PointsAdjustParams, metadata| async move {
            match additive_command(context, metadata, || {
                ApiError::invalid_params("points.adjust requires commandId and hostInstanceId")
            })? {
                CommandDispatch::Durable { caller, request } => {
                    let result = crate::modules::blocking_task("points.adjust", move || {
                        core.points_command(
                            caller,
                            request,
                            PointsAdjustment {
                                expected_currency: params.expected_currency,
                                unique_id: params.unique_id,
                                delta: params.delta,
                            },
                        )
                    })
                    .await??;
                    let award = serde_json::from_value::<PointAward>(result.value.clone())
                        .map_err(|_| ApiError::internal("invalid points command result"))?;
                    return Ok(result.map(|_| award));
                }
                CommandDispatch::Local { .. } => {}
            }
            if params.expected_currency.is_some() {
                return Err(ApiError::invalid_params(
                    "expectedCurrency requires durable command metadata",
                ));
            }
            // Synchronous SQLite write: off the Tokio workers.
            crate::modules::blocking_task("points.adjust", move || {
                core.points_adjust(&params.unique_id, params.delta)
            })
            .await?
            .map(|award| Versioned::new(award, vec![]))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<PointsLeaderboardParams, PointsLeaderboardResult, _, _>(
        "points.leaderboard",
        "Points leaderboard, highest first",
        false,
        |core: Arc<AppCore>, context, params: PointsLeaderboardParams, _metadata| async move {
            if let crate::RequestContext::Remote(session) = context {
                let grant = session.grant()?;
                let bytes = session
                    .connection_limits()
                    .map_or(512 * 1024, |l| l.frame_bytes / 2);
                return crate::modules::blocking_task("points.leaderboard", move || {
                    core.points_leaderboard_scoped(&grant, params.limit, bytes)
                })
                .await?
                .map(|result| result.map(|viewers| PointsLeaderboardResult { viewers }))
                .map_err(ApiError::from);
            }
            Ok(Versioned::new(
                PointsLeaderboardResult {
                    viewers: core.points_leaderboard(params.limit),
                },
                vec![],
            ))
        },
    );
    router.register_typed::<PointsResetParams, OkResult, _, _>(
        "points.reset",
        "Resets points for one viewer, or all viewers when uniqueId is omitted",
        true,
        |core: Arc<AppCore>, params: PointsResetParams| async move {
            // Synchronous SQLite write: off the Tokio workers.
            crate::modules::blocking_task("points.reset", move || {
                core.points_reset(params.unique_id.as_deref());
            })
            .await?;
            Ok::<OkResult, ApiError>(OkResult::ok())
        },
    );
    router.set_durable_commands("points.adjust", true);
    router.set_concurrency(
        "points.adjust",
        crate::registry::ConcurrencyStrategy::Additive,
    );
    for method in ["points.viewer.get", "points.leaderboard", "points.adjust"] {
        router.omit_unversioned_owner_metadata(method);
    }
}
