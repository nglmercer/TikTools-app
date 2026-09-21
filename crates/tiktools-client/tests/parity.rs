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
