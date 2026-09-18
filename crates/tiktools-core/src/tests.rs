use std::{
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use super::*;
use tiktools_plugin_api::{
    AudioPlayOptions, AudioPlaybackResult, MediaFileRef, MediaPickerOptions,
};

#[derive(Default)]
struct RecordingEmitter {
    messages: Mutex<Vec<HostMessage>>,
}

impl HostEmitter for RecordingEmitter {
    fn emit(&self, message: HostMessage) {
        self.messages
            .lock()
            .expect("test emitter poisoned")
            .push(message);
    }
}

#[derive(Default)]
struct RecordingMediaHost {
    selected: Mutex<Option<PathBuf>>,
    played: Mutex<Vec<MediaFileRef>>,
}

impl MediaHost for RecordingMediaHost {
    fn open_picker(&self, _options: MediaPickerOptions) -> MediaHostFuture<Option<PathBuf>> {
        let selected = self
            .selected
            .lock()
            .expect("media selection lock poisoned")
            .clone();
        Box::pin(async move { Ok(selected) })
    }

    fn play_audio(
        &self,
        file: MediaFileRef,
        _options: AudioPlayOptions,
    ) -> MediaHostFuture<AudioPlaybackResult> {
        self.played
            .lock()
            .expect("media playback lock poisoned")
            .push(file);
        Box::pin(async {
            Ok(AudioPlaybackResult {
                played: true,
                reason: None,
                active_players: 1,
            })
        })
    }
}

fn media_fixture() -> (PathBuf, PathBuf) {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("tiktools-core-media-{suffix}"));
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("alert.wav");
    std::fs::write(&file, b"fixture").unwrap();
    (root, file)
}

#[tokio::test]
async fn native_event_runner_executes_saved_behavior() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter.clone()));
    core.automation.replace_snapshot(&json!({
        "actions": [{
            "id": "say-hello",
            "name": "Say hello",
            "typeId": "core.log",
            "enabled": true,
            "config": {"message": "hello {{ event.user.uniqueId }}"}
        }],
        "events": [{
            "id": "chat-event",
            "name": "Chat event",
            "enabled": true,
            "trigger": "tiktok.chat",
            "filters": [],
            "cooldownMs": 0,
            "cooldownScope": "user",
            "actionIds": ["say-hello"],
            "runMode": "all"
        }]
    }));

    core.publish_automation_event(json!({
        "id": "chat-1",
        "type": "tiktok.chat",
        "timestamp": 1,
        "user": {"uniqueId": "alice"},
        "data": {"comment": "hello"}
    }))
    .await;

    let runs = core.automation.recent_runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["status"], "ok");
    assert_eq!(runs[0]["eventId"], "chat-event");
    assert_eq!(runs[0]["summary"], "hello alice");
    assert!(emitter
        .messages
        .lock()
        .expect("test emitter poisoned")
        .iter()
        .any(|message| matches!(message, HostMessage::BehaviorRuns { .. })));
}

#[tokio::test]
async fn hotkey_status_event_reaches_the_ui_message_boundary() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter.clone()));

    core.publish_automation_event(json!({
        "id": "hotkey-status-1",
        "type": "hotkey.status",
        "timestamp": 1,
        "data": {
            "platform": "linux",
            "session": "wayland",
            "backends": [{
                "backend": "evdev",
                "state": "permission required",
                "detail": "no readable devices",
                "summary": "Global Hotkeys: permission required via raw input (evdev)"
            }]
        }
    }))
    .await;

    assert!(emitter
        .messages
        .lock()
        .expect("test emitter poisoned")
        .iter()
        .any(|message| matches!(
            message,
            HostMessage::HotkeyStatus { status }
                if status["session"] == "wayland"
        )));
}

