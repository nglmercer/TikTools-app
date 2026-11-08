#![cfg(feature = "persistence")]
//! Manifest previews are owner reads, not grants, approvals or executions.
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext, RpcResponse};
use tiktools_core::{
    collaboration::{hash_secret, Capability, ClientKind, Collect, Grant, InviteCreate, Redeem},
    control::AutomationKind,
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
}
struct Fixture {
    core: Arc<AppCore>,
    api: ControlApi,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-api-manifest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths,
        ));
        Self {
            api: ControlApi::new(core.clone()),
            core,
            root,
        }
    }
    async fn call(&self, context: &RequestContext, params: Value, meta: Value) -> RpcResponse {
        let mut request = json!({"id":1,"method":"automation.manifest.get","params":params});
        if !meta.is_null() {
            request["meta"] = meta;
        }
        self.api.execute_value_with_context(context, &request).await
    }
    fn remote(&self) -> RequestContext {
        let service = self.core.collaboration().unwrap();
        let mut config = service.configuration().unwrap();
        config.enabled = true;
        config.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(config).unwrap();
        let grant = Grant {
            capabilities: [
                Capability::LiveRead,
                Capability::PointsRead,
                Capability::PointsAdjust,
                Capability::RankingsRead,
                Capability::RankingsManage,
                Capability::CompetitionsRead,
                Capability::CompetitionsManage,
                Capability::WorkflowsRead,
                Capability::WorkflowsEdit,
                Capability::WorkflowsToggle,
                Capability::AutomationRead,
                Capability::AutomationEdit,
                Capability::AutomationToggle,
                Capability::PluginsRead,
                Capability::PluginsToggle,
                Capability::PluginSettingsRead,
                Capability::PluginSettingsWrite,
                Capability::PluginActionExecute,
                Capability::WidgetsRead,
                Capability::WidgetsConfigure,
                Capability::CommandsRead,
            ]
            .into(),
            ..Grant::default()
        };
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "c".repeat(64);
        let request = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Manifest caller".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "fixture",
            )
            .unwrap();
        service.decide_request(&request.request_id, true).unwrap();
        let credentials = service
            .collect(
                Collect {
                    request_id: request.request_id,
                    exchange_secret: exchange,
                },
                "fixture",
            )
            .unwrap();
        let auth = service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap();
        RequestContext::Remote(RemoteSession::from_authorization(auth).unwrap())
    }
    async fn close(self) {
        self.core.shutdown().await;
        drop(self.api);
        drop(self.core);
        std::fs::remove_dir_all(self.root).unwrap();
    }
}

#[tokio::test]
async fn owner_manifest_binds_every_action_revision_without_returning_candidate_source_or_executing(
) {
    let f = Fixture::new();
    f.core.automation_create(AutomationKind::Action,json!({"id":"same","name":"Private action name","enabled":false,"typeId":"core.log","config":{"message":"private-manifest-source"},"source":"private-manifest-script"})).unwrap();
    f.core.automation_create(AutomationKind::Event,json!({"id":"same","name":"Private event name","enabled":false,"trigger":"tiktok.chat","actionIds":["same","missing","same"],"runMode":"random"})).unwrap();
    let before = f.core.behavior_snapshot_versioned().unwrap();
    let response = f
        .call(
            &RequestContext::LocalOwner,
            json!({"kind":"event","id":"same"}),
            Value::Null,
        )
        .await;
    assert!(response.is_ok(), "{response:?}");
    let body = response.result.unwrap();
    assert_eq!(body["policyVersion"], 1);
    assert_eq!(body["executionAuthorized"], false);
    assert_eq!(body["primary"]["kind"], "event");
    assert_eq!(body["primary"]["id"], "same");
    assert_eq!(body["dependencies"].as_array().unwrap().len(), 2);
    assert_eq!(body["dependencies"][0]["id"], "same");
    assert_eq!(body["dependencies"][1]["id"], "missing");
    let serialized = body.to_string();
    for secret in [
        "private-manifest-source",
        "private-manifest-script",
        "Private action name",
        "Private event name",
    ] {
        assert!(!serialized.contains(secret));
    }
    let meta = response.meta.unwrap();
    assert!(meta.command.is_none());
    assert_eq!(
        serde_json::to_value(meta.resource_versions).unwrap(),
        json!([
            {"kind":"automation","scope":"event","id":"same","revision":"1"},
            {"kind":"automation","scope":"action","id":"same","revision":"1"},
            {"kind":"automation","scope":"action","id":"missing","revision":"0"}
        ])
    );
    assert_eq!(f.core.behavior_snapshot_versioned().unwrap(), before);
    assert!(f.core.automation_runs().is_empty());
    f.close().await;
}

