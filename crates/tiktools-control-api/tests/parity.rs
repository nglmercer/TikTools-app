//! WebView parity: every legacy `PageMessage` operation maps to a
//! Control API method (§16). The sample JSON proves the legacy type name
//! still parses (renames/removals fail here); the method list proves the
//! Control API exposes an equivalent (missing registrations fail below).
//! Add new legacy operations to this table when extending `PageMessage`.

use std::{collections::HashSet, sync::Arc};

use serde_json::{json, Value};
use tiktools_control_api::{ControlApi, RpcId, RpcRequest};
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

// Async-aware: the guard intentionally spans awaits so the isolated home
// stays stable for the whole test.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct NullEmitter;

impl HostEmitter for NullEmitter {
    fn emit(&self, _message: HostMessage) {}
}

/// (legacy `PageMessage` type, minimal valid sample, equivalent methods).
#[allow(clippy::too_many_lines)]
const PARITY: &[(&str, &str, &[&str])] = &[
    (
        "connect",
        r#"{"type":"connect","uniqueId":"alice","sessionCookie":"x"}"#,
        &["live.connect"],
    ),
    (
        "pick-live",
        r#"{"type":"pick-live","sessionCookie":"x"}"#,
        &["live.pick"],
    ),
    (
        "open-media-picker",
        r#"{"type":"open-media-picker","requestId":"r1"}"#,
        &["media.pick"],
    ),
    (
        "disconnect",
        r#"{"type":"disconnect"}"#,
        &["live.disconnect"],
    ),
    (
        "get-points-config",
        r#"{"type":"get-points-config"}"#,
        &["points.config.get"],
    ),
    (
        "update-points-config",
        r#"{"type":"update-points-config","config":{}}"#,
        &["points.config.set"],
    ),
    (
        "get-leaderboard",
        r#"{"type":"get-leaderboard"}"#,
        &["points.leaderboard"],
    ),
    (
        "reset-points",
        r#"{"type":"reset-points"}"#,
        &["points.reset"],
    ),
    (
        "adjust-points",
        r#"{"type":"adjust-points","uniqueId":"alice","delta":1.0}"#,
        &["points.adjust"],
    ),
    (
        "get-creator",
        r#"{"type":"get-creator"}"#,
        &["creators.get"],
    ),
    (
        "get-recent-creators",
        r#"{"type":"get-recent-creators"}"#,
        &["creators.recent"],
    ),
    (
        "get-app-state",
        r#"{"type":"get-app-state"}"#,
        &["app.state.get"],
    ),
    (
        "set-app-state",
        r#"{"type":"set-app-state","key":"k","value":"v"}"#,
        &["app.state.set"],
    ),
    (
        "clear-creator-history",
        r#"{"type":"clear-creator-history"}"#,
        &["creators.history.clear"],
    ),
    ("debug-gift", r#"{"type":"debug-gift"}"#, &["gifts.debug"]),
    (
        "get-automation-workflows",
        r#"{"type":"get-automation-workflows"}"#,
        &["workflows.list"],
    ),
    (
        "get-automation-nodes",
        r#"{"type":"get-automation-nodes"}"#,
        &["automation.nodes.list"],
    ),
    (
        "get-automation-context",
        r#"{"type":"get-automation-context"}"#,
        &["automation.context"],
    ),
    (
        "save-automation-workflow",
        r#"{"type":"save-automation-workflow","graph":{}}"#,
        &["workflows.save"],
    ),
    (
        "delete-automation-workflow",
        r#"{"type":"delete-automation-workflow","id":"w1"}"#,
        &["workflows.delete"],
    ),
    (
        "set-automation-workflow-enabled",
        r#"{"type":"set-automation-workflow-enabled","id":"w1","enabled":true}"#,
        &["workflows.enable", "workflows.disable"],
    ),
    (
        "analyze-automation-script",
        r#"{"type":"analyze-automation-script","nodeId":"n1","source":"x","offset":0}"#,
        &["automation.script.analyze"],
    ),
    (
        "get-gift-catalog",
        r#"{"type":"get-gift-catalog"}"#,
        &["gifts.list"],
    ),
    (
        "get-behavior",
        r#"{"type":"get-behavior"}"#,
        &["automation.snapshot", "automation.runs"],
    ),
    (
        "save-action",
        r#"{"type":"save-action","action":{}}"#,
        &["automation.create", "automation.update"],
    ),
    (
        "delete-action",
        r#"{"type":"delete-action","id":"a1"}"#,
        &["automation.delete"],
    ),
    (
        "set-action-enabled",
        r#"{"type":"set-action-enabled","id":"a1","enabled":true}"#,
        &["automation.enable", "automation.disable"],
    ),
    (
        "test-action",
        r#"{"type":"test-action","action":{}}"#,
        &["automation.test"],
    ),
    (
        "save-event",
        r#"{"type":"save-event","event":{}}"#,
        &["automation.create", "automation.update"],
    ),
    (
        "delete-event",
        r#"{"type":"delete-event","id":"e1"}"#,
        &["automation.delete"],
    ),
    (
        "set-event-enabled",
        r#"{"type":"set-event-enabled","id":"e1","enabled":false}"#,
        &["automation.enable", "automation.disable"],
    ),
    (
        "test-event",
        r#"{"type":"test-event","event":{}}"#,
        &["automation.test"],
    ),
    (
        "set-plugin-install",
        r#"{"type":"set-plugin-install","id":"p","installed":true}"#,
        &["plugins.install.set"],
    ),
    (
        "install-plugin-package",
        r#"{"type":"install-plugin-package","path":"/tmp/x.plugin"}"#,
        &["plugins.install"],
    ),
    (
        "uninstall-plugin-package",
        r#"{"type":"uninstall-plugin-package","id":"p"}"#,
        &["plugins.uninstall"],
    ),
    (
        "set-plugin-enabled",
        r#"{"type":"set-plugin-enabled","id":"p","enabled":true}"#,
        &["plugins.enable", "plugins.disable"],
    ),
    (
        "get-plugin-settings",
        r#"{"type":"get-plugin-settings","id":"p"}"#,
        &["plugins.settings.get"],
    ),
    (
        "save-plugin-settings",
        r#"{"type":"save-plugin-settings","id":"p","values":{"port":8080}}"#,
        &["plugins.settings.set"],
    ),
    (
        "get-action-options",
        r#"{"type":"get-action-options","source":"plugin-action-options:tts.speak:voice"}"#,
        &["plugins.options"],
    ),
    (
        "execute-plugin-action",
        r#"{"type":"execute-plugin-action","actionType":"tts.speak","config":{"text":"hi"}}"#,
        &["plugins.action.execute"],
    ),
    (
        "test-plugin-connection",
        r#"{"type":"test-plugin-connection","id":"p"}"#,
        &["plugins.health"],
    ),
    (
        "provision-plugin-token",
        r#"{"type":"provision-plugin-token","id":"p","username":"u","password":"pw"}"#,
        &["plugins.token.provision"],
    ),
    (
        "test-processor",
        r#"{"type":"test-processor","pluginId":"p","processorId":"q","event":{}}"#,
        &["processors.test"],
    ),
    (
        "get-processor-status",
        r#"{"type":"get-processor-status"}"#,
        &["processors.status"],
    ),
    (
        "get-analytics-summary",
        r#"{"type":"get-analytics-summary"}"#,
        &["analytics.summary"],
    ),
];

#[tokio::test]
async fn every_legacy_operation_has_a_control_api_equivalent() {
    let _env = ENV_LOCK.lock().await;
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let home = std::env::temp_dir().join(format!(
        "tiktools-parity-test-{}-{unique}",
        std::process::id()
    ));
    std::env::set_var("TIKTOOLS_HOME", &home);

    // Every legacy sample still parses and reports its documented type.
    for (legacy, sample, _) in PARITY {
        let message = tiktools_core::ipc::messages::PageMessage::parse(sample)
            .unwrap_or_else(|error| panic!("legacy `{legacy}` no longer parses: {error}"));
        assert_eq!(message.type_name(), *legacy);
    }

    let api = ControlApi::new(Arc::new(AppCore::new(Arc::new(NullEmitter))));
    let response = api
        .execute(RpcRequest::new(RpcId::Number(1), "rpc.discover", json!({})))
        .await;
    assert!(response.is_ok(), "rpc.discover failed: {response:?}");
    let discovered: Value =
        serde_json::to_value(&response.result).expect("discover result serializes");
    let methods: Vec<&Value> = discovered
        .get("methods")
        .and_then(Value::as_array)
        .map(|methods| methods.iter().collect())
        .unwrap_or_default();
    let names: HashSet<&str> = methods
        .iter()
        .filter_map(|method| method.get("name").and_then(Value::as_str))
        .collect();
    assert!(
        names.len() >= 60,
        "method registry shrank unexpectedly: {} methods",
        names.len()
    );

    for (legacy, _, equivalents) in PARITY {
        for method in *equivalents {
            assert!(
                names.contains(method),
                "legacy `{legacy}` has no Control API equivalent: `{method}` is not registered"
            );
            let meta = methods
                .iter()
                .find(|meta| meta.get("name").and_then(Value::as_str) == Some(*method))
                .expect("method metadata present");
            assert!(
                !meta
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .is_empty(),
                "{method} needs a description for agent discovery"
            );
            for schema in ["paramsSchema", "resultSchema"] {
                assert!(
                    meta.get(schema).and_then(Value::as_object).is_some(),
                    "{method} needs a {schema} for agent discovery"
                );
            }
        }
    }

    let _ = std::fs::remove_dir_all(&home);
}
