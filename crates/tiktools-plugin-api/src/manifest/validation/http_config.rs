//! Declarative HTTP integration, authentication, token import and provisioning descriptors.
//! Runtime URL policy still runs after rendering.

use super::{
    fields::is_header_token,
    http_fields::{validate_header_map, validate_http_timeout, validate_relative_path},
};
use super::{ManifestError, MAX_BASE_URL_LEN, MAX_HTTP_FETCH_TIMEOUT_MS};
use serde_json::Value;

/// Validate the top-level declarative `http` integration block (v3 only).
/// The block is parsed strictly: a malformed base URL or auth shape fails
/// discovery instead of silently degrading host-side fetches.
pub fn validate_http_config(http: &Value) -> Result<(), ManifestError> {
    let object = http
        .as_object()
        .ok_or(ManifestError::InvalidField("http"))?;
    let base_url = object
        .get("baseUrl")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("http"))?;
    validate_base_url(base_url)?;
    if let Some(timeout) = object.get("timeoutMs") {
        validate_http_timeout(timeout, MAX_HTTP_FETCH_TIMEOUT_MS)
            .map_err(|_| ManifestError::InvalidField("http"))?;
    }
    if let Some(allow) = object.get("allowPrivateNetwork") {
        if !allow.is_boolean() {
            return Err(ManifestError::InvalidField("http"));
        }
    }
    if let Some(headers) = object.get("headers") {
        validate_header_map(headers).map_err(|_| ManifestError::InvalidField("http"))?;
    }
    if let Some(auth) = object.get("auth") {
        validate_http_auth(auth)?;
    }
    if let Some(health) = object.get("health") {
        let health = health
            .as_object()
            .ok_or(ManifestError::InvalidField("http"))?;
        let path = health
            .get("path")
            .and_then(Value::as_str)
            .ok_or(ManifestError::InvalidField("http"))?;
        validate_relative_path(path).map_err(|_| ManifestError::InvalidField("http"))?;
        if health.get("readinessField").is_some_and(|v| {
            v.as_str().is_none_or(|s| {
                s.is_empty()
                    || s.len() > 128
                    || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
        }) {
            return Err(ManifestError::InvalidField("http"));
        }
        if let Some(probe) = health.get("authProbePath") {
            let probe = probe.as_str().ok_or(ManifestError::InvalidField("http"))?;
            validate_relative_path(probe).map_err(|_| ManifestError::InvalidField("http"))?;
        }
        if let Some(timeout) = health.get("timeoutMs") {
            validate_http_timeout(timeout, MAX_HTTP_FETCH_TIMEOUT_MS)
                .map_err(|_| ManifestError::InvalidField("http"))?;
        }
    }
    if let Some(spec) = object.get("tokenImport") {
        validate_token_import(spec, object.get("auth"))?;
    }
    if let Some(provisioning) = object.get("tokenProvisioning") {
        validate_token_provisioning(provisioning)?;
    }
    Ok(())
}

fn validate_token_import(spec: &Value, auth: Option<&Value>) -> Result<(), ManifestError> {
    let spec = spec
        .as_object()
        .ok_or(ManifestError::InvalidField("http"))?;
    if spec.get("format").and_then(Value::as_str) != Some("ini") {
        return Err(ManifestError::InvalidField("http"));
    }
    for key in ["section", "key", "fileName"] {
        if spec.get(key).and_then(Value::as_str).is_none_or(|s| {
            s.trim().is_empty()
                || s.len() > 128
                || s.chars().any(char::is_control)
                || s.contains(['/', '\\', '[', ']', '='])
        }) {
            return Err(ManifestError::InvalidField("http"));
        }
    }
    if spec
        .get("minLength")
        .is_some_and(|n| n.as_u64().is_none_or(|n| !(1..=4096).contains(&n)))
    {
        return Err(ManifestError::InvalidField("http"));
    }
    if let Some(discovery) = spec.get("discovery") {
        for key in ["steamFolder", "modManagerGame"] {
            if discovery.get(key).and_then(Value::as_str).is_none_or(|s| {
                s.is_empty()
                    || s.len() > 128
                    || s == "."
                    || s == ".."
                    || s.contains(['/', '\\', ':'])
                    || s.chars().any(char::is_control)
            }) {
                return Err(ManifestError::InvalidField("http"));
            }
        }
        let path = discovery
            .get("relativePath")
            .and_then(Value::as_str)
            .ok_or(ManifestError::InvalidField("http"))?;
        if path.len() > 256
            || path.split('/').count() > 8
            || path.contains(['\\', ':'])
            || path.chars().any(char::is_control)
            || path
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == "..")
            || path.rsplit('/').next() != spec.get("fileName").and_then(Value::as_str)
        {
            return Err(ManifestError::InvalidField("http"));
        }
    }
    if let Some(connection) = spec.get("connection") {
        for key in ["setting", "section", "hostKey", "portKey"] {
            if connection.get(key).and_then(Value::as_str).is_none_or(|s| {
                s.trim().is_empty() || s.len() > 128 || s.chars().any(char::is_control)
            }) {
                return Err(ManifestError::InvalidField("http"));
            }
        }
    }
    if auth
        .and_then(|auth| auth.get("tokenSetting"))
        .and_then(Value::as_str)
        .is_none()
    {
        return Err(ManifestError::InvalidField("http"));
    }
    Ok(())
}

