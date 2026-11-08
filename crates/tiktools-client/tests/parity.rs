//! Parity: the typed client must cover the control registry exactly.
//!
//! `ControlApi::register_all` is the single source of truth for the
//! control surface. This test fails CI when a registered method has no
//! typed wrapper, or when the client covers a name the registry does
//! not know (renamed or removed RPC method).

use std::collections::BTreeSet;

use tiktools_client::TikToolsClient;
use tiktools_control_api::{ControlApi, ControlRouter};

#[test]
fn client_covers_every_registered_method() {
    let mut router = ControlRouter::new();
    ControlApi::register_all(&mut router);
    let metadata = router.metadata();
    let registered: BTreeSet<&str> = metadata.iter().map(|meta| meta.name.as_str()).collect();
    let covered: BTreeSet<&str> = TikToolsClient::covered_methods().into_iter().collect();
    assert!(
        !registered.is_empty(),
        "registry must not be empty; register_all regressed"
    );
    let missing: Vec<&str> = registered.difference(&covered).copied().collect();
    let extra: Vec<&str> = covered.difference(&registered).copied().collect();
    assert!(
        missing.is_empty(),
        "typed client is missing methods: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "typed client covers unregistered methods: {extra:?}"
    );
    assert_eq!(
        registered.len(),
        covered.len(),
        "coverage count drifted from the registry"
    );
}

#[test]
fn validate_covers_every_registered_method() {
    // Every covered method must have a validation arm: `null` never
    // satisfies a params struct, so anything but `method_not_found`
    // proves the arm exists and type-checks.
    for method in TikToolsClient::covered_methods() {
        let outcome = tiktools_client::validate_params(method, &serde_json::Value::Null);
        assert!(
            outcome.is_err(),
            "null params must not validate for {method}"
        );
        assert_ne!(
            outcome.expect_err("checked above").code,
            "method_not_found",
            "missing validation arm for {method}"
        );
    }
    let unknown = tiktools_client::validate_params("no.such.method", &serde_json::json!({}));
    assert_eq!(
        unknown.expect_err("unknown method must fail").code,
        "method_not_found"
    );
}

#[test]
fn validate_accepts_well_formed_params() {
    assert!(
        tiktools_client::validate_params("plugins.get", &serde_json::json!({"pluginId": "x"}))
            .is_ok()
    );
    assert!(tiktools_client::validate_params("automation.list", &serde_json::json!({})).is_ok());
    assert!(tiktools_client::validate_params(
        "automation.review",
        &serde_json::json!({"kind":"event","id":"reviewed"})
    )
    .is_ok());
    assert_eq!(
        tiktools_client::validate_params("automation.review", &serde_json::json!({}))
            .expect_err("review requires an id")
            .code,
        "invalid_params"
    );
    assert!(tiktools_client::validate_params(
        "automation.fire",
        &serde_json::json!({"trigger": "tiktok.gift"})
    )
    .is_ok());
    assert!(tiktools_client::validate_params(
        "points.adjust",
        &serde_json::json!({"uniqueId": "x", "delta": 1.5})
    )
    .is_ok());
    let bad = tiktools_client::validate_params("plugins.get", &serde_json::json!({}));
    assert_eq!(
        bad.expect_err("missing pluginId must fail").code,
        "invalid_params"
    );
}

#[test]
fn owner_adoption_validation_uses_the_host_journal_params_without_client_command_metadata() {
    for method in ["automation.adoption.get", "automation.adoption.resolve"] {
        assert!(tiktools_client::validate_params(method, &serde_json::json!({})).is_ok());
        assert_eq!(
            tiktools_client::validate_params(
                method,
                &serde_json::json!({"commandId":"caller-chosen"})
            )
            .expect_err("native get and resolve accept no caller metadata")
            .code,
            "invalid_params"
        );
    }
    let submit = serde_json::json!({
        "hostId":"owner-host","hostInstanceId":"owner-instance","kind":"event","id":"draft",
        "expectedRevision":"1","acknowledgeAuthority":true,
        "reviewedVersions":[{"kind":"automation","scope":"event","id":"draft","revision":"1"}]
    });
    assert!(tiktools_client::validate_params("automation.adoption.submit", &submit).is_ok());
    for required in [
        "hostId",
        "hostInstanceId",
        "kind",
        "id",
        "expectedRevision",
        "reviewedVersions",
        "acknowledgeAuthority",
    ] {
        let mut missing = submit.clone();
        missing.as_object_mut().unwrap().remove(required);
        assert_eq!(
            tiktools_client::validate_params("automation.adoption.submit", &missing)
                .expect_err("native owner submit requires complete reviewed host binding")
                .code,
            "invalid_params",
            "{required}"
        );
    }
    let mut chosen_id = submit;
    chosen_id["commandId"] = "caller-chosen".into();
    assert_eq!(
        tiktools_client::validate_params("automation.adoption.submit", &chosen_id)
            .expect_err("only the host chooses adoption command IDs")
            .code,
        "invalid_params"
    );
    assert!(tiktools_client::validate_params(
        "automation.adoption.review_unknown",
        &serde_json::json!({"commandId":"owner-adopt-original","acknowledgeUnknown":true})
    )
    .is_ok());
    assert_eq!(
        tiktools_client::validate_params(
            "automation.adoption.review_unknown",
            &serde_json::json!({"commandId":"owner-adopt-original"})
        )
        .expect_err("unknown review requires explicit acknowledgement")
        .code,
        "invalid_params"
    );
}

