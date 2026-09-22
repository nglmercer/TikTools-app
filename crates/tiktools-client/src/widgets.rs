//! Typed `widgets.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::widgets::{
    WidgetsCopyParams, WidgetsCopyResult, WidgetsStatusResult,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[WIDGETS_STATUS, WIDGETS_COPY_OBS_URL];

const WIDGETS_STATUS: &str = "widgets.status";
const WIDGETS_COPY_OBS_URL: &str = "widgets.copyObsUrl";

impl TikToolsClient {
    /// Resolves OBS widget readiness without touching the browser.
    /// RPC method `widgets.status`.
    pub async fn widgets_status(&self) -> Result<WidgetsStatusResult, ClientError> {
        self.call(WIDGETS_STATUS, Empty::default()).await
    }
    /// Copies one OBS Browser Source URL to the OS clipboard.
    /// RPC method `widgets.copyObsUrl`.
    pub async fn widgets_copy_obs_url(
        &self,
        params: WidgetsCopyParams,
    ) -> Result<WidgetsCopyResult, ClientError> {
        self.call(WIDGETS_COPY_OBS_URL, params).await
    }
}
