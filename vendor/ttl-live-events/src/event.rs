use serde::{Deserialize, Serialize};

use crate::user::EventUser;

/// Upper bound on the emotes carried by a single event. A chat comment can only
/// hold so many; the cap stops a hostile or corrupt payload from turning one
/// event into an unbounded allocation.
pub const MAX_EMOTES_PER_EVENT: usize = 64;

/// Upper bound on an emote identifier. Real ids are short; anything longer is
/// malformed and is dropped rather than truncated into a colliding id.
pub const MAX_EMOTE_ID_LENGTH: usize = 128;

/// Upper bound on an emote artwork URL. Anything longer is discarded (the emote
/// itself is kept) so a single field cannot balloon an event.
pub const MAX_EMOTE_URL_LENGTH: usize = 2048;

/// A graphical emote carried by a LIVE message, normalised from the generated
/// `EmoteModel`/`EmoteWithIndex` types.
///
/// Every field except [`id`](Self::id) is optional because proto3 cannot
/// distinguish an absent scalar from an explicit zero/empty value. A missing
/// field is therefore *omitted* rather than reported as a confirmed default.
///
/// This is the wire-independent contract: it never exposes the generated Prost
/// layout, and the public JSON uses camelCase (`imageUrl`, `emoteType`,
/// `packageId`) so downstream renderers have one shape to read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEmote {
    /// The original emote identifier exactly as it arrived.
    pub id: String,

    /// The textual shortcode (`[laughcry]`), when one is *reliably* known.
    ///
    /// The v3 schema carries no field that is verified to be the shortcode, so
    /// this stays `None` today. A numeric or opaque id is deliberately not
    /// reinterpreted as a display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcode: Option<String>,

    /// Artwork URL taken verbatim from the emote's `ImageModel`, validated as an
    /// absolute `http(s)` URL. Absent when the message carried no usable image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,

    /// The raw `EmoteWithIndex.index`.
    ///
    /// Preserved without conversion: its unit (UTF-8 bytes, code points, UTF-16
    /// units, ...) is **not** confirmed against real traffic, so text is never
    /// sliced with it and the original comment is never rewritten.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,

    /// The raw `EmoteModel.emote_type` enum value.
    ///
    /// `0` (`EMOTE_TYPE_NORMAL`) is indistinguishable from an absent field, so a
    /// zero is omitted rather than asserted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emote_type: Option<i32>,

    /// The original `EmoteModel.package_id` when non-empty (creator-specific or
    /// custom emote packages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
}

/// A live comment.
///
/// `emotes` was added after the initial release; it defaults to an empty list
/// and is omitted from the serialized form when empty, so text-only chat JSON is
/// byte-for-byte unchanged. Rust source compatibility is affected only for code
/// that builds `ChatEvent` with a struct literal, which must now add
/// `..Default::default()` (or the `emotes` field).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatEvent {
    pub user: EventUser,
    pub comment: String,
    /// Emotes embedded in the comment (`WebcastChatMessage.emotes`), in wire
    /// order. Empty for a plain text comment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emotes: Vec<LiveEmote>,
}

/// A standalone emote message (`WebcastEmoteChatMessage`): one or more emotes
/// with no accompanying text comment.
///
/// It is kept distinct from [`ChatEvent`] on purpose — inventing a `comment`
/// from the emotes would corrupt the meaning of a text comment downstream.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmoteChatEvent {
    pub user: EventUser,
    /// The `emote_list` block, in wire order. Standalone emotes carry no index.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emotes: Vec<LiveEmote>,
}

/// A gift send. TikTok streams repeated gifts as a burst of messages sharing a
/// `group_id`; `repeat_end` marks the final one, which carries the true total.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GiftEvent {
    pub user: EventUser,
    pub gift_id: u64,
    pub gift_name: String,
    pub diamond_count: u64,
    pub repeat_count: u64,
    pub combo_count: u64,
    pub group_id: u64,
    pub repeat_end: bool,
    /// Best available gift artwork URL from the detail block (`image`, then
    /// `icon`, then `preview_image`). Absent on repeat messages whose detail
    /// block TikTok omits.
    pub gift_image_url: Option<String>,
}

