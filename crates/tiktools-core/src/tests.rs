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

#[cfg(feature = "native-tiktok")]
#[tokio::test]
async fn native_avatar_reaches_ui_points_and_room_tops() {
    use tiktools_tiktok::events::{
        CanonicalLiveEvent, ChatEvent, EventMetadata, EventUser, RoomUserEvent, TopViewer,
    };

    const AVATAR: &str = "https://cdn.example/avatars/alice.png";

    fn user() -> EventUser {
        EventUser {
            id: 42,
            unique_id: "alice".to_owned(),
            nickname: "Alice".to_owned(),
            sec_uid: "sec-alice".to_owned(),
            avatar_url: Some(AVATAR.to_owned()),
        }
    }

    fn wrap(base: CanonicalLiveEvent) -> NativeLiveEvent {
        NativeLiveEvent {
            base,
            metadata: EventMetadata {
                method: "WebcastTest".to_owned(),
                msg_id: 7,
                is_history: false,
            },
            gift: None,
        }
    }

    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter.clone()));
    *recover_rwlock_write(&core.connection_context, "test connection context") =
        Some(LiveContext {
            unique_id: "creator".to_owned(),
            room_id: "room-1".to_owned(),
            connection_id: "connection-1".to_owned(),
        });

    // Unit boundary: the UI shape and the automation user both carry it.
    let chat = wrap(CanonicalLiveEvent::Chat(ChatEvent {
        user: user(),
        comment: "hello".to_owned(),
    }));
    let (ui_event, _, options, _) = core.ui_event_and_points(&chat).expect("chat maps");
    assert_eq!(ui_event["avatarUrl"], AVATAR);
    assert_eq!(options.avatar_url.as_deref(), Some(AVATAR));
    let automation = core.normalize_native_event(&chat).expect("chat normalizes");
    assert_eq!(automation["user"]["avatarUrl"], AVATAR);

    // Integration: one handled event lands on the domain bus and the board.
    let mut domain = core.events.subscribe_domain();
    core.handle_native_event(ClientEvent::Event(chat)).await;
    let mut saw_avatar = false;
    while let Ok(event) = domain.try_recv() {
        if let crate::events::DomainEvent::LiveUiEvent { event } = event {
            if event.get("avatarUrl").and_then(Value::as_str) == Some(AVATAR) {
                saw_avatar = true;
            }
        }
    }
    assert!(saw_avatar, "live.ui-event must carry avatarUrl");
    // The board may hold viewers from other tests; resolve ours by handle.
    let board = core.points.leaderboard(Some(1_000));
    let alice = board
        .iter()
        .find(|viewer| viewer.get("uniqueId").and_then(Value::as_str) == Some("alice"))
        .expect("alice must be on the leaderboard");
    assert_eq!(alice["avatarUrl"], AVATAR);

    // Room tops: native ranks map to TopViewerPayload rows with avatars.
    let room = wrap(CanonicalLiveEvent::RoomUser(RoomUserEvent {
        total: 120,
        popularity: 500,
        total_user: 90,
        anonymous: 30,
        top_viewers: vec![TopViewer {
            rank: 1,
            score: 9000,
            delta: 100,
            user: user(),
        }],
        ranked_viewers: Vec::new(),
    }));
    core.handle_native_event(ClientEvent::Event(room)).await;
    let mut saw_top = false;
    while let Ok(event) = domain.try_recv() {
        if let crate::events::DomainEvent::RoomStats {
            viewers,
            total_users,
            top_viewers,
        } = event
        {
            assert_eq!(viewers, 120);
            assert_eq!(total_users, 90);
            assert_eq!(top_viewers.len(), 1);
            assert_eq!(top_viewers[0]["uniqueId"], "alice");
            assert_eq!(top_viewers[0]["avatarUrl"], AVATAR);
            saw_top = true;
        }
    }
    assert!(saw_top, "room.stats must carry mapped top viewers");
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
        let event =
            core.make_plugin_event("hotkeys", &context, "hotkey.pressed", json!({"key": "k"}));
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
            core.automation_state
                .slots
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

