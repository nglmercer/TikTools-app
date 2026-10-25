//! Stable TikTok LIVE event API.
//!
//! This crate sits between the generated v3 schema (`ttl-live-proto`) and the
//! application. It owns two responsibilities and nothing else:
//!
//! 1. split a decompressed batch into individual events;
//! 2. normalise the events we understand into version-independent structs.
//!
//! Transport concerns — WebSocket, gzip, ACK, heartbeat — stay in `ttl-live-ws`,
//! and the generated Prost types are never part of this crate's public API.
//!
//! ```no_run
//! use ttl_live_events::{decode_batch, LiveEvent};
//!
//! # fn main() -> Result<(), ttl_live_events::EventError> {
//! # let payload: &[u8] = &[];
//! for event in decode_batch(payload)?.events() {
//!     match event {
//!         LiveEvent::Chat(chat) => println!("{}: {}", chat.user.label(), chat.comment),
//!         LiveEvent::Unknown { method, .. } => println!("unhandled {method}"),
//!         other => println!("{}", other.method()),
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod dynamic;
mod event;
mod normalize;
pub mod social;
mod user;

pub use dynamic::{decode_webcast_message, SchemaField, SchemaMessage, SchemaObject, SchemaValue};
pub use event::{
    ChatEvent, DecodedEvent, EmoteChatEvent, EventBatch, EventEnvelope, EventSupport, GiftEvent,
    JsonMode, LikeEvent, LiveEmote, LiveEvent, MemberEvent, RawEvent, RoomUserEvent, SocialEvent,
    TopViewer, MAX_EMOTES_PER_EVENT, MAX_EMOTE_ID_LENGTH, MAX_EMOTE_URL_LENGTH,
};
pub use social::{SocialClassification, SocialConfidence, SocialEvidence, SocialKind};
pub use user::EventUser;

use event::RawEvent as Raw;
use ttl_live_proto::BaseProtoMessage;

/// Schema method names for the events this crate normalises.
pub mod method {
    pub const CHAT: &str = "WebcastChatMessage";
    pub const EMOTE_CHAT: &str = "WebcastEmoteChatMessage";
    pub const GIFT: &str = "WebcastGiftMessage";
    pub const LIKE: &str = "WebcastLikeMessage";
    pub const MEMBER: &str = "WebcastMemberMessage";
    pub const SOCIAL: &str = "WebcastSocialMessage";
    pub const ROOM_USER: &str = "WebcastRoomUserSeqMessage";
}

/// Errors from decoding a batch.
///
/// Note that a *single* malformed event never produces an error: it degrades to
/// [`LiveEvent::Unknown`] so the rest of the batch still reaches the consumer.
/// Only a corrupt outer envelope fails.
#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("batch exceeds 16 MiB or 8192-message limit")]
    TooLarge,
    #[error("failed to decode the event batch envelope: {0}")]
    Batch(#[from] prost::DecodeError),
}

/// Decodes one decompressed WebSocket payload into normalised events.
fn check_batch(payload: &[u8]) -> Result<(), EventError> {
    if payload.len() > 16 * 1024 * 1024 {
        return Err(EventError::TooLarge);
    }
    // Preflight with borrowed wire views before Prost allocates repeated messages.
    let mut reader = ttl_sign_core::proto::Reader::new(payload);
    let mut count = 0;
    while let Some(field) = reader.next_field() {
        match field {
            Ok((1, _)) => {
                count += 1;
                if count > 8192 {
                    return Err(EventError::TooLarge);
                }
            }
            Ok(_) => (),
            Err(_) => break, // Prost reports the precise envelope error.
        }
    }
    Ok(())
}

/// Decodes a batch with the legacy full representation and serialization contract.
pub fn decode_batch(payload: &[u8]) -> Result<EventBatch, EventError> {
    check_batch(payload)?;
    let result = ttl_live_proto::decode_event_batch(payload)?;
    let mut schema_budget = 65_536;
    Ok(EventBatch {
        events: result
            .messages
            .iter()
            .map(|message| decode_message(message, &mut schema_budget))
            .collect(),
        cursor: result.cursor,
        internal_ext: result.internal_ext,
        need_ack: result.need_ack,
        heartbeat_duration: result.heartbeat_duration,
        push_server: result.push_server,
    })
}

/// Representations requested by an optimized consumer. Legacy APIs retain Full behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DecodeMode {
    Normalized,
    Schema,
    #[default]
    Full,
    Raw,
}

/// Selective decoding preserves envelope metadata independently of representations.
#[derive(Debug)]
pub struct SelectedEvent {
    /// Server-defined common.create_time units, matching EventEnvelope::timestamp.
    pub timestamp: Option<u64>,
    pub method: String,
    pub msg_id: u64,
    pub is_history: bool,
    pub payload: Option<Vec<u8>>,
    pub event: Option<LiveEvent>,
    pub schema: Option<SchemaMessage>,
    pub truncated: bool,
}

/// Batch metadata has the same meaning as in the legacy EventBatch.
#[derive(Debug)]
pub struct SelectedBatch {
    pub events: Vec<SelectedEvent>,
    pub cursor: String,
    pub internal_ext: String,
    pub need_ack: bool,
    pub heartbeat_duration: i64,
    pub push_server: String,
}