/// A like burst. `count` is this batch, `total` the room-wide running total.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LikeEvent {
    pub user: EventUser,
    pub count: u64,
    pub total: u64,
}

/// A room membership change (join, follow, subscribe, moderator action, ...).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberEvent {
    pub user: EventUser,
    pub member_count: u64,
    /// Raw `MemberMessageAction`. Left numeric on purpose: TikTok adds actions
    /// without notice, and an unknown value must not become a decode failure.
    pub action: i32,
}

/// A follow or share.
///
/// `action` is the discriminator and the only field that decides between a
/// follow and a share; see [`crate::social`] for the classification. The
/// remaining fields are retained verbatim so nothing observed is thrown away:
/// they are evidence for diagnosis and for future classification rules, but
/// none of them changes the kind today.
///
/// These are proto3 scalars, so an absent field and an explicit `0`/empty value
/// are indistinguishable on the wire. A zero here means "absent or zero", never
/// "confirmed zero".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialEvent {
    pub user: EventUser,
    pub action: i64,
    pub follow_count: u64,
    pub share_count: u64,
    /// `WebcastSocialMessage.share_type` (field 3). Meaning unconfirmed.
    pub share_type: i64,
    /// `WebcastSocialMessage.share_target` (field 5), e.g. a share URL.
    pub share_target: String,
    /// `WebcastSocialMessage.share_display_style` (field 7). Meaning unconfirmed.
    pub share_display_style: i64,
    /// `WebcastSocialMessage.follow_type` (field 13). Meaning unconfirmed.
    pub follow_type: i32,
    /// `WebcastSocialMessage.scene` (field 15). Free-form tag; not interpreted.
    pub scene: String,
    /// `WebcastSocialMessage.target_user_id` (field 14), the follow target when
    /// TikTok sends one. `0` when absent.
    pub target_user_id: u64,
}

/// Periodic viewer-count update.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomUserEvent {
    pub total: u64,
    pub popularity: u64,
    pub total_user: u64,
    pub anonymous: u64,
    pub top_viewers: Vec<TopViewer>,
    pub ranked_viewers: Vec<TopViewer>,
}

/// A contributor in the room's viewer ranking.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopViewer {
    pub rank: u64,
    pub score: u64,
    pub delta: u64,
    pub user: EventUser,
}

/// The stable, listener-facing event API.
///
/// This type never exposes generated Prost structs, so the schema version can
/// change without breaking consumers. Anything we do not model yet arrives as
/// [`LiveEvent::Unknown`] with its bytes intact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LiveEvent {
    Chat(ChatEvent),
    /// A standalone `WebcastEmoteChatMessage`. Serialises as `type: "emote_chat"`.
    EmoteChat(EmoteChatEvent),
    Gift(GiftEvent),
    Like(LikeEvent),
    Member(MemberEvent),
    Social(SocialEvent),
    RoomUser(RoomUserEvent),
    /// An event we have no normaliser for. The payload is preserved verbatim so
    /// callers can decode it themselves and so nothing is ever silently lost.
    Unknown {
        method: String,
        payload: Vec<u8>,
    },
}

impl LiveEvent {
    /// The schema method name this event came from.
    pub fn method(&self) -> &str {
        match self {
            Self::Chat(_) => crate::method::CHAT,
            Self::EmoteChat(_) => crate::method::EMOTE_CHAT,
            Self::Gift(_) => crate::method::GIFT,
            Self::Like(_) => crate::method::LIKE,
            Self::Member(_) => crate::method::MEMBER,
            Self::Social(_) => crate::method::SOCIAL,
            Self::RoomUser(_) => crate::method::ROOM_USER,
            Self::Unknown { method, .. } => method,
        }
    }

    /// The user behind the event, when it has one.
    pub fn user(&self) -> Option<&EventUser> {
        match self {
            Self::Chat(event) => Some(&event.user),
            Self::EmoteChat(event) => Some(&event.user),
            Self::Gift(event) => Some(&event.user),
            Self::Like(event) => Some(&event.user),
            Self::Member(event) => Some(&event.user),
            Self::Social(event) => Some(&event.user),
            Self::RoomUser(_) | Self::Unknown { .. } => None,
        }
    }

