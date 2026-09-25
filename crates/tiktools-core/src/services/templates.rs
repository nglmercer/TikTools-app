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
//! under `examples/templates/fixtures/` pin the agreement.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::control::fresh_record_id;

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

fn read_text(value: Option<&Value>, max: usize) -> Option<String> {
    let text = value?.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    Some(text.chars().take(max).collect())
}

fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    id.len() >= 2
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

fn read_localized(value: Option<&Value>, max: usize) -> Option<TemplateText> {
    match value {
        Some(Value::String(_)) => read_text(value, max).map(|default| TemplateText {
            default,
            i18key: None,
        }),
        Some(Value::Object(map)) => {
            let default = read_text(map.get("default"), max)?;
            let i18key = read_text(map.get("i18key"), 128);
            Some(TemplateText { default, i18key })
        }
        _ => None,
    }
}

fn parse_filter(
    value: &Value,
    index: usize,
    errors: &mut Vec<String>,
) -> Option<RuleTemplateFilter> {
    let Some(map) = value.as_object() else {
        errors.push(format!("event.filters[{index}] must be an object"));
        return None;
    };
    let path = read_text(map.get("path"), 200);
    let operator = map.get("operator").and_then(Value::as_str);
    if path.is_none() {
        errors.push(format!("event.filters[{index}].path is required"));
    }
    let Some(operator) = operator.filter(|op| OPERATORS.contains(op)) else {
        errors.push(format!(
            "event.filters[{index}].operator must be one of {}",
            OPERATORS.join(", ")
        ));
        return None;
    };
    let values = match map.get("values") {
        None => None,
        Some(Value::Array(items)) => {
            let mut kept = Vec::with_capacity(items.len());
            for item in items {
                match item.as_str() {
                    Some(text) => kept.push(text.chars().take(500).collect()),
                    None => {
                        errors.push(format!(
                            "event.filters[{index}].values must be an array of strings"
                        ));
                        return None;
                    }
                }
            }
            Some(kept)
        }
        Some(_) => {
            errors.push(format!(
                "event.filters[{index}].values must be an array of strings"
            ));
            return None;
        }
    };
    path.map(|path| RuleTemplateFilter {
        path,
        operator: operator.to_owned(),
        value: map
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .chars()
            .take(500)
            .collect(),
        values,
    })
}

