//! Filter predicate language, event path access, and value normalization.

use super::*;

pub(crate) fn event_record_matches(record: &Value, event: &Value) -> bool {
    let Some(filters) = record.get("filters").and_then(Value::as_array) else {
        return true;
    };
    filters.iter().all(|filter| matches_filter(filter, event))
}

fn matches_filter(filter: &Value, event: &Value) -> bool {
    let path = filter
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let raw = read_event_path(event, path);
    let left = raw.map(value_to_string).unwrap_or_default();
    let right = filter
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let left_number = left.parse::<f64>().ok();
    let right_number = right.parse::<f64>().ok();
    let numeric = left_number.is_some() && right_number.is_some() && !right.trim().is_empty();
    match filter
        .get("operator")
        .and_then(Value::as_str)
        .unwrap_or("eq")
    {
        "gte" => numeric && left_number >= right_number,
        "gt" => numeric && left_number > right_number,
        "lte" => numeric && left_number <= right_number,
        "lt" => numeric && left_number < right_number,
        "eq" => {
            if numeric {
                left_number == right_number
            } else {
                left == right
            }
        }
        "neq" => {
            if numeric {
                left_number != right_number
            } else {
                left != right
            }
        }
        "contains" => left
            .to_ascii_lowercase()
            .contains(&right.to_ascii_lowercase()),
        "starts-with" => left
            .to_ascii_lowercase()
            .starts_with(&right.to_ascii_lowercase()),
        "in" => filter
            .get("values")
            .and_then(Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|value| value.trim().eq_ignore_ascii_case(left.trim()))
            }),
        "is-true" => raw == Some(&Value::Bool(true)) || left == "true" || left == "1",
        "is-false" => {
            raw == Some(&Value::Bool(false)) || left == "false" || left == "0" || left.is_empty()
        }
        _ => false,
    }
}

/// Reads one `event.*` path with the shared filter/template path language
/// (a leading `event.` prefix addresses the envelope root).
pub(crate) fn read_event_path<'a>(event: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path
        .trim()
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim();
    let path = path.strip_prefix("event.").unwrap_or(path);
    let path = path.strip_prefix("event").unwrap_or(path);
    let mut current = event;
    for part in path.trim_matches('.').split('.') {
        if part.is_empty() {
            continue;
        }
        current = current.get(part)?;
    }
    Some(current)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

pub(crate) fn number_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| {
            value
                .as_f64()
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value as u64)
        })
        .or_else(|| value.as_str()?.parse().ok())
}
