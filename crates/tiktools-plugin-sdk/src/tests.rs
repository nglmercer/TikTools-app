use std::collections::BTreeMap;

use super::*;
use serde_json::Value;
use tiktools_plugin_api::{CapabilitySet, PermissionSet, PluginRequest, METHOD_CALL};

use crate::process::handle_process_request;

#[derive(Default)]
struct TestPlugin;

impl Plugin for TestPlugin {
    fn action(
        &mut self,
        _context: &PluginContext,
        _call: ActionCall,
    ) -> PluginResult<ActionResult> {
        Ok(ActionResult::summary("action handled"))
    }
}

fn test_context() -> PluginContext {
    PluginContext::new(
        PluginIdentity::new("test.plugin", "1.0.0"),
        CapabilitySet::default(),
        PermissionSet::default(),
    )
}

#[test]
fn typed_plugin_call_preserves_legacy_wire_shape() {
    let call = PluginCall::action(
        serde_json::json!({"typeId": "demo.action", "config": {"value": 1}}),
        serde_json::json!({"type": "tiktok.chat"}),
    );
    let value = serde_json::to_value(call).unwrap();
    assert_eq!(value["type"], "action");
    assert_eq!(value["action"]["typeId"], "demo.action");
}

#[test]
fn event_call_uses_only_the_stable_topic_data_envelope() {
    let call = PluginCall::event(tiktools_plugin_api::DomainEventEnvelope::new(
        "live.event",
        serde_json::json!({"eventType": "tiktok.chat"}),
    ));
    assert_eq!(
        serde_json::to_value(&call).unwrap(),
        serde_json::json!({
            "type": "event",
            "event": {"topic": "live.event", "data": {"eventType": "tiktok.chat"}}
        })
    );
    assert_eq!(call.clone().into_event().unwrap().topic, "live.event");
    assert_eq!(
        dispatch_plugin_call(&mut TestPlugin, &test_context(), call).unwrap(),
        Value::Null
    );
}

#[test]
fn compatibility_decoder_maps_legacy_intents_once() {
    let result = decode_plugin_result(serde_json::json!({
        "summary": "done",
        "logs": ["one"],
        "emit": [{"type": "demo.event", "data": {"ok": true}}],
        "playAudio": {"fileRef": {"path": "/tmp/alert.wav"}, "volume": 0.5}
    }))
    .unwrap();
    assert_eq!(result.summary.as_deref(), Some("done"));
    assert_eq!(result.logs, vec!["one"]);
    assert_eq!(result.intents.len(), 2);
    assert!(matches!(result.intents[0], HostIntent::Emit(_)));
    assert!(matches!(result.intents[1], HostIntent::AudioPlay(_)));
}

#[test]
fn compatibility_decoder_rejects_malformed_legacy_fields() {
    for (field, value) in [
        ("summary", serde_json::json!(42)),
        ("logs", serde_json::json!(["ok", 42])),
        ("emit", serde_json::json!([{"data": {}}])),
        ("playAudio", serde_json::json!([{"volume": "loud"}])),
        ("events", serde_json::json!([{"type": "missing-data"}])),
    ] {
        let error = decode_plugin_result(serde_json::json!({field: value})).unwrap_err();
        assert!(error.to_string().contains(field), "{field}: {error}");
    }
}

#[test]
fn audio_intent_serializes_with_typed_file_reference() {
    let result = ActionResult::default().intent(HostIntent::audio_play(
        AudioPlayIntent::from_path("/tmp/alert.wav"),
    ));
    let value = serde_json::to_value(PluginCallResult::from(result)).unwrap();
    assert_eq!(value["intents"][0]["type"], "audio-play");
    assert_eq!(
        value["intents"][0]["data"]["fileRef"]["path"],
        "/tmp/alert.wav"
    );
}

#[test]
fn process_path_helpers_reject_missing_and_empty_values() {
    assert!(process::path_from_os("TIKTOOLS_PLUGIN_DATA_DIR", None).is_err());
    assert!(process::path_from_os(
        "TIKTOOLS_PLUGIN_STORAGE_FILE",
        Some(std::ffi::OsString::new())
    )
    .is_err());
}

#[test]
fn process_request_dispatches_raw_typed_call_inside_the_wire_envelope() {
    let call = PluginCall::action(
        serde_json::json!({"typeId": "demo.action"}),
        serde_json::json!({"type": "demo.event"}),
    );
    let request = PluginRequest::new(
        METHOD_CALL,
        METHOD_CALL,
        serde_json::to_value(call).unwrap(),
    );
    let response = handle_process_request(&mut TestPlugin, &test_context(), request);
    assert!(response.ok);
    assert_eq!(response.id, METHOD_CALL);
    let result: PluginCallResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.summary.as_deref(), Some("action handled"));
}