#[tokio::test]
async fn public_media_api_returns_a_reference_and_revalidates_playback() {
    let (root, file) = media_fixture();
    let emitter = Arc::new(RecordingEmitter::default());
    let media = Arc::new(RecordingMediaHost::default());
    *media
        .selected
        .lock()
        .expect("media selection lock poisoned") = Some(file.clone());
    let core = Arc::new(AppCore::with_media_host(emitter, media.clone()));

    let selection = core
        .open_media_picker(MediaPickerOptions::default())
        .await
        .unwrap()
        .expect("fixture should be selected");
    let file_ref = match selection {
        tiktools_plugin_api::MediaSelection::File { file } => file,
        tiktools_plugin_api::MediaSelection::Directory { .. } => {
            panic!("expected a file selection")
        }
    };
    assert_eq!(
        file_ref.path,
        std::fs::canonicalize(&file).unwrap().to_string_lossy()
    );
    assert_eq!(file_ref.size_bytes, 7);

    core.play_audio(
        MediaFileRef::from_path(file.to_string_lossy()),
        AudioPlayOptions::default(),
    )
    .await
    .unwrap();
    let played = media.played.lock().expect("media playback lock poisoned");
    assert_eq!(played.len(), 1);
    assert_eq!(played[0].path, file_ref.path);
    drop(played);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn napi_vm_audio_intent_uses_the_same_validated_media_api() {
    let (root, file) = media_fixture();
    let emitter = Arc::new(RecordingEmitter::default());
    let media = Arc::new(RecordingMediaHost::default());
    let core = Arc::new(AppCore::with_media_host(emitter, media.clone()));

    let action = json!({
        "id": "vm-audio",
        "name": "VM audio",
        "typeId": "core.code",
        "config": {
            "source": format!(
                "return {{ playAudio: {{ fileRef: {{ path: {:?} }}, volume: 0.5 }} }};",
                file.to_string_lossy()
            )
        }
    });
    let run = core.execute_action(&action, &json!({}), None, false).await;

    assert_eq!(run["status"], "ok");
    let played = media.played.lock().expect("media playback lock poisoned");
    assert_eq!(played.len(), 1);
    assert_eq!(
        played[0].path,
        std::fs::canonicalize(&file).unwrap().to_string_lossy()
    );
    drop(played);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_action_is_a_dry_run_for_audio() {
    let (root, file) = media_fixture();
    let emitter = Arc::new(RecordingEmitter::default());
    let media = Arc::new(RecordingMediaHost::default());
    let core = Arc::new(AppCore::with_media_host(emitter, media.clone()));

    let run = core
        .test_action(
            &json!({
                "id": "vm-audio-test",
                "name": "VM audio test",
                "typeId": "core.code",
                "config": {
                    "source": format!(
                        "return {{ playAudio: {{ fileRef: {{ path: {:?} }}, volume: 0.5 }} }};",
                        file.to_string_lossy()
                    )
                }
            }),
            None,
        )
        .await;

    assert_eq!(run["status"], "ok");
    assert!(run["summary"].as_str().unwrap().contains("would play"));
    assert!(media
        .played
        .lock()
        .expect("media playback lock poisoned")
        .is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(feature = "native-tiktok")]
#[tokio::test]
async fn native_live_event_reaches_the_domain_boundary() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter.clone()));
    let mut domain = core.events.subscribe_domain();
    core.handle_native_event(ClientEvent::Event(NativeLiveEvent {
        base: tiktools_tiktok::events::CanonicalLiveEvent::Chat(
            tiktools_tiktok::events::ChatEvent {
                user: tiktools_tiktok::events::EventUser {
                    id: 42,
                    unique_id: "alice".to_owned(),
                    nickname: "Alice".to_owned(),
                    sec_uid: String::new(),
                    avatar_url: None,
                },
                comment: "hello".to_owned(),
            },
        ),
        metadata: tiktools_tiktok::events::EventMetadata {
            method: "WebcastChatMessage".to_owned(),
            msg_id: 1,
            is_history: false,
        },
        gift: None,
    }))
    .await;

    // The legacy `live-event` push was removed once the frontend migrated;
    // the UI-ready chat event now travels as `live.ui-event` on the domain
    // bus, while the leaderboard snapshot push (no domain twin) stays.
    let mut saw_ui_event = false;
    while let Ok(event) = domain.try_recv() {
        if let crate::events::DomainEvent::LiveUiEvent { event } = event {
            if event.get("kind").and_then(Value::as_str) == Some("chat") {
                saw_ui_event = true;
            }
        }
    }
    assert!(saw_ui_event, "live.ui-event domain event missing");
    let messages = emitter.messages.lock().expect("test emitter poisoned");
    assert!(messages
        .iter()
        .any(|message| matches!(message, HostMessage::Leaderboard { .. })));
}

