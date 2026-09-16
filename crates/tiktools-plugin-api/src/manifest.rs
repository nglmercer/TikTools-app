//! Versioned, runtime-neutral plugin manifests.
//!
//! The host only accepts the native schema. There is no compatibility parser
//! for the removed TypeScript plugin format: a package must declare its
//! runtime and entry explicitly before it can be discovered.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::{TIKTOOLS_PLUGIN_ABI_VERSION, TIKTOOLS_PLUGIN_PROTOCOL_VERSION};

const PLUGIN_SCHEMA_VERSION: u32 = 2;
const MAX_MANIFEST_BYTES: usize = 256 * 1024;
const MAX_LIST_ENTRIES: usize = 128;
const MAX_DESCRIPTOR_BYTES: usize = 64 * 1024;

pub const DEFAULT_PLUGIN_ACTION_TIMEOUT_MS: u64 = 30_000;
pub const MAX_PLUGIN_ACTION_TIMEOUT_MS: u64 = 180_000;

/// Default deadline for one pre-filter processor call. Processors run on the
/// live-message hot path and fail open, so the default stays far below the
/// action timeout. Tune after benchmarking local processor latency.
pub const DEFAULT_PLUGIN_PROCESSOR_TIMEOUT_MS: u64 = 250;
/// Upper bound for a manifest-declared processor `timeoutMs`. Live enrichment
/// must never wait seconds for one chat event.
pub const MAX_PLUGIN_PROCESSOR_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginRuntimeKind {
    Native,
    Wasm,
    Process,
}

/// Runtime boundary semantics used for host policy and documentation. This
/// is intentionally separate from the serialized `trust` field so schema v2
/// values remain compatible while process isolation is not mislabeled as a
/// sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginSecurityModel {
    Trusted,
    Isolated,
    Sandboxed,
}

