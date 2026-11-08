use super::*;
use serde_json::json;

const SOURCE: &str = "private-owner-definition-source";

fn record() -> Value {
    json!({"id":"draft","name":"Saved action","enabled":false,"typeId":"core.log","config":{"message":SOURCE}})
}
fn plain(result: Value) -> CallResult<Value> {
    CallResult {
        result,
        meta: Some(serde_json::from_value(json!({"resourceVersions":[]})).unwrap()),
    }
}
fn fixture(operation: &str) -> (CallResult<Value>, OwnerDefinitionSubmit) {
    let mut body = record();
    let mut params = json!({"operation":operation,"kind":"action","expectedRevision":"2","hostId":"host","hostInstanceId":"instance"});
    if operation == "create" {
        params["record"] = body.clone();
    } else {
        params["id"] = json!("draft");
        if operation == "update" {
            params["record"] = body.clone();
        }
    }
    let input_digest = if matches!(operation, "create" | "update") {
        Some(record_digest(&body).unwrap())
    } else {
        None
    };
    if matches!(operation, "enable" | "disable") {
        body["enabled"] = json!(operation == "enable");
    } else if operation == "delete" {
        body = json!({"id":"draft","deleted":true});
    }
    let output_digest = if operation == "delete" {
        None
    } else {
        Some(record_digest(&body).unwrap())
    };
    let command_id = format!("owner-definition-{}", "a".repeat(32));
    let intent = json!({"commandId":command_id,"operation":operation,"kind":"action","id":"draft","expectedRevision":"2","hostInstanceId":"instance","recordDigest":input_digest,"createdAt":30});
    let receipt = json!({"revision":"3","recordDigest":output_digest,"enabled":if operation == "delete" { Value::Null } else { body["enabled"].clone() },"deleted":if operation == "delete" { json!(true) } else { Value::Null },"affectedResources":1});
    let entry = json!({"intent":intent,"outcome":"succeeded","receipt":receipt,"errorCode":null,"recordedAt":20});
    let state =
        json!({"hostId":"host","hostInstanceId":"instance","pending":null,"recent":[entry]});
    let ack = json!({"value":body,"resourceVersions":[{"kind":"automation","scope":"action","id":"draft","revision":"3"}],"command":{"commandId":command_id,"state":"succeeded","resource":{"kind":"automation","scope":"action","id":"draft"},"committedAt":40},"hostInstanceId":"instance"});
    (
        plain(json!({"state":state,"acknowledgement":ack})),
        serde_json::from_value(params).unwrap(),
    )
}
fn state_fixture() -> CallResult<Value> {
    let (response, _) = fixture("create");
    plain(response.result["state"].clone())
}

#[test]
fn fresh_acknowledgements_bind_each_operation_and_tombstone_recreation() {
    for operation in ["create", "update", "delete", "enable", "disable"] {
        let (response, params) = fixture(operation);
        let parsed = validate_submission(&response, &params).unwrap();
        assert!(parsed.acknowledgement.is_some(), "{operation}");
        assert_eq!(
            parsed.state.recent[0].receipt.as_ref().unwrap().revision,
            "3"
        );
    }
    let (mut response, params) = fixture("delete");
    response.result["state"]["recent"][0]["receipt"]["revision"] = json!("2");
    response.result["state"]["recent"][0]["receipt"]["deleted"] = json!(false);
    response.result["acknowledgement"]["value"]["deleted"] = json!(false);
    response.result["acknowledgement"]["resourceVersions"][0]["revision"] = json!("2");
    validate_submission(&response, &params).unwrap();
}

#[test]
fn revision_zero_preserves_missing_delete_and_original_update_rejections() {
    let (mut response, params) = fixture("delete");
    let mut params = serde_json::to_value(params).unwrap();
    params["expectedRevision"] = json!("0");
    response.result["state"]["recent"][0]["intent"]["expectedRevision"] = json!("0");
    response.result["state"]["recent"][0]["receipt"]["revision"] = json!("0");
    response.result["state"]["recent"][0]["receipt"]["deleted"] = json!(false);
    response.result["acknowledgement"]["value"]["deleted"] = json!(false);
    response.result["acknowledgement"]["resourceVersions"][0]["revision"] = json!("0");
    validate_submission(&response, &serde_json::from_value(params).unwrap()).unwrap();

    for code in ["not_found", "revision_conflict"] {
        let (mut response, params) = fixture("update");
        let mut params = serde_json::to_value(params).unwrap();
        params["expectedRevision"] = json!("0");
        response.result["state"]["recent"][0]["intent"]["expectedRevision"] = json!("0");
        response.result["state"]["recent"][0]["outcome"] = json!("rejected");
        response.result["state"]["recent"][0]["receipt"] = Value::Null;
        response.result["state"]["recent"][0]["errorCode"] = json!(code);
        response.result["acknowledgement"] = Value::Null;
        let parsed =
            validate_submission(&response, &serde_json::from_value(params).unwrap()).unwrap();
        assert_eq!(parsed.state.recent[0].error_code.as_deref(), Some(code));
        assert!(parsed.acknowledgement.is_none());
    }
}