/// Validates unknown JSON into a normalized template. Never panics.
pub fn parse_rule_template(value: &Value) -> Result<RuleTemplate, TemplateErrors> {
    let mut errors: Vec<String> = Vec::new();
    let Some(doc) = value.as_object() else {
        return Err(TemplateErrors(vec![
            "template must be a JSON object".to_owned()
        ]));
    };
    if doc.get("templateVersion").and_then(Value::as_u64) != Some(RULE_TEMPLATE_VERSION) {
        return Err(TemplateErrors(vec![format!(
            "unsupported templateVersion (want {RULE_TEMPLATE_VERSION})"
        )]));
    }
    let id = read_text(doc.get("id"), 64).filter(|id| valid_id(id));
    if id.is_none() {
        errors.push("id must match [a-z0-9][a-z0-9._-]{1,63}".to_owned());
    }
    let title = read_localized(doc.get("title"), 120);
    if title.is_none() {
        errors.push("title is required".to_owned());
    }
    let description = read_localized(doc.get("description"), 500).unwrap_or(TemplateText {
        default: String::new(),
        i18key: None,
    });
    let icon = read_text(doc.get("icon"), 32).unwrap_or_else(|| "plugin".to_owned());

    let params = match doc.get("params") {
        None => None,
        Some(Value::Object(map)) => Some(map.clone()),
        Some(_) => {
            errors.push("params must be an object".to_owned());
            None
        }
    };

    let mut actions = Vec::new();
    match doc.get("actions") {
        Some(Value::Array(items)) if !items.is_empty() && items.len() <= MAX_ACTIONS => {
            for (index, entry) in items.iter().enumerate() {
                let Some(item) = entry.as_object() else {
                    errors.push(format!("actions[{index}] must be an object"));
                    continue;
                };
                let name = read_text(item.get("name"), 80);
                let type_id = read_text(item.get("typeId"), 128);
                let config = item.get("config").and_then(Value::as_object).cloned();
                if name.is_none() {
                    errors.push(format!("actions[{index}].name is required"));
                }
                if type_id.is_none() {
                    errors.push(format!("actions[{index}].typeId is required"));
                }
                if config.is_none() {
                    errors.push(format!("actions[{index}].config must be an object"));
                }
                if let (Some(name), Some(type_id), Some(config)) = (name, type_id, config) {
                    actions.push(RuleTemplateAction {
                        name,
                        type_id,
                        enabled: item
                            .get("enabled")
                            .map_or(true, |v| v == &Value::Bool(true)),
                        config,
                    });
                }
            }
        }
        _ => errors.push(format!("actions must hold 1-{MAX_ACTIONS} entries")),
    }

    let mut event: Option<RuleTemplateEvent> = None;
    match doc.get("event") {
        Some(Value::Object(raw)) => {
            let name = read_text(raw.get("name"), 80);
            let trigger = read_text(raw.get("trigger"), 128);
            if name.is_none() {
                errors.push("event.name is required".to_owned());
            }
            if trigger.is_none() {
                errors.push("event.trigger is required".to_owned());
            }
            let mut filters = Vec::new();
            match raw.get("filters") {
                None => {}
                Some(Value::Array(items)) if items.len() <= MAX_FILTERS => {
                    for (index, entry) in items.iter().enumerate() {
                        if let Some(filter) = parse_filter(entry, index, &mut errors) {
                            filters.push(filter);
                        }
                    }
                }
                _ => errors.push(format!(
                    "event.filters allows at most {MAX_FILTERS} entries"
                )),
            }
            let cooldown_ms = raw.get("cooldownMs").and_then(Value::as_u64).unwrap_or(0);
            let cooldown_scope = if raw.get("cooldownScope").and_then(Value::as_str) == Some("user")
            {
                "user".to_owned()
            } else {
                "global".to_owned()
            };
            let run_mode = if raw.get("runMode").and_then(Value::as_str) == Some("random") {
                "random".to_owned()
            } else {
                "all".to_owned()
            };
            if let (Some(name), Some(trigger)) = (name, trigger) {
                event = Some(RuleTemplateEvent {
                    name,
                    enabled: raw.get("enabled").map_or(true, |v| v == &Value::Bool(true)),
                    trigger,
                    filters,
                    cooldown_ms,
                    cooldown_scope,
                    run_mode,
                });
            }
        }
        _ => errors.push("event must be an object".to_owned()),
    }

    match (id, title, event) {
        (Some(id), Some(title), Some(event)) if errors.is_empty() && !actions.is_empty() => {
            Ok(RuleTemplate {
                template_version: RULE_TEMPLATE_VERSION,
                id,
                title,
                description,
                icon,
                params,
                actions,
                event,
            })
        }
        _ => Err(TemplateErrors(errors)),
    }
}

/// Accepts one template document, an array, or `{templates: [...]}`.
pub fn parse_rule_template_list(value: &Value) -> (Vec<RuleTemplate>, Vec<String>) {
    let documents: Vec<&Value> = if let Some(items) = value.as_array() {
        items.iter().collect()
    } else if let Some(items) = value
        .as_object()
        .and_then(|doc| doc.get("templates"))
        .and_then(Value::as_array)
    {
        items.iter().collect()
    } else {
        vec![value]
    };
    if documents.is_empty() {
        return (Vec::new(), vec!["no templates found".to_owned()]);
    }
    if documents.len() > MAX_IMPORT_DOCS {
        return (
            Vec::new(),
            vec![format!("at most {MAX_IMPORT_DOCS} templates per import")],
        );
    }
    let mut templates = Vec::new();
    let mut errors = Vec::new();
    for (index, entry) in documents.iter().enumerate() {
        match parse_rule_template(entry) {
            Ok(template) => templates.push(template),
            Err(failures) => {
                let label = entry
                    .as_object()
                    .and_then(|doc| doc.get("id"))
                    .and_then(Value::as_str)
                    .map(|id| format!(" ({id})"))
                    .unwrap_or_else(|| format!("#{}", index + 1));
                for error in failures.0 {
                    errors.push(format!("template{label}: {error}"));
                }
            }
        }
    }
    (templates, errors)
}