impl PluginRuntimeKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "native" => Some(Self::Native),
            "wasm" => Some(Self::Wasm),
            "process" => Some(Self::Process),
            _ => None,
        }
    }

    pub const fn security_model(self) -> PluginSecurityModel {
        match self {
            Self::Native => PluginSecurityModel::Trusted,
            Self::Process => PluginSecurityModel::Isolated,
            Self::Wasm => PluginSecurityModel::Sandboxed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginTrust {
    /// Trusted in-process native code, or a process executable whose OS
    /// permissions remain outside TikTools' protocol policy.
    #[default]
    Trusted,
    /// A runtime that supplies an actual execution sandbox, currently the
    /// intended label for WASM. WASI grants still depend on host policy.
    Sandboxed,
    /// Declarative package metadata; this value is not an OS sandbox by
    /// itself and is preserved for manifest compatibility.
    Untrusted,
}

impl PluginTrust {
    /// Returns the schema-v2 compatibility default for a runtime when a
    /// manifest omits `trust`. The separate `security_model()` API describes
    /// the actual runtime boundary; this value preserves the old manifest
    /// interpretation until a deliberate schema revision changes it.
    pub const fn default_for_runtime(runtime: PluginRuntimeKind) -> Self {
        match runtime {
            PluginRuntimeKind::Native => Self::Trusted,
            PluginRuntimeKind::Wasm | PluginRuntimeKind::Process => Self::Sandboxed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub runtime: PluginRuntimeKind,
    pub entry: String,
    pub trust: PluginTrust,
    pub capabilities: Vec<String>,
    pub permissions: Vec<String>,
    pub protocol_version: u32,
    pub abi_version: Option<u32>,
    pub targets: Vec<String>,
    /// JSON action descriptors exposed by the plugin, if any.
    pub action_types: Vec<Value>,
    /// JSON event-type descriptors a plugin can publish (hotkeys, timers).
    /// Entries are validated when the host merges its catalog; the raw list
    /// is kept here so discovery never fails on a single bad entry.
    pub event_types: Vec<Value>,
    /// JSON processor descriptors a plugin offers for pre-filter event
    /// enrichment. Like `event_types`, entries are validated when the host
    /// builds its processor catalog; the raw list is kept here so discovery
    /// never fails on a single bad entry.
    pub processor_types: Vec<Value>,
    /// Host-rendered settings schema, kept as data and never executed.
    pub settings_schema: Option<Value>,
    pub settings_ui_hints: Option<Value>,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("plugin manifest is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("plugin manifest must be a JSON object")]
    NotAnObject,
    #[error("plugin manifest has unsupported schema version {0}")]
    UnsupportedSchema(u32),
    #[error("plugin manifest field `{0}` is missing or invalid")]
    MissingField(&'static str),
    #[error("plugin manifest field `{0}` is invalid")]
    InvalidField(&'static str),
    #[error("plugin manifest entry must stay inside its package")]
    UnsafeEntry,
    #[error("plugin manifest is larger than {MAX_MANIFEST_BYTES} bytes")]
    TooLarge,
}

impl PluginManifest {
    pub const fn security_model(&self) -> PluginSecurityModel {
        self.runtime.security_model()
    }

    pub fn from_json_str(input: &str) -> Result<Self, ManifestError> {
        if input.len() > MAX_MANIFEST_BYTES {
            return Err(ManifestError::TooLarge);
        }
        let value: Value = serde_json::from_str(input)?;
        Self::from_value(value)
    }

    pub fn from_value(value: Value) -> Result<Self, ManifestError> {
        let object = value.as_object().ok_or(ManifestError::NotAnObject)?;
        let schema_version =
            number(object, "schemaVersion").ok_or(ManifestError::MissingField("schemaVersion"))?;
        if schema_version != PLUGIN_SCHEMA_VERSION {
            return Err(ManifestError::UnsupportedSchema(schema_version));
        }

        let id = required_string(object, "id")?;
        if !is_valid_plugin_id(&id) {
            return Err(ManifestError::InvalidField("id"));
        }
        let name = required_string(object, "name")?;
        if name.trim().is_empty() || name.len() > 256 {
            return Err(ManifestError::InvalidField("name"));
        }
        let version = required_string(object, "version")?;
        if version.trim().is_empty()
            || version.len() > 128
            || version.chars().any(char::is_whitespace)
        {
            return Err(ManifestError::InvalidField("version"));
        }
        let description = optional_string(object, "description");
        if description
            .as_deref()
            .is_some_and(|value| value.len() > 4_096)
        {
            return Err(ManifestError::InvalidField("description"));
        }

        let entry = required_string(object, "entry")?;
        if !is_safe_relative_path(&entry) {
            return Err(ManifestError::UnsafeEntry);
        }
        let runtime = optional_string(object, "runtime")
            .and_then(|value| PluginRuntimeKind::parse(&value))
            .ok_or(ManifestError::MissingField("runtime"))?;

        let default_trust = PluginTrust::default_for_runtime(runtime);
        let trust = match optional_string(object, "trust") {
            None => default_trust,
            Some(value) => match value.as_str() {
                "trusted" => PluginTrust::Trusted,
                "sandboxed" => PluginTrust::Sandboxed,
                "untrusted" => PluginTrust::Untrusted,
                _ => return Err(ManifestError::InvalidField("trust")),
            },
        };

        let capabilities = string_list(object, "capabilities")?;
        let permissions = string_list(object, "permissions")?;
        let targets = string_list(object, "targets")?;
        let protocol_version =
            number(object, "protocolVersion").unwrap_or(TIKTOOLS_PLUGIN_PROTOCOL_VERSION);
        let abi_version = number(object, "abiVersion");
        if protocol_version == 0 {
            return Err(ManifestError::InvalidField("protocolVersion"));
        }
        if runtime == PluginRuntimeKind::Native && abi_version.is_some_and(|version| version == 0) {
            return Err(ManifestError::InvalidField("abiVersion"));
        }

        let action_types = json_list(object, "actionTypes")?;
        for action_type in &action_types {
            validate_action_type(action_type)?;
        }
        let event_types = json_list(object, "eventTypes")?;
        let processor_types = json_list(object, "processorTypes")?;
        let (settings_schema, settings_ui_hints) = settings(object)?;

        Ok(Self {
            schema_version,
            id,
            name,
            version,
            description,
            runtime,
            entry,
            trust,
            capabilities,
            permissions,
            protocol_version,
            abi_version,
            targets,
            action_types,
            event_types,
            processor_types,
            settings_schema,
            settings_ui_hints,
        })
    }

    pub fn validate_compatibility(&self) -> Result<(), ManifestError> {
        if self.schema_version != PLUGIN_SCHEMA_VERSION {
            return Err(ManifestError::UnsupportedSchema(self.schema_version));
        }
        if self.protocol_version != TIKTOOLS_PLUGIN_PROTOCOL_VERSION {
            return Err(ManifestError::InvalidField("protocolVersion"));
        }
        if self.runtime == PluginRuntimeKind::Native
            && self
                .abi_version
                .is_some_and(|version| version != TIKTOOLS_PLUGIN_ABI_VERSION)
        {
            return Err(ManifestError::InvalidField("abiVersion"));
        }
        for action_type in &self.action_types {
            validate_action_type(action_type)?;
        }
        Ok(())
    }

    pub fn target_matches_current_platform(&self) -> bool {
        if self.targets.is_empty() {
            return true;
        }
        let target = current_target();
        let platform = current_platform();
        self.targets
            .iter()
            .any(|candidate| candidate == &target || candidate == &platform)
    }
}

impl fmt::Display for PluginRuntimeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Native => "native",
            Self::Wasm => "wasm",
            Self::Process => "process",
        })
    }
}

fn required_string(
    object: &Map<String, Value>,
    key: &'static str,
) -> Result<String, ManifestError> {
    optional_string(object, key).ok_or(ManifestError::MissingField(key))
}

fn optional_string(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn number(object: &Map<String, Value>, key: &str) -> Option<u32> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn string_list(
    object: &Map<String, Value>,
    key: &'static str,
) -> Result<Vec<String>, ManifestError> {
    object
        .get(key)
        .map(|value| string_list_value(value, key))
        .transpose()
        .map(|value| value.unwrap_or_default())
}

fn string_list_value(value: &Value, key: &'static str) -> Result<Vec<String>, ManifestError> {
    let entries = value.as_array().ok_or(ManifestError::InvalidField(key))?;
    if entries.len() > MAX_LIST_ENTRIES {
        return Err(ManifestError::InvalidField(key));
    }
    entries
        .iter()
        .map(|entry| {
            let value = entry.as_str().ok_or(ManifestError::InvalidField(key))?;
            if value.is_empty() || value.len() > 256 || value.chars().any(char::is_whitespace) {
                return Err(ManifestError::InvalidField(key));
            }
            Ok(value.to_owned())
        })
        .collect()
}

fn json_list(object: &Map<String, Value>, key: &'static str) -> Result<Vec<Value>, ManifestError> {
    let Some(value) = object.get(key) else {
        return Ok(Vec::new());
    };
    let entries = value.as_array().ok_or(ManifestError::InvalidField(key))?;
    if entries.len() > MAX_LIST_ENTRIES
        || entries.iter().any(|entry| {
            serde_json::to_vec(entry)
                .map(|bytes| bytes.len() > MAX_DESCRIPTOR_BYTES)
                .unwrap_or(true)
        })
    {
        return Err(ManifestError::InvalidField(key));
    }
    Ok(entries.clone())
}

fn settings(object: &Map<String, Value>) -> Result<(Option<Value>, Option<Value>), ManifestError> {
    let Some(settings) = object.get("settings") else {
        return Ok((None, None));
    };
    let settings = settings
        .as_object()
        .ok_or(ManifestError::InvalidField("settings"))?;
    let schema = settings.get("schema").cloned();
    let ui_hints = settings.get("uiHints").cloned();
    if schema.as_ref().is_some_and(|value| !value.is_object())
        || ui_hints.as_ref().is_some_and(|value| !value.is_object())
    {
        return Err(ManifestError::InvalidField("settings"));
    }
    for value in [schema.as_ref(), ui_hints.as_ref()].into_iter().flatten() {
        if serde_json::to_vec(value)
            .map(|bytes| bytes.len() > MAX_DESCRIPTOR_BYTES)
            .unwrap_or(true)
        {
            return Err(ManifestError::InvalidField("settings"));
        }
    }
    Ok((schema, ui_hints))
}

/// Prefixes owned by the host. Plugins declare their own event types under
/// any other dotted name (for example hotkey.pressed or timer.tick).
const RESERVED_EVENT_PREFIXES: [&str; 3] = ["tiktok.", "points.", "plugin."];

const MAX_EVENT_TYPE_LEN: usize = 64;
const MAX_EVENT_FIELDS: usize = 64;
const MAX_EVENT_OPTIONS: usize = 128;

/// Validate manifest fields that affect host-side plugin action execution.
/// Descriptor payloads remain JSON so plugin-defined fields stay extensible.
pub fn validate_action_type(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    if let Some(timeout) = object.get("timeoutMs") {
        let timeout = timeout
            .as_u64()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if timeout == 0 || timeout > MAX_PLUGIN_ACTION_TIMEOUT_MS {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    Ok(())
}

/// Validate one eventTypes entry from a plugin manifest. Shape errors are
/// reported by the host catalog merge, which skips the entry with a warning.
pub fn validate_event_type(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let event_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    if !is_valid_event_type(event_type) {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    let title = object
        .get("title")
        .and_then(Value::as_object)
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let default = title
        .get("default")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if default.trim().is_empty() || default.len() > 120 {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    if let Some(fields) = object.get("fields") {
        let fields = fields
            .as_array()
            .ok_or(ManifestError::InvalidField("eventTypes"))?;
        if fields.len() > MAX_EVENT_FIELDS {
            return Err(ManifestError::InvalidField("eventTypes"));
        }
        for field in fields {
            validate_event_field(field)?;
        }
    }
    if object
        .get("sample")
        .is_some_and(|sample| !sample.is_object())
    {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    Ok(())
}

fn validate_event_field(field: &Value) -> Result<(), ManifestError> {
    let object = field
        .as_object()
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if path.trim().is_empty() || path.len() > 200 || path.chars().any(char::is_whitespace) {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    if let Some(kind) = object.get("kind") {
        let kind = kind.as_str().unwrap_or_default();
        if !matches!(kind, "text" | "number" | "boolean") {
            return Err(ManifestError::InvalidField("eventTypes"));
        }
    }
    if let Some(options) = object.get("options") {
        let options = options
            .as_array()
            .ok_or(ManifestError::InvalidField("eventTypes"))?;
        if options.len() > MAX_EVENT_OPTIONS {
            return Err(ManifestError::InvalidField("eventTypes"));
        }
        for option in options {
            validate_event_option(option)?;
        }
    }
    Ok(())
}

fn validate_event_option(option: &Value) -> Result<(), ManifestError> {
    let object = option
        .as_object()
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let value = object
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    // Empty values are legitimate ("none" options); only bound the length.
    if value.len() > 64 {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    if object.get("label").is_some_and(|label| !label.is_object()) {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    Ok(())
}

/// Typed `processorTypes` manifest entry. Processors enrich existing host
/// events before automation filters run; they are side-effect free and fail
/// open (see `ProcessorFailureMode`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginProcessorDescriptor {
    pub id: String,
    pub title: Value,
    #[serde(default)]
    pub description: Option<Value>,
    #[serde(default)]
    pub event_types: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<ProcessorInputDescriptor>,
    #[serde(default)]
    pub stage: ProcessorStage,
    #[serde(default)]
    pub failure_mode: ProcessorFailureMode,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// One event path a processor reads, with the semantic role the plugin
/// assigns it (for example `event.data.comment` as `message`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorInputDescriptor {
    pub path: String,
    pub role: String,
}

/// Lifecycle stage a processor runs in. Only pre-filter enrichment exists;
/// later stages are added deliberately, never speculatively.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessorStage {
    #[default]
    PreFilter,
}

/// Failure semantics for one processor call. Only pass-through exists: a
/// failing processor must never drop the host event.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessorFailureMode {
    #[default]
    PassThrough,
}

impl PluginProcessorDescriptor {
    /// Deadline for one processor call, clamped to the manifest bounds.
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(
            self.timeout_ms
                .unwrap_or(DEFAULT_PLUGIN_PROCESSOR_TIMEOUT_MS)
                .clamp(1, MAX_PLUGIN_PROCESSOR_TIMEOUT_MS),
        )
    }
}

const MAX_PROCESSOR_ID_LEN: usize = 64;
const MAX_PROCESSOR_EVENT_TYPES: usize = 32;
const MAX_PROCESSOR_INPUTS: usize = 16;
const MAX_PROCESSOR_PATH_LEN: usize = 200;
const MAX_PROCESSOR_ROLE_LEN: usize = 64;

/// Processor ids are plugin-scoped stable names (`textintel.analyze`), using
/// the same dotted-lowercase shape as event types but without reserved host
/// prefixes (processors subscribe to host events, they never publish types).
pub fn is_valid_processor_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=MAX_PROCESSOR_ID_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn is_processor_role(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=MAX_PROCESSOR_ROLE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

/// A subscribed event type uses the same dotted-lowercase shape as published
/// event types, but host namespaces (`tiktok.`, `points.`, `plugin.`) are
/// explicitly allowed: processors enrich host events, they do not declare
/// new trigger namespaces.
pub fn is_valid_processor_event_type(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=MAX_EVENT_TYPE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

/// Validate one `processorTypes` entry. Like event types, entries are
/// validated when the host builds its processor catalog; invalid entries are
/// skipped with a warning instead of failing discovery.
pub fn validate_processor_type(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("processorTypes"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("processorTypes"))?;
    if !is_valid_processor_id(id) {
        return Err(ManifestError::InvalidField("processorTypes"));
    }
    let title = object
        .get("title")
        .and_then(Value::as_object)
        .ok_or(ManifestError::InvalidField("processorTypes"))?;
    let default = title
        .get("default")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if default.trim().is_empty() || default.len() > 120 {
        return Err(ManifestError::InvalidField("processorTypes"));
    }
    if object
        .get("description")
        .is_some_and(|description| !description.is_object())
    {
        return Err(ManifestError::InvalidField("processorTypes"));
    }
    if let Some(event_types) = object.get("eventTypes") {
        let event_types = event_types
            .as_array()
            .ok_or(ManifestError::InvalidField("processorTypes"))?;
        if event_types.len() > MAX_PROCESSOR_EVENT_TYPES {
            return Err(ManifestError::InvalidField("processorTypes"));
        }
        for event_type in event_types {
            let event_type = event_type.as_str().unwrap_or_default();
            if !is_valid_processor_event_type(event_type) {
                return Err(ManifestError::InvalidField("processorTypes"));
            }
        }
    }
    if let Some(inputs) = object.get("inputs") {
        let inputs = inputs
            .as_array()
            .ok_or(ManifestError::InvalidField("processorTypes"))?;
        if inputs.len() > MAX_PROCESSOR_INPUTS {
            return Err(ManifestError::InvalidField("processorTypes"));
        }
        for input in inputs {
            validate_processor_input(input)?;
        }
    }
    if let Some(stage) = object.get("stage") {
        if stage.as_str() != Some("pre-filter") {
            return Err(ManifestError::InvalidField("processorTypes"));
        }
    }
    if let Some(failure_mode) = object.get("failureMode") {
        if failure_mode.as_str() != Some("pass-through") {
            return Err(ManifestError::InvalidField("processorTypes"));
        }
    }
    if let Some(timeout) = object.get("timeoutMs") {
        let timeout = timeout
            .as_u64()
            .ok_or(ManifestError::InvalidField("processorTypes"))?;
        if timeout == 0 || timeout > MAX_PLUGIN_PROCESSOR_TIMEOUT_MS {
            return Err(ManifestError::InvalidField("processorTypes"));
        }
    }
    Ok(())
}

fn validate_processor_input(input: &Value) -> Result<(), ManifestError> {
    let object = input
        .as_object()
        .ok_or(ManifestError::InvalidField("processorTypes"))?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if path.trim().is_empty()
        || path.len() > MAX_PROCESSOR_PATH_LEN
        || path.chars().any(char::is_whitespace)
    {
        return Err(ManifestError::InvalidField("processorTypes"));
    }
    let role = object
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !is_processor_role(role) {
        return Err(ManifestError::InvalidField("processorTypes"));
    }
    Ok(())
}

/// Event type names are dotted lowercase: hotkey.pressed, timer.tick.
/// Host namespaces stay reserved so a plugin can never shadow built-in
/// triggers or the internal plugin.emit channel.
pub fn is_valid_event_type(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=MAX_EVENT_TYPE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
        && !RESERVED_EVENT_PREFIXES
            .iter()
            .any(|prefix| value.starts_with(prefix))
}

pub fn is_valid_plugin_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=128).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[1..].iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

/// Validate a package-relative path before joining it with a plugin root.
pub fn is_safe_relative_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\0') || value.starts_with('/') || value.starts_with('\\')
    {
        return false;
    }
    let normalized = value.replace('\\', "/");
    if normalized.starts_with('/') || normalized.contains(":/") {
        return false;
    }
    let parts: Vec<&str> = normalized.split('/').collect();
    !parts.is_empty()
        && normalized != "."
        && parts.iter().all(|part| !part.is_empty() && *part != "..")
}

pub fn current_platform() -> String {
    match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        platform => platform,
    }
    .to_owned()
}

pub fn current_target() -> String {
    let platform = current_platform();
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        architecture => architecture,
    };
    let abi = match std::env::consts::OS {
        "windows" => "msvc",
        "linux" => "gnu",
        "macos" => "darwin",
        other => other,
    };
    format!("{platform}-{arch}-{abi}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_new_native_manifest_and_descriptors() {
        let manifest = PluginManifest::from_json_str(
            r#"{
                "schemaVersion": 2,
                "id": "miniaudio",
                "name": "MiniAudio",
                "version": "1.2.0",
                "runtime": "native",
                "entry": "native/miniaudio.dll",
                "actionTypes": [{"id":"audio.play"}],
                "settings": {"schema": {"type":"object"}},
                "permissions": ["audio"]
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.runtime, PluginRuntimeKind::Native);
        assert_eq!(manifest.entry, "native/miniaudio.dll");
        assert_eq!(manifest.action_types.len(), 1);
        assert!(manifest.settings_schema.is_some());
    }

    #[test]
    fn rejects_removed_schema_and_unsafe_entries() {
        assert!(matches!(
            PluginManifest::from_json_str(
                r#"{"schemaVersion":1,"id":"demo","name":"Demo","version":"1.0.0","main":"index.js"}"#
            ),
            Err(ManifestError::UnsupportedSchema(1))
        ));
        assert!(matches!(
            PluginManifest::from_json_str(
                r#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"../index"}"#
            ),
            Err(ManifestError::UnsafeEntry)
        ));
    }

    #[test]
    fn reads_event_types_and_validates_them() {
        let manifest = PluginManifest::from_json_str(
            r#"{"schemaVersion": 2, "id": "hotkeys", "name": "Hotkeys", "version": "1.0.0", "runtime": "process", "entry": "hotkeys", "capabilities": ["events.publish"], "eventTypes": [{"type": "hotkey.pressed", "title": {"default": "Hotkey pressed"}, "fields": [{"path": "event.data.key", "kind": "text"}], "sample": {"key": "ctrl+k"}}]}"#,
        )
        .unwrap();
        assert_eq!(manifest.event_types.len(), 1);
        assert!(validate_event_type(&manifest.event_types[0]).is_ok());

        // Fixed field options validate; empty values and bad labels do not.
        assert!(validate_event_type(&serde_json::json!({
            "type": "hotkey.pressed",
            "title": {"default": "Hotkey pressed"},
            "fields": [{"path": "event.data.key", "options": [{"value": "k"}, {"value": "space", "label": {"default": "Space"}}]}]
        }))
        .is_ok());
        assert!(validate_event_type(&serde_json::json!({
            "type": "hotkey.pressed",
            "title": {"default": "Hotkey pressed"},
            "fields": [{"path": "event.data.key", "options": [{"value": ""}, {"value": "k", "label": "oops"}]}]
        }))
        .is_err());

        // Reserved host namespaces can never be shadowed.
        for reserved in [
            "tiktok.chat",
            "points.awarded",
            "plugin.emit",
            "plugin.custom",
        ] {
            assert!(
                !is_valid_event_type(reserved),
                "{reserved} should be reserved"
            );
        }
        assert!(is_valid_event_type("hotkey.pressed"));
        assert!(is_valid_event_type("dom.match"));
        assert!(!is_valid_event_type("SHOUTY"));
        assert!(!is_valid_event_type("x"));

        // Missing title default is rejected.
        assert!(validate_event_type(&serde_json::json!({"type": "hotkey.pressed"})).is_err());
        assert!(validate_event_type(&serde_json::json!({"type": "hotkey.pressed", "title": {"default": "Hotkey pressed"}, "fields": [{"path": "event.data.key", "kind": "image"}]})).is_err());
    }

    #[test]
    fn omitted_trust_preserves_schema_v2_defaults() {
        let process = PluginManifest::from_json_str(
            r#"{"schemaVersion":2,"id":"process","name":"Process","version":"1.0.0","runtime":"process","entry":"plugin.exe"}"#,
        )
        .unwrap();
        let wasm = PluginManifest::from_json_str(
            r#"{"schemaVersion":2,"id":"wasm","name":"WASM","version":"1.0.0","runtime":"wasm","entry":"plugin.wasm"}"#,
        )
        .unwrap();
        assert_eq!(process.trust, PluginTrust::Sandboxed);
        assert_eq!(wasm.trust, PluginTrust::Sandboxed);
        assert_eq!(process.security_model(), PluginSecurityModel::Isolated);
        assert_eq!(wasm.security_model(), PluginSecurityModel::Sandboxed);
        assert_eq!(
            PluginRuntimeKind::Native.security_model(),
            PluginSecurityModel::Trusted
        );
    }

    #[test]
    fn current_target_is_platform_qualified() {
        assert!(current_target().starts_with(&format!("{}-", current_platform())));
    }

    #[test]
    fn rejects_traversal() {
        assert!(!is_safe_relative_path("../../secret"));
        assert!(!is_safe_relative_path("native/../secret"));
        assert!(is_safe_relative_path("native/plugin.dll"));
    }

    #[test]
    fn reads_processor_types_and_validates_them() {
        let manifest = PluginManifest::from_json_str(
            r#"{"schemaVersion": 2, "id": "textintel", "name": "Text Intelligence", "version": "0.1.0", "runtime": "process", "entry": "tiktools-textintel", "capabilities": ["events.enrich"], "processorTypes": [{"id": "textintel.analyze", "title": {"default": "Text Intelligence"}, "description": {"default": "Analyze comments."}, "eventTypes": ["tiktok.chat"], "stage": "pre-filter", "timeoutMs": 75, "failureMode": "pass-through", "inputs": [{"path": "event.data.comment", "role": "message"}, {"path": "event.user.nickname", "role": "display-name"}]}]}"#,
        )
        .unwrap();
        assert_eq!(manifest.processor_types.len(), 1);
        assert!(validate_processor_type(&manifest.processor_types[0]).is_ok());

        // Minimal descriptors validate; stage/failure mode default.
        let minimal = serde_json::json!({
            "id": "textintel.analyze",
            "title": {"default": "Text Intelligence"}
        });
        assert!(validate_processor_type(&minimal).is_ok());
        let typed: PluginProcessorDescriptor = serde_json::from_value(minimal).unwrap();
        assert_eq!(typed.stage, ProcessorStage::PreFilter);
        assert_eq!(typed.failure_mode, ProcessorFailureMode::PassThrough);
        assert_eq!(
            typed.timeout(),
            std::time::Duration::from_millis(DEFAULT_PLUGIN_PROCESSOR_TIMEOUT_MS)
        );

        // Subscribing to host event namespaces is allowed for processors.
        assert!(is_valid_processor_event_type("tiktok.chat"));
        assert!(is_valid_processor_event_type("points.awarded"));
        assert!(is_valid_processor_event_type("plugin.emit"));
        assert!(is_valid_processor_event_type("hotkey.pressed"));
        assert!(!is_valid_processor_event_type("SHOUTY"));
        assert!(!is_valid_processor_event_type("x"));

        // Malformed descriptors are rejected.
        for entry in [
            serde_json::json!({"title": {"default": "Missing id"}}),
            serde_json::json!({"id": "SHOUTY", "title": {"default": "Bad id"}}),
            serde_json::json!({"id": "x", "title": {"default": "Short id"}}),
            serde_json::json!({"id": "ok.id"}),
            serde_json::json!({"id": "ok.id", "title": {"default": ""}}),
            serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "description": "nope"}),
            serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "eventTypes": ["SHOUTY"]}),
            serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "stage": "post-filter"}),
            serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "failureMode": "drop"}),
            serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "inputs": [{"path": "", "role": "message"}]}),
            serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "inputs": [{"path": "event.data.comment", "role": "Bad Role"}]}),
            serde_json::json!({"id": "ok.id", "title": {"default": "Ok"}, "inputs": [{"path": "event.data.comment"}]}),
        ] {
            assert!(
                validate_processor_type(&entry).is_err(),
                "should reject {entry}"
            );
        }
    }

    #[test]
    fn manifests_without_processor_types_still_work() {
        let manifest = PluginManifest::from_json_str(
            r#"{"schemaVersion": 2, "id": "legacy", "name": "Legacy", "version": "1.0.0", "runtime": "process", "entry": "legacy"}"#,
        )
        .unwrap();
        assert!(manifest.processor_types.is_empty());
        assert!(manifest.validate_compatibility().is_ok());
    }

    #[test]
    fn validates_processor_collection_and_timeout_bounds() {
        let manifest = |descriptor: &str| {
            format!(
                r#"{{"schemaVersion":2,"id":"proc","name":"Proc","version":"1.0.0","runtime":"process","entry":"proc","processorTypes":[{descriptor}]}}"#
            )
        };
        // Discovery keeps raw entries (validated at catalog build), but the
        // entry validator enforces collection and timeout bounds.
        let many_inputs = (0..17)
            .map(|index| format!(r#"{{"path": "event.data.field{index}", "role": "message"}}"#))
            .collect::<Vec<_>>()
            .join(",");
        assert!(validate_processor_type(&serde_json::json!({
            "id": "proc.many",
            "title": {"default": "Too many inputs"},
            "inputs": serde_json::from_str::<Value>(&format!("[{many_inputs}]")).unwrap()
        }))
        .is_err());
        let many_events = (0..33)
            .map(|index| format!(r#""custom.event{index}""#))
            .collect::<Vec<_>>()
            .join(",");
        assert!(validate_processor_type(&serde_json::json!({
            "id": "proc.events",
            "title": {"default": "Too many events"},
            "eventTypes": serde_json::from_str::<Value>(&format!("[{many_events}]")).unwrap()
        }))
        .is_err());

        let valid = PluginManifest::from_json_str(&manifest(
            r#"{"id":"proc.ok","title":{"default":"Ok"},"timeoutMs":75}"#,
        ))
        .unwrap();
        assert!(validate_processor_type(&valid.processor_types[0]).is_ok());
        for invalid in ["0", "5001", "-1", "\"75\""] {
            let manifest = PluginManifest::from_json_str(&manifest(&format!(
                r#"{{"id":"proc.ok","title":{{"default":"Ok"}},"timeoutMs":{invalid}}}"#
            )))
            .unwrap();
            assert!(
                validate_processor_type(&manifest.processor_types[0]).is_err(),
                "timeout {invalid} should be rejected"
            );
        }
    }

    #[test]
    fn validates_optional_action_timeout_bounds() {
        let manifest = |timeout: &str| {
            format!(
                r#"{{"schemaVersion":2,"id":"timeout","name":"Timeout","version":"1.0.0","runtime":"process","entry":"plugin.exe","actionTypes":[{{"id":"timeout.action","timeoutMs":{timeout}}}]}}"#
            )
        };

        assert!(PluginManifest::from_json_str(&manifest("120000")).is_ok());
        assert!(PluginManifest::from_json_str(&manifest("180000")).is_ok());
        for invalid in ["0", "180001", "-1", "\"120000\""] {
            assert!(
                PluginManifest::from_json_str(&manifest(invalid)).is_err(),
                "timeout {invalid} should be rejected"
            );
        }
    }
}
