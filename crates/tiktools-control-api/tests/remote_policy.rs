use std::{sync::Arc, time::Duration};

use serde_json::json;
use tiktools_control_api::{
    ControlApi, RemoteCapability, RemoteSession, RequestContext, RpcId, RpcRequest,
};
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

struct NullEmitter;
impl HostEmitter for NullEmitter {
    fn emit(&self, _: HostMessage) {}
}

#[tokio::test]
async fn remote_identity_is_host_supplied_and_policy_filters_calls_and_discovery() {
    let home = std::env::temp_dir().join(format!(
        "tiktools-remote-policy-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.join("builtins"));
    let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let api = ControlApi::new(core.clone());
    let session = RemoteSession::new(
        "moderator",
        "device-b",
        [RemoteCapability::LiveRead],
        Duration::from_secs(60),
    )
    .unwrap();
    let context = RequestContext::Remote(session.clone());
    let call = |method: &str, params| RpcRequest::new(RpcId::Number(1), method, params);

    assert!(api
        .execute_with_context(&context, call("live.status", json!({})))
        .await
        .is_ok());
    assert!(api
        .execute_with_context(
            &context,
            call("ui.feed.backfill", json!({"version":1,"after":0}))
        )
        .await
        .is_ok());

    // A claimed owner or grant in raw JSON cannot replace trusted context.
    for method in [
        "system.shutdown",
        "points.adjust",
        "app.state.get",
        "system.info",
        "not.registered",
    ] {
        let response = api
            .execute_value_with_context(
                &context,
                &json!({
                    "id":1, "method":method,
                    "context":{"role":"owner", "capabilities":["*"]},
                    "params":{"role":"owner", "uniqueId":"amy", "delta":999}
                }),
            )
            .await;
        assert_eq!(response.error.unwrap().code, "forbidden", "{method}");
    }
    assert!(!core.is_shutdown());

    let discovered = api
        .execute_with_context(&context, call("rpc.discover", json!({})))
        .await
        .result
        .unwrap();
    let methods: Vec<_> = discovered["methods"]
        .as_array()
        .unwrap()
        .iter()
        .map(|meta| meta["name"].as_str().unwrap())
        .collect();
    assert!(methods.contains(&"live.status"));
    assert!(methods.contains(&"rpc.schema"));
    assert!(!methods.contains(&"system.shutdown"));
    assert!(!methods.contains(&"system.info"));
    for method in ["system.shutdown", "not.registered"] {
        assert_eq!(
            api.execute_with_context(&context, call("rpc.schema", json!({"method":method})))
                .await
                .error
                .unwrap()
                .code,
            "forbidden"
        );
    }
    assert!(api
        .execute_with_context(
            &context,
            call("rpc.schema", json!({"method":"live.status"}))
        )
        .await
        .is_ok());

    // Dynamic grant changes affect every clone, not only new connections.
    session.set_capabilities([]);
    assert_eq!(
        api.execute_with_context(&context, call("live.status", json!({})))
            .await
            .error
            .unwrap()
            .code,
        "forbidden"
    );
    let discovered = api
        .execute_with_context(&context, call("rpc.discover", json!({})))
        .await
        .result
        .unwrap();
    assert_eq!(discovered["methods"].as_array().unwrap().len(), 4);
    session.revoke();
    assert_eq!(
        api.execute_with_context(&context, call("rpc.discover", json!({})))
            .await
            .error
            .unwrap()
            .code,
        "unauthorized"
    );

    // The original trusted-local path is unchanged.
    assert!(api.execute(call("system.ping", json!({}))).await.is_ok());
    let local = api
        .execute(call("rpc.discover", json!({})))
        .await
        .result
        .unwrap();
    assert!(local["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|meta| meta["name"] == "system.shutdown"));
    core.shutdown().await;
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn every_unclassified_registry_method_is_denied_and_only_completed_mutations_are_allowed() {
    let mut router = tiktools_control_api::ControlRouter::new();
    ControlApi::register_all(&mut router);
    for meta in router.metadata() {
        let policy = router.get(&meta.name).unwrap().remote_policy;
        if !matches!(
            meta.name.as_str(),
            "live.status"
                | "live.monitor.status"
                | "ui.feed.backfill"
                | "rpc.discover"
                | "rpc.schema"
                | "collaboration.hello"
                | "collaboration.me"
                | "collaboration.commands.get"
                | "collaboration.subscribe"
                | "collaboration.snapshot"
                | "collaboration.resume"
                | "workflows.list"
                | "workflows.get"
                | "workflows.disable"
                | "workflows.delete"
                | "points.adjust"
                | "points.viewer.get"
                | "points.leaderboard"
                | "points.info"
                | "points.view"
                | "automation.create"
                | "automation.update"
                | "automation.list"
                | "automation.get"
                | "automation.disable"
                | "automation.delete"
        ) {
            assert_eq!(
                policy,
                tiktools_control_api::policy::RemotePolicy::Deny,
                "{}",
                meta.name
            );
        }
        if meta.side_effect && policy != tiktools_control_api::policy::RemotePolicy::Deny {
            assert!(matches!(
                meta.name.as_str(),
                "workflows.disable"
                    | "workflows.delete"
                    | "points.adjust"
                    | "automation.disable"
                    | "automation.delete"
                    | "automation.create"
                    | "automation.update"
            ));
            assert!(meta.durable_commands);
            assert_eq!(
                meta.concurrency,
                Some(if meta.name == "points.adjust" {
                    tiktools_control_api::registry::ConcurrencyStrategy::Additive
                } else {
                    tiktools_control_api::registry::ConcurrencyStrategy::CompareAndSet
                })
            );
        }
    }
}
