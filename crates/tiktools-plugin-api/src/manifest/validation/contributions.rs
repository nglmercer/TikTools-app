//! Host-rendered page sections and workflow templates declared by plugins.

use super::fields::{
    is_header_token, is_valid_plugin_id, validate_localized,
};
use super::{ManifestError, MAX_PAGE_SECTIONS};
use serde_json::Value;

/// Validate one templates entry from a v3 manifest. Templates are rule
/// documents — the same flat `{ actions, event, params }` grammar the
/// automation editor instantiates — validated here so anything the host
/// merges into the snapshot also parses in every client. Like event types,
/// shape errors are reported by the host catalog merge, which skips the entry.
pub fn validate_plugin_template(entry: &Value) -> Result<(), ManifestError> {
    const MAX_ACTIONS: usize = 8;
    const MAX_FILTERS: usize = 12;
    const MAX_FILTER_VALUE_LEN: usize = 500;
    let invalid = || ManifestError::InvalidField("templates");
    let object = entry.as_object().ok_or_else(invalid)?;
    if object.get("templateVersion").and_then(Value::as_u64) != Some(1) {
        return Err(invalid());
    }
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    if !is_valid_plugin_id(id) {
        return Err(invalid());
    }
    validate_localized(object.get("title")).map_err(|_| invalid())?;
    if let Some(description) = object.get("description") {
        validate_localized(Some(description)).map_err(|_| invalid())?;
    }
    if let Some(icon) = object.get("icon") {
        let icon = icon.as_str().ok_or_else(invalid)?;
        if icon.trim().is_empty() || icon.len() > 64 || !is_header_token(icon) {
            return Err(invalid());
        }
    }
    if object.get("params").is_some_and(|params| !params.is_object())
        || object.get("uiHints").is_some_and(|hints| !hints.is_object())
    {
        return Err(invalid());
    }
    let actions = object
        .get("actions")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if actions.is_empty() || actions.len() > MAX_ACTIONS {
        return Err(invalid());
    }
    for action in actions {
        let action = action.as_object().ok_or_else(invalid)?;
        let valid_name = action
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| !name.trim().is_empty() && name.len() <= 80);
        let valid_type = action
            .get("typeId")
            .and_then(Value::as_str)
            .is_some_and(|type_id| !type_id.trim().is_empty() && type_id.len() <= 128);
        if !valid_name || !valid_type || !action.get("config").is_some_and(Value::is_object) {
            return Err(invalid());
        }
    }
    let event = object
        .get("event")
        .and_then(Value::as_object)
        .ok_or_else(invalid)?;
    let valid_event_name = event
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| !name.trim().is_empty() && name.len() <= 80);
    let valid_trigger = event
        .get("trigger")
        .and_then(Value::as_str)
        .is_some_and(|trigger| !trigger.trim().is_empty() && trigger.len() <= 128);
    if !valid_event_name || !valid_trigger {
        return Err(invalid());
    }
    if let Some(filters) = event.get("filters") {
        let filters = filters.as_array().ok_or_else(invalid)?;
        if filters.len() > MAX_FILTERS {
            return Err(invalid());
        }
        for filter in filters {
            validate_template_filter(filter, MAX_FILTER_VALUE_LEN)?;
        }
    }
    Ok(())
}

/// One event filter inside a template: the same operators and alias rules
/// the editor's bounded instantiation understands. Anything else fails
/// closed at merge time instead of reaching a client half-parsed.
fn validate_template_filter(filter: &Value, max_value_len: usize) -> Result<(), ManifestError> {
    const OPERATORS: [&str; 11] = [
        "gte", "gt", "lte", "lt", "eq", "neq", "contains", "starts-with", "in", "is-true",
        "is-false",
    ];
    let invalid = || ManifestError::InvalidField("templates");
    let object = filter.as_object().ok_or_else(invalid)?;
    let valid_path = object
        .get("path")
        .and_then(Value::as_str)
        .is_some_and(|path| !path.trim().is_empty() && path.len() <= 200);
    let operator = object.get("operator").and_then(Value::as_str);
    if !valid_path || !operator.is_some_and(|operator| OPERATORS.contains(&operator)) {
        return Err(invalid());
    }
    let operator = operator.expect("checked above");
    let takes_value = operator != "is-true" && operator != "is-false";
    if !takes_value
        && (object.contains_key("values")
            || object.contains_key("quantifier")
            || object.contains_key("as"))
    {
        return Err(invalid());
    }
    if let Some(values) = object.get("values") {
        let values = values.as_array().ok_or_else(invalid)?;
        if values
            .iter()
            .any(|value| value.as_str().is_none_or(|value| value.len() > max_value_len))
        {
            return Err(invalid());
        }
    }
    if let Some(quantifier) = object.get("quantifier") {
        let quantifier = quantifier.as_str().ok_or_else(invalid)?;
        if quantifier != "any" && quantifier != "all" {
            return Err(invalid());
        }
        // `in` is a per-value probe: universal quantification is meaningless.
        if operator == "in" && quantifier == "all" {
            return Err(invalid());
        }
    }
    if let Some(value) = object.get("value") {
        if value.as_str().is_none_or(|value| value.len() > max_value_len) {
            return Err(invalid());
        }
    }
    if let Some(alias) = object.get("as") {
        let alias = alias.as_str().ok_or_else(invalid)?;
        let valid_alias = {
            let mut chars = alias.chars();
            matches!(chars.next(), Some(first) if first.is_ascii_alphabetic())
                && alias.len() <= 32
                && alias.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        };
        if !valid_alias || matches!(alias, "match" | "event" | "globals" | "data" | "user") {
            return Err(invalid());
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
