//! Compatibility adapter around the existing generation-fenced native client.
use crate::{
    events::{CanonicalLiveEvent, TikToolsEvent},
    ClientEvent, NativeTikTokClient,
};
use std::sync::Arc;
use tiktools_live::*;
use tokio_util::sync::CancellationToken;

pub struct TikTokAdapter(pub Arc<NativeTikTokClient>);
impl std::ops::Deref for TikTokAdapter {
    type Target = NativeTikTokClient;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub fn descriptor() -> PlatformDescriptor {
    PlatformDescriptor {
        platform: PlatformId::Tiktok,
        name: "TikTok".into(),
        capabilities: Capabilities {
            chat: true,
            follows: true,
            gifts: true,
            likes: true,
            shares: true,
            joins: true,
            subscriptions: true,
            room_stats: true,
            discovery: true,
            ..Default::default()
        },
        authentication: "session-cookie".into(),
    }
}
impl Adapter for TikTokAdapter {
    fn descriptor(&self) -> PlatformDescriptor {
        descriptor()
    }
    fn run<'a>(
        &'a self,
        request: AdapterRequest,
        sink: EventSink,
        stop: CancellationToken,
    ) -> BoxFuture<'a, Result<(), LiveError>> {
        Box::pin(async move {
            let mut receiver = self.subscribe_stamped_with_loss();
            let (generation, info) = self
                .connect_generation(crate::ConnectRequest {
                    unique_id: request.target,
                    session_cookie: request.credential.expose().into(),
                    room_id: request.room_id,
                })
                .await
                .map_err(|_| LiveError::new("unavailable", "TikTok connection failed"))?;
            if info.verified_creator.as_deref() != Some(info.unique_id.as_str()) {
                self.disconnect_native().await.ok();
                return Err(LiveError::new(
                    "unverified_channel",
                    "TikTok discovery did not verify the creator",
                ));
            }
            sink.connected(ChannelIdentity {
                platform: PlatformId::Tiktok,
                id: info.unique_id.clone(),
                handle: info.unique_id,
                room_id: info.room_id,
            })
            .await?;
            loop {
                let stamped = tokio::select! { biased; () = stop.cancelled() => { self.disconnect_native().await.ok(); return Ok(()); }, event = receiver.recv() => event };
                let stamped = match stamped {
                    Ok(e) => e,
                    Err(crate::TrackedReceiveError::Lagged { count, .. }) => {
                        sink.gap(count).await;
                        continue;
                    }
                    Err(crate::TrackedReceiveError::Closed) => return Ok(()),
                };
                if stamped.generation != generation {
                    if stamped.generation > generation {
                        return Err(LiveError::new(
                            "superseded",
                            "TikTok connection was replaced",
                        ));
                    }
                    continue;
                }
                match stamped.event {
                    ClientEvent::Event(event) => {
                        if let Some(payload) = payload(&event) {
                            let user = event.user().filter(|u| u.id != 0).map(|u| ViewerIdentity {
                                platform: PlatformId::Tiktok,
                                id: u.id.to_string(),
                                handle: u.unique_id.clone(),
                                display_name: u.nickname.clone(),
                                avatar_url: u.avatar_url.clone(),
                            });
                            let mut ext = serde_json::Map::new();
                            ext.insert("tiktok".into(), serde_json::json!({"method":event.method(),"msgId":event.msg_id().to_string()}));
                            sink.event(
                                (event.msg_id() != 0).then(|| event.msg_id().to_string()),
                                now_millis(),
                                event.is_history(),
                                user,
                                payload,
                                ext,
                            )
                            .await?;
                        }
                    }
                    ClientEvent::Reconnecting { attempt, delay_ms } => {
                        sink.reconnecting(attempt.min(u32::MAX as u64) as u32, delay_ms)
                            .await
                    }
                    ClientEvent::Disconnected { .. } => return Ok(()),
                    ClientEvent::Connected(info) => {
                        sink.connected(ChannelIdentity {
                            platform: PlatformId::Tiktok,
                            id: info.unique_id.clone(),
                            handle: info.unique_id,
                            room_id: info.room_id,
                        })
                        .await?;
                    }
                    ClientEvent::Error { .. } => {
                        return Err(LiveError::new(
                            "unavailable",
                            "TikTok live connection failed",
                        ))
                    }
                }
            }
        })
    }
}
pub fn payload(event: &TikToolsEvent) -> Option<LivePayload> {
    Some(match &event.base {
        CanonicalLiveEvent::Chat(e) => LivePayload::Chat {
            comment: e.comment.clone(),
            emotes: e
                .emotes
                .iter()
                .filter_map(|e| serde_json::to_value(e).ok())
                .collect(),
        },
        CanonicalLiveEvent::Like(e) => LivePayload::Like {
            count: e.count,
            total: Some(e.total),
        },
        CanonicalLiveEvent::Member(e) => {
            if e.action == 3 {
                LivePayload::Subscription {
                    tier: None,
                    months: None,
                    gifted: false,
                }
            } else {
                LivePayload::Join
            }
        }
        CanonicalLiveEvent::Social(e) => match crate::events::classify_social_action(e.action) {
            crate::events::SocialKind::Follow => LivePayload::Follow,
            crate::events::SocialKind::Share => LivePayload::Share { count: 1 },
            _ => return None,
        },
        CanonicalLiveEvent::Gift(e) => LivePayload::Gift {
            id: e.gift_id.to_string(),
            name: event.gift_name().unwrap_or("Gift").into(),
            count: e.repeat_count.max(e.combo_count).max(1),
            unit: "tiktok_diamonds".into(),
            amount: event
                .gift_diamond_count()
                .unwrap_or(e.diamond_count)
                .saturating_mul(e.repeat_count.max(e.combo_count).max(1))
                .to_string(),
        },
        CanonicalLiveEvent::RoomUser(e) => LivePayload::RoomStats {
            viewers: e.total,
            total_users: Some(e.total_user),
        },
        _ => return None,
    })
}