#[tokio::test]
async fn plugin_typed_event_runs_while_its_plugin_is_enabled() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter.clone()));
    let snapshot = serde_json::json!({
        "actions": [{
            "id": "say-key",
            "name": "Say key",
            "typeId": "core.log",
            "enabled": true,
            "config": {"message": "key {{ event.data.key }}"}
        }],
        "events": [{
            "id": "hotkey-event",
            "name": "Hotkey event",
            "enabled": true,
            "trigger": "hotkey.pressed",
            "filters": [],
            "cooldownMs": 0,
            "cooldownScope": "user",
            "actionIds": ["say-key"],
            "runMode": "all"
        }],
        "eventTypes": [{
            "type": "hotkey.pressed",
            "title": {"default": "Hotkey pressed"},
            "source": {"kind": "plugin", "pluginId": "hotkeys"}
        }],
        "plugins": [{
            "descriptor": {"id": "hotkeys"},
            "installed": true,
            "enabled": true,
            "available": true
        }]
    });
    core.automation.replace_snapshot(&snapshot);

    core.publish_automation_event(serde_json::json!({
        "id": "hk-1",
        "type": "hotkey.pressed",
        "timestamp": 1,
        "user": {"uniqueId": "alice"},
        "data": {"key": "ctrl+k", "depth": 1}
    }))
    .await;

    let runs = core.automation.recent_runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["status"], "ok");
    assert_eq!(runs[0]["summary"], "key ctrl+k");

    // Disabling the plugin pauses its triggers without touching the record.
    let mut disabled = snapshot.clone();
    disabled["plugins"][0]["enabled"] = serde_json::Value::Bool(false);
    core.automation.replace_snapshot(&disabled);

    core.publish_automation_event(serde_json::json!({
        "id": "hk-2",
        "type": "hotkey.pressed",
        "timestamp": 2,
        "user": {"uniqueId": "alice"},
        "data": {"key": "ctrl+k", "depth": 1}
    }))
    .await;

    assert_eq!(core.automation.recent_runs().len(), 1);
}

#[tokio::test]
async fn plugin_typed_event_respects_declaration_and_capability() {
    use tiktools_plugin_api::manifest::PluginManifest;

    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter));
    let manifest = PluginManifest::from_json_str(
        r#"{"schemaVersion":2,"id":"hotkeys","name":"Hotkeys","version":"1.0.0","runtime":"process","entry":"hotkeys","capabilities":["events.publish"],"eventTypes":[{"type":"hotkey.pressed","title":{"default":"Hotkey pressed"}}]}"#,
    )
    .expect("fixture manifest should parse");

    let source = serde_json::json!({"id": "src-1"});
    let typed = core
        .plugin_typed_event(
            &manifest,
            "hotkey.pressed",
            &serde_json::json!({"key": "k"}),
            &source,
        )
        .expect("declared type with capability should publish")
        .expect("declared type should not fall back to plugin.emit");
    assert_eq!(typed["type"], "hotkey.pressed");
    assert_eq!(typed["data"]["key"], "k");
    assert_eq!(typed["sourceEventId"], "src-1");
    assert_eq!(typed["data"]["depth"], 1);

    // Undeclared types keep the internal plugin.emit channel.
    assert!(core
        .plugin_typed_event(&manifest, "other.thing", &serde_json::json!({}), &source)
        .expect("undeclared type should not error")
        .is_none());

    // Declared but unpermitted types fail loudly instead of misrouting.
    let bare = PluginManifest::from_json_str(
        r#"{"schemaVersion":2,"id":"hotkeys","name":"Hotkeys","version":"1.0.0","runtime":"process","entry":"hotkeys","eventTypes":[{"type":"hotkey.pressed","title":{"default":"Hotkey pressed"}}]}"#,
    )
    .expect("fixture manifest should parse");
    let error = core
        .plugin_typed_event(&bare, "hotkey.pressed", &serde_json::json!({}), &source)
        .expect_err("missing capability should error");
    assert!(error.contains("events.publish"), "{error}");
}

