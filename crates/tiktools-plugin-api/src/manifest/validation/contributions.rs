//! Host-rendered page sections and workflow templates declared by plugins.

use super::fields::{
    is_header_token, is_valid_plugin_id, is_valid_trigger_name, validate_localized,
};
use super::{ManifestError, MAX_PAGE_SECTIONS, MAX_TEMPLATE_NODES};
use serde_json::Value;

/// Validate one templates entry from a v3 manifest. Like event types, shape
/// errors are reported by the host catalog merge, which skips the entry.
pub fn validate_plugin_template(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("templates"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("templates"))?;
    if !is_valid_plugin_id(id) {
        return Err(ManifestError::InvalidField("templates"));
    }
    validate_localized(object.get("title"))
        .map_err(|_| ManifestError::InvalidField("templates"))?;
    if let Some(description) = object.get("description") {
        validate_localized(Some(description))
            .map_err(|_| ManifestError::InvalidField("templates"))?;
    }
    if let Some(icon) = object.get("icon") {
        let icon = icon
            .as_str()
            .ok_or(ManifestError::InvalidField("templates"))?;
        if icon.trim().is_empty() || icon.len() > 64 || !is_header_token(icon) {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    let event_type = object
        .get("eventType")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("templates"))?;
    // Templates target host triggers (tiktok.chat), so only the dotted
    // syntax is checked here; host namespaces stay reserved for plugins that
    // declare their own event types.
    if !is_valid_trigger_name(event_type) {
        return Err(ManifestError::InvalidField("templates"));
    }
    let required = object
        .get("requiredNodeTypes")
        .and_then(Value::as_array)
        .ok_or(ManifestError::InvalidField("templates"))?;
    if required.is_empty() || required.len() > MAX_TEMPLATE_NODES {
        return Err(ManifestError::InvalidField("templates"));
    }
    for node_type in required {
        let node_type = node_type
            .as_str()
            .ok_or(ManifestError::InvalidField("templates"))?;
        if !is_valid_trigger_name(node_type) {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    if let Some(category) = object.get("category") {
        let category = category
            .as_str()
            .ok_or(ManifestError::InvalidField("templates"))?;
        if category.trim().is_empty() || category.len() > 64 {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    if let Some(params) = object.get("params") {
        if !params.is_object() {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    if let Some(ui_hints) = object.get("uiHints") {
        if !ui_hints.is_object() {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    let workflow = object
        .get("workflow")
        .and_then(Value::as_object)
        .ok_or(ManifestError::InvalidField("templates"))?;
    let nodes = workflow
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or(ManifestError::InvalidField("templates"))?;
    if nodes.is_empty() || nodes.len() > MAX_TEMPLATE_NODES {
        return Err(ManifestError::InvalidField("templates"));
    }
    for node in nodes {
        let node = node
            .as_object()
            .ok_or(ManifestError::InvalidField("templates"))?;
        let node_type = node
            .get("type")
            .and_then(Value::as_str)
            .ok_or(ManifestError::InvalidField("templates"))?;
        if !is_valid_trigger_name(node_type) {
            return Err(ManifestError::InvalidField("templates"));
        }
        if let Some(config) = node.get("config") {
            if !config.is_object() {
                return Err(ManifestError::InvalidField("templates"));
            }
        }
    }
    Ok(())
}

/// Validate one pages entry from a v3 manifest. Section kinds form a fixed,
/// host-rendered widget set: unknown kinds are rejected so a manifest can
/// never smuggle arbitrary markup or script into the WebView.
pub fn validate_plugin_page(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("pages"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("pages"))?;
    if !is_valid_plugin_id(id) {
        return Err(ManifestError::InvalidField("pages"));
    }
    validate_localized(object.get("title")).map_err(|_| ManifestError::InvalidField("pages"))?;
    if let Some(icon) = object.get("icon") {
        let icon = icon.as_str().ok_or(ManifestError::InvalidField("pages"))?;
        if icon.trim().is_empty() || icon.len() > 64 || !is_header_token(icon) {
            return Err(ManifestError::InvalidField("pages"));
        }
    }
    let sections = object
        .get("sections")
        .and_then(Value::as_array)
        .ok_or(ManifestError::InvalidField("pages"))?;
    if sections.is_empty() || sections.len() > MAX_PAGE_SECTIONS {
        return Err(ManifestError::InvalidField("pages"));
    }
    for section in sections {
        validate_page_section(section)?;
    }
    Ok(())
}

fn validate_page_section(section: &Value) -> Result<(), ManifestError> {
    let object = section
        .as_object()
        .ok_or(ManifestError::InvalidField("pages"))?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("pages"))?;
    if let Some(title) = object.get("title") {
        validate_localized(Some(title)).map_err(|_| ManifestError::InvalidField("pages"))?;
    }
    match kind {
        "text" => {
            let text = object
                .get("text")
                .ok_or(ManifestError::InvalidField("pages"))?;
            validate_localized(Some(text)).map_err(|_| ManifestError::InvalidField("pages"))?;
            Ok(())
        }
        "form" => {
            if object
                .get("schema")
                .is_some_and(|schema| !schema.is_object())
            {
                return Err(ManifestError::InvalidField("pages"));
            }
            if object
                .get("uiHints")
                .is_some_and(|hints| !hints.is_object())
            {
                return Err(ManifestError::InvalidField("pages"));
            }
            Ok(())
        }
        "connection" => Ok(()),
        "list" => {
            let source = object
                .get("optionsFrom")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("pages"))?;
            if source.trim().is_empty() || source.len() > 256 {
                return Err(ManifestError::InvalidField("pages"));
            }
            Ok(())
        }
        "tts" => {
            let action_type = object
                .get("actionType")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("pages"))?;
            if action_type.trim().is_empty() || action_type.len() > 128 {
                return Err(ManifestError::InvalidField("pages"));
            }
            let voices_from = object
                .get("voicesFrom")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("pages"))?;
            if voices_from.trim().is_empty() || voices_from.len() > 256 {
                return Err(ManifestError::InvalidField("pages"));
            }
            // Server-side audio outputs are optional: older manifests omit
            // the selector, and the host hides it when the source is absent.
            if let Some(outputs_from) = object.get("outputsFrom") {
                let outputs_from = outputs_from
                    .as_str()
                    .ok_or(ManifestError::InvalidField("pages"))?;
                if outputs_from.trim().is_empty() || outputs_from.len() > 256 {
                    return Err(ManifestError::InvalidField("pages"));
                }
            }
            Ok(())
        }
        _ => Err(ManifestError::InvalidField("pages")),
    }
}
