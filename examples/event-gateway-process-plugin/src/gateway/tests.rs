//! Event gateway unit tests.

use super::auth::{origin_allowed, tokens_equal, widget_topics_allowed};
use super::config::persist_generated_credentials;
use super::config::{settings_path_for, GatewayConfig, DEFAULT_ORIGIN, DEFAULT_PORT};
use super::http::{cors_headers, parse_http_request};
use super::server::reconcile_connections;
use super::topics::{matches_topics, query_topics};
use super::websocket::{handshake_response, WsEndpoint};
use serde_json::json;
use std::collections::HashMap;

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

fn websocket_request_headers() -> HashMap<String, String> {
    HashMap::from([
        ("host".to_owned(), "127.0.0.1:45231".to_owned()),
        ("upgrade".to_owned(), "websocket".to_owned()),
        ("connection".to_owned(), "Upgrade".to_owned()),
        (
            "sec-websocket-key".to_owned(),
            "dGhlIHNhbXBsZSBub25jZQ==".to_owned(),
        ),
        ("sec-websocket-version".to_owned(), "13".to_owned()),
    ])
}

#[test]
fn websocket_handshake_matches_rfc_example() {
    let response = handshake_response(&websocket_request_headers(), http::Version::HTTP_11)
        .expect("valid handshake");
    assert_eq!(response.status(), http::StatusCode::SWITCHING_PROTOCOLS);
    assert_eq!(
        response
            .headers()
            .get(http::header::SEC_WEBSOCKET_ACCEPT)
            .and_then(|value| value.to_str().ok()),
        Some("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=")
    );
}

#[test]
fn websocket_handshake_rejects_wrong_sec_version() {
    let mut headers = websocket_request_headers();
    headers.insert("sec-websocket-version".to_owned(), "12".to_owned());
    assert!(handshake_response(&headers, http::Version::HTTP_11).is_err());
}

#[test]
fn websocket_handshake_rejects_invalid_key() {
    let mut headers = websocket_request_headers();
    headers.insert(
        "sec-websocket-key".to_owned(),
        "not-valid-base64!!!".to_owned(),
    );
    assert!(handshake_response(&headers, http::Version::HTTP_11).is_err());
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
    let topics = query_topics("topics=live.%2A%2Cplugin.%2A").expect("valid topics");
    assert!(matches_topics(&topics, "live.event"));
    assert!(matches_topics(&topics, "plugin.started"));
    assert!(!matches_topics(&topics, "points.changed"));
}

#[test]
fn missing_topics_query_defaults_to_wildcard() {
    assert_eq!(query_topics("").expect("default topics"), vec!["*"]);
    assert_eq!(query_topics("other=1").expect("default topics"), vec!["*"]);
}

#[test]
fn malformed_topics_query_is_rejected_not_widened() {
    assert!(query_topics("topics=live.***").is_err());
    assert!(query_topics("topics=").is_err());
    assert!(query_topics("topics=%20%20").is_err());
    assert!(query_topics("topics=live.event%2C%20").expect("valid topics") == vec!["live.event"]);
}

#[test]
fn namespace_wildcards_cover_bare_prefix_and_children() {
    let topics = vec!["live.*".to_owned()];
    assert!(matches_topics(&topics, "live"));
    assert!(matches_topics(&topics, "live.event"));
    assert!(matches_topics(&topics, "live.event.deeper"));
    assert!(!matches_topics(&topics, "lively"));
    assert!(!matches_topics(&topics, "points.changed"));
}

#[test]
fn match_all_and_exact_topics() {
    assert!(matches_topics(&["*".to_owned()], "anything.at.all"));
    assert!(matches_topics(
        &["points.changed".to_owned()],
        "points.changed"
    ));
    assert!(!matches_topics(&["points.changed".to_owned()], "points"));
    assert!(!matches_topics(&[], "points.changed"));
}

#[test]
fn widget_routes_cover_index_and_assets() {
    use super::widgets::{is_widget_index, is_widget_route};
    assert!(is_widget_route("/widgets"));
    assert!(is_widget_route("/widgets/"));
    assert!(is_widget_route("/widgets/follow/"));
    assert!(is_widget_route("/widgets/gift/index.html"));
    assert!(is_widget_route("/widgets/chat/"));
    assert!(is_widget_route("/widgets/share/"));
    assert!(is_widget_route("/widgets/subscribe/"));
    assert!(!is_widget_route("/widgets-follow/"));
    assert!(!is_widget_route("/health"));
    assert!(is_widget_index("/widgets"));
    assert!(is_widget_index("/widgets/"));
    assert!(!is_widget_index("/widgets/follow/"));
}

