#![cfg(feature = "persistence")]
//! Actual native write adapters, original-result recovery and Owner-only policy.
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext, RpcResponse};
use tiktools_core::{
    collaboration::{hash_secret, Capability, ClientKind, Collect, Grant, InviteCreate, Redeem},
    control::AutomationKind,
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};

const SOURCE: &str = "private-native-owner-definition-source";
const METHODS: [&str; 4] = [
    "automation.definition.get",
    "automation.definition.submit",
    "automation.definition.resolve",
    "automation.definition.review_unknown",
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
            "tt-owner-definition-api-{}-{}",
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
    fn host(&self) -> Value {
        let service = self.core.collaboration().unwrap();
        json!({"hostId":service.host_id(),"hostInstanceId":service.instance_id()})
    }
    fn paired_all_capabilities(&self) -> RequestContext {
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
                Capability::HttpRequest,
                Capability::AudioPlay,
                Capability::EventsEmit,
                Capability::ScriptsExecute,
            ]
            .into(),
            ..Grant::default()
        };
        let invitation = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "f".repeat(64);
        let pending = service
            .redeem(
                Redeem {
                    invite_id: invitation.invite_id,
                    secret: invitation.secret,
                    client_label: "Owner definition API fixture".into(),
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
    async fn owner(&self, method: &str, params: Value) -> RpcResponse {
        self.call(&RequestContext::LocalOwner, method, params, Value::Null)
            .await
    }
    fn params(
        &self,
        operation: &str,
        kind: &str,
        id: &str,
        revision: &str,
        record: Option<Value>,
    ) -> Value {
        let host = self.host();
        let mut params = json!({"operation":operation,"kind":kind,"expectedRevision":revision,"hostId":host["hostId"],"hostInstanceId":host["hostInstanceId"]});
        if operation != "create" {
            params["id"] = json!(id);
        }
        if let Some(record) = record {
            params["record"] = record;
        }
        params
    }
    async fn close(self) {
        self.core.shutdown().await;
        let _ = std::fs::remove_dir_all(self.root);
    }
}
fn record(kind: &str, id: &str) -> Value {
    if kind == "action" {
        json!({"id":id,"name":"Owner action","enabled":false,"typeId":"core.log","config":{"message":SOURCE}})
    } else {
        json!({"id":id,"name":SOURCE,"enabled":false,"trigger":"tiktok.chat","filters":[],"actionIds":[]})
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
    for marker in [
        SOURCE,
        "acknowledgement",
        "provenance",
        "grant",
        "authority",
    ] {
        assert!(!result.to_string().contains(marker), "{result}");
    }
    result
}
fn submission(response: RpcResponse) -> Value {
    assert!(response.is_ok(), "{response:?}");
    assert_eq!(
        serde_json::to_value(response.meta.unwrap()).unwrap(),
        json!({"resourceVersions":[]})
    );
    let result = response.result.unwrap();
    assert_eq!(result.as_object().unwrap().len(), 2);
    assert!(result["state"]["pending"].is_null());
    assert_eq!(result["state"]["recent"][0]["outcome"], "succeeded");
    let intent = &result["state"]["recent"][0]["intent"];
    assert!(intent["commandId"]
        .as_str()
        .unwrap()
        .starts_with("owner-definition-"));
    assert_eq!(
        result["acknowledgement"]["command"]["commandId"],
        intent["commandId"]
    );
    assert_eq!(
        result["acknowledgement"]["hostInstanceId"],
        intent["hostInstanceId"]
    );
    assert_eq!(result["acknowledgement"].as_object().unwrap().len(), 4);
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
async fn native_owner_crud_keeps_recreation_and_recovery_separate_from_newer_source() {
    let f = Fixture::new();
    let initial = state(f.owner(METHODS[0], json!({})).await);
    assert_eq!(initial["recent"], json!([]));
    for (kind, typed_kind) in [
        ("action", AutomationKind::Action),
        ("event", AutomationKind::Event),
    ] {
        let id = format!("owner-{kind}");
        let original = record(kind, &id);
        let created = submission(
            f.owner(
                METHODS[1],
                f.params("create", kind, &id, "0", Some(original.clone())),
            )
            .await,
        );
        assert_eq!(created["acknowledgement"]["value"], original);
        assert_eq!(created["state"]["recent"][0]["receipt"]["revision"], "1");
        let mut update = original.clone();
        update["name"] = json!("Updated owner source");
        let updated = submission(
            f.owner(METHODS[1], f.params("update", kind, &id, "1", Some(update)))
                .await,
        );
        assert_eq!(
            updated["acknowledgement"]["value"]["name"],
            "Updated owner source"
        );
        for (operation, revision, enabled) in [("enable", "2", true), ("disable", "3", false)] {
            let changed = submission(
                f.owner(METHODS[1], f.params(operation, kind, &id, revision, None))
                    .await,
            );
            assert_eq!(changed["acknowledgement"]["value"]["enabled"], enabled);
        }
        let deleted = submission(
            f.owner(METHODS[1], f.params("delete", kind, &id, "4", None))
                .await,
        );
        assert_eq!(
            deleted["acknowledgement"]["value"],
            json!({"id":id,"deleted":true})
        );
        let recreated = submission(
            f.owner(
                METHODS[1],
                f.params("create", kind, &id, "5", Some(original.clone())),
            )
            .await,
        );
        assert_eq!(recreated["state"]["recent"][0]["receipt"]["revision"], "6");
        let mut newer = original;
        newer["name"] = json!("Newer direct owner edit");
        f.core.automation_update(typed_kind, &id, newer).unwrap();
        let recovered = state(f.owner(METHODS[2], json!({})).await);
        assert_eq!(recovered, recreated["state"]);
        assert_eq!(
            f.core.automation_get(typed_kind, &id).unwrap()["name"],
            "Newer direct owner edit"
        );
        assert_eq!(state(f.owner(METHODS[0], json!({})).await), recovered);
    }
    f.close().await;
}

#[tokio::test]
async fn action_delete_acknowledges_all_cascaded_event_versions_and_missing_delete_does_not_cascade(
) {
    let f = Fixture::new();
    submission(
        f.owner(
            METHODS[1],
            f.params(
                "create",
                "action",
                "parent",
                "0",
                Some(record("action", "parent")),
            ),
        )
        .await,
    );
    for id in ["z-event", "a-event"] {
        let mut event = record("event", id);
        event["actionIds"] = json!(["parent"]);
        f.core
            .automation_create(AutomationKind::Event, event)
            .unwrap();
    }
    let deleted = submission(
        f.owner(
            METHODS[1],
            f.params("delete", "action", "parent", "1", None),
        )
        .await,
    );
    let ack = &deleted["acknowledgement"];
    assert_eq!(
        deleted["state"]["recent"][0]["receipt"]["affectedResources"],
        3
    );
    assert_eq!(
        ack["resourceVersions"],
        json!([
            {"kind":"automation","scope":"action","id":"parent","revision":"2"},
            {"kind":"automation","scope":"event","id":"a-event","revision":"2"},
            {"kind":"automation","scope":"event","id":"z-event","revision":"2"},
        ])
    );
    for id in ["a-event", "z-event"] {
        assert_eq!(
            f.core.automation_get(AutomationKind::Event, id).unwrap()["actionIds"],
            json!([])
        );
    }
    let absent = submission(
        f.owner(
            METHODS[1],
            f.params("delete", "action", "parent", "2", None),
        )
        .await,
    );
    assert_eq!(absent["acknowledgement"]["value"]["deleted"], false);
    assert_eq!(absent["state"]["recent"][0]["receipt"]["revision"], "2");
    assert_eq!(
        absent["state"]["recent"][0]["receipt"]["affectedResources"],
        1
    );
    f.close().await;
}

#[tokio::test]
async fn definition_routes_deny_all_remote_capabilities_at_ingress_and_egress() {
    let f = Fixture::new();
    let remote = f.paired_all_capabilities();
    let RequestContext::Remote(session) = &remote else {
        panic!()
    };
    for (method, params) in [
        (METHODS[0], json!({})),
        (
            METHODS[1],
            f.params(
                "create",
                "action",
                "draft",
                "0",
                Some(record("action", "draft")),
            ),
        ),
        (METHODS[2], json!({})),
        (
            METHODS[3],
            json!({"commandId":format!("owner-definition-{}","a".repeat(32)),"acknowledgeUnknown":true}),
        ),
    ] {
        for meta in [
            Value::Null,
            json!({"commandId":"forged-owner-id","hostInstanceId":"forged-instance","expectedRevision":"0"}),
        ] {
            failure(
                f.call(&remote, method, params.clone(), meta).await,
                "forbidden",
            );
        }
        assert!(f
            .api
            .authorize_remote_response(session, method, &json!({"source":SOURCE}))
            .is_err());
        assert!(f
            .api
            .authorize_remote_failure(session, method, Some(&json!({"resourceVersions":[]})))
            .is_err());
    }
    let current = state(f.owner(METHODS[0], json!({})).await);
    assert_eq!(current["recent"], json!([]));
    assert!(current["pending"].is_null());
    f.close().await;
}

#[tokio::test]
async fn native_definition_params_reject_forged_identity_proof_and_mismatched_source_before_intent()
{
    let f = Fixture::new();
    let input = f.params(
        "create",
        "action",
        "draft",
        "0",
        Some(record("action", "draft")),
    );
    for method in METHODS {
        let params = match method {
            "automation.definition.submit" => input.clone(),
            "automation.definition.review_unknown" => {
                json!({"commandId":format!("owner-definition-{}","a".repeat(32)),"acknowledgeUnknown":true})
            }
            _ => json!({}),
        };
        for meta in [
            json!({"commandId":"chosen"}),
            json!({"hostInstanceId":"chosen"}),
            json!({"expectedRevision":"0"}),
        ] {
            assert!(!f
                .call(&RequestContext::LocalOwner, method, params.clone(), meta)
                .await
                .is_ok());
        }
    }
    for field in [
        "commandId",
        "proof",
        "source",
        "subjects",
        "authority",
        "grant",
        "permit",
    ] {
        let mut forged = input.clone();
        forged[field] = json!({});
        failure(f.owner(METHODS[1], forged).await, "invalid_params");
    }
    for field in ["hostId", "hostInstanceId"] {
        let mut forged = input.clone();
        forged[field] = json!("another-host");
        failure(f.owner(METHODS[1], forged).await, "host_restarted");
    }
    let mismatched = f.params(
        "update",
        "action",
        "wanted",
        "1",
        Some(record("action", "different")),
    );
    failure(f.owner(METHODS[1], mismatched).await, "invalid_params");
    for revision in ["01", "-1", "9223372036854775808"] {
        let mut forged = input.clone();
        forged["expectedRevision"] = json!(revision);
        failure(f.owner(METHODS[1], forged).await, "invalid_params");
    }
    let current = state(f.owner(METHODS[0], json!({})).await);
    assert!(current["pending"].is_null());
    assert_eq!(current["recent"], json!([]));
    f.close().await;
}
