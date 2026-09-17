use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::AppCore;

use crate::{error::ApiError, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsSummaryParams {
    pub creator_unique_id: Option<String>,
    pub start_day: Option<i64>,
    pub end_day: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsSummaryResult {
    pub summary: Option<Value>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<AnalyticsSummaryParams, AnalyticsSummaryResult, _, _>(
        "analytics.summary",
        "Per-day analytics summary for a creator (defaults to the current one)",
        false,
        |core: Arc<AppCore>, params: AnalyticsSummaryParams| async move {
            // Day-range SQLite aggregations stay off Tokio workers.
            let summary = crate::modules::blocking_task("analytics.summary", move || {
                core.analytics_summary(
                    params.creator_unique_id,
                    params.start_day,
                    params.end_day,
                    params.limit,
                )
            })
            .await?;
            Ok::<AnalyticsSummaryResult, ApiError>(AnalyticsSummaryResult { summary })
        },
    );
}
