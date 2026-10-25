//! Bounded, descriptor-driven decoding for arbitrary methods.
//!
//! [`crate::decode_batch`] answers "give me a `ChatEvent`". This module answers
//! the other question: "what is *in* this message, whatever it is?" — used by
//! the WebView's page-owned WebSocket relay, which sees every method TikTok
//! sends rather than the six we normalise.
//!
//! Payloads are decoded through the [registry] descriptors instead of into
//! generated structs. TikTok ships methods and fields newer than any pinned
//! schema, and a generated struct would either reject them or recurse
//! unboundedly; the dynamic representation keeps every field that arrives and
//! reports explicit depth and field-count limits instead.
//!
//! [registry]: ttl_live_proto::registry

use ttl_live_proto::{schema_by_name, schema_for_method, FieldKind, FieldSchema, MessageSchema};
use ttl_live_proto::{FieldCardinality, FieldValueKind as K};
use ttl_sign_core::proto::{ProtoError, RawProtoValue};

/// Maximum nested messages decoded from one event.
///
/// A limit keeps untrusted page traffic bounded when a future TikTok field
/// recursively embeds a message type the pinned schema does not describe.
const MAX_SCHEMA_DEPTH: usize = 8;
// Wire validation follows Prost's recursion bound, not the debug view depth.
const MAX_NORMALIZATION_DEPTH: usize = 100;

/// Maximum protobuf fields decoded from one event, including nested messages.
const MAX_SCHEMA_FIELDS: usize = 4_096;

/// A page WebSocket event decoded against its descriptor, when one exists.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaMessage {
    /// TikTok transport method, for example `WebcastChatMessage`.
    pub method: String,
    /// Descriptor matched from the method name. `None` means the method is newer
    /// than the pinned schema, or is not a standard `Webcast*` message.
    pub schema: Option<&'static MessageSchema>,
    /// Top-level fields decoded before any limit, including fields absent from
    /// the descriptor.
    pub fields: Vec<SchemaField>,
    /// `true` when a limit stopped decoding before the event ended.
    pub truncated: bool,
    pub warning: bool,
}

impl SchemaMessage {
    /// Fully qualified schema name, or a stable marker for an unknown method.
    pub fn schema_name(&self) -> &str {
        self.schema.map_or("Unknown", |schema| schema.name)
    }

    /// Was this method found in the pinned schema?
    ///
    /// `false` means TikTok shipped a message type newer than the pin. The event
    /// is still decoded: its fields are in [`SchemaMessage::fields`] with wire
    /// numbers and values, only without names.
    pub fn is_known(&self) -> bool {
        self.schema.is_some()
    }

    /// First top-level field whose descriptor name matches, ignoring ASCII case.
    pub fn field_named(&self, name: &str) -> Option<&SchemaField> {
        find_field(&self.fields, name)
    }

    /// Text value of a named field, if it is text.
    pub fn text(&self, name: &str) -> Option<&str> {
        field_text(&self.fields, name)
    }

    /// Numeric value of a named field, whatever integer width it arrived as.
    pub fn number(&self, name: &str) -> Option<u64> {
        field_number(&self.fields, name)
    }

    /// Boolean value of a named field. Protobuf sends these as varints.
    pub fn boolean(&self, name: &str) -> Option<bool> {
        field_number(&self.fields, name).map(|value| value != 0)
    }

    /// Nested message under a named field, for reaching into `user`, `gift`, ….
    pub fn message(&self, name: &str) -> Option<&SchemaObject> {
        field_message(&self.fields, name)
    }
}

/// A descriptor-described nested protobuf object.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaObject {
    /// Descriptor used to decode this object, if its type is in the schema.
    pub schema: Option<&'static MessageSchema>,
    /// Fields in original wire order.
    pub fields: Vec<SchemaField>,
    /// `true` when a limit stopped decoding before this object ended.
    pub truncated: bool,
    pub warning: bool,
}

impl SchemaObject {
    /// First field whose descriptor name matches, ignoring ASCII case.
    pub fn field_named(&self, name: &str) -> Option<&SchemaField> {
        find_field(&self.fields, name)
    }

    pub fn text(&self, name: &str) -> Option<&str> {
        field_text(&self.fields, name)
    }

