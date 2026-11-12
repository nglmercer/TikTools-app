//! Behavior rule templates and profiles: JSON-defined event+actions bundles.
//!
//! A rule template (v1) holds one trigger event, its actions, and a
//! creation-time params form. `{{ params.* }}` spans resolve when the
//! template is instantiated; `{{ event.* }}` spans survive for the runtime.
//! A profile (v1) bundles several templates with shared params (e.g. the
//! Minecraft CommandAPI host/port every rule posts to).
//!
//! This engine mirrors `src/web/views/behavior/rule-templates.ts` exactly:
//! the same documents must validate (or fail) on both sides. Shared fixtures
//! under `tests/fixtures/templates/` pin the agreement.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::control::fresh_record_id;
mod instantiation;
mod params;
mod validation;

use instantiation::instantiate_template_with_ids;
pub use instantiation::{instantiate_template, template_requirements};
pub use params::{merge_params, param_defaults, substitute_params};
use params::{read_param_path_ref, scan_params_span, sole_params_span, substitute_string};
pub use validation::{parse_profile, parse_rule_template, parse_rule_template_list};

mod bounded;
pub use bounded::{
    instantiate_template_bounded, TemplateInstantiationBudget, TemplateInstantiationError,
};

pub const RULE_TEMPLATE_VERSION: u64 = 1;
pub const PROFILE_VERSION: u64 = 1;
/// app.state key shared with the web gallery for user-imported templates.
pub const CUSTOM_TEMPLATES_KEY: &str = "behavior.templates.custom";
const MAX_ACTIONS: usize = 8;
const MAX_FILTERS: usize = 12;
const MAX_IMPORT_DOCS: usize = 32;

const OPERATORS: &[&str] = &[
    "gte",
    "gt",
    "lte",
    "lt",
    "eq",
    "neq",
    "contains",
    "starts-with",
    "in",
    "is-true",
    "is-false",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateText {
    pub default: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i18key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTemplateAction {
    pub name: String,
    #[serde(rename = "typeId")]
    pub type_id: String,
    pub enabled: bool,
    pub config: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTemplateFilter {
    pub path: String,
    pub operator: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTemplateEvent {
    pub name: String,
    pub enabled: bool,
    pub trigger: String,
    pub filters: Vec<RuleTemplateFilter>,
    #[serde(rename = "cooldownMs")]
    pub cooldown_ms: u64,
    #[serde(rename = "cooldownScope")]
    pub cooldown_scope: String,
    #[serde(rename = "runMode")]
    pub run_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTemplate {
    #[serde(rename = "templateVersion")]
    pub template_version: u64,
    pub id: String,
    pub title: TemplateText,
    pub description: TemplateText,
    pub icon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Map<String, Value>>,
    pub actions: Vec<RuleTemplateAction>,
    pub event: RuleTemplateEvent,
}

#[derive(Debug, Clone)]
pub struct ProfileEntry {
    pub template: RuleTemplate,
    pub params: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub params: Map<String, Value>,
    pub entries: Vec<ProfileEntry>,
}

/// Validation failure: every message is display-safe for CLI and UI alike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateErrors(pub Vec<String>);

impl std::fmt::Display for TemplateErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("; "))
    }
}

impl std::error::Error for TemplateErrors {}

pub struct InstantiatedRule {
    pub actions: Vec<Value>,
    pub event: Value,
}

#[cfg(test)]
mod tests;
