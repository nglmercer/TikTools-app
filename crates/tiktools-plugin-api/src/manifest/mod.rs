//! Versioned, runtime-neutral plugin manifests.
//!
//! The host only accepts the native schema. There is no compatibility parser
//! for the removed TypeScript plugin format: a package must declare its
//! runtime and entry explicitly before it can be discovered.

mod parse;
mod processor;
#[cfg(test)]
mod tests;
mod types;
mod validation;

pub use processor::{
    is_valid_processor_event_type, is_valid_processor_id, validate_processor_type,
    PluginProcessorDescriptor, ProcessorFailureMode, ProcessorInputDescriptor, ProcessorStage,
};
pub use types::{
    PluginManifest, PluginRuntimeKind, PluginSecurityModel, PluginTrust,
    DEFAULT_PLUGIN_ACTION_TIMEOUT_MS, DEFAULT_PLUGIN_PROCESSOR_TIMEOUT_MS,
    MAX_PLUGIN_ACTION_TIMEOUT_MS, MAX_PLUGIN_PROCESSOR_TIMEOUT_MS,
};
pub use validation::{
    current_platform, current_target, is_safe_relative_path, is_valid_event_type,
    is_valid_plugin_id, validate_action_type, validate_declarative_action, validate_event_type,
    validate_http_config, validate_plugin_page, validate_plugin_template, ManifestError,
};
