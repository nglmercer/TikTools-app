use std::error::Error;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

use prost::Message;
use prost_types::field_descriptor_proto::{Label as ProtoFieldLabel, Type as ProtoFieldType};
use prost_types::{DescriptorProto, FieldDescriptorProto, FileDescriptorSet};

const PROTO_ROOT: &str = "proto/v3";
const INCLUDE_FILE: &str = "v3.rs";
const STAGE_DIR: &str = "proto-stage/v3";
const DESCRIPTOR_FILE: &str = "v3_descriptor.bin";
const REGISTRY_FILE: &str = "schema_registry.rs";

/// Package holding the `Webcast*Message` types that WebSocket methods name.
/// Preferred when a short name exists in more than one package.
const METHOD_PACKAGE: &str = "webcast.model.message";

/// Renames applied to a staged copy of the vendored schemas before codegen.
///
/// The files under `proto/v3/` are kept byte-for-byte identical to upstream, so
/// any deviation we need has to live here, where it is reviewable.
///
/// Upstream flattens nested protobuf types into underscore-separated names.
/// `SubPinCardText_TextType` (used by `chatroom.api.SubPinCardText`) and
/// `SubPinCard_Text_TextType` (used by `chatroom.api.Text`) are distinct proto
/// enums that both normalise to the Rust identifier `SubPinCardTextTextType`,
/// which does not compile. We rename the latter, together with its single
/// reference site. This is purely cosmetic: enum *names* never appear on the
/// wire, and every tag number is left untouched.
const RENAMES: &[(&str, &str, &str)] = &[
    (
        "webcast/model/data/messages.proto",
        "enum SubPinCard_Text_TextType {",
        "enum SubPinCardNestedText_TextType {",
    ),
    (
        "webcast/chatroom/api.proto",
        ".webcast.model.data.SubPinCard_Text_TextType type = 1;",
        ".webcast.model.data.SubPinCardNestedText_TextType type = 1;",
    ),
];

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={PROTO_ROOT}");

    println!("cargo:rerun-if-env-changed=PROTOC");
    if cfg!(target_os = "android") {
        // This runs on the build host. Termux needs a Bionic executable;
        // Linux/glibc vendored binaries cannot run here.
        let protoc = env::var_os("PROTOC").unwrap_or_else(|| "protoc".into());
        let status = std::process::Command::new(&protoc).arg("--version").status()
            .map_err(|error| format!("Android requires native protoc (pkg install protobuf): {error}"))?;
        if !status.success() {
            return Err("native protoc --version failed".into());
        }
        env::set_var("PROTOC", protoc);
    } else {
        env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }

    let mut sources = Vec::new();
    collect_proto_files(Path::new(PROTO_ROOT), &mut sources)?;
    sources.sort();
    if sources.is_empty() {
        return Err(format!("no .proto files found under {PROTO_ROOT}").into());
    }
    for file in &sources {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let stage = out_dir.join(STAGE_DIR);
    let staged = stage_protos(&sources, &stage)?;
    let descriptor_path = out_dir.join(DESCRIPTOR_FILE);

    let mut config = prost_build::Config::new();
    config.include_file(INCLUDE_FILE);
    config.file_descriptor_set_path(&descriptor_path);
    // TikTok adds fields continuously; unknown fields must never abort a decode,
    // which is prost's default behaviour, so no extra configuration is needed.
    config.compile_protos(&staged, &[&stage])?;

    // The same descriptors also drive the dynamic decoder used for page-owned
    // traffic, where a static struct per message type is the wrong tool: TikTok
    // ships methods newer than any pinned schema, and those must stay readable.
    let descriptor_set = FileDescriptorSet::decode(fs::read(&descriptor_path)?.as_slice())?;
    let (messages, method_count) = collect_schema_info(&descriptor_set)?;
    fs::write(
        out_dir.join(REGISTRY_FILE),
        render_registry(&messages, method_count, &descriptor_set),
    )?;

    Ok(())
}

#[derive(Clone)]
struct MessageInfo {
    name: String,
    fields: Vec<FieldInfo>,
    is_map_entry: bool,
}

