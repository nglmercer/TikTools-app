//! Response validation for the v3 unified catalog. The server owns ordering and
//! cursors; the client only refuses shapes it cannot render or safely forward.
use crate::SyncError;
use serde_json::Value;

const MAX_V3_PAGE: usize = 50;
const MAX_V3_CURSOR: usize = 1024;

/// Opaque cursors are base64url JSON. Anything else is refused before it is sent.
pub(super) fn valid_v3_cursor(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_V3_CURSOR
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'='))
}

/// Item identifiers for plugins, templates and profiles. Templates and profiles
/// may use identifiers that the v2 plugin id rule would reject.
pub(super) fn valid_item_id_v3(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_identity(item: &Value) -> bool {
    let Some(identity) = item.get("identity").and_then(Value::as_object) else {
        return false;
    };
    matches!(
        identity.get("kind").and_then(Value::as_str),
        Some("plugin" | "template" | "profile")
    ) && identity
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(valid_item_id_v3)
}

pub(super) fn validate_item_v3(value: &Value) -> Result<(), SyncError> {
    if valid_identity(value) {
        Ok(())
    } else {
        Err(SyncError::Protocol)
    }
}

pub(super) fn validate_items_v3(value: &Value) -> Result<(), SyncError> {
    let object = value.as_object().ok_or(SyncError::Protocol)?;
    let items = object
        .get("items")
        .and_then(Value::as_array)
        .ok_or(SyncError::Protocol)?;
    if items.len() > MAX_V3_PAGE || !items.iter().all(valid_identity) {
        return Err(SyncError::Protocol);
    }
    match object.get("nextCursor") {
        Some(Value::Null) | None => Ok(()),
        Some(Value::String(cursor)) if valid_v3_cursor(cursor) => Ok(()),
        _ => Err(SyncError::Protocol),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cursors_and_ids_are_bounded_to_the_wire_alphabet() {
        assert!(valid_v3_cursor("eyJrIjoxfQ=="));
        assert!(!valid_v3_cursor(""));
        assert!(!valid_v3_cursor("a b"));
        assert!(!valid_v3_cursor(&"a".repeat(MAX_V3_CURSOR + 1)));
        assert!(valid_item_id_v3("3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c"));
        assert!(!valid_item_id_v3("../etc"));
    }

    #[test]
    fn page_shape_is_checked_before_forwarding() {
        let good =
            json!({"items":[{"identity":{"kind":"plugin","id":"acme.gift"}}],"nextCursor":null});
        assert!(validate_items_v3(&good).is_ok());
        let bad_kind =
            json!({"items":[{"identity":{"kind":"overlay","id":"x"}}],"nextCursor":null});
        assert!(validate_items_v3(&bad_kind).is_err());
        let bad_cursor = json!({"items":[],"nextCursor":42});
        assert!(validate_items_v3(&bad_cursor).is_err());
    }
}