    /// Whether this event was preserved rather than normalised.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}

/// The undecoded envelope of a single event, always retained alongside the
/// normalised form so no capture is lossy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawEvent {
    pub method: String,
    pub msg_id: u64,
    pub payload: Vec<u8>,
    pub is_history: bool,
}

/// One event: its raw envelope plus the normalised interpretation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DecodedEvent {
    pub raw: RawEvent,
    pub event: LiveEvent,
    #[serde(skip)]
    pub schema: crate::SchemaMessage,
}

/// A decoded `ProtoMessageFetchResult` batch.
///
/// The three transport-relevant fields (`cursor`, `internal_ext`, `need_ack`)
/// are surfaced because the WebSocket ACK path needs them; the transport itself
/// still reads them via its own decoder during the migration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EventBatch {
    pub events: Vec<DecodedEvent>,
    pub cursor: String,
    pub internal_ext: String,
    pub need_ack: bool,
    pub heartbeat_duration: i64,
    pub push_server: String,
}

impl EventBatch {
    /// Iterates the normalised events, skipping the raw envelopes.
    pub fn events(&self) -> impl Iterator<Item = &LiveEvent> {
        self.events.iter().map(|decoded| &decoded.event)
    }

    /// Method names in this batch that have no normaliser yet. Useful for
    /// deciding which event to implement next from real traffic.
    pub fn unknown_methods(&self) -> Vec<&str> {
        let mut methods: Vec<&str> = self
            .events
            .iter()
            .filter(|decoded| decoded.event.is_unknown())
            .map(|decoded| decoded.raw.method.as_str())
            .collect();
        methods.sort_unstable();
        methods.dedup();
        methods
    }
}