#[derive(Clone)]
struct FieldInfo {
    number: i32,
    name: String,
    json_name: String,
    kind: FieldKindInfo,
    value_kind: FieldValueKindInfo,
    cardinality: FieldCardinalityInfo,
    oneof_name: Option<String>,
    oneof_index: Option<i32>,
    deprecated: bool,
}

#[derive(Clone)]
enum FieldKindInfo {
    Varint,
    Fixed64,
    Fixed32,
    String,
    Bytes,
    Message(String),
}

#[derive(Clone)]
enum FieldValueKindInfo {
    Double,
    Float,
    Int64,
    Uint64,
    Int32,
    Fixed64,
    Fixed32,
    Bool,
    String,
    Message(String),
    Bytes,
    Uint32,
    Enum(String),
    Sfixed32,
    Sfixed64,
    Sint32,
    Sint64,
}

#[derive(Clone, Copy)]
enum FieldCardinalityInfo {
    Optional,
    Required,
    Repeated,
}

/// Flattens every message in every file into `package.Name` descriptors.
///
/// Returns the descriptors plus how many of them are addressable as a WebSocket
/// method, which is the number worth watching after a schema update.
fn collect_schema_info(
    descriptor_set: &FileDescriptorSet,
) -> Result<(Vec<MessageInfo>, usize), Box<dyn Error>> {
    let mut messages = Vec::new();
    for file in &descriptor_set.file {
        let package = file.package.as_deref().unwrap_or_default();
        for message in &file.message_type {
            collect_messages(package, message, &mut messages)?;
        }
    }

    let method_count = messages
        .iter()
        .filter(|message| {
            message
                .name
                .rsplit('.')
                .next()
                .is_some_and(|short| short.starts_with("Webcast"))
        })
        .count();

    Ok((messages, method_count))
}

fn collect_messages(
    parent_name: &str,
    message: &DescriptorProto,
    messages: &mut Vec<MessageInfo>,
) -> Result<(), Box<dyn Error>> {
    let short_name = message
        .name
        .as_deref()
        .ok_or("schema message has no name")?;
    let name = if parent_name.is_empty() {
        short_name.to_owned()
    } else {
        format!("{parent_name}.{short_name}")
    };
    let mut fields = Vec::with_capacity(message.field.len());
    for field in &message.field {
        let mut info = field_info(&name, field)?;
        info.oneof_name = field
            .oneof_index
            .and_then(|i| message.oneof_decl.get(i as usize))
            .and_then(|o| o.name.clone());
        fields.push(info);
    }
    messages.push(MessageInfo {
        name: name.clone(),
        fields,
        is_map_entry: message
            .options
            .as_ref()
            .is_some_and(|o| o.map_entry.unwrap_or(false)),
    });
    for nested in &message.nested_type {
        collect_messages(&name, nested, messages)?;
    }
    Ok(())
}

