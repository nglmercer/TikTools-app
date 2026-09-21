//! Plugin UI manifest fragment (`ui` key, uiVersion 1).
//!
//! One canonical shape for both modes:
//!
//! ```json
//! {
//!   "apiVersion": 1,
//!   "mode": "declarative",
//!   "pages": [
//!     {
//!       "id": "main",
//!       "title": { "default": "Settings" },
//!       "body": { "type": "stack", "children": [] }
//!     }
//!   ]
//! }
//! ```
//!
//! Declarative pages carry their own `body` tree rendered by the trusted
//! host frontend. Webview pages carry no body; the manifest-level `entry`
//! points at the plugin's compiled static assets, rendered inside a
//! separate isolated WebView through the restricted `PluginUiBroker`.
//!
//! The fragment is parsed and validated during plugin discovery
//! ([`parse_ui_manifest`]), not only in the frontend. Schema-v3 `pages`
//! remain the compatibility input for legacy manifests and are converted
//! by the host frontend adapter; `ui` is the forward path.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::nodes::{LocalizedText, PluginUiNode};
use super::validation::{validate_localized_text, validate_ui_node, MAX_UI_TEXT};
use crate::manifest::{is_safe_relative_path, is_valid_plugin_id, ManifestError};

/// Current declarative UI contract version.
pub const PLUGIN_UI_VERSION: u32 = 1;
/// Maximum pages per `ui` manifest.
pub const MAX_UI_PAGES: usize = 16;

/// Plugin UI rendering mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginUiMode {
    /// Host-rendered declarative pages (default).
    #[default]
    Declarative,
    /// Isolated custom WebView loading the manifest `entry`.
    Webview,
}

/// One page entry in a `ui` manifest.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginUiManifestPage {
    pub id: String,
    pub title: LocalizedText,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Declarative page body. Required in `declarative` mode, forbidden
    /// in `webview` mode (custom UI comes from `entry` assets).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<PluginUiNode>,
}

/// The `ui` fragment of a plugin manifest.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginUiManifest {
    #[serde(rename = "apiVersion", default = "default_api_version")]
    pub api_version: u32,
    #[serde(default)]
    pub mode: PluginUiMode,
    /// Webview entry asset, relative to the plugin root and confined to
    /// `ui/` (e.g. `ui/dist/index.html`). Declarative manifests omit it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(default)]
    pub pages: Vec<PluginUiManifestPage>,
}

fn default_api_version() -> u32 {
    PLUGIN_UI_VERSION
}

/// Parses and validates the `ui` manifest fragment at discovery time.
/// Unknown node types, unknown action types, bad bindings, over-deep
/// trees, and mode/body/entry mismatches all fail the manifest — the
/// frontend never sees an unvalidated descriptor.
pub fn parse_ui_manifest(value: &Value) -> Result<PluginUiManifest, ManifestError> {
    let manifest: PluginUiManifest =
        serde_json::from_value(value.clone()).map_err(|_| ManifestError::InvalidField("ui"))?;
    if manifest.api_version != PLUGIN_UI_VERSION {
        return Err(ManifestError::InvalidField("ui"));
    }
    if manifest.pages.is_empty() || manifest.pages.len() > MAX_UI_PAGES {
        return Err(ManifestError::InvalidField("ui"));
    }
    match manifest.mode {
        PluginUiMode::Declarative => {
            if manifest.entry.is_some() {
                return Err(ManifestError::InvalidField("ui"));
            }
            for page in &manifest.pages {
                validate_ui_page(page)?;
                let Some(body) = &page.body else {
                    return Err(ManifestError::InvalidField("ui"));
                };
                validate_ui_node(body, 0).map_err(|_| ManifestError::InvalidField("ui"))?;
            }
        }
        PluginUiMode::Webview => {
            let Some(entry) = manifest.entry.as_deref() else {
                return Err(ManifestError::InvalidField("ui"));
            };
            if !is_valid_ui_entry(entry) {
                return Err(ManifestError::InvalidField("ui"));
            }
            for page in &manifest.pages {
                validate_ui_page(page)?;
                if page.body.is_some() {
                    return Err(ManifestError::InvalidField("ui"));
                }
            }
        }
    }
    Ok(manifest)
}

fn validate_ui_page(page: &PluginUiManifestPage) -> Result<(), ManifestError> {
    if !is_valid_plugin_id(&page.id) {
        return Err(ManifestError::InvalidField("ui"));
    }
    validate_localized_text(&page.title).map_err(|_| ManifestError::InvalidField("ui"))?;
    if let Some(icon) = &page.icon {
        if icon.trim().is_empty() || icon.len() > 64 {
            return Err(ManifestError::InvalidField("ui"));
        }
    }
    Ok(())
}