#[test]
fn process_request_rejects_bad_protocol_method_and_call() {
    let context = test_context();
    let mut bad_version = PluginRequest::new(
        "version",
        METHOD_CALL,
        serde_json::json!({
            "type": "poll"
        }),
    );
    bad_version.protocol_version += 1;
    assert!(!handle_process_request(&mut TestPlugin, &context, bad_version).ok);

    let bad_method = PluginRequest::new(
        "method",
        "other",
        serde_json::json!({
            "type": "poll"
        }),
    );
    assert!(!handle_process_request(&mut TestPlugin, &context, bad_method).ok);

    let bad_call = PluginRequest::new(
        "call",
        METHOD_CALL,
        serde_json::json!({
            "type": "unknown"
        }),
    );
    assert!(!handle_process_request(&mut TestPlugin, &context, bad_call).ok);
}

#[test]
fn enrich_call_serializes_additively_without_changing_action_and_poll() {
    // Existing wire shapes are byte-identical to the pre-enrich protocol.
    let action = PluginCall::action(serde_json::json!({}), serde_json::json!({}));
    assert_eq!(
        serde_json::to_value(&action).unwrap(),
        serde_json::json!({"type": "action", "action": {}, "event": {}})
    );
    let poll = PluginCall::Poll;
    assert_eq!(
        serde_json::to_value(&poll).unwrap(),
        serde_json::json!({"type": "poll"})
    );
    // The new variant round-trips through the same envelope.
    let enrich = PluginCall::enrich(EventEnrichmentRequest::new(
        "textintel.analyze",
        serde_json::json!({"type": "tiktok.chat"}),
    ));
    let value = serde_json::to_value(&enrich).unwrap();
    assert_eq!(value["type"], "enrich");
    assert_eq!(value["request"]["processorId"], "textintel.analyze");
    assert_eq!(serde_json::from_value::<PluginCall>(value).unwrap(), enrich);
    assert!(enrich.into_action().is_none());
    assert!(poll.into_enrich().is_none());
}

#[test]
fn default_enrich_is_a_noop_and_dispatch_routes_it() {
    // Existing plugins that never implemented `enrich` keep working.
    let context = test_context();
    let request = EventEnrichmentRequest::new("demo.enrich", serde_json::json!({}));
    let result = TestPlugin.enrich(&context, request.clone()).unwrap();
    assert_eq!(result, EventEnrichmentResult::default());

    let value =
        dispatch_plugin_call(&mut TestPlugin, &context, PluginCall::enrich(request)).unwrap();
    assert_eq!(
        decode_enrichment_result(value).unwrap(),
        EventEnrichmentResult::default()
    );
}

#[derive(Default)]
struct EnrichPlugin;

impl Plugin for EnrichPlugin {
    fn enrich(
        &mut self,
        _context: &PluginContext,
        request: EventEnrichmentRequest,
    ) -> PluginResult<EventEnrichmentResult> {
        let comment = request
            .event
            .pointer("/data/comment")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let mut result = EventEnrichmentResult::default();
        result.annotations.insert(
            "comment".to_owned(),
            serde_json::json!({"normalized": comment.trim()}),
        );
        result.views.insert(
            "tts".to_owned(),
            TextView::new(comment.trim(), "normalized").language("es", 0.9),
        );
        Ok(result)
    }
}

#[test]
fn enrich_dispatch_flows_through_the_process_envelope() {
    let call = PluginCall::enrich(EventEnrichmentRequest::new(
        "demo.enrich",
        serde_json::json!({"type": "tiktok.chat", "data": {"comment": "  Hola  "}}),
    ));
    let request = PluginRequest::new("enrich-1", METHOD_CALL, serde_json::to_value(call).unwrap());
    let response = handle_process_request(&mut EnrichPlugin, &test_context(), request);
    assert!(response.ok);
    let result = decode_enrichment_result(response.result.unwrap()).unwrap();
    assert_eq!(
        result.annotations["comment"]["normalized"],
        serde_json::json!("Hola")
    );
    assert_eq!(result.views["tts"].text, "Hola");
    assert_eq!(result.views["tts"].language.as_deref(), Some("es"));
}

