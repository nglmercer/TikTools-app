//! Protobuf provenance for the TikTok automation contracts.
//!
//! Every TikTok automation-data field is normalized from one protobuf field of
//! the vendor message that carried the event. This module records that
//! projection (TikTools field <- source path + transform) and stamps it onto
//! the generated JSON Schema as `x-native-method` / `x-native-source`, so the
//! TypeScript registry and autocomplete can show where each value comes from.
//!
//! Raw schema facts come from the pinned signer registry: field numbers,
//! protobuf names, JSON names, logical types, and cardinality are read off
//! [`ttl_live_proto::FieldSchema`], never restated here. This table owns only
//! what the descriptors cannot know — the normalization semantics (which
//! source path feeds which TikTools field, and through which [`SourceTransform`]).
//! Generation fails loudly when a transform disagrees with the descriptor
//! (see [`validate_projection`]), so protobuf drift surfaces here instead of
//! shipping stale provenance.
//!
//! Two deliberate boundaries:
//!
//! * The runtime DTOs and their serialization are untouched: JSON output types
//!   stay authoritative from the normalized contract, and generation fails
//!   loudly if a projection's expected type drifts from what schemars emits.
//! * Transforms are declared, not inferred. The descriptor says what the
//!   source *is*; the table says what normalization *does* with it.

use serde_json::{json, Value};
use ttl_live_events::method as live_method;
use ttl_live_proto::{FieldCardinality, FieldKind, FieldSchema, FieldValueKind};

/// How a normalized JSON value derives from its protobuf source field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTransform {
    /// Value passes through unchanged (string stays string, integer stays integer).
    Native,
    /// Normalized `u64` rendered as a decimal string. The `u64` is the
    /// `ttl-live-events` normalized intermediate (`count()` clamps the raw
    /// protobuf integer at zero); the raw descriptor type is usually `int64`
    /// and is recorded separately as `protobufType`.
    U64ToString,
    /// Normalized integer rendered as a decimal string.
    IntegerToString,
    /// Non-zero integer becomes `true`.
    NonzeroToBoolean,
    /// Signed protobuf integer clamped at zero into an unsigned count.
    NormalizedUnsigned,
}

impl SourceTransform {
    /// Kebab-case name used in `x-native-source` and the TS registry.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::U64ToString => "u64-to-string",
            Self::IntegerToString => "integer-to-string",
            Self::NonzeroToBoolean => "nonzero-to-boolean",
            Self::NormalizedUnsigned => "normalized-unsigned",
        }
    }
}

/// One TikTools field's projection from its protobuf source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Projection {
    /// `$defs` entry the field belongs to, e.g. `GiftAutomationData`.
    contract: &'static str,
    /// camelCase JSON key, e.g. `giftId`.
    field: &'static str,
    /// Vendor method, one of `ttl_live_events::method::*`.
    method: &'static str,
    /// Dotted protobuf path, e.g. `gift.name` for the nested gift detail.
    path: &'static str,
    transform: SourceTransform,
    /// Expected schemars JSON type: `string`, `boolean`, or `integer`.
    json_type: &'static str,
}

