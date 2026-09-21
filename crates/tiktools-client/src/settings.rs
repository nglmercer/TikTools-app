//! Typed `settings.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::settings::{
    PluginSettingsGet, PluginSettingsResult, PluginSettingsSet,
};
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[
    PLUGINS_SETTINGS_GET,
    PLUGINS_SETTINGS_SET,
    PLUGINS_SETTINGS_RESET,
];

const PLUGINS_SETTINGS_GET: &str = "plugins.settings.get";
const PLUGINS_SETTINGS_SET: &str = "plugins.settings.set";
const PLUGINS_SETTINGS_RESET: &str = "plugins.settings.reset";

impl TikToolsClient {
    /// Plugin settings schema plus redacted values.
    /// RPC method `plugins.settings.get`.
    pub async fn plugins_settings_get(
        &self,
        params: PluginSettingsGet,
    ) -> Result<PluginSettingsResult, ClientError> {
        self.call(PLUGINS_SETTINGS_GET, params).await
    }
    /// Updates plugin settings (placeholder preserves stored secrets).
    /// RPC method `plugins.settings.set`.
    pub async fn plugins_settings_set(
        &self,
        params: PluginSettingsSet,
    ) -> Result<PluginSettingsResult, ClientError> {
        self.call(PLUGINS_SETTINGS_SET, params).await
    }
    /// Deletes stored settings so schema defaults apply again.
    /// RPC method `plugins.settings.reset`.
    pub async fn plugins_settings_reset(
        &self,
        params: PluginSettingsGet,
    ) -> Result<PluginSettingsResult, ClientError> {
        self.call(PLUGINS_SETTINGS_RESET, params).await
    }
}
