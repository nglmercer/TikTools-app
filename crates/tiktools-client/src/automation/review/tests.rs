use super::*;
use serde_json::json;

fn snapshot() -> (Value, Value) {
    (
        json!({
            "kind":"event","id":"same",
            "record":{"id":"same","enabled":false,"actionIds":["same","gone","same","missing"],"provenance":{"authority":{"kind":"owner"},"generation":"999"}},
            "provenance":{"authority":{"kind":"delegated","principalId":"author","sessionId":"session"},"generation":"9223372036854775807"},
            "dependencies":[
                {"id":"same","record":{"id":"same","enabled":false},"provenance":{"authority":{"kind":"owner"},"generation":"2"}},
                {"id":"gone","record":null,"provenance":{"authority":{"kind":"owner"},"generation":"7"}},
                {"id":"missing","record":null,"provenance":null}
            ]
        }),
        json!({"resourceVersions":[
            {"kind":"automation","scope":"event","id":"same","revision":"3"},
            {"kind":"automation","scope":"action","id":"same","revision":"9"},
            {"kind":"automation","scope":"action","id":"gone","revision":"7"},
            {"kind":"automation","scope":"action","id":"missing","revision":"0"}
        ]}),
    )
}

fn response(body: Value, meta: Value) -> Result<CallResult<AutomationReviewResult>, ClientError> {
    Ok(CallResult {
        result: serde_json::from_value(body).map_err(|_| protocol())?,
        meta: Some(serde_json::from_value(meta).map_err(|_| protocol())?),
    })
}

fn params() -> AutomationGetParams {
    AutomationGetParams {
        kind: Some(" EVENTS ".into()),
        id: " same ".into(),
    }
}

#[test]
fn review_validation_keeps_all_dependencies_tombstones_and_trusted_authority_separate() {
    let (body, meta) = snapshot();
    let reply = response(body, meta).unwrap();
    validate(&reply, &params()).unwrap();
    assert!(matches!(
        reply.result.provenance.unwrap().authority,
        AutomationReviewAuthority::Delegated { .. }
    ));
}

#[test]
fn review_validation_rejects_partial_reordered_inconsistent_or_noncanonical_snapshots() {
    for corruption in [
        "family",
        "target",
        "order",
        "omitted",
        "duplicate",
        "version-scope",
        "version-kind",
        "version-target",
        "version-order",
        "version-noncanonical",
        "version-negative",
        "missing-live-provenance",
        "missing-tombstone-provenance",
        "unexpected-zero-provenance",
        "unexpected-zero-record",
        "record-target",
        "generation-zero",
        "generation-noncanonical",
        "generation-overflow",
        "principal",
        "session",
        "unknown-authority",
        "references-type",
        "reference-type",
        "reference-trim",
        "reference-wildcard",
        "reference-control",
    ] {
        let (mut body, mut meta) = snapshot();
        match corruption {
            "family" => body["kind"] = "action".into(),
            "target" => body["id"] = "other".into(),
            "order" => body["dependencies"].as_array_mut().unwrap().swap(0, 1),
            "omitted" => {
                body["dependencies"].as_array_mut().unwrap().pop();
            }
            "duplicate" => body["dependencies"][1] = body["dependencies"][0].clone(),
            "version-scope" => meta["resourceVersions"][1]["scope"] = "event".into(),
            "version-kind" => meta["resourceVersions"][1]["kind"] = "viewer".into(),
            "version-target" => meta["resourceVersions"][1]["id"] = "other".into(),
            "version-order" => meta["resourceVersions"].as_array_mut().unwrap().swap(1, 2),
            "version-noncanonical" => meta["resourceVersions"][1]["revision"] = "09".into(),
            "version-negative" => meta["resourceVersions"][1]["revision"] = "-1".into(),
            "missing-live-provenance" => body["provenance"] = Value::Null,
            "missing-tombstone-provenance" => body["dependencies"][1]["provenance"] = Value::Null,
            "unexpected-zero-provenance" => {
                body["dependencies"][2]["provenance"] =
                    body["dependencies"][1]["provenance"].clone()
            }
            "unexpected-zero-record" => body["dependencies"][2]["record"] = json!({"id":"missing"}),
            "record-target" => body["dependencies"][0]["record"]["id"] = "other".into(),
            "generation-zero" => body["provenance"]["generation"] = "0".into(),
            "generation-noncanonical" => body["provenance"]["generation"] = "01".into(),
            "generation-overflow" => {
                body["provenance"]["generation"] = "9223372036854775808".into()
            }
            "principal" => body["provenance"]["authority"]["principalId"] = "author*".into(),
            "session" => body["provenance"]["authority"]["sessionId"] = " session ".into(),
            "unknown-authority" => body["provenance"]["authority"]["kind"] = "arbitrary".into(),
            "references-type" => body["record"]["actionIds"] = "same".into(),
            "reference-type" => body["record"]["actionIds"][0] = 7.into(),
            "reference-trim" => body["record"]["actionIds"][0] = " same ".into(),
            "reference-wildcard" => body["record"]["actionIds"][0] = "*".into(),
            "reference-control" => body["record"]["actionIds"][0] = "same\n".into(),
            _ => unreachable!(),
        }
        let outcome = response(body, meta).and_then(|reply| validate(&reply, &params()));
        assert_eq!(outcome.unwrap_err().code, "protocol", "{corruption}");
    }
    let (body, meta) = snapshot();
    let mut reply = response(body, meta).unwrap();
    reply.meta = None;
    assert_eq!(validate(&reply, &params()).unwrap_err().code, "protocol");
}

