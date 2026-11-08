use serde_json::json;
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RpcId, RpcRequest};
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

#[tokio::test]
async fn owner_administration_uses_the_shared_store_and_never_grants_remote_administration() {
    let home = std::env::temp_dir().join(format!("tt-collaboration-api-{}", std::process::id()));
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.join("builtins"));
    let core = Arc::new(AppCore::new(Arc::new(Emitter)));
    let api = ControlApi::new(core.clone());
    let request = |method: &str, params| RpcRequest::new(RpcId::Number(1), method, params);
    let response = api
        .execute(request("collaboration.status", json!({})))
        .await;
    #[cfg(not(feature = "persistence"))]
    {
        assert_eq!(response.error.unwrap().code, "capability_unavailable");
        assert!(!home.join("data/collaboration/collaboration.db").exists());
    }
    #[cfg(feature = "persistence")]
    {
        use tiktools_core::collaboration::{hash_secret, ClientKind, Collect, Redeem};
        let status = response.result.unwrap();
        assert_eq!(status["configuration"]["enabled"], false);
        assert_eq!(
            api.execute(request(
                "collaboration.configure",
                json!({"enabled":true,"advertisedOrigin":"http://127.0.0.1:8788"})
            ))
            .await
            .error
            .unwrap()
            .code,
            "unsupported_remote"
        );
        core.set_collaboration_gateway_available(true);
        assert!(api
            .execute(request(
                "collaboration.configure",
                json!({"enabled":true,"advertisedOrigin":"http://public.example"})
            ))
            .await
            .error
            .is_some());
        let configured = api
            .execute(request(
                "collaboration.configure",
                json!({"enabled":true,"advertisedOrigin":"http://127.0.0.1:8788"}),
            ))
            .await;
        assert!(configured.is_ok());
        let mut checked = configured.result.unwrap()["configuration"].clone();
        checked["expectedRevision"] = json!(
            core.collaboration()
                .unwrap()
                .status()
                .unwrap()
                .configuration_revision
        );
        assert!(api
            .execute(request("collaboration.configure", checked.clone()))
            .await
            .is_ok());
        checked["enabled"] = json!(false);
        assert_eq!(
            api.execute(request("collaboration.configure", checked))
                .await
                .error
                .unwrap()
                .code,
            "revision_conflict"
        );
        assert!(
            core.collaboration()
                .unwrap()
                .status()
                .unwrap()
                .configuration
                .enabled
        );
        let invitation = api
            .execute(request("collaboration.invites.create", json!({})))
            .await
            .result
            .unwrap();
        let service = core.collaboration().unwrap();
        let exchange = "a".repeat(64);
        let pending = service
            .redeem(
                Redeem {
                    invite_id: invitation["inviteId"].as_str().unwrap().into(),
                    secret: invitation["secret"].as_str().unwrap().into(),
                    client_label: "PC B".into(),
                    client_kind: ClientKind::Desktop,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "loopback-fixture",
            )
            .unwrap();
        let requests = api
            .execute(request("collaboration.requests.list", json!({})))
            .await
            .result
            .unwrap();
        assert_eq!(requests["requests"].as_array().unwrap().len(), 1);
        assert!(api
            .execute(request(
                "collaboration.requests.approve",
                json!({"id":pending.request_id})
            ))
            .await
            .is_ok());
        let credentials = service
            .collect(
                Collect {
                    request_id: pending.request_id,
                    exchange_secret: exchange,
                },
                "loopback-fixture",
            )
            .unwrap();
        let authority = service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap();
        let session = tiktools_control_api::RemoteSession::from_authorization(authority).unwrap();
        let context = tiktools_control_api::RequestContext::Remote(session);
        assert!(api
            .execute_with_context(&context, request("live.status", json!({})))
            .await
            .is_ok());
        for name in [
            "collaboration.status",
            "collaboration.configure",
            "collaboration.invites.create",
            "collaboration.requests.approve",
            "collaboration.sessions.list",
            "collaboration.sessions.revoke",
            "collaboration.principals.list",
            "collaboration.principals.get",
            "collaboration.grants.set",
            "collaboration.audit.list",
        ] {
            let result = api
                .execute_with_context(&context, request(name, json!({"role":"owner"})))
                .await;
            assert_eq!(result.error.unwrap().code, "forbidden", "{name}");
        }
        let principals = api
            .execute(request("collaboration.principals.list", json!({})))
            .await
            .result
            .unwrap();
        assert_eq!(principals["principals"][0]["grantRevision"], "1");
        let principal = api
            .execute(request(
                "collaboration.principals.get",
                json!({"id":credentials.principal_id}),
            ))
            .await
            .result
            .unwrap();
        assert_eq!(principal, principals["principals"][0]);
        let params = json!({"principalId":credentials.principal_id,"expectedRevision":"1","grant":{"capabilities":[]}});
        assert!(api
            .execute(request("collaboration.grants.set", params.clone()))
            .await
            .is_ok());
        assert_eq!(
            api.execute_with_context(&context, request("live.status", json!({})))
                .await
                .error
                .unwrap()
                .code,
            "forbidden"
        );
        assert_eq!(
            api.execute(request("collaboration.grants.set", params))
                .await
                .error
                .unwrap()
                .code,
            "revision_conflict"
        );
        assert!(api
            .execute(request(
                "collaboration.sessions.revoke",
                json!({"id":credentials.session_id})
            ))
            .await
            .is_ok());
        assert!(service
            .authenticate(&credentials.access_token, false, false)
            .is_err());
        assert_eq!(
            api.execute_with_context(&context, request("live.status", json!({})))
                .await
                .error
                .unwrap()
                .code,
            "unauthorized"
        );
        let page = api
            .execute(request("collaboration.audit.list", json!({"limit":2})))
            .await
            .result
            .unwrap();
        assert_eq!(page["entries"].as_array().unwrap().len(), 2);
        assert!(page["nextAfter"].as_i64().unwrap() > 0);
        let exhausted = api
            .execute(request("collaboration.audit.list", json!({"after":999999})))
            .await
            .result
            .unwrap();
        assert_eq!(exhausted["nextAfter"], 999999);
        assert!(exhausted["entries"].as_array().unwrap().is_empty());
        let redacted = api
            .execute(request("collaboration.invites.list", json!({})))
            .await
            .result
            .unwrap()
            .to_string();
        assert!(!redacted.contains(invitation["secret"].as_str().unwrap()));
        assert!(!redacted.contains(&credentials.access_token));
    }
    core.shutdown().await;
    drop(api);
    drop(core);
    std::fs::remove_dir_all(home).unwrap();
}
