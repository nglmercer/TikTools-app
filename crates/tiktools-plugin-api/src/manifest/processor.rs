use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    types::{DEFAULT_PLUGIN_PROCESSOR_TIMEOUT_MS, MAX_PLUGIN_PROCESSOR_TIMEOUT_MS},
    validation::{ManifestError, MAX_EVENT_TYPE_LEN},
};

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
    /// Stable-field selection priority. Higher runs first; the first
    /// contributor owns each stable `intel` key outright instead of merging
    /// per-leaf with other providers. Ties break by `(plugin id,
    /// processor id)`.
    #[serde(default)]
    pub priority: Option<i32>,
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

    /// Selection priority, defaulting to zero when undeclared.
    pub fn priority_value(&self) -> i32 {
        self.priority.unwrap_or(0)
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
        let mut roles = std::collections::BTreeSet::new();
        for input in inputs {
            validate_processor_input(input)?;
            let role = input
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !roles.insert(role.to_owned()) {
                return Err(ManifestError::InvalidField("processorTypes"));
            }
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
    if let Some(priority) = object.get("priority") {
        let priority = priority
            .as_i64()
            .ok_or(ManifestError::InvalidField("processorTypes"))?;
        if priority < i64::from(i32::MIN) || priority > i64::from(i32::MAX) {
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
