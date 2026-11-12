//! Plugin webview target resolution and asset confinement, independent of runtime mutation
//! and action execution.

use super::{PluginUiAssets, PluginUiTarget};
use crate::{control::OperationError, AppCore};

/// Mirrors the discovery-time `is_valid_ui_entry` rule so a tampered
/// manifest can never widen the servable directory at open time.
pub(super) fn is_confined_ui_entry(entry: &str) -> bool {
    if entry.len() > 256 || !entry.starts_with("ui/") || !entry.ends_with(".html") {
        return false;
    }
    if entry.contains('\\') {
        return false;
    }
    let path = std::path::Path::new(entry);
    !path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
}

impl AppCore {
    /// Resolves the isolated-UI target for a webview-mode plugin page.
    /// Fails when the plugin is missing, has no webview UI, the page is
    /// unknown, or the built entry asset is absent. The entry path is
    /// re-validated here (defense in depth: discovery already checked it)
    /// and the asset root is confined to the entry's own directory, so the
    /// desktop asset server can only serve built UI files — never the
    /// plugin manifest, settings, or backend.
    pub fn plugin_ui_target(
        &self,
        id: &str,
        page_id: &str,
    ) -> Result<PluginUiTarget, OperationError> {
        let assets = self.plugin_ui_assets(id)?;
        if !assets.page_ids.iter().any(|page| page == page_id) {
            return Err(OperationError::not_found(format!(
                "Plugin `{}` has no UI page `{page_id}`.",
                assets.id
            )));
        }
        let file = assets.directory.join(&assets.entry);
        if !file.is_file() {
            return Err(OperationError::unavailable(format!(
                "Plugin `{}` UI is not built (missing `{}`).",
                assets.id, assets.entry
            )));
        }
        Ok(PluginUiTarget {
            id: assets.id,
            name: assets.name,
            directory: assets.directory,
            asset_root: assets.asset_root,
            entry_file: assets.entry_file,
            page_id: page_id.to_owned(),
        })
    }

    /// Resolves a webview-mode plugin's servable UI assets without
    /// selecting a page. The main window's shared asset server calls this
    /// per request to confine inline plugin frames; unknown plugins,
    /// declarative plugins, and escaping entries fail before any file is
    /// touched. Missing build output is left to the file server, which
    /// 404s naturally per asset.
    pub fn plugin_ui_assets(&self, id: &str) -> Result<PluginUiAssets, OperationError> {
        let plugin = self.require_discovered(id)?;
        let manifest = &plugin.manifest;
        let ui = manifest.ui.as_ref().ok_or_else(|| {
            OperationError::unavailable(format!("Plugin `{}` has no custom UI.", manifest.id))
        })?;
        if ui.mode != tiktools_plugin_api::ui::PluginUiMode::Webview {
            return Err(OperationError::unavailable(format!(
                "Plugin `{}` uses declarative UI; no isolated webview.",
                manifest.id
            )));
        }
        let entry = ui.entry.as_deref().ok_or_else(|| {
            OperationError::unavailable(format!(
                "Plugin `{}` webview UI has no entry asset.",
                manifest.id
            ))
        })?;
        if !is_confined_ui_entry(entry) {
            return Err(OperationError::invalid(format!(
                "Plugin `{}` UI entry `{entry}` escapes the ui/ asset directory.",
                manifest.id
            )));
        }
        let file = plugin.directory.join(entry);
        let asset_root = file
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| plugin.directory.clone());
        let entry_file = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "index.html".to_owned());
        Ok(PluginUiAssets {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            directory: plugin.directory.clone(),
            asset_root,
            entry: entry.to_owned(),
            entry_file,
            page_ids: ui.pages.iter().map(|page| page.id.clone()).collect(),
        })
    }
}
