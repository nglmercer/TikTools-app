use std::{collections::BTreeMap, sync::Arc};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::{control::PluginSettingsResult, AppCore};

use crate::{error::ApiError, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingsGet {
    pub plugin_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingsSet {
    pub plugin_id: String,
    pub values: BTreeMap<String, Value>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<PluginSettingsGet, PluginSettingsResult, _, _>(
        "plugins.settings.get",
        "Plugin settings schema plus redacted values",
        false,
        |core: Arc<AppCore>, params: PluginSettingsGet| async move {
            core.plugin_settings(&params.plugin_id)
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginSettingsSet, PluginSettingsResult, _, _>(
        "plugins.settings.set",
        "Updates plugin settings (placeholder preserves stored secrets)",
        true,
        |core: Arc<AppCore>, params: PluginSettingsSet| async move {
            core.plugin_settings_save(&params.plugin_id, params.values)
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
    router.register_typed::<PluginSettingsGet, PluginSettingsResult, _, _>(
        "plugins.settings.reset",
        "Deletes stored settings so schema defaults apply again",
        true,
        |core: Arc<AppCore>, params: PluginSettingsGet| async move {
            core.plugin_settings_reset(&params.plugin_id)
                .map_err(|error| ApiError::from(error).scoped_not_found("plugin_not_found"))
        },
    );
}