/// Webview entries are confined to the plugin's `ui/` asset directory:
/// relative, no `..`, no absolute paths, HTML only.
fn is_valid_ui_entry(entry: &str) -> bool {
    if entry.len() > 256 || !entry.starts_with("ui/") || !entry.ends_with(".html") {
        return false;
    }
    if entry.contains('\\') {
        return false;
    }
    is_safe_relative_path(entry) && entry.len() <= MAX_UI_TEXT
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declarative_fixture() -> Value {
        serde_json::json!({
            "apiVersion": 1,
            "mode": "declarative",
            "pages": [
                {
                    "id": "main",
                    "title": { "default": "Settings", "i18key": "" },
                    "body": {
                        "type": "stack",
                        "children": [
                            { "type": "text", "text": { "default": "Hello", "i18key": "" } }
                        ]
                    }
                }
            ]
        })
    }

    fn webview_fixture() -> Value {
        serde_json::json!({
            "apiVersion": 1,
            "mode": "webview",
            "entry": "ui/dist/index.html",
            "pages": [
                { "id": "tts", "title": { "default": "Text to Speech", "i18key": "" }, "icon": "voice" }
            ]
        })
    }

    #[test]
    fn accepts_canonical_manifests() {
        let declarative = parse_ui_manifest(&declarative_fixture()).expect("declarative");
        assert_eq!(declarative.mode, PluginUiMode::Declarative);
        assert_eq!(declarative.pages.len(), 1);
        assert!(declarative.pages[0].body.is_some());
        let webview = parse_ui_manifest(&webview_fixture()).expect("webview");
        assert_eq!(webview.mode, PluginUiMode::Webview);
        assert_eq!(webview.entry.as_deref(), Some("ui/dist/index.html"));
        // Mode defaults to declarative; apiVersion defaults to 1.
        let defaulted = serde_json::json!({
            "pages": [
                {
                    "id": "main",
                    "title": { "default": "Settings", "i18key": "" },
                    "body": { "type": "separator" }
                }
            ]
        });
        let parsed = parse_ui_manifest(&defaulted).expect("defaulted");
        assert_eq!(parsed.mode, PluginUiMode::Declarative);
        assert_eq!(parsed.api_version, 1);
    }

    #[test]
    fn rejects_mode_mismatches_and_bad_versions() {
        // Declarative page without a body.
        let mut bad = declarative_fixture();
        bad["pages"][0].as_object_mut().expect("page").remove("body");
        assert!(parse_ui_manifest(&bad).is_err());
        // Declarative manifest with an entry.
        let mut bad = declarative_fixture();
        bad.as_object_mut()
            .expect("manifest")
            .insert("entry".to_string(), Value::String("ui/dist/index.html".to_string()));
        assert!(parse_ui_manifest(&bad).is_err());
        // Webview page carrying a body.
        let mut bad = webview_fixture();
        bad["pages"][0]
            .as_object_mut()
            .expect("page")
            .insert("body".to_string(), serde_json::json!({"type": "separator"}));
        assert!(parse_ui_manifest(&bad).is_err());
        // Webview manifest without an entry.
        let mut bad = webview_fixture();
        bad.as_object_mut().expect("manifest").remove("entry");
        assert!(parse_ui_manifest(&bad).is_err());
        // Wrong contract version.
        let mut bad = declarative_fixture();
        bad["apiVersion"] = serde_json::json!(2);
        assert!(parse_ui_manifest(&bad).is_err());
        // Unknown node type inside a body fails discovery, not render.
        let mut bad = declarative_fixture();
        bad["pages"][0]["body"]["children"][0] = serde_json::json!({"type": "obs-scene"});
        assert!(parse_ui_manifest(&bad).is_err());
        // No pages at all.
        let bad = serde_json::json!({"apiVersion": 1, "mode": "declarative", "pages": []});
        assert!(parse_ui_manifest(&bad).is_err());
    }

    #[test]
    fn rejects_unsafe_webview_entries() {
        for entry in [
            "../escape.html",
            "/absolute.html",
            "ui/../plugin.json",
            "assets/index.html",
            "ui/dist/bundle.js",
            "ui\\dist\\index.html",
            "",
        ] {
            let mut fixture = webview_fixture();
            fixture["entry"] = Value::String(entry.to_string());
            assert!(parse_ui_manifest(&fixture).is_err(), "entry: {entry}");
        }
    }
}