/// Validates a profile document: shared params plus inline template entries.
pub fn parse_profile(value: &Value) -> Result<Profile, TemplateErrors> {
    let Some(doc) = value.as_object() else {
        return Err(TemplateErrors(vec![
            "profile must be a JSON object".to_owned()
        ]));
    };
    if doc.get("profileVersion").and_then(Value::as_u64) != Some(PROFILE_VERSION) {
        return Err(TemplateErrors(vec![format!(
            "unsupported profileVersion (want {PROFILE_VERSION})"
        )]));
    }
    let mut errors: Vec<String> = Vec::new();
    let id = read_text(doc.get("id"), 64).filter(|id| valid_id(id));
    if id.is_none() {
        errors.push("id must match [a-z0-9][a-z0-9._-]{1,63}".to_owned());
    }
    let name = read_text(doc.get("name"), 80);
    if name.is_none() {
        errors.push("name is required".to_owned());
    }
    let description = read_text(doc.get("description"), 500).unwrap_or_default();
    let params = match doc.get("params") {
        None => Map::new(),
        Some(Value::Object(map)) => map.clone(),
        Some(_) => {
            errors.push("params must be an object".to_owned());
            Map::new()
        }
    };
    let mut entries = Vec::new();
    match doc.get("templates") {
        Some(Value::Array(items)) if !items.is_empty() && items.len() <= MAX_IMPORT_DOCS => {
            for (index, item) in items.iter().enumerate() {
                let Some(entry) = item.as_object() else {
                    errors.push(format!("templates[{index}] must be an object"));
                    continue;
                };
                let template_value = entry.get("template").unwrap_or(item);
                match parse_rule_template(template_value) {
                    Ok(template) => {
                        let entry_params = match entry.get("params") {
                            None => Map::new(),
                            Some(Value::Object(map)) => map.clone(),
                            Some(_) => {
                                errors.push(format!("templates[{index}].params must be an object"));
                                Map::new()
                            }
                        };
                        entries.push(ProfileEntry {
                            template,
                            params: entry_params,
                        });
                    }
                    Err(failures) => {
                        for error in failures.0 {
                            errors.push(format!("templates[{index}]: {error}"));
                        }
                    }
                }
            }
        }
        _ => errors.push(format!("templates must hold 1-{MAX_IMPORT_DOCS} entries")),
    }
    match (id, name) {
        (Some(id), Some(name)) if errors.is_empty() => Ok(Profile {
            id,
            name,
            description,
            params,
            entries,
        }),
        _ => Err(TemplateErrors(errors)),
    }
}

fn read_param_path(params: &Map<String, Value>, path: &str) -> Option<Value> {
    let mut current: Option<&Value> = None;
    for (depth, part) in path.split('.').enumerate() {
        if part.is_empty() {
            return None;
        }
        if depth == 0 {
            current = params.get(part);
        } else {
            current = current?.as_object()?.get(part);
        }
    }
    current.cloned()
}

fn scalar_text(value: Option<Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text,
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(other) => serde_json::to_string(&other).unwrap_or_default(),
    }
}

fn valid_param_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 128
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// Scans one `{{ params.* }}` span starting at `start` (the `{{` offset).
/// Returns the span end (past `}}`) plus the trimmed param path.
fn scan_params_span(text: &str, start: usize) -> Option<(usize, String)> {
    let rest = text.get(start + 2..)?;
    let close = rest.find("}}")?;
    let inner = rest.get(..close)?.trim();
    let path = inner.strip_prefix("params.")?.trim();
    if !valid_param_path(path) {
        return None;
    }
    Some((start + 2 + close + 2, path.to_owned()))
}

