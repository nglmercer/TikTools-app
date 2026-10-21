//! Descriptor registry for the pinned schema.
//!
//! Generated structs answer "decode these bytes as `WebcastChatMessage`". This
//! registry answers the different question the page-owned WebSocket path asks:
//! "what is field 7 of whatever this method is called?" — including for methods
//! TikTok shipped after the pin, where no generated struct exists.
//!
//! It is plain data with no dependencies. The decoder that walks it lives in
//! `ttl-live-events`.

/// Protobuf wire representation expected for a schema field.
///
/// This is what the dynamic decoder switches on: several logical types share
/// one wire category (every integer and bool is a varint on the wire), so a
/// consumer that needs the declared type reads [`FieldSchema::value_kind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Varint,
    Fixed64,
    Fixed32,
    String,
    Bytes,
    /// Fully qualified name of the nested message type.
    Message(&'static str),
}

/// Logical protobuf type of a schema field, as declared in the `.proto` source.
///
/// Unlike [`FieldKind`], nothing is collapsed to a wire category: `int64` and
/// `uint64` stay distinct, and message and enum references keep the fully
/// qualified name of the type they point at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldValueKind {
    Double,
    Float,
    Int64,
    Uint64,
    Int32,
    Fixed64,
    Fixed32,
    Bool,
    String,
    /// Fully qualified name of the nested message type.
    Message(&'static str),
    Bytes,
    Uint32,
    /// Fully qualified name of the enum type.
    Enum(&'static str),
    Sfixed32,
    Sfixed64,
    Sint32,
    Sint64,
}

/// How many values a schema field holds, from the descriptor label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldCardinality {
    Optional,
    Required,
    Repeated,
}

/// Descriptor for one field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSchema {
    pub number: u32,
    pub name: &'static str,
    /// JSON name from the descriptor (`display_id` -> `displayId`).
    pub json_name: &'static str,
    pub kind: FieldKind,
    pub value_kind: FieldValueKind,
    pub cardinality: FieldCardinality,
    pub is_map: bool,
    pub oneof_name: Option<&'static str>,
    pub oneof_index: Option<i32>,
    pub deprecated: bool,
}

/// Descriptor for one protobuf message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageSchema {
    /// Fully qualified protobuf name, e.g. `webcast.model.message.WebcastChatMessage`.
    pub name: &'static str,
    pub fields: &'static [FieldSchema],
    pub is_map_entry: bool,
}

impl MessageSchema {
    /// Descriptor for a field number, if the schema declares one.
    pub fn field(&self, number: u32) -> Option<&'static FieldSchema> {
        self.fields.iter().find(|field| field.number == number)
    }
}

