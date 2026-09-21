//! Origin policy and token authentication.
//!
//! The persistent gateway token is accepted only as an
//! `Authorization: Bearer` header (HTTP) or inside the encrypted
//! WebSocket `auth` control message. It is never accepted from URL
//! query strings, which leak into logs, history, and referers.

use super::config::GatewayConfig;
use std::collections::HashMap;

/// Topics a widget-scoped credential may subscribe to. Widgets render
/// `live.event` alerts and observe `event.gap` markers; wildcards and every
/// other topic stay exclusive to the full gateway token.
pub(crate) const WIDGET_TOPICS: &[&str] = &["live.event", "event.gap"];

pub(crate) fn widget_topics_allowed(topics: &[String]) -> bool {
    topics
        .iter()
        .all(|topic| WIDGET_TOPICS.contains(&topic.as_str()))
}

pub(crate) fn origin_allowed(origin: Option<&str>, config: &GatewayConfig) -> bool {
    origin.is_none_or(|origin| {
        config
            .allowed_origins
            .iter()
            .any(|allowed| allowed == origin)
            || self_origins(config).iter().any(|own| own == origin)
    })
}

/// The gateway's own loopback origins. Widget pages are served by the
/// gateway itself, so they must be allowed to connect back without manual
/// `allowedOrigins` edits. Both loopback spellings are always safe here:
/// the server only binds loopback, so a remote attacker can never mint
/// traffic carrying one of these origins.
pub(crate) fn self_origins(config: &GatewayConfig) -> [String; 2] {
    let port = config.port;
    let bound = match config.bind {
        std::net::IpAddr::V4(addr) => format!("http://{addr}:{port}"),
        std::net::IpAddr::V6(addr) => format!("http://[{addr}]:{port}"),
    };
    [format!("http://127.0.0.1:{port}"), bound]
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
/// guessed token matches the real one. An empty expected credential never
/// matches, so a missing stored secret fails closed instead of accepting
/// an empty guess.
pub(crate) fn tokens_equal(provided: &str, expected: &str) -> bool {
    let provided = provided.as_bytes();
    let expected = expected.as_bytes();
    if expected.is_empty() || provided.len() != expected.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in provided.iter().zip(expected.iter()) {
        difference |= left ^ right;
    }
    difference == 0
}
