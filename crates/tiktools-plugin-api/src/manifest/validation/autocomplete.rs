//! Ownership and shape validation for plugin enrichment suggestions. Only stable enrichment
//! paths and the declaring plugin subtree can be claimed.

use super::fields::{is_valid_plugin_id, is_valid_trigger_name, validate_localized};
use super::{
    ManifestError, MAX_AUTOCOMPLETE_FIELDS, MAX_AUTOCOMPLETE_PATHS, MAX_AUTOCOMPLETE_PATH_LEN,
    MAX_AUTOCOMPLETE_PREFIXES, MAX_AUTOCOMPLETE_TRIGGERS,
};
use serde_json::Value;

/// Validate one `autocomplete` entry from a plugin manifest. The host calls
/// this with the manifest's own id when it merges the behavior snapshot and
/// skips invalid entries with a warning, like event types and templates.
///
/// A contribution may only claim enrichment paths: the host-defined stable
/// keys (`event.intel.comment`, `event.intel.user`) plus the plugin's own
/// provider subtree (`event.intel.providers.<ownId>`). Core paths
/// (`event.data.*`, `event.user.*`), the host-stamped
/// `event.intel.processing.*`, and other plugins' subtrees are rejected, so
/// a manifest can never hide suggestions it does not own.
pub fn validate_plugin_autocomplete(plugin_id: &str, entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("autocomplete"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("autocomplete"))?;
    if !is_valid_plugin_id(id) {
        return Err(ManifestError::InvalidField("autocomplete"));
    }
    let prefixes = validate_autocomplete_paths(object.get("prefixes"), MAX_AUTOCOMPLETE_PREFIXES)?;
    let paths = validate_autocomplete_paths(object.get("paths"), MAX_AUTOCOMPLETE_PATHS)?;
    let fields = validate_autocomplete_fields(object.get("fields"))?;
    if prefixes.is_empty() && paths.is_empty() && fields.is_empty() {
        return Err(ManifestError::InvalidField("autocomplete"));
    }
    for path in prefixes.iter().chain(paths.iter()).chain(fields.iter()) {
        if !is_claimable_autocomplete_path(plugin_id, path) {
            return Err(ManifestError::InvalidField("autocomplete"));
        }
    }
    if let Some(triggers) = object.get("triggers") {
        let triggers = triggers
            .as_array()
            .ok_or(ManifestError::InvalidField("autocomplete"))?;
        if triggers.len() > MAX_AUTOCOMPLETE_TRIGGERS {
            return Err(ManifestError::InvalidField("autocomplete"));
        }
        for trigger in triggers {
            let trigger = trigger
                .as_str()
                .ok_or(ManifestError::InvalidField("autocomplete"))?;
            // Triggers name host event types (tiktok.chat), so only the
            // dotted syntax is checked; reserved namespaces stay allowed
            // here because contributions scope *to* host triggers.
            if !is_valid_trigger_name(trigger) {
                return Err(ManifestError::InvalidField("autocomplete"));
            }
        }
    }
    Ok(())
}

fn validate_autocomplete_paths(
    value: Option<&Value>,
    max: usize,
) -> Result<Vec<String>, ManifestError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let entries = value
        .as_array()
        .ok_or(ManifestError::InvalidField("autocomplete"))?;
    if entries.len() > max {
        return Err(ManifestError::InvalidField("autocomplete"));
    }
    entries
        .iter()
        .map(|entry| {
            let path = entry
                .as_str()
                .ok_or(ManifestError::InvalidField("autocomplete"))?;
            let trimmed = path.trim();
            if trimmed.is_empty()
                || trimmed.len() > MAX_AUTOCOMPLETE_PATH_LEN
                || trimmed.chars().any(char::is_whitespace)
            {
                return Err(ManifestError::InvalidField("autocomplete"));
            }
            Ok(trimmed.to_owned())
        })
        .collect()
}

fn validate_autocomplete_fields(value: Option<&Value>) -> Result<Vec<String>, ManifestError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let entries = value
        .as_array()
        .ok_or(ManifestError::InvalidField("autocomplete"))?;
    if entries.len() > MAX_AUTOCOMPLETE_FIELDS {
        return Err(ManifestError::InvalidField("autocomplete"));
    }
    let mut paths = Vec::with_capacity(entries.len());
    for entry in entries {
        let object = entry
            .as_object()
            .ok_or(ManifestError::InvalidField("autocomplete"))?;
        let path = object
            .get("path")
            .and_then(Value::as_str)
            .ok_or(ManifestError::InvalidField("autocomplete"))?;
        let trimmed = path.trim();
        if trimmed.is_empty()
            || trimmed.len() > MAX_AUTOCOMPLETE_PATH_LEN
            || trimmed.ends_with('.')
            || trimmed.chars().any(char::is_whitespace)
        {
            return Err(ManifestError::InvalidField("autocomplete"));
        }
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .ok_or(ManifestError::InvalidField("autocomplete"))?;
        if !matches!(kind, "string" | "number" | "boolean") {
            return Err(ManifestError::InvalidField("autocomplete"));
        }
        validate_localized(object.get("label"))
            .map_err(|_| ManifestError::InvalidField("autocomplete"))?;
        if object.get("hint").is_some() {
            validate_localized(object.get("hint"))
                .map_err(|_| ManifestError::InvalidField("autocomplete"))?;
        }
        paths.push(trimmed.to_owned());
    }
    Ok(paths)
}

/// Stable enrichment keys the host promotes from processor output. Plugins
/// claim these shared prefixes (first-writer-wins at merge time); everything
/// else claimable lives under the plugin's own provider subtree.
const STABLE_INTEL_PREFIXES: [&str; 2] = ["event.intel.comment", "event.intel.user"];

fn is_claimable_autocomplete_path(plugin_id: &str, path: &str) -> bool {
    let candidate = path.strip_suffix('.').unwrap_or(path);
    if candidate.is_empty() {
        return false;
    }
    let provider_root = format!("event.intel.providers.{plugin_id}");
    if candidate == provider_root || candidate.starts_with(&format!("{provider_root}.")) {
        return true;
    }
    STABLE_INTEL_PREFIXES
        .iter()
        .any(|stable| candidate == *stable || candidate.starts_with(&format!("{stable}.")))
}