/**
 * Deep-substitutes `{{ params.* }}` spans. A string holding exactly one span
 * takes the raw value (numbers and booleans survive for typed configs);
 * embedded spans interpolate as text. `{{ event.* }}` and unknown spans pass
 * through for the runtime. Mirrors the web `substituteRuleParams`.
 */
pub fn substitute_params(value: &Value, params: &Map<String, Value>) -> Value {
    match value {
        Value::String(text) => substitute_string(text, params),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| substitute_params(item, params))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), substitute_params(item, params)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn substitute_string(text: &str, params: &Map<String, Value>) -> Value {
    // Sole-span fast path: the whole string is exactly one placeholder.
    if let Some(sole) = sole_params_span(text) {
        return read_param_path(params, &sole).unwrap_or(Value::String(String::new()));
    }
    let mut rendered = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(open) = text[cursor..].find("{{") {
        let start = cursor + open;
        match scan_params_span(text, start) {
            Some((end, path)) => {
                rendered.push_str(&text[cursor..start]);
                rendered.push_str(&scalar_text(read_param_path(params, &path)));
                cursor = end;
            }
            None => {
                rendered.push_str(&text[cursor..start + 2]);
                cursor = start + 2;
            }
        }
    }
    rendered.push_str(&text[cursor..]);
    Value::String(rendered)
}

fn sole_params_span(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if !trimmed.starts_with("{{") || !trimmed.ends_with("}}") {
        return None;
    }
    let (end, path) = scan_params_span(trimmed, 0)?;
    if end == trimmed.len() {
        Some(path)
    } else {
        None
    }
}

/// Schema defaults for a template `params` block (`{properties: {key: {default}}}`).
pub fn param_defaults(params: Option<&Map<String, Value>>) -> Map<String, Value> {
    let mut defaults = Map::new();
    let Some(properties) = params
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)
    else {
        return defaults;
    };
    for (key, field) in properties {
        if let Some(default) = field.as_object().and_then(|schema| schema.get("default")) {
            defaults.insert(key.clone(), default.clone());
        }
    }
    defaults
}

/// Merges param layers; later layers win. The reserved `name` key is dropped.
pub fn merge_params(layers: &[&Map<String, Value>]) -> Map<String, Value> {
    let mut merged = Map::new();
    for layer in layers {
        for (key, value) in *layer {
            merged.insert(key.clone(), value.clone());
        }
    }
    merged.remove("name");
    merged
}

pub struct InstantiatedRule {
    pub actions: Vec<Value>,
    pub event: Value,
}