    pub fn number(&self, name: &str) -> Option<u64> {
        field_number(&self.fields, name)
    }

    pub fn boolean(&self, name: &str) -> Option<bool> {
        field_number(&self.fields, name).map(|value| value != 0)
    }

    pub fn message(&self, name: &str) -> Option<&SchemaObject> {
        field_message(&self.fields, name)
    }
}

/// A protobuf field with its descriptor name when known.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaField {
    pub number: u32,
    pub name: Option<&'static str>,
    pub value: SchemaValue,
    pub descriptor: Option<&'static FieldSchema>,
    pub wire_type: u8,
    pub warning: bool,
}

/// Lossless-or-bounded value from a descriptor-aware protobuf decoder.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaValue {
    Varint(u64), // unknown wire-level integer
    Int64(i64),
    Uint64(u64),
    Sint64(i64),
    Int32(i32),
    Uint32(u32),
    Sint32(i32),
    Sfixed64(i64),
    Sfixed32(i32),
    Double(f64),
    Float(f32),
    Bool(bool),
    Enum {
        number: i32,
        name: Option<&'static str>,
        enum_type: &'static str,
    },
    Fixed64(u64),
    Fixed32(u32),
    Text(String),
    Bytes(Vec<u8>),
    Message(Box<SchemaObject>),
    /// A limit was reached before the enclosed bytes could be interpreted.
    Truncated(Vec<u8>),
}

// Field lookup is shared: an event and a nested object are the same shape, and a
// caller walking `user.badge.name` should not meet a different API at each level.

fn find_field<'a>(fields: &'a [SchemaField], name: &str) -> Option<&'a SchemaField> {
    fields.iter().find(|field| {
        field
            .name
            .is_some_and(|field_name| field_name.eq_ignore_ascii_case(name))
    })
}

fn field_text<'a>(fields: &'a [SchemaField], name: &str) -> Option<&'a str> {
    match &find_field(fields, name)?.value {
        SchemaValue::Text(text) => Some(text),
        _ => None,
    }
}

fn field_number(fields: &[SchemaField], name: &str) -> Option<u64> {
    match find_field(fields, name)?.value {
        SchemaValue::Varint(value) | SchemaValue::Fixed64(value) => Some(value),
        SchemaValue::Fixed32(value) => Some(u64::from(value)),
        SchemaValue::Uint64(v) => Some(v),
        SchemaValue::Uint32(v) => Some(v as u64),
        SchemaValue::Int64(v) | SchemaValue::Sint64(v) | SchemaValue::Sfixed64(v) => {
            u64::try_from(v).ok()
        }
        SchemaValue::Int32(v) | SchemaValue::Sint32(v) | SchemaValue::Sfixed32(v) => {
            u64::try_from(v).ok()
        }
        SchemaValue::Bool(v) => Some(v as u64),
        SchemaValue::Enum { number, .. } => u64::try_from(number).ok(),
        _ => None,
    }
}

fn field_message<'a>(fields: &'a [SchemaField], name: &str) -> Option<&'a SchemaObject> {
    match &find_field(fields, name)?.value {
        SchemaValue::Message(object) => Some(object),
        _ => None,
    }
}

/// Decode one event payload using the pinned schema's descriptors.
///
/// This never instantiates a generated message struct, so a payload newer than
/// the pin cannot turn into an unbounded decode. Generated types remain
/// available in [`ttl_live_proto::v3`] for known, trusted payloads.
pub fn decode_webcast_message(method: &str, payload: &[u8]) -> Result<SchemaMessage, ProtoError> {
    let mut budget = MAX_SCHEMA_FIELDS;
    decode_webcast_message_budget(method, payload, &mut budget)
}

pub(crate) fn decode_webcast_message_budget(
    method: &str,
    payload: &[u8],
    budget: &mut usize,
) -> Result<SchemaMessage, ProtoError> {
    let schema = schema_for_method(method);
    let oversized = payload.len() > MAX_EVENT_BYTES;
    let payload = &payload[..payload.len().min(MAX_EVENT_BYTES)];
    let allowance = (*budget).min(MAX_SCHEMA_FIELDS);
    let mut fields_remaining = allowance;
    let object = decode_object(payload, schema, 0, &mut fields_remaining)?;
    *budget -= allowance - fields_remaining;
    Ok(SchemaMessage {
        method: method.to_owned(),
        schema,
        fields: object.fields,
        truncated: object.truncated || oversized,
        warning: object.warning,
    })
}