#[tokio::test]
async fn sequential_polled_presses_start_fresh_chains() {
    // Regression test: poll ticks used to derive each new event's depth from
    // the previously remembered event, so after a couple of keypresses every
    // further press exceeded the emit depth limit and was dropped forever.
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter));
    core.automation.replace_snapshot(&json!({
        "actions": [{
            "id": "say-key",
            "name": "Say key",
            "typeId": "core.log",
            "enabled": true,
            "config": {"message": "key {{ event.data.key }}"}
        }],
        "events": [{
            "id": "hotkey-event",
            "name": "Hotkey event",
            "enabled": true,
            "trigger": "hotkey.pressed",
            "filters": [],
            "cooldownMs": 0,
            "cooldownScope": "user",
            "actionIds": ["say-key"],
            "runMode": "all"
        }],
        "eventTypes": [{
            "type": "hotkey.pressed",
            "title": {"default": "Hotkey pressed"},
            "source": {"kind": "plugin", "pluginId": "hotkeys"}
        }],
        "plugins": [{
            "descriptor": {"id": "hotkeys"},
            "installed": true,
            "enabled": true,
            "available": true
        }]
    }));

    // Simulate sequential poll ticks, each building context from the last
    // published event exactly like poll_plugin_events does.
    let mut source = json!({});
    for _ in 0..5 {
        let context = crate::fresh_poll_context(&source);
        let event = core.make_plugin_event(&context, "hotkey.pressed", json!({"key": "k"}));
        assert_eq!(event["data"]["depth"], json!(1));
        source = event.clone();
        core.publish_automation_event(event).await;
    }

    assert_eq!(core.automation.recent_runs().len(), 5);
}

#[tokio::test]
async fn test_event_names_sample_data_on_mismatch() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter));
    let result = core
        .test_event(&serde_json::json!({
            "id": "evt-1",
            "name": "Mismatch",
            "enabled": true,
            "trigger": "tiktok.chat",
            "filters": [{"path": "event.data.comment", "operator": "eq", "value": "zzz-no-match"}],
            "cooldownMs": 0,
            "cooldownScope": "user",
            "actionIds": [],
            "runMode": "all"
        }))
        .await;
    assert_eq!(result["status"], "error");
    let summary = result["summary"].as_str().unwrap_or_default();
    assert!(summary.contains("sample data:"), "{summary}");
    assert!(summary.contains("hello"), "{summary}");
}

#[test]
fn manual_points_adjustment_publishes_points_changed() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = AppCore::new(emitter);
    // Unique viewer per run: the points store persists across runs, so a
    // fixed name would accumulate totals and flake the exact-value assert.
    let unique_id = format!(
        "alice-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    );
    let mut events = core.events.subscribe_domain();
    let award = core
        .points_adjust(&unique_id, 10.0)
        .expect("manual adjustment works");
    assert_eq!(award.total_points, 10.0);
    let event = events.try_recv().expect("points.changed is published");
    assert_eq!(
        event,
        crate::events::DomainEvent::PointsChanged {
            unique_id,
            delta: award.delta,
            total_points: award.total_points,
            level: award.level,
        }
    );
}

#[test]
fn normalized_live_event_publishes_domain_event_before_automation() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = AppCore::new(emitter);
    let mut events = core.events.subscribe_domain();
    // The authoritative fan-out happens on the normalized event alone, with
    // no automation slot, enrichment, or pipeline involved.
    core.publish_live_domain_event(&serde_json::json!({
        "type": "tiktok.chat",
        "data": {"comment": "hello"},
    }));
    let event = events.try_recv().expect("live.event is published");
    match event {
        crate::events::DomainEvent::LiveEvent { event_type, .. } => {
            assert_eq!(event_type, "tiktok.chat");
        }
        other => panic!("unexpected event: {other:?}"),
    }
    // Non-TikTok automation types (points, hotkeys) have their own topics
    // and must not synthesize a live.event.
    core.publish_live_domain_event(&serde_json::json!({
        "type": "points.awarded",
        "data": {},
    }));
    assert!(events.try_recv().is_err());
}

#[cfg(feature = "native-tiktok")]
#[tokio::test]
async fn saturated_automation_slots_do_not_drop_domain_events() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter));
    // Occupy every automation slot so the pipeline must shed load.
    let mut permits = Vec::new();
    for _ in 0..32 {
        permits.push(
            core.automation_slots
                .clone()
                .try_acquire_owned()
                .expect("slot available"),
        );
    }
    let mut events = core.events.subscribe_domain();
    core.queue_automation_event(serde_json::json!({
        "type": "tiktok.chat",
        "data": {"comment": "shed me"},
    }));
    let event = tokio::time::timeout(std::time::Duration::from_secs(5), events.recv())
        .await
        .expect("domain event arrives despite saturation")
        .expect("channel open");
    assert_eq!(event.topic(), "live.event");
    drop(permits);
}

