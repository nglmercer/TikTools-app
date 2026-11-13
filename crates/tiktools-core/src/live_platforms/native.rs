use crate::*;
use tiktools_live::*;

impl AppCore {
    pub(crate) fn publish_normalized_tiktok(
        &self,
        event: &NativeLiveEvent,
        context: Option<&LiveContext>,
        totals: &Value,
    ) {
        let (Some(context), Some(mut payload)) =
            (context, tiktools_tiktok::adapter::payload(event))
        else {
            return;
        };
        if let LivePayload::Gift { count, amount, .. } = &mut payload {
            if let Some(increment) = totals["increment"].as_u64() {
                *count = increment;
            }
            if let Some(diamonds) = totals["diamonds"].as_u64() {
                *amount = diamonds.to_string();
            }
        }
        let mut extensions = serde_json::Map::new();
        extensions.insert("tiktok".into(),json!({"method":event.method(),"msgId":event.msg_id().to_string(),"aggregation":totals}));
        let user = event.user().filter(|u| u.id != 0).map(|u| ViewerIdentity {
            platform: PlatformId::Tiktok,
            id: u.id.to_string(),
            handle: u.unique_id.clone(),
            display_name: u.nickname.clone(),
            avatar_url: u.avatar_url.clone(),
        });
        self.events
            .publish_domain(crate::events::DomainEvent::NormalizedLive {
                event: LiveEnvelope {
                    schema_version: 1,
                    platform: PlatformId::Tiktok,
                    connection_id: context.connection_id.clone(),
                    channel: ChannelIdentity {
                        platform: PlatformId::Tiktok,
                        id: context.unique_id.clone(),
                        handle: context.unique_id.clone(),
                        room_id: context.room_id.clone(),
                    },
                    session_id: identity_key(PlatformId::Tiktok, &context.room_id),
                    source_event_id: (event.msg_id() != 0).then(|| event.msg_id().to_string()),
                    occurred_at: tiktools_live::now_millis(),
                    received_at: tiktools_live::now_millis(),
                    is_history: event.is_history(),
                    user,
                    payload,
                    extensions,
                },
            });
    }
}
