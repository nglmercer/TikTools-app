use super::*;
use serde_json::Value;

#[test]
fn reads_new_native_manifest_and_descriptors() {
    let manifest = PluginManifest::from_json_str(
        r#"{
            "schemaVersion": 2,
            "id": "miniaudio",
            "name": "MiniAudio",
            "version": "1.2.0",
            "runtime": "native",
            "entry": "native/miniaudio.dll",
            "actionTypes": [{"id":"audio.play"}],
            "settings": {"schema": {"type":"object"}},
            "permissions": ["audio"]
        }"#,
    )
    .unwrap();

    assert_eq!(manifest.runtime, PluginRuntimeKind::Native);
    assert_eq!(manifest.entry, "native/miniaudio.dll");
    assert_eq!(manifest.action_types.len(), 1);
    assert!(manifest.settings_schema.is_some());
}

#[test]
fn rejects_removed_schema_and_unsafe_entries() {
    assert!(matches!(
        PluginManifest::from_json_str(
            r#"{"schemaVersion":1,"id":"demo","name":"Demo","version":"1.0.0","main":"index.js"}"#
        ),
        Err(ManifestError::UnsupportedSchema(1))
    ));
    assert!(matches!(
        PluginManifest::from_json_str(
            r#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"../index"}"#
        ),
        Err(ManifestError::UnsafeEntry)
    ));
}

#[test]
fn reads_event_types_and_validates_them() {
    let manifest = PluginManifest::from_json_str(
        r#"{"schemaVersion": 2, "id": "hotkeys", "name": "Hotkeys", "version": "1.0.0", "runtime": "process", "entry": "hotkeys", "capabilities": ["events.publish"], "eventTypes": [{"type": "hotkey.pressed", "title": {"default": "Hotkey pressed"}, "fields": [{"path": "event.data.key", "kind": "text"}], "sample": {"key": "ctrl+k"}}]}"#,
    )
    .unwrap();
    assert_eq!(manifest.event_types.len(), 1);
    assert!(validate_event_type(&manifest.event_types[0]).is_ok());

    // Fixed field options validate; empty values and bad labels do not.
    assert!(validate_event_type(&serde_json::json!({
        "type": "hotkey.pressed",
        "title": {"default": "Hotkey pressed"},
        "fields": [{"path": "event.data.key", "options": [{"value": "k"}, {"value": "space", "label": {"default": "Space"}}]}]
    }))
    .is_ok());
    assert!(validate_event_type(&serde_json::json!({
        "type": "hotkey.pressed",
        "title": {"default": "Hotkey pressed"},
        "fields": [{"path": "event.data.key", "options": [{"value": ""}, {"value": "k", "label": "oops"}]}]
    }))
    .is_err());

    // Reserved host namespaces can never be shadowed.
    for reserved in [
        "tiktok.chat",
        "points.awarded",
        "plugin.emit",
        "plugin.custom",
    ] {
        assert!(
            !is_valid_event_type(reserved),
            "{reserved} should be reserved"
        );
    }
    assert!(is_valid_event_type("hotkey.pressed"));
    assert!(is_valid_event_type("dom.match"));
    assert!(!is_valid_event_type("SHOUTY"));
    assert!(!is_valid_event_type("x"));

    // Missing title default is rejected.
    assert!(validate_event_type(&serde_json::json!({"type": "hotkey.pressed"})).is_err());
    assert!(validate_event_type(&serde_json::json!({"type": "hotkey.pressed", "title": {"default": "Hotkey pressed"}, "fields": [{"path": "event.data.key", "kind": "image"}]})).is_err());
}

#[test]
fn omitted_trust_preserves_schema_v2_defaults() {
    let process = PluginManifest::from_json_str(
        r#"{"schemaVersion":2,"id":"process","name":"Process","version":"1.0.0","runtime":"process","entry":"plugin.exe"}"#,
    )
    .unwrap();
    let wasm = PluginManifest::from_json_str(
        r#"{"schemaVersion":2,"id":"wasm","name":"WASM","version":"1.0.0","runtime":"wasm","entry":"plugin.wasm"}"#,
    )
    .unwrap();
    assert_eq!(process.trust, PluginTrust::Sandboxed);
    assert_eq!(wasm.trust, PluginTrust::Sandboxed);
    assert_eq!(process.security_model(), PluginSecurityModel::Isolated);
    assert_eq!(wasm.security_model(), PluginSecurityModel::Sandboxed);
    assert_eq!(
        PluginRuntimeKind::Native.security_model(),
        PluginSecurityModel::Trusted
    );
}

#[test]
fn current_target_is_platform_qualified() {
    assert!(current_target().starts_with(&format!("{}-", current_platform())));
}

#[test]
fn rejects_traversal() {
    assert!(!is_safe_relative_path("../../secret"));
    assert!(!is_safe_relative_path("native/../secret"));
    assert!(is_safe_relative_path("native/plugin.dll"));
}