/// Extracts one field's wire kind, logical type, JSON name, and cardinality.
///
/// A malformed descriptor (missing number, name, type, or label) fails
/// generation loudly: silently dropping a field would let downstream
/// provenance drift without anyone noticing.
fn field_info(message_name: &str, field: &FieldDescriptorProto) -> Result<FieldInfo, String> {
    let name = field
        .name
        .clone()
        .ok_or_else(|| format!("malformed descriptor: a field of {message_name} has no name"))?;
    let context = format!("{message_name}.{name}");
    let number = field
        .number
        .ok_or_else(|| format!("malformed descriptor: {context} has no field number"))?;
    if number <= 0 {
        return Err(format!(
            "malformed descriptor: {context} has field number {number}"
        ));
    }
    let raw_type = field
        .r#type
        .ok_or_else(|| format!("malformed descriptor: {context} has no field type"))?;
    let proto_type = ProtoFieldType::try_from(raw_type).map_err(|_| {
        format!("malformed descriptor: {context} has unknown field type {raw_type}")
    })?;
    let kind = match proto_type {
        ProtoFieldType::Double | ProtoFieldType::Fixed64 | ProtoFieldType::Sfixed64 => {
            FieldKindInfo::Fixed64
        }
        ProtoFieldType::Float | ProtoFieldType::Fixed32 | ProtoFieldType::Sfixed32 => {
            FieldKindInfo::Fixed32
        }
        ProtoFieldType::String => FieldKindInfo::String,
        ProtoFieldType::Bytes => FieldKindInfo::Bytes,
        ProtoFieldType::Message | ProtoFieldType::Group => {
            FieldKindInfo::Message(qualified_type_name(field))
        }
        ProtoFieldType::Int64
        | ProtoFieldType::Uint64
        | ProtoFieldType::Int32
        | ProtoFieldType::Bool
        | ProtoFieldType::Uint32
        | ProtoFieldType::Enum
        | ProtoFieldType::Sint32
        | ProtoFieldType::Sint64 => FieldKindInfo::Varint,
    };
    let value_kind = match proto_type {
        ProtoFieldType::Double => FieldValueKindInfo::Double,
        ProtoFieldType::Float => FieldValueKindInfo::Float,
        ProtoFieldType::Int64 => FieldValueKindInfo::Int64,
        ProtoFieldType::Uint64 => FieldValueKindInfo::Uint64,
        ProtoFieldType::Int32 => FieldValueKindInfo::Int32,
        ProtoFieldType::Fixed64 => FieldValueKindInfo::Fixed64,
        ProtoFieldType::Fixed32 => FieldValueKindInfo::Fixed32,
        ProtoFieldType::Bool => FieldValueKindInfo::Bool,
        ProtoFieldType::String => FieldValueKindInfo::String,
        // Groups predate proto3 and cannot appear here — every vendored file
        // declares `syntax = "proto3"` — but the descriptor enum still has the
        // variant, so the match must say something deliberate: a group names a
        // message type and would decode as one.
        ProtoFieldType::Message | ProtoFieldType::Group => {
            FieldValueKindInfo::Message(qualified_type_name(field))
        }
        ProtoFieldType::Bytes => FieldValueKindInfo::Bytes,
        ProtoFieldType::Uint32 => FieldValueKindInfo::Uint32,
        ProtoFieldType::Enum => FieldValueKindInfo::Enum(qualified_type_name(field)),
        ProtoFieldType::Sfixed32 => FieldValueKindInfo::Sfixed32,
        ProtoFieldType::Sfixed64 => FieldValueKindInfo::Sfixed64,
        ProtoFieldType::Sint32 => FieldValueKindInfo::Sint32,
        ProtoFieldType::Sint64 => FieldValueKindInfo::Sint64,
    };
    let raw_label = field
        .label
        .ok_or_else(|| format!("malformed descriptor: {context} has no label"))?;
    let label = ProtoFieldLabel::try_from(raw_label)
        .map_err(|_| format!("malformed descriptor: {context} has unknown label {raw_label}"))?;
    let cardinality = match label {
        ProtoFieldLabel::Optional => FieldCardinalityInfo::Optional,
        ProtoFieldLabel::Required => FieldCardinalityInfo::Required,
        ProtoFieldLabel::Repeated => FieldCardinalityInfo::Repeated,
    };
    // `protoc` fills `json_name` for every field; the fallback keeps generation
    // correct (not merely running) if a future toolchain ever leaves it empty.
    let json_name = match field.json_name.clone() {
        Some(json_name) if !json_name.is_empty() => json_name,
        _ => fallback_json_name(&name),
    };
    Ok(FieldInfo {
        number,
        name,
        json_name,
        kind,
        value_kind,
        cardinality,
        oneof_name: None,
        oneof_index: field.oneof_index,
        deprecated: field
            .options
            .as_ref()
            .is_some_and(|o| o.deprecated.unwrap_or(false)),
    })
}

