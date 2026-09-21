//! Event gateway unit tests.

use super::config::{GatewayConfig, DEFAULT_ORIGIN, DEFAULT_PORT};
use super::http::parse_http_request;
use super::topics::{matches_topics, query_topics};
use super::websocket::websocket_accept_key;
use serde_json::json;

#[test]
fn defaults_are_loopback_only() {
    let config = GatewayConfig::from_settings(&json!({})).unwrap();
    assert_eq!(config.port, DEFAULT_PORT);
    assert!(config.bind.is_loopback());
    assert_eq!(config.allowed_origins, vec![DEFAULT_ORIGIN]);
    assert!(!config.token.is_empty());
}

#[test]
fn remote_binding_is_rejected() {
    let error = GatewayConfig::from_settings(&json!({"bind": "0.0.0.0"})).unwrap_err();
    assert!(error.contains("loopback"));
}

#[test]
fn websocket_accept_key_matches_rfc_example() {
    assert_eq!(
        websocket_accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    );
}

#[test]
fn request_parser_keeps_auth_and_origin_headers() {
    let request = parse_http_request(
        b"GET /events?topics=live.%2A HTTP/1.1\r\nOrigin: https://widgets.tiktools.app\r\nAuthorization: Bearer abc\r\n\r\n",
    )
    .unwrap();
    assert_eq!(request.0, "GET");
    assert_eq!(request.1, "/events?topics=live.%2A");
    assert_eq!(request.2["origin"], "https://widgets.tiktools.app");
    assert_eq!(request.2["authorization"], "Bearer abc");
}

#[test]
fn topic_queries_are_wildcard_matched() {
    let topics = query_topics("topics=live.%2A%2Cplugin.%2A");
    assert!(matches_topics(&topics, "live.event"));
    assert!(matches_topics(&topics, "plugin.started"));
    assert!(!matches_topics(&topics, "points.changed"));
}
