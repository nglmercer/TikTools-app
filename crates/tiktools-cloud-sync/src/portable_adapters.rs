//! Portable adapters for `preferences` and `plugin-settings`.
//!
//! The portable shapes are the Cloud validators' contract (see
//! `cloud-contracts::portable::validate`). Native objects may carry fields
//! the portable schema does not know. Conversion extracts only the portable
//! subset for Cloud; application merges that subset back and preserves every
//! unknown native field, including secrets and local-only keys.

use serde_json::{Map, Value};

/// Extract the portable preferences subset from a native object. Unknown
/// fields are dropped for Cloud; they remain in the native object.
pub fn preference_from_native(native: &Value) -> Option<Value> {
    let native = native.as_object()?;
    let mut portable = Map::new();
    portable.insert("schemaVersion".into(), Value::from(1));
    for key in ["theme", "locale", "reducedMotion", "updateChannel"] {
        if let Some(value) = native.get(key) {
            portable.insert(key.into(), value.clone());
        }
    }
    Some(Value::Object(portable))
}

/// Merge a portable preferences value into a native object, preserving
/// unknown fields. Returns true when the native object changed.
pub fn preference_apply(native: &mut Value, portable: &Value) -> bool {
    let Some(portable) = portable.as_object() else {
        return false;
    };
    if !native.is_object() {
        *native = Value::Object(Map::new());
    }
    let map = native
        .as_object_mut()
        .expect("native preferences must be an object");
    let mut changed = false;
    for key in ["theme", "locale", "reducedMotion", "updateChannel"] {
        if let Some(value) = portable.get(key) {
            if map.get(key) != Some(value) {
                map.insert(key.into(), value.clone());
                changed = true;
            }
        }
    }
    changed
}

/// Wrap a native plugin-settings object in its portable envelope. The
/// inner object is cloned verbatim (including unknown nested fields).
pub fn settings_from_native(plugin_id: &str, native_settings: &Value) -> Option<Value> {
    if !native_settings.is_object() {
        return None;
    }
    let mut envelope = Map::new();
    envelope.insert("schemaVersion".into(), Value::from(1));
    envelope.insert("pluginId".into(), Value::from(plugin_id));
    envelope.insert("settings".into(), native_settings.clone());
    Some(Value::Object(envelope))
}

/// Merge portable plugin-settings into a native settings object, preserving
/// unknown top-level fields. Returns true when the native object changed.
pub fn settings_apply(plugin_id: &str, native: &mut Value, portable: &Value) -> bool {
    let Some(portable) = portable.as_object() else {
        return false;
    };
    if portable.get("pluginId").and_then(Value::as_str) != Some(plugin_id) {
        return false;
    }
    let Some(portable_settings) = portable.get("settings") else {
        return false;
    };
    if !native.is_object() {
        *native = Value::Object(Map::new());
    }
    let map = native
        .as_object_mut()
        .expect("native settings must be an object");
    let mut changed = false;
    if let Some(portable_map) = portable_settings.as_object() {
        for (key, value) in portable_map {
            if map.get(key) != Some(value) {
                map.insert(key.clone(), value.clone());
                changed = true;
            }
        }
    } else if map.get("__portableSettings") != Some(portable_settings) {
        // Non-object settings payloads are applied as a single preserved value.
        map.insert("__portableSettings".into(), portable_settings.clone());
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preference_round_trip_preserves_unknown_fields() {
        let mut native = json!({
            "theme": "light",
            "locale": "en-US",
            "reducedMotion": false,
            "updateChannel": "stable",
            "futureDesktopOnly": {"keep": "me"},
            "localSecret": "device-only"
        });
        let portable = preference_from_native(&native).expect("portable subset");
        assert_eq!(portable["schemaVersion"], 1);
        assert_eq!(portable["theme"], "light");
        assert!(portable.get("futureDesktopOnly").is_none());
        let mut divergent = json!({
            "theme": "dark",
            "futureDesktopOnly": {"keep": "me"},
            "localSecret": "device-only",
            "anotherLocal": true
        });
        assert!(preference_apply(&mut divergent, &portable));
        assert_eq!(divergent["theme"], "light");
        assert_eq!(divergent["futureDesktopOnly"]["keep"], "me");
        assert_eq!(divergent["localSecret"], "device-only");
        assert_eq!(divergent["anotherLocal"], true);
        assert!(!preference_apply(&mut divergent, &portable));
    }

    #[test]
    fn settings_round_trip_preserves_unknown_fields() {
        let native = json!({"volume": 50, "nested": {"keep": true}, "localOnly": [1, 2]});
        let portable = settings_from_native("acme.plugin", &native).expect("envelope");
        assert_eq!(portable["pluginId"], "acme.plugin");
        assert_eq!(portable["settings"]["localOnly"], json!([1, 2]));
        let mut target = json!({"volume": 10, "localOnly": [1, 2], "otherLocal": "x"});
        assert!(settings_apply("acme.plugin", &mut target, &portable));
        assert_eq!(target["volume"], 50);
        assert_eq!(target["otherLocal"], "x");
        assert!(!settings_apply("acme.plugin", &mut target, &portable));
        assert!(!settings_apply("other.plugin", &mut target, &portable));
    }
}
