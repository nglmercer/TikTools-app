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
fn reads_and_validates_domain_event_subscriptions() {
    let manifest = PluginManifest::from_json_str(
        r#"{"schemaVersion":2,"id":"gateway","name":"Gateway","version":"1.0.0","runtime":"process","entry":"gateway","capabilities":["events.subscribe"],"eventSubscriptions":["*","live.*","points.changed","plugin.*"]}"#,
    )
    .unwrap();
    assert_eq!(
        manifest.event_subscriptions,
        vec![
            "*".to_owned(),
            "live.*".to_owned(),
            "points.changed".to_owned(),
            "plugin.*".to_owned()
        ]
    );
    assert!(is_valid_event_subscription("*"));
    assert!(is_valid_event_subscription("live.*"));
    assert!(is_valid_event_subscription("points.changed"));
    assert!(!is_valid_event_subscription("live.**"));
    assert!(!is_valid_event_subscription("LIVE.*"));
    assert!(manifest
        .capabilities
        .iter()
        .any(|capability| capability == "events.subscribe"));
}

#[test]
fn malformed_domain_event_subscriptions_fail_manifest_parsing() {
    for subscriptions in [
        r#"["live.**"]"#,
        r#"["live.*.event"]"#,
        r#"["* "]"#,
        r#""not-an-array""#,
    ] {
        let manifest = format!(
            r#"{{"schemaVersion":2,"id":"gateway","name":"Gateway","version":"1.0.0","runtime":"process","entry":"gateway","eventSubscriptions":{subscriptions}}}"#
        );
        assert!(
            PluginManifest::from_json_str(&manifest).is_err(),
            "subscription {subscriptions} should be rejected"
        );
    }
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
fn reads_napi_vm_manifest_with_kebab_case_runtime() {
    let manifest = PluginManifest::from_json_str(
        r#"{"schemaVersion":3,"id":"example.plugin","name":"Example","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js","capabilities":[]}"#,
    )
    .unwrap();
    assert_eq!(manifest.runtime, PluginRuntimeKind::NapiVm);
    assert_eq!(manifest.entry, "dist/index.js");
    assert_eq!(manifest.trust, PluginTrust::Sandboxed);
    assert_eq!(manifest.security_model(), PluginSecurityModel::Sandboxed);
    assert_eq!(manifest.runtime.to_string(), "napi-vm");
    assert_eq!(
        PluginRuntimeKind::parse("napi-vm"),
        Some(PluginRuntimeKind::NapiVm)
    );
    let serialized = serde_json::to_value(manifest.runtime).unwrap();
    assert_eq!(serialized, Value::String("napi-vm".to_owned()));
    let round_trip: PluginRuntimeKind = serde_json::from_value(serialized).unwrap();
    assert_eq!(round_trip, PluginRuntimeKind::NapiVm);
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
fn tts_outputs_source_is_optional_but_bounded() {
    // A declared outputs source enables the host output selector...
    assert!(validate_plugin_page(&serde_json::json!({
        "id": "tts",
        "title": {"default": "TTS"},
        "sections": [{
            "kind": "tts",
            "actionType": "sonicboom.server.speak",
            "voicesFrom": "plugin-action-options:sonicboom.server.speak:voice",
            "outputsFrom": "plugin-action-options:sonicboom.server.set-output-device:device"
        }]
    }))
    .is_ok());
    // ...while malformed values fail discovery instead of reaching the UI.
    for outputs_from in [
        serde_json::json!("   "),
        serde_json::json!("x".repeat(257)),
        serde_json::json!(42),
    ] {
        assert!(
            validate_plugin_page(&serde_json::json!({
                "id": "tts",
                "title": {"default": "TTS"},
                "sections": [{
                    "kind": "tts",
                    "actionType": "sonicboom.server.speak",
                    "voicesFrom": "plugin-action-options:sonicboom.server.speak:voice",
                    "outputsFrom": outputs_from
                }]
            }))
            .is_err(),
            "outputsFrom {outputs_from} should be rejected"
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
    assert_eq!(manifest.icon.as_deref(), Some("voice"));
    assert!(manifest.tags.contains(&"tts".to_owned()));
    assert!(manifest.long_description.is_some());
    assert!(manifest.validate_compatibility().is_ok());
    assert!(validate_http_config(manifest.http.as_ref().unwrap()).is_ok());
    assert_eq!(manifest.action_types.len(), 2);
    for action in &manifest.action_types {
        assert!(validate_declarative_action(action).is_ok());
    }
    assert_eq!(manifest.templates.len(), 1);
    assert!(validate_plugin_template(&manifest.templates[0]).is_ok());
    assert_eq!(manifest.pages.len(), 2);
    for page in &manifest.pages {
        assert!(validate_plugin_page(page).is_ok());
    }
}

#[test]
fn sonicboom_output_action_declares_live_device_source() {
    // Pins the integration contract the TTS output selector relies on: the
    // switch action posts the chosen device to SonicBoom, and its option
    // source reads the live device list (plus server selection) back from
    // the same server. No hardcoded device names anywhere.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/sonicboom-server/plugin.json");
    if !path.exists() {
        // The crate is consumed outside the workspace; nothing to gate.
        return;
    }
    let input = std::fs::read_to_string(&path).unwrap();
    let manifest = PluginManifest::from_json_str(&input).unwrap();
    let action = manifest
        .action_types
        .iter()
        .find(|action| {
            action.get("id").and_then(Value::as_str) == Some("sonicboom.server.set-output-device")
        })
        .expect("set-output-device action declared");
    let device = action
        .get("fields")
        .and_then(Value::as_array)
        .and_then(|fields| {
            fields
                .iter()
                .find(|field| field.get("key").and_then(Value::as_str) == Some("device"))
        })
        .expect("device field declared");
    assert_eq!(
        device.get("optionsFrom").and_then(Value::as_str),
        Some("plugin-action-options:sonicboom.server.set-output-device:device")
    );
    let http = action.get("http").expect("declarative http block");
    assert_eq!(http.get("method").and_then(Value::as_str), Some("POST"));
    assert_eq!(
        http.get("path").and_then(Value::as_str),
        Some("/api/audio/output")
    );
    assert!(
        http.get("body")
            .and_then(Value::as_str)
            .is_some_and(|body| body.contains("{{ config.device }}")),
        "switch posts the chosen device verbatim"
    );
    let source = action
        .get("optionSources")
        .and_then(|sources| sources.get("device"))
        .expect("device option source declared");
    assert_eq!(
        source.get("path").and_then(Value::as_str),
        Some("/api/audio/devices")
    );
    assert_eq!(
        source.get("itemsPath").and_then(Value::as_str),
        Some("devices")
    );
    assert_eq!(source.get("valuePath").and_then(Value::as_str), Some("id"));
    assert_eq!(
        source.get("labelPath").and_then(Value::as_str),
        Some("name")
    );
}

#[test]
fn reads_display_metadata_for_plugin_cards() {
    let manifest = PluginManifest::from_json_str(
        r#"{
            "schemaVersion": 2,
            "id": "demo.cards",
            "name": "Demo",
            "version": "1.0.0",
            "description": "Short **pitch** with `code`.",
            "longDescription": "Long **pitch**.\n\n- first\n- second",
            "icon": "voice",
            "tags": ["TTS", "chat"],
            "runtime": "process",
            "entry": "demo"
        }"#,
    )
    .unwrap();
    assert_eq!(manifest.icon.as_deref(), Some("voice"));
    assert_eq!(manifest.tags, vec!["tts".to_owned(), "chat".to_owned()]);
    assert!(
        manifest
            .long_description
            .as_deref()
            .is_some_and(|long| long.contains("- second")),
        "long description survives verbatim for the markdown-lite renderer"
    );
}

#[test]
fn sanitizes_card_metadata_without_failing_discovery() {
    let manifest = PluginManifest::from_json_str(
        r#"{
            "schemaVersion": 2,
            "id": "demo.cards",
            "name": "Demo",
            "version": "1.0.0",
            "runtime": "process",
            "entry": "demo",
            "icon": "<img onerror=1>",
            "tags": ["ok", "  ", "has space", "ok", 42, "way-too-long-tag-name-over-32-chars!"]
        }"#,
    )
    .unwrap();
    assert_eq!(manifest.icon, None);
    assert_eq!(manifest.tags, vec!["ok".to_owned()]);

    let manifest = PluginManifest::from_json_str(
        r#"{
            "schemaVersion": 2,
            "id": "demo.cards",
            "name": "Demo",
            "version": "1.0.0",
            "runtime": "process",
            "entry": "demo",
            "tags": "not-an-array"
        }"#,
    )
    .unwrap();
    assert!(manifest.tags.is_empty());
}

#[test]
fn rejects_oversized_long_description() {
    let long = "x".repeat(17 * 1024);
    let manifest = format!(
        r#"{{"schemaVersion": 2, "id": "demo.cards", "name": "Demo", "version": "1.0.0", "runtime": "process", "entry": "demo", "longDescription": "{long}"}}"#
    );
    assert!(matches!(
        PluginManifest::from_json_str(&manifest),
        Err(ManifestError::InvalidField("longDescription"))
    ));
}

#[test]
fn parses_the_migrated_sonicboom_package_manifest() {
    // Pins the real isolated package to the parser: typed `ui` fragment,
    // legacy `pages` compatibility input, declarative HTTP actions, and
    // the process backend entry must all ingest together.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/sonicboom/plugin.json");
    let input = std::fs::read_to_string(&root).expect("package manifest");
    let manifest = PluginManifest::from_json_str(&input).expect("valid manifest");
    assert_eq!(manifest.id, "sonicboom.server");
    let ui = manifest.ui.as_ref().expect("ui fragment");
    assert_eq!(ui.mode, crate::ui::PluginUiMode::Webview);
    assert_eq!(ui.entry.as_deref(), Some("ui/dist/index.html"));
    assert_eq!(ui.pages.len(), 1);
    assert_eq!(ui.pages[0].id, "tts");
    assert!(!manifest.pages.is_empty(), "legacy pages kept");
    assert!(!manifest.action_types.is_empty(), "http actions kept");
}

#[test]
fn reads_autocomplete_section_raw_on_any_schema() {
    for schema in [2, 3] {
        let manifest = PluginManifest::from_json_str(&format!(
            r#"{{"schemaVersion": {schema}, "id": "textintel", "name": "TextIntel", "version": "0.1.0", "runtime": "process", "entry": "tiktools-textintel", "autocomplete": [{{"id": "stable-views", "prefixes": ["event.intel.comment."]}}]}}"#
        ))
        .unwrap();
        assert_eq!(manifest.autocomplete.len(), 1);
    }
    let manifest = PluginManifest::from_json_str(
        r#"{"schemaVersion": 2, "id": "plain", "name": "Plain", "version": "1.0.0", "runtime": "process", "entry": "plain"}"#,
    )
    .unwrap();
    assert!(manifest.autocomplete.is_empty());
}

#[test]
fn accepts_stable_prefixes_and_own_provider_fields() {
    assert!(validate_plugin_autocomplete(
        "textintel",
        &serde_json::json!({
            "id": "stable-views",
            "prefixes": ["event.intel.comment.", "event.intel.user"],
            "fields": [{
                "path": "event.intel.providers.textintel.comment.moderation.blocked",
                "kind": "boolean",
                "label": {"default": "Moderation blocked"},
                "hint": {"default": "True when blocked."}
            }],
            "triggers": ["tiktok.chat"]
        })
    )
    .is_ok());
    // The bare provider root claims the plugin's whole subtree.
    assert!(validate_plugin_autocomplete(
        "demo",
        &serde_json::json!({"id": "all", "prefixes": ["event.intel.providers.demo"]})
    )
    .is_ok());
}

#[test]
fn rejects_paths_outside_the_plugins_own_enrichment_namespace() {
    for path in [
        "event.data.comment",
        "event.user.uniqueId",
        "event.intel.processing.status",
        "event.intel.providers.other.comment.normalized",
        "event.intel.providers.textintelother.comment.x",
        "event.intel",
        "event.intel.commentary.drift",
    ] {
        assert!(
            validate_plugin_autocomplete(
                "textintel",
                &serde_json::json!({"id": "claim", "prefixes": [path]})
            )
            .is_err(),
            "must reject {path}"
        );
        assert!(
            validate_plugin_autocomplete(
                "textintel",
                &serde_json::json!({"id": "claim", "paths": [path]})
            )
            .is_err(),
            "must reject {path}"
        );
    }
}

#[test]
fn rejects_malformed_autocomplete_entries() {
    // Missing id, unknown id shape, and claim-less entries.
    assert!(validate_plugin_autocomplete(
        "demo",
        &serde_json::json!({"prefixes": ["event.intel.comment."]})
    )
    .is_err());
    assert!(validate_plugin_autocomplete(
        "demo",
        &serde_json::json!({"id": "has space", "prefixes": ["event.intel.comment."]})
    )
    .is_err());
    assert!(validate_plugin_autocomplete("demo", &serde_json::json!({"id": "empty"})).is_err());
    assert!(validate_plugin_autocomplete(
        "demo",
        &serde_json::json!({"id": "empty", "prefixes": []})
    )
    .is_err());
    // Fields need a path, a scalar kind, and a label.
    assert!(validate_plugin_autocomplete("demo", &serde_json::json!({"id": "fld", "fields": [{"path": "event.intel.providers.demo.x", "kind": "object", "label": {"default": "X"}}]})).is_err());
    assert!(validate_plugin_autocomplete("demo", &serde_json::json!({"id": "fld", "fields": [{"path": "event.intel.providers.demo.x", "kind": "boolean"}]})).is_err());
    assert!(validate_plugin_autocomplete("demo", &serde_json::json!({"id": "fld", "fields": [{"path": "event.intel.providers.demo.x.", "kind": "boolean", "label": {"default": "X"}}]})).is_err());
    // Triggers name dotted event types.
    assert!(validate_plugin_autocomplete("demo", &serde_json::json!({"id": "trg", "prefixes": ["event.intel.comment."], "triggers": ["CHAT"]})).is_err());
    // Caps are enforced.
    let many = vec!["event.intel.comment."; 17];
    assert!(validate_plugin_autocomplete(
        "demo",
        &serde_json::json!({"id": "cap", "prefixes": many})
    )
    .is_err());
}

#[test]
fn shipped_textintel_example_declares_valid_autocomplete() {
    // Pins the reference processor manifest to the validator: its stable
    // views and moderation verdict must parse and validate, or the editor
    // silently loses TextIntel suggestions on the next snapshot.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/textintel-process-plugin/plugin.json");
    let input = std::fs::read_to_string(&root).expect("textintel manifest");
    let manifest = PluginManifest::from_json_str(&input).expect("valid manifest");
    assert_eq!(manifest.id, "textintel");
    assert_eq!(manifest.autocomplete.len(), 2);
    for entry in &manifest.autocomplete {
        validate_plugin_autocomplete(&manifest.id, entry).expect("valid contribution");
    }
}

#[test]
fn shipped_hotkeys_example_declares_native_addon() {
    // Pins the napi-vm hotkeys example to the validator: its id, runtime,
    // trust, entry, and native declaration must parse, or the reference
    // native plugin silently stops loading on the next snapshot.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/hotkey-napi-plugin/plugin.json");
    let input = std::fs::read_to_string(&root).expect("hotkeys manifest");
    let manifest = PluginManifest::from_json_str(&input).expect("valid manifest");
    assert_eq!(manifest.id, "hotkeys");
    assert_eq!(manifest.runtime, PluginRuntimeKind::NapiVm);
    assert_eq!(manifest.trust, PluginTrust::Trusted);
    assert_eq!(manifest.native_addons.len(), 1);
    assert_eq!(manifest.native_addons[0].package, "rdev-node");
    assert_eq!(manifest.native_addons[0].root, "node_modules/rdev-node");
    assert_eq!(manifest.native_libs.len(), 1);
    assert_eq!(manifest.native_libs[0].package, "rdev-node");
    assert_eq!(manifest.native_libs[0].version, "1.0.5");
    assert_eq!(manifest.native_libs[0].provider, NativeLibProvider::Npm);
}

#[test]
fn parses_simple_native_addon_declarations() {
    let manifest = PluginManifest::from_json_str(
        r#"{
            "schemaVersion": 3,
            "id": "hotkeys",
            "name": "Hotkeys",
            "version": "1.0.0",
            "runtime": "napi-vm",
            "entry": "dist/index.js",
            "apiVersion": 1,
            "nativeAddons": [
                {
                    "package": "rdev-node",
                    "root": "node_modules/rdev-node"
                },
                {
                    "package": "@scope/other",
                    "root": "node_modules/@scope/other"
                }
            ]
        }"#,
    )
    .unwrap();
    assert_eq!(manifest.native_addons.len(), 2);
    assert_eq!(manifest.native_addons[0].package, "rdev-node");
    assert_eq!(manifest.native_addons[0].root, "node_modules/rdev-node");
    assert_eq!(manifest.native_addons[1].package, "@scope/other");
    assert_eq!(manifest.native_addons[1].root, "node_modules/@scope/other");
    // Absent declarations default to none: the guest stays in the pure VM.
    let plain = PluginManifest::from_json_str(
        r#"{"schemaVersion":3,"id":"xx","name":"X","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js"}"#,
    )
    .unwrap();
    assert!(plain.native_addons.is_empty());
}

