//! TikTools' enrichment boundary around the canonical `ttl-live-events` API.
//!
//! `ttl-live-events` owns the normalized TikTok event contracts. This module
//! adds only values that belong to the application transport: the raw envelope
//! metadata and the gift catalog lookup used to fill repeat-gift messages.

use std::collections::HashMap;

use crate::GiftInfo;

pub use ttl_live_events::{
    ChatEvent, EmoteChatEvent, EventUser, GiftEvent, LikeEvent, LiveEmote,
    LiveEvent as CanonicalLiveEvent, MemberEvent, RoomUserEvent, SocialEvent, TopViewer,
};

pub use ttl_live_events::social::SocialKind;

/// The single `action` -> kind table for the application.
///
/// Classification is driven by `action` alone; `follow_count`/`share_count` are
/// cumulative room totals, not per-event increments, so deciding by "the first
/// positive counter" would mislabel every share in a room that has any
/// follower. Delegates to the canonical `ttl-live-events` classifier so the
/// application mirror cannot drift from the crates it consumes.
pub fn classify_social_action(action: i64) -> SocialKind {
    ttl_live_events::social::classify_action(action).kind
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMetadata {
    pub method: String,
    pub msg_id: u64,
    pub is_history: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GiftEnrichment {
    pub name: String,
    pub diamond_count: u64,
    pub streakable: bool,
    pub icon_url: Option<String>,
}

/// One canonical normalized event plus TikTools-only enrichment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TikToolsEvent {
    pub base: ttl_live_events::LiveEvent,
    pub metadata: EventMetadata,
    pub gift: Option<GiftEnrichment>,
}

impl TikToolsEvent {
    pub fn from_decoded(
        decoded: ttl_live_events::DecodedEvent,
        gifts: &HashMap<String, GiftInfo>,
    ) -> Self {
        let metadata = EventMetadata {
            method: decoded.raw.method,
            msg_id: decoded.raw.msg_id,
            is_history: decoded.raw.is_history,
        };
        Self::enrich(decoded.event, metadata, gifts)
    }

    fn from_normalized(
        decoded: ttl_live_events::SelectedEvent,
        gifts: &HashMap<String, GiftInfo>,
    ) -> Self {
        let metadata = EventMetadata {
            method: decoded.method.clone(),
            msg_id: decoded.msg_id,
            is_history: decoded.is_history,
        };
        let base = decoded
            .event
            .unwrap_or_else(|| CanonicalLiveEvent::Unknown {
                method: decoded.method,
                payload: decoded.payload.unwrap_or_default(),
            });
        Self::enrich(base, metadata, gifts)
    }

    fn enrich(
        base: CanonicalLiveEvent,
        metadata: EventMetadata,
        gifts: &HashMap<String, GiftInfo>,
    ) -> Self {
        let gift =
            match &base {
                ttl_live_events::LiveEvent::Gift(event) => gifts
                    .get(&event.gift_id.to_string())
                    .map(|gift| GiftEnrichment {
                        name: gift.name.clone(),
                        diamond_count: gift.diamond_count,
                        streakable: gift.streakable,
                        icon_url: gift.icon_url.clone(),
                    }),
                _ => None,
            };
        Self {
            base,
            metadata,
            gift,
        }
    }

    /// Cache only observed metadata by stable gift ID within this room.
    /// Existing catalog values remain available for omitted streak details.
    pub(crate) fn remember_gift(&self, gifts: &mut HashMap<String, GiftInfo>) {
        let CanonicalLiveEvent::Gift(event) = &self.base else {
            return;
        };
        if event.gift_id == 0 {
            return;
        }
        let key = event.gift_id.to_string();
        if !gifts.contains_key(&key) && gifts.len() >= 4096 {
            return;
        }
        let entry = gifts.entry(key.clone()).or_insert_with(|| GiftInfo {
            id: key,
            name: String::new(),
            diamond_count: 0,
            icon_url: None,
            streakable: false,
        });
        if !event.gift_name.is_empty() {
            entry.name.clone_from(&event.gift_name);
        }
        if event.diamond_count > 0 {
            entry.diamond_count = event.diamond_count;
        }
        if let Some(url) = event
            .gift_image_url
            .as_deref()
            .filter(|url| safe_artwork(url))
        {
            entry.icon_url = Some(url.to_owned());
        }
    }

    pub fn user(&self) -> Option<&EventUser> {
        self.base.user()
    }

    pub fn method(&self) -> &str {
        &self.metadata.method
    }

    pub fn msg_id(&self) -> u64 {
        self.metadata.msg_id
    }

    pub fn is_history(&self) -> bool {
        self.metadata.is_history
    }

    pub fn gift_name(&self) -> Option<&str> {
        let ttl_live_events::LiveEvent::Gift(event) = &self.base else {
            return None;
        };
        if !event.gift_name.is_empty() {
            return Some(event.gift_name.as_str());
        }
        self.gift
            .as_ref()
            .map(|gift| gift.name.as_str())
            .filter(|name| !name.is_empty())
            .or(Some("Gift"))
    }

    pub fn gift_diamond_count(&self) -> Option<u64> {
        let ttl_live_events::LiveEvent::Gift(event) = &self.base else {
            return None;
        };
        Some(if event.diamond_count == 0 {
            self.gift
                .as_ref()
                .map(|gift| gift.diamond_count)
                .unwrap_or_default()
        } else {
            event.diamond_count
        })
    }

    pub fn gift_streakable(&self) -> bool {
        self.gift.as_ref().is_some_and(|gift| gift.streakable)
    }

    pub fn gift_icon_url(&self) -> Option<&str> {
        let event_image = match &self.base {
            CanonicalLiveEvent::Gift(event) => event.gift_image_url.as_deref(),
            _ => None,
        };
        event_image.filter(|url| safe_artwork(url)).or_else(|| {
            self.gift
                .as_ref()
                .and_then(|gift| gift.icon_url.as_deref())
                .filter(|url| safe_artwork(url))
        })
    }
}

fn safe_artwork(value: &str) -> bool {
    value.len() <= 2048
        && reqwest::Url::parse(value).is_ok_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        })
}

