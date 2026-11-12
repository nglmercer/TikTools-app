//! Creation-time parameter paths, scalar substitution and layer merging.
use super::*;

fn read_param_path(params: &Map<String, Value>, path: &str) -> Option<Value> {
    read_param_path_ref(params, path).cloned()
}

pub(super) fn read_param_path_ref<'a>(
    params: &'a Map<String, Value>,
    path: &str,
) -> Option<&'a Value> {
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
    current
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
pub(super) fn scan_params_span(text: &str, start: usize) -> Option<(usize, String)> {
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

pub(super) fn substitute_string(text: &str, params: &Map<String, Value>) -> Value {
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

pub(super) fn sole_params_span(text: &str) -> Option<String> {
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
