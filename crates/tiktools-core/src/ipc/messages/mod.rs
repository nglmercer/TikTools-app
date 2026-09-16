//! Rust wire model for `src/shared/messages.ts`.
//!
//! The model intentionally keeps automation/live payloads as JSON values until
//! their parity ports are complete. Field names and message discriminators are
//! the compatibility boundary used by the existing Vue UI.

use std::collections::BTreeMap;

use serde_json::Value;

mod host;
mod install;
mod page;
mod points;
#[cfg(test)]
mod tests;
mod validation;

pub type JsonObject = BTreeMap<String, Value>;

pub use host::{ConnectionStatus, ErrorPhase, HostMessage, PluginProgressState};
pub use install::{
    classify_plugin_install_error, PluginInstallErrorCode, MAX_PLUGIN_PACKAGE_PATH_LEN,
};
pub use page::PageMessage;
pub use points::{PartialPointsConfig, PointsConfig};
pub use validation::IpcMessageError;
