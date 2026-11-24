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
