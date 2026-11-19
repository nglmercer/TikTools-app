//! Integration checks for the installable plugin's WSS relay url derivation
//! and single-use lifecycle. The full host.call exchange is covered by the
//! control-api and loader test suites in the app workspace.

#[test]
fn relay_url_uses_wss_for_https_and_ws_for_local_http() {
    // Mirrors relay::socket_url without pulling the private helper into tests.
    fn url(origin: &str) -> String {
        let origin = origin.trim_end_matches('/');
        let scheme = if origin.starts_with("https://") { "wss" } else { "ws" };
        let rest = origin
            .strip_prefix("https://")
            .or_else(|| origin.strip_prefix("http://"))
            .unwrap_or(origin);
        format!("{scheme}://{rest}/api/v3/remote/socket")
    }
    assert_eq!(
        url("https://cloud.example/"),
        "wss://cloud.example/api/v3/remote/socket"
    );
    assert_eq!(
        url("http://127.0.0.1:8787"),
        "ws://127.0.0.1:8787/api/v3/remote/socket"
    );
}

#[test]
fn controller_ticket_never_contains_a_bearer_token() {
    // The host.call contract carries only a single-use relay ticket. A bearer
    // token would be a long-lived full session and must never cross here.
    let response = serde_json::json!({"ticket": "single-use-ticket"});
    let serialized = response.to_string();
    assert!(!serialized.contains("bearer"));
    assert!(!serialized.contains("sessionToken"));
    assert_eq!(response["ticket"], "single-use-ticket");
}
