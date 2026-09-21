//! Typed `creators.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::creators::{
    CreatorGetParams, CreatorGetResult, CreatorsRecentParams, CreatorsRecentResult,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::modules::OkResult;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[CREATORS_GET, CREATORS_RECENT, CREATORS_HISTORY_CLEAR];

const CREATORS_GET: &str = "creators.get";
const CREATORS_RECENT: &str = "creators.recent";
const CREATORS_HISTORY_CLEAR: &str = "creators.history.clear";

impl TikToolsClient {
    /// Creator record by uniqueId (defaults to the last connected creator).
    /// RPC method `creators.get`.
    pub async fn creators_get(
        &self,
        params: CreatorGetParams,
    ) -> Result<CreatorGetResult, ClientError> {
        self.call(CREATORS_GET, params).await
    }
    /// Recently connected creators, most recent first.
    /// RPC method `creators.recent`.
    pub async fn creators_recent(
        &self,
        params: CreatorsRecentParams,
    ) -> Result<CreatorsRecentResult, ClientError> {
        self.call(CREATORS_RECENT, params).await
    }
    /// Clears creator history and the last-creator pointer.
    /// RPC method `creators.history.clear`.
    pub async fn creators_history_clear(&self) -> Result<OkResult, ClientError> {
        self.call(CREATORS_HISTORY_CLEAR, Empty::default()).await
    }
}
