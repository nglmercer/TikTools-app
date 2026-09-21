//! Generic declarative UI nodes and actions.
//!
//! The node set is the smallest that reproduces the current plugin screens
//! (text, form, connection, list, TTS) plus the reusable primitives new
//! plugins need (select, range, checkbox, button, status). Domain panels
//! such as TTS resolve to host-owned renderers; they are addressed by a
//! contribution id, never by embedded markup or script.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Localized `{ default, i18key }` metadata shared with manifests.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocalizedText {
    pub default: String,
    #[serde(default)]
    pub i18key: String,
}

/// Static fallback option for `select` nodes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectOption {
    pub value: String,
    pub label: LocalizedText,
}

/// Closed set of generic node types. Never extended per-domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginUiNodeType {
    Stack,
    Card,
    Text,
    Form,
    Select,
    Range,
    Checkbox,
    Button,
    List,
    Status,
    Separator,
    Connection,
    TtsSettings,
}

/// Allowlisted UI action descriptor. There is deliberately no generic
/// arbitrary-RPC action: every effect maps to one explicit host operation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PluginUiAction {
    SaveSettings,
    PluginAction {
        #[serde(rename = "actionType")]
        action_type: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        config: Option<Value>,
    },
    RefreshSource {
        source: String,
    },
    TestConnection,
    OpenMediaPicker {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        accept: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<String>,
    },
}

/// One declarative UI node. Flattened struct (rather than an enum) so
/// unknown-future fields deserialize leniently while validation stays
/// strict per `node_type`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginUiNode {
    #[serde(rename = "type")]
    pub node_type: PluginUiNodeType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<LocalizedText>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<LocalizedText>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<PluginUiNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<LocalizedText>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
    #[serde(rename = "uiHints", default, skip_serializing_if = "Option::is_none")]
    pub ui_hints: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SelectOption>>,
    #[serde(
        rename = "optionsFrom",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub options_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<PluginUiAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution: Option<String>,
}