#[cfg(test)]
mod tests {
    use super::{classify_social_action, SocialKind};

    #[test]
    fn event_artwork_wins_and_missing_details_reuse_verified_room_metadata() {
        use super::*;
        let mut gifts = HashMap::new();
        gifts.insert(
            "1".into(),
            GiftInfo {
                id: "1".into(),
                name: "Catalog Rose".into(),
                diamond_count: 1,
                icon_url: Some("https://cdn.example/catalog.png".into()),
                streakable: true,
            },
        );
        let gift = TikToolsEvent {
            base: CanonicalLiveEvent::Gift(GiftEvent {
                gift_id: 1,
                gift_name: "Event Rose".into(),
                gift_image_url: Some("https://cdn.example/event.png".into()),
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastGiftMessage".into(),
                msg_id: 1,
                is_history: false,
            },
            gift: Some(GiftEnrichment {
                name: "Catalog Rose".into(),
                diamond_count: 1,
                icon_url: Some("https://cdn.example/catalog.png".into()),
                streakable: true,
            }),
        };
        assert_eq!(gift.gift_icon_url(), Some("https://cdn.example/event.png"));
        gift.remember_gift(&mut gifts);
        assert_eq!(
            gifts["1"].icon_url.as_deref(),
            Some("https://cdn.example/event.png")
        );
        assert!(gifts["1"].streakable);
        let mut unsafe_event = gift.clone();
        if let CanonicalLiveEvent::Gift(gift) = &mut unsafe_event.base {
            gift.gift_image_url = Some("javascript:alert(1)".into());
        }
        assert_eq!(
            unsafe_event.gift_icon_url(),
            Some("https://cdn.example/catalog.png")
        );
        unsafe_event.remember_gift(&mut gifts);
        assert_eq!(
            gifts["1"].icon_url.as_deref(),
            Some("https://cdn.example/event.png")
        );
    }

    /// The same table the `tiktok-signer` fixture pins. Duplicated here only
    /// because the fixture lives in the other repository.
    #[test]
    fn action_table_is_follow_share_or_unclassified() {
        assert_eq!(classify_social_action(1), SocialKind::Follow);
        assert_eq!(classify_social_action(3), SocialKind::Share);
        assert_eq!(classify_social_action(0), SocialKind::Unknown);
        for code in [-1, 2, 4, 5, 8, 97, i64::MAX] {
            assert_eq!(
                classify_social_action(code),
                SocialKind::Other,
                "action {code} must stay unclassified"
            );
        }
    }

    /// The counters are cumulative room totals; they must never influence the
    /// kind. This is the classifier contract that the counters cannot reach.
    #[test]
    fn the_table_has_no_counter_inputs() {
        // The signature is the guarantee: only `action` is consulted.
        let _: fn(i64) -> SocialKind = classify_social_action;
    }
}

/// Decode only canonical events. Debug/schema presentation limits must not
/// veto otherwise valid protobuf normalization.
pub fn decode_batch(
    payload: &[u8],
    gifts: &mut HashMap<String, GiftInfo>,
) -> Result<Vec<TikToolsEvent>, ttl_live_events::EventError> {
    let batch =
        ttl_live_events::decode_batch_with_mode(payload, ttl_live_events::DecodeMode::Normalized)?;
    Ok(batch.events.into_iter().map(|decoded| {
        if decoded.truncated {
            tracing::warn!(method = %decoded.method, msg_id = decoded.msg_id, "LIVE packet exceeded normalization limits");
        }
        let event = TikToolsEvent::from_normalized(decoded, gifts);
        event.remember_gift(gifts);
        event
    }).collect())
}

#[cfg(test)]
mod deep_packet_tests {
    use super::*;
    use prost::Message;
    #[test]
    fn diagnostic_depth_does_not_hide_real_gifts_or_lose_envelope_identity() {
        let payload = include_bytes!("../tests/fixtures/deep-gift.pb");
        // Structurally scrubbed from a real LIVE packet: numbers replaced,
        // strings replaced with 'fixture', opaque/unknown fields removed.
        assert!(ttl_live_proto::messages::WebcastGiftMessage::decode(payload.as_slice()).is_ok());
        let legacy =
            ttl_live_events::decode_event_envelope("WebcastGiftMessage", 123, true, payload);
        assert!(legacy.schema.truncated);
        assert!(matches!(legacy.event, CanonicalLiveEvent::Unknown { .. }));
        let batch = ttl_live_proto::ProtoMessageFetchResult {
            messages: vec![
                ttl_live_proto::BaseProtoMessage {
                    method: "WebcastGiftMessage".into(),
                    msg_id: 123,
                    is_history: true,
                    payload: payload.to_vec(),
                    ..Default::default()
                },
                ttl_live_proto::BaseProtoMessage {
                    method: "WebcastGiftMessage".into(),
                    msg_id: 124,
                    payload: vec![255],
                    ..Default::default()
                },
                ttl_live_proto::BaseProtoMessage {
                    method: "WebcastGiftMessage".into(),
                    msg_id: 125,
                    payload: ttl_live_proto::messages::WebcastGiftMessage {
                        gift_id: 1,
                        repeat_count: 2,
                        ..Default::default()
                    }
                    .encode_to_vec(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
        .encode_to_vec();
        let decoded = decode_batch(&batch, &mut HashMap::new()).unwrap();
        assert_eq!(decoded.len(), 3);
        assert!(matches!(decoded[0].base, CanonicalLiveEvent::Gift(_)));
        assert_eq!(decoded[0].msg_id(), 123);
        assert!(decoded[0].is_history());
        assert!(matches!(
            decoded[1].base,
            CanonicalLiveEvent::Unknown { .. }
        ));
        assert_eq!(decoded[1].msg_id(), 124);
        assert_eq!(decoded[2].gift_name(), Some("fixture"));
        assert_eq!(decoded[2].gift_diamond_count(), Some(1));
    }
}
