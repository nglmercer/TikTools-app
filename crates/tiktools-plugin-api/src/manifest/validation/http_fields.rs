//! Shared bounds and syntax for declarative request paths, header maps and HTTP timeouts.

use super::fields::is_header_token;
use super::{MAX_HTTP_HEADERS, MAX_HTTP_PATH_LEN, MIN_HTTP_TIMEOUT_MS};
use serde_json::Value;

pub(super) fn validate_relative_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() || path.len() > MAX_HTTP_PATH_LEN {
        return Err("path length");
    }
    if !path.starts_with('/') {
        return Err("path must be base-relative");
    }
    if path.chars().any(|character| character.is_control()) {
        return Err("path holds control characters");
    }
    // Template spans may carry whitespace (`{{ config.voice }}`); the
    // literal remainder must not, and it must not carry a scheme.
    let mut rest = path;
    while let Some(start) = rest.find("{{") {
        let head = &rest[..start];
        if head.chars().any(char::is_whitespace) || head.contains("://") {
            return Err("path must stay base-relative");
        }
        let tail = &rest[start + 2..];
        let Some(end) = tail.find("}}") else {
            return Err("path holds an unclosed template");
        };
        rest = &tail[end + 2..];
    }
    if rest.chars().any(char::is_whitespace) || rest.contains("://") {
        return Err("path must stay base-relative");
    }
    Ok(())
}

pub(super) fn validate_header_map(headers: &Value) -> Result<(), &'static str> {
    let headers = headers.as_object().ok_or("headers must be an object")?;
    if headers.len() > MAX_HTTP_HEADERS {
        return Err("too many headers");
    }
    for (name, value) in headers {
        if !is_header_token(name) {
            return Err("invalid header name");
        }
        let value = value.as_str().ok_or("header values must be strings")?;
        if value.len() > 4_096 || value.chars().any(|char| matches!(char, '\r' | '\n')) {
            return Err("invalid header value");
        }
    }
    Ok(())
}

pub(super) fn validate_http_timeout(timeout: &Value, max: u64) -> Result<(), &'static str> {
    let timeout = timeout.as_u64().ok_or("timeout must be a number")?;
    if !(MIN_HTTP_TIMEOUT_MS..=max).contains(&timeout) {
        return Err("timeout out of range");
    }
    Ok(())
}