// ------------------------------------------------------------------
// Plugin poll end-to-end: fake process plugin through the real host.
// ------------------------------------------------------------------

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use tiktools_plugin_api::PluginRuntimeKind;
use tiktools_plugin_loader::{
    PluginInstance, PluginLoaderError, PluginManager, PluginRoot, PluginRuntime, PluginSource,
    RuntimeRegistry,
};

const FAKE_HOTKEY_MANIFEST: &str = r#"{
    "schemaVersion": 2,
    "id": "hotkeys",
    "name": "Hotkeys",
    "version": "1.0.0",
    "runtime": "process",
    "entry": "fake-entry",
    "capabilities": ["events.publish", "events.subscribe"],
    "eventSubscriptions": ["live.*", "points.changed", "plugin.*"],
    "actionTypes": [{"id": "hotkey.bind"}],
    "eventTypes": [
        {"type": "hotkey.pressed", "title": {"default": "Hotkey pressed"}},
        {"type": "hotkey.status", "title": {"default": "Hotkey status"}}
    ]
}"#;

#[derive(Default)]
struct FakePluginState {
    poll_batches: Mutex<VecDeque<Vec<Value>>>,
    actions: Mutex<Vec<Value>>,
    event_calls: Mutex<Vec<Value>>,
    event_delay_ms: AtomicU64,
    fail_next_event: AtomicBool,
    loads: AtomicU64,
}

struct FakeRuntime {
    state: Arc<FakePluginState>,
}

impl PluginRuntime for FakeRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::Process
    }

    fn load(
        &self,
        manifest: &tiktools_plugin_api::PluginManifest,
        _directory: &std::path::Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        self.state.loads.fetch_add(1, AtomicOrdering::SeqCst);
        Ok(Box::new(FakeInstance {
            id: manifest.id.clone(),
            state: Arc::clone(&self.state),
        }))
    }
}

struct FakeInstance {
    id: String,
    state: Arc<FakePluginState>,
}

impl PluginInstance for FakeInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        let request: Value = serde_json::from_slice(request)
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        let response = match request.get("type").and_then(Value::as_str) {
            Some("poll") => {
                let batch = self
                    .state
                    .poll_batches
                    .lock()
                    .expect("fake batches poisoned")
                    .pop_front()
                    .unwrap_or_default();
                json!({"events": batch})
            }
            Some("action") => {
                self.state
                    .actions
                    .lock()
                    .expect("fake actions poisoned")
                    .push(request.clone());
                json!({"summary": "fake ok"})
            }
            Some("event") => {
                let delay = self.state.event_delay_ms.load(AtomicOrdering::SeqCst);
                if delay > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                }
                if self
                    .state
                    .fail_next_event
                    .swap(false, AtomicOrdering::SeqCst)
                {
                    return Err(PluginLoaderError::Runtime("fake event crash".to_owned()));
                }
                self.state
                    .event_calls
                    .lock()
                    .expect("fake event calls poisoned")
                    .push(request.clone());
                json!({})
            }
            other => {
                return Err(PluginLoaderError::Runtime(format!(
                    "unexpected fake call {other:?}"
                )));
            }
        };
        serde_json::to_vec(&response).map_err(|error| PluginLoaderError::Runtime(error.to_string()))
    }

    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        Ok(())
    }
}

fn fake_hotkey_manager(state: Arc<FakePluginState>, ids: &[&str]) -> (PluginManager, PathBuf) {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("tiktools-fake-plugins-{suffix}"));
    for id in ids {
        let directory = root.join(id);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("plugin.json"),
            FAKE_HOTKEY_MANIFEST.replacen("\"hotkeys\"", &format!("\"{id}\""), 1),
        )
        .unwrap();
        std::fs::write(directory.join("fake-entry"), b"fake").unwrap();
    }
    let mut registry = RuntimeRegistry::new();
    registry.register(Arc::new(FakeRuntime { state }));
    let manager = PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        registry,
    );
    manager.scan().expect("fake plugin should scan");
    for id in ids {
        assert!(manager.get(id).is_some_and(|plugin| plugin.available));
    }
    (manager, root)
}

fn core_with_fake_hotkeys(
    state: Arc<FakePluginState>,
) -> (Arc<AppCore>, Arc<RecordingEmitter>, PathBuf) {
    core_with_fake_plugins(state, &["hotkeys"])
}