/// Per-event cap, enforced before copying or normalization in the envelope.
pub const MAX_EVENT_BYTES: usize = 4 * 1024 * 1024;

impl Default for SchemaMessage {
    fn default() -> Self {
        decode_webcast_message("", &[]).expect("empty protobuf")
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Cursor<'a> {
    fn varint(&mut self) -> Result<u64, ProtoError> {
        let mut v = 0u64;
        for i in 0..10 {
            let b = *self.bytes.get(self.pos).ok_or(ProtoError::Truncated)?;
            self.pos += 1;
            if i == 9 && b > 1 {
                return Err(ProtoError::VarintOverflow);
            }
            v |= u64::from(b & 127) << (i * 7);
            if b < 128 {
                return Ok(v);
            }
        }
        Err(ProtoError::VarintOverflow)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProtoError> {
        let end = self.pos.checked_add(n).ok_or(ProtoError::Truncated)?;
        let b = self.bytes.get(self.pos..end).ok_or(ProtoError::Truncated)?;
        self.pos = end;
        Ok(b)
    }
    fn value(&mut self, wire: u8) -> Result<RawProtoValue, ProtoError> {
        Ok(match wire {
            0 => RawProtoValue::Varint(self.varint()?),
            1 => RawProtoValue::Fixed64(u64::from_le_bytes(self.take(8)?.try_into().unwrap())),
            5 => RawProtoValue::Fixed32(u32::from_le_bytes(self.take(4)?.try_into().unwrap())),
            2 => {
                let n = usize::try_from(self.varint()?).map_err(|_| ProtoError::Truncated)?;
                RawProtoValue::Bytes(self.take(n)?.to_vec())
            }
            w => return Err(ProtoError::UnsupportedWireType(w)),
        })
    }
}

fn scalar(raw: RawProtoValue, d: Option<&FieldSchema>) -> (SchemaValue, bool) {
    let k = d.map(|d| d.value_kind);
    let value = match (raw, k) {
        (RawProtoValue::Varint(v), Some(K::Int64)) => SchemaValue::Int64(v as i64),
        (RawProtoValue::Varint(v), Some(K::Uint64)) => SchemaValue::Uint64(v),
        (RawProtoValue::Varint(v), Some(K::Sint64)) => {
            SchemaValue::Sint64(((v >> 1) as i64) ^ -((v & 1) as i64))
        }
        (RawProtoValue::Varint(v), Some(K::Int32)) => SchemaValue::Int32(v as i32),
        (RawProtoValue::Varint(v), Some(K::Uint32)) => SchemaValue::Uint32(v as u32),
        (RawProtoValue::Varint(v), Some(K::Sint32)) => {
            let v = v as u32;
            SchemaValue::Sint32(((v >> 1) as i32) ^ -((v & 1) as i32))
        }
        (RawProtoValue::Varint(v), Some(K::Bool)) => SchemaValue::Bool(v != 0),
        (RawProtoValue::Varint(v), Some(K::Enum(enum_type))) => SchemaValue::Enum {
            number: v as i32,
            enum_type,
            name: ttl_live_proto::registry::enum_by_name(enum_type)
                .and_then(|e| e.values.iter().find(|e| e.number == v as i32))
                .map(|e| e.name),
        },
        (RawProtoValue::Fixed32(v), Some(K::Float)) => SchemaValue::Float(f32::from_bits(v)),
        (RawProtoValue::Fixed32(v), Some(K::Sfixed32)) => SchemaValue::Sfixed32(v as i32),
        (RawProtoValue::Fixed64(v), Some(K::Double)) => SchemaValue::Double(f64::from_bits(v)),
        (RawProtoValue::Fixed64(v), Some(K::Sfixed64)) => SchemaValue::Sfixed64(v as i64),
        (RawProtoValue::Varint(v), _) => return (SchemaValue::Varint(v), d.is_some()),
        (RawProtoValue::Fixed32(v), _) => {
            return (SchemaValue::Fixed32(v), k.is_some_and(|k| k != K::Fixed32))
        }
        (RawProtoValue::Fixed64(v), _) => {
            return (SchemaValue::Fixed64(v), k.is_some_and(|k| k != K::Fixed64))
        }
        (RawProtoValue::Bytes(v), _) => return (SchemaValue::Bytes(v), d.is_some()),
    };
    (value, false)
}
fn field(
    number: u32,
    wire_type: u8,
    descriptor: Option<&'static FieldSchema>,
    value: SchemaValue,
    warning: bool,
) -> SchemaField {
    SchemaField {
        number,
        name: descriptor.map(|d| d.name),
        descriptor,
        value,
        wire_type,
        warning,
    }
}
fn decode_object(
    payload: &[u8],
    schema: Option<&'static MessageSchema>,
    depth: usize,
    remaining: &mut usize,
) -> Result<SchemaObject, ProtoError> {
    let mut c = Cursor {
        bytes: payload,
        pos: 0,
    };
    let mut fields = Vec::new();
    let mut warning = false;
    let mut truncated = false;
    while c.pos < payload.len() && *remaining > 0 {
        let start = c.pos;
        let tag = match c.varint() {
            Ok(t) if t >> 3 > 0 && t >> 3 <= 0x1fffffff => t,
            _ => {
                warning = true;
                truncated = true;
                break;
            }
        };
        let number = (tag >> 3) as u32;
        let wire = (tag & 7) as u8;
        let d = schema.and_then(|s| s.field(number));
        let raw = match c.value(wire) {
            Ok(v) => v,
            Err(_) => {
                fields.push(field(
                    number,
                    wire,
                    d,
                    SchemaValue::Truncated(payload[start..].to_vec()),
                    true,
                ));
                warning = true;
                truncated = true;
                break;
            }
        };
        if let RawProtoValue::Bytes(ref b) = raw {
            if let Some(d) = d.filter(|d| {
                d.cardinality == FieldCardinality::Repeated
                    && matches!(
                        d.kind,
                        FieldKind::Varint | FieldKind::Fixed32 | FieldKind::Fixed64
                    )
            }) {
                let packed_wire = match d.kind {
                    FieldKind::Varint => 0,
                    FieldKind::Fixed32 => 5,
                    _ => 1,
                };
                let mut p = Cursor { bytes: b, pos: 0 };
                while p.pos < b.len() && *remaining > 0 {
                    let at = p.pos;
                    *remaining -= 1;
                    match p.value(packed_wire) {
                        Ok(v) => {
                            let (v, w) = scalar(v, Some(d));
                            fields.push(field(number, wire, Some(d), v, w));
                            warning |= w;
                        }
                        Err(_) => {
                            fields.push(field(
                                number,
                                wire,
                                Some(d),
                                SchemaValue::Truncated(b[at..].to_vec()),
                                true,
                            ));
                            warning = true;
                            break;
                        }
                    }
                }
                truncated |= p.pos < b.len();
                continue;
            }
        }
        *remaining -= 1;
        let (v, w) = match raw {
            RawProtoValue::Bytes(b) => match d.map(|d| d.kind) {
                Some(FieldKind::String) => match String::from_utf8(b) {
                    Ok(s) => (SchemaValue::Text(s), false),
                    Err(e) => (SchemaValue::Bytes(e.into_bytes()), true),
                },
                Some(FieldKind::Message(n)) if depth < MAX_SCHEMA_DEPTH && *remaining > 0 => {
                    let nested = decode_object(&b, schema_by_name(n), depth + 1, remaining)?;
                    if nested.warning {
                        (SchemaValue::Bytes(b), true)
                    } else {
                        truncated |= nested.truncated;
                        (SchemaValue::Message(Box::new(nested)), false)
                    }
                }
                Some(FieldKind::Message(_)) => {
                    truncated = true;
                    (SchemaValue::Truncated(b), false)
                }
                Some(FieldKind::Bytes) | None => (SchemaValue::Bytes(b), false),
                _ => (SchemaValue::Bytes(b), true),
            },
            v => scalar(v, d),
        };
        warning |= w;
        fields.push(field(number, wire, d, v, w));
    }
    truncated |= c.pos < payload.len();
    Ok(SchemaObject {
        schema,
        fields,
        truncated,
        warning,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use ttl_sign_core::proto::Writer;

    const CHAT_CONTENT_FIELD: u32 = 3;
    const UNKNOWN_FIELD: u32 = 7;
    const UNKNOWN_VALUE: u64 = 42;

    #[test]
    fn decodes_a_schema_mapped_chat_message() {
        let payload = Writer::new()
            .str_field(CHAT_CONTENT_FIELD, "hello from the schema registry")
            .clone()
            .finish();

        let event = decode_webcast_message("WebcastChatMessage", &payload).unwrap();

        assert_eq!(
            event.schema_name(),
            "webcast.model.message.WebcastChatMessage"
        );
        assert_eq!(
            event.text("content"),
            Some("hello from the schema registry")
        );
    }

    /// The real capture decodes with field names, including the methods that had
    /// no descriptor under the retired snapshot.
    #[test]
    fn decodes_captured_events_with_names() {
        let payload = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/chat.pb"),
        )
        .expect("chat fixture");

        let event = decode_webcast_message("WebcastChatMessage", &payload).unwrap();

        assert!(event.is_known());
        assert!(!event.truncated);
        assert_eq!(event.text("content"), Some("ese edificio de que es?"));

        let user = event.message("user").expect("chat carries a user");
        assert_eq!(user.text("nickname"), Some("Rolando"));
        assert_eq!(user.text("display_id"), Some("rolandodmf"));
        assert_eq!(user.number("id"), Some(6_811_680_656_739_271_686));
    }

    #[test]
    fn preserves_unknown_methods_as_raw_fields() {
        let payload = Writer::new()
            .u64_field(UNKNOWN_FIELD, UNKNOWN_VALUE)
            .clone()
            .finish();

        let event = decode_webcast_message("WebcastFutureMessage", &payload).unwrap();

        assert!(!event.is_known(), "not in the pinned schema");
        assert_eq!(event.schema_name(), "Unknown");
        assert_eq!(event.text("anything"), None, "no names to match against");
        // The payload is not lost, only unnamed.
        assert_eq!(
            event.fields.as_slice(),
            [SchemaField {
                number: UNKNOWN_FIELD,
                name: None,
                descriptor: None,
                wire_type: 0,
                warning: false,
                value: SchemaValue::Varint(UNKNOWN_VALUE),
            }]
        );
    }

    /// Named accessors reach any method in the schema, without a hand-written
    /// struct per message type.
    #[test]
    fn named_accessors_reach_values_and_nested_messages() {
        const USER_FIELD: u32 = 2;
        const CONTENT_FIELD: u32 = 3;
        const USER_ID_FIELD: u32 = 1;
        const NICKNAME_FIELD: u32 = 3;

        let user = Writer::new()
            .u64_field(USER_ID_FIELD, 4242)
            .str_field(NICKNAME_FIELD, "Ada")
            .clone()
            .finish();
        let payload = Writer::new()
            .bytes_field(USER_FIELD, &user)
            .str_field(CONTENT_FIELD, "hello")
            .clone()
            .finish();

        let event = decode_webcast_message("WebcastChatMessage", &payload).unwrap();

        assert_eq!(event.text("content"), Some("hello"));
        // Names are matched case-insensitively.
        assert_eq!(event.text("CONTENT"), Some("hello"));

        let sender = event.message("user").expect("chat carries a user");
        assert_eq!(sender.text("nickname"), Some("Ada"));
        assert_eq!(sender.number("id"), Some(4242));

        // Asking for the wrong shape is `None`, never a panic or a coerced value.
        assert_eq!(event.number("content"), None);
        assert_eq!(event.text("user"), None);
        assert_eq!(event.message("content"), None);
        assert_eq!(event.text("no_such_field"), None);
    }

    #[test]
    fn limits_field_count_without_parsing_the_remaining_packet() {
        let mut writer = Writer::new();
        for _ in 0..=MAX_SCHEMA_FIELDS {
            writer.u64_field(UNKNOWN_FIELD, UNKNOWN_VALUE);
        }
        let payload = writer.finish();

        let event = decode_webcast_message("WebcastFutureMessage", &payload).unwrap();

        assert_eq!(event.fields.len(), MAX_SCHEMA_FIELDS);
        assert!(event.truncated);
    }
}

#[cfg(test)]
mod semantic_tests {
    use super::*;
    fn d(number: u32, kind: FieldKind, value_kind: K) -> FieldSchema {
        FieldSchema {
            number,
            name: "value",
            json_name: "value",
            kind,
            value_kind,
            cardinality: FieldCardinality::Repeated,
            is_map: false,
            oneof_name: None,
            oneof_index: None,
            deprecated: false,
        }
    }
    fn vi(v: u64) -> Vec<u8> {
        let mut v = v;
        let mut b = Vec::new();
        loop {
            b.push((v as u8 & 127) | if v > 127 { 128 } else { 0 });
            v >>= 7;
            if v == 0 {
                return b;
            }
        }
    }
    #[test]
    fn all_logical_primitives_decode_packed_and_unpacked_with_order() {
        // Synthetic descriptor exercises types the current pin may not use.
        let cases = [
            (
                K::Double,
                FieldKind::Fixed64,
                (-1.25f64).to_bits().to_le_bytes().to_vec(),
                SchemaValue::Double(-1.25),
            ),
            (
                K::Float,
                FieldKind::Fixed32,
                (-1.25f32).to_bits().to_le_bytes().to_vec(),
                SchemaValue::Float(-1.25),
            ),
            (
                K::Int64,
                FieldKind::Varint,
                vi((-42i64) as u64),
                SchemaValue::Int64(-42),
            ),
            (
                K::Uint64,
                FieldKind::Varint,
                vi(u64::MAX),
                SchemaValue::Uint64(u64::MAX),
            ),
            (
                K::Int32,
                FieldKind::Varint,
                vi((-42i64) as u64),
                SchemaValue::Int32(-42),
            ),
            (
                K::Uint32,
                FieldKind::Varint,
                vi(u32::MAX as u64),
                SchemaValue::Uint32(u32::MAX),
            ),
            (
                K::Sint32,
                FieldKind::Varint,
                vi(83),
                SchemaValue::Sint32(-42),
            ),
            (
                K::Sint64,
                FieldKind::Varint,
                vi(83),
                SchemaValue::Sint64(-42),
            ),
            (
                K::Sfixed32,
                FieldKind::Fixed32,
                (-42i32).to_le_bytes().to_vec(),
                SchemaValue::Sfixed32(-42),
            ),
            (
                K::Sfixed64,
                FieldKind::Fixed64,
                (-42i64).to_le_bytes().to_vec(),
                SchemaValue::Sfixed64(-42),
            ),
            (
                K::Fixed32,
                FieldKind::Fixed32,
                u32::MAX.to_le_bytes().to_vec(),
                SchemaValue::Fixed32(u32::MAX),
            ),
            (
                K::Fixed64,
                FieldKind::Fixed64,
                u64::MAX.to_le_bytes().to_vec(),
                SchemaValue::Fixed64(u64::MAX),
            ),
            (K::Bool, FieldKind::Varint, vi(1), SchemaValue::Bool(true)),
            (
                K::Enum("webcast.im.MemberMessageAction"),
                FieldKind::Varint,
                vi(99999),
                SchemaValue::Enum {
                    number: 99999,
                    name: None,
                    enum_type: "webcast.im.MemberMessageAction",
                },
            ),
        ];
        for (k, wire, b, expected) in cases {
            let fields = Box::leak(vec![d(1, wire, k)].into_boxed_slice());
            let schema = Box::leak(Box::new(MessageSchema {
                name: "synthetic",
                fields,
                is_map_entry: false,
            }));
            let tag = match wire {
                FieldKind::Varint => 8,
                FieldKind::Fixed32 => 13,
                _ => 9,
            };
            let mut payload = vec![tag];
            payload.extend(&b);
            payload.push(10);
            payload.extend(vi((b.len() * 2) as u64));
            payload.extend(&b);
            payload.extend(&b);
            payload.push(tag);
            payload.extend(&b);
            let o = decode_object(&payload, Some(schema), 0, &mut 100).unwrap();
            assert!(!o.warning && !o.truncated, "{k:?}");
            assert_eq!(o.fields.len(), 4, "{k:?}");
            assert!(o.fields.iter().all(|f| f.value == expected), "{k:?}");
            assert_eq!(o.fields[1].wire_type, 2);
        }
    }
    #[test]
    fn malformed_wire_retains_prefix_and_reports_diagnostics() {
        for bad in [
            vec![0],
            vec![8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 2],
            vec![10, 255, 255],
            vec![15],
        ] {
            let mut p = vec![8, 42];
            p.extend(bad);
            let e = decode_webcast_message("WebcastFutureMessage", &p).unwrap();
            assert_eq!(e.fields[0].value, SchemaValue::Varint(42));
            assert!(e.warning && e.truncated);
        }
    }
    #[test]
    fn packed_expansion_obeys_global_field_budget() {
        let fields = Box::leak(vec![d(1, FieldKind::Varint, K::Uint64)].into_boxed_slice());
        let schema = Box::leak(Box::new(MessageSchema {
            name: "synthetic",
            fields,
            is_map_entry: false,
        }));
        let o = decode_object(&[10, 5, 1, 2, 3, 4, 5], Some(schema), 0, &mut 2).unwrap();
        assert_eq!(o.fields.len(), 2);
        assert!(o.truncated);
    }
    #[test]
    fn mismatched_wire_type_is_preserved_with_warning() {
        let e = decode_webcast_message("WebcastChatMessage", &[24, 42]).unwrap();
        assert_eq!(e.fields[0].value, SchemaValue::Varint(42));
        assert!(e.warning);
    }
}

/// Enforce descriptor field/depth budgets with borrowed wire values, without a schema tree.
pub(crate) fn preflight_event(
    method: &str,
    payload: &[u8],
    budget: &mut usize,
) -> (bool, Option<u64>) {
    fn scan(
        payload: &[u8],
        schema: Option<&'static MessageSchema>,
        depth: usize,
        left: &mut usize,
        common: bool,
        timestamp: &mut Option<u64>,
    ) -> bool {
        use ttl_sign_core::proto::{Reader, WireValue};
        let mut reader = Reader::new(payload);
        let mut seen_common = false;
        let mut seen_timestamp = false;
        while let Some(field) = reader.next_field() {
            if *left == 0 {
                return true;
            }
            let Ok((number, value)) = field else {
                return true;
            };
            if number == 0 || number > 0x1fffffff {
                return true;
            }
            let descriptor = schema.and_then(|s| s.field(number));
            if common && !seen_timestamp && descriptor.is_some_and(|d| d.name == "create_time") {
                seen_timestamp = true;
                *timestamp = match value {
                    WireValue::Varint(value) | WireValue::Fixed64(value) => Some(value),
                    WireValue::Fixed32(value) => Some(u64::from(value)),
                    WireValue::Bytes(_) => None,
                };
            }
            if let WireValue::Bytes(bytes) = value {
                if let Some(d) = descriptor.filter(|d| {
                    d.cardinality == FieldCardinality::Repeated
                        && matches!(
                            d.kind,
                            FieldKind::Varint | FieldKind::Fixed32 | FieldKind::Fixed64
                        )
                }) {
                    let wire = match d.kind {
                        FieldKind::Varint => 0,
                        FieldKind::Fixed32 => 5,
                        _ => 1,
                    };
                    let mut cursor = Cursor { bytes, pos: 0 };
                    while cursor.pos < bytes.len() {
                        if *left == 0 {
                            return true;
                        }
                        *left -= 1;
                        if cursor.value(wire).is_err() {
                            return true;
                        }
                    }
                    continue;
                }
                *left -= 1;
                if let Some(FieldKind::Message(name)) = descriptor.map(|d| d.kind) {
                    if depth >= MAX_NORMALIZATION_DEPTH || *left == 0 {
                        return true;
                    }
                    let is_common = depth == 0
                        && !seen_common
                        && descriptor.is_some_and(|d| d.name == "common");
                    if is_common {
                        seen_common = true;
                    }
                    if scan(
                        bytes,
                        schema_by_name(name),
                        depth + 1,
                        left,
                        is_common,
                        timestamp,
                    ) {
                        return true;
                    }
                }
            } else {
                *left -= 1;
            }
        }
        false
    }
    let allowance = (*budget).min(MAX_SCHEMA_FIELDS);
    let mut left = allowance;
    let mut timestamp = None;
    let truncated = scan(
        payload,
        schema_for_method(method),
        0,
        &mut left,
        false,
        &mut timestamp,
    );
    *budget -= allowance - left;
    (truncated, timestamp)
}
