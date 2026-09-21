//! Plugin runtime unit tests.

use super::{
    activation_from_snapshot, manifest_action_declares_field, PluginActionDescriptor,
    PluginActivation,
};
use crate::*;

#[test]
fn activation_snapshot_defaults_missing_rows_to_active() {
    let snapshot = json!({
        "plugins": [
            {"id": "on", "installed": true, "enabled": true},
            {"id": "off", "installed": true, "enabled": false},
            {"id": "partial"},
        ],
    });
    let activation = activation_from_snapshot(&snapshot);
    assert_eq!(activation.len(), 3);
    assert_eq!(
        activation["on"],
        PluginActivation {
            installed: true,
            enabled: true,
        }
    );
    assert!(!activation["off"].enabled);
    assert_eq!(
        activation["partial"],
        PluginActivation {
            installed: true,
            enabled: true,
        }
    );
    assert!(activation_from_snapshot(&json!({})).is_empty());
}

#[test]
fn action_timeout_defaults_to_thirty_seconds() {
    let descriptor: PluginActionDescriptor = serde_json::from_value(json!({
        "id": "demo.action"
    }))
    .unwrap();
    assert_eq!(descriptor.timeout(), std::time::Duration::from_secs(30));
}

#[test]
fn action_timeout_uses_manifest_milliseconds() {
    let descriptor: PluginActionDescriptor = serde_json::from_value(json!({
        "id": "demo.action",
        "timeoutMs": 120000
    }))
    .unwrap();
    assert_eq!(descriptor.timeout(), std::time::Duration::from_secs(120));
}

#[test]
fn text_rule_follows_declared_action_fields() {
    let manifest = tiktools_plugin_api::PluginManifest::from_json_str(
        &serde_json::json!({
            "schemaVersion": 3,
            "id": "demo.tts",
            "name": "Demo",
            "version": "1.0.0",
            "runtime": "declarative",
            "actionTypes": [
                {"id": "demo.tts.speak", "fields": [{"key": "text"}, {"key": "voice"}]},
                {"id": "demo.tts.set-output-device", "fields": [{"key": "device"}]},
                {"id": "demo.tts.bare"}
            ]
        })
        .to_string(),
    )
    .unwrap();
    // The TTS tester keeps its text requirement...
    assert!(manifest_action_declares_field(
        &manifest,
        "demo.tts.speak",
        "text"
    ));
    // ...while textless actions and unknown ids run without it.
    assert!(!manifest_action_declares_field(
        &manifest,
        "demo.tts.set-output-device",
        "text"
    ));
    assert!(manifest_action_declares_field(
        &manifest,
        "demo.tts.set-output-device",
        "device"
    ));
    assert!(!manifest_action_declares_field(
        &manifest,
        "demo.tts.bare",
        "text"
    ));
    assert!(!manifest_action_declares_field(
        &manifest,
        "demo.tts.missing",
        "text"
    ));
}
