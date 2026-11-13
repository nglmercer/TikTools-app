use super::*;
use tiktools_live::*;

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}
fn core() -> Arc<AppCore> {
    let root = std::env::temp_dir().join(format!(
        "tiktools-platform-{}-{}",
        std::process::id(),
        fastrand::u64(..)
    ));
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin-plugins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    };
    Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        paths,
    ))
}
fn event(platform: PlatformId, id: &str) -> LiveEnvelope {
    LiveEnvelope {
        schema_version: 1,
        platform,
        connection_id: format!("{}-1", platform.as_str()),
        channel: ChannelIdentity {
            platform,
            id: "channel-1".into(),
            handle: "creator".into(),
            room_id: "room-1".into(),
        },
        session_id: identity_key(platform, "room-1"),
        source_event_id: Some(id.into()),
        occurred_at: 1,
        received_at: 1000,
        is_history: false,
        user: Some(ViewerIdentity {
            platform,
            id: "42".into(),
            handle: "alice".into(),
            display_name: "Alice".into(),
            avatar_url: None,
        }),
        payload: LivePayload::Chat {
            comment: "hello".into(),
            emotes: vec![],
        },
        extensions: Default::default(),
    }
}
#[tokio::test]
async fn platform_pipeline_isolates_points_and_skips_replays_and_history() {
    let core = core();
    let a = event(PlatformId::Twitch, "same-id");
    core.handle_platform_event(a.clone()).await;
    core.handle_platform_event(a).await;
    core.handle_platform_event(event(PlatformId::Youtube, "same-id"))
        .await;
    let mut history = event(PlatformId::Twitch, "history");
    history.is_history = true;
    core.handle_platform_event(history).await;
    assert_eq!(core.points.viewer("twitch:2:42").unwrap()["points"], 1.0);
    assert_eq!(core.points.viewer("youtube:2:42").unwrap()["points"], 1.0);
    assert!(core.points.viewer("alice").is_none());
    #[cfg(feature = "persistence")]
    {
        let stats = core.db.platform_analytics().unwrap();
        assert_eq!(stats.len(), 2);
        assert!(stats.iter().all(|row| row["count"] == 1));
    }
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(&core.db.paths().root);
}
#[tokio::test]
async fn credentials_use_opaque_refs_and_never_appear_in_snapshots() {
    let core = core();
    let saved = core
        .live_platform_store_credential(control::PlatformCredentialParams {
            platform: PlatformId::Twitch,
            token: "test-private-token".into(),
        })
        .await
        .unwrap();
    assert_eq!(saved.credential_ref.len(), 32);
    assert!(!serde_json::to_string(&saved)
        .unwrap()
        .contains("test-private-token"));
    let path = core
        .db
        .paths()
        .data
        .join("platform-credentials")
        .join(format!("twitch-{}", saved.credential_ref));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert!(core
        .live_platform_delete_credential(PlatformId::Twitch, "../../secret")
        .await
        .is_err());
    core.live_platform_delete_credential(PlatformId::Twitch, &saved.credential_ref)
        .await
        .unwrap();
    assert!(!path.exists());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(&core.db.paths().root);
}
#[test]
fn generic_aliases_match_each_rule_once_and_do_not_match_other_platforms_as_tiktok() {
    let service = AutomationService::default();
    service.replace_snapshot(&json!({"actions":[{"id":"log","enabled":true,"typeId":"core.log"}],"events":[
        {"id":"legacy","enabled":true,"trigger":"tiktok.chat","actionIds":["log"]},
        {"id":"shared","enabled":true,"trigger":"live.chat","actionIds":["log"],"filters":[{"path":"event.type","operator":"eq","value":"live.chat"}]}
    ]}));
    let tiktok = service.matching_events(
        &json!({"type":"tiktok.chat","platform":"tiktok","data":{"comment":"hello"}}),
    );
    assert_eq!(tiktok.len(), 2);
    assert_eq!(tiktok.iter().filter(|e| e["id"] == "shared").count(), 1);
    let twitch = service.matching_events(
        &json!({"type":"live.chat","platform":"twitch","data":{"comment":"hello"}}),
    );
    assert_eq!(twitch.len(), 1);
    assert_eq!(twitch[0]["id"], "shared");
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn additive_migration_preserves_tiktok_balances_and_does_not_relabel_new_providers() {
    let core = core();
    let db = rusqlite::Connection::open(core.db.points_path()).unwrap();
    db.execute(
        "INSERT INTO viewers(unique_id,user_id,nickname,points) VALUES ('alice','42','Alice',7.5)",
        [],
    )
    .unwrap();
    let _reopened = crate::db::DatabaseManager::new(core.db.paths().clone());
    core.db
        .record_platform_activity(&event(PlatformId::Twitch, "new"))
        .unwrap();
    let _reopened = crate::db::DatabaseManager::new(core.db.paths().clone());
    assert_eq!(
        db.query_row(
            "SELECT points FROM viewers WHERE unique_id='alice'",
            [],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        7.5
    );
    let identities:Vec<(String,String)>=db.prepare("SELECT platform,storage_key FROM platform_viewers WHERE user_id='42' ORDER BY platform").unwrap().query_map([],|r|Ok((r.get(0)?,r.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(
        identities,
        vec![
            ("tiktok".into(), "alice".into()),
            ("twitch".into(), "twitch:2:42".into())
        ]
    );
    drop(db);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(&core.db.paths().root);
}