/// Instantiates behavior records with fresh ids and resolved params.
pub fn instantiate_template(
    template: &RuleTemplate,
    params: &Map<String, Value>,
    event_name: Option<&str>,
    action_names: &[String],
) -> InstantiatedRule {
    let actions: Vec<Value> = template
        .actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            let name = action_names
                .get(index)
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    substitute_params(&Value::String(action.name.clone()), params)
                        .as_str()
                        .unwrap_or_default()
                        .to_owned()
                });
            serde_json::json!({
                "schemaVersion": 2,
                "id": fresh_record_id("act"),
                "name": name,
                "typeId": action.type_id,
                "enabled": action.enabled,
                "config": substitute_params(&Value::Object(action.config.clone()), params),
            })
        })
        .collect();
    let action_ids: Vec<Value> = actions
        .iter()
        .filter_map(|action| action.get("id").cloned())
        .collect();
    let name = event_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            substitute_params(&Value::String(template.event.name.clone()), params)
                .as_str()
                .unwrap_or_default()
                .to_owned()
        });
    let filters = substitute_params(
        &serde_json::to_value(&template.event.filters).unwrap_or(Value::Array(Vec::new())),
        params,
    );
    let event = serde_json::json!({
        "schemaVersion": 1,
        "id": fresh_record_id("evt"),
        "name": name,
        "enabled": template.event.enabled,
        "trigger": template.event.trigger,
        "filters": filters,
        "cooldownMs": template.event.cooldown_ms,
        "cooldownScope": template.event.cooldown_scope,
        "actionIds": action_ids,
        "runMode": template.event.run_mode,
    });
    InstantiatedRule { actions, event }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn minimal_doc() -> Value {
        json!({
            "templateVersion": 1,
            "id": "gift-log",
            "title": "Gift log",
            "actions": [{"name": "Log", "typeId": "core.log", "config": {"message": "hi"}}],
            "event": {"name": "Gift", "trigger": "tiktok.gift"},
        })
    }

    #[test]
    fn minimal_template_normalizes_defaults() {
        let template = parse_rule_template(&minimal_doc()).expect("valid");
        assert_eq!(template.id, "gift-log");
        assert_eq!(template.icon, "plugin");
        assert!(template.actions[0].enabled);
        assert_eq!(template.event.cooldown_scope, "global");
        assert_eq!(template.event.run_mode, "all");
        assert!(template.event.filters.is_empty());
    }

    #[test]
    fn malformed_templates_list_every_problem() {
        assert!(parse_rule_template(&Value::Null).is_err());
        let mut version = minimal_doc();
        version["templateVersion"] = json!(99);
        assert!(parse_rule_template(&version).is_err());
        let mut id = minimal_doc();
        id["id"] = json!("../evil");
        assert!(parse_rule_template(&id).is_err());
        let mut actions = minimal_doc();
        actions["actions"] = json!([]);
        assert!(parse_rule_template(&actions).is_err());
        let mut operator = minimal_doc();
        operator["event"]["filters"] = json!([{"path": "p", "operator": "nope", "value": ""}]);
        let errors = parse_rule_template(&operator).expect_err("bad operator").0;
        assert!(errors.iter().any(|error| error.contains("operator")));
    }

    #[test]
    fn list_import_accepts_single_array_and_wrapped_docs() {
        let (single, _) = parse_rule_template_list(&minimal_doc());
        assert_eq!(single.len(), 1);
        let (pair, _) = parse_rule_template_list(&json!([minimal_doc(), minimal_doc()]));
        assert_eq!(pair.len(), 2);
        let (wrapped, _) = parse_rule_template_list(&json!({"templates": [minimal_doc()]}));
        assert_eq!(wrapped.len(), 1);
        let (mixed, errors) =
            parse_rule_template_list(&json!([minimal_doc(), {"templateVersion": 1}]));
        assert_eq!(mixed.len(), 1);
        assert!(!errors.is_empty());
    }

    #[test]
    fn params_substitute_raw_for_sole_spans_and_text_otherwise() {
        let mut params = Map::new();
        params.insert("delta".to_owned(), json!(10));
        assert_eq!(
            substitute_params(&json!("{{ params.delta }}"), &params),
            json!(10)
        );
        assert_eq!(
            substitute_params(&json!("n={{ params.delta }}!"), &params),
            json!("n=10!")
        );
        assert_eq!(
            substitute_params(&json!("{{ event.user.uniqueId }}"), &params),
            json!("{{ event.user.uniqueId }}")
        );
        assert_eq!(
            substitute_params(&json!("{{ params.missing }}"), &params),
            json!("")
        );
    }

    #[test]
    fn param_defaults_and_merge_follow_layer_order() {
        let schema = json!({"type": "object", "properties": {"url": {"default": "https://"}}});
        let defaults = param_defaults(schema.as_object());
        assert_eq!(defaults.get("url"), Some(&json!("https://")));
        let mut profile = Map::new();
        profile.insert("port".to_owned(), json!(1));
        let mut entry = Map::new();
        entry.insert("port".to_owned(), json!(2));
        entry.insert("name".to_owned(), json!("dropped"));
        let merged = merge_params(&[&defaults, &profile, &entry]);
        assert_eq!(merged.get("port"), Some(&json!(2)));
        assert!(!merged.contains_key("name"));
    }

    #[test]
    fn instantiate_assigns_fresh_ids_and_keeps_runtime_spans() {
        let template = parse_rule_template(&json!({
            "templateVersion": 1,
            "id": "points",
            "title": "Points",
            "actions": [{"name": "P {{ params.delta }}", "typeId": "core.points",
                "config": {"delta": "{{ params.delta }}", "note": "{{ event.type }}"}}],
            "event": {"name": "Chat", "trigger": "tiktok.chat"},
        }))
        .expect("valid");
        let mut params = Map::new();
        params.insert("delta".to_owned(), json!(5));
        let first = instantiate_template(&template, &params, None, &[]);
        let second = instantiate_template(&template, &params, Some("Custom"), &[]);
        assert_eq!(first.actions[0]["config"]["delta"], json!(5));
        assert_eq!(
            first.actions[0]["config"]["note"],
            json!("{{ event.type }}")
        );
        assert_eq!(first.actions[0]["name"], json!("P 5"));
        assert_eq!(first.event["actionIds"][0], first.actions[0]["id"]);
        assert_ne!(first.event["id"], second.event["id"]);
        assert_ne!(first.actions[0]["id"], second.actions[0]["id"]);
        assert_eq!(second.event["name"], json!("Custom"));
    }

    #[test]
    fn profiles_validate_entries_and_share_params() {
        let profile = parse_profile(&json!({
            "profileVersion": 1,
            "id": "minecraft",
            "name": "Minecraft",
            "params": {"commandPort": 8080},
            "templates": [
                {"template": minimal_doc(), "params": {"extra": true}},
                {"template": minimal_doc()},
            ],
        }))
        .expect("valid");
        assert_eq!(profile.entries.len(), 2);
        assert_eq!(profile.params.get("commandPort"), Some(&json!(8080)));
        assert_eq!(
            profile.entries[0].params.get("extra"),
            Some(&Value::Bool(true))
        );
        assert!(parse_profile(&json!({"profileVersion": 99})).is_err());
        assert!(parse_profile(&json!({
            "profileVersion": 1, "id": "x", "name": "X", "templates": []
        }))
        .is_err());
    }

    #[test]
    fn shared_fixtures_agree_with_the_web_engine() {
        let valid: Value = serde_json::from_str(include_str!(
            "../../../../examples/templates/fixtures/valid-minimal.json"
        ))
        .expect("fixture parses");
        assert!(parse_rule_template(&valid).is_ok());
        for name in ["invalid-version.json", "invalid-operator.json"] {
            let text = match name {
                "invalid-version.json" => {
                    include_str!("../../../../examples/templates/fixtures/invalid-version.json")
                }
                _ => include_str!("../../../../examples/templates/fixtures/invalid-operator.json"),
            };
            let doc: Value = serde_json::from_str(text).expect("fixture parses");
            assert!(parse_rule_template(&doc).is_err(), "{name} must fail");
        }
    }

    #[test]
    fn requirements_dedupe_action_types() {
        let template = parse_rule_template(&json!({
            "templateVersion": 1,
            "id": "two",
            "title": "Two",
            "actions": [
                {"name": "A", "typeId": "core.log", "config": {}},
                {"name": "B", "typeId": "core.log", "config": {}},
            ],
            "event": {"name": "E", "trigger": "tiktok.chat"},
        }))
        .expect("valid");
        assert_eq!(
            template_requirements(&template),
            (vec!["core.log".to_owned()], "tiktok.chat".to_owned())
        );
    }
}

/// Action type ids plus the trigger a template needs from the host.
pub fn template_requirements(template: &RuleTemplate) -> (Vec<String>, String) {
    let mut type_ids: Vec<String> = template
        .actions
        .iter()
        .map(|action| action.type_id.clone())
        .collect();
    type_ids.sort();
    type_ids.dedup();
    (type_ids, template.event.trigger.clone())
}
