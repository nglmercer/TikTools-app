//! Typed `points.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::points::{
    PartialPointsConfig, PointAward, PointsAdjustParams, PointsConfig, PointsLeaderboardParams,
    PointsLeaderboardResult, PointsResetParams, PointsViewerParams, PointsViewerResult,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::modules::OkResult;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[
    POINTS_CONFIG_GET,
    POINTS_CONFIG_SET,
    POINTS_VIEWER_GET,
    POINTS_ADJUST,
    POINTS_LEADERBOARD,
    POINTS_RESET,
];

const POINTS_CONFIG_GET: &str = "points.config.get";
const POINTS_CONFIG_SET: &str = "points.config.set";
const POINTS_VIEWER_GET: &str = "points.viewer.get";
const POINTS_ADJUST: &str = "points.adjust";
const POINTS_LEADERBOARD: &str = "points.leaderboard";
const POINTS_RESET: &str = "points.reset";

impl TikToolsClient {
    /// Current points configuration.
    /// RPC method `points.config.get`.
    pub async fn points_config_get(&self) -> Result<PointsConfig, ClientError> {
        self.call(POINTS_CONFIG_GET, Empty::default()).await
    }
    /// Updates points configuration (partial object).
    /// RPC method `points.config.set`.
    pub async fn points_config_set(
        &self,
        params: PartialPointsConfig,
    ) -> Result<PointsConfig, ClientError> {
        self.call(POINTS_CONFIG_SET, params).await
    }
    /// One viewer points record by uniqueId.
    /// RPC method `points.viewer.get`.
    pub async fn points_viewer_get(
        &self,
        params: PointsViewerParams,
    ) -> Result<PointsViewerResult, ClientError> {
        self.call(POINTS_VIEWER_GET, params).await
    }
    /// Manual points adjustment (creates the viewer record if missing).
    /// RPC method `points.adjust`.
    pub async fn points_adjust(
        &self,
        params: PointsAdjustParams,
    ) -> Result<PointAward, ClientError> {
        self.call(POINTS_ADJUST, params).await
    }
    /// Points leaderboard, highest first.
    /// RPC method `points.leaderboard`.
    pub async fn points_leaderboard(
        &self,
        params: PointsLeaderboardParams,
    ) -> Result<PointsLeaderboardResult, ClientError> {
        self.call(POINTS_LEADERBOARD, params).await
    }
    /// Resets points for one viewer, or all viewers when uniqueId is omitted.
    /// RPC method `points.reset`.
    pub async fn points_reset(&self, params: PointsResetParams) -> Result<OkResult, ClientError> {
        self.call(POINTS_RESET, params).await
    }
}