fn core_with_fake_plugins(
    state: Arc<FakePluginState>,
    ids: &[&str],
) -> (Arc<AppCore>, Arc<RecordingEmitter>, PathBuf) {
    let emitter = Arc::new(RecordingEmitter::default());
    let mut core = AppCore::new(emitter.clone());
    let (manager, root) = fake_hotkey_manager(Arc::clone(&state), ids);
    core.plugins = Arc::new(manager);
    (Arc::new(core), emitter, root)
}

fn hotkey_snapshot(action_type: &str, action_config: Value) -> Value {
    json!({
        "actions": [{
            "id": "hotkey-action",
            "name": "Hotkey action",
            "typeId": action_type,
            "enabled": true,
            "config": action_config,
        }],
        "events": [{
            "id": "hotkey-event",
            "name": "Hotkey event",
            "enabled": true,
            "trigger": "hotkey.pressed",
            "filters": [
                {"path": "event.data.key", "operator": "eq", "value": "k"},
                {"path": "event.data.modifiers", "operator": "eq", "value": "ctrl"}
            ],
            "cooldownMs": 0,
            "cooldownScope": "user",
            "actionIds": ["hotkey-action"],
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
    })
}

fn fake_press(key: &str, modifiers: &str) -> Value {
    json!({
        "type": "hotkey.pressed",
        "data": {"key": key, "modifiers": modifiers, "sequence": key, "backend": "rdev"}
    })
}

fn drain_domain(core: &AppCore) -> Vec<crate::events::DomainEvent> {
    let mut events = core.events.subscribe_domain();
    let mut drained = Vec::new();
    while let Ok(event) = events.try_recv() {
        drained.push(event);
    }
    drained
}

#[tokio::test]
async fn fake_hotkey_poll_reaches_domain_automation_and_runs() {
    let state = Arc::new(FakePluginState::default());
    state
        .poll_batches
        .lock()
        .unwrap()
        .push_back(vec![fake_press("k", "ctrl")]);
    let (core, emitter, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.automation
        .replace_snapshot(&hotkey_snapshot("core.log", json!({"message": "key!"})));
    let mut domain = core.events.subscribe_domain();

    core.poll_plugin_events().await;

    // Domain: authoritative plugin event with host-stamped ownership.
    let mut saw_plugin_event = false;
    let mut saw_runs_changed = false;
    let mut saw_run_completed = false;
    while let Ok(event) = domain.try_recv() {
        match event {
            crate::events::DomainEvent::PluginEvent {
                plugin_id,
                event_type,
                event,
            } => {
                assert_eq!(plugin_id, "hotkeys");
                assert_eq!(event_type, "hotkey.pressed");
                assert_eq!(event["source"]["pluginId"], "hotkeys");
                assert_eq!(event["source"]["kind"], "plugin");
                assert_eq!(event["data"]["key"], "k");
                saw_plugin_event = true;
            }
            crate::events::DomainEvent::AutomationRunsChanged { runs } => {
                assert_eq!(runs.len(), 1);
                saw_runs_changed = true;
            }
            crate::events::DomainEvent::AutomationRunCompleted { run } => {
                assert_eq!(run["status"], "ok");
                saw_run_completed = true;
            }
            _ => {}
        }
    }
    assert!(saw_plugin_event, "plugin.event domain event missing");
    assert!(saw_runs_changed, "automation.runs.changed missing");
    assert!(saw_run_completed, "automation.run.completed missing");

    // Automation executed and the legacy push still fires for compat.
    assert_eq!(core.automation.recent_runs().len(), 1);
    assert!(emitter
        .messages
        .lock()
        .unwrap()
        .iter()
        .any(|message| matches!(message, HostMessage::BehaviorRuns { .. })));

    // Diagnostics recorded the chord without persisting history.
    let diagnostics = core.plugin_diagnostics();
    let hotkeys = diagnostics["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["pluginId"] == "hotkeys")
        .unwrap();
    assert_eq!(hotkeys["lastEventType"], "hotkey.pressed");
    assert_eq!(hotkeys["lastEvent"]["key"], "k");
    assert_eq!(hotkeys["lastEvent"]["modifiers"], "ctrl");
    assert_eq!(hotkeys["droppedEvents"], 0);

    // Initial bind projection was sent on first contact.
    let actions = state.actions.lock().unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0]["action"]["typeId"], "hotkey.bind");
    assert!(actions[0]["action"]["config"]["shortcuts"].is_array());
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn more_than_sixteen_events_are_all_processed_in_order() {
    let state = Arc::new(FakePluginState::default());
    let batch: Vec<Value> = (0..40)
        .map(|index| fake_press(&format!("k{index}"), "ctrl"))
        .collect();
    state.poll_batches.lock().unwrap().push_back(batch);
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.automation
        .replace_snapshot(&hotkey_snapshot("core.log", json!({"message": "x"})));
    let mut domain = core.events.subscribe_domain();

    core.poll_plugin_events().await;

    // All 40 cross the old 16-event boundary, in order.
    let mut keys = Vec::new();
    while let Ok(event) = domain.try_recv() {
        if let crate::events::DomainEvent::PluginEvent { event, .. } = event {
            keys.push(event["data"]["key"].as_str().unwrap_or_default().to_owned());
        }
    }
    assert_eq!(keys.len(), 40);
    assert_eq!(keys[0], "k0");
    assert_eq!(keys[39], "k39");
    assert_eq!(core.plugin_drop_total(), 0);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn failed_action_does_not_suppress_plugin_event() {
    let state = Arc::new(FakePluginState::default());
    state
        .poll_batches
        .lock()
        .unwrap()
        .push_back(vec![fake_press("k", "ctrl")]);
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    // Unknown action type: automation fails, control plane must not.
    core.automation
        .replace_snapshot(&hotkey_snapshot("no.such.action", json!({})));
    let mut domain = core.events.subscribe_domain();

    core.poll_plugin_events().await;

    let mut saw_plugin_event = false;
    while let Ok(event) = domain.try_recv() {
        if matches!(event, crate::events::DomainEvent::PluginEvent { .. }) {
            saw_plugin_event = true;
        }
    }
    assert!(saw_plugin_event);
    let runs = core.automation.recent_runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["status"], "error");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn disabled_plugin_publishes_nothing() {
    let state = Arc::new(FakePluginState::default());
    state
        .poll_batches
        .lock()
        .unwrap()
        .push_back(vec![fake_press("k", "ctrl")]);
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.automation
        .replace_snapshot(&hotkey_snapshot("core.log", json!({"message": "x"})));
    core.set_plugin_activation("hotkeys", true, false);
    core.poll_plugin_events().await;

    assert!(drain_domain(&core).is_empty());
    assert!(core.automation.recent_runs().is_empty());
    assert_eq!(state.loads.load(AtomicOrdering::SeqCst), 0);
    assert!(state.actions.lock().unwrap().is_empty());
    assert_eq!(state.poll_batches.lock().unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn undeclared_event_types_are_rejected_and_counted() {
    let state = Arc::new(FakePluginState::default());
    state.poll_batches.lock().unwrap().push_back(vec![
        json!({"type": "other.thing", "data": {}}),
        json!({"type": "hotkey.pressed", "data": "not-an-object"}),
    ]);
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.automation
        .replace_snapshot(&hotkey_snapshot("core.log", json!({"message": "x"})));

    core.poll_plugin_events().await;

    assert!(drain_domain(&core)
        .iter()
        .all(|event| !matches!(event, crate::events::DomainEvent::PluginEvent { .. })));
    assert!(core.automation.recent_runs().is_empty());
    assert_eq!(core.plugin_drop_total(), 2);
    assert_eq!(core.plugin_diagnostics()["plugins"][0]["droppedEvents"], 2);
    assert_eq!(core.system_health()["status"], "degraded");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn restarted_plugin_receives_bindings_again() {
    let state = Arc::new(FakePluginState::default());
    state
        .poll_batches
        .lock()
        .unwrap()
        .push_back(vec![fake_press("k", "ctrl")]);
    state
        .poll_batches
        .lock()
        .unwrap()
        .push_back(vec![fake_press("k", "ctrl")]);
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.automation
        .replace_snapshot(&hotkey_snapshot("core.log", json!({"message": "x"})));

    core.poll_plugin_events().await;
    assert_eq!(state.loads.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(state.actions.lock().unwrap().len(), 1);
    assert_eq!(core.automation.recent_runs().len(), 1);

    // Simulate a crash: the loader retires the instance, the next tick
    // restarts it and re-sends the binding projection before polling.
    core.plugins.stop("hotkeys").expect("stop works");
    core.poll_plugin_events().await;

    assert_eq!(state.loads.load(AtomicOrdering::SeqCst), 2);
    let actions = state.actions.lock().unwrap();
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[1]["action"]["typeId"], "hotkey.bind");
    assert_eq!(core.automation.recent_runs().len(), 2);
    let diagnostics = core.plugin_diagnostics();
    assert_eq!(diagnostics["hotkeySync"]["inSync"], true);
    assert!(diagnostics["hotkeySync"]["appliedConfig"]["shortcuts"].is_array());
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn plugin_poll_starts_without_any_webview() {
    // No plugins, no WebView, no frontend handshake: lifecycle only.
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter));
    assert!(!core
        .plugin_state
        .poll_started
        .load(std::sync::atomic::Ordering::Acquire));
    let handle = tokio::runtime::Handle::current();
    core.spawn_plugin_event_poll(&handle);
    core.spawn_plugin_event_poll(&handle);
    assert!(core
        .plugin_state
        .poll_started
        .load(std::sync::atomic::Ordering::Acquire));
    // A tick with zero candidates is a clean no-op.
    core.poll_plugin_events().await;
    core.shutdown().await;
}

#[tokio::test]
async fn enabled_event_subscriber_starts_with_host_runtime() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.spawn_plugin_event_poll(&tokio::runtime::Handle::current());

    assert!(core.plugins.is_running("hotkeys"));
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn generic_event_observer_delivers_stable_envelopes_to_running_subscribers() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.plugins.start("hotkeys").expect("fake plugin starts");
    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());

    core.events
        .publish_domain(crate::events::DomainEvent::PointsChanged {
            unique_id: "viewer".to_owned(),
            delta: 3.0,
            total_points: 3.0,
            level: 1,
        });

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if !state.event_calls.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("event observer should deliver without blocking the publisher");
    {
        let calls = state.event_calls.lock().unwrap();
        assert_eq!(calls[0]["type"], "event");
        assert_eq!(calls[0]["event"]["topic"], "points.changed");
        assert_eq!(calls[0]["event"]["data"]["uniqueId"], "viewer");
    }

    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn generic_event_observer_ignores_disabled_and_stopped_plugins() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.plugins.start("hotkeys").expect("fake plugin starts");
    core.set_plugin_activation("hotkeys", true, false);
    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());
    core.events
        .publish_domain(crate::events::DomainEvent::PointsChanged {
            unique_id: "disabled".to_owned(),
            delta: 1.0,
            total_points: 1.0,
            level: 1,
        });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(state.event_calls.lock().unwrap().is_empty());

    core.set_plugin_activation("hotkeys", true, true);
    core.plugins.stop("hotkeys").expect("fake plugin stops");
    core.events
        .publish_domain(crate::events::DomainEvent::PointsChanged {
            unique_id: "stopped".to_owned(),
            delta: 1.0,
            total_points: 2.0,
            level: 1,
        });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(state.event_calls.lock().unwrap().is_empty());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

async fn wait_for_event_calls(state: &FakePluginState, count: usize) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if state.event_calls.lock().unwrap().len() >= count {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("timed out waiting for plugin event deliveries");
}

fn points_changed(unique_id: &str) -> crate::events::DomainEvent {
    crate::events::DomainEvent::PointsChanged {
        unique_id: unique_id.to_owned(),
        delta: 1.0,
        total_points: 2.0,
        level: 1,
    }
}

#[tokio::test]
async fn observer_shutdown_with_no_workers_completes() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());
    core.events.publish_domain(points_changed("nobody"));
    tokio::time::timeout(std::time::Duration::from_secs(10), core.shutdown())
        .await
        .expect("shutdown with no workers must not hang");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn observer_shutdown_with_one_worker_completes() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.plugins.start("hotkeys").expect("fake plugin starts");
    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());
    core.events.publish_domain(points_changed("solo"));
    wait_for_event_calls(&state, 1).await;
    tokio::time::timeout(std::time::Duration::from_secs(10), core.shutdown())
        .await
        .expect("shutdown with one worker must not hang");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn observer_shutdown_with_multiple_workers_completes() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_plugins(Arc::clone(&state), &["hotkeys", "echo", "relay"]);
    for id in ["hotkeys", "echo", "relay"] {
        core.plugins.start(id).expect("fake plugin starts");
    }
    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());
    core.events.publish_domain(points_changed("crowd"));
    // One delivery per worker proves three workers exist; cancellation is
    // persistent so no worker can steal the supervisor's wakeup.
    wait_for_event_calls(&state, 3).await;
    tokio::time::timeout(std::time::Duration::from_secs(10), core.shutdown())
        .await
        .expect("shutdown with multiple workers must not hang");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn observer_shutdown_cancelled_before_first_select_completes() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.plugins.start("hotkeys").expect("fake plugin starts");
    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());
    // No yield between spawn and shutdown: cancellation must be observed
    // even if the supervisor never reached its select point.
    tokio::time::timeout(std::time::Duration::from_secs(10), core.shutdown())
        .await
        .expect("shutdown during observer startup must not hang");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn observer_shutdown_during_slow_delivery_completes() {
    let state = Arc::new(FakePluginState::default());
    state
        .event_delay_ms
        .store(30_000, std::sync::atomic::Ordering::SeqCst);
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.plugins.start("hotkeys").expect("fake plugin starts");
    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());
    core.events.publish_domain(points_changed("slow"));
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    // In-flight delivery is bounded by the delivery timeout; shutdown must
    // complete instead of hanging behind the stuck plugin call.
    tokio::time::timeout(std::time::Duration::from_secs(20), core.shutdown())
        .await
        .expect("shutdown during slow delivery must not hang");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn observer_shutdown_with_queued_events_completes() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.plugins.start("hotkeys").expect("fake plugin starts");
    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());
    for index in 0..200 {
        core.events
            .publish_domain(points_changed(&format!("burst-{index}")));
    }
    // Shutdown must preempt the backlog, not drain hundreds of events.
    tokio::time::timeout(std::time::Duration::from_secs(10), core.shutdown())
        .await
        .expect("shutdown with queued events must not hang");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn observer_prunes_workers_for_stopped_plugins_and_respawns() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_hotkeys(Arc::clone(&state));
    core.plugins.start("hotkeys").expect("fake plugin starts");
    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());
    core.events.publish_domain(points_changed("first"));
    wait_for_event_calls(&state, 1).await;

    core.plugins.stop("hotkeys").expect("fake plugin stops");
    core.events.publish_domain(points_changed("stopped"));
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(
        state.event_calls.lock().unwrap().len(),
        1,
        "stopped plugin must receive no further events"
    );

    core.plugins.start("hotkeys").expect("fake plugin restarts");
    core.events.publish_domain(points_changed("again"));
    wait_for_event_calls(&state, 2).await;

    tokio::time::timeout(std::time::Duration::from_secs(10), core.shutdown())
        .await
        .expect("shutdown after prune/respawn must not hang");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn hotkey_filter_contract_matches_key_and_modifiers_separately() {
    // Contract: `event.data.key eq k` + `event.data.modifiers eq ctrl`
    // (never `key eq ctrl+k`). One snapshot per case through the real
    // matcher, covering chords, bare keys, sequences, and special keys.
    let emitter = Arc::new(RecordingEmitter::default());
    let core = AppCore::new(emitter);
    let cases = [
        ("k", "ctrl", "k", "ctrl", true),
        ("k", "ctrl+shift", "k", "ctrl+shift", true),
        ("k", "ctrl", "k", "ctrl+shift", false),
        ("a", "", "a", "", true),
        ("o", "", "o", "", true),
        ("f5", "ctrl", "f5", "ctrl", true),
        ("up", "alt", "up", "alt", true),
        ("5", "ctrl", "5", "ctrl", true),
        // Combined `ctrl+k` in `key` never matches the real contract.
        ("ctrl+k", "", "k", "ctrl", false),
    ];
    for (filter_key, filter_modifiers, event_key, event_modifiers, expected) in cases {
        core.automation.replace_snapshot(&json!({
            "actions": [{"id": "a", "name": "a", "typeId": "core.log", "enabled": true, "config": {}}],
            "events": [{
                "id": "e", "name": "e", "enabled": true, "trigger": "hotkey.pressed",
                "filters": [
                    {"path": "event.data.key", "operator": "eq", "value": filter_key},
                    {"path": "event.data.modifiers", "operator": "eq", "value": filter_modifiers}
                ],
                "cooldownMs": 0, "cooldownScope": "user",
                "actionIds": ["a"], "runMode": "all"
            }],
            "eventTypes": [{
                "type": "hotkey.pressed", "title": {"default": "Hotkey pressed"},
                "source": {"kind": "plugin", "pluginId": "hotkeys"}
            }],
            "plugins": [{
                "descriptor": {"id": "hotkeys"},
                "installed": true, "enabled": true, "available": true
            }]
        }));
        let event = json!({
            "type": "hotkey.pressed",
            "data": {"key": event_key, "modifiers": event_modifiers, "sequence": "g o", "backend": "rdev"}
        });
        assert_eq!(
            core.automation.matching_events(&event).len(),
            usize::from(expected),
            "key={filter_key} modifiers={filter_modifiers} vs event {event_key}/{event_modifiers}"
        );
    }
    // Sequence filters observe the rolling history independently.
    core.automation.replace_snapshot(&json!({
        "actions": [{"id": "a", "name": "a", "typeId": "core.log", "enabled": true, "config": {}}],
        "events": [{
            "id": "e", "name": "e", "enabled": true, "trigger": "hotkey.pressed",
            "filters": [{"path": "event.data.sequence", "operator": "contains", "value": "g o"}],
            "cooldownMs": 0, "cooldownScope": "user",
            "actionIds": ["a"], "runMode": "all"
        }],
        "eventTypes": [{
            "type": "hotkey.pressed", "title": {"default": "Hotkey pressed"},
            "source": {"kind": "plugin", "pluginId": "hotkeys"}
        }],
        "plugins": [{
            "descriptor": {"id": "hotkeys"},
            "installed": true, "enabled": true, "available": true
        }]
    }));
    assert_eq!(
        core.automation
            .matching_events(&json!({
                "type": "hotkey.pressed",
                "data": {"key": "o", "modifiers": "", "sequence": "g o", "backend": "rdev"}
            }))
            .len(),
        1
    );
}