#[test]
fn reads_processor_types_and_validates_them() {
    let manifest = PluginManifest::from_json_str(
        r#"{"schemaVersion": 2, "id": "textintel", "name": "Text Intelligence", "version": "0.1.0", "runtime": "process", "entry": "tiktools-textintel", "capabilities": ["events.enrich"], "processorTypes": [{"id": "textintel.analyze", "title": {"default": "Text Intelligence"}, "description": {"default": "Analyze comments."}, "eventTypes": ["tiktok.chat"], "stage": "pre-filter", "timeoutMs": 75, "failureMode": "pass-through", "inputs": [{"path": "event.data.comment", "role": "message"}, {"path": "event.user.nickname", "role": "display-name"}]}]}"#,
    )
    .unwrap();
    assert_eq!(manifest.processor_types.len(), 1);
    assert!(validate_processor_type(&manifest.processor_types[0]).is_ok());

    // Minimal descriptors validate; stage/failure mode default.
    let minimal = serde_json::json!({
        "id": "textintel.analyze",
        "title": {"default": "Text Intelligence"}
    });
    assert!(validate_processor_type(&minimal).is_ok());
    let typed: PluginProcessorDescriptor = serde_json::from_value(minimal).unwrap();
    assert_eq!(typed.stage, ProcessorStage::PreFilter);
    assert_eq!(typed.failure_mode, ProcessorFailureMode::PassThrough);
    assert_eq!(
        typed.timeout(),
        std::time::Duration::from_millis(DEFAULT_PLUGIN_PROCESSOR_TIMEOUT_MS)
    );

    // Subscribing to host event namespaces is allowed for processors.
    assert!(is_valid_processor_event_type("tiktok.chat"));
    assert!(is_valid_processor_event_type("points.awarded"));
    assert!(is_valid_processor_event_type("plugin.emit"));
    assert!(is_valid_processor_event_type("hotkey.pressed"));
    assert!(!is_valid_processor_event_type("SHOUTY"));
    assert!(!is_valid_processor_event_type("x"));

    // Malformed descriptors are rejected.
    for entry in [
        serde_json::json!({"title": {"default": "Missing id"}}),
        serde_json::json!({"id": "SHOUTY", "title": {"default": "Bad id"}}),
        serde_json::json!({"id": "x", "title": {"default": "Short id"}}),
        serde_json::json!({"id": "ok.id"}),
        serde_json::json!({"id": "ok.id", "title": {"default": ""}}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "description": "nope"}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "eventTypes": ["SHOUTY"]}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "stage": "post-filter"}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "failureMode": "drop"}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "inputs": [{"path": "", "role": "message"}]}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "inputs": [{"path": "event.data.comment", "role": "Bad Role"}]}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "inputs": [{"path": "event.data.comment"}]}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "inputs": [
            {"path": "event.data.comment", "role": "message"},
            {"path": "event.data.other", "role": "message"},
        ]}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "priority": "high"}),
        serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "priority": 9_000_000_000i64}),
    ] {
        assert!(
            validate_processor_type(&entry).is_err(),
            "should reject {entry}"
        );
    }
}

#[test]
fn manifests_without_processor_types_still_work() {
    let manifest = PluginManifest::from_json_str(
        r#"{"schemaVersion": 2, "id": "legacy", "name": "Legacy", "version": "1.0.0", "runtime": "process", "entry": "legacy"}"#,
    )
    .unwrap();
    assert!(manifest.processor_types.is_empty());
    assert!(manifest.validate_compatibility().is_ok());
}

#[test]
fn validates_processor_collection_and_timeout_bounds() {
    let manifest = |descriptor: &str| {
        format!(
            r#"{{"schemaVersion":2,"id":"proc","name":"Proc","version":"1.0.0","runtime":"process","entry":"proc","processorTypes":[{descriptor}]}}"#
        )
    };
    // Discovery keeps raw entries (validated at catalog build), but the
    // entry validator enforces collection and timeout bounds.
    let many_inputs = (0..17)
        .map(|index| format!(r#"{{"path": "event.data.field{index}", "role": "message"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    assert!(validate_processor_type(&serde_json::json!({
        "id": "proc.many",
        "title": {"default": "Too many inputs"},
        "inputs": serde_json::from_str::<Value>(&format!("[{many_inputs}]")).unwrap()
    }))
    .is_err());
    let many_events = (0..33)
        .map(|index| format!(r#""custom.event{index}""#))
        .collect::<Vec<_>>()
        .join(",");
    assert!(validate_processor_type(&serde_json::json!({
        "id": "proc.events",
        "title": {"default": "Too many events"},
        "eventTypes": serde_json::from_str::<Value>(&format!("[{many_events}]")).unwrap()
    }))
    .is_err());

    let valid = PluginManifest::from_json_str(&manifest(
        r#"{"id":"proc.ok","title":{"default":"Ok"},"timeoutMs":75}"#,
    ))
    .unwrap();
    assert!(validate_processor_type(&valid.processor_types[0]).is_ok());
    for invalid in ["0", "5001", "-1", "\"75\""] {
        let manifest = PluginManifest::from_json_str(&manifest(&format!(
            r#"{{"id":"proc.ok","title":{{"default":"Ok"}},"timeoutMs":{invalid}}}"#
        )))
        .unwrap();
        assert!(
            validate_processor_type(&manifest.processor_types[0]).is_err(),
            "timeout {invalid} should be rejected"
        );
    }
}

#[test]
fn validates_optional_action_timeout_bounds() {
    let manifest = |timeout: &str| {
        format!(
            r#"{{"schemaVersion":2,"id":"timeout","name":"Timeout","version":"1.0.0","runtime":"process","entry":"plugin.exe","actionTypes":[{{"id":"timeout.action","timeoutMs":{timeout}}}]}}"#
        )
    };

    assert!(PluginManifest::from_json_str(&manifest("120000")).is_ok());
    assert!(PluginManifest::from_json_str(&manifest("180000")).is_ok());
    for invalid in ["0", "180001", "-1", "\"120000\""] {
        assert!(
            PluginManifest::from_json_str(&manifest(invalid)).is_err(),
            "timeout {invalid} should be rejected"
        );
    }
}
