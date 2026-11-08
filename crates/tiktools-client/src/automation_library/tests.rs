use super::*;

fn plain(result: Value) -> CallResult<Value> {
    CallResult {
        result,
        meta: Some(serde_json::from_value(json!({"resourceVersions":[]})).unwrap()),
    }
}
fn entry() -> Value {
    json!({"actions":[{"id":"action","name":"Private action","enabled":false,"typeId":"core.log","config":{"message":"private-library-source","large":u64::MAX}}],"event":{"id":"event","name":"Private event","enabled":false,"trigger":"tiktok.chat","actionIds":["action"]}})
}
fn mutations() -> Vec<Value> {
    vec![
        json!({"operation":"create-profile","id":"a","name":"Profile","description":""}),
        json!({"operation":"switch-profile","id":"a"}),
        json!({"operation":"delete-profile","id":"a"}),
        json!({"operation":"apply-template","entry":entry(),"profileId":"default"}),
        json!({"operation":"import-profile","profile":{"id":"a","name":"Profile","description":""},"entries":[entry()]}),
        json!({"operation":"assign-record","kind":"action","id":"action","profileId":"a"}),
        json!({"operation":"import-templates","templates":[{"id":"template","name":"Private template"}]}),
        json!({"operation":"delete-template","id":"template"}),
    ]
}
fn input(mutation: Value) -> OwnerLibrarySubmit {
    serde_json::from_value(json!({"mutation":mutation,"expectedRevision":"2","definitionDigest":"a".repeat(64),"hostId":"host","hostInstanceId":"instance"})).unwrap()
}
fn fixture(mutation: Value) -> (CallResult<Value>, OwnerLibrarySubmit) {
    let input = input(mutation);
    let command = format!("owner-library-{}", "a".repeat(32));
    let intent = json!({"commandId":command,"operation":input.mutation.operation(),"target":input.mutation.target(),"expectedRevision":"2","definitionDigest":input.definition_digest,"commandHash":command_hash(&input).unwrap(),"hostInstanceId":"instance","createdAt":30});
    let versions = json!([{"kind":"automation","scope":"library","id":"catalog","revision":"3"}]);
    let receipt = json!({"revision":"3","catalogDigest":"b".repeat(64),"definitionDigest":"c".repeat(64),"versionsDigest":digest(&versions).unwrap(),"affectedResources":1});
    let state = json!({"hostId":"host","hostInstanceId":"instance","pending":null,"recent":[{"intent":intent,"outcome":"succeeded","receipt":receipt,"errorCode":null,"recordedAt":20}]});
    let ack = json!({"value":receipt,"resourceVersions":versions,"command":{"commandId":command,"state":"succeeded","resource":{"kind":"automation","scope":"library","id":"catalog"},"committedAt":40},"hostInstanceId":"instance"});
    (plain(json!({"state":state,"acknowledgement":ack})), input)
}
fn snapshot() -> CallResult<Value> {
    let entry = entry();
    let actions = entry["actions"].clone();
    let events = json!([entry["event"]]);
    let versions = json!([
        {"kind":"automation","scope":"action","id":"action","revision":"9007199254740993"},
        {"kind":"automation","scope":"event","id":"event","revision":"1"}
    ]);
    let hash =
        digest(&json!({"actions":actions,"events":events,"resourceVersions":versions})).unwrap();
    plain(
        json!({"hostId":"host","hostInstanceId":"instance","revision":"0","definitionDigest":hash,
        "catalog":{"packs":[{"id":"á","name":"Unicode profile","description":"","eventIds":["event"],"actionIds":["action"]}],"activeId":"á","templates":[]},
        "definitions":{"actions":actions,"events":events,"plugins":[],"actionTypes":[],"translations":{}},"resourceVersions":versions}),
    )
}

#[test]
fn snapshots_link_exact_large_integers_source_and_sorted_family_versions() {
    let response = snapshot();
    let parsed = validate_snapshot(&response).unwrap();
    assert_eq!(
        parsed.definitions["actions"][0]["config"]["large"],
        json!(u64::MAX)
    );
    assert_eq!(parsed.resource_versions[0].revision, "9007199254740993");
    for path in [
        "source",
        "revision",
        "family",
        "missing",
        "duplicate",
        "hash",
    ] {
        let mut bad = response.clone();
        match path {
            "source" => bad.result["definitions"]["actions"][0]["name"] = json!("Changed source"),
            "revision" => bad.result["resourceVersions"][0]["revision"] = json!("01"),
            "family" => bad.result["resourceVersions"][0]["scope"] = json!("event"),
            "missing" => {
                bad.result.as_object_mut().unwrap().remove("catalog");
            }
            "duplicate" => bad.result["definitions"]["actions"]
                .as_array_mut()
                .unwrap()
                .push(entry()["actions"][0].clone()),
            _ => bad.result["definitionDigest"] = json!("a".repeat(64)),
        }
        assert!(validate_snapshot(&bad).is_err(), "{path}");
    }
}

