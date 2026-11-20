//! Plugin-side access to the TikTools store. The implementation lives in
//! `tiktools-plugin-store` and is shared with the host, so a plugin and the
//! host read and write the same format.

pub use tiktools_plugin_store::*;

use crate::{data_dir, PluginError, PluginResult};

impl From<StoreError> for PluginError {
    fn from(error: StoreError) -> Self {
        PluginError::other(error.to_string())
    }
}

/// Opens this plugin's store at `<data>/<plugin id>/store`. The host passes
/// the shared plugin data root, so the plugin id keeps stores apart; this is
/// the same directory the host uses for the plugin's settings.
pub fn from_plugin_data() -> PluginResult<Store> {
    let root = data_dir()?;
    let plugin_id = std::env::var("TIKTOOLS_PLUGIN_ID").map_err(|_| {
        PluginError::other("required process environment variable TIKTOOLS_PLUGIN_ID is missing")
    })?;
    if plugin_id.is_empty() || plugin_id.contains(['/', '\\']) || plugin_id == ".." {
        return Err(PluginError::other(
            "TIKTOOLS_PLUGIN_ID is not a valid plugin id",
        ));
    }
    Store::open(root.join(plugin_id).join("store")).map_err(PluginError::from)
}
