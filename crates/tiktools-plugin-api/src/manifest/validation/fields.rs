//! Shared lexical fields, localized text and package-relative path checks. Callers retain
//! their domain-specific error mapping.

use super::MAX_EVENT_TYPE_LEN;
use serde_json::Value;

pub(super) fn is_header_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub(super) fn validate_localized(value: Option<&Value>) -> Result<(), &'static str> {
    let object = value
        .and_then(Value::as_object)
        .ok_or("localized text must be an object")?;
    let default = object
        .get("default")
        .and_then(Value::as_str)
        .ok_or("localized text needs a default")?;
    if default.trim().is_empty() || default.len() > 1_024 {
        return Err("localized default out of range");
    }
    if let Some(key) = object.get("i18key") {
        let key = key.as_str().ok_or("i18key must be a string")?;
        if key.trim().is_empty() || key.len() > 256 {
            return Err("i18key out of range");
        }
    }
    Ok(())
}

pub(super) fn is_valid_trigger_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=MAX_EVENT_TYPE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

pub fn is_valid_plugin_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=128).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[1..].iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

/// Validate a package-relative path before joining it with a plugin root.
pub fn is_safe_relative_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\0') || value.starts_with('/') || value.starts_with('\\')
    {
        return false;
    }
    let normalized = value.replace('\\', "/");
    if normalized.starts_with('/') || normalized.contains(":/") {
        return false;
    }
    let parts: Vec<&str> = normalized.split('/').collect();
    !parts.is_empty()
        && normalized != "."
        && parts.iter().all(|part| !part.is_empty() && *part != "..")
}
