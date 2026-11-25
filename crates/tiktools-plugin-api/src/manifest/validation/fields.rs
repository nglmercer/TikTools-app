//! Shared lexical fields, localized text and package-relative path checks. Callers retain
//! their domain-specific error mapping.

use super::MAX_EVENT_TYPE_LEN;
use serde_json::Value;

pub(super) fn is_header_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub(super) fn validate_localized(value: Option<&Value>) -> Result<(), &'static str> {
    let object = value
        .and_then(Value::as_object)
        .ok_or("localized text must be an object")?;
    let default = object
        .get("default")
        .and_then(Value::as_str)
        .ok_or("localized text needs a default")?;
    if default.trim().is_empty() || default.len() > 1_024 {
        return Err("localized default out of range");
    }
    if let Some(key) = object.get("i18key") {
        let key = key.as_str().ok_or("i18key must be a string")?;
        if key.trim().is_empty() || key.len() > 256 {
            return Err("i18key out of range");
        }
    }
    Ok(())
}

pub(super) fn is_valid_trigger_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=MAX_EVENT_TYPE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
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

/// Validate one `cliCommands` entry. The command must reference an action type
/// declared by the same manifest: invocation stays host-owned and ownership is
/// always checked against the host catalog.
pub fn validate_cli_command(
    value: &Value,
    action_types: &[Value],
) -> Result<(), super::ManifestError> {
    let object = value
        .as_object()
        .ok_or(super::ManifestError::InvalidField("cliCommands"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or(super::ManifestError::InvalidField("cliCommands"))?;
    let valid_id = !id.is_empty()
        && id.len() <= 64
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        });
    if !valid_id {
        return Err(super::ManifestError::InvalidField("cliCommands"));
    }
    let action_type = object
        .get("actionType")
        .and_then(Value::as_str)
        .ok_or(super::ManifestError::InvalidField("cliCommands"))?;
    if action_type.is_empty()
        || action_type.len() > 128
        || !action_types.iter().any(|declared| {
            declared
                .as_object()
                .and_then(|object| object.get("id"))
                .and_then(Value::as_str)
                == Some(action_type)
        })
    {
        return Err(super::ManifestError::InvalidField("cliCommands"));
    }
    let description = object
        .get("description")
        .and_then(Value::as_str)
        .ok_or(super::ManifestError::InvalidField("cliCommands"))?;
    if description.is_empty() || description.len() > 256 {
        return Err(super::ManifestError::InvalidField("cliCommands"));
    }
    if let Some(alias) = object.get("alias").and_then(Value::as_str) {
        if alias.is_empty()
            || alias.len() > 64
            || alias
                .bytes()
                .any(|byte| !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'))
        {
            return Err(super::ManifestError::InvalidField("cliCommands"));
        }
    }
    Ok(())
}

/// Validate one `capabilityProviders` entry. The capability must be declared by
/// the same manifest and the action type must exist in its `actionTypes`, so a
/// provider can never route to an action the plugin did not declare.
pub fn validate_capability_provider(
    value: &Value,
    capabilities: &[String],
    action_types: &[Value],
) -> Result<(), super::ManifestError> {
    let object = value
        .as_object()
        .ok_or(super::ManifestError::InvalidField("capabilityProviders"))?;
    let capability = object
        .get("capability")
        .and_then(Value::as_str)
        .ok_or(super::ManifestError::InvalidField("capabilityProviders"))?;
    if !capabilities.iter().any(|declared| declared == capability) {
        return Err(super::ManifestError::InvalidField("capabilityProviders"));
    }
    let action_type = object
        .get("actionType")
        .and_then(Value::as_str)
        .ok_or(super::ManifestError::InvalidField("capabilityProviders"))?;
    let declared = action_types.iter().any(|declared| {
        declared
            .as_object()
            .and_then(|object| object.get("id"))
            .and_then(Value::as_str)
            == Some(action_type)
    });
    if !declared {
        return Err(super::ManifestError::InvalidField("capabilityProviders"));
    }
    Ok(())
}

/// Locale tags are BCP-47-ish lowercase tokens (`en`, `pt-br`): bounded and
/// ASCII-only because they double as object keys in the snapshot catalog.
fn is_valid_locale_tag(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=16).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-'))
}

/// Translation message keys follow the same lexical rule as i18key references,
/// plus the hard namespacing requirement: every key starts with `<pluginId>.`
/// so one plugin can never overwrite host or other-plugin messages.
fn is_valid_translation_key(key: &str, plugin_id: &str) -> bool {
    let Some(suffix) = key.strip_prefix(plugin_id).and_then(|rest| rest.strip_prefix('.')) else {
        return false;
    };
    !suffix.is_empty()
        && key.len() <= 256
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// Parse and validate the additive `translations` block: `{ locale: { key: text } }`.
/// Fail-closed on any malformed entry — a plugin that cannot spell its own
/// catalog is discovery-broken, not partially translated.
pub(crate) fn parse_translations(
    value: Option<&Value>,
    plugin_id: &str,
) -> Result<
    std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    super::ManifestError,
> {
    const MAX_LOCALES: usize = 16;
    const MAX_MESSAGES_PER_LOCALE: usize = 256;
    const MAX_TOTAL_MESSAGES: usize = 1_024;
    const MAX_MESSAGE_LEN: usize = 1_024;
    let mut catalog = std::collections::BTreeMap::new();
    let Some(value) = value else {
        return Ok(catalog);
    };
    let object = value
        .as_object()
        .ok_or(super::ManifestError::InvalidField("translations"))?;
    if object.len() > MAX_LOCALES {
        return Err(super::ManifestError::InvalidField("translations"));
    }
    let mut total = 0usize;
    for (locale, messages) in object {
        if !is_valid_locale_tag(locale) {
            return Err(super::ManifestError::InvalidField("translations"));
        }
        let messages = messages
            .as_object()
            .ok_or(super::ManifestError::InvalidField("translations"))?;
        if messages.is_empty() || messages.len() > MAX_MESSAGES_PER_LOCALE {
            return Err(super::ManifestError::InvalidField("translations"));
        }
        total += messages.len();
        if total > MAX_TOTAL_MESSAGES {
            return Err(super::ManifestError::InvalidField("translations"));
        }
        let mut entries = std::collections::BTreeMap::new();
        for (key, text) in messages {
            let valid = is_valid_translation_key(key, plugin_id)
                && text
                    .as_str()
                    .is_some_and(|text| !text.is_empty() && text.len() <= MAX_MESSAGE_LEN);
            if !valid {
                return Err(super::ManifestError::InvalidField("translations"));
            }
            entries.insert(key.clone(), text.as_str().expect("checked above").to_owned());
        }
        catalog.insert(locale.clone(), entries);
    }
    Ok(catalog)
}
