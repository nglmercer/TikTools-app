//! Typed `app.update.*` methods (desktop application updater).

use super::TikToolsClient;
use serde_json::Value;
use tiktools_control_api::modules::updates::{CheckParams, NoUpdateParams};
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[
    APP_UPDATE_STATUS,
    APP_UPDATE_CHECK,
    APP_UPDATE_DOWNLOAD,
    APP_UPDATE_INSTALL,
];

const APP_UPDATE_STATUS: &str = "app.update.status";
const APP_UPDATE_CHECK: &str = "app.update.check";
const APP_UPDATE_DOWNLOAD: &str = "app.update.download";
const APP_UPDATE_INSTALL: &str = "app.update.install";

impl TikToolsClient {
    /// Reads the application updater status.
    /// RPC method `app.update.status`.
    pub async fn app_update_status(&self) -> Result<Value, ClientError> {
        self.call(APP_UPDATE_STATUS, NoUpdateParams {}).await
    }
    /// Checks signed update metadata for a channel.
    /// RPC method `app.update.check`.
    pub async fn app_update_check(&self, params: CheckParams) -> Result<Value, ClientError> {
        self.call(APP_UPDATE_CHECK, params).await
    }
    /// Downloads and verifies the selected update.
    /// RPC method `app.update.download`.
    pub async fn app_update_download(&self) -> Result<Value, ClientError> {
        self.call(APP_UPDATE_DOWNLOAD, NoUpdateParams {}).await
    }
    /// Requests installation; the host requires native desktop confirmation.
    /// RPC method `app.update.install`.
    pub async fn app_update_install(&self) -> Result<Value, ClientError> {
        self.call(APP_UPDATE_INSTALL, NoUpdateParams {}).await
    }
}
