#![cfg(feature = "persistence")]
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext, RpcResponse};
use tiktools_core::{
    collaboration::*, ipc::messages::HostMessage, paths::AppPaths, AppCore, HostEmitter,
    NoopMediaHost,
};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}
struct Fixture {
    core: Arc<AppCore>,
    api: ControlApi,
    root: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("tt-points-api-{}-{nonce}", std::process::id()));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent-builtin"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths,
        ));
        let api = ControlApi::new(core.clone());
        let service = core.collaboration().unwrap();
        let mut config = service.configuration().unwrap();
        config.enabled = true;
        config.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(config).unwrap();
        Self { core, api, root }
    }
    fn pair(&self, grant: Grant) -> (RequestContext, Credentials) {
        let service = self.core.collaboration().unwrap();
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let secret = "a".repeat(64);
        let pending = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Points agent".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&secret),
                },
                "fixture",
            )
            .unwrap();
        service.decide_request(&pending.request_id, true).unwrap();
        let credentials = service
            .collect(
                Collect {
                    request_id: pending.request_id,
                    exchange_secret: secret,
                },
                "fixture",
            )
            .unwrap();
        let session = RemoteSession::from_authorization(
            service
                .authenticate_live(&credentials.access_token, false, false)
                .unwrap(),
        )
        .unwrap();
        (RequestContext::Remote(session), credentials)
    }
    fn mutation(&self, id: &str, viewer: &str, delta: f64) -> Value {
        json!({"id":"wire","method":"points.adjust","params":{"uniqueId":viewer,"delta":delta},"meta":{"commandId":id,"hostInstanceId":self.core.collaboration().unwrap().instance_id()}})
    }
    async fn call(&self, context: &RequestContext, method: &str, params: Value) -> RpcResponse {
        self.api
            .execute_value_with_context(context, &json!({"id":1,"method":method,"params":params}))
            .await
    }
    async fn close(self) {
        self.core.shutdown().await;
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn grant() -> Grant {
    Grant {
        capabilities: [
            Capability::PointsRead,
            Capability::PointsAdjust,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(
            ResourceKind::Viewer,
            ["alice".into(), "missing".into()].into(),
        )]
        .into(),
        max_points_delta: Some(10.0),
        ..Grant::default()
    }
}

#[tokio::test]
async fn display_view_is_minimal_scoped_and_versions_the_union_including_missing_lookup() {
    let f = Fixture::new();
    f.core.points_adjust("alice", 3.0).unwrap();
    f.core.points_adjust("hidden", 100.0).unwrap();
    let (context, _) = f.pair(grant());
    let view = f
        .call(
            &context,
            "points.view",
            json!({"limit":20,"uniqueId":"alice"}),
        )
        .await;
    assert!(view.is_ok(), "{view:?}");
    let body = view.result.unwrap();
    assert_eq!(body["currencyName"], "Points");
    assert_eq!(body["viewers"].as_array().unwrap().len(), 1);
    assert_eq!(body["viewers"][0], body["selected"]);
    assert!(body["selected"].get("avatarUrl").is_none());
    assert!(body["selected"].get("totalCoins").is_none());
    assert_eq!(view.meta.unwrap().resource_versions.len(), 1);
    let missing = f
        .call(
            &context,
            "points.view",
            json!({"limit":0,"uniqueId":"missing"}),
        )
        .await;
    assert!(missing.is_ok(), "{missing:?}");
    assert_eq!(
        missing.result.unwrap(),
        json!({"currencyName":"Points","viewers":[],"selectedId":"missing","selected":null})
    );
    let versions = missing.meta.unwrap().resource_versions;
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].id, "missing");
    assert_eq!(versions[0].revision, "0");
    for id in ["hidden", "@alice", " alice "] {
        assert!(f
            .call(&context, "points.view", json!({"uniqueId":id}))
            .await
            .error
            .is_some());
    }
    let mut creator = grant();
    creator
        .resources
        .insert(ResourceKind::Live, ["creator".into()].into());
    let (creator, _) = f.pair(creator);
    assert_eq!(
        f.call(&creator, "points.view", json!({}))
            .await
            .error
            .unwrap()
            .code,
        "forbidden"
    );
    f.close().await;
}

