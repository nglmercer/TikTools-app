//! Typed `analytics.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::analytics::{AnalyticsSummaryParams, AnalyticsSummaryResult};
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[ANALYTICS_SUMMARY];

const ANALYTICS_SUMMARY: &str = "analytics.summary";

impl TikToolsClient {
    /// Per-day analytics summary for a creator (defaults to the current one).
    /// RPC method `analytics.summary`.
    pub async fn analytics_summary(
        &self,
        params: AnalyticsSummaryParams,
    ) -> Result<AnalyticsSummaryResult, ClientError> {
        self.call(ANALYTICS_SUMMARY, params).await
    }
}
