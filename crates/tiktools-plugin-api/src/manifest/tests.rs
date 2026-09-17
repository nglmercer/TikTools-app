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

#[test]
fn schema_v2_ignores_declarative_keys() {
    // A v2 manifest carrying v3 keys keeps historical behavior: the keys are
    // dropped, never validated, so old packages parse exactly as before.
    let manifest = PluginManifest::from_json_str(
        r#"{
            "schemaVersion": 2,
            "id": "legacy.http",
            "name": "Legacy",
            "version": "1.0.0",
            "runtime": "process",
            "entry": "legacy",
            "http": {"baseUrl": "not a url", "auth": {"type": "bogus"}},
            "templates": [{"id": "!!!"}],
            "pages": [{"kind": "script"}],
            "actionTypes": [{"id": "legacy.action", "http": {"path": "nope"}}]
        }"#,
    )
    .unwrap();
    assert!(manifest.http.is_none());
    assert!(manifest.templates.is_empty());
    assert!(manifest.pages.is_empty());
    assert_eq!(manifest.action_types.len(), 1);
}

#[test]
fn reads_declarative_manifest_without_entry() {
    let manifest = PluginManifest::from_json_str(
        r#"{
            "schemaVersion": 3,
            "id": "sonicboom.server",
            "name": "SonicBoom Server",
            "version": "1.0.0",
            "runtime": "declarative",
            "capabilities": ["http.request"],
            "http": {
                "baseUrl": "{{ settings.serverUrl }}",
                "auth": {"type": "bearer", "tokenSetting": "apiToken"},
                "health": {"path": "/ready"}
            },
            "actionTypes": [{
                "id": "sonicboom.server.speak",
                "http": {"method": "POST", "path": "/api/tts/play"},
                "optionSources": {"voice": {"path": "/v1/voices"}}
            }],
            "templates": [{
                "id": "chat-tts",
                "title": {"default": "Chat to TTS"},
                "eventType": "tiktok.chat",
                "requiredNodeTypes": ["trigger.event", "action.http"],
                "workflow": {"nodes": [{"type": "trigger.event"}, {"type": "action.http"}]}
            }],
            "pages": [{
                "id": "connection",
                "title": {"default": "Connection"},
                "sections": [{"kind": "connection"}]
            }]
        }"#,
    )
    .unwrap();
    assert_eq!(manifest.runtime, PluginRuntimeKind::Declarative);
    assert!(manifest.entry.is_empty());
    assert_eq!(manifest.trust, PluginTrust::Untrusted);
    assert!(manifest.http.is_some());
    assert_eq!(manifest.templates.len(), 1);
    assert_eq!(manifest.pages.len(), 1);
    assert!(manifest.validate_compatibility().is_ok());
    assert!(validate_plugin_template(&manifest.templates[0]).is_ok());
    assert!(validate_plugin_page(&manifest.pages[0]).is_ok());
}

#[test]
fn rejects_malformed_declarative_blocks() {
    // Bad base URL scheme.
    assert!(PluginManifest::from_json_str(
        r#"{"schemaVersion":3,"id":"bad","name":"Bad","version":"1.0.0","runtime":"declarative","http":{"baseUrl":"ftp://x"}}"#,
    )
    .is_err());
    // Unknown auth type.
    assert!(PluginManifest::from_json_str(
        r#"{"schemaVersion":3,"id":"bad","name":"Bad","version":"1.0.0","runtime":"declarative","http":{"baseUrl":"http://localhost:3000","auth":{"type":"oauth"}}}"#,
    )
    .is_err());
    // Absolute action path.
    assert!(PluginManifest::from_json_str(
        r#"{"schemaVersion":3,"id":"bad","name":"Bad","version":"1.0.0","runtime":"declarative","actionTypes":[{"id":"bad.action","http":{"path":"https://evil.example/x"}}]}"#,
    )
    .is_err());
    // Malformed token provisioning descriptor.
    assert!(PluginManifest::from_json_str(
        r#"{"schemaVersion":3,"id":"bad","name":"Bad","version":"1.0.0","runtime":"declarative","http":{"baseUrl":"http://localhost:3000","tokenProvisioning":{"strategy":""}}}"#,
    )
    .is_err());
    // Unknown provisioning strategies pass validation; the host just
    // offers no provisioning button for them.
    assert!(PluginManifest::from_json_str(
        r#"{"schemaVersion":3,"id":"ok","name":"Ok","version":"1.0.0","runtime":"declarative","http":{"baseUrl":"http://localhost:3000","tokenProvisioning":{"strategy":"future-v2"}}}"#,
    )
    .is_ok());
    // Template without workflow nodes.
    assert!(validate_plugin_template(&serde_json::json!({
        "id": "empty",
        "title": {"default": "Empty"},
        "eventType": "tiktok.chat",
        "requiredNodeTypes": ["trigger.event"],
        "workflow": {"nodes": []}
    }))
    .is_err());
    // Unknown page section kinds never reach the WebView.
    for kind in ["html", "script", "component", "iframe"] {
        assert!(
            validate_plugin_page(&serde_json::json!({
                "id": "page",
                "title": {"default": "Page"},
                "sections": [{"kind": kind}]
            }))
            .is_err(),
            "section kind {kind} should be rejected"
        );
    }
}

#[test]
fn tts_page_section_needs_an_action_and_a_voice_source() {
    // Host-owned TTS panel binding: action executed for real speech plus the
    // manifest-declared voice source feeding its selectors.
    assert!(validate_plugin_page(&serde_json::json!({
        "id": "tts",
        "title": {"default": "TTS"},
        "sections": [{
            "kind": "tts",
            "actionType": "sonicboom.server.speak",
            "voicesFrom": "plugin-action-options:sonicboom.server.speak:voice"
        }]
    }))
    .is_ok());
    for section in [
        serde_json::json!({"kind": "tts"}),
        serde_json::json!({"kind": "tts", "actionType": "  ", "voicesFrom": "plugin-action-options:a:b"}),
        serde_json::json!({"kind": "tts", "actionType": "sonicboom.server.speak"}),
        serde_json::json!({"kind": "tts", "actionType": "sonicboom.server.speak", "voicesFrom": "   "}),
    ] {
        assert!(
            validate_plugin_page(&serde_json::json!({
                "id": "tts",
                "title": {"default": "TTS"},
                "sections": [section]
            }))
            .is_err(),
            "tts section {section} should be rejected"
        );
    }
}

#[test]
fn shipped_sonicboom_example_parses() {
    // Conformance gate for examples/sonicboom-server/plugin.json: the
    // declarative example the host ships must always parse and validate.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/sonicboom-server/plugin.json");
    if !path.exists() {
        // The crate is consumed outside the workspace; nothing to gate.
        return;
    }
    let input = std::fs::read_to_string(&path).unwrap();
    let manifest = PluginManifest::from_json_str(&input).unwrap();
    assert_eq!(manifest.id, "sonicboom.server");
    assert_eq!(manifest.runtime, PluginRuntimeKind::Declarative);
    assert!(manifest.validate_compatibility().is_ok());
    assert!(validate_http_config(manifest.http.as_ref().unwrap()).is_ok());
    assert_eq!(manifest.action_types.len(), 1);
    assert!(validate_declarative_action(&manifest.action_types[0]).is_ok());
    assert_eq!(manifest.templates.len(), 1);
    assert!(validate_plugin_template(&manifest.templates[0]).is_ok());
    assert_eq!(manifest.pages.len(), 2);
    for page in &manifest.pages {
        assert!(validate_plugin_page(page).is_ok());
    }
}
