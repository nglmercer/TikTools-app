use super::*;
use serde_json::Value;

#[tokio::test]
async fn typed_runtime_variables_are_atomic_owner_only_operations() {
    use serde_json::json;
    struct Emitter;
    impl tiktools_core::HostEmitter for Emitter {
        fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
    }
    let root = std::env::temp_dir().join(format!(
        "tt-typed-variables-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let paths = tiktools_core::paths::AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.clone(),
    };
    paths.ensure_directories().unwrap();
    let core = Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(tiktools_core::NoopMediaHost),
        paths,
    ));
    let mut router = ControlRouter::new();
    ControlApi::register_all(&mut router);
    let api = ControlApi {
        core: core.clone(),
        router,
    };
    let list = api
        .execute(RpcRequest::new(
            RpcId::Number(1),
            "globals.typed.list",
            json!({}),
        ))
        .await
        .result
        .unwrap();
    let params = json!({"expectedRevision":list["revision"],"variables":{"port":{"type":"number","value":46665},"ordinary":{"type":"secret","value":"rpc-local-private"}}});
    let saved = api
        .execute(RpcRequest::new(
            RpcId::Number(2),
            "globals.typed.replace",
            params.clone(),
        ))
        .await
        .result
        .unwrap();
    assert_eq!(saved["variables"]["port"]["value"], 46665);
    assert_eq!(saved["variables"]["ordinary"]["type"], "secret");
    let stale = api
        .execute(RpcRequest::new(
            RpcId::Number(3),
            "globals.typed.replace",
            params.clone(),
        ))
        .await
        .error
        .unwrap();
    assert_eq!(stale.code, "conflict");
    let session = RemoteSession::new(
        "peer",
        "peer-session",
        [
            RemoteCapability::AutomationRead,
            RemoteCapability::AutomationEdit,
            RemoteCapability::WorkflowsRead,
            RemoteCapability::WorkflowsEdit,
        ],
        Duration::from_secs(60),
    )
    .unwrap();
    let operations = vec![
        ("globals.typed.list", json!({})),
        ("globals.typed.replace", params),
    ];
    #[cfg(feature = "cloud-auth")]
    let operations = {
        let mut operations = operations;
        operations.extend([
            ("sync.v2.variables.review", json!({})),
            ("sync.v2.variables.fetch", json!({})),
            (
                "sync.v2.variables.send",
                json!({"review":"ordinary-comparison-reference"}),
            ),
            (
                "sync.v2.variables.withdraw",
                json!({"review":"ordinary-comparison-reference"}),
            ),
            (
                "sync.v2.variables.apply",
                json!({"review":"ordinary-comparison-reference"}),
            ),
        ]);
        operations
    };
    for (method, params) in operations {
        let denied = api
            .execute_with_context(
                &RequestContext::Remote(session.clone()),
                RpcRequest::new(RpcId::Number(4), method, params),
            )
            .await;
        assert!(denied.result.is_none());
        assert_eq!(denied.error.unwrap().code, "forbidden");
    }
    assert_eq!(
        core.runtime_variables_list().unwrap().revision,
        saved["revision"].as_str().unwrap()
    );
    core.shutdown().await;
    drop(api);
    drop(core);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cloud_login_deadlines_leave_time_for_callback_and_server_response() {
    assert_eq!(
        request_timeout_for("cloud.login.complete"),
        Duration::from_secs(240)
    );
    assert_eq!(
        client_timeout_for("cloud.login.complete"),
        Duration::from_secs(270)
    );
    for other in [
        "cloud.login.begin",
        "cloud.login.cancel",
        "cloud.login.complete.extra",
        "",
    ] {
        assert_eq!(request_timeout_for(other), REQUEST_TIMEOUT);
        assert_eq!(client_timeout_for(other), REQUEST_TIMEOUT);
    }
}

#[tokio::test]
async fn remote_storage_failures_omit_owner_diagnostics_but_local_failures_keep_them() {
    struct Emitter;
    impl tiktools_core::HostEmitter for Emitter {
        fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
    }
    let root = std::env::temp_dir().join(format!(
        "tt-private-error-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let paths = tiktools_core::paths::AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.clone(),
    };
    paths.ensure_directories().unwrap();
    let core = Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(tiktools_core::NoopMediaHost),
        paths,
    ));
    let mut router = ControlRouter::new();
    router.register_typed::<modules::Empty, Value, _, _>(
        "test.failure",
        "fixture",
        false,
        |_, _| async {
            let mut error =
                ApiError::internal("SQL trigger failed at /private/owner.db: secret-marker");
            error.data = Some(serde_json::json!({"diagnostic":"secret-marker"}));
            Err(error)
        },
    );
    router.set_remote_policy("test.failure", policy::RemotePolicy::Authenticated);
    let api = ControlApi {
        core: core.clone(),
        router,
    };
    let session = RemoteSession::new("remote", "session", [], Duration::from_secs(60)).unwrap();
    let call = || RpcRequest::new(RpcId::Number(1), "test.failure", serde_json::json!({}));
    let remote = api
        .execute_with_context(&RequestContext::Remote(session), call())
        .await
        .error
        .unwrap();
    assert_eq!(remote.code, "internal");
    assert!(remote.data.is_none());
    assert!(!remote.message.contains("secret-marker"));
    assert!(!remote.message.contains("/private"));
    let local = api.execute(call()).await.error.unwrap();
    assert!(local.message.contains("secret-marker"));
    assert!(local.data.is_some());
    core.shutdown().await;
    drop(api);
    drop(core);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn plugin_and_automation_topics_map_to_event_notifications() {
    // The stdio/IPC/WebView transports all send `event_notification`
    // verbatim, so this envelope is what an IPC subscriber receives.
    for event in [
        tiktools_core::events::DomainEvent::PluginEvent {
            plugin_id: "hotkeys".to_owned(),
            event_type: "hotkey.pressed".to_owned(),
            event: serde_json::json!({"type": "hotkey.pressed"}),
        },
        tiktools_core::events::DomainEvent::PluginStatus {
            plugin_id: "hotkeys".to_owned(),
            status: serde_json::json!({}),
        },
        tiktools_core::events::DomainEvent::AutomationRunsChanged { runs: vec![] },
        tiktools_core::events::DomainEvent::AutomationRunCompleted {
            run: serde_json::json!({}),
        },
    ] {
        let notification = event_notification(&event);
        assert_eq!(notification["method"], "event");
        assert_eq!(notification["params"]["topic"], event.topic());
    }
    let notification = event_notification(&tiktools_core::events::DomainEvent::PluginEvent {
        plugin_id: "hotkeys".to_owned(),
        event_type: "hotkey.pressed".to_owned(),
        event: serde_json::json!({"type": "hotkey.pressed"}),
    });
    assert_eq!(notification["params"]["data"]["pluginId"], "hotkeys");
    assert_eq!(
        notification["params"]["data"]["eventType"],
        "hotkey.pressed"
    );
}
