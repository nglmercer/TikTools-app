#![cfg(feature = "persistence")]
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_aggregation::model::{Definition as Ranking, SourceRule};
use tiktools_competitions::{CompetitionKind, Definition, SelectionMode, TiePolicy, WinnerAction};
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext};
use tiktools_core::{
    collaboration::*, ipc::messages::HostMessage, paths::AppPaths, AppCore, HostEmitter,
    NoopMediaHost,
};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

#[tokio::test]
async fn scoped_competition_summary_is_bounded_redacted_and_reauthorized() {
    let root = std::env::temp_dir().join(format!(
        "tt-competition-scope-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let paths = AppPaths {
        root: root.clone(),
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
    };
    let core = Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        paths,
    ));
    core.ranking_create(Ranking {
        schema_version: 1,
        id: "private-ranking".into(),
        creator_id: "owner".into(),
        room_id: "private-room".into(),
        group_by: Default::default(),
        window: Default::default(),
        user_limits: None,
        capacity: 10,
        limit: 10,
        sources: vec![SourceRule {
            source: "custom.score".into(),
            metric: "value".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
    })
    .unwrap();
    for id in ["alpha", "bravo", "charlie"] {
        core.competition_create(Definition {
            schema_version: 1,
            id: id.into(),
            name: format!("Competition {id}"),
            kind: CompetitionKind::LeaderboardChallenge,
            ranking_id: "private-ranking".into(),
            options: vec![],
            selection_mode: SelectionMode::HighestScore,
            tie_policy: TiePolicy::Stable,
            duration_ms: None,
            vote_limit: 1,
            allow_switch: false,
            minimum_participants: 1,
            target_score: None,
            on_user_win: Some(WinnerAction {
                action_id: "private-action".into(),
                inputs: json!({"secret":"do-not-send"}),
            }),
        })
        .unwrap();
    }
    let api = ControlApi::new(core.clone());
    let service = core.collaboration().unwrap();
    let mut configuration = service.configuration().unwrap();
    configuration.enabled = true;
    configuration.advertised_origin = Some("http://127.0.0.1:8788".into());
    service.configure(configuration).unwrap();
    let grant = Grant {
        capabilities: [Capability::CompetitionsRead].into(),
        resources: [(
            ResourceKind::Competition,
            ["alpha".into(), "charlie".into()].into(),
        )]
        .into(),
        ..Grant::default()
    };
    let invite = service
        .create_invitation(InviteCreate {
            grant: grant.clone(),
        })
        .unwrap();
    let secret = "a".repeat(64);
    let pending = service
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "moderator".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&secret),
            },
            "test",
        )
        .unwrap();
    service.decide_request(&pending.request_id, true).unwrap();
    let credentials = service
        .collect(
            Collect {
                request_id: pending.request_id,
                exchange_secret: secret,
            },
            "test",
        )
        .unwrap();
    let session = RemoteSession::from_authorization(
        service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap(),
    )
    .unwrap();
    let context = RequestContext::Remote(session.clone());
    let call = |params: Value| {
        let api = &api;
        let context = &context;
        async move {
            api.execute_value_with_context(
                context,
                &json!({"id":1,"method":"competitions.summary","params":params}),
            )
            .await
        }
    };
    let first = call(json!({"limit":1})).await.result.unwrap();
    assert_eq!(first["competitions"][0]["id"], "alpha");
    assert_eq!(first["competitions"][0]["lifecycle"], "draft");
    assert_eq!(first["nextAfter"], "alpha");
    let mut extended = first.clone();
    extended["competitions"][0]["privateAction"] = json!("secret");
    assert_eq!(
        api.authorize_remote_response(&session, "competitions.summary", &extended)
            .unwrap_err()
            .code,
        "forbidden"
    );
    let mut nested = first.clone();
    nested["competitions"][0]["name"] = json!({"privateAction":"secret"});
    assert_eq!(
        api.authorize_remote_response(&session, "competitions.summary", &nested)
            .unwrap_err()
            .code,
        "forbidden"
    );
    for secret in [
        "bravo",
        "charlie",
        "private-ranking",
        "private-room",
        "private-action",
        "do-not-send",
    ] {
        assert!(!first.to_string().contains(secret));
    }
    let second = call(json!({"after":"alpha","limit":1}))
        .await
        .result
        .unwrap();
    assert_eq!(second["competitions"][0]["id"], "charlie");
    assert!(second["nextAfter"].is_null());
    assert!(call(json!({"limit":51})).await.error.is_some());
    assert!(api
        .execute_value_with_context(
            &context,
            &json!({"id":2,"method":"competitions.start","params":{"id":"alpha"}})
        )
        .await
        .error
        .is_some());
    let mut narrowed = grant.clone();
    narrowed
        .resources
        .insert(ResourceKind::Competition, ["charlie".into()].into());
    service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id.clone(),
            expected_revision: "1".into(),
            grant: narrowed.clone(),
        })
        .unwrap();
    assert_eq!(
        api.authorize_remote_response(&session, "competitions.summary", &first)
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert_eq!(
        call(json!({})).await.result.unwrap()["competitions"][0]["id"],
        "charlie"
    );
    narrowed
        .resources
        .insert(ResourceKind::Live, ["creator".into()].into());
    service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id.clone(),
            expected_revision: "2".into(),
            grant: narrowed.clone(),
        })
        .unwrap();
    assert!(call(json!({})).await.error.is_some());
    assert_eq!(
        api.authorize_remote_response(&session, "competitions.summary", &second)
            .unwrap_err()
            .code,
        "forbidden"
    );
    narrowed.resources.remove(&ResourceKind::Live);
    narrowed.capabilities.remove(&Capability::CompetitionsRead);
    service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id,
            expected_revision: "3".into(),
            grant: narrowed,
        })
        .unwrap();
    assert_eq!(
        api.authorize_remote_response(&session, "competitions.summary", &second)
            .unwrap_err()
            .code,
        "forbidden"
    );
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}