#[test]
fn rejects_native_addon_traversal_and_bad_names() {
    for (package, root) in [
        ("../evil", "node_modules/evil"),
        ("evil", "../evil"),
        ("evil", "/absolute"),
        ("evil", "node_modules/../evil"),
        ("evil", ""),
        ("./evil", "node_modules/evil"),
        ("evil/sub", "node_modules/evil"),
        ("@scope", "node_modules/evil"),
        ("@scope/", "node_modules/evil"),
        ("has space", "node_modules/evil"),
        ("evil", "node_modules/evil/../sneaky"),
    ] {
        let input = serde_json::json!({
            "schemaVersion": 3,
            "id": "xx",
            "name": "X",
            "version": "1.0.0",
            "runtime": "napi-vm",
            "entry": "dist/index.js",
            "nativeAddons": [{
                "package": package,
                "root": root,
            }],
        });
        assert!(
            matches!(
                PluginManifest::from_value(input),
                Err(ManifestError::InvalidField("nativeAddons"))
            ),
            "package={package} root={root} should be rejected"
        );
    }
    // Scoped aliases and nested roots are fine.
    let scoped = PluginManifest::from_json_str(
        r#"{"schemaVersion":3,"id":"xx","name":"X","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js","nativeAddons":[{"package":"@scope/pkg","root":"node_modules/@scope/pkg"}]}"#,
    )
    .unwrap();
    assert_eq!(scoped.native_addons[0].package, "@scope/pkg");
}