#[test]
fn historical_state_keeps_original_instance_and_clock_rollback_without_returning_source() {
    let mut response = state_fixture();
    response.result["hostInstanceId"] = json!("new-instance");
    let state = validate_state(&response).unwrap();
    assert_eq!(state.recent[0].intent.host_instance_id, "instance");
    assert!(state.recent[0].recorded_at < state.recent[0].intent.created_at);
    let encoded = serde_json::to_string(&state).unwrap();
    assert!(!encoded.contains(SOURCE));
    let (submission, _) = fixture("create");
    assert!(
        validate_state(&submission).is_err(),
        "recovery state cannot carry an acknowledgement"
    );
}

#[test]
fn native_state_rejects_ambiguous_missing_fields_or_borrowed_receipts() {
    for corruption in [
        "pending",
        "source",
        "intent-record",
        "digest",
        "missing-null",
        "revision",
        "operation",
        "receipt",
        "enabled",
        "deleted",
        "count-zero",
        "count-oversized",
        "time",
        "duplicate",
        "pending-duplicate",
        "seventeen",
        "meta",
        "rejected",
        "unknown",
    ] {
        let mut response = state_fixture();
        match corruption {
            "pending" => {
                response.result.as_object_mut().unwrap().remove("pending");
            }
            "source" => response.result["source"] = record(),
            "intent-record" => response.result["recent"][0]["intent"]["record"] = record(),
            "digest" => {
                response.result["recent"][0]["intent"]["recordDigest"] = json!("A".repeat(64))
            }
            "missing-null" => {
                response.result["recent"][0]["receipt"]
                    .as_object_mut()
                    .unwrap()
                    .remove("deleted");
            }
            "revision" => response.result["recent"][0]["receipt"]["revision"] = json!("4"),
            "operation" => response.result["recent"][0]["intent"]["operation"] = json!("adopt"),
            "receipt" => response.result["recent"][0]["receipt"] = Value::Null,
            "enabled" => response.result["recent"][0]["receipt"]["enabled"] = json!("false"),
            "deleted" => response.result["recent"][0]["receipt"]["deleted"] = json!(false),
            "count-zero" => response.result["recent"][0]["receipt"]["affectedResources"] = json!(0),
            "count-oversized" => {
                response.result["recent"][0]["receipt"]["affectedResources"] =
                    json!(u64::from(u32::MAX) + 1)
            }
            "time" => response.result["recent"][0]["recordedAt"] = json!(9_007_199_254_740_992u64),
            "duplicate" => {
                let entry = response.result["recent"][0].clone();
                response.result["recent"]
                    .as_array_mut()
                    .unwrap()
                    .push(entry);
            }
            "pending-duplicate" => {
                response.result["pending"] = response.result["recent"][0]["intent"].clone()
            }
            "seventeen" => {
                response.result["recent"] = json!(vec![response.result["recent"][0].clone(); 17])
            }
            "meta" => response.meta.as_mut().unwrap().host_instance_id = Some("borrowed".into()),
            "rejected" => {
                response.result["recent"][0]["outcome"] = json!("rejected");
                response.result["recent"][0]["receipt"] = Value::Null;
                response.result["recent"][0]["errorCode"] = json!("internal");
            }
            "unknown" => response.result["recent"][0]["outcome"] = json!("reviewed-unknown"),
            _ => unreachable!(),
        }
        let error = validate_state(&response).unwrap_err();
        assert_eq!(error.code, "protocol", "{corruption}");
        assert!(!error.message.contains(SOURCE));
    }
}

#[test]
fn fresh_acknowledgement_never_attaches_another_command_body_or_version() {
    for corruption in [
        "host",
        "command",
        "command-resource",
        "command-time",
        "command-null",
        "extra",
        "body-id",
        "body-source",
        "version",
        "version-kind",
        "version-target",
        "version-duplicate",
        "intent-source",
    ] {
        let (mut response, params) = fixture("create");
        match corruption {
            "host" => response.result["acknowledgement"]["hostInstanceId"] = json!("new-instance"),
            "command" => {
                response.result["acknowledgement"]["command"]["commandId"] =
                    json!(format!("owner-definition-{}", "b".repeat(32)))
            }
            "command-resource" => {
                response.result["acknowledgement"]["command"]
                    .as_object_mut()
                    .unwrap()
                    .remove("resource");
            }
            "command-time" => {
                response.result["acknowledgement"]["command"]["committedAt"] =
                    json!(9_007_199_254_740_992u64)
            }
            "command-null" => response.result["acknowledgement"]["command"] = Value::Null,
            "extra" => response.result["acknowledgement"]["source"] = record(),
            "body-id" => response.result["acknowledgement"]["value"]["id"] = json!("other"),
            "body-source" => {
                response.result["acknowledgement"]["value"]["config"]["message"] = json!("changed")
            }
            "version" => {
                response.result["acknowledgement"]["resourceVersions"][0]["revision"] = json!("4")
            }
            "version-kind" => {
                response.result["acknowledgement"]["resourceVersions"][0]["kind"] =
                    json!("workflow")
            }
            "version-target" => {
                response.result["acknowledgement"]["resourceVersions"][0]["id"] = json!("other")
            }
            "version-duplicate" => {
                let version = response.result["acknowledgement"]["resourceVersions"][0].clone();
                response.result["acknowledgement"]["resourceVersions"]
                    .as_array_mut()
                    .unwrap()
                    .push(version);
            }
            "intent-source" => {
                response.result["state"]["recent"][0]["intent"]["recordDigest"] =
                    json!("b".repeat(64))
            }
            _ => unreachable!(),
        }
        assert!(
            validate_submission(&response, &params).is_err(),
            "{corruption}"
        );
    }
}