#[tokio::test]
async fn additive_api_preserves_local_envelopes_and_returns_scoped_durable_remote_results() {
    let f = Fixture::new();
    let (context, credentials) = f.pair(grant());
    let (other, _) = f.pair(grant());
    let owner = f
        .call(
            &RequestContext::LocalOwner,
            "points.adjust",
            json!({"uniqueId":"bob","delta":50}),
        )
        .await;
    assert!(owner.is_ok());
    assert!(owner.meta.is_none());
    let schema = f
        .call(&context, "rpc.schema", json!({"method":"points.adjust"}))
        .await
        .result
        .unwrap();
    assert_eq!(schema["concurrency"], "additive");
    assert_eq!(schema["durableCommands"], true);
    let request = f.mutation("once", "alice", 3.0);
    let result = f.api.execute_value_with_context(&context, &request).await;
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(result.result.as_ref().unwrap()["totalPoints"], 3.0);
    let meta = result.meta.as_ref().unwrap();
    assert_eq!(meta.resource_versions[0].kind, ResourceKind::Viewer);
    assert_eq!(meta.resource_versions[0].revision, "1");
    assert_eq!(meta.command.as_ref().unwrap().command_id, "once");
    let board = f
        .call(&context, "points.leaderboard", json!({"limit":1}))
        .await;
    assert_eq!(board.result.unwrap()["viewers"][0]["uniqueId"], "alice");
    assert_eq!(board.meta.unwrap().resource_versions[0].id, "alice");
    let local = f
        .call(
            &RequestContext::LocalOwner,
            "points.viewer.get",
            json!({"uniqueId":"alice"}),
        )
        .await;
    assert_eq!(local.result.unwrap()["viewer"]["points"], 3.0);
    assert!(local.meta.is_none());
    let later = f
        .call(
            &RequestContext::LocalOwner,
            "points.adjust",
            json!({"uniqueId":"alice","delta":2}),
        )
        .await;
    assert!(later.is_ok());
    assert!(later.meta.is_none());
    let replay = f.api.execute_value_with_context(&context, &request).await;
    assert_eq!(replay.result, result.result);
    assert_eq!(replay.meta.unwrap().command, meta.command);
    assert_eq!(
        f.call(
            &RequestContext::LocalOwner,
            "points.viewer.get",
            json!({"uniqueId":"alice"})
        )
        .await
        .result
        .unwrap()["viewer"]["points"],
        5.0
    );
    let outcome = f
        .call(
            &context,
            "collaboration.commands.get",
            json!({"commandId":"once"}),
        )
        .await
        .result
        .unwrap();
    assert_eq!(outcome["principalId"], credentials.principal_id);
    assert_eq!(outcome["result"]["totalPoints"], 3.0);
    assert_eq!(
        f.call(
            &other,
            "collaboration.commands.get",
            json!({"commandId":"once"})
        )
        .await
        .error
        .unwrap()
        .code,
        "forbidden"
    );
    let mut changed = request.clone();
    changed["params"]["delta"] = json!(4);
    assert_eq!(
        f.api
            .execute_value_with_context(&context, &changed)
            .await
            .error
            .unwrap()
            .code,
        "command_id_reused"
    );
    assert_eq!(
        f.call(&context, "points.viewer.get", json!({"uniqueId":"bob"}))
            .await
            .error
            .unwrap()
            .code,
        "forbidden"
    );
    let missing = f
        .call(&context, "points.viewer.get", json!({"uniqueId":"missing"}))
        .await;
    assert_eq!(missing.error.as_ref().unwrap().code, "viewer_not_found");
    assert_eq!(
        missing.error.unwrap().data.unwrap()["resourceVersions"][0]["revision"],
        "0"
    );
    f.close().await;
}

#[tokio::test]
async fn missing_metadata_cas_preconditions_and_ungranted_operations_never_adjust_points() {
    let f = Fixture::new();
    let (context, _) = f.pair(grant());
    assert_eq!(
        f.call(
            &context,
            "points.adjust",
            json!({"uniqueId":"alice","delta":1})
        )
        .await
        .error
        .unwrap()
        .code,
        "invalid_params"
    );
    let base = f.mutation("metadata", "alice", 1.0);
    for metadata in [
        json!({"commandId":"only"}),
        json!({"hostInstanceId":"only"}),
        json!({"commandId":"bad id","hostInstanceId":"instance"}),
    ] {
        let mut request = base.clone();
        request["meta"] = metadata;
        assert_eq!(
            f.api
                .execute_value_with_context(&context, &request)
                .await
                .error
                .unwrap()
                .code,
            "invalid_params"
        );
    }
    let mut cas = base.clone();
    cas["meta"]["expectedRevision"] = json!("0");
    assert_eq!(
        f.api
            .execute_value_with_context(&context, &cas)
            .await
            .error
            .unwrap()
            .code,
        "invalid_params"
    );
    let mut excessive = base.clone();
    excessive["params"]["delta"] = json!(11);
    assert_eq!(
        f.api
            .execute_value_with_context(&context, &excessive)
            .await
            .error
            .unwrap()
            .code,
        "forbidden"
    );
    let mut no_read = grant();
    no_read.capabilities.remove(&Capability::PointsRead);
    let (no_read, _) = f.pair(no_read);
    assert_eq!(
        f.api
            .execute_value_with_context(&no_read, &base)
            .await
            .error
            .unwrap()
            .code,
        "forbidden"
    );
    let mut restricted = grant();
    restricted
        .resources
        .insert(ResourceKind::Live, ["creator".into()].into());
    let (restricted, _) = f.pair(restricted);
    for method in [
        "points.adjust",
        "points.info",
        "points.viewer.get",
        "points.leaderboard",
        "points.reset",
        "points.config.set",
    ] {
        assert_eq!(
            f.call(&restricted, method, json!({}))
                .await
                .error
                .unwrap()
                .code,
            "forbidden"
        );
    }
    for method in ["points.reset", "points.config.set", "points.config.get"] {
        assert_eq!(
            f.call(&context, method, json!({}))
                .await
                .error
                .unwrap()
                .code,
            "forbidden"
        );
    }
    let missing = f
        .call(&context, "points.viewer.get", json!({"uniqueId":"alice"}))
        .await;
    assert_eq!(missing.error.unwrap().code, "viewer_not_found");
    assert!(f
        .api
        .execute_value_with_context(&context, &base)
        .await
        .is_ok());
    f.close().await;
}

