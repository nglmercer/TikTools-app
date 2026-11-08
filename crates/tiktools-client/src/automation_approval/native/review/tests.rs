use super::*;
use serde_json::json;

fn params() -> OwnerApprovalReviewParams {
    serde_json::from_value(json!({"kind":"event","id":"draft"})).unwrap()
}
fn fixture() -> CallResult<Value> {
    let delegated = |principal: &str| json!({"authority":{"kind":"delegated","principalId":principal,"sessionId":format!("session-{principal}")},"generation":"1"});
    let owner = json!({"authority":{"kind":"owner"},"generation":"1"});
    let event = json!({"id":"draft","name":"Reviewed event","enabled":false,"trigger":"tiktok.chat","actionIds":["action-b","action-owner"]});
    let first = json!({"id":"action-b","name":"Delegated log","enabled":false,"typeId":"core.log","config":{"message":"reviewed source"}});
    let second = json!({"id":"action-owner","name":"Owner log","enabled":false,"typeId":"core.log","config":{"message":"owner source"}});
    let source = json!({"kind":"event","id":"draft","record":event,"provenance":delegated("a"),"dependencies":[
        {"id":"action-b","record":first,"provenance":delegated("b")},
        {"id":"action-owner","record":second,"provenance":owner}
    ]});
    let binding = |kind: &str, id: &str, record: &Value, provenance: &Value| json!({"kind":kind,"id":id,"revision":"1","exists":true,"contentDigest":review_digest("definition",record).unwrap(),"provenance":provenance});
    let effect = |id: &str| json!({"family":"action","id":id,"typeId":"core.log","kind":"log","enabled":false,"decision":"classified","renderFields":[],"dynamicTarget":false,"requiredCapabilities":[],"reasons":[]});
    let subject = |principal: &str| json!({"principalId":principal,"sessionId":format!("session-{principal}"),"grantRevision":"2","configurationRevision":"3"});
    let mut manifest = json!({"policyVersion":1,"digestAlgorithm":"sha256-canonical-json-v1","hostId":"host","subjects":[subject("a"),subject("b")],
        "primary":binding("event","draft",&source["record"],&source["provenance"]),
        "dependencies":[binding("action","action-b",&source["dependencies"][0]["record"],&source["dependencies"][0]["provenance"]),binding("action","action-owner",&source["dependencies"][1]["record"],&source["dependencies"][1]["provenance"])],
        "effects":[effect("action-b"),effect("action-owner")],"reasons":["approval-required"],"executionAuthorized":false});
    manifest["manifestDigest"] = json!(review_digest("manifest", &manifest).unwrap());
    let grant = serde_json::to_value(Grant {
        capabilities: [
            tiktools_control_api::modules::collaboration::Capability::LiveRead,
            tiktools_control_api::modules::collaboration::Capability::AutomationRead,
        ]
        .into(),
        ..Grant::default()
    })
    .unwrap();
    let subjects: Vec<_> = ["a", "b"]
        .iter()
        .map(|principal| {
            let mut value = subject(principal);
            value["grant"] = grant.clone();
            value
        })
        .collect();
    CallResult {
        result: json!({"hostId":"host","hostInstanceId":"instance","kind":"event","id":"draft","source":source,"manifest":manifest,"subjects":subjects,
            "eligibility":{"eligible":true,"reasonCode":null},"approval":{"state":"none","generation":null,"policyVersion":null,"manifestDigest":null,"current":false}}),
        meta: Some(
            serde_json::from_value(json!({"hostInstanceId":"instance","resourceVersions":[
                {"kind":"automation","scope":"event","id":"draft","revision":"1"},
                {"kind":"automation","scope":"action","id":"action-b","revision":"1"},
                {"kind":"automation","scope":"action","id":"action-owner","revision":"1"}
            ]}))
            .unwrap(),
        ),
    }
}

#[test]
fn source_manifest_and_current_subjects_are_one_exact_review() {
    let response = fixture();
    let parsed = validate(response.clone(), &params()).unwrap();
    assert_eq!(parsed.result.id, "draft");
    assert_eq!(parsed.meta.unwrap().resource_versions.len(), 3);
    let mut current = response;
    current.result["approval"] = json!({"state":"active","generation":"2","policyVersion":1,"manifestDigest":current.result["manifest"]["manifestDigest"],"current":true});
    validate(current, &params()).unwrap();
}

