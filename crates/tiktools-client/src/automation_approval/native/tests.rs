use super::*;
use serde_json::json;

fn fixture() -> CallResult<Value> {
    CallResult {
        result: json!({"hostId":"host","hostInstanceId":"instance","pending":null,"recent":[{
            "intent":{"commandId":format!("owner-approval-{}","a".repeat(32)),"operation":"activate","kind":"event","id":"draft","expectedRevision":"2","hostInstanceId":"instance-original","createdAt":30},
            "outcome":"succeeded","receipt":{"revision":"3","approvalGeneration":"4","policyVersion":1,"manifestDigest":"b".repeat(64)},"errorCode":null,"recordedAt":20
        }]}),
        meta: Some(serde_json::from_value(json!({"resourceVersions":[]})).unwrap()),
    }
}

#[test]
fn native_state_preserves_original_metadata_across_host_restart_and_clock_rollback() {
    let response = fixture();
    let state = validate_state(&response).unwrap();
    assert_eq!(state.host_instance_id, "instance");
    assert_eq!(state.recent[0].intent.host_instance_id, "instance-original");
    assert!(state.recent[0].recorded_at < state.recent[0].intent.created_at);
    let mut revoked = fixture();
    revoked.result["recent"][0]["intent"]["operation"] = json!("revoke");
    revoked.result["recent"][0]["receipt"]["policyVersion"] = Value::Null;
    revoked.result["recent"][0]["receipt"]["manifestDigest"] = Value::Null;
    validate_state(&revoked).unwrap();
}

#[test]
fn native_state_rejects_missing_nullable_fields_source_and_receipt_confusion() {
    for corruption in [
        "pending-missing",
        "source",
        "intent-source",
        "receipt-missing",
        "error-missing",
        "command-id",
        "generation",
        "revision",
        "policy",
        "digest",
        "meta-versions",
        "meta-command",
        "duplicate",
        "pending-duplicate",
        "count",
        "time",
        "rejection",
        "reviewed-unknown",
    ] {
        let mut response = fixture();
        match corruption {
            "pending-missing" => {
                response.result.as_object_mut().unwrap().remove("pending");
            }
            "source" => response.result["source"] = json!({}),
            "intent-source" => response.result["recent"][0]["intent"]["enablePlan"] = json!([]),
            "receipt-missing" => {
                response.result["recent"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("receipt");
            }
            "error-missing" => {
                response.result["recent"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("errorCode");
            }
            "command-id" => {
                response.result["recent"][0]["intent"]["commandId"] = json!("chosen-command")
            }
            "generation" => {
                response.result["recent"][0]["receipt"]["approvalGeneration"] = json!("04")
            }
            "revision" => response.result["recent"][0]["receipt"]["revision"] = json!("4"),
            "policy" => response.result["recent"][0]["receipt"]["policyVersion"] = Value::Null,
            "digest" => {
                response.result["recent"][0]["receipt"]["manifestDigest"] = json!("B".repeat(64))
            }
            "meta-versions" => {
                response.meta.as_mut().unwrap().resource_versions = serde_json::from_value(
                    json!([{"kind":"automation","scope":"event","id":"draft","revision":"3"}]),
                )
                .unwrap()
            }
            "meta-command" => {
                response.meta.as_mut().unwrap().host_instance_id = Some("instance".into())
            }
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
            "count" => {
                let entry = response.result["recent"][0].clone();
                response.result["recent"] = Value::Array(vec![entry; 17]);
            }
            "time" => response.result["recent"][0]["recordedAt"] = json!(9_007_199_254_740_992u64),
            "rejection" => {
                response.result["recent"][0]["outcome"] = json!("rejected");
                response.result["recent"][0]["errorCode"] = json!("internal");
                response.result["recent"][0]["receipt"] = Value::Null;
            }
            "reviewed-unknown" => {
                response.result["recent"][0]["outcome"] = json!("reviewed-unknown")
            }
            _ => unreachable!(),
        }
        assert!(validate_state(&response).is_err(), "{corruption}");
    }
}

#[test]
fn submit_state_must_bind_the_host_target_operation_and_original_base() {
    let mut response = fixture();
    let params:OwnerApprovalSubmit=serde_json::from_value(json!({
        "operation":"activate","hostId":"host","hostInstanceId":"instance-original","kind":"event","id":"draft","expectedRevision":"2",
        "reviewedVersions":[{"kind":"automation","scope":"event","id":"draft","revision":"2"}],
        "manifestDigest":"a".repeat(64),"enablePlan":[{"kind":"event","id":"draft","enabled":true}],"acknowledgeEffects":true
    })).unwrap();
    response.result["hostInstanceId"] = json!("instance-original");
    validate_submission(&response, &params).unwrap();
    for field in [
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
    ] {
        let mut changed = response.clone();
        changed.result["recent"][0]["intent"][field] = json!(match field {
            "operation" => "revoke",
            "kind" => "action",
            "id" => "other",
            "expectedRevision" => "3",
            _ => "instance-other",
        });
        assert!(validate_submission(&changed, &params).is_err(), "{field}");
    }
    response.result["hostId"] = json!("another-host");
    assert!(validate_submission(&response, &params).is_err());
}
