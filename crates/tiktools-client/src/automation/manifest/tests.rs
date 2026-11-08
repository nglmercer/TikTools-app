use super::*;
use serde_json::json;

fn snapshot() -> (Value, Value) {
    let delegated = json!({"authority":{"kind":"delegated","principalId":"author","sessionId":"session"},"generation":"3"});
    let owner = json!({"authority":{"kind":"owner"},"generation":"4"});
    let binding = |kind: &str, id: &str, revision: &str, exists: bool, provenance: Value| json!({"kind":kind,"id":id,"revision":revision,"exists":exists,"contentDigest":if exists{json!("1".repeat(64))}else{Value::Null},"provenance":provenance});
    let effect = |id: &str, present: bool| json!({"family":"action","id":id,"typeId":if present{json!("core.log")}else{Value::Null},"kind":if present{"log"}else{"missing-definition"},"enabled":if present{json!(false)}else{Value::Null},"decision":if present{"classified"}else{"unsupported"},"renderFields":[],"dynamicTarget":false,"requiredCapabilities":[],"reasons":if present{json!([])}else{json!(["missing-dependency"])}});
    (
        json!({"policyVersion":1,"digestAlgorithm":"sha256-canonical-json-v1","hostId":"host","subjects":[{"principalId":"author","sessionId":"session","grantRevision":"2","configurationRevision":"4"}],
        "primary":binding("event","same","3",true,delegated.clone()),
        "dependencies":[binding("action","same","4",true,owner),binding("action","gone","5",false,delegated),binding("action","missing","0",false,Value::Null)],
        "effects":[effect("same",true),effect("gone",false),effect("missing",false)],"reasons":["approval-required","missing-dependency"],"executionAuthorized":false,"manifestDigest":"a".repeat(64)}),
        json!({"resourceVersions":[{"kind":"automation","scope":"event","id":"same","revision":"3"},{"kind":"automation","scope":"action","id":"same","revision":"4"},{"kind":"automation","scope":"action","id":"gone","revision":"5"},{"kind":"automation","scope":"action","id":"missing","revision":"0"}]}),
    )
}
fn response(body: Value, meta: Value) -> CallResult<Value> {
    CallResult {
        result: body,
        meta: Some(serde_json::from_value(meta).unwrap()),
    }
}
fn params() -> AutomationManifestParams {
    serde_json::from_value(json!({"kind":"event","id":"same"})).unwrap()
}

#[test]
fn manifest_validation_retains_exact_family_versions_tombstones_subjects_and_opaque_digests() {
    let (body, meta) = snapshot();
    let response = response(body, meta);
    validate(&response, &params()).unwrap();
    assert_eq!(response.result["dependencies"][1]["exists"], false);
    assert_eq!(response.result["dependencies"][1]["revision"], "5");
    assert_eq!(response.result["executionAuthorized"], false);
}

#[test]
fn malformed_partial_or_authority_broadening_manifests_are_rejected() {
    for corruption in [
        "policy",
        "algorithm",
        "authorization",
        "source",
        "digest",
        "target",
        "kind",
        "dependency-order",
        "dependency-omitted",
        "dependency-duplicate",
        "revision",
        "meta-kind",
        "meta-scope",
        "meta-omitted",
        "provenance",
        "generation",
        "tombstone",
        "absent",
        "subject-omitted",
        "subject-extra",
        "subject-generation",
        "subject-configuration-missing",
        "subject-configuration-generation",
        "subject-forged",
        "effect-omitted",
        "effect-order",
        "effect-source",
        "effect-type",
        "effect-family",
        "effect-decision",
        "effect-capability",
        "effect-reason",
        "effect-render-duplicate",
        "global-reason",
    ] {
        let (mut body, mut meta) = snapshot();
        match corruption {
            "policy" => body["policyVersion"] = json!(2),
            "algorithm" => body["digestAlgorithm"] = json!("different"),
            "authorization" => body["executionAuthorized"] = json!(true),
            "source" => body["source"] = json!("must not materialize"),
            "digest" => body["manifestDigest"] = json!("A".repeat(64)),
            "target" => body["primary"]["id"] = json!("other"),
            "kind" => body["primary"]["kind"] = json!("action"),
            "dependency-order" => body["dependencies"].as_array_mut().unwrap().swap(0, 1),
            "dependency-omitted" => {
                body["dependencies"].as_array_mut().unwrap().pop();
            }
            "dependency-duplicate" => body["dependencies"][1] = body["dependencies"][0].clone(),
            "revision" => body["dependencies"][0]["revision"] = json!("04"),
            "meta-kind" => meta["resourceVersions"][0]["kind"] = json!("workflow"),
            "meta-scope" => meta["resourceVersions"][1]["scope"] = json!("event"),
            "meta-omitted" => {
                meta["resourceVersions"].as_array_mut().unwrap().pop();
            }
            "provenance" => body["primary"]["provenance"] = Value::Null,
            "generation" => body["primary"]["provenance"]["generation"] = json!("0"),
            "tombstone" => body["dependencies"][1]["contentDigest"] = json!("b".repeat(64)),
            "absent" => {
                body["dependencies"][2]["provenance"] = body["primary"]["provenance"].clone()
            }
            "subject-omitted" => body["subjects"] = json!([]),
            "subject-extra" => body["subjects"]
                .as_array_mut()
                .unwrap()
                .push(json!({"principalId":"extra","sessionId":"session","grantRevision":"1","configurationRevision":"4"})),
            "subject-generation" => body["subjects"][0]["grantRevision"] = json!("0"),
            "subject-configuration-missing" => { body["subjects"][0].as_object_mut().unwrap().remove("configurationRevision"); },
            "subject-configuration-generation" => body["subjects"][0]["configurationRevision"] = json!("04"),
            "subject-forged" => body["subjects"][0]["principalId"] = json!("other"),
            "effect-omitted" => {
                body["effects"].as_array_mut().unwrap().pop();
            }
            "effect-order" => body["effects"].as_array_mut().unwrap().swap(0, 1),
            "effect-source" => body["effects"][0]["source"] = json!("private script"),
            "effect-type" => body["effects"][0]["typeId"] = json!("bad*type"),
            "effect-family" => body["effects"][0]["family"] = json!("event"),
            "effect-decision" => body["effects"][0]["decision"] = json!("approved"),
            "effect-capability" => {
                body["effects"][0]["requiredCapabilities"] = json!(["system.shutdown"])
            }
            "effect-reason" => body["effects"][0]["reasons"] = json!(["approved"]),
            "effect-render-duplicate" => {
                body["effects"][0]["renderFields"] =
                    json!(["event.data.viewer", "event.data.viewer"])
            }
            "global-reason" => body["reasons"] = json!(["approved"]),
            _ => unreachable!(),
        }
        assert_eq!(
            validate(&response(body, meta), &params()).unwrap_err().code,
            "protocol",
            "{corruption}"
        );
    }
}
