//! Schema-version-specific normalisers.
//!
//! Every function here maps one generated v3 message onto a stable event struct.
//! When a v4 schema lands, it gets its own module of normalisers producing the
//! same output types, and consumers are unaffected.

use prost::Message;
use ttl_live_proto::messages::{
    Contributor, WebcastChatMessage, WebcastEmoteChatMessage, WebcastGiftMessage,
    WebcastLikeMessage, WebcastMemberMessage, WebcastRoomUserSeqMessage, WebcastSocialMessage,
};
use ttl_live_proto::webcast::model::base::emoji::EmoteModel;
use ttl_live_proto::webcast::model::Gift;

use crate::event::{
    ChatEvent, EmoteChatEvent, GiftEvent, LikeEvent, LiveEmote, LiveEvent, MemberEvent,
    RoomUserEvent, SocialEvent, MAX_EMOTES_PER_EVENT, MAX_EMOTE_ID_LENGTH, MAX_EMOTE_URL_LENGTH,
};
use crate::user::EventUser;

/// Protobuf integers are signed; the stable API reports counts as unsigned.
/// Negatives are impossible for these fields in practice, and clamping keeps a
/// corrupt value from wrapping into an absurd number.
fn count(value: impl Into<i64>) -> u64 {
    value.into().max(0) as u64
}

pub(crate) fn chat(payload: &[u8]) -> Result<LiveEvent, prost::DecodeError> {
    let message = WebcastChatMessage::decode(payload)?;
    Ok(LiveEvent::Chat(ChatEvent {
        user: EventUser::normalize(message.user.as_ref()),
        comment: message.content,
        emotes: message
            .emotes
            .iter()
            .filter_map(|inline| normalize_emote(inline.emote.as_ref(), Some(inline.index)))
            .take(MAX_EMOTES_PER_EVENT)
            .collect(),
    }))
}

pub(crate) fn emote_chat(payload: &[u8]) -> Result<LiveEvent, prost::DecodeError> {
    let message = WebcastEmoteChatMessage::decode(payload)?;
    Ok(LiveEvent::EmoteChat(EmoteChatEvent {
        user: EventUser::normalize(message.user.as_ref()),
        // Standalone emotes carry no `EmoteWithIndex`, so there is no index.
        emotes: message
            .emote_list
            .iter()
            .filter_map(|emote| normalize_emote(Some(emote), None))
            .take(MAX_EMOTES_PER_EVENT)
            .collect(),
    }))
}

/// Maps one generated `EmoteModel` (plus the enclosing index, when the message
/// supplies one) onto the stable [`LiveEmote`].
///
/// Infallible: every failure path degrades by keeping the emote with less
/// metadata, never by failing the whole chat message. `None` is returned only
/// when there is nothing usable to keep — no id and no artwork.
fn normalize_emote(emote: Option<&EmoteModel>, index: Option<i32>) -> Option<LiveEmote> {
    let emote = emote?;
    if emote.emote_id.len() > MAX_EMOTE_ID_LENGTH {
        // A mangled id is worse than a missing one: never truncate into a
        // colliding identifier.
        return None;
    }
    let image_url = emote_image_url(emote);
    if emote.emote_id.is_empty() && image_url.is_none() {
        return None;
    }
    Some(LiveEmote {
        id: emote.emote_id.clone(),
        // No v3 field is verified to be the textual shortcode; see `LiveEmote`.
        shortcode: None,
        image_url,
        index,
        emote_type: (emote.emote_type != 0).then_some(emote.emote_type),
        package_id: (!emote.package_id.is_empty()).then(|| emote.package_id.clone()),
    })
}

/// First usable artwork URL from the emote's `ImageModel`.
fn emote_image_url(emote: &EmoteModel) -> Option<String> {
    emote
        .image
        .as_ref()?
        .url_list
        .iter()
        .find_map(|url| sanitize_image_url(url))
}

