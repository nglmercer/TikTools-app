use super::*;
use crate::paths::AppPaths;
use std::path::PathBuf;

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

fn fixture(capabilities: Vec<&str>) -> (Arc<AppCore>, PathBuf) {
    let root = std::env::temp_dir().join(format!("tiktools-live-platforms-{}", fastrand::u128(..)));
    let directory = root.join("packages/acme.twitch");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("entry.bin"), b"fixture").unwrap();
    std::fs::write(
        directory.join("plugin.json"),
        json!({
            "schemaVersion":3,
            "id":"acme.twitch",
            "name":"Fixture Twitch",
            "version":"1.0.0",
            "runtime":"process",
            "entry":"entry.bin",
            "capabilities":capabilities,
            "eventTypes":[{"type":"platform.live.ingest","title":{"default":"Live ingest"}}]
        })
        .to_string(),
    )
    .unwrap();
    let paths = AppPaths {
        root: root.clone(),
        data: root.join("data"),
        plugins: root.join("packages"),
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
    (core, root)
}

fn batch(messages: Value) -> Value {
    json!({"schemaVersion":1,"messages":messages})
}

fn session_start() -> Value {
    json!({
        "kind":"session_start",
        "stream":"main",
        "channel":{"id":"channel-1","handle":"streamer","roomId":"room-1"},
        "capabilities":{"chat":true,"follows":true,"gifts":true,"likes":true,"shares":true,"joins":true,"roomStats":true}
    })
}

fn chat(source: &str) -> Value {
    json!({
        "kind":"event",
        "stream":"main",
        "event":{
            "sourceEventId":source,
            "user":{"id":"user-1","handle":"viewer","displayName":"Viewer"},
            "payload":{"kind":"chat","comment":"hello"}
        }
    })
}

#[tokio::test]
async fn validated_batches_create_sessions_feed_rows_and_deduplicate_sources() {
    let (core, root) = fixture(vec!["events.publish", "live.ingest"]);
    assert!(core.plugins.get("acme.twitch").is_some());

    let outcome = core
        .ingest_live_platform_batch(
            "acme.twitch",
            batch(json!([session_start(), chat("source-1")])),
        )
        .await
        .expect("valid batch");
    assert_eq!(outcome.accepted, 2);
    assert_eq!(outcome.rejected, 0);
    let sessions = core.live_platform_sessions();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].platform, "acme.twitch");
    assert_eq!(sessions[0].channel.as_ref().unwrap().handle, "streamer");

    let creator = identity_key("acme.twitch", "channel-1");
    let feed = core
        .events
        .creator_feed(&creator, None, 0, 10, 64 * 1024)
        .expect("creator feed");
    assert!(feed.events.iter().any(|event| {
        event.kind == "chat"
            && event.text == "hello"
            && event.author == identity_key("acme.twitch", "user-1")
    }));

    let duplicate = core
        .ingest_live_platform_batch("acme.twitch", batch(json!([chat("source-1")])))
        .await
        .expect("duplicate batch");
    assert_eq!(duplicate.duplicates, 1);
    assert_eq!(duplicate.accepted, 0);

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn ingest_requires_capability_validates_payloads_and_never_accepts_raw_live_events() {
    let (core, root) = fixture(vec!["events.publish"]);
    let error = core
        .ingest_live_platform_batch("acme.twitch", batch(json!([session_start()])))
        .await
        .expect_err("live.ingest is required");
    assert!(error.contains("capability") || error.contains("live.ingest"));
    let _ = std::fs::remove_dir_all(root);

    let (core, root) = fixture(vec!["events.publish", "live.ingest"]);
    let mut invalid = chat("bad");
    invalid["event"]["payload"]["comment"] = json!("x".repeat(5000));
    let outcome = core
        .ingest_live_platform_batch(
            "acme.twitch",
            batch(json!([session_start(), invalid, chat("good")])),
        )
        .await
        .expect("per-message validation outcome");
    assert_eq!(outcome.accepted, 2);
    assert_eq!(outcome.rejected, 1);
    let _ = std::fs::remove_dir_all(root);
}