/// The full TikTools-field <- protobuf-source projection table.
///
/// TikTools-only fields (`method`, `msgId`, `isHistory`, `streakable`,
/// `giftIconUrl`) are deliberately absent: they must never claim provenance.
const PROJECTIONS: &[Projection] = &[
    // ChatAutomationData <- WebcastChatMessage
    Projection {
        contract: "ChatAutomationData",
        field: "comment",
        method: live_method::CHAT,
        path: "content",
        transform: SourceTransform::Native,
        json_type: "string",
    },
    // GiftAutomationData <- WebcastGiftMessage
    Projection {
        contract: "GiftAutomationData",
        field: "giftId",
        method: live_method::GIFT,
        path: "gift_id",
        transform: SourceTransform::U64ToString,
        json_type: "string",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "giftName",
        method: live_method::GIFT,
        path: "gift.name",
        transform: SourceTransform::Native,
        json_type: "string",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "diamondCount",
        method: live_method::GIFT,
        path: "gift.diamond_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "repeatCount",
        method: live_method::GIFT,
        path: "repeat_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "comboCount",
        method: live_method::GIFT,
        path: "combo_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "groupId",
        method: live_method::GIFT,
        path: "group_id",
        transform: SourceTransform::IntegerToString,
        json_type: "string",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "repeatEnd",
        method: live_method::GIFT,
        path: "repeat_end",
        transform: SourceTransform::NonzeroToBoolean,
        json_type: "boolean",
    },
    // LikeAutomationData <- WebcastLikeMessage
    Projection {
        contract: "LikeAutomationData",
        field: "count",
        method: live_method::LIKE,
        path: "count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "LikeAutomationData",
        field: "total",
        method: live_method::LIKE,
        path: "total",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    // SocialAutomationData <- WebcastSocialMessage
    Projection {
        contract: "SocialAutomationData",
        field: "action",
        method: live_method::SOCIAL,
        path: "action",
        transform: SourceTransform::Native,
        json_type: "integer",
    },
    Projection {
        contract: "SocialAutomationData",
        field: "followCount",
        method: live_method::SOCIAL,
        path: "follow_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "SocialAutomationData",
        field: "shareCount",
        method: live_method::SOCIAL,
        path: "share_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    // MemberAutomationData <- WebcastMemberMessage
    Projection {
        contract: "MemberAutomationData",
        field: "memberCount",
        method: live_method::MEMBER,
        path: "member_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "MemberAutomationData",
        field: "action",
        method: live_method::MEMBER,
        path: "action",
        transform: SourceTransform::Native,
        json_type: "integer",
    },
    // RoomStatsAutomationData <- WebcastRoomUserSeqMessage. `viewers` is the
    // normalized `total`; the raw name never leaks into the contract.
    Projection {
        contract: "RoomStatsAutomationData",
        field: "viewers",
        method: live_method::ROOM_USER,
        path: "total",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "RoomStatsAutomationData",
        field: "totalUsers",
        method: live_method::ROOM_USER,
        path: "total_user",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "RoomStatsAutomationData",
        field: "popularity",
        method: live_method::ROOM_USER,
        path: "popularity",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "RoomStatsAutomationData",
        field: "anonymous",
        method: live_method::ROOM_USER,
        path: "anonymous",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
];

/// A projection path that no longer resolves against the pinned registry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("stale TikTok contract projection: {method}.{path} ({reason})")]
pub struct StaleProjection {
    method: String,
    path: String,
    reason: String,
}

/// A projection path resolved against the pinned registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathMetadata {
    /// The leaf field's descriptor, carrying its authoritative `number`,
    /// `name`, `json_name`, `value_kind`, and `cardinality`.
    pub field: &'static FieldSchema,
    /// Dotted protobuf JSON path (`gift.diamond_count` -> `gift.diamondCount`),
    /// built from each segment's descriptor `json_name`.
    pub json_path: String,
}

/// Resolves a dotted protobuf path against the current registry.
///
/// Nested segments (e.g. `gift` in `gift.name`) must be message-typed and are
/// followed through [`ttl_live_proto::schema_by_name`]. Any drift — unknown
/// method, renamed field, scalar in the middle of the path, missing nested
/// descriptor — is an error so generation fails instead of stamping stale
/// provenance.
pub fn try_path_metadata(method: &str, path: &str) -> Result<PathMetadata, StaleProjection> {
    let stale = |reason: String| StaleProjection {
        method: method.to_owned(),
        path: path.to_owned(),
        reason,
    };
    let mut schema = ttl_live_proto::schema_for_method(method)
        .ok_or_else(|| stale("unknown method (not in the pinned registry)".to_owned()))?;
    let segments: Vec<&str> = path.split('.').collect();
    let mut json_segments = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            return Err(stale("empty path segment".to_owned()));
        }
        let field = schema
            .fields
            .iter()
            .find(|field| field.name == *segment)
            .ok_or_else(|| stale(format!("{segment:?} is not a field of {}", schema.name)))?;
        json_segments.push(field.json_name);
        if index + 1 == segments.len() {
            return Ok(PathMetadata {
                field,
                json_path: json_segments.join("."),
            });
        }
        match field.kind {
            FieldKind::Message(qualified) => {
                schema = ttl_live_proto::schema_by_name(qualified)
                    .ok_or_else(|| stale(format!("nested type {qualified} has no descriptor")))?;
            }
            _ => {
                return Err(stale(format!(
                    "{segment:?} is a scalar; cannot resolve a nested path through it"
                )));
            }
        }
    }
    Err(stale("empty path".to_owned()))
}

