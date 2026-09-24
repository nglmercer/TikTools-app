//! Runtime plugin discovery and lifecycle.
//!
//! The loader owns no GUI objects and has no compile-time plugin registry.
//! Every plugin is found through a package directory and a manifest at
//! runtime. Native plugins are trusted in-process code; process plugins are
//! isolated executables with a crash boundary, not an OS sandbox.

mod declarative;
mod discovery;
#[cfg(feature = "plugin-install")]
mod installer;
mod manager;
mod napi_vm;
mod native;
mod process;
#[cfg(test)]
mod tests;
mod types;
mod wasm;
mod worker;

pub use declarative::DeclarativePluginRuntime;
pub use discovery::plugin_roots;
#[cfg(feature = "plugin-install")]
pub use installer::{InstalledPluginPackage, PluginInstaller};
pub use manager::{PluginManager, RuntimeRegistry};
pub use napi_vm::NapiVmPluginRuntime;
pub use native::NativePluginRuntime;
pub use process::ProcessPluginRuntime;
pub use types::{
    DiscoveredPlugin, PluginInstance, PluginLoaderError, PluginRoot, PluginRuntime, PluginSource,
};
pub use wasm::WasmPluginRuntime;
