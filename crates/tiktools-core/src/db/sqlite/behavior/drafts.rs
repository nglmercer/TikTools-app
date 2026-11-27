//! Editor-safe structure for fresh remote drafts. This does not grant execution
//! authority or validate the effects of unknown plugin action/trigger names.
use super::*;
use crate::wire;
use serde_json::Map;

const ITEMS: usize = 512;
const MAX_SAFE_NUMBER: f64 = 9_007_199_254_740_991.0;

fn invalid() -> DatabaseError {
    DatabaseError::Invalid("invalid disabled automation draft structure".into())
}

fn canonical(value: &Value, bytes: usize) -> bool {
    value.as_str().is_some_and(|s| wire::canonical_id(s, bytes))
}

fn schema(
    object: &mut Map<String, Value>,
    default: u64,
    allowed: &[u64],
) -> Result<(), DatabaseError> {
    let value = object.entry("schemaVersion").or_insert(default.into());
    if !value.as_u64().is_some_and(|v| allowed.contains(&v)) {
        return Err(invalid());
    }
    Ok(())
}

fn choice(
    object: &mut Map<String, Value>,
    key: &str,
    default: &str,
    allowed: &[&str],
) -> Result<(), DatabaseError> {
    let value = object.entry(key).or_insert_with(|| default.into());
    if !value.as_str().is_some_and(|v| allowed.contains(&v)) {
        return Err(invalid());
    }
    Ok(())
}

fn validate_filter(filter: &mut Value) -> Result<(), DatabaseError> {
    let filter = filter.as_object_mut().ok_or_else(invalid)?;
    if !filter
        .get("path")
        .and_then(Value::as_str)
        .is_some_and(|path| {
            !path.is_empty()
                && path.len() <= 256
                && path.trim() == path
                && !path.chars().any(char::is_control)
        })
        || !filter
            .get("operator")
            .and_then(Value::as_str)
            .is_some_and(|op| {
                matches!(
                    op,
                    "gte"
                        | "gt"
                        | "lte"
                        | "lt"
                        | "eq"
                        | "neq"
                        | "contains"
                        | "starts-with"
                        | "in"
                        | "is-true"
                        | "is-false"
                )
            })
    {
        return Err(invalid());
    }
    if !filter
        .entry("value")
        .or_insert_with(|| "".into())
        .is_string()
    {
        return Err(invalid());
    }
    if let Some(values) = filter.get("values") {
        let values = values.as_array().ok_or_else(invalid)?;
        if values.len() > ITEMS || !values.iter().all(Value::is_string) {
            return Err(invalid());
        }
    }
    if let Some(quantifier) = filter.get("quantifier") {
        if !quantifier
            .as_str()
            .is_some_and(|q| matches!(q, "any" | "all"))
        {
            return Err(invalid());
        }
    }
    if let Some(alias) = filter.get("as") {
        let valid = alias.as_str().is_some_and(|a| {
            !a.is_empty()
                && a.len() <= 32
                && a.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && a.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !matches!(a, "match" | "event" | "globals" | "data" | "user")
        });
        if !valid {
            return Err(invalid());
        }
    }
    Ok(())
}

fn event(object: &mut Map<String, Value>) -> Result<(), DatabaseError> {
    if !object.get("trigger").is_some_and(|v| canonical(v, 128)) {
        return Err(invalid());
    }
    schema(object, 1, &[1])?;
    let ids = object
        .entry("actionIds")
        .or_insert_with(|| json!([]))
        .as_array()
        .ok_or_else(invalid)?;
    if ids.len() > ITEMS || !ids.iter().all(|v| canonical(v, 128)) {
        return Err(invalid());
    }
    let filters = object
        .entry("filters")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(invalid)?;
    if filters.len() > ITEMS {
        return Err(invalid());
    }
    for filter in filters {
        validate_filter(filter)?;
    }
    // Flat OR groups: at most 4 groups of at most 12 filters each.
    if let Some(any_of) = object.get_mut("anyOf") {
        let groups = any_of.as_array_mut().ok_or_else(invalid)?;
        if groups.len() > 4 {
            return Err(invalid());
        }
        for group in groups {
            let items = group.as_array_mut().ok_or_else(invalid)?;
            if items.is_empty() || items.len() > 12 {
                return Err(invalid());
            }
            for filter in items {
                validate_filter(filter)?;
            }
        }
    }
    let cooldown = object.entry("cooldownMs").or_insert_with(|| 0.into());
    if !cooldown
        .as_f64()
        .is_some_and(|v| v.is_finite() && (0.0..=MAX_SAFE_NUMBER).contains(&v))
    {
        return Err(invalid());
    }
    choice(object, "cooldownScope", "global", &["global", "user"])?;
    choice(object, "runMode", "all", &["all", "random"])?;
    choice(object, "matchMode", "and", &["and", "or"])?;
    Ok(())
}

pub(super) fn normalize(table: &str, record: &Value) -> Result<Value, DatabaseError> {
    let mut record = record.clone();
    let object = record.as_object_mut().ok_or_else(invalid)?;
    if object.get("enabled") != Some(&Value::Bool(false)) {
        return Err(invalid());
    }
    required_value_string(object, "name").map_err(|_| invalid())?;
    match table {
        "behavior_actions" => {
            if !object.get("typeId").is_some_and(|v| canonical(v, 128)) {
                return Err(invalid());
            }
            if !object
                .entry("config")
                .or_insert_with(|| json!({}))
                .is_object()
            {
                return Err(invalid());
            }
            schema(object, 2, &[1, 2])?;
        }
        "behavior_events" => event(object)?,
        _ => return Err(invalid()),
    }
    Ok(record)
}
