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
    NativeAddonArtifact, NativeAddonDeclaration, PluginManifest, PluginRuntimeKind,
    PluginSecurityModel, PluginTrust, DEFAULT_PLUGIN_ACTION_TIMEOUT_MS,
    DEFAULT_PLUGIN_PROCESSOR_TIMEOUT_MS, MAX_PLUGIN_ACTION_TIMEOUT_MS,
    MAX_PLUGIN_PROCESSOR_TIMEOUT_MS,
};
pub use validation::{
    current_arch, current_napi_target, current_platform, current_target, host_native_artifact_keys,
    is_musl, is_safe_relative_path, is_valid_event_subscription, is_valid_event_type,
    is_valid_native_package_name, is_valid_native_platform_key, is_valid_plugin_id,
    parse_sha256_hex, validate_action_type, validate_declarative_action, validate_event_type,
    validate_http_config, validate_plugin_autocomplete, validate_plugin_page,
    validate_plugin_template, ManifestError,
};