#[test]
fn ui_ready_domain_topics_match_frontend_contract() {
    // Wire contract for the migrated live feed: topic names plus camelCase
    // data keys must match `src/web/features/live.ts` exactly.
    let ui = serde_json::to_value(crate::events::DomainEvent::LiveUiEvent {
        event: serde_json::json!({"kind": "chat"}),
    })
    .expect("ui event serializes");
    assert_eq!(ui["topic"], "live.ui-event");
    assert_eq!(ui["data"]["event"]["kind"], "chat");

    let stats = serde_json::to_value(crate::events::DomainEvent::RoomStats {
        viewers: 7,
        total_users: 9,
        top_viewers: Vec::new(),
    })
    .expect("room stats serialize");
    assert_eq!(stats["topic"], "room.stats");
    assert_eq!(stats["data"]["viewers"], 7);
    assert_eq!(stats["data"]["totalUsers"], 9);
    assert_eq!(stats["data"]["topViewers"], serde_json::json!([]));

    let catalog = serde_json::to_value(crate::events::DomainEvent::GiftsCatalog {
        gifts: vec![serde_json::json!({"id": "1"})],
    })
    .expect("catalog serializes");
    assert_eq!(catalog["topic"], "gifts.catalog");
    assert_eq!(catalog["data"]["gifts"][0]["id"], "1");

    let reconnecting = serde_json::to_value(crate::events::DomainEvent::LiveReconnecting {
        attempt: 2,
        delay_ms: 500,
    })
    .expect("reconnecting serializes");
    assert_eq!(reconnecting["topic"], "live.reconnecting");
    assert_eq!(reconnecting["data"]["attempt"], 2);
    assert_eq!(reconnecting["data"]["delayMs"], 500);

    let error = serde_json::to_value(crate::events::DomainEvent::LiveError {
        phase: "live".to_owned(),
        message: "boom".to_owned(),
    })
    .expect("error serializes");
    assert_eq!(error["topic"], "live.error");
    assert_eq!(error["data"]["phase"], "live");
    assert_eq!(error["data"]["message"], "boom");
}

#[test]
fn empty_session_cookie_is_allowed_for_guest_mode() {
    // The cookie field is optional in the UI and the native client
    // bootstraps a guest session when it is empty, so validation must
    // only reject overlong values.
    assert!(crate::control::check_session_cookie_len("").is_ok());
    assert!(crate::control::check_session_cookie_len("   ").is_ok());
    assert!(crate::control::check_session_cookie_len("sessionid=abc").is_ok());
    let overlong = "x".repeat(16_385);
    let error = crate::control::check_session_cookie_len(&overlong)
        .expect_err("overlong cookie must be rejected");
    assert_eq!(error.code(), "invalid_params");
}

#[test]
fn saturated_subscriber_does_not_block_live_domain_events() {
    use crate::events::{DomainEvent, EventBus};

    let bus = EventBus::new(16);
    // A saturated automation consumer that never polls: publishing is
    // synchronous and must never block on it, and every other
    // subscriber must keep receiving live events after the burst.
    let _stalled_automation = bus.subscribe_domain();
    let mut live = bus.subscribe_domain();
    for _ in 0..600 {
        bus.publish_domain(DomainEvent::LiveDisconnected);
    }
    bus.publish_domain(DomainEvent::LiveError {
        phase: "burst-marker".to_owned(),
        message: "post-burst".to_owned(),
    });
    let mut saw_marker = false;
    let mut saw_reliable_gap = false;
    for _ in 0..64 {
        match live.try_recv() {
            Ok(DomainEvent::LiveError { phase, .. }) if phase == "burst-marker" => {
                saw_marker = true;
                break;
            }
            Ok(_) => {}
            Err(crate::events::DomainTryRecvError::ReliableLagged(_)) => {
                saw_reliable_gap = true;
            }
            Err(crate::events::DomainTryRecvError::LossyLagged(_)) => {}
            Err(crate::events::DomainTryRecvError::Empty) => break,
            Err(crate::events::DomainTryRecvError::Closed) => {
                panic!("bus must stay open under saturation")
            }
        }
    }
    assert!(
        saw_reliable_gap,
        "a reliable burst must surface as an explicit gap, never silent loss"
    );
    assert!(
        saw_marker,
        "live subscriber must receive post-burst events despite saturation"
    );
}