/// Decode only requested representations. Limits and ordered malformed-event isolation
/// are preserved. Unknown normalized events own their bytes exactly once.
pub fn decode_batch_with_mode(
    payload: &[u8],
    mode: DecodeMode,
) -> Result<SelectedBatch, EventError> {
    check_batch(payload)?;
    let result = ttl_live_proto::decode_event_batch(payload)?;
    let mut budget = 65_536;
    let events = result
        .messages
        .into_iter()
        .map(|mut message| {
            let oversized = message.payload.len() > dynamic::MAX_EVENT_BYTES;
            message.payload.truncate(dynamic::MAX_EVENT_BYTES);
            let schema = if matches!(mode, DecodeMode::Full | DecodeMode::Schema) {
                let mut schema = dynamic::decode_webcast_message_budget(
                    &message.method,
                    &message.payload,
                    &mut budget,
                )
                .expect("partial decoder is infallible");
                schema.truncated |= oversized;
                Some(schema)
            } else {
                None
            };
            let (wire_truncated, timestamp) = if schema.is_none() {
                dynamic::preflight_event(&message.method, &message.payload, &mut budget)
            } else {
                (
                    false,
                    schema
                        .as_ref()
                        .and_then(|s| s.message("common")?.number("create_time")),
                )
            };
            let truncated =
                oversized || wire_truncated || schema.as_ref().is_some_and(|s| s.truncated);
            let mut event =
                if matches!(mode, DecodeMode::Full | DecodeMode::Normalized) && !truncated {
                    normalize_payload(&message.method, &message.payload)
                        .ok()
                        .flatten()
                } else {
                    None
                };
            let retain_payload = matches!(mode, DecodeMode::Full | DecodeMode::Raw);
            let payload = if retain_payload {
                Some(message.payload)
            } else {
                if mode == DecodeMode::Normalized && event.is_none() {
                    event = Some(LiveEvent::Unknown {
                        method: message.method.clone(),
                        payload: message.payload,
                    });
                }
                None
            };
            if mode == DecodeMode::Full && event.is_none() {
                event = Some(LiveEvent::Unknown {
                    method: message.method.clone(),
                    payload: payload.as_ref().unwrap().clone(),
                });
            }
            SelectedEvent {
                timestamp,
                method: message.method,
                msg_id: message.msg_id as u64,
                is_history: message.is_history,
                payload,
                event,
                schema,
                truncated,
            }
        })
        .collect();
    Ok(SelectedBatch {
        events,
        cursor: result.cursor,
        internal_ext: result.internal_ext,
        need_ack: result.need_ack,
        heartbeat_duration: result.heartbeat_duration,
        push_server: result.push_server,
    })
}

fn normalize_payload(
    method: &str,
    payload: &[u8],
) -> Result<Option<LiveEvent>, prost::DecodeError> {
    match method {
        method::CHAT => normalize::chat(payload).map(Some),
        method::EMOTE_CHAT => normalize::emote_chat(payload).map(Some),
        method::GIFT => normalize::gift(payload).map(Some),
        method::LIKE => normalize::like(payload).map(Some),
        method::MEMBER => normalize::member(payload).map(Some),
        method::SOCIAL => normalize::social(payload).map(Some),
        method::ROOM_USER => normalize::room_user(payload).map(Some),
        _ => Ok(None),
    }
}

/// Normalises a single event payload, given the method that carried it.
///
/// The entry point for callers that already split a batch themselves, such as a
/// relay that receives one message at a time. Never fails: an unmodelled method
/// or an unreadable payload becomes [`LiveEvent::Unknown`] with its bytes kept.
pub fn decode_event(method: &str, payload: &[u8]) -> LiveEvent {
    decode_event_envelope(method, 0, false, payload).event
}

/// Normalises one `BaseProtoMessage`, keeping its raw envelope alongside.
///
/// Internal: `BaseProtoMessage` is a generated type, and this crate's public API
/// deliberately does not expose those. Use [`decode_batch`] or [`decode_event`].
fn decode_message(message: &BaseProtoMessage, budget: &mut usize) -> DecodedEvent {
    decode_envelope_budget(
        &message.method,
        message.msg_id as u64,
        message.is_history,
        &message.payload,
        budget,
    )
}

/// Transport-independent, complete event decoding. Oversized events retain a
/// 4 MiB prefix and are flagged truncated; typed normalization is skipped.
pub fn decode_event_envelope(
    method: &str,
    msg_id: u64,
    is_history: bool,
    payload: &[u8],
) -> EventEnvelope {
    decode_envelope_budget(method, msg_id, is_history, payload, &mut 4096)
}

fn decode_envelope_budget(
    method: &str,
    msg_id: u64,
    is_history: bool,
    payload: &[u8],
    budget: &mut usize,
) -> EventEnvelope {
    let oversized = payload.len() > dynamic::MAX_EVENT_BYTES;
    let payload = &payload[..payload.len().min(dynamic::MAX_EVENT_BYTES)];
    let raw = Raw {
        method: method.to_owned(),
        msg_id,
        payload: payload.to_vec(),
        is_history,
    };
    let mut schema = dynamic::decode_webcast_message_budget(method, payload, budget)
        .expect("partial decoder is infallible");
    schema.truncated |= oversized;
    let normalized = if schema.truncated {
        Ok(unknown(&raw))
    } else {
        match method {
            method::CHAT => normalize::chat(payload),
            method::EMOTE_CHAT => normalize::emote_chat(payload),
            method::GIFT => normalize::gift(payload),
            method::LIKE => normalize::like(payload),
            method::MEMBER => normalize::member(payload),
            method::SOCIAL => normalize::social(payload),
            method::ROOM_USER => normalize::room_user(payload),
            _ => Ok(unknown(&raw)),
        }
    };
    let event = normalized.unwrap_or_else(|_| unknown(&raw));
    DecodedEvent { raw, event, schema }
}

fn unknown(raw: &Raw) -> LiveEvent {
    LiveEvent::Unknown {
        method: raw.method.clone(),
        payload: raw.payload.clone(),
    }
}

pub use event::method_id as event_method_id;
