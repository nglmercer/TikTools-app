use super::metadata::{field_for_path, try_field_for_path, try_path_metadata};
use super::projection::{SourceTransform, PROJECTIONS};
use super::validation::validate_projection;
use crate::contracts::automation_contract_schema;
use serde_json::{json, Value};
use ttl_live_events::method as live_method;
use ttl_live_proto::{FieldCardinality, FieldKind, FieldSchema, FieldValueKind};

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
            json!({"type": "integer", "minimum": 0}),
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
        // Normalized-unsigned without a declared minimum is not
        // provably non-negative.
        (
            FieldValueKind::Int64,
            SourceTransform::NormalizedUnsigned,
            json!({"type": "integer"}),
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
