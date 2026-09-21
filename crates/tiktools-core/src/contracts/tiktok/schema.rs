use serde_json::{json, Value};
use ttl_live_proto::FieldValueKind;

use super::metadata::try_path_metadata;
use super::projection::PROJECTIONS;
use super::validation::{cardinality_name, protobuf_type_name, validate_projection};

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
