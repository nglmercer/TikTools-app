use std::{collections::BTreeMap, sync::Arc};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::{
    control::{
        PluginActionOutcome, PluginConnectionResult, PluginInstallResult, PluginProvisionResult,
    },
    AppCore,
};

use crate::{
    error::ApiError,
    modules::{Empty, OkResult},
    router::ControlRouter,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginIdParams {
    pub plugin_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginListResult {
    pub plugins: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginResult {
    pub plugin: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallParams {
    pub path: String,
    #[serde(default)]
    pub replace_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginOptionsParams {
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginOptionsResult {
    pub source: String,
    pub options: Vec<Value>,
    pub selected: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginProvisionParams {
    pub plugin_id: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallSetParams {
    pub plugin_id: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionParams {
    pub action_type: String,
    #[serde(default)]
    pub config: BTreeMap<String, Value>,
    /// Dry run by default; `live: true` performs the real execution.
    #[serde(default)]
    pub live: bool,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, PluginListResult, _, _>(
        "plugins.list",
        "Safe plugin summaries (never includes settings values)",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<PluginListResult, ApiError>(PluginListResult {
                plugins: core.plugin_list(),
            })
        },
    );
    router.register_typed::<PluginIdParams, PluginResult, _, _>(
        "plugins.get",
        "One plugin summary by id",
        false,
        |core: Arc<AppCore>, params: PluginIdParams| async move {
            core.plugin_get(&params.plugin_id)
                .map(|plugin| PluginResult { plugin })
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginInstallParams, PluginInstallResult, _, _>(
        "plugins.install",
        "Installs a .plugin archive (identity comes from plugin.json)",
        true,
        |core: Arc<AppCore>, params: PluginInstallParams| async move {
            // Archive extraction + filesystem writes stay off Tokio workers.
            tokio::task::spawn_blocking(move || {
                core.plugin_install(&params.path, params.replace_existing)
            })
            .await
            .map_err(|error| ApiError::internal(format!("install worker failed: {error}")))?
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<PluginIdParams, OkResult, _, _>(
        "plugins.uninstall",
        "Removes a user-installed plugin package",
        true,
        |core: Arc<AppCore>, params: PluginIdParams| async move {
            // Recursive removal stays off Tokio workers.
            tokio::task::spawn_blocking(move || core.plugin_uninstall(&params.plugin_id))
                .await
                .map_err(|error| ApiError::internal(format!("uninstall worker failed: {error}")))?
                .map(|()| OkResult::ok())
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginIdParams, OkResult, _, _>(
        "plugins.enable",
        "Marks a plugin installed+enabled and starts its runtime",
        true,
        |core: Arc<AppCore>, params: PluginIdParams| async move {
            core.plugin_set_enabled(&params.plugin_id, true)
                .map(|()| OkResult::ok())
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginIdParams, OkResult, _, _>(
        "plugins.disable",
        "Stops a plugin runtime and marks it disabled",
        true,
        |core: Arc<AppCore>, params: PluginIdParams| async move {
            core.plugin_set_enabled(&params.plugin_id, false)
                .map(|()| OkResult::ok())
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginIdParams, OkResult, _, _>(
        "plugins.start",
        "Starts a plugin runtime without changing persisted state",
        true,
        |core: Arc<AppCore>, params: PluginIdParams| async move {
            core.plugin_start(&params.plugin_id)
                .map(|()| OkResult::ok())
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginIdParams, OkResult, _, _>(
        "plugins.stop",
        "Stops a plugin runtime without changing persisted state",
        true,
        |core: Arc<AppCore>, params: PluginIdParams| async move {
            core.plugin_stop(&params.plugin_id)
                .map(|()| OkResult::ok())
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginIdParams, PluginConnectionResult, _, _>(
        "plugins.health",
        "Probes a plugin health endpoint (schema v3 http.health)",
        false,
        |core: Arc<AppCore>, params: PluginIdParams| async move {
            core.plugin_connection_check(&params.plugin_id)
                .await
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginOptionsParams, PluginOptionsResult, _, _>(
        "plugins.options",
        "Resolves action-type/field option documents",
        false,
        |core: Arc<AppCore>, params: PluginOptionsParams| async move {
            core.plugin_action_options(&params.source)
                .await
                .map(|(options, selected)| PluginOptionsResult {
                    source: params.source,
                    options,
                    selected,
                })
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PluginActionParams, PluginActionOutcome, _, _>(
        "plugins.action.execute",
        "Executes one plugin action (dry run unless live is true)",
        true,
        |core: Arc<AppCore>, params: PluginActionParams| async move {
            let live = params.live;
            core.plugin_action_execute(&params.action_type, params.config, live)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PluginProvisionParams, PluginProvisionResult, _, _>(
        "plugins.token.provision",
        "Mints an API token via the plugin provisioning flow (password is used once, never stored)",
        true,
        |core: Arc<AppCore>, params: PluginProvisionParams| async move {
            core.plugin_token_provision(&params.plugin_id, &params.username, &params.password)
                .await
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginInstallSetParams, OkResult, _, _>(
        "plugins.install.set",
        "Logical installed-state toggle (never touches the filesystem)",
        true,
        |core: Arc<AppCore>, params: PluginInstallSetParams| async move {
            core.plugin_set_installed(&params.plugin_id, params.installed)
                .map(|()| OkResult::ok())
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
}