/// Protobuf JSON-name rule (`protoc`'s `ToJsonName`): drop every underscore,
/// capitalizing the letter that follows each one (`gift_id` -> `giftId`).
///
/// Only a fallback — real descriptors already carry `json_name` — but it must
/// implement the actual rule rather than echoing the snake_case name, or the
/// registry would silently disagree with protobuf JSON.
fn fallback_json_name(name: &str) -> String {
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

/// Fully qualified message or enum name from a descriptor, without the leading
/// dot `protoc` prefixes (`".webcast.im.Foo"` -> `"webcast.im.Foo"`).
fn qualified_type_name(field: &FieldDescriptorProto) -> String {
    field
        .type_name
        .as_deref()
        .unwrap_or_default()
        .trim_start_matches('.')
        .to_owned()
}

fn render_registry(
    messages: &[MessageInfo],
    method_count: usize,
    descriptors: &FileDescriptorSet,
) -> String {
    let mut source = String::new();
    source.push_str("// @generated by build.rs from proto/v3. Do not edit manually.\n\n");
    let _ = writeln!(
        source,
        "/// Number of message descriptors in the pinned schema.\n\
         pub const GENERATED_SCHEMA_MESSAGE_COUNT: usize = {};\n",
        messages.len()
    );
    let _ = writeln!(
        source,
        "/// Number of descriptors addressable as a `Webcast*` WebSocket method.\n\
         pub const GENERATED_WEBCAST_METHOD_COUNT: usize = {method_count};\n"
    );

    for (index, message) in messages.iter().enumerate() {
        let _ = writeln!(source, "static FIELDS_{index}: &[FieldSchema] = &[");
        for field in &message.fields {
            let kind = render_field_kind(&field.kind);
            let value_kind = render_field_value_kind(&field.value_kind);
            let cardinality = render_field_cardinality(&field.cardinality);
            let _ = writeln!(
                source,
                "    FieldSchema {{ number: {}, name: {:?}, json_name: {:?}, \
                 kind: {kind}, value_kind: {value_kind}, cardinality: {cardinality}, oneof_name: {}, oneof_index: {:?}, deprecated: {}, is_map: {} }},",
                field.number, field.name, field.json_name,
                field.oneof_name.as_ref().map_or("None".to_string(), |n| format!("Some({n:?})")),
                field.oneof_index, field.deprecated,
                match &field.value_kind { FieldValueKindInfo::Message(n) => messages.iter().any(|m| &m.name == n && m.is_map_entry), _ => false }
            );
        }
        source.push_str("];\n\n");
    }

    source.push_str("static SCHEMAS: &[MessageSchema] = &[\n");
    for (index, message) in messages.iter().enumerate() {
        let _ = writeln!(
            source,
            "    MessageSchema {{ name: {:?}, fields: FIELDS_{index}, is_map_entry: {} }},",
            message.name, message.is_map_entry
        );
    }
    source.push_str("];\n\n");

    let _ = write!(
        source,
        r#"/// Every descriptor in the pinned schema, in declaration order.
pub fn schemas() -> &'static [MessageSchema] {{
    SCHEMAS
}}

/// Find a descriptor by its fully qualified protobuf name.
pub fn schema_by_name(name: &str) -> Option<&'static MessageSchema> {{
    SCHEMAS.iter().find(|schema| schema.name == name)
}}

/// Find the descriptor for a TikTok WebSocket method.
///
/// In the v3 schema a method name *is* the message name, so `WebcastChatMessage`
/// resolves to `{METHOD_PACKAGE}.WebcastChatMessage`. Short names are not unique
/// across packages, so that package wins and anything else is a fallback; a
/// nested type that merely shares a short name never matches.
pub fn schema_for_method(method: &str) -> Option<&'static MessageSchema> {{
    if !method.starts_with("Webcast") {{
        return None;
    }}
    let qualified = format!("{METHOD_PACKAGE}.{{method}}");
    if let Some(schema) = SCHEMAS.iter().find(|schema| schema.name == qualified) {{
        return Some(schema);
    }}
    SCHEMAS
        .iter()
        .find(|schema| schema.name.rsplit('.').next() == Some(method))
}}
"#
    );
    let mut enums = Vec::new();
    fn walk(
        parent: &str,
        m: &DescriptorProto,
        out: &mut Vec<(String, prost_types::EnumDescriptorProto)>,
    ) {
        let name = format!("{parent}.{}", m.name.as_deref().unwrap_or_default());
        for e in &m.enum_type {
            out.push((
                format!("{name}.{}", e.name.as_deref().unwrap_or_default()),
                e.clone(),
            ));
        }
        for child in &m.nested_type {
            walk(&name, child, out);
        }
    }
    for file in &descriptors.file {
        let parent = file.package.as_deref().unwrap_or_default();
        for e in &file.enum_type {
            enums.push((
                format!("{parent}.{}", e.name.as_deref().unwrap_or_default()),
                e.clone(),
            ));
        }
        for m in &file.message_type {
            walk(parent, m, &mut enums);
        }
    }
    source.push_str("pub static ENUMS: &[EnumSchema] = &[\n");
    for (name, e) in enums {
        let _ = writeln!(source, "EnumSchema {{ name: {name:?}, values: &[");
        for v in e.value {
            let _ = writeln!(
                source,
                "EnumValueSchema {{ number: {}, name: {:?} }},",
                v.number.unwrap_or_default(),
                v.name.unwrap_or_default()
            );
        }
        source.push_str("] },\n");
    }
    source.push_str("];\n");
    source
}

