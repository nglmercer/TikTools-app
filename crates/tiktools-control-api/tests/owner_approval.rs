#![cfg(feature = "persistence")]
//! Real Owner API review/coordinator integration. Remote grants cannot review,
//! choose durable identities, or retrieve the Owner's native journal.
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext, RpcResponse};
use tiktools_core::{
    collaboration::{hash_secret, Capability, ClientKind, Collect, Grant, InviteCreate, Redeem},
    control::AutomationKind,
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};

const SOURCE: &str = "private-native-owner-approval-source";
const METHODS: [&str; 5] = [
    "automation.approval.review",
    "automation.approval.get",
    "automation.approval.submit",
    "automation.approval.resolve",
    "automation.approval.review_unknown",
];
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
}
struct Fixture {
    core: Arc<AppCore>,
    api: ControlApi,
    root: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-owner-approval-api-{}-{}",
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
    fn pair(&self) -> RequestContext {
        let service = self.core.collaboration().unwrap();
        let mut configuration = service.configuration().unwrap();
        configuration.enabled = true;
        configuration.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(configuration).unwrap();
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
        let exchange = "e".repeat(64);
        let pending = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Owner approval API fixture".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "fixture",
            )
            .unwrap();
        service.decide_request(&pending.request_id, true).unwrap();
        let credentials = service
            .collect(
                Collect {
                    request_id: pending.request_id,
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
    async fn call(
        &self,
        context: &RequestContext,
        method: &str,
        params: Value,
        meta: Value,
    ) -> RpcResponse {
        let mut request = json!({"id":1,"method":method,"params":params});
        if !meta.is_null() {
            request["meta"] = meta;
        }
        self.api.execute_value_with_context(context, &request).await
    }
    async fn owner(&self, method: &str, params: Value, meta: Value) -> RpcResponse {
        self.call(&RequestContext::LocalOwner, method, params, meta)
            .await
    }
    async fn drafts(&self, context: &RequestContext) {
        let instance = self.core.collaboration().unwrap().instance_id().to_owned();
        for kind in ["action", "event"] {
            let response=self.call(context,"automation.create",json!({"kind":kind,"record":draft(kind)}),json!({"commandId":format!("author-{kind}"),"hostInstanceId":instance,"expectedRevision":"0"})).await;
            assert!(response.is_ok(), "{response:?}");
        }
    }
    async fn review(&self) -> RpcResponse {
        let response = self
            .owner(
                METHODS[0],
                json!({"kind":"event","id":"draft"}),
                Value::Null,
            )
            .await;
        assert!(response.is_ok(), "{response:?}");
        response
    }
    fn activation(&self, review: &RpcResponse) -> Value {
        let result = review.result.as_ref().unwrap();
        let versions = &review.meta.as_ref().unwrap().resource_versions;
        json!({"operation":"activate","kind":"event","id":"draft","hostId":result["hostId"],"hostInstanceId":result["hostInstanceId"],"expectedRevision":versions[0].revision,
            "reviewedVersions":versions,"manifestDigest":result["manifest"]["manifestDigest"],"enablePlan":[{"kind":"event","id":"draft","enabled":true},{"kind":"action","id":"draft","enabled":true}],"acknowledgeEffects":true})
    }
    async fn close(self) {
        self.core.shutdown().await;
        let _ = std::fs::remove_dir_all(self.root);
    }
}
fn draft(kind: &str) -> Value {
    if kind == "action" {
        json!({"id":"draft","name":"Delegated log","enabled":false,"typeId":"core.log","config":{"message":SOURCE}})
    } else {
        json!({"id":"draft","name":"Delegated event","enabled":false,"trigger":"tiktok.chat","actionIds":["draft"]})
    }
}
fn state(response: RpcResponse) -> Value {
    assert!(response.is_ok(), "{response:?}");
    assert_eq!(
        serde_json::to_value(response.meta.unwrap()).unwrap(),
        json!({"resourceVersions":[]})
    );
    let result = response.result.unwrap();
    assert_eq!(result.as_object().unwrap().len(), 4);
    let encoded = result.to_string();
    for marker in [
        SOURCE,
        "provenance",
        "subjects",
        "reviewedVersions",
        "enablePlan",
        "grant",
    ] {
        assert!(!encoded.contains(marker), "{encoded}");
    }
    result
}
fn failure(response: RpcResponse, code: &str) {
    assert!(
        response.result.is_none() && response.meta.is_none(),
        "{response:?}"
    );
    let error = response.error.unwrap();
    assert_eq!(error.code, code, "{error:?}");
    assert!(error.data.is_none());
    assert!(!error.message.contains(SOURCE));
}

#[tokio::test]
async fn owner_native_review_activation_and_revocation_keep_provenance_and_original_receipts() {
    let f = Fixture::new();
    let author = f.pair();
    f.drafts(&author).await;
    let review = f.review().await;
    let result = review.result.as_ref().unwrap();
    assert_eq!(result.as_object().unwrap().len(), 9);
    assert_eq!(
        result["eligibility"],
        json!({"eligible":true,"reasonCode":null})
    );
    assert_eq!(
        result["source"]["dependencies"][0]["record"]["config"]["message"],
        SOURCE
    );
    assert_eq!(
        result["manifest"]["subjects"][0]["configurationRevision"],
        result["subjects"][0]["configurationRevision"]
    );
    assert_eq!(
        review.meta.as_ref().unwrap().host_instance_id.as_deref(),
        result["hostInstanceId"].as_str()
    );
    let activated = state(
        f.owner(METHODS[2], f.activation(&review), Value::Null)
            .await,
    );
    assert!(activated["pending"].is_null());
    assert_eq!(activated["recent"][0]["outcome"], "succeeded");
    assert_eq!(activated["recent"][0]["receipt"]["revision"], "2");
    let command = activated["recent"][0]["intent"]["commandId"]
        .as_str()
        .unwrap();
    assert!(command.starts_with("owner-approval-"));
    assert_eq!(command.len(), "owner-approval-".len() + 32);
    assert_ne!(
        activated["recent"][0]["receipt"]["manifestDigest"],
        review.result.as_ref().unwrap()["manifest"]["manifestDigest"]
    );
    let current = f.review().await;
    let current = current.result.unwrap();
    assert_eq!(current["approval"]["current"], true);
    assert_eq!(
        current["source"]["provenance"]["authority"]["kind"],
        "delegated"
    );
    assert_eq!(
        state(f.owner(METHODS[3], json!({}), Value::Null).await),
        activated
    );
    let RequestContext::Remote(session) = author else {
        panic!()
    };
    f.core
        .collaboration()
        .unwrap()
        .revoke_session(
            &session.authenticated_identity().unwrap().session_id,
            "local-owner",
        )
        .unwrap();
    let unavailable = f.review().await;
    let snapshot = unavailable.result.as_ref().unwrap();
    assert!(snapshot["manifest"].is_null());
    assert_eq!(
        snapshot["eligibility"]["reasonCode"],
        "subjects-unavailable"
    );
    assert_eq!(snapshot["approval"]["state"], "active");
    assert_eq!(snapshot["approval"]["current"], false);
    let revoked=state(f.owner(METHODS[2],json!({"operation":"revoke","kind":"event","id":"draft","hostId":snapshot["hostId"],"hostInstanceId":snapshot["hostInstanceId"],"expectedRevision":unavailable.meta.unwrap().resource_versions[0].revision,"acknowledgeRevocation":true}),Value::Null).await);
    assert_eq!(revoked["recent"].as_array().unwrap().len(), 2);
    let receipt = revoked["recent"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["intent"]["operation"] == "revoke")
        .unwrap();
    assert_eq!(receipt["receipt"]["revision"], "3");
    assert!(
        receipt["receipt"]["manifestDigest"].is_null()
            && receipt["receipt"]["policyVersion"].is_null()
    );
    assert_eq!(
        f.review().await.result.unwrap()["approval"]["state"],
        "revoked"
    );
    f.close().await;
}

#[tokio::test]
async fn every_native_owner_approval_method_denies_remote_ingress_and_egress_with_all_grants() {
    let f = Fixture::new();
    let author = f.pair();
    f.drafts(&author).await;
    let review = f.review().await;
    let RequestContext::Remote(session) = &author else {
        panic!()
    };
    for (method, input) in [
        (METHODS[0], json!({"kind":"event","id":"draft"})),
        (METHODS[1], json!({})),
        (METHODS[2], f.activation(&review)),
        (METHODS[3], json!({})),
        (
            METHODS[4],
            json!({"commandId":format!("owner-approval-{}","a".repeat(32)),"acknowledgeUnknown":true}),
        ),
    ] {
        for meta in [
            Value::Null,
            json!({"commandId":"forged-owner-id","hostInstanceId":"forged-instance","expectedRevision":"1"}),
        ] {
            failure(
                f.call(&author, method, input.clone(), meta).await,
                "forbidden",
            );
        }
        assert!(f
            .api
            .authorize_remote_response(session, method, review.result.as_ref().unwrap())
            .is_err());
        assert!(f
            .api
            .authorize_remote_failure(
                session,
                method,
                Some(&json!({"resourceVersions":review.meta.as_ref().unwrap().resource_versions}))
            )
            .is_err());
    }
    let initial = state(f.owner(METHODS[1], json!({}), Value::Null).await);
    assert!(initial["pending"].is_null());
    assert_eq!(initial["recent"], json!([]));
    f.close().await;
}

#[tokio::test]
async fn native_owner_requests_reject_caller_ids_authority_extra_proofs_and_changed_host_before_intent(
) {
    let f = Fixture::new();
    let author = f.pair();
    f.drafts(&author).await;
    let review = f.review().await;
    let input = f.activation(&review);
    for (method, params) in [
        (METHODS[0], json!({"kind":"event","id":"draft"})),
        (METHODS[1], json!({})),
        (METHODS[2], input.clone()),
        (METHODS[3], json!({})),
        (
            METHODS[4],
            json!({"commandId":format!("owner-approval-{}","a".repeat(32)),"acknowledgeUnknown":true}),
        ),
    ] {
        for meta in [
            json!({"commandId":"chosen"}),
            json!({"hostInstanceId":"chosen"}),
            json!({"expectedRevision":"1"}),
        ] {
            let code = if meta.get("expectedRevision").is_some() {
                "invalid_params"
            } else {
                "unsupported_remote"
            };
            failure(f.owner(method, params.clone(), meta).await, code);
        }
    }
    for field in [
        "commandId",
        "source",
        "record",
        "subjects",
        "grant",
        "permit",
        "principalId",
    ] {
        let mut forged = input.clone();
        forged[field] = json!({});
        failure(
            f.owner(METHODS[2], forged, Value::Null).await,
            "invalid_params",
        );
    }
    for field in ["hostId", "hostInstanceId"] {
        let mut forged = input.clone();
        forged[field] = json!("another-host");
        failure(
            f.owner(METHODS[2], forged, Value::Null).await,
            "host_restarted",
        );
    }
    let mut unconfirmed = input;
    unconfirmed["acknowledgeEffects"] = false.into();
    failure(
        f.owner(METHODS[2], unconfirmed, Value::Null).await,
        "confirmation_required",
    );
    let empty = state(f.owner(METHODS[1], json!({}), Value::Null).await);
    assert!(empty["pending"].is_null());
    assert_eq!(empty["recent"], json!([]));
    f.close().await;
}

#[tokio::test]
async fn stale_dependency_rejects_once_and_native_resolution_never_replays_the_operation() {
    let f = Fixture::new();
    let author = f.pair();
    f.drafts(&author).await;
    let review = f.review().await;
    let input = f.activation(&review);
    let mut action = draft("action");
    action["name"] = json!("Changed dependency");
    f.core
        .automation_update(AutomationKind::Action, "draft", action)
        .unwrap();
    let rejected = state(f.owner(METHODS[2], input, Value::Null).await);
    assert!(rejected["pending"].is_null());
    assert_eq!(rejected["recent"][0]["outcome"], "rejected");
    assert_eq!(rejected["recent"][0]["errorCode"], "revision_conflict");
    assert!(rejected["recent"][0]["receipt"].is_null());
    assert_eq!(
        state(f.owner(METHODS[3], json!({}), Value::Null).await),
        rejected
    );
    let after = f.review().await;
    assert_eq!(
        after.result.as_ref().unwrap()["source"]["record"]["enabled"],
        false
    );
    assert_eq!(after.result.as_ref().unwrap()["approval"]["state"], "none");
    let succeeded = state(f.owner(METHODS[2], f.activation(&after), Value::Null).await);
    let old = &rejected["recent"][0]["intent"]["commandId"];
    let fresh = succeeded["recent"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["outcome"] == "succeeded")
        .unwrap();
    assert_ne!(&fresh["intent"]["commandId"], old);
    assert_eq!(fresh["receipt"]["revision"], "2");
    f.close().await;
}
