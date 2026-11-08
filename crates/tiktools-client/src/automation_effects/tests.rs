use super::*;
use serde_json::json;
fn row(id: &str) -> Value {
    json!({"effectId":id,"kind":"http","state":"unknown","acceptedAt":1,"updatedAt":2,"outcomeExpired":false})
}
fn response(result: Value) -> CallResult<Value> {
    CallResult {
        result,
        meta: Some(crate::ResponseMetadata {
            resource_versions: Vec::new(),
            command: None,
            host_instance_id: None,
        }),
    }
}
#[test]
fn get_requires_exact_metadata_only_shapes_and_requested_identity() {
    let id = "a".repeat(64);
    let good = response(row(&id));
    assert_eq!(
        get(&good, &id).unwrap().unwrap().state,
        EffectState::Unknown
    );
    assert!(get(&response(Value::Null), &id).unwrap().is_none());
    assert!(get(&good, &"b".repeat(64)).is_err());
    for field in [
        "effectId",
        "kind",
        "state",
        "acceptedAt",
        "updatedAt",
        "outcomeExpired",
    ] {
        let mut changed = good.clone();
        changed.result.as_object_mut().unwrap().remove(field);
        assert!(get(&changed, &id).is_err(), "{field}");
    }
    for (field, value) in [
        ("source", json!("secret")),
        ("requestDigest", json!("b".repeat(64))),
        ("state", json!(["unknown"])),
        ("kind", json!("arbitrary-plugin")),
        ("effectId", json!("A".repeat(64))),
        ("acceptedAt", json!(-1)),
        ("updatedAt", json!(0)),
        ("outcomeExpired", Value::Null),
    ] {
        let mut changed = good.clone();
        changed.result[field] = value;
        assert!(get(&changed, &id).is_err(), "{field}");
    }
    let mut expired = good.clone();
    expired.result["outcomeExpired"] = json!(true);
    expired.result["state"] = json!("succeeded");
    assert!(get(&expired, &id).is_err());
    let mut missing = good.clone();
    missing.meta = None;
    assert!(get(&missing, &id).is_err());
    let mut forged = good;
    forged.meta.as_mut().unwrap().host_instance_id = Some("forged-authority".into());
    assert!(get(&forged, &id).is_err());
}
#[test]
fn history_enforces_cursor_binding_order_uniqueness_and_bounded_results() {
    let a = "a".repeat(64);
    let b = "b".repeat(64);
    let c = "c".repeat(64);
    let params = EffectHistoryParams {
        before: Some(c),
        limit: 2,
    };
    let good = response(json!({"items":[row(&b),row(&a)],"nextBefore":a}));
    assert_eq!(history(&good, &params).unwrap().items.len(), 2);
    for result in [
        json!({"items":[row(&a),row(&b)],"nextBefore":null}),
        json!({"items":[row(&a),row(&a)],"nextBefore":null}),
        json!({"items":[row(&b)],"nextBefore":b}),
        json!({"items":[row(&b),row(&a)],"nextBefore":b}),
        json!({"items":[row(&b),row(&a),row(&a)],"nextBefore":null}),
        json!({"items":[]}),
        json!({"items":[],"nextBefore":null,"retry":true}),
    ] {
        assert!(history(&response(result), &params).is_err());
    }
    assert!(history(&response(json!({"items":[],"nextBefore":null})), &params).is_ok());
}
#[test]
fn dry_run_validators_match_closed_host_params_and_semantic_bounds() {
    for (method, params) in [
        (GET, json!({"effectId":"a".repeat(64)})),
        (HISTORY, json!({"before":null,"limit":100})),
    ] {
        assert!(crate::validate_params(method, &params).is_ok());
        let mut extra = params;
        extra["replay"] = json!(true);
        assert_eq!(
            crate::validate_params(method, &extra).unwrap_err().code,
            "invalid_params"
        );
    }
    for (method, params) in [
        (GET, json!({"effectId":"chosen-command-id"})),
        (HISTORY, json!({"before":null,"limit":0})),
        (HISTORY, json!({"before":null,"limit":101})),
        (HISTORY, json!({"before":"A".repeat(64),"limit":10})),
    ] {
        assert_eq!(
            crate::validate_params(method, &params).unwrap_err().code,
            "invalid_params"
        );
    }
    assert_eq!(
        crate::validate_params("automation.effects.retry", &json!({}))
            .unwrap_err()
            .code,
        "method_not_found"
    );
}

#[test]
fn metadata_receipts_accept_only_the_closed_scoped_flow_effect_kinds() {
    let id = "a".repeat(64);
    for (name, kind) in [("emit", EffectKind::Emit), ("script", EffectKind::Script)] {
        let mut value = row(&id);
        value["kind"] = json!(name);
        assert_eq!(get(&response(value), &id).unwrap().unwrap().kind, kind);
    }
}