include!(concat!(env!("OUT_DIR"), "/schema_registry.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_the_methods_we_normalise() {
        for method in [
            "WebcastChatMessage",
            "WebcastGiftMessage",
            "WebcastLikeMessage",
            "WebcastMemberMessage",
            "WebcastSocialMessage",
            "WebcastRoomUserSeqMessage",
        ] {
            let schema = schema_for_method(method)
                .unwrap_or_else(|| panic!("{method} has a descriptor in the pinned schema"));
            assert_eq!(
                schema.name,
                format!("webcast.model.message.{method}"),
                "{method} resolved to the wrong package"
            );
        }
    }

    /// Methods the old snapshot could not describe. The whole point of re-pinning
    /// to v3 was that these stop coming back as unknown.
    #[test]
    fn resolves_methods_absent_from_the_retired_snapshot() {
        for method in [
            "WebcastGiftPanelUpdateMessage",
            "WebcastLinkMicFanTicketMethod",
        ] {
            assert!(
                schema_for_method(method).is_some(),
                "{method} should resolve in v3"
            );
        }
    }

    /// A genuinely newer method must stay unknown rather than matching some
    /// unrelated type, and a non-method name must never resolve at all.
    #[test]
    fn unknown_and_non_method_names_do_not_resolve() {
        assert!(schema_for_method("WebcastDefinitelyNotARealMessage").is_none());
        assert!(schema_for_method("NotEvenAWebcastMethod").is_none());
        // `User` is a real message but not a method; it must not resolve.
        assert!(schema_for_method("User").is_none());
    }

    #[test]
    fn nested_types_are_addressable_by_qualified_name() {
        let user = schema_by_name("webcast.model.base.user.User").expect("User descriptor");
        assert_eq!(user.field(3).map(|field| field.name), Some("nickname"));
        assert_eq!(user.field(38).map(|field| field.name), Some("display_id"));
    }

    #[test]
    fn chat_content_is_a_string_and_user_is_a_message() {
        let chat = schema_for_method("WebcastChatMessage").expect("chat descriptor");
        assert_eq!(
            chat.field(3).map(|field| field.kind),
            Some(FieldKind::String)
        );
        assert_eq!(
            chat.field(2).map(|field| field.kind),
            Some(FieldKind::Message("webcast.model.base.user.User"))
        );
    }

    #[test]
    fn chat_content_has_logical_string_kind_and_json_name() {
        let chat = schema_for_method("WebcastChatMessage").expect("chat descriptor");
        let content = chat.field(3).expect("content field");
        assert_eq!(content.name, "content");
        assert_eq!(content.json_name, "content");
        assert_eq!(content.kind, FieldKind::String);
        assert_eq!(content.value_kind, FieldValueKind::String);
        assert_eq!(content.cardinality, FieldCardinality::Optional);
    }

    #[test]
    fn chat_user_references_the_nested_user_message() {
        let chat = schema_for_method("WebcastChatMessage").expect("chat descriptor");
        let user = chat.field(2).expect("user field");
        assert_eq!(user.name, "user");
        assert_eq!(user.json_name, "user");
        assert_eq!(
            user.value_kind,
            FieldValueKind::Message("webcast.model.base.user.User")
        );
        assert_eq!(user.cardinality, FieldCardinality::Optional);
    }

    #[test]
    fn room_user_ranks_is_a_repeated_contributor_message() {
        let room_user =
            schema_for_method("WebcastRoomUserSeqMessage").expect("room-user descriptor");
        let ranks = room_user.field(2).expect("ranks field");
        assert_eq!(ranks.name, "ranks");
        assert_eq!(ranks.json_name, "ranks");
        assert_eq!(
            ranks.kind,
            FieldKind::Message("webcast.model.message.Contributor")
        );
        assert_eq!(
            ranks.value_kind,
            FieldValueKind::Message("webcast.model.message.Contributor")
        );
        assert_eq!(ranks.cardinality, FieldCardinality::Repeated);

        // The scalar beside it keeps its declared type instead of collapsing
        // to the shared varint wire category.
        let total = room_user.field(3).expect("total field");
        assert_eq!(total.kind, FieldKind::Varint);
        assert_eq!(total.value_kind, FieldValueKind::Int64);
        assert_eq!(total.cardinality, FieldCardinality::Optional);
    }

    #[test]
    fn enums_keep_their_type_and_json_names_are_camel_case() {
        let member = schema_for_method("WebcastMemberMessage").expect("member descriptor");
        let action = member.field(10).expect("action field");
        assert_eq!(action.kind, FieldKind::Varint);
        assert_eq!(
            action.value_kind,
            FieldValueKind::Enum("webcast.im.MemberMessageAction")
        );

        // `json_name` comes from the descriptor, not from echoing `name`.
        let effect_config = member.field(13).expect("effect_config field");
        assert_eq!(effect_config.name, "effect_config");
        assert_eq!(effect_config.json_name, "effectConfig");
        assert_eq!(effect_config.value_kind, FieldValueKind::Bytes);
    }

    #[test]
    fn gift_scalars_stay_distinct_while_sharing_the_varint_wire() {
        let gift = schema_for_method("WebcastGiftMessage").expect("gift descriptor");

        // The migration's core case: one wire category, three logical types.
        // (`gift_id` is `int64` in the pinned schema, not `uint64`.)
        let gift_id = gift.field(2).expect("gift_id field");
        assert_eq!(gift_id.name, "gift_id");
        assert_eq!(gift_id.json_name, "giftId");
        assert_eq!(gift_id.kind, FieldKind::Varint);
        assert_eq!(gift_id.value_kind, FieldValueKind::Int64);
        assert_eq!(gift_id.cardinality, FieldCardinality::Optional);

        let repeat_count = gift.field(5).expect("repeat_count field");
        assert_eq!(repeat_count.value_kind, FieldValueKind::Int32);
        assert_eq!(repeat_count.kind, FieldKind::Varint);

        let is_first_sent = gift.field(25).expect("is_first_sent field");
        assert_eq!(is_first_sent.value_kind, FieldValueKind::Bool);
        assert_eq!(is_first_sent.kind, FieldKind::Varint);
        assert_eq!(is_first_sent.json_name, "isFirstSent");

        // Nested detail resolution depends on this exact reference.
        let gift_detail = gift.field(15).expect("gift field");
        assert_eq!(gift_detail.kind, FieldKind::Message("webcast.model.Gift"));
        assert_eq!(
            gift_detail.value_kind,
            FieldValueKind::Message("webcast.model.Gift")
        );
    }

    #[test]
    fn chat_booleans_keep_their_logical_type() {
        let chat = schema_for_method("WebcastChatMessage").expect("chat descriptor");
        let visible = chat.field(4).expect("visible_to_sender field");
        assert_eq!(visible.name, "visible_to_sender");
        assert_eq!(visible.json_name, "visibleToSender");
        assert_eq!(visible.kind, FieldKind::Varint);
        assert_eq!(visible.value_kind, FieldValueKind::Bool);
        assert_eq!(visible.cardinality, FieldCardinality::Optional);
    }

    /// Every `json_name` in the registry must equal the protobuf JSON-name rule
    /// applied to `name`. The pinned descriptors come from `protoc` with no
    /// explicit `json_name` overrides, so any deviation is either a generator
    /// bug or an upstream override that downstream provenance must account for
    /// deliberately. The oracle below reimplements the rule independently of
    /// the build script.
    #[test]
    fn every_json_name_matches_the_protobuf_rule() {
        fn oracle(name: &str) -> String {
            let mut json_name = String::with_capacity(name.len());
            let mut capitalize_next = false;
            for byte in name.bytes() {
                if byte == b'_' {
                    capitalize_next = true;
                } else {
                    json_name.push(if capitalize_next {
                        byte.to_ascii_uppercase() as char
                    } else {
                        byte as char
                    });
                    capitalize_next = false;
                }
            }
            json_name
        }

        let mut checked = 0;
        for schema in schemas() {
            for field in schema.fields {
                assert_eq!(
                    field.json_name,
                    oracle(field.name),
                    "{}.{} has an unexpected json_name",
                    schema.name,
                    field.name
                );
                checked += 1;
            }
        }
        assert!(checked > 1_000, "only {checked} fields checked");
    }

    /// Guards against a schema update silently gutting the registry. The bounds
    /// are deliberately loose — they catch "the descriptor set came back empty",
    /// not ordinary upstream churn.
    #[test]
    fn the_registry_is_populated() {
        let all = schemas();
        let methods = all
            .iter()
            .filter(|schema| {
                schema
                    .name
                    .rsplit('.')
                    .next()
                    .is_some_and(|short| short.starts_with("Webcast"))
            })
            .count();

        assert!(all.len() > 500, "only {} descriptors", all.len());
        assert!(methods > 40, "only {methods} Webcast* descriptors");

        // The generated counts must describe the generated data.
        assert_eq!(all.len(), GENERATED_SCHEMA_MESSAGE_COUNT);
        assert_eq!(methods, GENERATED_WEBCAST_METHOD_COUNT);
    }

    /// Every method observed in the committed capture must resolve.
    ///
    /// Three of these (`GiftPanelUpdate`, `GiftDynamicRestriction`,
    /// `LinkMicLayoutState`) had no descriptor in the retired January-2025
    /// snapshot and decoded without field names. Re-pinning to v3 is what fixed
    /// them, so this test is the regression guard for that gain.
    #[test]
    fn every_method_in_the_capture_resolves() {
        for method in [
            "WebcastChatMessage",
            "WebcastLiveIntroMessage",
            "WebcastGiftPanelUpdateMessage",
            "WebcastGiftDynamicRestrictionMessage",
            "WebcastLinkMicLayoutStateMessage",
            "WebcastLinkMicFanTicketMethod",
        ] {
            assert!(
                schema_for_method(method).is_some(),
                "{method} appears in fixtures/events but has no descriptor"
            );
        }
    }

    /// Two methods in the capture resolve in *no* schema — neither v3 nor the
    /// retired snapshot ever described them. That is not a regression, and it is
    /// the case the dynamic decoder exists for: they still decode, with wire
    /// numbers and values but no field names.
    #[test]
    fn methods_absent_from_every_schema_stay_unresolved() {
        assert!(schema_for_method("WebcastUpdateShareRevenueNoticeMessage").is_none());
        // Not a `Webcast*` name at all, so it cannot be a method lookup.
        assert!(schema_for_method("RoomMessage").is_none());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumValueSchema {
    pub number: i32,
    pub name: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumSchema {
    pub name: &'static str,
    pub values: &'static [EnumValueSchema],
}
pub fn enum_by_name(name: &str) -> Option<&'static EnumSchema> {
    ENUMS.iter().find(|e| e.name == name)
}
/// SHA-256 of the generated descriptor set, including enum/oneof/map semantics.
pub const DESCRIPTOR_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/v3_descriptor.bin"));
