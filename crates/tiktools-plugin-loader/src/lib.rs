//! Runtime plugin discovery and lifecycle.
//!
//! The loader owns no GUI objects and has no compile-time plugin registry.
//! Every plugin is found through a package directory and a manifest at
//! runtime. Native plugins are trusted in-process code; process plugins are
//! isolated executables with a crash boundary, not an OS sandbox.

mod artifact;
mod declarative;
mod discovery;
mod isolated;
mod manager;
mod native;
mod process;
mod quickjs;
#[cfg(test)]
mod tests;
mod types;
mod wasm;
mod worker;

pub use artifact::plugin_artifact_fingerprint;
pub use declarative::DeclarativePluginRuntime;
pub use discovery::plugin_roots;
pub use isolated::{IsolatedPluginCall, PreparedIsolatedPlugin};
pub use manager::{
    PluginEventDeliveryPermit, PluginManager, PluginPackageLease, PluginWorkerObservation,
    RuntimeRegistry,
};
pub use native::NativePluginRuntime;
pub use process::ProcessPluginRuntime;
pub use quickjs::{
    native_addons_allowed, QuickJsPluginRuntime, QuickJsPluginRuntime as NapiVmPluginRuntime,
};
pub use types::{
    DiscoveredPlugin, EmittedEvent, ManagedWorker, PluginEventSender, PluginInstance,
    PluginLoaderError, PluginRoot, PluginRuntime, PluginSource, WorkerContext,
};
pub use wasm::WasmPluginRuntime;
