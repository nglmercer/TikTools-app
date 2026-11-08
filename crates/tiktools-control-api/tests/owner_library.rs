#![cfg(feature = "persistence")]
//! Real Owner graph/catalog transactions, scoped transport budgets and recovery.
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext, RpcResponse};
use tiktools_core::{
    collaboration::{hash_secret, Capability, ClientKind, Collect, Grant, InviteCreate, Redeem},
    control::AutomationKind,
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};

const METHODS: [&str; 5] = [
    "automation.library.snapshot",
    "automation.library.get",
    "automation.library.submit",
    "automation.library.resolve",
    "automation.library.review_unknown",
];
const SOURCE: &str = "private-library-api-source";
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
}
struct Fixture {
    core: Arc<AppCore>,
    api: ControlApi,
    paths: AppPaths,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-library-api-{}-{}",
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
            root,
        };
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        Self::open(paths)
    }
    fn open(paths: AppPaths) -> Self {
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        ));
        Self {
            api: ControlApi::new(core.clone()),
            core,
            paths,
        }
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
    async fn snapshot(&self) -> Value {
        result(self.owner(METHODS[0], json!({})).await)
    }
    async fn submit(&self, mutation: Value) -> Value {
        let snapshot = self.snapshot().await;
        let input = params(&snapshot, mutation);
        result(self.owner(METHODS[2], input).await)
    }
    fn all_capabilities(&self) -> RequestContext {
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
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "e".repeat(64);
        let request = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Library fixture".into(),
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
        RequestContext::Remote(
            RemoteSession::from_authorization(
                service
                    .authenticate_live(&credentials.access_token, false, false)
                    .unwrap(),
            )
            .unwrap(),
        )
    }
    async fn close(self) {
        self.core.shutdown().await;
        let _ = std::fs::remove_dir_all(self.paths.root);
    }
}
fn params(snapshot: &Value, mutation: Value) -> Value {
    json!({"mutation":mutation,"expectedRevision":snapshot["revision"],"definitionDigest":snapshot["definitionDigest"],"hostId":snapshot["hostId"],"hostInstanceId":snapshot["hostInstanceId"]})
}
fn result(response: RpcResponse) -> Value {
    assert!(response.is_ok(), "{response:?}");
    assert_eq!(
        serde_json::to_value(response.meta.unwrap()).unwrap(),
        json!({"resourceVersions":[]})
    );
    response.result.unwrap()
}
fn failure(response: RpcResponse, code: &str) {
    assert!(
        response.result.is_none() && response.meta.is_none(),
        "{response:?}"
    );
    let error = response.error.unwrap();
    assert_eq!(error.code, code, "{error:?}");
    assert!(!error.message.contains(SOURCE));
}
fn entry(prefix: &str, large: bool) -> Value {
    let actions:Vec<_> = (0..if large { 8 } else { 1 }).map(|index| json!({"id":format!("{prefix}-action-{index}"),"name":"Library action","enabled":false,"typeId":"core.log","config":{"message":if large { "x".repeat(180_000) } else { SOURCE.into() }}})).collect();
    json!({"actions":actions,"event":{"id":format!("{prefix}-event"),"name":"Library event","enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all","actionIds":actions.iter().map(|a|a["id"].clone()).collect::<Vec<_>>()}})
}
fn template() -> Value {
    json!({"templateVersion":1,"id":"api-template","title":{"default":"API template"},"description":{"default":""},"icon":"plugin","actions":[{"name":"Action","typeId":"core.log","enabled":false,"config":{"message":SOURCE}}],"event":{"name":"Event","enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all"},"custom":true})
}
fn original(submission: &Value) -> &Value {
    assert!(submission["state"]["pending"].is_null());
    let recent = &submission["state"]["recent"][0];
    assert_eq!(recent["outcome"], "succeeded");
    assert_eq!(submission["acknowledgement"]["value"], recent["receipt"]);
    assert_eq!(
        submission["acknowledgement"]["command"]["commandId"],
        recent["intent"]["commandId"]
    );
    assert_eq!(
        submission["acknowledgement"]["resourceVersions"][0]["scope"],
        "library"
    );
    assert!(!submission.to_string().contains(SOURCE));
    recent
}