#[test]
fn manifest_preview_requires_only_a_saved_family_and_id() {
    let valid = serde_json::json!({"kind":"event","id":"reviewed"});
    assert!(tiktools_client::validate_params("automation.manifest.get", &valid).is_ok());
    for params in [
        serde_json::json!({"id":"reviewed"}),
        serde_json::json!({"kind":"events","id":"reviewed"}),
        serde_json::json!({"kind":"event","id":"reviewed","record":{}}),
        serde_json::json!({"kind":"event","id":"reviewed","principalId":"caller"}),
        serde_json::json!({"kind":"event","id":"reviewed","grant":{}}),
        serde_json::json!({"kind":"event","id":"reviewed","commandId":"chosen"}),
    ] {
        assert_eq!(
            tiktools_client::validate_params("automation.manifest.get", &params)
                .unwrap_err()
                .code,
            "invalid_params"
        );
    }
}

#[test]
fn approval_requires_saved_review_and_explicit_plan_without_caller_authority() {
    let activate = serde_json::json!({
        "kind":"event","id":"draft","manifestDigest":"a".repeat(64),
        "reviewedVersions":[{"kind":"automation","scope":"event","id":"draft","revision":"1"}],
        "enablePlan":[{"kind":"event","id":"draft","enabled":true}],
        "acknowledgeEffects":true
    });
    assert!(tiktools_client::validate_params("automation.approval.activate", &activate).is_ok());
    for required in [
        "kind",
        "id",
        "manifestDigest",
        "reviewedVersions",
        "enablePlan",
        "acknowledgeEffects",
    ] {
        let mut missing = activate.clone();
        missing.as_object_mut().unwrap().remove(required);
        assert_eq!(
            tiktools_client::validate_params("automation.approval.activate", &missing)
                .unwrap_err()
                .code,
            "invalid_params"
        );
    }
    for field in [
        "record",
        "source",
        "principalId",
        "sessionId",
        "provenance",
        "grant",
        "permit",
        "subjects",
        "effects",
        "expectedRevision",
        "commandId",
    ] {
        let mut extra = activate.clone();
        extra[field] = serde_json::json!({});
        assert_eq!(
            tiktools_client::validate_params("automation.approval.activate", &extra)
                .unwrap_err()
                .code,
            "invalid_params",
            "{field}"
        );
    }
    let mut nested = activate;
    nested["enablePlan"][0]["provenance"] = serde_json::json!({});
    assert_eq!(
        tiktools_client::validate_params("automation.approval.activate", &nested)
            .unwrap_err()
            .code,
        "invalid_params"
    );
    let revoke = serde_json::json!({"kind":"event","id":"draft","acknowledgeRevocation":true});
    assert!(tiktools_client::validate_params("automation.approval.revoke", &revoke).is_ok());
    for params in [
        serde_json::json!({"kind":"event","id":"draft"}),
        serde_json::json!({"kind":"event","id":"draft","acknowledgeRevocation":true,"approvalGeneration":"chosen"}),
    ] {
        assert_eq!(
            tiktools_client::validate_params("automation.approval.revoke", &params)
                .unwrap_err()
                .code,
            "invalid_params"
        );
    }
}