#[tokio::test]
async fn hotkey_status_event_reaches_plugin_status_topic() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter));
    let mut domain = core.events.subscribe_domain();
    let source = json!({});
    let event = core.make_plugin_event(
        "hotkeys",
        &source,
        "hotkey.status",
        json!({"platform": "windows", "backends": []}),
    );
    core.publish_automation_event(event).await;

    let mut saw_event = false;
    let mut saw_status = false;
    while let Ok(event) = domain.try_recv() {
        match event {
            crate::events::DomainEvent::PluginEvent { event_type, .. } => {
                assert_eq!(event_type, "hotkey.status");
                saw_event = true;
            }
            crate::events::DomainEvent::PluginStatus { plugin_id, status } => {
                assert_eq!(plugin_id, "hotkeys");
                assert_eq!(status["platform"], "windows");
                saw_status = true;
            }
            _ => {}
        }
    }
    assert!(saw_event && saw_status);
}

#[test]
fn system_info_advertises_napi_vm_node_api_by_default() {
    // Pins the default feature wiring: the loader dependency opts out of
    // loader defaults, so this forwarding is the only thing compiling the
    // real `.node` selection into every host. If it goes missing, napi-vm
    // plugins declaring `nativeAddons` fail closed at load with "this
    // build lacks napi-vm-node-api support".
    let emitter = Arc::new(RecordingEmitter::default());
    let core = AppCore::new(emitter);
    let info = core.system_info();
    assert_eq!(info["name"], "tiktools");
    assert_eq!(info["features"]["napiVmNodeApi"], true);
}
