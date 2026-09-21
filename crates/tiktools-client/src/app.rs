//! Typed `app.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::app::{AppStateGetParams, AppStateResult, AppStateSetParams};
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[APP_STATE_GET, APP_STATE_SET];

const APP_STATE_GET: &str = "app.state.get";
const APP_STATE_SET: &str = "app.state.set";

impl TikToolsClient {
    /// Reads app state (all keys, or a filtered subset).
    /// RPC method `app.state.get`.
    pub async fn app_state_get(
        &self,
        params: AppStateGetParams,
    ) -> Result<AppStateResult, ClientError> {
        self.call(APP_STATE_GET, params).await
    }
    /// Writes one app state key.
    /// RPC method `app.state.set`.
    pub async fn app_state_set(
        &self,
        params: AppStateSetParams,
    ) -> Result<AppStateResult, ClientError> {
        self.call(APP_STATE_SET, params).await
    }
}
