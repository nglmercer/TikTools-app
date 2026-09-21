//! Origin policy and token authentication.
//!
//! The persistent gateway token is accepted only as an
//! `Authorization: Bearer` header (HTTP) or inside the encrypted
//! WebSocket `auth` control message. It is never accepted from URL
//! query strings, which leak into logs, history, and referers.

use super::config::GatewayConfig;
use std::collections::HashMap;

pub(crate) fn origin_allowed(origin: Option<&str>, config: &GatewayConfig) -> bool {
    origin.is_none_or(|origin| {
        config
            .allowed_origins
            .iter()
            .any(|allowed| allowed == origin)
    })
}

pub(crate) fn authorization_token(headers: &HashMap<String, String>) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
}

pub(crate) fn authorized_http(headers: &HashMap<String, String>, config: &GatewayConfig) -> bool {
    authorization_token(headers).is_some_and(|token| tokens_equal(token, &config.token))
}

/// Compares bearer tokens without short-circuiting on the first
/// differing byte, so response timing does not reveal how much of a
/// guessed token matches the real one.
pub(crate) fn tokens_equal(provided: &str, expected: &str) -> bool {
    let provided = provided.as_bytes();
    let expected = expected.as_bytes();
    if provided.len() != expected.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in provided.iter().zip(expected.iter()) {
        difference |= left ^ right;
    }
    difference == 0
}
