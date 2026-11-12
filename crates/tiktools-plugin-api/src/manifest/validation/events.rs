//! Plugin event declarations and domain subscription grammar. Host event namespaces remain
//! reserved for declarations.

use super::{
    ManifestError, MAX_EVENT_FIELDS, MAX_EVENT_OPTIONS, MAX_EVENT_TYPE_LEN, RESERVED_EVENT_PREFIXES,
};
use serde_json::Value;

/// Validated event types from one plugin manifest, in manifest order.
/// Invalid entries are skipped (the host catalog merge reports them);
/// callers treat the result as the authoritative publish allowlist for
/// the plugin, on both the poll and push paths.
pub fn declared_event_types(manifest: &crate::manifest::PluginManifest) -> Vec<String> {
    manifest
        .event_types
        .iter()
        .filter_map(|entry| {
            if validate_event_type(entry).is_err() {
                return None;
            }
            entry
                .get("type")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect()
}

/// Validate one eventTypes entry from a plugin manifest. Shape errors are
/// reported by the host catalog merge, which skips the entry with a warning.
pub fn validate_event_type(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let event_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    if !is_valid_event_type(event_type) {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    let title = object
        .get("title")
        .and_then(Value::as_object)
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let default = title
        .get("default")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if default.trim().is_empty() || default.len() > 120 {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    if let Some(fields) = object.get("fields") {
        let fields = fields
            .as_array()
            .ok_or(ManifestError::InvalidField("eventTypes"))?;
        if fields.len() > MAX_EVENT_FIELDS {
            return Err(ManifestError::InvalidField("eventTypes"));
        }
        for field in fields {
            validate_event_field(field)?;
        }
    }
    if object
        .get("sample")
        .is_some_and(|sample| !sample.is_object())
    {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    Ok(())
}

fn validate_event_field(field: &Value) -> Result<(), ManifestError> {
    let object = field
        .as_object()
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if path.trim().is_empty() || path.len() > 200 || path.chars().any(char::is_whitespace) {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    if let Some(kind) = object.get("kind") {
        let kind = kind.as_str().unwrap_or_default();
        if !matches!(kind, "text" | "number" | "boolean") {
            return Err(ManifestError::InvalidField("eventTypes"));
        }
    }
    if let Some(options) = object.get("options") {
        let options = options
            .as_array()
            .ok_or(ManifestError::InvalidField("eventTypes"))?;
        if options.len() > MAX_EVENT_OPTIONS {
            return Err(ManifestError::InvalidField("eventTypes"));
        }
        for option in options {
            validate_event_option(option)?;
        }
    }
    Ok(())
}

fn validate_event_option(option: &Value) -> Result<(), ManifestError> {
    let object = option
        .as_object()
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let value = object
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    // Empty values are legitimate ("none" options); only bound the length.
    if value.len() > 64 {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    if object.get("label").is_some_and(|label| !label.is_object()) {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    Ok(())
}
/// Event type names are dotted lowercase: hotkey.pressed, timer.tick.
/// Host namespaces stay reserved so a plugin can never shadow built-in
/// triggers or the internal plugin.emit channel.
pub fn is_valid_event_type(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=MAX_EVENT_TYPE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
        && !RESERVED_EVENT_PREFIXES
            .iter()
            .any(|prefix| value.starts_with(prefix))
}

/// Domain event subscriptions may observe host namespaces, unlike plugin
/// `eventTypes` which are restricted to plugin-owned trigger namespaces.
/// Keep the grammar deliberately small so wildcard routing is deterministic.
pub fn is_valid_event_subscription(value: &str) -> bool {
    if value == "*" {
        return true;
    }
    let topic = value.strip_suffix(".*").unwrap_or(value);
    let bytes = topic.as_bytes();
    (2..=MAX_EVENT_TYPE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
        && (!value.contains('*') || value.ends_with(".*"))
}