#[test]
fn widget_segments_map_names_to_index_and_reject_traversal() {
    use super::widgets::widget_asset_segments;
    assert_eq!(
        widget_asset_segments("/widgets/follow/"),
        Some(vec!["follow".to_owned(), "index.html".to_owned()])
    );
    assert_eq!(
        widget_asset_segments("/widgets/gift"),
        Some(vec!["gift".to_owned(), "index.html".to_owned()])
    );
    for kind in ["chat", "share", "subscribe"] {
        assert_eq!(
            widget_asset_segments(&format!("/widgets/{kind}/")),
            Some(vec![kind.to_owned(), "index.html".to_owned()]),
            "{kind} must resolve"
        );
    }
    assert_eq!(
        widget_asset_segments("/widgets/follow/assets/app.js"),
        Some(vec![
            "follow".to_owned(),
            "assets".to_owned(),
            "app.js".to_owned()
        ])
    );
    assert_eq!(widget_asset_segments("/widgets/"), None);
    assert_eq!(widget_asset_segments("/widgets"), None);
    assert_eq!(widget_asset_segments("/widgets/other/"), None);
    assert_eq!(widget_asset_segments("/widgets/../secret"), None);
    assert_eq!(widget_asset_segments("/widgets/follow/../../secret"), None);
    assert_eq!(widget_asset_segments("/widgets/%2e%2e/secret"), None);
    assert_eq!(widget_asset_segments("/widgets/follow%2f..%2fsecret"), None);
    assert_eq!(widget_asset_segments("/widgets/follow\\..\\secret"), None);
    assert_eq!(widget_asset_segments("/widgets/%"), None);
    assert_eq!(widget_asset_segments("/widgets/%zz"), None);
}

#[test]
fn widget_content_types_cover_built_assets() {
    use super::widgets::widget_content_type;
    assert_eq!(
        widget_content_type("index.html"),
        "text/html; charset=utf-8"
    );
    assert_eq!(
        widget_content_type("app.js"),
        "text/javascript; charset=utf-8"
    );
    assert_eq!(
        widget_content_type("app.mjs"),
        "text/javascript; charset=utf-8"
    );
    assert_eq!(widget_content_type("app.css"), "text/css; charset=utf-8");
    assert_eq!(widget_content_type("icon.svg"), "image/svg+xml");
    assert_eq!(widget_content_type("font.woff2"), "font/woff2");
    assert_eq!(widget_content_type("APP.CSS"), "text/css; charset=utf-8");
    assert_eq!(
        widget_content_type("no-extension"),
        "application/octet-stream"
    );
}