#[test]
fn each_closed_operation_binds_its_original_first_receipt_and_no_recovery_source() {
    for mutation in mutations() {
        let (response, input) = fixture(mutation);
        validate_owner_library_submit(&input).unwrap();
        let parsed = validate_submission(&response, &input).unwrap();
        assert!(parsed.acknowledgement.is_some());
        assert!(!serde_json::to_string(&parsed)
            .unwrap()
            .contains("private-library-source"));
        let mut historical = response.clone();
        historical.result["acknowledgement"] = Value::Null;
        validate_submission(&historical, &input).unwrap();
        validate_state(&plain(historical.result["state"].clone())).unwrap();
    }
}

#[test]
fn fresh_receipts_reject_cross_command_host_mutation_and_version_hashes() {
    let (response, input) = fixture(mutations().remove(0));
    for field in [
        "command",
        "host",
        "target",
        "input",
        "receipt",
        "versions",
        "primary",
        "unsafe-time",
        "nullable",
    ] {
        let mut bad = response.clone();
        match field {
            "command" => {
                bad.result["acknowledgement"]["command"]["commandId"] =
                    json!(format!("owner-library-{}", "b".repeat(32)))
            }
            "host" => bad.result["acknowledgement"]["hostInstanceId"] = json!("other"),
            "target" => bad.result["state"]["recent"][0]["intent"]["target"] = json!("other"),
            "input" => {
                bad.result["state"]["recent"][0]["intent"]["commandHash"] = json!("b".repeat(64))
            }
            "receipt" => {
                bad.result["acknowledgement"]["value"]["catalogDigest"] = json!("c".repeat(64))
            }
            "versions" => {
                bad.result["acknowledgement"]["resourceVersions"][0]["revision"] = json!("4")
            }
            "primary" => {
                bad.result["acknowledgement"]["command"]["resource"]["scope"] = json!("action")
            }
            "unsafe-time" => {
                bad.result["acknowledgement"]["command"]["committedAt"] =
                    json!(9_007_199_254_740_992u64)
            }
            _ => {
                bad.result["state"]["recent"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("errorCode");
            }
        }
        assert!(validate_submission(&bad, &input).is_err(), "{field}");
    }
    let mut old = response.clone();
    let mut newer = old.result["state"]["recent"][0].clone();
    newer["intent"]["commandId"] = json!(format!("owner-library-{}", "b".repeat(32)));
    old.result["state"]["recent"]
        .as_array_mut()
        .unwrap()
        .insert(0, newer);
    assert!(validate_submission(&old, &input).is_err());
}

#[test]
fn acknowledgement_versions_have_one_primary_then_unique_sorted_family_rows() {
    let (mut response, input) = fixture(mutations().remove(1));
    let rows = json!([
        {"kind":"automation","scope":"library","id":"catalog","revision":"3"},
        {"kind":"automation","scope":"action","id":"same","revision":"4"},
        {"kind":"automation","scope":"event","id":"same","revision":"5"}
    ]);
    response.result["acknowledgement"]["resourceVersions"] = rows.clone();
    response.result["state"]["recent"][0]["receipt"]["affectedResources"] = json!(3);
    response.result["state"]["recent"][0]["receipt"]["versionsDigest"] =
        json!(digest(&rows).unwrap());
    response.result["acknowledgement"]["value"] =
        response.result["state"]["recent"][0]["receipt"].clone();
    validate_submission(&response, &input).unwrap();
    for changed in [
        json!([rows[1], rows[0], rows[2]]),
        json!([rows[0], rows[2], rows[1]]),
        json!([rows[0], rows[1], rows[1]]),
    ] {
        let mut bad = response.clone();
        bad.result["acknowledgement"]["resourceVersions"] = changed;
        assert!(validate_submission(&bad, &input).is_err());
    }
}

#[test]
fn retained_state_is_closed_bounded_and_accepts_clock_rollback_old_instances() {
    let (response, _) = fixture(mutations().remove(0));
    let mut original = response.result["state"].clone();
    original["recent"][0]["intent"]["hostInstanceId"] = json!("prior-instance");
    validate_state(&plain(original.clone())).unwrap();
    for field in [
        "source",
        "count",
        "duplicate",
        "operation",
        "revision",
        "integer",
    ] {
        let mut bad = original.clone();
        match field {
            "source" => bad["recent"][0]["source"] = entry(),
            "count" => bad["recent"] = json!(vec![bad["recent"][0].clone(); 17]),
            "duplicate" => bad["pending"] = bad["recent"][0]["intent"].clone(),
            "operation" => bad["recent"][0]["intent"]["operation"] = json!("execute"),
            "revision" => bad["recent"][0]["receipt"]["revision"] = json!("9223372036854775808"),
            _ => bad["recent"][0]["recordedAt"] = json!(u64::MAX),
        }
        assert!(validate_state(&plain(bad)).is_err(), "{field}");
    }
}

#[test]
fn source_structure_and_byte_budgets_precede_canonical_hashing() {
    let mut deep = Value::Null;
    for _ in 0..65 {
        deep = json!([deep]);
    }
    assert!(source(&deep).is_err());
    assert!(source(&json!(vec![Value::Null; 65_536])).is_err());
    assert!(source(&json!({"source":"x".repeat(RECORD_BYTES)})).is_err());
    let mut value = snapshot();
    value.result["definitions"]["actions"][0]["config"] = deep;
    assert!(validate_snapshot(&value).is_err());
}

#[test]
fn legacy_snapshot_universe_is_byte_bounded_without_a_512_row_or_new_write_size_cap() {
    let mut response = snapshot();
    response.result["catalog"]["templates"] =
        json!([{"id":"legacy-template","params":{"retained":"t".repeat(300_000)}}]);
    let actions:Vec<_> = (0..600).map(|index|json!({"id":format!("legacy-{index:04}"),"name":"Legacy","enabled":false,"config":if index == 0 {json!({"source":"s".repeat(300_000)})} else {json!({})}})).collect();
    let versions: Vec<_> = actions
        .iter()
        .map(|row| json!({"kind":"automation","scope":"action","id":row["id"],"revision":"1"}))
        .collect();
    response.result["definitions"]["actions"] = json!(actions);
    response.result["definitions"]["events"] = json!([]);
    response.result["resourceVersions"] = json!(versions);
    response.result["definitionDigest"] = json!(digest(&json!({"actions":response.result["definitions"]["actions"],"events":[],"resourceVersions":versions})).unwrap());
    validate_snapshot(&response).unwrap();
}

#[test]
fn combined_gallery_graph_fields_are_closed_and_part_of_the_original_command_hash() {
    let base = json!({"operation":"apply-template","entry":entry(),"profileId":"default"});
    let omitted = input(base.clone());
    let mut explicit = base.clone();
    explicit["additionalEntries"] = json!([]);
    explicit["templates"] = json!([]);
    let explicit = input(explicit);
    validate_owner_library_submit(&explicit).unwrap();
    assert_ne!(
        command_hash(&omitted).unwrap(),
        command_hash(&explicit).unwrap()
    );
    let mut combined = base;
    combined["templates"] = json!([{"id":"gallery","source":"retained template"}]);
    let (response, input) = fixture(combined);
    validate_owner_library_submit(&input).unwrap();
    validate_submission(&response, &input).unwrap();
    let mut changed = serde_json::to_value(&input).unwrap();
    changed["mutation"]["templates"][0]["source"] = json!("changed");
    let changed = serde_json::from_value(changed).unwrap();
    assert!(validate_submission(&response, &changed).is_err());
    let mut bad = serde_json::to_value(input).unwrap();
    bad["mutation"]["entry"]["proof"] = json!({});
    assert!(serde_json::from_value::<OwnerLibrarySubmit>(bad).is_err());
}

#[test]
fn canonical_hash_interoperates_with_ecmascript_floats_and_unicode_scalar_key_order() {
    let value:Value=serde_json::from_str(r#"{"2":1e-6,"10":1.0,"a":[1e21,1e-7,-0.0,1e20,18446744073709551615],"𐀀":"supplementary","":"bmp"}"#).unwrap();
    let mut encoded = String::new();
    canonical(&value, &mut encoded).unwrap();
    assert_eq!(
        encoded,
        r#"{"10":1,"2":0.000001,"a":[1e+21,1e-7,0,100000000000000000000,18446744073709551615],"":"bmp","𐀀":"supplementary"}"#
    );
    assert_eq!(
        digest(&value).unwrap(),
        tiktools_control_api::modules::automation_library::owner_library_digest(&value)
    );
    let mut source = entry();
    source["actions"][0]["config"]["floats"] = value;
    let input = input(json!({"operation":"apply-template","entry":source,"profileId":"default"}));
    assert_eq!(
        command_hash(&input).unwrap(),
        tiktools_control_api::modules::automation_library::owner_library_command_hash(&input)
    );
}
