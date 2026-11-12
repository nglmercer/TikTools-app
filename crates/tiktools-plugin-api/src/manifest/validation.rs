//! Manifest error contract, shared bounds and stable validation entry points.
//!
//! Domain modules validate the original descriptor shapes. The parser retains
//! schema dispatch, so accepting v2 never silently enables v3 declarations.

mod actions;
mod autocomplete;
mod contributions;
mod events;
mod fields;
mod http_config;
mod http_fields;
mod native;

pub use actions::{validate_action_type, validate_declarative_action};
pub use autocomplete::validate_plugin_autocomplete;
pub use contributions::{validate_plugin_page, validate_plugin_template};
pub use events::{
    declared_event_types, is_valid_event_subscription, is_valid_event_type, validate_event_type,
};
pub use fields::{is_safe_relative_path, is_valid_plugin_id};
pub use http_config::validate_http_config;
pub use native::{
    current_arch, current_napi_target, current_platform, current_target, is_known_napi_target,
    is_musl, is_valid_native_lib_binary, is_valid_native_lib_repo, is_valid_native_lib_tag,
    is_valid_native_lib_version, is_valid_native_package_name, select_host_native_binary,
    NativeBinarySelectError, NAPI_TARGETS,
};

use thiserror::Error;

use super::types::{MAX_DECLARATIVE_HTTP_TIMEOUT_MS, MAX_PLUGIN_ACTION_TIMEOUT_MS};

pub(crate) const PLUGIN_SCHEMA_VERSION: u32 = 3;
/// Oldest schema the host still accepts. Schema v2 manifests parse exactly
/// as before: declarative keys are ignored unless the manifest declares v3.
pub(crate) const MIN_PLUGIN_SCHEMA_VERSION: u32 = 2;
pub(crate) fn is_supported_schema(version: u32) -> bool {
    (MIN_PLUGIN_SCHEMA_VERSION..=PLUGIN_SCHEMA_VERSION).contains(&version)
}
pub(crate) const MAX_MANIFEST_BYTES: usize = 256 * 1024;
pub(crate) const MAX_LIST_ENTRIES: usize = 128;
pub(crate) const MAX_DESCRIPTOR_BYTES: usize = 64 * 1024;
/// Long-form card copy rendered as markdown-lite in Details. Four times the
/// short description: room for a paragraph plus a short list, still bounded.
pub(crate) const MAX_LONG_DESCRIPTION_LEN: usize = 16 * 1024;
/// Declarative fetch timeouts (catalogs, health, connection probes) stay
/// inside the automation executor's generic clamp: none of these may hold a UI
/// load for minutes.
pub(crate) const MIN_HTTP_TIMEOUT_MS: u64 = 100;
pub(crate) const MAX_HTTP_FETCH_TIMEOUT_MS: u64 = 120_000;
/// Plugin-declared action requests may request a larger, bounded budget for
/// slow synthesis fallbacks (see [`MAX_DECLARATIVE_HTTP_TIMEOUT_MS`]).
pub(crate) const MAX_HTTP_ACTION_TIMEOUT_MS: u64 = MAX_DECLARATIVE_HTTP_TIMEOUT_MS;
pub(crate) const MAX_HTTP_HEADERS: usize = 32;
pub(crate) const MAX_BASE_URL_LEN: usize = 512;
pub(crate) const MAX_HTTP_PATH_LEN: usize = 1_024;
pub(crate) const MAX_OPTION_SOURCES: usize = 32;
pub(crate) const MAX_TEMPLATE_NODES: usize = 16;
pub(crate) const MAX_PAGE_SECTIONS: usize = 16;
#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("plugin manifest is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("plugin manifest must be a JSON object")]
    NotAnObject,
    #[error("plugin manifest has unsupported schema version {0}")]
    UnsupportedSchema(u32),
    #[error("plugin manifest field `{0}` is missing or invalid")]
    MissingField(&'static str),
    #[error("plugin manifest field `{0}` is invalid")]
    InvalidField(&'static str),
    #[error("plugin manifest entry must stay inside its package")]
    UnsafeEntry,
    #[error("plugin manifest is larger than {MAX_MANIFEST_BYTES} bytes")]
    TooLarge,
}
/// Prefixes owned by the host. Plugins declare their own event types under
/// any other dotted name (for example hotkey.pressed or timer.tick).
const RESERVED_EVENT_PREFIXES: [&str; 3] = ["tiktok.", "points.", "plugin."];

pub(crate) const MAX_EVENT_TYPE_LEN: usize = 64;
const MAX_EVENT_FIELDS: usize = 64;
const MAX_EVENT_OPTIONS: usize = 128;
const MAX_AUTOCOMPLETE_PREFIXES: usize = 16;
const MAX_AUTOCOMPLETE_PATHS: usize = 32;
const MAX_AUTOCOMPLETE_FIELDS: usize = 32;
const MAX_AUTOCOMPLETE_TRIGGERS: usize = 32;
const MAX_AUTOCOMPLETE_PATH_LEN: usize = 256;

/// Maximum declared native packages per manifest. Real plugins bundle one
/// or two; the cap only bounds adversarial manifests.
pub(crate) const MAX_NATIVE_ADDON_PACKAGES: usize = 32;
pub(crate) const MAX_NATIVE_PACKAGE_LEN: usize = 256;
pub(crate) const MAX_NATIVE_PATH_LEN: usize = 512;
pub(crate) const MAX_NATIVE_LIB_PACKAGES: usize = 32;
pub(crate) const MAX_NATIVE_LIB_VERSION_LEN: usize = 64;
pub(crate) const MAX_NATIVE_LIB_REPO_LEN: usize = 256;
pub(crate) const MAX_NATIVE_LIB_TAG_LEN: usize = 128;
pub(crate) const MAX_NATIVE_LIB_BINARY_LEN: usize = 128;

#[cfg(test)]
mod plugin_id_corpus_tests {
    use super::is_valid_plugin_id;
    use crate::manifest::PluginManifest;
    use serde_json::{json, Value};

    /// Shared with `scripts/lib/plugin-id.test.ts`, which also checks the
    /// same cases against `plugin.schema.json`.
    const CORPUS: &str = include_str!("../../../../packages/plugin-sdk/plugin-id-corpus.json");

    fn cases(kind: &str) -> Vec<String> {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus is valid JSON");
        corpus[kind]
            .as_array()
            .expect("corpus lists cases")
            .iter()
            .map(|case| case["id"].as_str().expect("case has an id").to_owned())
            .collect()
    }

    fn manifest_accepts(id: &str) -> bool {
        let document = json!({
            "schemaVersion": 3,
            "id": id,
            "name": "Corpus",
            "version": "1.0.0",
            "runtime": "declarative"
        });
        PluginManifest::from_json_str(&document.to_string()).is_ok()
    }

    #[test]
    fn host_accepts_and_rejects_the_shared_corpus() {
        let valid = cases("valid");
        let invalid = cases("invalid");
        assert!(!valid.is_empty() && !invalid.is_empty());
        for id in &valid {
            assert!(is_valid_plugin_id(id), "{id:?} should be a valid plugin id");
            assert!(manifest_accepts(id), "manifest should accept {id:?}");
        }
        for id in &invalid {
            assert!(!is_valid_plugin_id(id), "{id:?} should be rejected");
            assert!(!manifest_accepts(id), "manifest should reject {id:?}");
        }
    }
}