#[test]
fn rejects_native_addon_shape_errors() {
    let manifest_with = |addons: serde_json::Value| {
        serde_json::json!({
            "schemaVersion": 3,
            "id": "xx",
            "name": "X",
            "version": "1.0.0",
            "runtime": "napi-vm",
            "entry": "dist/index.js",
            "nativeAddons": addons,
        })
    };
    // Non-array list, non-object entries, and missing or mistyped fields.
    for addons in [
        serde_json::json!("pkg"),
        serde_json::json!([42]),
        serde_json::json!([{"package": "pkg"}]),
        serde_json::json!([{"root": "node_modules/pkg"}]),
        serde_json::json!([{"package": 42, "root": "node_modules/pkg"}]),
        serde_json::json!([{"package": "pkg", "root": ["node_modules/pkg"]}]),
    ] {
        assert!(
            matches!(
                PluginManifest::from_value(manifest_with(addons)),
                Err(ManifestError::InvalidField("nativeAddons"))
            ),
            "malformed nativeAddons should be rejected"
        );
    }
    // Absurd package counts are bounded.
    let many: Vec<_> = (0..33)
        .map(|index| {
            serde_json::json!({"package": format!("pkg{index}"), "root": format!("node_modules/pkg{index}")})
        })
        .collect();
    assert!(matches!(
        PluginManifest::from_value(manifest_with(serde_json::Value::Array(many))),
        Err(ManifestError::InvalidField("nativeAddons"))
    ));
}

