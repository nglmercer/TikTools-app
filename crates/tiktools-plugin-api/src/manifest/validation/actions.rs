//! Action timeouts, declarative HTTP requests and option-source descriptors. These rules
//! apply only where the caller selects the matching manifest schema.

use super::http_fields::{validate_header_map, validate_http_timeout, validate_relative_path};
use super::{
    ManifestError, MAX_DESCRIPTOR_BYTES, MAX_HTTP_ACTION_TIMEOUT_MS, MAX_HTTP_FETCH_TIMEOUT_MS,
    MAX_OPTION_SOURCES, MAX_PLUGIN_ACTION_TIMEOUT_MS,
};
use serde_json::Value;

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

/// Validate the declarative `http` block of a v3 action descriptor, plus its
/// per-field `optionSources`. Only called for schema v3 manifests so v2
/// descriptors keep flowing to process plugins untouched.
pub fn validate_declarative_action(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    if let Some(http) = object.get("http") {
        validate_action_http(http)?;
    }
    if let Some(sources) = object.get("optionSources") {
        let sources = sources
            .as_object()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if sources.len() > MAX_OPTION_SOURCES {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
        for (field, source) in sources {
            if field.trim().is_empty() || field.len() > 128 {
                return Err(ManifestError::InvalidField("actionTypes"));
            }
            validate_option_source(source)?;
        }
    }
    Ok(())
}

fn validate_action_http(http: &Value) -> Result<(), ManifestError> {
    let object = http
        .as_object()
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    if let Some(method) = object.get("method") {
        let method = method
            .as_str()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if !matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD") {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    validate_relative_path(path).map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    if let Some(headers) = object.get("headers") {
        validate_header_map(headers).map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    }
    for key in ["bodyJson", "bodyVariants", "idempotency"] {
        if let Some(value) = object.get(key) {
            if !value.is_object() || value.to_string().len() > MAX_DESCRIPTOR_BYTES {
                return Err(ManifestError::InvalidField("actionTypes"));
            }
        }
    }
    if let Some(spec) = object.get("bodyVariants") {
        if spec
            .get("field")
            .and_then(Value::as_str)
            .is_none_or(|s| s.is_empty())
            || spec
                .get("values")
                .and_then(Value::as_object)
                .is_none_or(|values| values.is_empty() || values.values().any(|v| !v.is_object()))
            || !object.contains_key("bodyJson")
        {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    if object.contains_key("bodyJson") && object.contains_key("body") {
        return Err(ManifestError::InvalidField("actionTypes"));
    }
    if let Some(transforms) = object.get("bodyTransforms") {
        // Transforms run on the rendered text body, so they only make sense
        // alongside `body` (never `bodyJson`, which is structured data).
        let valid = validate_body_transforms(transforms)
            && object.contains_key("body")
            && !object.contains_key("bodyJson");
        if !valid {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    if let Some(spec) = object.get("idempotency") {
        if !spec.get("enabled").is_some_and(Value::is_boolean)
            || spec.get("header").is_some_and(|h| {
                h.as_str().is_none_or(|h| {
                    h.is_empty() || !h.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                })
            })
        {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    if let Some(body) = object.get("body") {
        let body = body
            .as_str()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if body.len() > MAX_DESCRIPTOR_BYTES {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    if let Some(timeout) = object.get("timeoutMs") {
        validate_http_timeout(timeout, MAX_HTTP_ACTION_TIMEOUT_MS)
            .map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    }
    if let Some(defaults) = object.get("defaultsFromSettings") {
        validate_settings_defaults(defaults)?;
    }
    if let Some(spec) = object.get("timeoutByField") {
        validate_timeout_by_field(spec).map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    }
    if let Some(emit) = object.get("emitResponseAs") {
        let emit = emit
            .as_str()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if emit.trim().is_empty() || emit.len() > 128 {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    Ok(())
}

/// Shape rules for the ordered `bodyTransforms` pipeline. The executor owns
/// the op semantics; discovery only rejects specs it could never honour.
fn validate_body_transforms(transforms: &Value) -> bool {
    const MAX_TRANSFORMS: usize = 8;
    const OPS: &[&str] = &[
        "strip-prefix",
        "strip-suffix",
        "trim",
        "replace",
        "regex-replace",
    ];
    let Some(items) = transforms.as_array() else {
        return false;
    };
    if items.is_empty() || items.len() > MAX_TRANSFORMS {
        return false;
    }
    items.iter().all(|item| {
        let Some(op) = item.as_object() else {
            return false;
        };
        let Some(kind) = op.get("op").and_then(Value::as_str) else {
            return false;
        };
        if !OPS.contains(&kind) {
            return false;
        }
        let bounded = |key: &str| {
            op.get(key).is_none_or(|value| {
                value
                    .as_str()
                    .is_some_and(|text| text.len() <= MAX_DESCRIPTOR_BYTES)
            })
        };
        let required = |key: &str| {
            op.get(key)
                .and_then(Value::as_str)
                .is_some_and(|text| !text.is_empty() && text.len() <= MAX_DESCRIPTOR_BYTES)
        };
        match kind {
            "strip-prefix" | "strip-suffix" => required("value") && bounded("with"),
            "trim" => op.get("side").is_none_or(|side| {
                side.as_str()
                    .is_some_and(|text| matches!(text, "start" | "end" | "both"))
            }),
            "replace" => required("find") && bounded("with"),
            "regex-replace" => required("pattern") && bounded("with"),
            _ => false,
        }
    })
}

fn validate_settings_defaults(defaults: &Value) -> Result<(), ManifestError> {
    let valid = defaults.as_object().is_some_and(|defaults| {
        defaults.len() <= 32
            && defaults.iter().all(|(field, pointers)| {
                !field.is_empty()
                    && field.len() <= 64
                    && pointers.as_array().is_some_and(|pointers| {
                        !pointers.is_empty()
                            && pointers.len() <= 8
                            && pointers.iter().all(|pointer| {
                                pointer.as_str().is_some_and(|pointer| {
                                    pointer.starts_with('/') && pointer.len() <= 128
                                })
                            })
                    })
            })
    });
    if !valid {
        return Err(ManifestError::InvalidField("actionTypes"));
    }
    Ok(())
}

fn validate_option_source(source: &Value) -> Result<(), ManifestError> {
    let object = source
        .as_object()
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    validate_relative_path(path).map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    for key in ["itemsPath", "valuePath", "labelPath"] {
        if let Some(value) = object.get(key) {
            let value = value
                .as_str()
                .ok_or(ManifestError::InvalidField("actionTypes"))?;
            if value.trim().is_empty() || value.len() > 256 {
                return Err(ManifestError::InvalidField("actionTypes"));
            }
        }
    }
    if let Some(timeout) = object.get("timeoutMs") {
        validate_http_timeout(timeout, MAX_HTTP_FETCH_TIMEOUT_MS)
            .map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    }
    if let Some(params) = object.get("params") {
        // Contextual option sources: each entry renders a `{{ config.* }}`/
        // `{{ settings.* }}` template into one query argument. Bounded like
        // other untrusted descriptor text.
        let valid = params.as_object().is_some_and(|params| {
            !params.is_empty()
                && params.len() <= 8
                && params.iter().all(|(key, value)| {
                    !key.is_empty()
                        && key.len() <= 128
                        && key
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
                        && value
                            .as_str()
                            .is_some_and(|value| !value.is_empty() && value.len() <= 1024)
                })
        });
        if !valid {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    Ok(())
}

/// Validate the optional provider-aware `timeoutByField` action descriptor.
///
/// Shape errors fail discovery (the host cannot honour an unknown field shape),
/// so the resolved budget is always a bounded number. Each candidate value is
/// validated against the declarative action ceiling; the generic fetch clamp
/// does not apply here.
fn validate_timeout_by_field(spec: &Value) -> Result<(), &'static str> {
    let object = spec.as_object().ok_or("timeoutByField must be an object")?;
    let field = object
        .get("field")
        .and_then(Value::as_str)
        .ok_or("timeoutByField.field must be a string")?;
    if field.trim().is_empty() || field.len() > 128 {
        return Err("timeoutByField.field out of range");
    }
    let values = object
        .get("values")
        .and_then(Value::as_object)
        .ok_or("timeoutByField.values must be an object")?;
    if values.is_empty() || values.len() > 32 {
        return Err("timeoutByField.values out of range");
    }
    for (key, value) in values {
        if key.trim().is_empty() || key.len() > 128 {
            return Err("timeoutByField key out of range");
        }
        validate_http_timeout(value, MAX_HTTP_ACTION_TIMEOUT_MS)?;
    }
    if let Some(default) = object.get("defaultMs") {
        validate_http_timeout(default, MAX_HTTP_ACTION_TIMEOUT_MS)?;
    }
    Ok(())
}

#[cfg(test)]
mod settings_defaults_validation_tests {
    use crate::manifest::PluginManifest;
    use serde_json::json;

    #[test]
    fn settings_defaults_require_bounded_json_pointer_lists() {
        for (defaults, valid) in [
            (json!({"provider":["/tts/providerId"]}), true),
            (json!({"provider":"/tts/providerId"}), false),
            (json!({"provider":["tts.providerId"]}), false),
            (json!({"provider":[]}), false),
            (json!({"provider":[null]}), false),
        ] {
            let document = json!({"schemaVersion":3,"id":"test.defaults","name":"Defaults","version":"1.0.0","runtime":"declarative","http":{"baseUrl":"http://127.0.0.1:17842"},"actionTypes":[{"id":"test.defaults.speak","http":{"method":"POST","path":"/api/tts/play","defaultsFromSettings":defaults}}]});
            assert_eq!(
                PluginManifest::from_json_str(&document.to_string()).is_ok(),
                valid
            );
        }
    }
}