fn render_field_kind(kind: &FieldKindInfo) -> String {
    match kind {
        FieldKindInfo::Varint => "FieldKind::Varint".into(),
        FieldKindInfo::Fixed64 => "FieldKind::Fixed64".into(),
        FieldKindInfo::Fixed32 => "FieldKind::Fixed32".into(),
        FieldKindInfo::String => "FieldKind::String".into(),
        FieldKindInfo::Bytes => "FieldKind::Bytes".into(),
        FieldKindInfo::Message(name) => format!("FieldKind::Message({name:?})"),
    }
}

fn render_field_value_kind(kind: &FieldValueKindInfo) -> String {
    match kind {
        FieldValueKindInfo::Double => "FieldValueKind::Double".into(),
        FieldValueKindInfo::Float => "FieldValueKind::Float".into(),
        FieldValueKindInfo::Int64 => "FieldValueKind::Int64".into(),
        FieldValueKindInfo::Uint64 => "FieldValueKind::Uint64".into(),
        FieldValueKindInfo::Int32 => "FieldValueKind::Int32".into(),
        FieldValueKindInfo::Fixed64 => "FieldValueKind::Fixed64".into(),
        FieldValueKindInfo::Fixed32 => "FieldValueKind::Fixed32".into(),
        FieldValueKindInfo::Bool => "FieldValueKind::Bool".into(),
        FieldValueKindInfo::String => "FieldValueKind::String".into(),
        FieldValueKindInfo::Message(name) => format!("FieldValueKind::Message({name:?})"),
        FieldValueKindInfo::Bytes => "FieldValueKind::Bytes".into(),
        FieldValueKindInfo::Uint32 => "FieldValueKind::Uint32".into(),
        FieldValueKindInfo::Enum(name) => format!("FieldValueKind::Enum({name:?})"),
        FieldValueKindInfo::Sfixed32 => "FieldValueKind::Sfixed32".into(),
        FieldValueKindInfo::Sfixed64 => "FieldValueKind::Sfixed64".into(),
        FieldValueKindInfo::Sint32 => "FieldValueKind::Sint32".into(),
        FieldValueKindInfo::Sint64 => "FieldValueKind::Sint64".into(),
    }
}

fn render_field_cardinality(cardinality: &FieldCardinalityInfo) -> String {
    match cardinality {
        FieldCardinalityInfo::Optional => "FieldCardinality::Optional".into(),
        FieldCardinalityInfo::Required => "FieldCardinality::Required".into(),
        FieldCardinalityInfo::Repeated => "FieldCardinality::Repeated".into(),
    }
}

/// Copies the vendored schemas into `OUT_DIR`, applying [`RENAMES`] on the way.
fn stage_protos(sources: &[PathBuf], stage: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if stage.exists() {
        fs::remove_dir_all(stage)?;
    }

    let root = Path::new(PROTO_ROOT);
    let mut applied = vec![false; RENAMES.len()];
    let mut staged = Vec::with_capacity(sources.len());

    for source in sources {
        let relative = source.strip_prefix(root)?;
        let target = stage.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut contents = fs::read_to_string(source)?;
        for (index, (file, from, to)) in RENAMES.iter().enumerate() {
            if Path::new(file) == relative && contents.contains(from) {
                contents = contents.replace(from, to);
                applied[index] = true;
            }
        }

        fs::write(&target, contents)?;
        staged.push(target);
    }

    // A rename that stops matching means upstream changed underneath us; fail
    // loudly rather than emitting code that silently differs from expectations.
    for (index, (file, from, _)) in RENAMES.iter().enumerate() {
        if !applied[index] {
            return Err(format!("stale rename for {file}: {from:?} not found").into());
        }
    }

    Ok(staged)
}

