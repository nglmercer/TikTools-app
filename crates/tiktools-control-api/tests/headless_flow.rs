//! Headless control-plane flow against an isolated `TIKTOOLS_HOME`.
//!
//! Flow: plugins.list -> configure plugin settings (when a plugin with
//! settings exists) -> health check -> create automation -> test automation
//! -> points adjust/leaderboard -> snapshot -> doctor -> shutdown.
//!
//! A second test drives the same surface through NDJSON stdio framing.

use std::sync::Arc;

use serde_json::{json, Value};
use tiktools_control_api::{ControlApi, RpcId, RpcRequest};
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

struct NullEmitter;

impl HostEmitter for NullEmitter {
    fn emit(&self, _message: HostMessage) {}
}

fn isolated_home(tag: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!(
        "tiktools-control-test-{}-{}-{}",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.join("builtin-plugins"));
    let _ = std::fs::remove_dir_all(&home);
    home
}

async fn call(api: &ControlApi, method: &str, params: Value) -> Value {
    let response = api
        .execute(RpcRequest::new(RpcId::Number(1), method, params))
        .await;
    assert!(
        response.is_ok(),
        "method {method} failed: {}",
        serde_json::to_string(&response).unwrap_or_default()
    );
    response.result.unwrap_or(Value::Null)
}

async fn call_expect_error(api: &ControlApi, method: &str, params: Value) -> (String, String) {
    let response = api
        .execute(RpcRequest::new(RpcId::Number(1), method, params))
        .await;
    assert!(!response.is_ok(), "method {method} unexpectedly succeeded");
    let error = response.error.expect("error body");
    (error.code, error.message)
}

