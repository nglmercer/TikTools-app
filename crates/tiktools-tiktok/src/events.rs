//! TikTools' enrichment boundary around the canonical `ttl-live-events` API.
//!
//! `ttl-live-events` owns the normalized TikTok event contracts. This module
//! adds only values that belong to the application transport: the raw envelope
//! metadata and the gift catalog lookup used to fill repeat-gift messages.

use std::collections::HashMap;

use crate::GiftInfo;

pub use ttl_live_events::{
    ChatEvent, EventUser, GiftEvent, LikeEvent, LiveEvent as CanonicalLiveEvent, MemberEvent,
    RoomUserEvent, SocialEvent,
};

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
        let gift =
            match &decoded.event {
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
            base: decoded.event,
            metadata,
            gift,
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
        self.gift.as_ref().and_then(|gift| gift.icon_url.as_deref())
    }
}