/// Resolves a dotted protobuf path to its leaf field descriptor.
///
/// Convenience wrapper over [`try_path_metadata`] for callers that only need
/// the leaf [`FieldSchema`].
pub fn try_field_for_path(
    method: &str,
    path: &str,
) -> Result<&'static ttl_live_proto::FieldSchema, StaleProjection> {
    try_path_metadata(method, path).map(|metadata| metadata.field)
}

/// [`try_field_for_path`] that panics loudly on drift.
pub fn field_for_path(method: &str, path: &str) -> &'static ttl_live_proto::FieldSchema {
    try_field_for_path(method, path).unwrap_or_else(|error| panic!("{error}"))
}

/// `true` for protobuf types the normalizer reads as Rust integers: every
/// varint, fixed-width, and zigzag integer width, plus enums, which decode as
/// `i32` and normalize to plain numbers.
fn is_integer_like(kind: FieldValueKind) -> bool {
    matches!(
        kind,
        FieldValueKind::Int32
            | FieldValueKind::Int64
            | FieldValueKind::Uint32
            | FieldValueKind::Uint64
            | FieldValueKind::Sint32
            | FieldValueKind::Sint64
            | FieldValueKind::Fixed32
            | FieldValueKind::Fixed64
            | FieldValueKind::Sfixed32
            | FieldValueKind::Sfixed64
            | FieldValueKind::Enum(_)
    )
}

/// Canonical protobuf type name for `x-native-source.protobufType`.
///
/// Deliberately the `.proto` spelling (`int64`, `sint32`, …), not the wire
/// category: several of these share one wire kind. Keep the output stable —
/// generated files depend on it.
fn protobuf_type_name(kind: FieldValueKind) -> &'static str {
    match kind {
        FieldValueKind::Double => "double",
        FieldValueKind::Float => "float",
        FieldValueKind::Int64 => "int64",
        FieldValueKind::Uint64 => "uint64",
        FieldValueKind::Int32 => "int32",
        FieldValueKind::Fixed64 => "fixed64",
        FieldValueKind::Fixed32 => "fixed32",
        FieldValueKind::Bool => "bool",
        FieldValueKind::String => "string",
        FieldValueKind::Message(_) => "message",
        FieldValueKind::Bytes => "bytes",
        FieldValueKind::Uint32 => "uint32",
        FieldValueKind::Enum(_) => "enum",
        FieldValueKind::Sfixed32 => "sfixed32",
        FieldValueKind::Sfixed64 => "sfixed64",
        FieldValueKind::Sint32 => "sint32",
        FieldValueKind::Sint64 => "sint64",
    }
}

/// Canonical cardinality name for `x-native-source.cardinality`.
fn cardinality_name(cardinality: FieldCardinality) -> &'static str {
    match cardinality {
        FieldCardinality::Optional => "optional",
        FieldCardinality::Required => "required",
        FieldCardinality::Repeated => "repeated",
    }
}

/// Checks that a `Native` source type projects to the given JSON type.
///
/// Used for scalar outputs and, for repeated sources, for the array's element
/// type: the source shape always derives from the descriptor.
fn validate_native_element(kind: FieldValueKind, json_type: &str) -> Result<(), String> {
    let expected = if kind == FieldValueKind::String {
        "string"
    } else if kind == FieldValueKind::Bool {
        "boolean"
    } else if matches!(kind, FieldValueKind::Float | FieldValueKind::Double) {
        "number"
    } else if is_integer_like(kind) {
        "integer"
    } else {
        return Err(format!(
            "native cannot project protobuf {} (message/bytes sources need an explicit transform)",
            protobuf_type_name(kind)
        ));
    };
    if json_type == expected {
        Ok(())
    } else {
        Err(format!(
            "native protobuf {} projects to {expected}, not {json_type}",
            protobuf_type_name(kind)
        ))
    }
}

