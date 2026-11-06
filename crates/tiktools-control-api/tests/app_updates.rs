use serde_json::json;
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RpcId, RpcRequest};
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

struct NullEmitter;
impl HostEmitter for NullEmitter {
    fn emit(&self, _: HostMessage) {}
}

#[tokio::test]
async fn updater_rpc_is_registered_and_install_is_never_remotely_authorized() {
    let home = std::env::temp_dir().join(format!("tiktools-updater-api-{}", std::process::id()));
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.join("builtins"));
    let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let api = ControlApi::new(core.clone());
    let call = |method: &str, params| RpcRequest::new(RpcId::Number(1), method, params);
    let status = api.execute(call("app.update.status", json!({}))).await;
    assert_eq!(status.error.unwrap().code, "capability_unavailable");
    let install = api.execute(call("app.update.install", json!({}))).await;
    assert_eq!(install.error.unwrap().code, "capability_unavailable");
    let path = api
        .execute(call("app.update.install", json!({"path":"/tmp/evil"})))
        .await;
    assert_eq!(path.error.unwrap().code, "invalid_params");
    let url = api
        .execute(call(
            "app.update.check",
            json!({"channel":"stable","url":"https://evil.example"}),
        ))
        .await;
    assert_eq!(url.error.unwrap().code, "invalid_params");
    assert!(api.execute(call("system.ping", json!({}))).await.is_ok());
    let mut events = api.subscribe();
    core.events
        .publish_domain(tiktools_core::events::DomainEvent::AppUpdateStateChanged {
            state: json!({ "state": "downloading", "downloadedBytes": 12, "totalBytes": null }),
        });
    let envelope = events.recv().await.unwrap().to_envelope().unwrap();
    assert_eq!(envelope.topic, "app.update.stateChanged");
    assert_eq!(envelope.data["state"]["downloadedBytes"], 12);
    #[cfg(feature = "app-update")]
    {
        core.enable_desktop_app_updater();
        let status = api
            .execute(call("app.update.status", json!({})))
            .await
            .result
            .unwrap();
        assert_eq!(status["currentVersion"], env!("CARGO_PKG_VERSION"));
        assert_eq!(status["installSupported"], false);
        assert_eq!(status["state"], "idle");
        let download = api.execute(call("app.update.download", json!({}))).await;
        assert_eq!(
            download.error.unwrap().code,
            if status["checkSupported"] == true {
                "invalid_update_state"
            } else {
                "capability_unavailable"
            }
        );
    }
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(home);
}