/// The complete envelope; `event` remains the legacy compact convenience model.
pub type EventEnvelope = DecodedEvent;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSupport {
    Normalized = 1,
    Schema = 2,
    Raw = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonMode {
    Normalized,
    Schema,
    Full,
    Compact,
    Raw,
}

impl DecodedEvent {
    pub fn method(&self) -> &str {
        &self.raw.method
    }
    pub fn message_id(&self) -> u64 {
        self.raw.msg_id
    }
    pub fn normalized(&self) -> Option<&LiveEvent> {
        (!self.event.is_unknown()).then_some(&self.event)
    }
    pub fn schema(&self) -> &crate::SchemaMessage {
        &self.schema
    }
    pub fn raw_payload(&self) -> &[u8] {
        &self.raw.payload
    }
    pub fn support(&self) -> EventSupport {
        if self.normalized().is_some() {
            EventSupport::Normalized
        } else if self.schema.is_known() {
            EventSupport::Schema
        } else {
            EventSupport::Raw
        }
    }
    /// FNV-1a over UTF-8 method bytes. Collisions are possible; method is canonical.
    pub fn method_id(&self) -> u32 {
        method_id(self.method())
    }
    pub fn flags(&self) -> u32 {
        fn unknown(fields: &[crate::SchemaField]) -> bool {
            fields.iter().any(|f| {
                f.descriptor.is_none()
                    || matches!(&f.value, crate::SchemaValue::Message(o) if unknown(&o.fields))
            })
        }
        u32::from(self.schema.truncated)
            | (u32::from(!self.schema.is_known()) << 1)
            | (u32::from(unknown(&self.schema.fields)) << 2)
            | (u32::from(self.schema.warning) << 3)
    }
    /// Timestamp from common.create_time, when present, in server-defined units.
    pub fn timestamp(&self) -> Option<u64> {
        self.schema.message("common")?.number("create_time")
    }
    pub fn to_json(&self, mode: JsonMode) -> Option<String> {
        use serde_json::json;
        let normalized = || self.normalized().map(normalized_json);
        let schema = || {
            json!({"method": self.method(), "schema": self.schema.schema.map(|s| s.name),
            "fields": object_json(&self.schema.fields), "wireFields": wire_json(&self.schema.fields),
            "truncated": self.schema.truncated, "warning": self.schema.warning})
        };
        let v = match mode {
            JsonMode::Normalized => normalized()?,
            JsonMode::Schema => schema(),
            JsonMode::Full => {
                json!({"version":1, "method":self.method(), "methodId":self.method_id(),
                "messageId": self.message_id().to_string(), "history":self.raw.is_history,
                "support": match self.support() { EventSupport::Normalized => "normalized", EventSupport::Schema => "schema", EventSupport::Raw => "raw" },
                "schema": {"known":self.schema.is_known(), "name":self.schema.schema.map(|s| s.name)},
                "normalized":normalized(), "data":object_json(&self.schema.fields),
                "wireFields":wire_json(&self.schema.fields), "flags":self.flags(),
                "raw":{"size":self.raw.payload.len()}, "timestamp":self.timestamp().map(|v| v.to_string())})
            }
            JsonMode::Raw => {
                json!({"method": self.method(), "messageId":self.message_id().to_string(),
                "history":self.raw.is_history, "fields":wire_json(&self.schema.fields), "payload":bytes_json(&self.raw.payload)})
            }
            JsonMode::Compact => {
                if self.event.is_unknown() {
                    json!({"type":"unknown", "method":self.method(), "bytes":self.raw.payload.len()})
                } else {
                    normalized()?
                }
            }
        };
        Some(v.to_string())
    }
}
pub fn method_id(method: &str) -> u32 {
    method.bytes().fold(2_166_136_261, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16_777_619)
    })
}
fn normalized_json(event: &LiveEvent) -> serde_json::Value {
    fn safe(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Number(n) => *v = serde_json::Value::String(n.to_string()),
            serde_json::Value::Object(o) => {
                for v in o.values_mut() {
                    safe(v);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    safe(v);
                }
            }
            _ => (),
        }
    }
    let mut v = serde_json::to_value(event).expect("normalized model serializes");
    safe(&mut v);
    if let LiveEvent::Member(e) = event {
        v["action"] = serde_json::json!(e.action);
    }
    v
}
fn bytes_json(bytes: &[u8]) -> serde_json::Value {
    use base64::Engine;
    serde_json::json!({"$bytes":base64::engine::general_purpose::STANDARD.encode(bytes)})
}
fn value_json(v: &crate::SchemaValue) -> serde_json::Value {
    use crate::SchemaValue as V;
    use serde_json::json;
    match v {
        V::Varint(v) | V::Uint64(v) | V::Fixed64(v) => json!(v.to_string()),
        V::Int64(v) | V::Sint64(v) | V::Sfixed64(v) => json!(v.to_string()),
        V::Uint32(v) | V::Fixed32(v) => json!(v),
        V::Int32(v) | V::Sint32(v) | V::Sfixed32(v) => json!(v),
        V::Bool(v) => json!(v),
        V::Text(v) => json!(v),
        V::Double(v) => float_json(*v),
        V::Float(v) => float_json(*v as f64),
        V::Enum { number, .. } => json!(number),
        V::Bytes(b) | V::Truncated(b) => bytes_json(b),
        V::Message(o) => object_json(&o.fields),
    }
}
fn float_json(v: f64) -> serde_json::Value {
    if v.is_finite() {
        serde_json::json!(v)
    } else {
        serde_json::json!(if v.is_nan() {
            "NaN"
        } else if v > 0. {
            "Infinity"
        } else {
            "-Infinity"
        })
    }
}
/// Named protobuf JSON; repeated fields and unexpected duplicate occurrences are arrays.
fn object_json(fields: &[crate::SchemaField]) -> serde_json::Value {
    use ttl_live_proto::FieldCardinality;
    let mut o = serde_json::Map::new();
    for f in fields {
        let key = f
            .descriptor
            .map(|d| d.json_name.to_owned())
            .unwrap_or_else(|| format!("#{}", f.number));
        if f.descriptor.is_some_and(|d| d.is_map) {
            if let crate::SchemaValue::Message(entry) = &f.value {
                let k = entry
                    .fields
                    .iter()
                    .rev()
                    .find(|f| f.number == 1)
                    .map(|f| value_json(&f.value))
                    .unwrap_or_else(|| default_json(entry.schema.and_then(|s| s.field(1))));
                let v = entry
                    .fields
                    .iter()
                    .rev()
                    .find(|f| f.number == 2)
                    .map(|f| value_json(&f.value))
                    .unwrap_or_else(|| default_json(entry.schema.and_then(|s| s.field(2))));
                let map = o.entry(key).or_insert_with(|| serde_json::json!({}));
                if let Some(map) = map.as_object_mut() {
                    map.insert(k.as_str().map_or_else(|| k.to_string(), str::to_owned), v);
                }
                continue;
            }
        }
        let v = value_json(&f.value);
        let repeated = f
            .descriptor
            .is_some_and(|d| d.cardinality == FieldCardinality::Repeated);
        match o.entry(key) {
            serde_json::map::Entry::Vacant(e) => {
                e.insert(if repeated {
                    serde_json::Value::Array(vec![v])
                } else {
                    v
                });
            }
            serde_json::map::Entry::Occupied(mut e) => {
                if let serde_json::Value::Array(a) = e.get_mut() {
                    a.push(v);
                } else {
                    let old = e.get().clone();
                    e.insert(serde_json::Value::Array(vec![old, v]));
                }
            }
        }
    }
    serde_json::Value::Object(o)
}
fn wire_json(fields: &[crate::SchemaField]) -> serde_json::Value {
    serde_json::Value::Array(fields.iter().map(|f| serde_json::json!({
        "number":f.number,"name":f.name,"jsonName":f.descriptor.map(|d| d.json_name),
        "known":f.descriptor.is_some(),"wireType":f.wire_type,
        "protoType": f.descriptor.map(|d| proto_type_name(d.value_kind)),
        "enumType": f.descriptor.and_then(|d| match d.value_kind { ttl_live_proto::FieldValueKind::Enum(n) => Some(n),_ => None }),
        "enumName": match &f.value { crate::SchemaValue::Enum { name,.. } => *name,_ => None },
        "messageType": f.descriptor.and_then(|d| match d.value_kind { ttl_live_proto::FieldValueKind::Message(n) => Some(n),_ => None }),
        "repeated":f.descriptor.is_some_and(|d| d.cardinality == ttl_live_proto::FieldCardinality::Repeated),
        "map":f.descriptor.is_some_and(|d| d.is_map), "oneof":f.descriptor.and_then(|d| d.oneof_name),
        "value":value_json(&f.value), "warning": f.warning,
        "fields": match &f.value { crate::SchemaValue::Message(o) => wire_json(&o.fields), _ => serde_json::Value::Null }
    })).collect())
}