#[test]
fn native_approval_chooses_ids_and_separates_activation_from_revocation_proofs() {
    for method in ["automation.approval.get", "automation.approval.resolve"] {
        assert!(tiktools_client::validate_params(method, &serde_json::json!({})).is_ok());
        assert!(tiktools_client::validate_params(
            method,
            &serde_json::json!({"commandId":"chosen"})
        )
        .is_err());
    }
    let review = serde_json::json!({"kind":"event","id":"draft"});
    assert!(tiktools_client::validate_params("automation.approval.review", &review).is_ok());
    let activate = serde_json::json!({"operation":"activate","hostId":"host","hostInstanceId":"instance","kind":"event","id":"draft","expectedRevision":"1",
        "reviewedVersions":[{"kind":"automation","scope":"event","id":"draft","revision":"1"}],
        "manifestDigest":"a".repeat(64),"enablePlan":[{"kind":"event","id":"draft","enabled":true}],"acknowledgeEffects":true});
    let revoke = serde_json::json!({"operation":"revoke","hostId":"host","hostInstanceId":"instance","kind":"event","id":"draft","expectedRevision":"2","acknowledgeRevocation":true});
    for input in [&activate, &revoke] {
        assert!(tiktools_client::validate_params("automation.approval.submit", input).is_ok());
        for field in [
            "commandId",
            "record",
            "principalId",
            "subjects",
            "authority",
            "permit",
        ] {
            let mut forged = (*input).clone();
            forged[field] = serde_json::json!({});
            assert!(
                tiktools_client::validate_params("automation.approval.submit", &forged).is_err(),
                "{field}"
            );
        }
    }
    let mut confused = revoke;
    confused["manifestDigest"] = activate["manifestDigest"].clone();
    assert!(tiktools_client::validate_params("automation.approval.submit", &confused).is_err());
    let mut absent = activate;
    absent.as_object_mut().unwrap().remove("enablePlan");
    assert!(tiktools_client::validate_params("automation.approval.submit", &absent).is_err());
    assert!(tiktools_client::validate_params(
        "automation.approval.review_unknown",
        &serde_json::json!({"commandId":"original","acknowledgeUnknown":true})
    )
    .is_ok());
    assert!(tiktools_client::validate_params(
        "automation.approval.review_unknown",
        &serde_json::json!({"commandId":"original"})
    )
    .is_err());
}

#[test]
fn native_definition_validation_accepts_each_zero_revision_union_without_caller_metadata() {
    for method in ["automation.definition.get", "automation.definition.resolve"] {
        assert!(tiktools_client::validate_params(method, &serde_json::json!({})).is_ok());
        for extra in [
            serde_json::json!({"commandId":"caller-chosen"}),
            serde_json::json!({"expectedRevision":"0"}),
            serde_json::json!({"hostInstanceId":"instance"}),
        ] {
            assert_eq!(
                tiktools_client::validate_params(method, &extra)
                    .unwrap_err()
                    .code,
                "invalid_params"
            );
        }
    }
    for kind in ["action", "event"] {
        for operation in ["create", "update", "delete", "enable", "disable"] {
            let mut input = serde_json::json!({
                "operation":operation,"kind":kind,"expectedRevision":"0",
                "hostId":"owner-host","hostInstanceId":"owner-instance"
            });
            if operation != "create" {
                input["id"] = "definition".into();
            }
            if matches!(operation, "create" | "update") {
                input["record"] = serde_json::json!({"id":"definition","name":"Definition"});
            }
            assert!(
                tiktools_client::validate_params("automation.definition.submit", &input).is_ok(),
                "{kind} {operation} accepts the canonical zero base, including missing delete/update"
            );
            for required in input.as_object().unwrap().keys() {
                let mut missing = input.clone();
                missing.as_object_mut().unwrap().remove(required);
                assert_eq!(
                    tiktools_client::validate_params("automation.definition.submit", &missing)
                        .unwrap_err()
                        .code,
                    "invalid_params",
                    "{operation}: missing {required}"
                );
            }
            for field in [
                "commandId",
                "meta",
                "proof",
                "candidate",
                "authority",
                "provenance",
                "principalId",
                "permit",
                "reviewedVersions",
                "acknowledgeEffects",
            ] {
                let mut extra = input.clone();
                extra[field] = serde_json::json!({});
                assert_eq!(
                    tiktools_client::validate_params("automation.definition.submit", &extra)
                        .unwrap_err()
                        .code,
                    "invalid_params",
                    "{operation}: caller cannot supply {field}"
                );
            }
            let mut confused = input.clone();
            if operation == "create" {
                confused["id"] = "definition".into();
            } else if matches!(operation, "delete" | "enable" | "disable") {
                confused["record"] = serde_json::json!({"id":"definition"});
            } else {
                confused["deleted"] = true.into();
            }
            assert!(
                tiktools_client::validate_params("automation.definition.submit", &confused)
                    .is_err()
            );
            input["expectedRevision"] = 0.into();
            assert_eq!(
                tiktools_client::validate_params("automation.definition.submit", &input)
                    .unwrap_err()
                    .code,
                "invalid_params",
                "revisions are decimal strings"
            );
        }
    }
    let review = serde_json::json!({
        "commandId":"owner-definition-00000000000000000000000000000001",
        "acknowledgeUnknown":true
    });
    assert!(
        tiktools_client::validate_params("automation.definition.review_unknown", &review).is_ok()
    );
    for required in ["commandId", "acknowledgeUnknown"] {
        let mut missing = review.clone();
        missing.as_object_mut().unwrap().remove(required);
        assert_eq!(
            tiktools_client::validate_params("automation.definition.review_unknown", &missing)
                .unwrap_err()
                .code,
            "invalid_params"
        );
    }
    let mut extra = review;
    extra["record"] = serde_json::json!({"id":"definition"});
    assert_eq!(
        tiktools_client::validate_params("automation.definition.review_unknown", &extra)
            .unwrap_err()
            .code,
        "invalid_params"
    );
}
