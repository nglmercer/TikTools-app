use super::*;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

struct Fake(PlatformId);
impl Adapter for Fake {
    fn descriptor(&self) -> PlatformDescriptor {
        PlatformDescriptor {
            platform: self.0,
            name: "Fake".into(),
            capabilities: Capabilities {
                chat: true,
                ..Default::default()
            },
            authentication: "none".into(),
        }
    }
    fn run<'a>(
        &'a self,
        request: AdapterRequest,
        sink: EventSink,
        stop: CancellationToken,
    ) -> BoxFuture<'a, Result<(), LiveError>> {
        Box::pin(async move {
            sink.connected(ChannelIdentity {
                platform: self.0,
                id: request.target.clone(),
                handle: request.target,
                room_id: "same-room".into(),
            })
            .await?;
            sink.event(
                Some("same-id".into()),
                1,
                false,
                Some(ViewerIdentity {
                    platform: self.0,
                    id: "same-user".into(),
                    handle: "alice".into(),
                    display_name: "Alice".into(),
                    avatar_url: None,
                }),
                LivePayload::Chat {
                    comment: "hello".into(),
                    emotes: vec![],
                },
                Default::default(),
            )
            .await?;
            stop.cancelled().await;
            Ok(())
        })
    }
}
fn request(target: &str) -> AdapterRequest {
    AdapterRequest {
        target: target.into(),
        room_id: None,
        credential: Credential::default(),
    }
}

#[tokio::test]
async fn isolates_identical_ids_and_disconnects_only_one_connection() {
    let manager = ConnectionManager::default();
    manager
        .register(Arc::new(Fake(PlatformId::Twitch)))
        .unwrap();
    manager
        .register(Arc::new(Fake(PlatformId::Youtube)))
        .unwrap();
    let mut events = manager.take_events().unwrap();
    let a = manager
        .connect(PlatformId::Twitch, request("alice"))
        .await
        .unwrap();
    let b = manager
        .connect(PlatformId::Youtube, request("alice"))
        .await
        .unwrap();
    let mut received = vec![];
    while received.len() < 2 {
        if let ManagedEvent::Event(e) =
            tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
                .await
                .unwrap()
                .unwrap()
        {
            received.push(e);
        }
    }
    assert_ne!(
        received[0].user.as_ref().unwrap().key(),
        received[1].user.as_ref().unwrap().key()
    );
    assert_ne!(received[0].session_id, received[1].session_id);
    assert_ne!(a.connection_id, b.connection_id);
    manager.disconnect(&a.connection_id).await.unwrap();
    assert_eq!(manager.snapshots().len(), 1);
    assert_eq!(manager.snapshots()[0].platform, PlatformId::Youtube);
    manager.shutdown().await;
    assert!(manager
        .connect(PlatformId::Twitch, request("bob"))
        .await
        .is_err());
}
#[tokio::test]
async fn rejects_duplicate_and_unsupported_providers_and_cancels_backpressure() {
    let manager = ConnectionManager::new(1);
    manager
        .register(Arc::new(Fake(PlatformId::Twitch)))
        .unwrap();
    assert!(manager
        .register(Arc::new(Fake(PlatformId::Twitch)))
        .is_err());
    assert!(manager
        .connect(PlatformId::Kick, request("alice"))
        .await
        .is_err());
    let a = manager
        .connect(PlatformId::Twitch, request("alice"))
        .await
        .unwrap();
    assert!(manager
        .connect(PlatformId::Twitch, request("alice"))
        .await
        .is_err());
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        manager.disconnect(&a.connection_id),
    )
    .await
    .unwrap()
    .unwrap();
}
#[test]
fn compatibility_is_one_way_and_secrets_are_redacted() {
    assert!(trigger_matches(Some("live.chat"), Some("tiktok.chat")));
    assert!(!trigger_matches(Some("tiktok.chat"), Some("live.chat")));
    assert!(!is_builtin_live_trigger("tiktok.forged"));
    assert!(!is_live_trigger("live.forged"));
    assert!(trigger_matches_event(
        Some("live.subscription"),
        &serde_json::json!({"type":"tiktok.join","data":{"action":3}})
    ));
    assert!(!trigger_matches_event(
        Some("live.join"),
        &serde_json::json!({"type":"tiktok.join","data":{"action":3}})
    ));
    let view = event_for_trigger(
        &serde_json::json!({"type":"tiktok.chat","data":{"comment":"hello"}}),
        Some("live.chat"),
    )
    .into_owned();
    assert_eq!(view["type"], "live.chat");
    assert!(!format!("{:?}", Credential::new("top-secret".into())).contains("top-secret"));
}
