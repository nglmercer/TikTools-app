//! Plugin UI manifest fragment (`ui` key, uiVersion 1).
//!
//! Two modes: `declarative` (default) describes pages with generic nodes
//! rendered by the trusted host frontend; `webview` (future/advanced)
//! points at static `ui/` assets rendered inside a separate isolated
//! WebView through the restricted `PluginUiBroker` — never in the main
//! privileged document. This module types both; the desktop shell enables
//! `webview` only when its broker, asset sandbox, and CSP land.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::nodes::{LocalizedText, PluginUiNode};

/// Current declarative UI contract version.
pub const PLUGIN_UI_VERSION: u32 = 1;

/// Plugin UI rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginUiMode {
    Declarative,
    Webview,
}

/// One page entry in a `webview`-mode manifest (isolated custom UI).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginUiManifestPage {
    pub id: String,
    pub title: LocalizedText,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// The `ui` fragment of a plugin manifest.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginUiManifest {
    #[serde(rename = "apiVersion", default = "default_api_version")]
    pub api_version: u32,
    #[serde(default)]
    pub mode: UiModeDefault,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<PluginUiManifestPage>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<PluginUiNode>,
}

fn default_api_version() -> u32 {
    PLUGIN_UI_VERSION
}

/// Serde-friendly default wrapper so `mode` defaults to declarative.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum UiModeDefault {
    #[default]
    Declarative,
    Webview,
}

impl From<UiModeDefault> for PluginUiMode {
    fn from(value: UiModeDefault) -> Self {
        match value {
            UiModeDefault::Declarative => Self::Declarative,
            UiModeDefault::Webview => Self::Webview,
        }
    }
}
