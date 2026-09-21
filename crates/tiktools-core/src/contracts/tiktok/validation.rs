use serde_json::Value;
use ttl_live_proto::{FieldCardinality, FieldSchema, FieldValueKind};

use super::projection::SourceTransform;

/// `true` for protobuf types the normalizer reads as Rust integers: every
/// varint, fixed-width, and zigzag integer width, plus enums, which decode as
/// `i32` and normalize to plain numbers.
pub(super) fn is_integer_like(kind: FieldValueKind) -> bool {
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
pub(super) fn protobuf_type_name(kind: FieldValueKind) -> &'static str {
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
pub(super) fn cardinality_name(cardinality: FieldCardinality) -> &'static str {
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
pub(super) fn validate_native_element(kind: FieldValueKind, json_type: &str) -> Result<(), String> {
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
pub(super) fn validate_projection(
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
            // The clamp promises non-negativity; the schema must say so too
            // (schemars renders unsigned DTO integers with `minimum: 0`).
            if property.get("minimum").and_then(Value::as_u64) != Some(0) {
                return Err(
                    "normalized-unsigned renders a non-negative integer (minimum 0)".to_owned(),
                );
            }
            Ok(())
        }
    }
}
