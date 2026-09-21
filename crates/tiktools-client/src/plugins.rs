//! Typed `plugins.*` methods.

use super::TikToolsClient;
use serde_json::Value;
use tiktools_control_api::modules::plugins::{
    PluginActionOutcome, PluginActionParams, PluginConnectionResult, PluginIdParams,
    PluginInstallParams, PluginInstallResult, PluginInstallSetParams, PluginListResult,
    PluginOptionsParams, PluginOptionsResult, PluginProvisionParams, PluginProvisionResult,
    PluginResult,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::modules::OkResult;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[
    PLUGINS_LIST,
    PLUGINS_GET,
    PLUGINS_INSTALL,
    PLUGINS_UNINSTALL,
    PLUGINS_ENABLE,
    PLUGINS_DISABLE,
    PLUGINS_START,
    PLUGINS_STOP,
    PLUGINS_HEALTH,
    PLUGINS_DIAGNOSTICS,
    PLUGINS_OPTIONS,
    PLUGINS_ACTION_EXECUTE,
    PLUGINS_TOKEN_PROVISION,
    PLUGINS_INSTALL_SET,
];

const PLUGINS_LIST: &str = "plugins.list";
const PLUGINS_GET: &str = "plugins.get";
const PLUGINS_INSTALL: &str = "plugins.install";
const PLUGINS_UNINSTALL: &str = "plugins.uninstall";
const PLUGINS_ENABLE: &str = "plugins.enable";
const PLUGINS_DISABLE: &str = "plugins.disable";
const PLUGINS_START: &str = "plugins.start";
const PLUGINS_STOP: &str = "plugins.stop";
const PLUGINS_HEALTH: &str = "plugins.health";
const PLUGINS_DIAGNOSTICS: &str = "plugins.diagnostics";
const PLUGINS_OPTIONS: &str = "plugins.options";
const PLUGINS_ACTION_EXECUTE: &str = "plugins.action.execute";
const PLUGINS_TOKEN_PROVISION: &str = "plugins.token.provision";
const PLUGINS_INSTALL_SET: &str = "plugins.install.set";

impl TikToolsClient {
    /// Safe plugin summaries (never includes settings values).
    /// RPC method `plugins.list`.
    pub async fn plugins_list(&self) -> Result<PluginListResult, ClientError> {
        self.call(PLUGINS_LIST, Empty::default()).await
    }
    /// One plugin summary by id.
    /// RPC method `plugins.get`.
    pub async fn plugins_get(&self, params: PluginIdParams) -> Result<PluginResult, ClientError> {
        self.call(PLUGINS_GET, params).await
    }
    /// Installs a .plugin archive (identity comes from plugin.json).
    /// RPC method `plugins.install`.
    pub async fn plugins_install(
        &self,
        params: PluginInstallParams,
    ) -> Result<PluginInstallResult, ClientError> {
        self.call(PLUGINS_INSTALL, params).await
    }
    /// Removes a user-installed plugin package.
    /// RPC method `plugins.uninstall`.
    pub async fn plugins_uninstall(&self, params: PluginIdParams) -> Result<OkResult, ClientError> {
        self.call(PLUGINS_UNINSTALL, params).await
    }
    /// Marks a plugin installed+enabled and starts its runtime.
    /// RPC method `plugins.enable`.
    pub async fn plugins_enable(&self, params: PluginIdParams) -> Result<OkResult, ClientError> {
        self.call(PLUGINS_ENABLE, params).await
    }
    /// Stops a plugin runtime and marks it disabled.
    /// RPC method `plugins.disable`.
    pub async fn plugins_disable(&self, params: PluginIdParams) -> Result<OkResult, ClientError> {
        self.call(PLUGINS_DISABLE, params).await
    }
    /// Starts a plugin runtime without changing persisted state.
    /// RPC method `plugins.start`.
    pub async fn plugins_start(&self, params: PluginIdParams) -> Result<OkResult, ClientError> {
        self.call(PLUGINS_START, params).await
    }
    /// Stops a plugin runtime without changing persisted state.
    /// RPC method `plugins.stop`.
    pub async fn plugins_stop(&self, params: PluginIdParams) -> Result<OkResult, ClientError> {
        self.call(PLUGINS_STOP, params).await
    }
    /// Probes a plugin health endpoint (schema v3 http.health).
    /// RPC method `plugins.health`.
    pub async fn plugins_health(
        &self,
        params: PluginIdParams,
    ) -> Result<PluginConnectionResult, ClientError> {
        self.call(PLUGINS_HEALTH, params).await
    }
    /// In-memory plugin runtime diagnostics: last event, poll drops, hotkey sync state.
    /// RPC method `plugins.diagnostics`.
    pub async fn plugins_diagnostics(&self) -> Result<Value, ClientError> {
        self.call(PLUGINS_DIAGNOSTICS, Empty::default()).await
    }
    /// Resolves action-type/field option documents.
    /// RPC method `plugins.options`.
    pub async fn plugins_options(
        &self,
        params: PluginOptionsParams,
    ) -> Result<PluginOptionsResult, ClientError> {
        self.call(PLUGINS_OPTIONS, params).await
    }
    /// Executes one plugin action (dry run unless live is true).
    /// RPC method `plugins.action.execute`.
    pub async fn plugins_action_execute(
        &self,
        params: PluginActionParams,
    ) -> Result<PluginActionOutcome, ClientError> {
        self.call(PLUGINS_ACTION_EXECUTE, params).await
    }
    /// Mints an API token via the plugin provisioning flow (password is used once, never stored).
    /// RPC method `plugins.token.provision`.
    pub async fn plugins_token_provision(
        &self,
        params: PluginProvisionParams,
    ) -> Result<PluginProvisionResult, ClientError> {
        self.call(PLUGINS_TOKEN_PROVISION, params).await
    }
    /// Logical installed-state toggle (never touches the filesystem).
    /// RPC method `plugins.install.set`.
    pub async fn plugins_install_set(
        &self,
        params: PluginInstallSetParams,
    ) -> Result<OkResult, ClientError> {
        self.call(PLUGINS_INSTALL_SET, params).await
    }
}