#[test]
fn an_older_success_cannot_supply_the_acknowledgement_for_a_newer_rejection_or_pending_write() {
    let (mut response, params) = fixture("create");
    let mut rejected = response.result["state"]["recent"][0].clone();
    rejected["intent"]["commandId"] = json!(format!("owner-definition-{}", "b".repeat(32)));
    rejected["outcome"] = json!("rejected");
    rejected["receipt"] = Value::Null;
    rejected["errorCode"] = json!("revision_conflict");
    response.result["state"]["recent"]
        .as_array_mut()
        .unwrap()
        .insert(0, rejected.clone());
    assert!(validate_submission(&response, &params).is_err());
    response.result["acknowledgement"] = Value::Null;
    validate_submission(&response, &params).unwrap();
    response.result["state"]["recent"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    response.result["state"]["pending"] = rejected["intent"].clone();
    validate_submission(&response, &params).unwrap();
    let (old, _) = fixture("create");
    response.result["acknowledgement"] = old.result["acknowledgement"].clone();
    assert!(validate_submission(&response, &params).is_err());
}

#[test]
fn action_delete_accepts_large_ordered_cascades_but_rejects_wrong_family_duplicate_and_reordered_rows(
) {
    let (mut response, params) = fixture("delete");
    let versions = response.result["acknowledgement"]["resourceVersions"]
        .as_array_mut()
        .unwrap();
    for number in 0..514 {
        versions.push(json!({"kind":"automation","scope":"event","id":format!("event-{number:04}"),"revision":"12"}));
    }
    response.result["state"]["recent"][0]["receipt"]["affectedResources"] = json!(515);
    validate_submission(&response, &params).unwrap();
    for corruption in ["family", "duplicate", "order", "missing-action"] {
        let mut changed = response.clone();
        match corruption {
            "family" => {
                changed.result["acknowledgement"]["resourceVersions"][1]["scope"] = json!("action")
            }
            "duplicate" => {
                changed.result["acknowledgement"]["resourceVersions"][2] =
                    changed.result["acknowledgement"]["resourceVersions"][1].clone()
            }
            "order" => changed.result["acknowledgement"]["resourceVersions"]
                .as_array_mut()
                .unwrap()
                .swap(1, 2),
            "missing-action" => {
                changed.result["state"]["recent"][0]["receipt"]["deleted"] = json!(false);
                changed.result["state"]["recent"][0]["receipt"]["revision"] = json!("2");
                changed.result["acknowledgement"]["value"]["deleted"] = json!(false);
                changed.result["acknowledgement"]["resourceVersions"][0]["revision"] = json!("2");
            }
            _ => unreachable!(),
        }
        assert!(
            validate_submission(&changed, &params).is_err(),
            "{corruption}"
        );
    }
}

#[test]
fn normalized_source_and_exact_large_integers_are_bound_while_depth_nodes_and_bytes_are_bounded() {
    let (mut response, _) = fixture("create");
    let mut source = record();
    source["name"] = json!("   ");
    source["config"]["large"] = json!(u64::MAX);
    let params:OwnerDefinitionSubmit=serde_json::from_value(json!({"operation":"create","kind":"action","record":source,"expectedRevision":"2","hostId":"host","hostInstanceId":"instance"})).unwrap();
    source["name"] = json!("Untitled action");
    let digest = record_digest(&source).unwrap();
    response.result["state"]["recent"][0]["intent"]["recordDigest"] = json!(digest);
    response.result["state"]["recent"][0]["receipt"]["recordDigest"] = json!(digest);
    response.result["acknowledgement"]["value"] = source;
    validate_submission(&response, &params).unwrap();
    let mut over_nodes = record();
    over_nodes["config"]["array"] = json!(vec![0; 65_537]);
    assert!(serde_json::to_vec(&over_nodes).unwrap().len() < RECORD_BYTES);
    assert!(record_digest(&over_nodes).is_err());
    let mut deep = json!(0);
    for _ in 0..65 {
        deep = json!({"child":deep});
    }
    assert!(record_digest(&deep).is_err());
    assert!(record_digest(&json!({"id":"draft","source":"x".repeat(RECORD_BYTES)})).is_err());
}
