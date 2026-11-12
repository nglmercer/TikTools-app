//! Plugin operation contracts, summary reads and package entry points.
//!
//! Focused child modules share the existing AppCore state and invocation gate;
//! source reservation, runtime effects and confirmation retain their ordering.

mod actions;
mod activation;
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
mod durable;
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
mod local_sync;
#[cfg(test)]
mod tests;
mod ui;

#[cfg(all(
    feature = "cloud-auth",
    feature = "persistence",
    feature = "plugin-install"
))]
pub(crate) use local_sync::with_fenced_plugin_removal;
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
pub(crate) use local_sync::PluginSyncRuntimeGuard;

use super::{clean_plugin_id, OperationError};
#[cfg(feature = "plugin-install")]
use crate::events::DomainEvent;
use crate::AppCore;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionOutcome {
    pub action_type: String,
    pub ok: bool,
    pub summary: String,
    pub logs: Vec<String>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallResult {
    pub id: String,
    pub version: String,
    pub directory: String,
    pub replaced: bool,
}

/// Resolved isolated-UI target: which plugin page to open and which
/// directory the desktop asset server may serve it from.
#[derive(Debug, Clone)]
pub struct PluginUiTarget {
    pub id: String,
    pub name: String,
    pub directory: std::path::PathBuf,
    pub asset_root: std::path::PathBuf,
    pub entry_file: String,
    pub page_id: String,
}

/// Page-independent UI assets for the shared (multi-plugin) asset server.
#[derive(Debug, Clone)]
pub struct PluginUiAssets {
    pub id: String,
    pub name: String,
    pub directory: std::path::PathBuf,
    pub asset_root: std::path::PathBuf,
    pub entry: String,
    pub entry_file: String,
    pub page_ids: Vec<String>,
}

impl AppCore {
    // ------------------------------------------------------------------
    // Plugins.
    // ------------------------------------------------------------------

    /// Safe plugin summaries. Never includes settings values.
    pub fn plugin_list(&self) -> Vec<Value> {
        let snapshot = self.load_merged_behavior_snapshot();
        snapshot
            .get("plugins")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    pub fn plugin_get(&self, id: &str) -> Result<Value, OperationError> {
        // Canonical existence check first so every plugin op shares one
        // not-found message.
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id;
        self.plugin_list()
            .into_iter()
            .find(|plugin| {
                plugin
                    .get("descriptor")
                    .and_then(|descriptor| descriptor.get("id"))
                    .and_then(Value::as_str)
                    == Some(id.as_str())
                    || plugin.get("id").and_then(Value::as_str) == Some(id.as_str())
            })
            .ok_or_else(|| OperationError::not_found(format!("Plugin `{id}` is not installed.")))
    }

    pub(crate) fn require_discovered(
        &self,
        id: &str,
    ) -> Result<tiktools_plugin_loader::DiscoveredPlugin, OperationError> {
        let id = clean_plugin_id(id)?;
        self.plugins
            .get(&id)
            .ok_or_else(|| OperationError::not_found(format!("Plugin `{id}` is not installed.")))
    }

    #[cfg(feature = "plugin-install")]
    pub fn plugin_install(
        &self,
        archive: &str,
        replace_existing: bool,
    ) -> Result<PluginInstallResult, OperationError> {
        let trimmed = archive.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 {
            return Err(OperationError::invalid(
                "plugin archive path must be 1..=4096 characters",
            ));
        }
        let outcome = self
            .install_plugin(std::path::Path::new(trimmed), replace_existing)
            .map_err(OperationError::install)?;
        self.after_package_change(&outcome);
        Ok(PluginInstallResult {
            id: outcome.receipt.id,
            version: outcome.receipt.version,
            directory: outcome.directory,
            replaced: outcome.previous_version.is_some(),
        })
    }

    #[cfg(not(feature = "plugin-install"))]
    pub fn plugin_install(
        &self,
        _archive: &str,
        _replace_existing: bool,
    ) -> Result<PluginInstallResult, OperationError> {
        Err(OperationError::unavailable(
            "plugin installation was disabled in this build",
        ))
    }

    #[cfg(feature = "plugin-install")]
    pub fn plugin_uninstall(&self, id: &str, purge_data: bool) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.uninstall_plugin(&id, purge_data)
            .map_err(OperationError::install)?;
        self.refresh_automation_snapshot();
        self.events
            .publish_domain(DomainEvent::PluginUninstalled { plugin_id: id });
        Ok(())
    }

    #[cfg(not(feature = "plugin-install"))]
    pub fn plugin_uninstall(&self, _id: &str, _purge_data: bool) -> Result<(), OperationError> {
        Err(OperationError::unavailable(
            "plugin installation was disabled in this build",
        ))
    }
}
