//! Runtime global values for automations.
//!
//! Globals are operator-controlled key/value pairs rendered at event time
//! (`{{ globals.commandPort }}`), unlike `{{ params.* }}` which bakes once
//! at template import. They persist in the shared `app_state` store under
//! the reserved `globals.` prefix, so CLI, desktop, and headless hosts
//! stay in one library with zero schema changes. Rendering is textual:
//! values interpolate into URLs, headers, bodies, and messages.

use serde_json::Value;

use crate::control::OperationError;
use crate::helpers::is_identifier;

/// Reserved `app_state` prefix. Callers use bare keys; storage adds this.
pub const GLOBALS_KEY_PREFIX: &str = "globals.";
/// Bare keys stay short (identifiers, not paths).
pub const MAX_GLOBAL_KEY_LEN: usize = 64;
/// Values stay small (hosts, ports, tokens, short snippets).
pub const MAX_GLOBAL_VALUE_LEN: usize = 4096;
/// Enough for every integration; keeps render snapshots cheap.
pub const MAX_GLOBAL_KEYS: usize = 128;

/// Bare-key rule: identifier charset, `1..=64` bytes.
pub fn validate_key(key: &str) -> Result<(), OperationError> {
    if key.is_empty() || key.len() > MAX_GLOBAL_KEY_LEN || !is_identifier(key) {
        return Err(OperationError::invalid(
            "global keys are 1..=64 characters: letters, digits, and `._-`, starting with a letter or `_`",
        ));
    }
    Ok(())
}

/// `commandPort` → `globals.commandPort`.
pub fn storage_key(key: &str) -> String {
    format!("{GLOBALS_KEY_PREFIX}{key}")
}

/// `globals.commandPort` → `Some("commandPort")`; anything else → `None`.
pub fn bare_key(storage: &str) -> Option<&str> {
    storage
        .strip_prefix(GLOBALS_KEY_PREFIX)
        .filter(|key| !key.is_empty())
}

/// Normalizes raw CLI/API input for textual rendering: JSON parses when
/// possible (numbers and booleans keep their text), strings store bare
/// (no quotes), and containers store compact JSON.
pub fn normalize_value(raw: &str) -> Result<String, OperationError> {
    let parsed: Value = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_owned()));
    let normalized = match parsed {
        Value::String(value) => value,
        Value::Null => {
            return Err(OperationError::invalid("global value cannot be null"));
        }
        Value::Array(_) | Value::Object(_) => serde_json::to_string(&parsed).unwrap_or_default(),
        Value::Bool(_) | Value::Number(_) => parsed.to_string(),
    };
    if normalized.len() > MAX_GLOBAL_VALUE_LEN {
        return Err(OperationError::invalid(
            "global value exceeds 4096 characters",
        ));
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_rule_matches_identifiers() {
        for key in ["commandPort", "_port", "a", "A1._-b", "x"] {
            assert!(validate_key(key).is_ok(), "accepts {key}");
        }
        for key in [
            "",
            "9lives",
            "has space",
            "semi;colon",
            "slash/a",
            "{{ x }}",
        ] {
            assert!(validate_key(key).is_err(), "rejects {key:?}");
        }
        assert!(validate_key(&"k".repeat(65)).is_err());
    }

    #[test]
    fn storage_round_trip_keeps_bare_keys() {
        assert_eq!(storage_key("commandPort"), "globals.commandPort");
        assert_eq!(bare_key("globals.commandPort"), Some("commandPort"));
        assert_eq!(bare_key("globals."), None);
        assert_eq!(bare_key("behavior.templates.custom"), None);
    }

    #[test]
    fn values_normalize_for_textual_rendering() {
        assert_eq!(normalize_value("127.0.0.1").unwrap(), "127.0.0.1");
        assert_eq!(normalize_value("46665").unwrap(), "46665");
        assert_eq!(normalize_value("true").unwrap(), "true");
        assert_eq!(normalize_value("\"quoted\"").unwrap(), "quoted");
        assert_eq!(normalize_value("").unwrap(), "");
        assert_eq!(normalize_value(r#"{"a": 1}"#).unwrap(), r#"{"a":1}"#);
        assert!(normalize_value("null").is_err());
        assert!(normalize_value(&"v".repeat(4097)).is_err());
    }
}
