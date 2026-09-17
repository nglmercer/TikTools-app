//! Protobuf provenance for the TikTok automation contracts.
//!
//! Every TikTok automation-data field is normalized from one protobuf field of
//! the vendor message that carried the event. This module records that
//! projection (TikTools field <- source path + transform) and stamps it onto
//! the generated JSON Schema as `x-native-method` / `x-native-source`, so the
//! TypeScript registry and autocomplete can show where each value comes from.
//!
//! Two deliberate boundaries:
//!
//! * The runtime DTOs and their serialization are untouched: JSON output types
//!   stay authoritative from the normalized contract, and generation fails
//!   loudly if a projection's expected type drifts from what schemars emits.
//! * Transforms are declared, not inferred. The pinned signer registry only
//!   exposes wire kinds ([`ttl_live_proto::FieldKind`]), which cannot tell an
//!   `int64` from a `bool`. Logical-type-driven inference plugs into
//!   [`try_field_for_path`] when a future signer revision exposes logical
//!   types; until then this table is the single source of truth.

use serde_json::{json, Value};
use ttl_live_events::method as live_method;

/// How a normalized JSON value derives from its protobuf source field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTransform {
    /// Value passes through unchanged (string stays string, integer stays integer).
    Native,
    /// Normalized `u64` rendered as a decimal string.
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

/// Resolves a dotted protobuf path against the current registry.
///
/// Nested segments (e.g. `gift` in `gift.name`) must be message-typed and are
/// followed through [`ttl_live_proto::schema_by_name`]; the returned schema is
/// the leaf field's, carrying its authoritative `number`/`name`/`kind`. Any
/// drift — unknown method, renamed field, scalar in the middle of the path —
/// is an error so generation fails instead of stamping stale provenance.
pub fn try_field_for_path(
    method: &str,
    path: &str,
) -> Result<&'static ttl_live_proto::FieldSchema, StaleProjection> {
    let stale = |reason: String| StaleProjection {
        method: method.to_owned(),
        path: path.to_owned(),
        reason,
    };
    let mut schema = ttl_live_proto::schema_for_method(method)
        .ok_or_else(|| stale("unknown method (not in the pinned registry)".to_owned()))?;
    let segments: Vec<&str> = path.split('.').collect();
    for (index, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            return Err(stale("empty path segment".to_owned()));
        }
        let field = schema
            .fields
            .iter()
            .find(|field| field.name == *segment)
            .ok_or_else(|| stale(format!("{segment:?} is not a field of {}", schema.name)))?;
        if index + 1 == segments.len() {
            return Ok(field);
        }
        match field.kind {
            ttl_live_proto::FieldKind::Message(qualified) => {
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

/// [`try_field_for_path`] that panics loudly on drift.
pub fn field_for_path(method: &str, path: &str) -> &'static ttl_live_proto::FieldSchema {
    try_field_for_path(method, path).unwrap_or_else(|error| panic!("{error}"))
}

/// Stamps protobuf provenance onto the TikTok contract definitions.
///
/// Post-schemars `$defs` step: for each [`PROJECTIONS`] entry it resolves the
/// source path against the pinned registry, asserts the emitted JSON type
/// still matches the normalized contract, and writes `x-native-method` on the
/// definition plus `x-native-source` (`method`/`path`/`fieldNumber`/
/// `transform`) on the property. Panics on any mismatch so generated files
/// can never silently go stale.
pub fn replace_tiktok_contract_schemas(schema: &mut Value) {
    for projection in PROJECTIONS {
        let source =
            try_field_for_path(projection.method, projection.path).unwrap_or_else(|error| {
                panic!(
                    "stale TikTok contract projection for {}.{}: {error}",
                    projection.contract, projection.field
                )
            });
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
        property["x-native-source"] = json!({
            "method": projection.method,
            "path": projection.path,
            "fieldNumber": source.number,
            "transform": projection.transform.as_str(),
        });
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
        assert_eq!(source["fieldNumber"].as_u64().unwrap(), 3);
        assert_eq!(source["transform"].as_str().unwrap(), "native");
    }

    #[test]
    fn gift_id_is_a_u64_rendered_as_a_string() {
        let source = source_for("GiftAutomationData", "giftId");
        assert_eq!(source["method"].as_str().unwrap(), live_method::GIFT);
        assert_eq!(source["path"].as_str().unwrap(), "gift_id");
        assert_eq!(source["fieldNumber"].as_u64().unwrap(), 2);
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
    }

    #[test]
    fn nested_gift_detail_paths_resolve_through_the_gift_message() {
        let name = field_for_path(live_method::GIFT, "gift.name");
        assert_eq!(name.name, "name");
        assert_eq!(name.number, 16);
        let diamonds = field_for_path(live_method::GIFT, "gift.diamond_count");
        assert_eq!(diamonds.number, 12);
    }

    #[test]
    fn every_projection_resolves_against_the_pinned_registry() {
        for projection in PROJECTIONS {
            try_field_for_path(projection.method, projection.path)
                .unwrap_or_else(|error| panic!("stale projection {projection:?}: {error}"));
        }
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