#[tokio::test]
async fn preview_rejects_caller_bodies_authority_and_command_metadata() {
    let f = Fixture::new();
    let good = json!({"kind":"action","id":"saved"});
    for params in [
        json!({"id":"saved"}),
        json!({"kind":"actions","id":"saved"}),
        json!({"kind":"action","id":"saved","record":{}}),
        json!({"kind":"action","id":"saved","grant":{}}),
        json!({"kind":"action","id":"saved","principalId":"forged"}),
        json!({"kind":"action","id":"saved","reviewedVersions":[]}),
    ] {
        let result = f
            .call(&RequestContext::LocalOwner, params, Value::Null)
            .await;
        assert_eq!(result.error.unwrap().code, "invalid_params");
        assert!(result.result.is_none());
        assert!(result.meta.is_none());
    }
    for meta in [
        json!({"expectedRevision":"0"}),
        json!({"commandId":"caller-choice"}),
        json!({"hostInstanceId":"caller-instance"}),
    ] {
        let result = f
            .call(&RequestContext::LocalOwner, good.clone(), meta)
            .await;
        assert!(matches!(
            result.error.unwrap().code.as_str(),
            "invalid_params" | "unsupported_remote"
        ));
        assert!(result.result.is_none());
        assert!(result.meta.is_none());
    }
    f.close().await;
}

#[tokio::test]
async fn every_remote_grant_is_denied_before_preview_and_on_response_egress() {
    let f = Fixture::new();
    let remote = f.remote();
    for params in [
        json!({"kind":"event","id":"never"}),
        json!({"kind":"action","id":"never","record":{"source":"caller"}}),
    ] {
        let response = f.call(&remote, params, Value::Null).await;
        assert_eq!(response.error.unwrap().code, "forbidden");
        assert!(response.result.is_none());
        assert!(response.meta.is_none());
    }
    let RequestContext::Remote(session) = &remote else {
        panic!()
    };
    assert!(f
        .api
        .authorize_remote_response(
            session,
            "automation.manifest.get",
            &json!({"manifestDigest":"owner-only-digest"})
        )
        .is_err());
    assert!(f.api.authorize_remote_failure(session,"automation.manifest.get",Some(&json!({"resourceVersions":[{"kind":"automation","scope":"event","id":"never","revision":"0"}]}))).is_err());
    f.close().await;
}

fn approval_params(method: &str) -> Value {
    if method == "automation.approval.activate" {
        json!({
            "kind":"event","id":"never","manifestDigest":"a".repeat(64),
            "reviewedVersions":[{"kind":"automation","scope":"event","id":"never","revision":"1"}],
            "enablePlan":[{"kind":"event","id":"never","enabled":true}],
            "acknowledgeEffects":true
        })
    } else {
        json!({"kind":"event","id":"never","acknowledgeRevocation":true})
    }
}

#[tokio::test]
async fn every_remote_grant_is_denied_owner_approval_on_ingress_and_egress() {
    let f = Fixture::new();
    let remote = f.remote();
    let RequestContext::Remote(session) = &remote else {
        panic!()
    };
    for method in ["automation.approval.activate", "automation.approval.revoke"] {
        for params in [
            approval_params(method),
            json!({"record":{"source":"forged"}}),
        ] {
            let response = f.api.execute_value_with_context(&remote, &json!({
                "id":1,"method":method,"params":params,
                "meta":{"commandId":"remote-command","hostInstanceId":"remote-instance","expectedRevision":"1"}
            })).await;
            assert_eq!(response.error.unwrap().code, "forbidden", "{method}");
            assert!(response.result.is_none());
            assert!(response.meta.is_none());
        }
        assert!(f
            .api
            .authorize_remote_response(
                session,
                method,
                &json!({
                    "kind":"event","id":"never","approvalGeneration":"1"
                })
            )
            .is_err());
        assert!(f.api.authorize_remote_failure(session, method, Some(&json!({
            "resourceVersions":[{"kind":"automation","scope":"event","id":"never","revision":"1"}]
        }))).is_err());
    }
    f.close().await;
}

#[tokio::test]
async fn owner_approval_accepts_only_saved_review_and_complete_durable_metadata() {
    let f = Fixture::new();
    for method in ["automation.approval.activate", "automation.approval.revoke"] {
        for field in [
            "record",
            "source",
            "candidate",
            "provenance",
            "subjects",
            "grant",
            "permit",
            "principalId",
        ] {
            let mut params = approval_params(method);
            params[field] = json!({});
            let response = f
                .api
                .execute_value(&json!({"id":1,"method":method,"params":params}))
                .await;
            assert_eq!(
                response.error.unwrap().code,
                "invalid_params",
                "{method}: {field}"
            );
            assert!(response.result.is_none());
            assert!(response.meta.is_none());
        }
        for meta in [
            Value::Null,
            json!({}),
            json!({"commandId":"owner-command"}),
            json!({"commandId":"owner-command","hostInstanceId":"instance"}),
        ] {
            let mut request = json!({"id":1,"method":method,"params":approval_params(method)});
            if !meta.is_null() {
                request["meta"] = meta;
            }
            let response = f.api.execute_value(&request).await;
            assert_eq!(response.error.unwrap().code, "invalid_params", "{method}");
            assert!(response.result.is_none());
            assert!(response.meta.is_none());
        }
        let mut params = approval_params(method);
        params[if method == "automation.approval.activate" {
            "acknowledgeEffects"
        } else {
            "acknowledgeRevocation"
        }] = false.into();
        let response = f.api.execute_value(&json!({
            "id":1,"method":method,"params":params,
            "meta":{"commandId":"owner-command","hostInstanceId":"instance","expectedRevision":"1"}
        })).await;
        assert_eq!(
            response.error.unwrap().code,
            "confirmation_required",
            "{method}"
        );
        assert!(response.result.is_none());
        assert!(response.meta.is_none());
    }
    f.close().await;
}