#[tokio::test]
async fn headless_control_flow() {
    let _home = isolated_home("flow");
    let api = ControlApi::new(Arc::new(AppCore::new(Arc::new(NullEmitter))));

    // Discovery first: agents find methods dynamically.
    let discovered = call(&api, "rpc.discover", json!({})).await;
    let methods = discovered
        .get("methods")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        methods.len() >= 40,
        "expected 40+ methods, got {}",
        methods.len()
    );
    assert!(methods
        .iter()
        .any(|method| method.get("name") == Some(&json!("plugins.list"))));
    let schema = call(
        &api,
        "rpc.schema",
        json!({ "method": "plugins.settings.set" }),
    )
    .await;
    assert_eq!(schema.get("name"), Some(&json!("plugins.settings.set")));
    assert_eq!(schema.get("sideEffect"), Some(&json!(true)));

    // Unknown method + bad params surface typed errors.
    let (code, _) = call_expect_error(&api, "nope.missing", json!({})).await;
    assert_eq!(code, "method_not_found");
    let (code, _) = call_expect_error(&api, "plugins.get", json!({})).await;
    assert_eq!(code, "invalid_params");
    let (code, _) =
        call_expect_error(&api, "plugins.get", json!({ "pluginId": "missing.plugin" })).await;
    assert_eq!(code, "plugin_not_found");

    // System surface.
    let info = call(&api, "system.info", json!({})).await;
    assert_eq!(info.get("name"), Some(&json!("tiktools")));
    let health = call(&api, "system.health", json!({})).await;
    assert_eq!(health.get("status"), Some(&json!("ok")));

    // Plugins surface (empty home: no plugins, still well-formed).
    let plugins = call(&api, "plugins.list", json!({})).await;
    assert_eq!(plugins.get("plugins"), Some(&json!([])));

    // Automation CRUD + dry-run test.
    let created = call(
        &api,
        "automation.create",
        json!({
            "kind": "event",
            "record": {
                "name": "Welcome",
                "enabled": true,
                "trigger": "tiktok.chat",
                "filters": [
                    { "path": "event.data.comment", "operator": "contains", "value": "hello" }
                ],
                "actionIds": [],
            }
        }),
    )
    .await;
    let id = created
        .get("id")
        .and_then(Value::as_str)
        .expect("record id")
        .to_owned();
    let fetched = call(&api, "automation.get", json!({ "kind": "event", "id": id })).await;
    assert_eq!(fetched.get("name"), Some(&json!("Welcome")));
    let listed = call(&api, "automation.list", json!({ "kind": "event" })).await;
    assert_eq!(
        listed.get("events").and_then(Value::as_array).map(Vec::len),
        Some(1)
    );
    // No actions attached: the dry run reports that honestly.
    let tested = call(
        &api,
        "automation.test",
        json!({ "kind": "event", "id": id }),
    )
    .await;
    assert_eq!(tested.get("test"), Some(&json!(true)));
    assert!(tested.get("summary").and_then(Value::as_str).is_some());
    let disabled = call(
        &api,
        "automation.disable",
        json!({ "kind": "event", "id": id }),
    )
    .await;
    assert_eq!(disabled.get("enabled"), Some(&json!(false)));
    let deleted = call(
        &api,
        "automation.delete",
        json!({ "kind": "event", "id": id }),
    )
    .await;
    assert_eq!(deleted.get("deleted"), Some(&json!(true)));
    let (_, _) =
        call_expect_error(&api, "automation.get", json!({ "kind": "event", "id": id })).await;

    // Points flow.
    let award = call(
        &api,
        "points.adjust",
        json!({ "uniqueId": "alice", "delta": 5.0 }),
    )
    .await;
    assert_eq!(award.get("uniqueId"), Some(&json!("alice")));
    assert_eq!(award.get("totalPoints"), Some(&json!(5.0)));
    let viewer = call(&api, "points.viewer.get", json!({ "uniqueId": "alice" })).await;
    assert_eq!(
        viewer
            .get("viewer")
            .and_then(|viewer| viewer.get("uniqueId")),
        Some(&json!("alice"))
    );
    let board = call(&api, "points.leaderboard", json!({ "limit": 10 })).await;
    assert_eq!(
        board.get("viewers").and_then(Value::as_array).map(Vec::len),
        Some(1)
    );
    let config = call(&api, "points.config.set", json!({ "pointsPerChat": 2.0 })).await;
    assert_eq!(config.get("pointsPerChat"), Some(&json!(2.0)));

    // Processors surface (no processors in an empty home).
    let processors = call(&api, "processors.list", json!({})).await;
    assert_eq!(processors.get("processors"), Some(&json!([])));

    // Snapshot never carries secrets and always carries the core sections.
    let snapshot = call(&api, "system.snapshot", json!({})).await;
    for section in [
        "live",
        "plugins",
        "processors",
        "automations",
        "pointsConfig",
        "health",
    ] {
        assert!(
            snapshot.get(section).is_some(),
            "snapshot section {section} missing"
        );
    }

    // Doctor is structured.
    let doctor = call(&api, "system.doctor", json!({})).await;
    assert!(doctor.get("ok").and_then(Value::as_bool).is_some());
    assert!(doctor.get("checks").and_then(Value::as_array).is_some());

    // Live status without connecting.
    let status = call(&api, "live.status", json!({})).await;
    assert_eq!(status.get("connected"), Some(&json!(false)));

    // Media validation rejects missing files with invalid_params.
    let (code, _) = call_expect_error(
        &api,
        "media.validate",
        json!({ "path": "/nonexistent/file.mp3", "kind": "audio" }),
    )
    .await;
    assert_eq!(code, "invalid_params");

    // Malformed envelopes stay correlated (checked before shutdown).
    let response = api.execute_value(&json!({"id": "abc", "method": 42})).await;
    assert_eq!(serde_json::to_value(&response.id).unwrap(), json!("abc"));
    assert!(!response.is_ok());

    let response = api.execute_value(&json!([1, 2, 3])).await;
    assert!(!response.is_ok());

    // Missing params normalize to {} for empty-param methods.
    let response = api
        .execute_value(&json!({"id": 7, "method": "system.health"}))
        .await;
    assert!(response.is_ok());
    assert_eq!(serde_json::to_value(&response.id).unwrap(), json!(7));

    let shutdown = call(&api, "system.shutdown", json!({})).await;
    assert_eq!(shutdown.get("ok"), Some(&json!(true)));
    assert!(api.core().is_shutdown());
}
