//! Real HTTP and audio sink admission; retained roots never dispatch twice.
use super::*;
use crate::automation_runtime::context::ExecutionContext;
use crate::control::EffectState;
use crate::services::{MediaHost, MediaHostFuture};
use std::sync::atomic::{AtomicUsize, Ordering};
use tiktools_plugin_api::{
    AudioPlayOptions, AudioPlaybackResult, MediaFileRef, MediaPickerOptions,
};

pub(super) struct AudioHost(pub(super) Arc<AtomicUsize>);
impl MediaHost for AudioHost {
    fn open_picker(&self, _: MediaPickerOptions) -> MediaHostFuture<Option<std::path::PathBuf>> {
        Box::pin(async { Ok(None) })
    }
    fn play_audio(
        &self,
        _: MediaFileRef,
        _: AudioPlayOptions,
    ) -> MediaHostFuture<AudioPlaybackResult> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(AudioPlaybackResult {
                played: true,
                reason: None,
                active_players: 1,
            })
        })
    }
}
fn activate(f: &Fixture, auth: &Authorization, id: &str, type_id: &str, config: Value) {
    f.draft(
        auth,
        AutomationKind::Action,
        json!({"id":id,"name":id,"enabled":false,"typeId":type_id,"config":config}),
    );
    f.approve(
        AutomationKind::Action,
        id,
        vec![enable(AutomationKind::Action, id)],
    );
}
async fn with_root(f: &Fixture, id: &str, root: &ExecutionContext, event: &Value) -> Value {
    f.core()
        .execute_action_with_context(
            &f.body(AutomationKind::Action, id),
            event,
            None,
            root,
            false,
        )
        .await
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approved_audio_uses_resolved_path_and_never_replays_a_retained_root() {
    let mut f = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    drop(f.core.take());
    f.core = Some(Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(AudioHost(calls.clone())),
        f.paths.clone(),
    )));
    let path = f.paths.data.join("alert.wav");
    std::fs::write(&path, b"host audio fixture").unwrap();
    let canonical = path.canonicalize().unwrap().to_str().unwrap().to_owned();
    let mut allowed = grant();
    allowed.capabilities.insert(Capability::AudioPlay);
    allowed.effects.audio.push(AudioEffectGrant {
        file_refs: [canonical].into(),
        output: "host-default".into(),
        max_volume: 0.5,
        overlaps: ["drop".into()].into(),
    });
    let auth = f.pair(allowed);
    activate(
        &f,
        &auth,
        "audio",
        "audio.play",
        json!({"fileRef":"alert.wav","volume":0.4,"overlap":"drop"}),
    );
    let root = ExecutionContext::owner();
    assert_eq!(
        with_root(&f, "audio", &root, &input("alice")).await["status"],
        "ok"
    );
    let retained = with_root(&f, "audio", &root, &input("alice")).await;
    assert_eq!(retained["status"], "error");
    assert!(retained["summary"]
        .as_str()
        .unwrap()
        .contains("not replayed"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let receipts = f.core().automation_effect_history(None, 10).unwrap();
    assert_eq!(receipts.items.len(), 1);
    assert_eq!(receipts.items[0].state, EffectState::Succeeded);
    assert!(!serde_json::to_string(&receipts)
        .unwrap()
        .contains("alert.wav"));
    f.assert_delegated(AutomationKind::Action, "audio");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reusable_audio_plays_on_host_and_stops_when_file_grant_is_removed() {
    let mut f = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    drop(f.core.take());
    f.core = Some(Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(AudioHost(calls.clone())),
        f.paths.clone(),
    )));
    let path = f.paths.data.join("alert.wav");
    std::fs::write(&path, b"host audio fixture").unwrap();
    let canonical = path.canonicalize().unwrap().to_str().unwrap().to_owned();
    let mut allowed = grant();
    allowed
        .capabilities
        .extend([Capability::AutomationToggle, Capability::AudioPlay]);
    allowed.effects.audio.push(AudioEffectGrant {
        file_refs: [canonical].into(),
        output: "host-default".into(),
        max_volume: 0.5,
        overlaps: ["drop".into()].into(),
    });
    allowed.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Audio].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(allowed.clone());
    let identity = auth.snapshot().unwrap();
    for (kind, record) in [
        (
            AutomationKind::Action,
            json!({"id":"sound","name":"Sound","enabled":true,"typeId":"audio.play","schemaVersion":2,"config":{"fileRef":"alert.wav","volume":0.5,"overlap":"drop"}}),
        ),
        (
            AutomationKind::Event,
            json!({"id":"rule","name":"Chat","enabled":true,"trigger":"tiktok.chat","actionIds":["sound"],"filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all","schemaVersion":1}),
        ),
    ] {
        f.core()
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                f.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let receipts = f.core().automation_effect_history(None, 10).unwrap();
    assert_eq!(receipts.items.len(), 1);
    assert_eq!(receipts.items[0].state, EffectState::Succeeded);
    allowed.effects.audio.clear();
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: identity.grant_revision,
            grant: allowed,
        })
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "revocation must prevent another playback"
    );
    assert_eq!(
        f.core()
            .automation_effect_history(None, 10)
            .unwrap()
            .items
            .len(),
        1
    );
    f.core().shutdown().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approved_audio_still_rejects_out_of_scope_files_and_unavailable_output() {
    let f = Fixture::new();
    let path = f.paths.data.join("alert.wav");
    std::fs::write(&path, b"audio").unwrap();
    let mut allowed = grant();
    allowed.capabilities.insert(Capability::AudioPlay);
    allowed.effects.audio.push(AudioEffectGrant {
        file_refs: [path.canonicalize().unwrap().to_str().unwrap().into()].into(),
        output: "host-default".into(),
        max_volume: 1.0,
        overlaps: ["allow".into()].into(),
    });
    let auth = f.pair(allowed);
    activate(
        &f,
        &auth,
        "audio",
        "core.audio.play",
        json!({"fileRef":"alert.wav"}),
    );
    assert_eq!(
        execute(&f, "audio", &input("alice")).await["status"],
        "error"
    );
    assert!(f
        .core()
        .automation_effect_history(None, 10)
        .unwrap()
        .items
        .is_empty());
}

