//! Closed rule/profile document validation and bounded list imports.
use super::*;

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
                        enabled: item.get("enabled").is_none_or(|v| v == &Value::Bool(true)),
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
                    enabled: raw.get("enabled").is_none_or(|v| v == &Value::Bool(true)),
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