#[test]
fn widgets_root_prefers_explicit_existing_dir() {
    use super::widgets::widgets_root;
    let dir = std::env::temp_dir().join(format!("tiktools-widgets-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config = GatewayConfig {
        widgets_dir: Some(dir.clone()),
        ..GatewayConfig::default()
    };
    assert_eq!(widgets_root(&config), Some(dir.clone()));
    let config = GatewayConfig {
        widgets_dir: Some(dir.join("missing")),
        ..GatewayConfig::default()
    };
    assert_eq!(widgets_root(&config), None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn widgets_dir_setting_parses_and_validates() {
    let config = GatewayConfig::from_settings(&json!({"widgetsDir": "/tmp/widgets"})).unwrap();
    assert_eq!(
        config.widgets_dir,
        Some(std::path::PathBuf::from("/tmp/widgets"))
    );
    let config = GatewayConfig::from_settings(&json!({})).unwrap();
    assert_eq!(config.widgets_dir, None);
    let config = GatewayConfig::from_settings(&json!({"widgetsDir": ""})).unwrap();
    assert_eq!(config.widgets_dir, None);
    assert!(GatewayConfig::from_settings(&json!({"widgetsDir": 42})).is_err());
}

#[test]
fn token_comparison_covers_equal_and_unequal() {
    assert!(tokens_equal("ttk_secret", "ttk_secret"));
    assert!(!tokens_equal("ttk_secret", "ttk_secreu"));
    assert!(!tokens_equal("short", "much-longer-token"));
    assert!(!tokens_equal("", "ttk_secret"));
    // An empty stored credential never matches, not even against an
    // empty guess: missing secrets fail closed.
    assert!(!tokens_equal("", ""));
}

#[test]
fn settings_generate_distinct_full_and_widget_credentials() {
    let config = GatewayConfig::from_settings(&json!({})).unwrap();
    assert!(config.token.starts_with("ttk_"));
    assert_eq!(config.token.len(), 4 + 64);
    assert!(config.widget_token.starts_with("ttw_"));
    assert_eq!(config.widget_token.len(), 4 + 64);
    assert_ne!(config.token, config.widget_token);
    // Explicit values are preserved verbatim.
    let config =
        GatewayConfig::from_settings(&json!({"token": "ttk_full", "widgetToken": "ttw_widget"}))
            .unwrap();
    assert_eq!(config.token, "ttk_full");
    assert_eq!(config.widget_token, "ttw_widget");
    assert!(GatewayConfig::from_settings(&json!({"widgetToken": "x".repeat(4097)})).is_err());
}

#[test]
fn settings_paths_are_namespaced_per_plugin() {
    let root = std::path::PathBuf::from("/data");
    assert_eq!(
        settings_path_for(&root, "tiktools.event-gateway"),
        std::path::PathBuf::from("/data/tiktools.event-gateway/settings.json")
    );
    // Missing or path-unsafe ids fall back to the legacy shared-root
    // file instead of escaping the data directory.
    for id in ["", "..", "../escape", "a/b", "a\\b", "a\0b"] {
        assert_eq!(
            settings_path_for(&root, id),
            std::path::PathBuf::from("/data/settings.json"),
            "id {id:?} must fall back to the shared-root file"
        );
    }
}

#[test]
fn generated_credentials_persist_and_survive_reload() {
    let dir = std::env::temp_dir().join(format!(
        "tiktools-gateway-credentials-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = dir.join("tiktools.event-gateway").join("settings.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // First start (fresh install): both credentials are generated and
    // persisted; a restart/reinstall reloads the same values, so saved
    // OBS URLs keep working.
    let first = GatewayConfig::from_settings(&json!({})).unwrap();
    persist_generated_credentials(&first, Some(&path)).unwrap();
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(stored["token"], first.token);
    assert_eq!(stored["widgetToken"], first.widget_token);
    let second = GatewayConfig::from_settings(&stored).unwrap();
    assert_eq!(second.token, first.token);
    assert_eq!(second.widget_token, first.widget_token);
    // Persisting again never rotates stored credentials.
    persist_generated_credentials(&second, Some(&path)).unwrap();
    let stored_again: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(stored_again, stored);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn self_loopback_origins_are_allowed_without_config() {
    let config = GatewayConfig::from_settings(&json!({})).unwrap();
    let port = config.port;
    assert!(origin_allowed(
        Some(&format!("http://127.0.0.1:{port}")),
        &config
    ));
    let bound = GatewayConfig::from_settings(&json!({"bind": "::1"})).unwrap();
    assert!(origin_allowed(
        Some(&format!("http://[::1]:{}", bound.port)),
        &bound
    ));
    // Configured origins keep working, unrelated origins stay rejected,
    // and non-browser clients without an origin still pass.
    assert!(origin_allowed(Some(DEFAULT_ORIGIN), &config));
    assert!(!origin_allowed(Some("https://evil.example"), &config));
    assert!(!origin_allowed(Some("http://127.0.0.1:9"), &config));
    assert!(origin_allowed(None, &config));
}

#[test]
fn cors_echo_matches_the_request_allowlist_without_wildcards() {
    let config = GatewayConfig::from_settings(&json!({})).unwrap();
    let own = format!("http://127.0.0.1:{}", config.port);
    let headers = cors_headers(Some(&own), &config);
    assert!(headers.contains(&format!("Access-Control-Allow-Origin: {own}")));
    assert!(cors_headers(Some("https://evil.example"), &config).is_empty());
    assert!(cors_headers(None, &config).is_empty());
    assert!(!headers.contains('*'));
}

#[test]
fn widget_endpoints_restrict_credentials_and_topics() {
    let config =
        GatewayConfig::from_settings(&json!({"token": "ttk_full", "widgetToken": "ttw_widget"}))
            .unwrap();
    assert_eq!(WsEndpoint::Full.expected_credential(&config), "ttk_full");
    assert_eq!(
        WsEndpoint::Widgets.expected_credential(&config),
        "ttw_widget"
    );
    assert_eq!(WsEndpoint::Full.initial_topics(), vec!["*".to_owned()]);
    assert_eq!(
        WsEndpoint::Widgets.initial_topics(),
        vec!["live.event".to_owned()]
    );
    assert!(widget_topics_allowed(&["live.event".to_owned()]));
    assert!(widget_topics_allowed(&[
        "live.event".to_owned(),
        "event.gap".to_owned()
    ]));
    for topics in [
        vec!["*".to_owned()],
        vec!["live.*".to_owned()],
        vec!["live.event".to_owned(), "*".to_owned()],
        vec!["plugin.event".to_owned()],
    ] {
        assert!(
            !widget_topics_allowed(&topics),
            "widget topics {topics:?} must be rejected"
        );
    }
}

#[tokio::test]
async fn reconcile_drops_only_finished_tasks() {
    let finished = tokio::spawn(async {});
    let (_sender, receiver) = tokio::sync::oneshot::channel::<()>();
    let pending = tokio::spawn(async move {
        let _ = receiver.await;
    });
    // Let the runtime poll both tasks: the empty task completes while
    // the channel waiter stays pending.
    for _ in 0..10 {
        if finished.is_finished() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(finished.is_finished());
    assert!(!pending.is_finished());
    let mut connections = vec![finished, pending];
    reconcile_connections(&mut connections);
    assert_eq!(connections.len(), 1);
    for handle in &connections {
        handle.abort();
    }
    for handle in connections {
        let _ = handle.await;
    }
}
