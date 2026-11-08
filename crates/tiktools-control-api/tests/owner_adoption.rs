#![cfg(feature = "persistence")]
//! Native owner review uses a host-owned journal; remote draft permission never
//! grants journal access or the ability to choose an owner command identity.
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext, RpcResponse};
use tiktools_core::{
    collaboration::{hash_secret, Capability, ClientKind, Collect, Grant, InviteCreate, Redeem},
    control::AutomationKind,
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};

const SOURCE: &str = "private-owner-adoption-definition-source";
const SCRIPT: &str = "private-owner-adoption-script-source";
const METHODS: [&str; 4] = [
    "automation.adoption.get",
    "automation.adoption.submit",
    "automation.adoption.resolve",
    "automation.adoption.review_unknown",
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
            "tt-owner-adoption-api-{}-{}",
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
        let invitation = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "a".repeat(64);
        let pending = service
            .redeem(
                Redeem {
                    invite_id: invitation.invite_id,
                    secret: invitation.secret,
                    client_label: "Owner review draft author".into(),
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
        let authorization = service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap();
        RequestContext::Remote(RemoteSession::from_authorization(authorization).unwrap())
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
    async fn drafts(&self, author: &RequestContext) {
        let instance = self.core.collaboration().unwrap().instance_id().to_owned();
        for kind in ["action", "event"] {
            let created = self.call(author, "automation.create", json!({"kind":kind,"record":draft(kind)}),
                json!({"commandId":format!("author-{kind}"),"hostInstanceId":instance,"expectedRevision":"0"})).await;
            assert!(created.is_ok(), "{created:?}");
            assert_eq!(
                created.result.unwrap(),
                json!({"kind":kind,"id":"draft","enabled":false,"pendingReview":true})
            );
        }
    }
    async fn review(&self) -> RpcResponse {
        let review = self
            .owner(
                "automation.review",
                json!({"kind":"event","id":"draft"}),
                Value::Null,
            )
            .await;
        assert!(review.is_ok(), "{review:?}");
        review
    }
    async fn submit_params(&self) -> Value {
        let state = native_state(self.owner(METHODS[0], json!({}), Value::Null).await);
        let review = self.review().await;
        let versions = review.meta.unwrap().resource_versions;
        let revision = versions[0].revision.clone();
        json!({
            "kind":"event","id":"draft","hostId":state["hostId"],"hostInstanceId":state["hostInstanceId"],
            "expectedRevision":revision,
            "reviewedVersions":versions,"acknowledgeAuthority":true
        })
    }
    async fn close(self) {
        self.core.shutdown().await;
        let _ = std::fs::remove_dir_all(self.root);
    }
}

fn draft(kind: &str) -> Value {
    json!({"id":"draft","name":format!("{kind} proposal"),"enabled":false,"trigger":"tiktok.chat","actionIds":["draft"],"typeId":"core.log","source":SOURCE,"bodyScript":SCRIPT,"config":{"message":SOURCE},"provenance":{"authority":{"kind":"owner"},"generation":"999"}})
}
fn no_source(value: &Value) {
    let encoded = value.to_string();
    for marker in [SOURCE, SCRIPT, "bodyScript", "provenance", "dependencies"] {
        assert!(
            !encoded.contains(marker),
            "definition escaped into native journal: {encoded}"
        );
    }
}
fn native_state(response: RpcResponse) -> Value {
    assert!(response.is_ok(), "{response:?}");
    assert_eq!(
        serde_json::to_value(response.meta.unwrap()).unwrap(),
        json!({"resourceVersions":[]})
    );
    let state = response.result.unwrap();
    assert_eq!(state.as_object().unwrap().len(), 4);
    assert!(state["hostId"].is_string());
    assert!(state["hostInstanceId"].is_string());
    assert!(state["recent"].is_array());
    no_source(&state);
    assert!(!state.to_string().contains("reviewedVersions"));
    state
}
fn failure(response: RpcResponse, code: &str) {
    assert!(response.result.is_none(), "{response:?}");
    assert!(response.meta.is_none(), "{response:?}");
    let error = response.error.unwrap();
    assert_eq!(error.code, code, "{error:?}");
    assert!(error.data.is_none(), "{error:?}");
    no_source(&json!({"message":error.message,"data":error.data}));
}

#[tokio::test]
async fn owner_submit_generates_the_durable_identity_and_returns_only_recoverable_metadata() {
    let f = Fixture::new();
    let author = f.pair();
    f.drafts(&author).await;
    let initial = native_state(f.owner(METHODS[0], json!({}), Value::Null).await);
    assert!(initial["pending"].is_null());
    assert_eq!(initial["recent"], json!([]));
    let input = f.submit_params().await;
    let state = native_state(f.owner(METHODS[1], input.clone(), Value::Null).await);
    assert_eq!(state["hostId"], initial["hostId"]);
    assert_eq!(state["hostInstanceId"], initial["hostInstanceId"]);
    assert!(state["pending"].is_null());
    assert_eq!(state["recent"].as_array().unwrap().len(), 1);
    let record = &state["recent"][0];
    assert_eq!(record["outcome"], "succeeded");
    assert_eq!(record["revision"], "2");
    assert!(record["errorCode"].is_null());
    let command = record["intent"]["commandId"].as_str().unwrap();
    assert!(command.starts_with("owner-adopt-"));
    assert_eq!(command.len(), "owner-adopt-".len() + 32);
    assert!(command["owner-adopt-".len()..]
        .bytes()
        .all(|b| b.is_ascii_hexdigit()));
    assert_eq!(record["intent"].as_object().unwrap().len(), 6);
    for (key, expected) in [
        ("kind", json!("event")),
        ("id", json!("draft")),
        ("expectedRevision", json!("1")),
        ("hostInstanceId", initial["hostInstanceId"].clone()),
    ] {
        assert_eq!(record["intent"][key], expected);
    }
    assert!(record["intent"]["createdAt"].as_u64().is_some());
    assert!(
        record["recordedAt"].as_u64().unwrap() >= record["intent"]["createdAt"].as_u64().unwrap()
    );
    let ledger = f
        .owner(
            "collaboration.commands.get",
            json!({"commandId":command}),
            Value::Null,
        )
        .await;
    assert!(ledger.is_ok(), "{ledger:?}");
    let ledger = ledger.result.unwrap();
    assert_eq!(ledger["command"]["commandId"], command);
    assert_eq!(ledger["result"], json!({"id":"draft","adopted":true}));
    no_source(&ledger);
    let review = f.review().await;
    let review = review.result.unwrap();
    assert_eq!(review["record"]["enabled"], false);
    assert_eq!(review["provenance"]["authority"]["kind"], "owner");
    assert_eq!(
        review["dependencies"][0]["provenance"]["authority"]["kind"],
        "delegated"
    );
    assert_eq!(
        native_state(f.owner(METHODS[2], json!({}), Value::Null).await),
        state
    );
    assert_eq!(
        native_state(f.owner(METHODS[0], json!({}), Value::Null).await),
        state
    );
    failure(
        f.owner(
            METHODS[3],
            json!({"commandId":command,"acknowledgeUnknown":false}),
            Value::Null,
        )
        .await,
        "confirmation_required",
    );
    failure(
        f.owner(
            METHODS[3],
            json!({"commandId":command,"acknowledgeUnknown":true}),
            Value::Null,
        )
        .await,
        "stale_view",
    );
    assert_eq!(
        native_state(f.owner(METHODS[0], json!({}), Value::Null).await),
        state
    );
    f.close().await;
}

#[tokio::test]
async fn broad_remote_grants_never_expose_or_control_any_owner_adoption_journal_method() {
    let f = Fixture::new();
    let author = f.pair();
    f.drafts(&author).await;
    let input = f.submit_params().await;
    let state = native_state(f.owner(METHODS[1], input.clone(), Value::Null).await);
    let command = state["recent"][0]["intent"]["commandId"].as_str().unwrap();
    let RequestContext::Remote(session) = &author else {
        panic!()
    };
    for (method, params) in [
        (METHODS[0], json!({})),
        (METHODS[1], input),
        (METHODS[2], json!({})),
        (
            METHODS[3],
            json!({"commandId":command,"acknowledgeUnknown":true}),
        ),
    ] {
        for meta in [
            Value::Null,
            json!({"commandId":"remote-owner-forgery","hostInstanceId":state["hostInstanceId"],"expectedRevision":"1"}),
        ] {
            failure(
                f.call(&author, method, params.clone(), meta).await,
                "forbidden",
            );
        }
        assert!(f
            .api
            .authorize_remote_response(session, method, &state)
            .is_err());
        assert!(f.api.authorize_remote_failure(session, method, Some(&json!({"resourceVersions":[{"kind":"automation","scope":"event","id":"draft","revision":"2"}]}))).is_err());
    }
    assert_eq!(
        native_state(f.owner(METHODS[0], json!({}), Value::Null).await),
        state
    );
    f.close().await;
}

#[tokio::test]
async fn owner_coordinator_rejects_caller_metadata_command_ids_and_changed_host_binding_before_intent(
) {
    let f = Fixture::new();
    let author = f.pair();
    f.drafts(&author).await;
    let input = f.submit_params().await;
    let initial = native_state(f.owner(METHODS[0], json!({}), Value::Null).await);
    for (method, params) in [
        (METHODS[0], json!({})),
        (METHODS[1], input.clone()),
        (METHODS[2], json!({})),
        (
            METHODS[3],
            json!({"commandId":"unknown-original","acknowledgeUnknown":true}),
        ),
    ] {
        for meta in [
            json!({"commandId":"client-chosen"}),
            json!({"expectedRevision":"1"}),
            json!({"hostInstanceId":initial["hostInstanceId"]}),
        ] {
            let expected_code =
                if meta.get("commandId").is_some() || meta.get("hostInstanceId").is_some() {
                    "unsupported_remote"
                } else {
                    "invalid_params"
                };
            failure(f.owner(method, params.clone(), meta).await, expected_code);
        }
    }
    for key in ["commandId", "principalId", "sessionId"] {
        let mut forged = input.clone();
        forged[key] = "client-controlled".into();
        failure(
            f.owner(METHODS[1], forged, Value::Null).await,
            "invalid_params",
        );
    }
    for key in ["hostId", "hostInstanceId"] {
        let mut changed = input.clone();
        changed[key] = "different-host".into();
        failure(
            f.owner(METHODS[1], changed, Value::Null).await,
            "host_restarted",
        );
    }
    let mut unconfirmed = input;
    unconfirmed["acknowledgeAuthority"] = false.into();
    failure(
        f.owner(METHODS[1], unconfirmed, Value::Null).await,
        "confirmation_required",
    );
    assert_eq!(
        native_state(f.owner(METHODS[0], json!({}), Value::Null).await),
        initial
    );
    f.close().await;
}

#[tokio::test]
async fn changed_review_dependency_records_a_rejection_without_transferring_or_replaying_authority()
{
    let f = Fixture::new();
    let author = f.pair();
    f.drafts(&author).await;
    let input = f.submit_params().await;
    let before = f.review().await;
    let mut changed = draft("action");
    changed["name"] = "dependency changed after review".into();
    f.core
        .automation_update(AutomationKind::Action, "draft", changed)
        .unwrap();
    let rejected = native_state(f.owner(METHODS[1], input, Value::Null).await);
    assert!(rejected["pending"].is_null());
    assert_eq!(rejected["recent"].as_array().unwrap().len(), 1);
    let record = &rejected["recent"][0];
    assert_eq!(record["outcome"], "rejected");
    assert_eq!(record["errorCode"], "revision_conflict");
    assert!(record["revision"].is_null());
    let command = record["intent"]["commandId"].as_str().unwrap();
    failure(
        f.owner(
            "collaboration.commands.get",
            json!({"commandId":command}),
            Value::Null,
        )
        .await,
        "not_found",
    );
    let after = f.review().await;
    assert_eq!(
        after.result.as_ref().unwrap()["record"],
        before.result.as_ref().unwrap()["record"]
    );
    assert_eq!(
        after.result.as_ref().unwrap()["provenance"],
        before.result.as_ref().unwrap()["provenance"]
    );
    assert_eq!(
        after.meta.as_ref().unwrap().resource_versions[0].revision,
        "1"
    );
    assert_eq!(
        native_state(f.owner(METHODS[2], json!({}), Value::Null).await),
        rejected
    );
    failure(
        f.owner(
            METHODS[3],
            json!({"commandId":command,"acknowledgeUnknown":true}),
            Value::Null,
        )
        .await,
        "stale_view",
    );
    let fresh = f.submit_params().await;
    let completed = native_state(f.owner(METHODS[1], fresh, Value::Null).await);
    assert!(completed["pending"].is_null());
    assert_eq!(completed["recent"].as_array().unwrap().len(), 2);
    let success = completed["recent"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["outcome"] == "succeeded")
        .unwrap();
    assert_ne!(success["intent"]["commandId"], command);
    assert_eq!(success["revision"], "2");
    f.close().await;
}