fn collect_proto_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_proto_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "proto") {
            out.push(path);
        }
    }
    Ok(())
}

// Cargo never executes build-script unit tests; these document the fallback
// rule next to its implementation. The executed coverage is the registry-wide
// `every_json_name_matches_the_protobuf_rule` test, which checks the generated
// output against an independent implementation of the same rule.
#[cfg(test)]
mod tests {
    use super::fallback_json_name;

    #[test]
    fn snake_case_becomes_lower_camel_case() {
        assert_eq!(fallback_json_name("gift_id"), "giftId");
        assert_eq!(fallback_json_name("diamond_count"), "diamondCount");
        assert_eq!(fallback_json_name("total_user"), "totalUser");
        assert_eq!(fallback_json_name("content"), "content");
        assert_eq!(fallback_json_name("effect_config"), "effectConfig");
        assert_eq!(fallback_json_name("is_first_sent"), "isFirstSent");
    }

    #[test]
    fn stray_underscores_follow_the_protoc_rule() {
        // `protoc` drops every underscore and capitalizes what follows it; a
        // trailing underscore simply vanishes.
        assert_eq!(fallback_json_name("trailing_"), "trailing");
        assert_eq!(fallback_json_name("_leading"), "Leading");
        assert_eq!(fallback_json_name("double__gap"), "doubleGap");
    }
}

#[cfg(test)]
mod descriptor_semantic_tests {
    use super::*;
    #[test]
    fn generator_retains_maps_oneofs_enum_aliases_and_deprecation() {
        use prost_types::{
            EnumDescriptorProto, EnumValueDescriptorProto, FieldOptions, FileDescriptorProto,
            MessageOptions, OneofDescriptorProto,
        };
        let field = |name: &str, number: i32, t: ProtoFieldType, type_name: Option<&str>| {
            FieldDescriptorProto {
                name: Some(name.into()),
                number: Some(number),
                r#type: Some(t as i32),
                label: Some(ProtoFieldLabel::Optional as i32),
                type_name: type_name.map(str::to_owned),
                ..Default::default()
            }
        };
        let mut map = field(
            "mapping",
            1,
            ProtoFieldType::Message,
            Some(".test.WebcastSynthetic.Entry"),
        );
        map.label = Some(ProtoFieldLabel::Repeated as i32);
        let mut oneof = field("selected", 2, ProtoFieldType::Enum, Some(".test.Choice"));
        oneof.oneof_index = Some(0);
        oneof.options = Some(FieldOptions {
            deprecated: Some(true),
            ..Default::default()
        });
        let descriptor = FileDescriptorSet {
            file: vec![FileDescriptorProto {
                package: Some("test".into()),
                message_type: vec![DescriptorProto {
                    name: Some("WebcastSynthetic".into()),
                    field: vec![map, oneof],
                    oneof_decl: vec![OneofDescriptorProto {
                        name: Some("selection".into()),
                        ..Default::default()
                    }],
                    nested_type: vec![DescriptorProto {
                        name: Some("Entry".into()),
                        options: Some(MessageOptions {
                            map_entry: Some(true),
                            ..Default::default()
                        }),
                        field: vec![
                            field("key", 1, ProtoFieldType::String, None),
                            field("value", 2, ProtoFieldType::Sint64, None),
                        ],
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                enum_type: vec![EnumDescriptorProto {
                    name: Some("Choice".into()),
                    value: vec![
                        EnumValueDescriptorProto {
                            name: Some("ZERO".into()),
                            number: Some(0),
                            ..Default::default()
                        },
                        EnumValueDescriptorProto {
                            name: Some("ALIAS".into()),
                            number: Some(0),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };
        let (messages, count) = collect_schema_info(&descriptor).unwrap();
        let rendered = render_registry(&messages, count, &descriptor);
        assert_eq!(count, 1);
        assert!(rendered.contains("is_map: true"));
        assert!(rendered.contains("oneof_name: Some(\"selection\")"));
        assert!(rendered.contains("oneof_index: Some(0), deprecated: true"));
        assert!(rendered.contains("FieldValueKind::Sint64"));
        assert!(rendered.contains("name: \"ALIAS\""));
        assert!(rendered.contains("is_map_entry: true"));
    }
}