/// Validates that a transform is compatible with its protobuf source and the
/// emitted JSON property.
///
/// The descriptor owns the source type, the normalized contract owns the
/// output type, and this function checks the two ends meet through the
/// declared transform: a descriptor drift that changes a field's logical type
/// fails generation instead of stamping a lie. Only the reason is returned;
/// the caller adds the contract/field context.
fn validate_projection(
    source: &FieldSchema,
    transform: SourceTransform,
    property: &Value,
) -> Result<(), String> {
    let output = property
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("<missing>");
    // A repeated source is an array on the wire; projecting it as a scalar
    // would misrepresent the shape, and no transform maps over arrays.
    if source.cardinality == FieldCardinality::Repeated {
        if transform != SourceTransform::Native {
            return Err(format!(
                "{} cannot project repeated protobuf {} (only native array projection is supported)",
                transform.as_str(),
                protobuf_type_name(source.value_kind)
            ));
        }
        if output != "array" {
            return Err(format!(
                "repeated protobuf {} must project to an array, not {output}",
                protobuf_type_name(source.value_kind)
            ));
        }
        let items = property
            .get("items")
            .and_then(|items| items.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        return validate_native_element(source.value_kind, items).map_err(|reason| {
            format!(
                "repeated protobuf {}: {reason}",
                protobuf_type_name(source.value_kind)
            )
        });
    }
    match transform {
        SourceTransform::Native => validate_native_element(source.value_kind, output),
        SourceTransform::U64ToString | SourceTransform::IntegerToString => {
            if !is_integer_like(source.value_kind) {
                return Err(format!(
                    "{} needs an integer-like protobuf source, not {}",
                    transform.as_str(),
                    protobuf_type_name(source.value_kind)
                ));
            }
            if output != "string" {
                return Err(format!(
                    "{} renders a string, not {output}",
                    transform.as_str()
                ));
            }
            Ok(())
        }
        SourceTransform::NonzeroToBoolean => {
            if !is_integer_like(source.value_kind) {
                return Err(format!(
                    "nonzero-to-boolean needs an integer-like protobuf source, not {}",
                    protobuf_type_name(source.value_kind)
                ));
            }
            if output != "boolean" {
                return Err(format!(
                    "nonzero-to-boolean renders a boolean, not {output}"
                ));
            }
            Ok(())
        }
        SourceTransform::NormalizedUnsigned => {
            if !is_integer_like(source.value_kind) {
                return Err(format!(
                    "normalized-unsigned needs an integer-like protobuf source, not {}",
                    protobuf_type_name(source.value_kind)
                ));
            }
            if output != "integer" {
                return Err(format!(
                    "normalized-unsigned renders an integer, not {output}"
                ));
            }
            Ok(())
        }
    }
}

/// Stamps protobuf provenance onto the TikTok contract definitions.
///
/// Post-schemars `$defs` step: for each [`PROJECTIONS`] entry it resolves the
/// source path against the pinned registry, asserts the emitted JSON type
/// still matches the normalized contract, validates the transform against the
/// descriptor's logical type, and writes `x-native-method` on the definition
/// plus `x-native-source` (`method`/`path`/`jsonPath`/`fieldNumber`/
/// `protobufType`/`cardinality`/`transform`, with `messageType`/`enumType` for
/// message/enum leaves) on the property. Panics on any mismatch so generated
/// files can never silently go stale.
pub fn replace_tiktok_contract_schemas(schema: &mut Value) {
    for projection in PROJECTIONS {
        let metadata =
            try_path_metadata(projection.method, projection.path).unwrap_or_else(|error| {
                panic!(
                    "stale TikTok contract projection for {}.{}: {error}",
                    projection.contract, projection.field
                )
            });
        let source = metadata.field;
        let defs = schema
            .get_mut("$defs")
            .and_then(Value::as_object_mut)
            .expect("automation contract schema must have a $defs object");
        let definition = defs
            .get_mut(projection.contract)
            .unwrap_or_else(|| panic!("contract {} missing from $defs", projection.contract));
        match definition.get("x-native-method") {
            None => {
                definition["x-native-method"] = Value::String(projection.method.to_owned());
            }
            Some(existing) => {
                assert_eq!(
                    existing.as_str(),
                    Some(projection.method),
                    "contract {} mixes protobuf methods",
                    projection.contract
                );
            }
        }
        let properties = definition
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .unwrap_or_else(|| panic!("contract {} has no properties", projection.contract));
        let property = properties.get_mut(projection.field).unwrap_or_else(|| {
            panic!(
                "contract {} lost its projected field {}",
                projection.contract, projection.field
            )
        });
        assert_eq!(
            property.get("type").and_then(Value::as_str),
            Some(projection.json_type),
            "contract {}.{} changed JSON type (normalized contract is authoritative)",
            projection.contract,
            projection.field
        );
        validate_projection(source, projection.transform, property).unwrap_or_else(|reason| {
            panic!(
                "contract {}.{}: {} cannot project {}.{} (protobuf {}): {reason}",
                projection.contract,
                projection.field,
                projection.transform.as_str(),
                projection.method,
                projection.path,
                protobuf_type_name(source.value_kind),
            )
        });
        let mut stamped = json!({
            "method": projection.method,
            "path": projection.path,
            "jsonPath": metadata.json_path,
            "fieldNumber": source.number,
            "protobufType": protobuf_type_name(source.value_kind),
            "cardinality": cardinality_name(source.cardinality),
            "transform": projection.transform.as_str(),
        });
        match source.value_kind {
            FieldValueKind::Message(name) => {
                stamped["messageType"] = Value::String(name.to_owned());
            }
            FieldValueKind::Enum(name) => {
                stamped["enumType"] = Value::String(name.to_owned());
            }
            _ => {}
        }
        property["x-native-source"] = stamped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::automation_contract_schema;

    fn source_for(contract: &str, field: &str) -> Value {
        let schema = automation_contract_schema();
        schema["$defs"][contract]["properties"][field]["x-native-source"].clone()
    }

    /// Synthetic descriptor leaf for [`validate_projection`] unit tests. Only
    /// `value_kind` and `cardinality` matter to validation; the rest is filler.
    fn probe_field(value_kind: FieldValueKind, cardinality: FieldCardinality) -> FieldSchema {
        FieldSchema {
            number: 1,
            name: "probe",
            json_name: "probe",
            kind: FieldKind::Varint,
            value_kind,
            cardinality,
        }
    }

    #[test]
    fn chat_comment_points_at_the_vendor_content_field() {
        let schema = automation_contract_schema();
        assert_eq!(
            schema["$defs"]["ChatAutomationData"]["x-native-method"]
                .as_str()
                .unwrap(),
            live_method::CHAT
        );
        let source = source_for("ChatAutomationData", "comment");
        assert_eq!(source["method"].as_str().unwrap(), live_method::CHAT);
        assert_eq!(source["path"].as_str().unwrap(), "content");
        assert_eq!(source["jsonPath"].as_str().unwrap(), "content");
        assert_eq!(source["fieldNumber"].as_u64().unwrap(), 3);
        assert_eq!(source["protobufType"].as_str().unwrap(), "string");
        assert_eq!(source["cardinality"].as_str().unwrap(), "optional");
        assert_eq!(source["transform"].as_str().unwrap(), "native");
    }

    #[test]
    fn gift_id_is_a_u64_rendered_as_a_string() {
        let source = source_for("GiftAutomationData", "giftId");
        assert_eq!(source["method"].as_str().unwrap(), live_method::GIFT);
        assert_eq!(source["path"].as_str().unwrap(), "gift_id");
        assert_eq!(source["jsonPath"].as_str().unwrap(), "giftId");
        assert_eq!(source["fieldNumber"].as_u64().unwrap(), 2);
        // The `u64` is the normalized intermediate; the descriptor says int64.
        assert_eq!(source["protobufType"].as_str().unwrap(), "int64");
        assert_eq!(source["cardinality"].as_str().unwrap(), "optional");
        assert_eq!(source["transform"].as_str().unwrap(), "u64-to-string");
        // The JSON output type stays authoritative from the normalized contract.
        let schema = automation_contract_schema();
        assert_eq!(
            schema["$defs"]["GiftAutomationData"]["properties"]["giftId"]["type"]
                .as_str()
                .unwrap(),
            "string"
        );
    }

    #[test]
    fn gift_repeat_end_is_a_nonzero_boolean() {
        let schema = automation_contract_schema();
        let property = &schema["$defs"]["GiftAutomationData"]["properties"]["repeatEnd"];
        assert_eq!(property["type"].as_str().unwrap(), "boolean");
        assert_eq!(
            property["x-native-source"]["transform"].as_str().unwrap(),
            "nonzero-to-boolean"
        );
        assert_eq!(
            property["x-native-source"]["fieldNumber"].as_u64().unwrap(),
            9
        );
        assert_eq!(
            property["x-native-source"]["protobufType"]
                .as_str()
                .unwrap(),
            "int32"
        );
        assert_eq!(
            property["x-native-source"]["jsonPath"].as_str().unwrap(),
            "repeatEnd"
        );
    }

    #[test]
    fn nested_paths_carry_json_names_and_logical_types() {
        let diamonds = source_for("GiftAutomationData", "diamondCount");
        assert_eq!(diamonds["path"].as_str().unwrap(), "gift.diamond_count");
        assert_eq!(diamonds["jsonPath"].as_str().unwrap(), "gift.diamondCount");
        assert_eq!(diamonds["fieldNumber"].as_u64().unwrap(), 12);
        assert_eq!(diamonds["protobufType"].as_str().unwrap(), "int32");
        assert_eq!(diamonds["cardinality"].as_str().unwrap(), "optional");
        assert_eq!(
            diamonds["transform"].as_str().unwrap(),
            "normalized-unsigned"
        );
        let name = source_for("GiftAutomationData", "giftName");
        assert_eq!(name["path"].as_str().unwrap(), "gift.name");
        assert_eq!(name["jsonPath"].as_str().unwrap(), "gift.name");
        assert_eq!(name["protobufType"].as_str().unwrap(), "string");
    }

    #[test]
    fn member_action_is_a_native_enum() {
        let source = source_for("MemberAutomationData", "action");
        assert_eq!(source["path"].as_str().unwrap(), "action");
        assert_eq!(source["protobufType"].as_str().unwrap(), "enum");
        assert_eq!(
            source["enumType"].as_str().unwrap(),
            "webcast.im.MemberMessageAction"
        );
        assert_eq!(source["transform"].as_str().unwrap(), "native");
        let schema = automation_contract_schema();
        assert_eq!(
            schema["$defs"]["MemberAutomationData"]["properties"]["action"]["type"]
                .as_str()
                .unwrap(),
            "integer"
        );
    }

    #[test]
    fn nested_gift_detail_paths_resolve_through_the_gift_message() {
        let name = field_for_path(live_method::GIFT, "gift.name");
        assert_eq!(name.name, "name");
        assert_eq!(name.number, 16);
        let diamonds = field_for_path(live_method::GIFT, "gift.diamond_count");
        assert_eq!(diamonds.number, 12);
        let metadata = try_path_metadata(live_method::GIFT, "gift.diamond_count")
            .expect("nested gift path resolves");
        assert_eq!(metadata.json_path, "gift.diamondCount");
        assert_eq!(metadata.field.value_kind, FieldValueKind::Int32);
    }

    #[test]
    fn repeated_sources_resolve_with_cardinality() {
        // Not projected as an automation field — the runtime payload carries no
        // ranks array — but path metadata must still describe it correctly.
        let metadata = try_path_metadata(live_method::ROOM_USER, "ranks")
            .expect("ranks resolves against the pinned registry");
        assert_eq!(metadata.json_path, "ranks");
        assert_eq!(metadata.field.number, 2);
        assert_eq!(metadata.field.cardinality, FieldCardinality::Repeated);
        assert_eq!(
            metadata.field.value_kind,
            FieldValueKind::Message("webcast.model.message.Contributor")
        );
    }

    #[test]
    fn every_projection_resolves_against_the_pinned_registry() {
        for projection in PROJECTIONS {
            try_path_metadata(projection.method, projection.path)
                .unwrap_or_else(|error| panic!("stale projection {projection:?}: {error}"));
        }
    }

    #[test]
    fn compatible_transforms_pass_validation() {
        let optional = FieldCardinality::Optional;
        let ok = [
            (
                FieldValueKind::String,
                SourceTransform::Native,
                json!({"type": "string"}),
            ),
            (
                FieldValueKind::Bool,
                SourceTransform::Native,
                json!({"type": "boolean"}),
            ),
            (
                FieldValueKind::Int64,
                SourceTransform::Native,
                json!({"type": "integer"}),
            ),
            (
                FieldValueKind::Int32,
                SourceTransform::Native,
                json!({"type": "integer"}),
            ),
            (
                FieldValueKind::Enum("webcast.im.MemberMessageAction"),
                SourceTransform::Native,
                json!({"type": "integer"}),
            ),
            (
                FieldValueKind::Double,
                SourceTransform::Native,
                json!({"type": "number"}),
            ),
            (
                FieldValueKind::Int64,
                SourceTransform::U64ToString,
                json!({"type": "string"}),
            ),
            (
                FieldValueKind::Uint64,
                SourceTransform::U64ToString,
                json!({"type": "string"}),
            ),
            (
                FieldValueKind::Int32,
                SourceTransform::IntegerToString,
                json!({"type": "string"}),
            ),
            (
                FieldValueKind::Int32,
                SourceTransform::NonzeroToBoolean,
                json!({"type": "boolean"}),
            ),
            (
                FieldValueKind::Int64,
                SourceTransform::NormalizedUnsigned,
                json!({"type": "integer"}),
            ),
        ];
        for (kind, transform, property) in ok {
            validate_projection(&probe_field(kind, optional), transform, &property).unwrap_or_else(
                |reason| panic!("{transform:?} over {kind:?} should validate: {reason}"),
            );
        }
        // A repeated native source projects to an array of the element type.
        validate_projection(
            &probe_field(FieldValueKind::String, FieldCardinality::Repeated),
            SourceTransform::Native,
            &json!({"type": "array", "items": {"type": "string"}}),
        )
        .expect("repeated native strings project to a string array");
    }

    #[test]
    fn incompatible_transforms_fail_validation() {
        let optional = FieldCardinality::Optional;
        let bad = [
            // u64-to-string over a string source.
            (
                FieldValueKind::String,
                SourceTransform::U64ToString,
                json!({"type": "string"}),
            ),
            // nonzero-to-boolean over a message source.
            (
                FieldValueKind::Message("webcast.model.Gift"),
                SourceTransform::NonzeroToBoolean,
                json!({"type": "boolean"}),
            ),
            // Native string rendered as a boolean.
            (
                FieldValueKind::String,
                SourceTransform::Native,
                json!({"type": "boolean"}),
            ),
            // Native integer rendered as a string.
            (
                FieldValueKind::Int64,
                SourceTransform::Native,
                json!({"type": "string"}),
            ),
            // Native message has no scalar shape.
            (
                FieldValueKind::Message("webcast.model.Gift"),
                SourceTransform::Native,
                json!({"type": "object"}),
            ),
            // Transforms do not map over arrays.
            (
                FieldValueKind::Int64,
                SourceTransform::NormalizedUnsigned,
                json!({"type": "array", "items": {"type": "integer"}}),
            ),
        ];
        for (kind, transform, property) in bad {
            assert!(
                validate_projection(&probe_field(kind, optional), transform, &property).is_err(),
                "{transform:?} over {kind:?} into {property} should fail validation"
            );
        }
        // A repeated source projected as a scalar misrepresents the shape.
        assert!(validate_projection(
            &probe_field(FieldValueKind::Int64, FieldCardinality::Repeated),
            SourceTransform::Native,
            &json!({"type": "integer"}),
        )
        .is_err());
        // ... as does a transform over a repeated source.
        assert!(validate_projection(
            &probe_field(FieldValueKind::Int64, FieldCardinality::Repeated),
            SourceTransform::NormalizedUnsigned,
            &json!({"type": "array", "items": {"type": "integer"}}),
        )
        .is_err());
    }

    #[test]
    fn app_only_fields_carry_no_provenance() {
        let schema = automation_contract_schema();
        let cases: &[(&str, &[&str])] = &[
            ("ChatAutomationData", &["method", "msgId", "isHistory"]),
            (
                "GiftAutomationData",
                &["streakable", "giftIconUrl", "method", "msgId", "isHistory"],
            ),
            ("LikeAutomationData", &["method", "msgId", "isHistory"]),
            ("SocialAutomationData", &["method", "msgId", "isHistory"]),
            ("MemberAutomationData", &["method", "msgId", "isHistory"]),
            ("RoomStatsAutomationData", &["method", "msgId", "isHistory"]),
        ];
        for (contract, fields) in cases {
            for field in *fields {
                assert!(
                    schema["$defs"][contract]["properties"][field]
                        .get("x-native-source")
                        .is_none(),
                    "{contract}.{field} must not claim protobuf provenance"
                );
            }
        }
    }

    #[test]
    fn stale_paths_fail() {
        assert!(try_field_for_path(live_method::CHAT, "no_such_field").is_err());
        assert!(try_field_for_path(live_method::GIFT, "gift.no_such_field").is_err());
        assert!(try_field_for_path(live_method::CHAT, "content.deeper").is_err());
        assert!(try_field_for_path("WebcastNopeMessage", "content").is_err());
        assert!(try_field_for_path(live_method::CHAT, "").is_err());
    }
}
