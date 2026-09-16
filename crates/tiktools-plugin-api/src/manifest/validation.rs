use serde_json::Value;
use thiserror::Error;

use super::types::MAX_PLUGIN_ACTION_TIMEOUT_MS;

pub(crate) const PLUGIN_SCHEMA_VERSION: u32 = 2;
pub(crate) const MAX_MANIFEST_BYTES: usize = 256 * 1024;
pub(crate) const MAX_LIST_ENTRIES: usize = 128;
pub(crate) const MAX_DESCRIPTOR_BYTES: usize = 64 * 1024;
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
/// Prefixes owned by the host. Plugins declare their own event types under
/// any other dotted name (for example hotkey.pressed or timer.tick).
const RESERVED_EVENT_PREFIXES: [&str; 3] = ["tiktok.", "points.", "plugin."];

pub(crate) const MAX_EVENT_TYPE_LEN: usize = 64;
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