#[tokio::test]
async fn library_operations_commit_graph_catalog_and_recover_original_metadata_after_restart() {
    let f = Fixture::new();
    original(&f.submit(json!({"operation":"create-profile","id":"á","name":"Unicode profile","description":""})).await);
    let applied = f
        .submit(json!({"operation":"apply-template","entry":entry("first",false),"profileId":"á"}))
        .await;
    original(&applied);
    // Membership assignment preserves an existing named-profile association.
    original(&f.submit(json!({"operation":"assign-record","kind":"action","id":"first-action-0","profileId":"default"})).await);
    original(
        &f.submit(json!({"operation":"switch-profile","id":"á"}))
            .await,
    );
    let snapshot = f.snapshot().await;
    assert_eq!(snapshot["catalog"]["activeId"], "á");
    assert_eq!(
        f.core
            .automation_get(AutomationKind::Event, "first-event")
            .unwrap()["enabled"],
        true
    );
    assert_eq!(
        f.core
            .automation_get(AutomationKind::Action, "first-action-0")
            .unwrap()["enabled"],
        true
    );
    original(&f.submit(json!({"operation":"import-profile","profile":{"id":"imported","name":"Imported","description":""},"entries":[entry("second",false)]})).await);
    original(
        &f.submit(json!({"operation":"import-templates","templates":[template()]}))
            .await,
    );
    let snapshot = f.snapshot().await;
    assert!(snapshot["catalog"]["templates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"] == "api-template"));
    original(
        &f.submit(json!({"operation":"delete-template","id":"api-template"}))
            .await,
    );
    let last = f
        .submit(json!({"operation":"delete-profile","id":"imported"}))
        .await;
    let last = original(&last).clone();
    assert!(f
        .core
        .automation_get(AutomationKind::Event, "second-event")
        .is_err());
    let mut newer = f
        .core
        .automation_get(AutomationKind::Action, "first-action-0")
        .unwrap();
    newer["name"] = json!("Newer owner source");
    f.core
        .automation_update(AutomationKind::Action, "first-action-0", newer)
        .unwrap();
    let history = result(f.owner(METHODS[3], json!({})).await);
    assert_eq!(history["recent"][0], last);
    assert!(!history.to_string().contains(SOURCE));
    let paths = f.paths.clone();
    f.core.shutdown().await;
    drop(f);
    let restarted = Fixture::open(paths);
    let recovered = result(restarted.owner(METHODS[1], json!({})).await);
    assert_eq!(recovered["recent"][0], last);
    assert_eq!(
        restarted
            .core
            .automation_get(AutomationKind::Action, "first-action-0")
            .unwrap()["name"],
        "Newer owner source"
    );
    restarted.close().await;
}

#[tokio::test]
async fn library_snapshot_fences_stale_universe_and_only_submit_has_large_owner_params() {
    let f = Fixture::new();
    let before = f.snapshot().await;
    f.core.automation_create(AutomationKind::Action,json!({"id":"concurrent","name":"Concurrent owner","enabled":false,"typeId":"core.log"})).unwrap();
    let stale = result(
        f.owner(
            METHODS[2],
            params(
                &before,
                json!({"operation":"create-profile","id":"stale","name":"Stale","description":""}),
            ),
        )
        .await,
    );
    assert!(stale["acknowledgement"].is_null());
    assert_eq!(
        stale["state"]["recent"][0]["errorCode"],
        "revision_conflict"
    );
    let snapshot = f.snapshot().await;
    assert!(snapshot["catalog"]["packs"].as_array().unwrap().is_empty());
    let input = params(
        &snapshot,
        json!({"operation":"apply-template","entry":entry("large",true),"profileId":"default"}),
    );
    assert!(input.to_string().len() > tiktools_control_api::MAX_REQUEST_BYTES);
    let large = result(f.owner(METHODS[2], input).await);
    original(&large);
    let source = f.snapshot().await;
    assert!(source.to_string().len() > tiktools_control_api::MAX_REQUEST_BYTES);
    failure(f.owner("app.state.set",json!({"key":"ordinary.fixture","value":"x".repeat(tiktools_control_api::MAX_PARAMS_BYTES)})).await,"too_large");
    f.close().await;
}

#[tokio::test]
async fn library_routes_deny_even_all_grants_and_reject_forged_proofs_before_intent() {
    let f = Fixture::new();
    let snapshot = f.snapshot().await;
    let input = params(
        &snapshot,
        json!({"operation":"create-profile","id":"native","name":"Native","description":""}),
    );
    let remote = f.all_capabilities();
    let RequestContext::Remote(session) = &remote else {
        panic!()
    };
    for method in METHODS {
        let params = if method == METHODS[2] {
            input.clone()
        } else if method == METHODS[4] {
            json!({"commandId":format!("owner-library-{}","a".repeat(32)),"acknowledgeUnknown":true})
        } else {
            json!({})
        };
        failure(
            f.call(&remote, method, params.clone(), Value::Null).await,
            "forbidden",
        );
        assert!(f
            .api
            .authorize_remote_response(session, method, &json!({"source":SOURCE}))
            .is_err());
        assert!(f
            .api
            .authorize_remote_failure(session, method, Some(&json!({"resourceVersions":[]})))
            .is_err());
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
        let schema = f.owner("rpc.schema", json!({"method":method})).await;
        assert!(schema.is_ok(), "{schema:?}");
        let schema = schema.result.unwrap();
        assert_eq!(schema["name"], method);
        assert_eq!(schema["paramsSchema"]["additionalProperties"], false);
    }
    let discovered = f
        .call(&remote, "rpc.discover", json!({}), Value::Null)
        .await;
    assert!(discovered.is_ok(), "{discovered:?}");
    assert!(!discovered.result.unwrap()["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["name"]
            .as_str()
            .is_some_and(|name| name.starts_with("automation.library."))));
    for field in [
        "commandId",
        "proof",
        "record",
        "authority",
        "candidate",
        "reviewedVersions",
    ] {
        let mut bad = input.clone();
        bad[field] = json!({});
        failure(f.owner(METHODS[2], bad).await, "invalid_params");
    }
    let mut nested = input.clone();
    nested["mutation"]["commandId"] = json!("chosen");
    failure(f.owner(METHODS[2], nested).await, "invalid_params");
    for revision in ["01", "-1", "9223372036854775808"] {
        let mut bad = input.clone();
        bad["expectedRevision"] = json!(revision);
        failure(f.owner(METHODS[2], bad).await, "invalid_params");
    }
    let state = result(f.owner(METHODS[1], json!({})).await);
    assert!(state["pending"].is_null());
    assert_eq!(state["recent"], json!([]));
    f.close().await;
}

