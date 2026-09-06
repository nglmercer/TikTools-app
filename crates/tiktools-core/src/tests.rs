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
async fn native_live_event_reaches_the_host_message_boundary() {
    let emitter = Arc::new(RecordingEmitter::default());
    let core = Arc::new(AppCore::new(emitter.clone()));
    core.handle_native_event(ClientEvent::Event(NativeLiveEvent {
        base: tiktools_tiktok::events::CanonicalLiveEvent::Chat(
            tiktools_tiktok::events::ChatEvent {
                user: tiktools_tiktok::events::EventUser {
                    id: 42,
                    unique_id: "alice".to_owned(),
                    nickname: "Alice".to_owned(),
                    sec_uid: String::new(),
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

    let messages = emitter.messages.lock().expect("test emitter poisoned");
    assert!(messages.iter().any(|message| {
        matches!(
            message,
            HostMessage::LiveEvent { event }
                if event.get("kind").and_then(Value::as_str) == Some("chat")
        )
    }));
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