/// Validate the optional `tokenProvisioning` descriptor: a named strategy
/// the host can execute to mint an API token from operator credentials.
/// Shape errors fail discovery; unknown strategy names pass validation but
/// the host offers no provisioning for them, so newer manifests keep
/// loading on older hosts with the button simply absent.
fn validate_token_provisioning(provisioning: &Value) -> Result<(), ManifestError> {
    let object = provisioning
        .as_object()
        .ok_or(ManifestError::InvalidField("http"))?;
    let strategy = object
        .get("strategy")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("http"))?;
    if strategy.trim().is_empty() || strategy.len() > 64 {
        return Err(ManifestError::InvalidField("http"));
    }
    Ok(())
}

fn validate_http_auth(auth: &Value) -> Result<(), ManifestError> {
    let object = auth
        .as_object()
        .ok_or(ManifestError::InvalidField("http"))?;
    let auth_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("http"))?;
    match auth_type {
        "none" => Ok(()),
        "bearer" => {
            require_token_setting(object)?;
            if let Some(scheme) = object.get("scheme") {
                let scheme = scheme.as_str().ok_or(ManifestError::InvalidField("http"))?;
                if scheme.trim().is_empty() || scheme.len() > 32 || !is_header_token(scheme) {
                    return Err(ManifestError::InvalidField("http"));
                }
            }
            Ok(())
        }
        "header" => {
            require_token_setting(object)?;
            let name = object
                .get("header")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("http"))?;
            if !is_header_token(name) || name.eq_ignore_ascii_case("authorization") {
                return Err(ManifestError::InvalidField("http"));
            }
            Ok(())
        }
        "query" => {
            require_token_setting(object)?;
            let param = object
                .get("param")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("http"))?;
            if param.trim().is_empty()
                || param.len() > 64
                || !param
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            {
                return Err(ManifestError::InvalidField("http"));
            }
            Ok(())
        }
        _ => Err(ManifestError::InvalidField("http")),
    }
}

fn require_token_setting(object: &serde_json::Map<String, Value>) -> Result<(), ManifestError> {
    let setting = object
        .get("tokenSetting")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("http"))?;
    if setting.trim().is_empty() || setting.len() > 128 {
        return Err(ManifestError::InvalidField("http"));
    }
    Ok(())
}

/// Base URLs are either literal http(s) URLs or `{{ settings.* }}` templates
/// (for example a user-overridable server URL). Only the skeleton is checked
/// here; the host validates the rendered URL strictly before every fetch, so
/// a setting value can never smuggle a new scheme past execution policy.
fn validate_base_url(base_url: &str) -> Result<(), ManifestError> {
    if base_url.is_empty() || base_url.len() > MAX_BASE_URL_LEN {
        return Err(ManifestError::InvalidField("http"));
    }
    if base_url.chars().any(|char| char.is_control()) {
        return Err(ManifestError::InvalidField("http"));
    }
    if base_url.contains("{{") {
        // Template spans may carry whitespace (`{{ settings.serverUrl }}`);
        // the literal remainder must not.
        let mut rest = base_url;
        let mut spans = 0;
        while let Some(start) = rest.find("{{") {
            let head = &rest[..start];
            if head.chars().any(char::is_whitespace) {
                return Err(ManifestError::InvalidField("http"));
            }
            let tail = &rest[start + 2..];
            let Some(end) = tail.find("}}") else {
                return Err(ManifestError::InvalidField("http"));
            };
            spans += 1;
            rest = &tail[end + 2..];
        }
        if spans == 0 || rest.chars().any(char::is_whitespace) {
            return Err(ManifestError::InvalidField("http"));
        }
        return Ok(());
    }
    if base_url.chars().any(char::is_whitespace) {
        return Err(ManifestError::InvalidField("http"));
    }
    let rest = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .ok_or(ManifestError::InvalidField("http"))?;
    if rest.is_empty() {
        return Err(ManifestError::InvalidField("http"));
    }
    Ok(())
}
