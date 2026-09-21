//! Typed declarative plugin UI contract (uiVersion 1).
//!
//! Canonical host-side contract for plugin UI descriptors. Plugins describe
//! UI as data; the TikTools frontend renders trusted host components. The
//! contract is intentionally closed: new domains compose these primitives
//! instead of adding domain-specific section kinds.
//!
//! Trust boundary: manifests are untrusted input. Every descriptor is
//! validated here at discovery ([`validation`]) and re-validated at render
//! time by the TypeScript mirror (`src/plugin-ui/`). Unknown node or action
//! types fail closed on both sides.
//!
//! TypeScript mirror: `src/plugin-ui/contracts.ts`. Both sides derive from
//! this module's shapes; validation parity is covered by fixture tests.

pub mod manifest;
pub mod nodes;
pub mod validation;

pub use manifest::{
    parse_ui_manifest, PluginUiManifest, PluginUiManifestPage, PluginUiMode, MAX_UI_PAGES,
    PLUGIN_UI_VERSION,
};
pub use nodes::{LocalizedText, PluginUiAction, PluginUiNode, PluginUiNodeType, SelectOption};
pub use validation::{
    validate_ui_node, validate_ui_page, UiValidationError, MAX_UI_CHILDREN, MAX_UI_DEPTH,
};