#[tokio::test]
async fn points_info_is_read_scoped_and_checked_currency_is_an_optional_durable_precondition() {
    let f = Fixture::new();
    let (context, _) = f.pair(grant());
    let info = f.call(&context, "points.info", json!({})).await;
    assert_eq!(info.result.unwrap(), json!({"currencyName":"Points"}));
    let mut input = f.mutation("checked", "alice", 2.0);
    input["params"]["expectedCurrency"] = json!("Wrong unit");
    assert_eq!(
        f.api
            .execute_value_with_context(&context, &input)
            .await
            .error
            .unwrap()
            .code,
        "currency_changed"
    );
    input["params"]["expectedCurrency"] = json!("Points");
    let original = f.api.execute_value_with_context(&context, &input).await;
    assert!(original.is_ok(), "{original:?}");
    f.core
        .points_update_config(serde_json::from_value(json!({"currencyName":"Gems"})).unwrap());
    assert_eq!(
        f.call(&context, "points.info", json!({}))
            .await
            .result
            .unwrap()["currencyName"],
        "Gems"
    );
    assert_eq!(
        f.api
            .execute_value_with_context(&context, &input)
            .await
            .result,
        original.result
    );
    assert_eq!(
        f.call(
            &RequestContext::LocalOwner,
            "points.adjust",
            json!({"uniqueId":"alice","delta":1,"expectedCurrency":"Gems"})
        )
        .await
        .error
        .unwrap()
        .code,
        "invalid_params"
    );
    let mut no_read = grant();
    no_read.capabilities.remove(&Capability::PointsRead);
    let (no_read, _) = f.pair(no_read);
    assert_eq!(
        f.call(&no_read, "points.info", json!({}))
            .await
            .error
            .unwrap()
            .code,
        "forbidden"
    );
    f.close().await;
}

#[tokio::test]
async fn scope_changes_reauthorize_queued_points_results_outcomes_and_failure_versions() {
    let f = Fixture::new();
    let (context, credentials) = f.pair(grant());
    let mutation = f
        .api
        .execute_value_with_context(&context, &f.mutation("queued", "alice", 3.0))
        .await
        .result
        .unwrap();
    let viewer = f
        .call(&context, "points.viewer.get", json!({"uniqueId":"alice"}))
        .await
        .result
        .unwrap();
    let board = f
        .call(&context, "points.leaderboard", json!({}))
        .await
        .result
        .unwrap();
    let display = f
        .call(
            &context,
            "points.view",
            json!({"limit":20,"uniqueId":"missing"}),
        )
        .await
        .result
        .unwrap();
    let outcome = f
        .call(
            &context,
            "collaboration.commands.get",
            json!({"commandId":"queued"}),
        )
        .await
        .result
        .unwrap();
    let missing = f
        .call(&context, "points.viewer.get", json!({"uniqueId":"missing"}))
        .await
        .error
        .unwrap();
    let mut narrowed = grant();
    narrowed
        .resources
        .insert(ResourceKind::Viewer, Default::default());
    f.core
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: credentials.principal_id,
            expected_revision: "1".into(),
            grant: narrowed,
        })
        .unwrap();
    let RequestContext::Remote(session) = &context else {
        panic!("remote session")
    };
    for (method, body) in [
        ("points.adjust", &mutation),
        ("points.viewer.get", &viewer),
        ("points.leaderboard", &board),
        ("points.view", &display),
        ("collaboration.commands.get", &outcome),
    ] {
        assert_eq!(
            f.api
                .authorize_remote_response(session, method, body)
                .unwrap_err()
                .code,
            "forbidden"
        );
    }
    assert_eq!(
        f.api
            .authorize_remote_failure(session, "points.viewer.get", missing.data.as_ref())
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert_eq!(
        f.call(
            &context,
            "collaboration.commands.get",
            json!({"commandId":"queued"})
        )
        .await
        .error
        .unwrap()
        .code,
        "forbidden"
    );
    f.close().await;
}