#[test]
fn rejects_duplicate_native_packages() {
    let duplicate = serde_json::json!({
        "schemaVersion": 3,
        "id": "xx",
        "name": "X",
        "version": "1.0.0",
        "runtime": "napi-vm",
        "entry": "dist/index.js",
        "nativeAddons": [
            {"package": "pkg", "root": "node_modules/a"},
            {"package": "pkg", "root": "node_modules/b"},
        ],
    });
    assert!(matches!(
        PluginManifest::from_value(duplicate),
        Err(ManifestError::InvalidField("nativeAddons"))
    ));
}

#[test]
fn parses_native_lib_declarations_with_npm_default() {
    let manifest = PluginManifest::from_json_str(
        r#"{
            "schemaVersion": 3,
            "id": "hotkeys",
            "name": "Hotkeys",
            "version": "1.0.0",
            "runtime": "napi-vm",
            "entry": "dist/index.js",
            "apiVersion": 1,
            "nativeLibs": [
                {"package": "rdev-node", "version": "1.0.1"},
                {
                    "package": "other",
                    "version": "2.0.0-beta.1",
                    "provider": "npm"
                },
                {
                    "package": "gh-only",
                    "version": "0.3.0",
                    "provider": "github",
                    "repo": "owner/gh-only",
                    "tag": "v0.3.0",
                    "binary": "gh-only"
                }
            ]
        }"#,
    )
    .unwrap();
    assert_eq!(manifest.native_libs.len(), 3);
    assert_eq!(manifest.native_libs[0].package, "rdev-node");
    assert_eq!(manifest.native_libs[0].version, "1.0.1");
    assert_eq!(manifest.native_libs[0].provider, NativeLibProvider::Npm);
    assert_eq!(manifest.native_libs[0].repo, None);
    assert_eq!(manifest.native_libs[1].provider, NativeLibProvider::Npm);
    assert_eq!(manifest.native_libs[2].provider, NativeLibProvider::Github);
    assert_eq!(
        manifest.native_libs[2].repo.as_deref(),
        Some("owner/gh-only")
    );
    assert_eq!(manifest.native_libs[2].tag.as_deref(), Some("v0.3.0"));
    assert_eq!(manifest.native_libs[2].binary.as_deref(), Some("gh-only"));
    // Absent declarations default to none: manually staged trees load.
    let plain = PluginManifest::from_json_str(
        r#"{"schemaVersion":3,"id":"xx","name":"X","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js"}"#,
    )
    .unwrap();
    assert!(plain.native_libs.is_empty());
}

