use std::env;

use tiktools_plugin_api::{CapabilitySet, PermissionSet};

/// A plugin's stable identity, independent of its runtime adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginIdentity {
    pub id: String,
    pub version: String,
}

impl PluginIdentity {
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
        }
    }
}

/// Runtime-neutral context supplied to plugin business logic.
///
/// File paths and host handles are intentionally absent. Future WASM
/// adapters can expose narrowly scoped host capabilities without changing the
/// trait or handing plugins arbitrary host internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginContext {
    pub identity: PluginIdentity,
    /// Capabilities declared by the plugin manifest. These are not grants.
    pub declared_capabilities: CapabilitySet,
    /// Permissions declared by the plugin manifest. User grants are a
    /// separate policy layer and are intentionally not represented here yet.
    pub declared_permissions: PermissionSet,
}

impl PluginContext {
    pub fn new(
        identity: PluginIdentity,
        declared_capabilities: CapabilitySet,
        declared_permissions: PermissionSet,
    ) -> Self {
        Self {
            identity,
            declared_capabilities,
            declared_permissions,
        }
    }

    /// Builds the process context from the existing launcher contract.
    /// WASM adapters can construct the same shape from their manifest and
    /// explicit host policy without relying on environment variables.
    pub fn from_process_environment() -> Self {
        Self::new(
            PluginIdentity::new(
                env::var("TIKTOOLS_PLUGIN_ID").unwrap_or_else(|_| "unknown".to_owned()),
                env::var("TIKTOOLS_PLUGIN_VERSION").unwrap_or_else(|_| "0.0.0".to_owned()),
            ),
            CapabilitySet::from_strings(environment_list("TIKTOOLS_PLUGIN_CAPABILITIES")),
            PermissionSet::from_strings(environment_list("TIKTOOLS_PLUGIN_PERMISSIONS")),
        )
    }

    /// Returns the limited context available through native ABI v1.
    ///
    /// ABI v1 does not pass manifest metadata into `create`, so native
    /// plugins must not mistake this context for a manifest-backed grant.
    /// A future ABI revision can add an explicit initialization payload.
    pub fn for_native_abi_v1() -> Self {
        Self::new(
            PluginIdentity::new("unknown", "0.0.0"),
            CapabilitySet::default(),
            PermissionSet::default(),
        )
    }
}

fn environment_list(name: &str) -> Vec<String> {
    env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