#[test]
fn enrichment_decoder_rejects_side_effects_and_oversized_results() {
    for key in [
        "emit",
        "events",
        "intents",
        "playAudio",
        "points",
        "http",
        "storage",
    ] {
        let error = decode_enrichment_result(serde_json::json!({key: {}})).unwrap_err();
        assert!(
            error.to_string().contains("must not contain"),
            "{key}: {error}"
        );
    }
    assert!(decode_enrichment_result(serde_json::json!([])).is_err());
    assert!(decode_enrichment_result(serde_json::json!({"annotations": []})).is_err());
    assert!(decode_enrichment_result(serde_json::json!({"views": []})).is_err());
    assert!(decode_enrichment_result(serde_json::json!({"logs": [42]})).is_err());
    // Oversized annotation payloads are rejected, never truncated.
    let big = "x".repeat(MAX_ENRICHMENT_ANNOTATION_BYTES + 1);
    assert!(decode_enrichment_result(serde_json::json!({
        "annotations": {"comment": {"normalized": big}}
    }))
    .is_err());
    // View bounds are enforced.
    assert!(decode_enrichment_result(serde_json::json!({
        "views": {"tts": {"text": "hi"}}
    }))
    .is_err());
    assert!(decode_enrichment_result(serde_json::json!({
        "views": {"tts": {"text": "hi", "source": "raw", "confidence": 2.0}}
    }))
    .is_err());
    let long_text = "x".repeat(MAX_ENRICHMENT_VIEW_TEXT_CHARS + 1);
    assert!(decode_enrichment_result(serde_json::json!({
        "views": {"tts": {"text": long_text, "source": "raw"}}
    }))
    .is_err());
    // Unknown non-side-effect keys stay forward-compatible.
    assert!(decode_enrichment_result(serde_json::json!({
        "annotations": {},
        "futureField": {"nested": true}
    }))
    .is_ok());
}

#[test]
fn views_carry_pronunciation_and_speak_policy_with_bounds() {
    let decoded = decode_enrichment_result(serde_json::json!({
        "views": {
            "nickname": {
                "text": "José",
                "language": "es",
                "confidence": 0.9,
                "source": "spoken",
                "pronunciation": {
                    "ipa": "xoˈse",
                    "language": "es",
                    "dialect": "es-ES",
                    "confidence": 0.8,
                },
            },
            "skipped": {"text": "", "source": "policy", "speak": false, "reason": "emoji-only"},
        },
    }))
    .unwrap();
    let nickname = &decoded.views["nickname"];
    assert!(nickname.speak);
    assert_eq!(
        nickname
            .pronunciation
            .as_ref()
            .and_then(|pronunciation| pronunciation.ipa.as_deref()),
        Some("xoˈse")
    );
    // Selection confidence and pronunciation confidence stay separate.
    assert_eq!(nickname.confidence, Some(0.9));
    assert_eq!(
        nickname
            .pronunciation
            .as_ref()
            .and_then(|pronunciation| pronunciation.confidence),
        Some(0.8)
    );
    let skipped = &decoded.views["skipped"];
    assert!(!skipped.speak);
    assert_eq!(skipped.reason.as_deref(), Some("emoji-only"));

    // Missing speak defaults to true for older producers.
    let legacy = decode_enrichment_result(serde_json::json!({
        "views": {"tts": {"text": "hi", "source": "raw"}},
    }))
    .unwrap();
    assert!(legacy.views["tts"].speak);

    // Pronunciation bounds are enforced.
    assert!(decode_enrichment_result(serde_json::json!({
        "views": {"tts": {"text": "hi", "source": "raw",
            "pronunciation": {"confidence": 2.0}}},
    }))
    .is_err());
    assert!(decode_enrichment_result(serde_json::json!({
        "views": {"tts": {"text": "hi", "source": "raw",
            "pronunciation": {"dialect": ""}}},
    }))
    .is_err());
}

#[test]
fn request_inputs_resolve_roles_with_legacy_pointer_fallback() {
    let event = serde_json::json!({
        "data": {"comment": "legacy"},
        "user": {"nickname": "Legacy"},
    });
    let legacy = EventEnrichmentRequest::new("demo.analyze", event.clone());
    assert_eq!(
        legacy
            .input("message", "/data/comment")
            .and_then(Value::as_str),
        Some("legacy")
    );
    let mut inputs = BTreeMap::new();
    inputs.insert("message".to_owned(), serde_json::json!("resolved"));
    let resolved = EventEnrichmentRequest::new("demo.analyze", event).inputs(inputs);
    assert_eq!(
        resolved
            .input("message", "/data/comment")
            .and_then(Value::as_str),
        Some("resolved")
    );
    // A host that resolved inputs owns them: unresolvable roles stay
    // absent instead of silently reading other pointers.
    assert!(resolved.input("display-name", "/user/nickname").is_none());
    // Inputs round-trip through the wire envelope.
    let value = serde_json::to_value(PluginCall::enrich(resolved.clone())).unwrap();
    assert_eq!(value["request"]["inputs"]["message"], "resolved");
    assert_eq!(
        serde_json::from_value::<PluginCall>(value)
            .unwrap()
            .into_enrich()
            .unwrap(),
        resolved
    );
}