// Preserve legacy serialized envelopes while rebuilding the complete schema
// on deserialization; descriptors themselves are never serialized as pointers.
impl<'de> Deserialize<'de> for DecodedEvent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Compatibility {
            raw: RawEvent,
            event: LiveEvent,
        }
        let old = Compatibility::deserialize(d)?;
        let mut decoded = crate::decode_event_envelope(
            &old.raw.method,
            old.raw.msg_id,
            old.raw.is_history,
            &old.raw.payload,
        );
        if !decoded.schema.truncated {
            decoded.event = old.event;
        }
        Ok(decoded)
    }
}

fn default_json(d: Option<&ttl_live_proto::FieldSchema>) -> serde_json::Value {
    use ttl_live_proto::FieldValueKind as K;
    match d.map(|d| d.value_kind) {
        Some(K::String) => serde_json::json!(""),
        Some(K::Bool) => serde_json::json!(false),
        Some(K::Int64 | K::Uint64 | K::Sint64 | K::Fixed64 | K::Sfixed64) => serde_json::json!("0"),
        Some(K::Bytes) => bytes_json(&[]),
        Some(K::Message(_)) | None => serde_json::Value::Null,
        _ => serde_json::json!(0),
    }
}

fn proto_type_name(k: ttl_live_proto::FieldValueKind) -> &'static str {
    use ttl_live_proto::FieldValueKind as K;
    match k {
        K::Double => "double",
        K::Float => "float",
        K::Int64 => "int64",
        K::Uint64 => "uint64",
        K::Sint64 => "sint64",
        K::Fixed64 => "fixed64",
        K::Sfixed64 => "sfixed64",
        K::Int32 => "int32",
        K::Uint32 => "uint32",
        K::Sint32 => "sint32",
        K::Fixed32 => "fixed32",
        K::Sfixed32 => "sfixed32",
        K::Bool => "bool",
        K::String => "string",
        K::Bytes => "bytes",
        K::Enum(_) => "enum",
        K::Message(_) => "message",
    }
}