/// Accepts an absolute `http`/`https` URL and rejects everything else, including
/// unsafe schemes (`javascript:`, `data:`, `file:`), embedded control characters
/// and over-long values. All metadata here is untrusted.
fn sanitize_image_url(url: &str) -> Option<String> {
    if url.len() > MAX_EMOTE_URL_LENGTH {
        return None;
    }
    if url.chars().any(|c| c.is_control() || c == ' ') {
        return None;
    }
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    // Require a host: a bare scheme or `https:///x` is not a usable URL.
    if rest.is_empty() || rest.starts_with('/') {
        return None;
    }
    Some(url.to_owned())
}

pub(crate) fn gift(payload: &[u8]) -> Result<LiveEvent, prost::DecodeError> {
    let message = WebcastGiftMessage::decode(payload)?;
    // The nested `gift` detail block is frequently omitted on repeat messages,
    // so name and diamond count fall back to empty rather than failing.
    let detail = message.gift.as_ref();
    Ok(LiveEvent::Gift(GiftEvent {
        user: EventUser::normalize(message.user.as_ref()),
        gift_id: count(message.gift_id),
        gift_name: detail.map(|gift| gift.name.clone()).unwrap_or_default(),
        diamond_count: detail.map(|gift| count(gift.diamond_count)).unwrap_or(0),
        repeat_count: count(message.repeat_count),
        combo_count: count(message.combo_count),
        group_id: count(message.group_id),
        repeat_end: message.repeat_end != 0,
        gift_image_url: detail.and_then(gift_image_url),
    }))
}

pub(crate) fn like(payload: &[u8]) -> Result<LiveEvent, prost::DecodeError> {
    let message = WebcastLikeMessage::decode(payload)?;
    Ok(LiveEvent::Like(LikeEvent {
        user: EventUser::normalize(message.user.as_ref()),
        count: count(message.count),
        total: count(message.total),
    }))
}

pub(crate) fn member(payload: &[u8]) -> Result<LiveEvent, prost::DecodeError> {
    let message = WebcastMemberMessage::decode(payload)?;
    Ok(LiveEvent::Member(MemberEvent {
        user: EventUser::normalize(message.user.as_ref()),
        member_count: count(message.member_count),
        action: message.action,
    }))
}

pub(crate) fn social(payload: &[u8]) -> Result<LiveEvent, prost::DecodeError> {
    let message = WebcastSocialMessage::decode(payload)?;
    Ok(LiveEvent::Social(SocialEvent {
        user: EventUser::normalize(message.user.as_ref()),
        action: message.action,
        follow_count: count(message.follow_count),
        share_count: count(message.share_count),
        // Retained verbatim as classification evidence; see `crate::social`.
        share_type: message.share_type,
        share_target: message.share_target,
        share_display_style: message.share_display_style,
        follow_type: message.follow_type,
        scene: message.scene,
        target_user_id: count(message.target_user_id),
    }))
}

pub(crate) fn room_user(payload: &[u8]) -> Result<LiveEvent, prost::DecodeError> {
    let message = WebcastRoomUserSeqMessage::decode(payload)?;
    Ok(LiveEvent::RoomUser(RoomUserEvent {
        total: count(message.total),
        popularity: count(message.popularity),
        total_user: count(message.total_user),
        anonymous: count(message.anonymous),
        top_viewers: message.ranks.iter().map(top_viewer).collect(),
        ranked_viewers: message.ranks.iter().map(top_viewer).collect(),
    }))
}

/// Best available gift artwork: the full `image`, then the small `icon`,
/// then the `preview_image`. Each is an `ImageModel` whose first URL wins.
fn gift_image_url(detail: &Gift) -> Option<String> {
    [&detail.image, &detail.icon, &detail.preview_image]
        .into_iter()
        .flatten()
        .find_map(|image| {
            image
                .url_list
                .first()
                .filter(|url| !url.is_empty())
                .cloned()
        })
}

fn top_viewer(contributor: &Contributor) -> crate::event::TopViewer {
    crate::event::TopViewer {
        rank: count(contributor.rank),
        score: count(contributor.score),
        delta: count(contributor.delta),
        user: EventUser::normalize(contributor.user.as_ref()),
    }
}