#[test]
fn rejects_native_lib_shape_errors() {
    let manifest_with = |libs: serde_json::Value| {
        serde_json::json!({
            "schemaVersion": 3,
            "id": "xx",
            "name": "X",
            "version": "1.0.0",
            "runtime": "napi-vm",
            "entry": "dist/index.js",
            "nativeLibs": libs,
        })
    };
    // Non-array list, non-object entries, missing or mistyped fields,
    // version ranges, unknown providers, github entries missing keys,
    // npm entries carrying github keys, and duplicate packages.
    let bad = [
        serde_json::json!("pkg"),
        serde_json::json!([42]),
        serde_json::json!([{"package": "pkg"}]),
        serde_json::json!([{"version": "1.0.0"}]),
        serde_json::json!([{"package": 42, "version": "1.0.0"}]),
        serde_json::json!([{"package": "../evil", "version": "1.0.0"}]),
        serde_json::json!([{"package": "pkg", "version": "^1.0.0"}]),
        serde_json::json!([{"package": "pkg", "version": "~1.0"}]),
        serde_json::json!([{"package": "pkg", "version": "latest"}]),
        serde_json::json!([{"package": "pkg", "version": "1.0"}]),
        serde_json::json!([{"package": "pkg", "version": "1.0.0", "provider": "ftp"}]),
        serde_json::json!([{"package": "pkg", "version": "1.0.0", "provider": "github"}]),
        serde_json::json!([{"package": "pkg", "version": "1.0.0", "provider": "github", "repo": "o/p", "tag": "v1"}]),
        serde_json::json!([{"package": "pkg", "version": "1.0.0", "provider": "github", "repo": "o/p", "tag": "v1.0.0", "binary": "../evil"}]),
        serde_json::json!([{"package": "pkg", "version": "1.0.0", "provider": "github", "repo": "noslash", "tag": "v1.0.0", "binary": "b"}]),
        serde_json::json!([{"package": "pkg", "version": "1.0.0", "provider": "github", "repo": "o/p", "tag": "has space", "binary": "b"}]),
        serde_json::json!([{"package": "pkg", "version": "1.0.0", "repo": "o/p"}]),
        serde_json::json!([{"package": "pkg", "version": "1.0.0", "tag": "v1.0.0"}]),
        serde_json::json!([
            {"package": "pkg", "version": "1.0.0"},
            {"package": "pkg", "version": "2.0.0"},
        ]),
    ];
    for libs in bad {
        assert!(
            matches!(
                PluginManifest::from_value(manifest_with(libs)),
                Err(ManifestError::InvalidField("nativeLibs"))
            ),
            "malformed nativeLibs should be rejected"
        );
    }
    // Absurd library counts are bounded.
    let many: Vec<_> = (0..33)
        .map(|index| serde_json::json!({"package": format!("pkg{index}"), "version": "1.0.0"}))
        .collect();
    assert!(matches!(
        PluginManifest::from_value(manifest_with(serde_json::Value::Array(many))),
        Err(ManifestError::InvalidField("nativeLibs"))
    ));
    // Only napi-vm manifests may declare fetch sources.
    let foreign = serde_json::json!({
        "schemaVersion": 3,
        "id": "xx",
        "name": "X",
        "version": "1.0.0",
        "runtime": "process",
        "entry": "plugin",
        "nativeLibs": [{"package": "pkg", "version": "1.0.0"}],
    });
    assert!(matches!(
        PluginManifest::from_value(foreign),
        Err(ManifestError::InvalidField("nativeLibs"))
    ));
}