#[test]
fn review_validation_requires_empty_action_or_missing_primary_dependencies_and_bounded_records() {
    let (mut body, mut meta) = snapshot();
    body["kind"] = "action".into();
    body["dependencies"] = json!([]);
    meta["resourceVersions"].as_array_mut().unwrap().truncate(1);
    meta["resourceVersions"][0]["scope"] = "action".into();
    let mut reply = response(body, meta).unwrap();
    let action_params = AutomationGetParams {
        kind: Some("action".into()),
        id: "same".into(),
    };
    validate(&reply, &action_params).unwrap();
    reply.result.record.as_mut().unwrap()["source"] = "x".repeat(DEFINITION_BYTES).into();
    assert_eq!(
        validate(&reply, &action_params).unwrap_err().code,
        "protocol"
    );

    let (mut body, mut meta) = snapshot();
    body["record"] = Value::Null;
    body["dependencies"] = json!([]);
    meta["resourceVersions"].as_array_mut().unwrap().truncate(1);
    let mut reply = response(body, meta).unwrap();
    validate(&reply, &params()).unwrap();
    reply.result.provenance = None;
    reply.meta.as_mut().unwrap().resource_versions[0].revision = "0".into();
    validate(&reply, &params()).unwrap();
}

#[test]
fn review_validation_rejects_more_than_512_distinct_dependencies() {
    let (mut body, mut meta) = snapshot();
    let ids: Vec<_> = (0..=MAX_DEPENDENCIES)
        .map(|index| format!("action:{index}"))
        .collect();
    body["record"]["actionIds"] = json!(ids);
    body["dependencies"] = json!(ids
        .iter()
        .map(|id| json!({"id":id,"record":null,"provenance":null}))
        .collect::<Vec<_>>());
    let versions = meta["resourceVersions"].as_array_mut().unwrap();
    versions.truncate(1);
    versions.extend(
        ids.iter()
            .map(|id| json!({"kind":"automation","scope":"action","id":id,"revision":"0"})),
    );
    assert_eq!(
        validate(&response(body, meta).unwrap(), &params())
            .unwrap_err()
            .code,
        "protocol"
    );
}

#[test]
fn review_validation_rejects_an_aggregate_larger_than_four_mebibytes() {
    let (mut body, mut meta) = snapshot();
    let ids: Vec<_> = (0..17).map(|index| format!("action:{index}")).collect();
    body["record"]["actionIds"] = json!(ids);
    body["dependencies"] = json!(ids
        .iter()
        .map(|id| json!({
            "id":id,
            "record":{"id":id,"source":"x".repeat(250_000)},
            "provenance":{"authority":{"kind":"owner"},"generation":"1"}
        }))
        .collect::<Vec<_>>());
    let versions = meta["resourceVersions"].as_array_mut().unwrap();
    versions.truncate(1);
    versions.extend(
        ids.iter()
            .map(|id| json!({"kind":"automation","scope":"action","id":id,"revision":"1"})),
    );
    assert_eq!(
        validate(&response(body, meta).unwrap(), &params())
            .unwrap_err()
            .code,
        "protocol"
    );
}