#[test]
fn lossy_flood_does_not_evict_reliable_transitions() {
    use crate::events::{DomainEvent, EventBus};

    let bus = EventBus::new(16);
    let mut live = bus.subscribe_domain();
    // A reliable transition published before the flood must survive it.
    bus.publish_domain(DomainEvent::LiveError {
        phase: "before-flood".to_owned(),
        message: "m".to_owned(),
    });
    for index in 0..600 {
        bus.publish_domain(DomainEvent::LiveUiEvent {
            event: serde_json::json!({"n": index}),
        });
    }
    bus.publish_domain(DomainEvent::LiveDisconnected);
    // Reliable drains first, intact and in order; the lossy lane lags
    // independently without touching it.
    let mut markers = Vec::new();
    let mut saw_reliable_gap = false;
    for _ in 0..700 {
        match live.try_recv() {
            Ok(DomainEvent::LiveError { phase, .. }) => markers.push(phase),
            Ok(DomainEvent::LiveDisconnected) => markers.push("disconnect".to_owned()),
            Ok(_) => {}
            Err(crate::events::DomainTryRecvError::ReliableLagged(_)) => {
                saw_reliable_gap = true;
            }
            Err(crate::events::DomainTryRecvError::LossyLagged(_)) => {}
            Err(crate::events::DomainTryRecvError::Empty) => break,
            Err(crate::events::DomainTryRecvError::Closed) => {
                panic!("bus must stay open under a lossy flood")
            }
        }
    }
    assert_eq!(
        markers,
        vec!["before-flood".to_owned(), "disconnect".to_owned()],
        "reliable transitions must survive a lossy flood in order"
    );
    assert!(
        !saw_reliable_gap,
        "a lossy flood must never cause a reliable gap"
    );
}

#[test]
fn event_gaps_degrade_system_health_until_acknowledged() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = AppCore::new(emitter);
    let health = core.system_health();
    assert_eq!(health["status"], "ok");
    assert_eq!(health["events"]["status"], "ok");
    assert_eq!(health["events"]["reliableGaps"], 0);
    assert!(health["events"]["lastGapAt"].is_null());

    core.record_event_gap();
    core.record_event_gap();
    let health = core.system_health();
    assert_eq!(health["status"], "degraded");
    assert_eq!(health["events"]["status"], "degraded");
    assert_eq!(health["events"]["reliableGaps"], 2);
    assert!(
        health["events"]["lastGapAt"].as_u64().unwrap_or_default() > 0,
        "health must timestamp the gap: {health}"
    );
    assert!(
        health.to_string().contains("resync"),
        "health must tell agents to resync: {health}"
    );

    // After every affected client resynced, an explicit acknowledge
    // returns health to OK.
    core.acknowledge_event_gaps();
    let health = core.system_health();
    assert_eq!(health["status"], "ok");
    assert_eq!(health["events"]["status"], "ok");
    assert_eq!(health["events"]["reliableGaps"], 0);
    assert!(health["events"]["lastGapAt"].is_null());
}

#[test]
fn webview_error_degrades_system_health() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = AppCore::new(emitter);
    assert_eq!(core.system_health()["status"], "ok");
    core.set_webview_error(Some("WebView transport failed".to_owned()));
    let health = core.system_health();
    assert_eq!(health["status"], "degraded");
    assert!(
        health.to_string().contains("WebView transport failed"),
        "health must name the WebView failure: {health}"
    );
    core.set_webview_error(None);
    assert_eq!(core.system_health()["status"], "ok");
}

#[test]
fn ipc_error_degrades_system_health() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = AppCore::new(emitter);
    assert_eq!(core.system_health()["status"], "ok");
    core.set_ipc_error(Some("control IPC unavailable".to_owned()));
    let health = core.system_health();
    assert_eq!(health["status"], "degraded");
    assert!(
        health.to_string().contains("control IPC unavailable"),
        "health must name the IPC failure: {health}"
    );
    core.set_ipc_error(None);
    assert_eq!(core.system_health()["status"], "ok");
}
