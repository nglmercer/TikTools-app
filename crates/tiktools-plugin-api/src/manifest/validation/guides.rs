//! Declarative guided-help contributions declared by plugins.
//!
//! Guides are data, never executable tour logic. The schema deliberately
//! exposes only host-owned semantic targets; plugins cannot name CSS
//! selectors, navigate arbitrarily, or inject markup/scripts.

use super::fields::{is_header_token, validate_localized};
use super::ManifestError;
use serde_json::Value;
use std::collections::BTreeSet;

const MAX_GUIDES: usize = 16;
const MAX_GUIDE_STEPS: usize = 16;
const MAX_GUIDE_TEXT_BYTES: usize = 8 * 1024;

fn invalid() -> ManifestError {
    ManifestError::InvalidField("guides")
}

fn valid_slug(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn action_type_descriptor<'a>(action_types: &'a [Value], action_type: &str) -> Option<&'a Value> {
    action_types.iter().find(|entry| {
        entry
            .as_object()
            .and_then(|object| object.get("id"))
            .and_then(Value::as_str)
            == Some(action_type)
    })
}

fn action_field_keys(action_type: &Value) -> Vec<&str> {
    let mut keys = Vec::new();
    if let Some(fields) = action_type.get("fields").and_then(Value::as_array) {
        for field in fields {
            if let Some(key) = field.get("key").and_then(Value::as_str) {
                keys.push(key);
            }
        }
    }
    if let Some(properties) = action_type
        .get("configSchema")
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)
    {
        for key in properties.keys() {
            if !keys.contains(&key.as_str()) {
                keys.push(key.as_str());
            }
        }
    }
    keys
}

fn allowed_keys(object: &serde_json::Map<String, Value>, allowed: &[&str]) -> bool {
    object.keys().all(|key| allowed.contains(&key.as_str()))
}

fn validate_step(step: &Value, field_keys: &[&str]) -> Result<(), ManifestError> {
    let object = step.as_object().ok_or_else(invalid)?;
    if !allowed_keys(
        object,
        &["id", "target", "title", "description", "optional"],
    ) {
        return Err(invalid());
    }
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    if !valid_slug(id, 64) {
        return Err(invalid());
    }
    validate_localized(object.get("title")).map_err(|_| invalid())?;
    validate_localized(object.get("description")).map_err(|_| invalid())?;
    if step.to_string().len() > MAX_GUIDE_TEXT_BYTES {
        return Err(invalid());
    }
    if let Some(optional) = object.get("optional") {
        if !optional.is_boolean() {
            return Err(invalid());
        }
    }

    let target = object
        .get("target")
        .and_then(Value::as_object)
        .ok_or_else(invalid)?;
    if !allowed_keys(target, &["kind", "key"]) {
        return Err(invalid());
    }
    let kind = target
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    match kind {
        // Closed host-owned regions of the action editor.
        "editor" | "settings" | "permissions" => {
            if target.contains_key("key") {
                return Err(invalid());
            }
        }
        // Field-level help must reference a real key declared by this action
        // type, so a manifest edit cannot leave the tour pointing at a field
        // that no longer exists.
        "field" => {
            let key = target
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(invalid)?;
            if key.len() > 128 || !field_keys.contains(&key) {
                return Err(invalid());
            }
        }
        _ => return Err(invalid()),
    }
    Ok(())
}

/// Validate one manifest `guides` entry against the action types declared by
/// the same plugin. Entries are checked when the host merges the behavior
/// snapshot; invalid entries are skipped there rather than breaking discovery.
pub fn validate_plugin_guide(entry: &Value, action_types: &[Value]) -> Result<(), ManifestError> {
    let object = entry.as_object().ok_or_else(invalid)?;
    if !allowed_keys(
        object,
        &[
            "id",
            "kind",
            "actionType",
            "mode",
            "title",
            "description",
            "icon",
            "steps",
        ],
    ) {
        return Err(invalid());
    }
    if entry.to_string().len() > MAX_DESCRIPTOR_GUIDE_BYTES {
        return Err(invalid());
    }
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    if !valid_slug(id, 64) {
        return Err(invalid());
    }
    if object.get("kind").and_then(Value::as_str) != Some("automation-action") {
        return Err(invalid());
    }
    let action_type = object
        .get("actionType")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    let action = action_type_descriptor(action_types, action_type).ok_or_else(invalid)?;
    match object.get("mode").and_then(Value::as_str).unwrap_or("both") {
        "create" | "edit" | "both" => {}
        _ => return Err(invalid()),
    }
    validate_localized(object.get("title")).map_err(|_| invalid())?;
    validate_localized(object.get("description")).map_err(|_| invalid())?;
    if let Some(icon) = object.get("icon") {
        let icon = icon.as_str().ok_or_else(invalid)?;
        if !is_header_token(icon) {
            return Err(invalid());
        }
    }

    let steps = object
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if steps.is_empty() || steps.len() > MAX_GUIDE_STEPS {
        return Err(invalid());
    }
    let field_keys = action_field_keys(action);
    let mut step_ids = BTreeSet::new();
    for step in steps {
        validate_step(step, &field_keys)?;
        let id = step.get("id").and_then(Value::as_str).ok_or_else(invalid)?;
        if !step_ids.insert(id) {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Bound for one whole guide document. Sixteen short steps fit comfortably;
/// larger documents are almost certainly markup or duplicated field copy and
/// must not ride every automation snapshot.
const MAX_DESCRIPTOR_GUIDE_BYTES: usize = 24 * 1024;

/// Upper bound used by hosts that expose the contribution count in diagnostics.
pub const PLUGIN_GUIDE_LIMIT: usize = MAX_GUIDES;