static NATIVE_SELECT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Scratch package root with `files` written into it. Unique per call so
/// selection tests can run in parallel.
fn native_package_fixture(files: &[&str]) -> std::path::PathBuf {
    let id = NATIVE_SELECT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    let root = std::env::temp_dir().join(format!(
        "tiktools-native-select-{}-{id}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    for file in files {
        let path = root.join(file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, b"fixture").unwrap();
    }
    root
}

#[test]
fn select_host_native_binary_picks_exact_host_file() {
    let target = current_napi_target();
    let platform = current_platform();
    let mut files = vec![
        format!("node-rdev.{target}.node"),
        // A platform that can never be this host.
        "node-rdev.fuchsia-arm64.node".to_owned(),
        // Same infix, wrong extension: never a binary.
        format!("node-rdev.{target}.node.txt"),
        format!("node-rdev.{target}.js"),
    ];
    // Foreign real targets: every platform but this host's.
    for (candidate, triple) in [
        ("win32", "node-rdev.win32-x64-msvc.node"),
        ("darwin", "node-rdev.darwin-arm64.node"),
        ("linux", "node-rdev.linux-x64-gnu.node"),
    ] {
        if platform != candidate {
            files.push(triple.to_owned());
        }
    }
    // The same-platform libc twin ships but must never be selected.
    let twin = if target.ends_with("-gnu") {
        Some(target.replace("-gnu", "-musl"))
    } else if target.ends_with("-musl") {
        Some(target.replace("-musl", "-gnu"))
    } else {
        None
    };
    if let Some(twin) = &twin {
        files.push(format!("node-rdev.{twin}.node"));
    }
    let root = native_package_fixture(&files.iter().map(String::as_str).collect::<Vec<_>>());
    // A host-named binary in a subdirectory is not top-level: ignored.
    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::write(
        root.join(format!("nested/node-rdev.{target}.node")),
        b"fixture",
    )
    .unwrap();

    let selected = select_host_native_binary(&root).unwrap();
    assert_eq!(
        selected,
        root.join(format!("node-rdev.{target}.node")),
        "must select exactly the host file"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn select_host_native_binary_rejects_missing_and_multiple() {
    let target = current_napi_target();

    // Only foreign binaries: nothing to authorize.
    let foreign = native_package_fixture(&["node-rdev.fuchsia-arm64.node", "index.js"]);
    let error = select_host_native_binary(&foreign).unwrap_err();
    assert!(
        matches!(error, NativeBinarySelectError::NoHostBinary { .. }),
        "{error}"
    );
    assert!(error.to_string().contains(&target), "{error}");

    // Two host-matching binaries: refusing to guess.
    let double = native_package_fixture(&[
        &format!("node-rdev.{target}.node"),
        &format!("other.{target}.node"),
    ]);
    let error = select_host_native_binary(&double).unwrap_err();
    assert!(
        matches!(error, NativeBinarySelectError::MultipleHostBinaries { .. }),
        "{error}"
    );

    // Not a directory at all.
    let missing = std::env::temp_dir().join(format!(
        "tiktools-native-select-missing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&missing);
    let error = select_host_native_binary(&missing).unwrap_err();
    assert!(
        matches!(error, NativeBinarySelectError::NotADirectory(_)),
        "{error}"
    );

    std::fs::remove_dir_all(&foreign).ok();
    std::fs::remove_dir_all(&double).ok();
}

#[test]
fn native_addons_rejected_on_non_napi_vm_runtimes() {
    for runtime in ["native", "process", "wasm", "declarative"] {
        let manifest = serde_json::json!({
            "schemaVersion": 3,
            "id": "xx",
            "name": "X",
            "version": "1.0.0",
            "runtime": runtime,
            "entry": "dist/index.js",
            "nativeAddons": [{
                "package": "pkg",
                "root": "node_modules/pkg",
            }],
        });
        assert!(
            matches!(
                PluginManifest::from_value(manifest),
                Err(ManifestError::InvalidField("nativeAddons"))
            ),
            "runtime {runtime} must reject nativeAddons"
        );
    }
}

#[test]
fn host_target_reports_real_libc_and_napi_spelling() {
    // Compile-time libc fact: musl builds report musl, all other Linux
    // builds report gnu.
    assert_eq!(is_musl(), cfg!(target_env = "musl"));
    if current_platform() == "linux" {
        assert_eq!(
            current_target(),
            format!(
                "linux-{}-{}",
                current_arch(),
                if is_musl() { "musl" } else { "gnu" }
            )
        );
    }
    // The napi-rs spelling matches artifact filenames (`<binary>.<target>.node`),
    // including un-suffixed darwin triples.
    let napi = current_napi_target();
    assert!(!napi.is_empty(), "napi target must not be empty");
    if current_platform() == "darwin" {
        assert_eq!(napi, format!("darwin-{}", current_arch()));
    } else {
        assert_eq!(napi, current_target());
    }
}