#[test]
fn exact_review_accepts_new_effect_scopes_without_widening_legacy_grants() {
    let mut response = fixture();
    response.result["subjects"][0]["grant"]["effects"] =
        json!({"http":[],"audio":[],"emitTopics":["moderator.alert"]});
    validate(response.clone(), &params()).unwrap();
    response.result["subjects"][0]["grant"]["effects"]["unknown"] = json!(true);
    assert!(validate(response, &params()).is_err());
    let mut empty = fixture();
    empty.result["subjects"][0]["grant"]["effects"] = json!({"http":[],"audio":[],"emitTopics":[]});
    // Host canonical serialization omits empty effects, retaining old five-key
    // grants. A differently shaped reply cannot smuggle additional authority.
    assert!(validate(empty, &params()).is_err());
}

#[test]
fn source_drift_digest_tampering_and_grant_or_configuration_mismatch_are_rejected() {
    for corruption in [
        "source-body",
        "source-provenance",
        "manifest-digest",
        "manifest-content",
        "config",
        "grant-revision",
        "subject-order",
        "subject-missing",
        "grant-extra",
        "nullable",
        "root-extra",
        "host",
        "meta-host",
        "status-current",
        "status-digest",
    ] {
        let mut response = fixture();
        match corruption {
            "source-body" => {
                response.result["source"]["dependencies"][0]["record"]["config"]["message"] =
                    json!("different source at same revision")
            }
            "source-provenance" => {
                response.result["source"]["provenance"]["generation"] = json!("2")
            }
            "manifest-digest" => {
                response.result["manifest"]["manifestDigest"] = json!("a".repeat(64))
            }
            "manifest-content" => response.result["manifest"]["reasons"] = json!([]),
            "config" => response.result["subjects"][0]["configurationRevision"] = json!("4"),
            "grant-revision" => response.result["subjects"][0]["grantRevision"] = json!("3"),
            "subject-order" => response.result["subjects"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1),
            "subject-missing" => {
                response.result["subjects"].as_array_mut().unwrap().pop();
            }
            "grant-extra" => response.result["subjects"][0]["grant"]["secrets"] = json!({}),
            "nullable" => {
                response.result["approval"]
                    .as_object_mut()
                    .unwrap()
                    .remove("generation");
            }
            "root-extra" => response.result["permit"] = json!({}),
            "host" => response.result["hostId"] = json!("another-host"),
            "meta-host" => {
                response.meta.as_mut().unwrap().host_instance_id = Some("another-instance".into())
            }
            "status-current" => response.result["approval"]["current"] = json!(true),
            "status-digest" => {
                response.result["approval"] = json!({"state":"active","generation":"2","policyVersion":1,"manifestDigest":"b".repeat(64),"current":true})
            }
            _ => unreachable!(),
        }
        assert!(validate(response, &params()).is_err(), "{corruption}");
    }
}

#[test]
fn unavailable_subjects_and_deleted_primary_still_allow_coherent_revoke_review() {
    let mut unavailable = fixture();
    unavailable.result["manifest"] = Value::Null;
    unavailable.result["subjects"] = json!([]);
    unavailable.result["eligibility"] =
        json!({"eligible":false,"reasonCode":"subjects-unavailable"});
    unavailable.result["approval"] = json!({"state":"active","generation":"2","policyVersion":1,"manifestDigest":"a".repeat(64),"current":false});
    validate(unavailable.clone(), &params()).unwrap();
    unavailable.result["source"]["record"] = Value::Null;
    unavailable.result["source"]["dependencies"] = json!([]);
    unavailable
        .meta
        .as_mut()
        .unwrap()
        .resource_versions
        .truncate(1);
    unavailable.result["eligibility"]["reasonCode"] = json!("definition-missing");
    validate(unavailable.clone(), &params()).unwrap();
    unavailable.result["approval"]["current"] = json!(true);
    assert!(validate(unavailable, &params()).is_err());
}