#[tokio::test]
async fn library_pending_excludes_every_owner_journal_and_definition_pending_excludes_library() {
    let f = Fixture::new();
    f.core
        .automation_create(
            AutomationKind::Action,
            json!({"id":"shared","name":"Shared owner action","enabled":false,"typeId":"core.log"}),
        )
        .unwrap();
    let snapshot = f.snapshot().await;
    let input = params(
        &snapshot,
        json!({"operation":"create-profile","id":"queued","name":"Queued","description":""}),
    );
    let typed: tiktools_core::control::OwnerLibrarySubmit =
        serde_json::from_value(input.clone()).unwrap();
    let command = format!("owner-library-{}", "a".repeat(32));
    let intent = json!({"commandId":command,"operation":"create-profile","target":"queued","expectedRevision":snapshot["revision"],"definitionDigest":snapshot["definitionDigest"],"commandHash":tiktools_core::control::owner_library_command_hash(&typed),"hostInstanceId":snapshot["hostInstanceId"],"createdAt":10});
    let stored = json!({"version":1,"hostId":snapshot["hostId"],"intent":intent,"record":null});
    let db = rusqlite::Connection::open(f.paths.automation_database()).unwrap();
    db.execute("INSERT INTO owner_automation_library(command_id,pending,payload_json,updated_at) VALUES(?1,1,?2,10)",rusqlite::params![command,stored.to_string()]).unwrap();
    let pending = result(f.owner(METHODS[1], json!({})).await);
    assert_eq!(pending["pending"]["commandId"], command);
    failure(f.owner(METHODS[2], input.clone()).await, "library_pending");
    let common = json!({"hostId":snapshot["hostId"],"hostInstanceId":snapshot["hostInstanceId"],"kind":"action","id":"shared","expectedRevision":"1"});
    let mut definition = common.clone();
    definition["operation"] = json!("disable");
    failure(
        f.owner("automation.definition.submit", definition).await,
        "command_pending",
    );
    let mut adoption = common.clone();
    adoption["acknowledgeAuthority"] = json!(true);
    adoption["reviewedVersions"] =
        json!([{"kind":"automation","scope":"action","id":"shared","revision":"1"}]);
    failure(
        f.owner("automation.adoption.submit", adoption).await,
        "command_pending",
    );
    let mut approval = common;
    approval["operation"] = json!("revoke");
    approval["acknowledgeRevocation"] = json!(true);
    failure(
        f.owner("automation.approval.submit", approval).await,
        "command_pending",
    );
    assert_eq!(
        result(f.owner(METHODS[3], json!({})).await)["pending"]["commandId"],
        command
    );
    let archived = result(
        f.owner(
            METHODS[4],
            json!({"commandId":command,"acknowledgeUnknown":true}),
        )
        .await,
    );
    assert!(archived["pending"].is_null());
    assert_eq!(archived["recent"][0]["outcome"], "reviewed-unknown");
    assert!(f.snapshot().await["catalog"]["packs"]
        .as_array()
        .unwrap()
        .is_empty());
    let definition_command = format!("owner-definition-{}", "b".repeat(32));
    let stored = json!({"version":1,"hostId":snapshot["hostId"],"payloadHash":"c".repeat(64),"intent":{"commandId":definition_command,"operation":"create","kind":"action","id":"queued-definition","expectedRevision":"0","hostInstanceId":snapshot["hostInstanceId"],"recordDigest":"d".repeat(64),"createdAt":10},"record":null});
    db.execute("INSERT INTO owner_automation_definitions(command_id,pending,payload_json,updated_at) VALUES(?1,1,?2,10)",rusqlite::params![definition_command,stored.to_string()]).unwrap();
    failure(f.owner(METHODS[2], input).await, "library_pending");
    assert!(f
        .core
        .automation_get(AutomationKind::Action, "queued-definition")
        .is_err());
    drop(db);
    f.close().await;
}