#[cfg(feature = "http")]
mod http {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::Notify,
    };
    fn http_grant(origin: String) -> Grant {
        let mut allowed = grant();
        allowed.capabilities.insert(Capability::HttpRequest);
        allowed.effects.http.push(HttpEffectGrant {
            origin,
            methods: ["POST".into()].into(),
            paths: ["/hook".into()].into(),
            query_fields: Default::default(),
            header_names: ["content-type".into()].into(),
            render_fields: ["event.data.comment".into()].into(),
            body_fields: ["comment".into()].into(),
            allow_text_body: false,
            max_body_bytes: 1024,
            max_response_bytes: 64,
            allow_private_network: true,
        });
        allowed
    }
    fn config(origin: &str) -> Value {
        json!({"url":format!("{origin}/hook"),"method":"POST","headers":{"content-type":"application/json"},"body":"{\"comment\":\"{{event.data.comment}}\"}","allowPrivateNetwork":true,"maxResponseBytes":64})
    }
    async fn request(listener: &TcpListener) -> tokio::net::TcpStream {
        let (mut socket, _) =
            tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
                .await
                .unwrap()
                .unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut chunk = [0u8; 4096];
            let read =
                tokio::time::timeout(std::time::Duration::from_secs(5), socket.read(&mut chunk))
                    .await
                    .unwrap()
                    .unwrap();
            assert!(read > 0, "request ended before its body");
            bytes.extend_from_slice(&chunk[..read]);
            let text = String::from_utf8_lossy(&bytes);
            if let Some(end) = text.find("\r\n\r\n") {
                let content_length = text[..end]
                    .lines()
                    .find_map(|line| {
                        line.split_once(':')
                            .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + content_length {
                    break;
                }
            }
            assert!(bytes.len() <= 8192);
        }
        let request = String::from_utf8_lossy(&bytes);
        assert!(request.starts_with("POST /hook HTTP/1.1"));
        assert!(request.contains("hello"));
        assert!(!request.contains("private-"));
        socket
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reusable_http_action_sends_once_and_stops_when_destination_revoked() {
        let f = Fixture::new();
        let listener = Arc::new(TcpListener::bind("127.0.0.1:0").await.unwrap());
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let mut allowed = http_grant(origin.clone());
        allowed.capabilities.insert(Capability::AutomationToggle);
        allowed.automation_delegation = Some(AutomationDelegation {
            effects: [DelegatedAutomationEffect::Http].into(),
            allow_dynamic_targets: false,
            allow_owner_edits: false,
            allow_owner_source: false,
            allow_other_author_edits: false,
            allow_other_author_source: false,
        });
        let auth = f.pair(allowed.clone());
        let identity = auth.snapshot().unwrap();
        for (kind, record) in [
            (
                AutomationKind::Action,
                json!({"id":"http","name":"Webhook","enabled":true,"typeId":"core.fetch","schemaVersion":2,"config":config(&origin)}),
            ),
            (
                AutomationKind::Event,
                json!({"id":"rule","name":"Chat","enabled":true,"trigger":"tiktok.chat","actionIds":["http"],"filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all","schemaVersion":1}),
            ),
        ] {
            f.core()
                .automation_command(
                    CommandCaller::Remote(auth.clone()),
                    f.request("0"),
                    AutomationMutation::Create { kind, record },
                )
                .unwrap();
        }
        let server = {
            let listener = listener.clone();
            tokio::spawn(async move {
                let mut socket = request(&listener).await;
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .await
                    .unwrap();
            })
        };
        f.core()
            .run_automation_event(AutomationEvent::owner(input("alice")))
            .await;
        server.await.unwrap();
        let receipts = f.core().automation_effect_history(None, 10).unwrap();
        assert_eq!(receipts.items.len(), 1);
        assert_eq!(receipts.items[0].state, EffectState::Succeeded);
        allowed.effects.http.clear();
        f.core()
            .collaboration()
            .unwrap()
            .set_grant(GrantSet {
                principal_id: identity.principal_id,
                expected_revision: identity.grant_revision,
                grant: allowed,
            })
            .unwrap();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            f.core()
                .run_automation_event(AutomationEvent::owner(input("alice"))),
        )
        .await
        .unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
        assert_eq!(
            f.core()
                .automation_effect_history(None, 10)
                .unwrap()
                .items
                .len(),
            1
        );
        f.core().shutdown().await;
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn approved_http_pins_destination_projects_input_and_retains_changed_semantics() {
        let f = Fixture::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let auth = f.pair(http_grant(origin.clone()));
        activate(&f, &auth, "http", "core.fetch", config(&origin));
        let server = tokio::spawn(async move {
            let mut socket = request(&listener).await;
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 22\r\nConnection: close\r\n\r\nprivate-response-token").await.unwrap();
        });
        let root = ExecutionContext::owner();
        let input = input("alice");
        let first = with_root(&f, "http", &root, &input).await;
        assert_eq!(first["status"], "ok", "{first}");
        assert!(!first.to_string().contains("private-"));
        assert_eq!(
            with_root(&f, "http", &root, &input).await["status"],
            "error"
        );
        let mut changed = input.clone();
        changed["data"]["comment"] = "changed".into();
        let changed = with_root(&f, "http", &root, &changed).await;
        assert_eq!(changed["status"], "error");
        assert!(
            changed["summary"].as_str().unwrap().contains("identity"),
            "{changed}"
        );
        assert_eq!(
            f.core()
                .automation_effect_history(None, 10)
                .unwrap()
                .items
                .len(),
            1
        );
        server.await.unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelled_http_is_unknown_and_the_same_root_cannot_dispatch_again() {
        let f = Fixture::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let auth = f.pair(http_grant(origin.clone()));
        activate(&f, &auth, "http", "core.fetch", config(&origin));
        let accepted = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let server = {
            let accepted = accepted.clone();
            let release = release.clone();
            tokio::spawn(async move {
                let _socket = request(&listener).await;
                accepted.notify_one();
                release.notified().await;
            })
        };
        let root = ExecutionContext::owner();
        let core = f.core().clone();
        let body = f.body(AutomationKind::Action, "http");
        let running_root = root.clone();
        let run = tokio::spawn(async move {
            core.execute_action_with_context(&body, &input("alice"), None, &running_root, false)
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), accepted.notified())
            .await
            .unwrap();
        run.abort();
        assert!(run.await.unwrap_err().is_cancelled());
        let receipts = f.core().automation_effect_history(None, 10).unwrap();
        assert_eq!(receipts.items.len(), 1);
        assert_eq!(receipts.items[0].state, EffectState::Unknown);
        let retained = with_root(&f, "http", &root, &input("alice")).await;
        assert_eq!(retained["status"], "error");
        assert!(retained["summary"]
            .as_str()
            .unwrap()
            .contains("not replayed"));
        assert_eq!(
            f.core()
                .automation_effect_history(None, 10)
                .unwrap()
                .items
                .len(),
            1
        );
        release.notify_one();
        server.await.unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn revoked_subject_after_dispatch_blocks_result_work_without_replaying_sink() {
        let f = Fixture::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let auth = f.pair(http_grant(origin.clone()));
        activate(&f, &auth, "http", "core.fetch", config(&origin));
        let accepted = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let server = {
            let accepted = accepted.clone();
            let release = release.clone();
            tokio::spawn(async move {
                let mut socket = request(&listener).await;
                accepted.notify_one();
                release.notified().await;
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .await
                    .unwrap();
            })
        };
        let core = f.core().clone();
        let body = f.body(AutomationKind::Action, "http");
        let run = tokio::spawn(async move {
            core.execute_action(&body, &input("alice"), None, false)
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), accepted.notified())
            .await
            .unwrap();
        f.core()
            .collaboration()
            .unwrap()
            .revoke_session(&auth.snapshot().unwrap().session_id, "local-owner")
            .unwrap();
        release.notify_one();
        let result = run.await.unwrap();
        assert_eq!(result["status"], "error");
        assert!(!result.to_string().contains("private-"));
        let receipts = f.core().automation_effect_history(None, 10).unwrap();
        assert_eq!(receipts.items.len(), 1);
        assert_eq!(receipts.items[0].state, EffectState::Succeeded);
        server.await.unwrap();
    }
}
