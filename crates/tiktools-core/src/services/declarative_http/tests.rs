//! Declarative HTTP unit tests.

use super::{
    build_declarative_request, declarative_request_line, redact_endpoint_secrets,
    render_scoped_template, with_auth_hint, DeclarativeBuild, DeclarativeEndpoint,
};
use crate::services::capabilities::CapabilityBroker;
use crate::services::capabilities::SECRET_SETTING_PLACEHOLDER;
use serde_json::{json, Map, Value};
use std::sync::Arc;
use tiktools_plugin_api::PluginManifest;

fn manifest(http: Value) -> PluginManifest {
    PluginManifest::from_json_str(
        &serde_json::json!({
            "schemaVersion": 3,
            "id": "demo.http",
            "name": "Demo",
            "version": "1.0.0",
            "runtime": "declarative",
            "permissions": ["network.bind"],
            "http": http
        })
        .to_string(),
    )
    .unwrap()
}

fn broker() -> CapabilityBroker {
    CapabilityBroker::new(std::env::temp_dir())
}

#[test]
fn auth_failures_name_the_plugin_setting_and_attachment() {
    // A token was attached but rejected: point at the stored value.
    let hinted = with_auth_hint(
        "HTTP 401 request failed",
        "sonicboom.server",
        "bearer",
        "apiToken",
        true,
    );
    assert!(hinted.contains("sonicboom.server"), "{hinted}");
    assert!(hinted.contains("`apiToken`"), "{hinted}");
    assert!(hinted.contains("wrong, expired, or revoked"), "{hinted}");
    // Nothing attached: point at the empty setting.
    let hinted = with_auth_hint(
        "HTTP 401 request failed",
        "sonicboom.server",
        "bearer",
        "apiToken",
        false,
    );
    assert!(hinted.contains("`apiToken` setting is empty"), "{hinted}");
    // 403 gets the same treatment with forbidden wording.
    let hinted = with_auth_hint(
        "HTTP 403 request failed",
        "demo.http",
        "header",
        "apiKey",
        true,
    );
    assert!(hinted.contains("forbade"), "{hinted}");
    assert!(hinted.contains("`apiKey`"), "{hinted}");
    // Other statuses pass through untouched.
    assert_eq!(
        with_auth_hint(
            "HTTP 500 server error",
            "demo.http",
            "bearer",
            "apiToken",
            true
        ),
        "HTTP 500 server error"
    );
    assert_eq!(
        with_auth_hint(
            "connection refused",
            "demo.http",
            "bearer",
            "apiToken",
            false
        ),
        "connection refused"
    );
}

#[test]
fn request_line_reports_path_and_auth_presence_without_secrets() {
    let endpoint = DeclarativeEndpoint {
        method: "POST".to_owned(),
        url: "http://localhost:17842/api/tts/play?voice=M1".to_owned(),
        headers: Vec::new(),
        body: None,
        timeout_ms: 10_000,
        secrets: vec!["super-secret-token".to_owned()],
        allow_private_network: true,
    };
    let line = redact_endpoint_secrets(
        &declarative_request_line(&endpoint, "bearer", "apiToken"),
        &endpoint.secrets,
    );
    assert!(line.contains("POST /api/tts/play?voice=M1"), "{line}");
    assert!(line.contains("auth: bearer attached"), "{line}");
    assert!(!line.contains("super-secret-token"), "{line}");

    let bare = DeclarativeEndpoint {
        method: "POST".to_owned(),
        url: "http://localhost:17842/api/tts/play?voice=M1".to_owned(),
        headers: Vec::new(),
        body: None,
        timeout_ms: 10_000,
        secrets: Vec::new(),
        allow_private_network: true,
    };
    let line = declarative_request_line(&bare, "bearer", "apiToken");
    assert!(
        line.contains("auth: none attached (`apiToken` is empty)"),
        "{line}"
    );

    // Query-string auth embeds the token in the URL: redaction blanks it.
    let query = DeclarativeEndpoint {
        method: "GET".to_owned(),
        url: "http://localhost:17842/v1/voices?token=super-secret-token".to_owned(),
        headers: Vec::new(),
        body: None,
        timeout_ms: 10_000,
        secrets: vec!["super-secret-token".to_owned()],
        allow_private_network: true,
    };
    let line = redact_endpoint_secrets(
        &declarative_request_line(&query, "query", "apiToken"),
        &query.secrets,
    );
    assert!(!line.contains("super-secret-token"), "{line}");
    assert!(line.contains(SECRET_SETTING_PLACEHOLDER), "{line}");
}

#[test]
fn renders_event_settings_and_config_scopes() {
    let scope = json!({
        "event": {"data": {"comment": "hello"}},
        "settings": {"serverUrl": "http://localhost:17842"},
        "config": {"voice": "M1"}
    });
    assert_eq!(
        render_scoped_template(
            "{{ settings.serverUrl }}/play?voice={{ config.voice }}&text={{ event.data.comment }}",
            &scope
        ),
        "http://localhost:17842/play?voice=M1&text=hello"
    );
    assert_eq!(render_scoped_template("a{{ missing }}b", &scope), "ab");
    assert_eq!(
        render_scoped_template("a{{ unclosed", &scope),
        "a{{ unclosed"
    );
}

#[test]
fn loopback_is_trusted_without_permission_or_token() {
    let manifest = manifest(json!({
        "baseUrl": "{{ settings.serverUrl }}",
        "auth": {"type": "bearer", "tokenSetting": "apiToken"}
    }));
    let broker = broker();
    let scope = json!({"settings": {"serverUrl": "http://localhost:17842"}});
    let endpoint = build_declarative_request(&DeclarativeBuild {
        manifest: &manifest,
        http: manifest.http.as_ref().unwrap(),
        method: "POST",
        path_template: "/api/tts/play?voice={{ config.voice }}",
        extra_headers: None,
        body_template: Some("{{ event.data.comment }}"),
        timeout_override_ms: None,
        timeout_by_field: None,
        scope: &scope,
        broker: &broker,
    })
    .unwrap();
    assert_eq!(endpoint.url, "http://localhost:17842/api/tts/play?voice=");
    assert!(endpoint.allow_private_network);
    assert!(endpoint.secrets.is_empty());
    assert!(endpoint.headers.is_empty());
}

#[test]
fn remote_hosts_require_permission_and_token() {
    let manifest = manifest(json!({
        "baseUrl": "https://tts.example.com",
        "auth": {"type": "bearer", "tokenSetting": "apiToken"}
    }));
    let broker = broker();
    // Missing token is rejected even with the permission declared.
    let scope = json!({"settings": {}});
    assert!(build_declarative_request(&DeclarativeBuild {
        manifest: &manifest,
        http: manifest.http.as_ref().unwrap(),
        method: "GET",
        path_template: "/v1/voices",
        extra_headers: None,
        body_template: None,
        timeout_override_ms: None,
        timeout_by_field: None,
        scope: &scope,
        broker: &broker,
    })
    .is_err());
    // A present token authorizes with a Bearer header.
    let scope = json!({"settings": {"apiToken": "tok-123"}});
    let endpoint = build_declarative_request(&DeclarativeBuild {
        manifest: &manifest,
        http: manifest.http.as_ref().unwrap(),
        method: "GET",
        path_template: "/v1/voices",
        extra_headers: None,
        body_template: None,
        timeout_override_ms: None,
        timeout_by_field: None,
        scope: &scope,
        broker: &broker,
    })
    .unwrap();
    assert_eq!(endpoint.url, "https://tts.example.com/v1/voices");
    assert_eq!(
        endpoint.headers,
        vec![("authorization".to_owned(), "Bearer tok-123".to_owned())]
    );
    assert_eq!(endpoint.secrets, vec!["tok-123".to_owned()]);
    assert_eq!(
        redact_endpoint_secrets(
            "https://tts.example.com/?token=tok-123 failed",
            &endpoint.secrets
        ),
        "https://tts.example.com/?token=•••••••• failed"
    );
}

#[test]
fn remote_hosts_without_permission_are_rejected() {
    let manifest = PluginManifest::from_json_str(
        &serde_json::json!({
            "schemaVersion": 3,
            "id": "demo.noperm",
            "name": "Demo",
            "version": "1.0.0",
            "runtime": "declarative",
            "http": {
                "baseUrl": "https://tts.example.com",
                "auth": {"type": "bearer", "tokenSetting": "apiToken"}
            }
        })
        .to_string(),
    )
    .unwrap();
    let broker = broker();
    let scope = json!({"settings": {"apiToken": "tok-123"}});
    let error = build_declarative_request(&DeclarativeBuild {
        manifest: &manifest,
        http: manifest.http.as_ref().unwrap(),
        method: "GET",
        path_template: "/v1/voices",
        extra_headers: None,
        body_template: None,
        timeout_override_ms: None,
        timeout_by_field: None,
        scope: &scope,
        broker: &broker,
    })
    .unwrap_err();
    assert!(error.contains("network.bind"), "unexpected error: {error}");
}

#[test]
fn lan_hosts_require_the_manifest_opt_in() {
    let manifest = manifest(json!({"baseUrl": "http://192.168.1.10:3000"}));
    let broker = broker();
    let scope = json!({"settings": {}});
    let error = build_declarative_request(&DeclarativeBuild {
        manifest: &manifest,
        http: manifest.http.as_ref().unwrap(),
        method: "GET",
        path_template: "/ready",
        extra_headers: None,
        body_template: None,
        timeout_override_ms: None,
        timeout_by_field: None,
        scope: &scope,
        broker: &broker,
    })
    .unwrap_err();
    assert!(
        error.contains("allowPrivateNetwork"),
        "unexpected error: {error}"
    );
}

#[test]
fn rendered_paths_cannot_change_the_host() {
    let manifest = manifest(json!({"baseUrl": "http://localhost:17842"}));
    let broker = broker();
    let scope = json!({"config": {"next": "https://evil.example/"}});
    assert!(build_declarative_request(&DeclarativeBuild {
        manifest: &manifest,
        http: manifest.http.as_ref().unwrap(),
        method: "GET",
        path_template: "{{ config.next }}",
        extra_headers: None,
        body_template: None,
        timeout_override_ms: None,
        timeout_by_field: None,
        scope: &scope,
        broker: &broker,
    })
    .is_err());
}

/// Minimal canned HTTP/1.1 stub on loopback. Serves one response per
/// connection based on the request path; records nothing sensitive.
#[cfg(feature = "http")]
async fn stub_server(
    routes: std::collections::HashMap<String, (u16, String)>,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let mut buffer = vec![0u8; 8_192];
            let Ok(read) = socket.read(&mut buffer).await else {
                continue;
            };
            let request = String::from_utf8_lossy(&buffer[..read]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");
            let path = path.split('?').next().unwrap_or(path);
            let (status, body) = routes
                .get(path)
                .cloned()
                .unwrap_or_else(|| (404, r#"{"error":"not found"}"#.to_owned()));
            let reason = if status == 200 { "OK" } else { "Not Found" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    (address, task)
}

#[cfg(feature = "http")]
#[tokio::test]
async fn fetches_option_documents_from_loopback() {
    use std::sync::Mutex;
    struct Emitter {
        messages: Mutex<Vec<crate::ipc::messages::HostMessage>>,
    }
    impl crate::HostEmitter for Emitter {
        fn emit(&self, message: crate::ipc::messages::HostMessage) {
            self.messages.lock().unwrap().push(message);
        }
    }
    let (address, server) = stub_server(
        [
            ("/ready".to_owned(), (200, r#"{"ok":true}"#.to_owned())),
            (
                "/v1/voices".to_owned(),
                (
                    200,
                    r#"{"voices":[{"id":"M1","name":"Marcus"},{"id":"F2","name":"Freya"}]}"#
                        .to_owned(),
                ),
            ),
        ]
        .into_iter()
        .collect(),
    )
    .await;
    let core = Arc::new(crate::AppCore::new(Arc::new(Emitter {
        messages: Mutex::new(Vec::new()),
    })));
    let manifest = manifest(json!({
        "baseUrl": format!("http://{address}"),
        "auth": {"type": "bearer", "tokenSetting": "apiToken"}
    }));
    let scope = json!({"settings": {"apiToken": "tok-123"}});
    let body = core
        .fetch_declarative_json(&manifest, "GET", "/ready", None, &scope)
        .await
        .unwrap();
    assert_eq!(body.get("ok"), Some(&Value::Bool(true)));
    let body = core
        .fetch_declarative_json(&manifest, "GET", "/v1/voices", None, &scope)
        .await
        .unwrap();
    let options = super::super::option_sources::map_option_items(&body, None, None, None).unwrap();
    assert_eq!(
        options,
        vec![
            json!({"value": "M1", "label": "Marcus"}),
            json!({"value": "F2", "label": "Freya"}),
        ]
    );
    server.abort();
}

#[test]
fn builds_output_switch_post_with_bearer_auth() {
    let manifest = manifest(json!({
        "baseUrl": "{{ settings.serverUrl }}",
        "auth": {"type": "bearer", "tokenSetting": "apiToken"}
    }));
    let broker = broker();
    let scope = json!({
        "settings": {"serverUrl": "http://localhost:17842", "apiToken": "tok-123"},
        "config": {"device": "CABLE Input (VB-Audio Virtual Cable)"}
    });
    let mut headers = Map::new();
    headers.insert(
        "Content-Type".to_owned(),
        Value::String("application/json".to_owned()),
    );
    let endpoint = build_declarative_request(&DeclarativeBuild {
        manifest: &manifest,
        http: manifest.http.as_ref().unwrap(),
        method: "POST",
        path_template: "/api/audio/output",
        extra_headers: Some(&headers),
        body_template: Some("{\"device\":\"{{ config.device }}\"}"),
        timeout_override_ms: Some(10_000),
        timeout_by_field: None,
        scope: &scope,
        broker: &broker,
    })
    .unwrap();
    assert_eq!(endpoint.method, "POST");
    assert_eq!(endpoint.url, "http://localhost:17842/api/audio/output");
    assert_eq!(
        endpoint.body.as_deref(),
        Some("{\"device\":\"CABLE Input (VB-Audio Virtual Cable)\"}")
    );
    assert_eq!(
        endpoint.headers,
        vec![
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("authorization".to_owned(), "Bearer tok-123".to_owned()),
        ]
    );
    assert_eq!(endpoint.secrets, vec!["tok-123".to_owned()]);
    assert_eq!(endpoint.timeout_ms, 10_000);
}

#[cfg(feature = "http")]
#[tokio::test]
async fn maps_device_list_and_reports_selection() {
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _message: crate::ipc::messages::HostMessage) {}
    }
    let (address, server) = stub_server(
        [(
            "/api/audio/devices".to_owned(),
            (
                200,
                r#"{"devices":[{"id":"default","name":"System Default","is_default":true,"is_selected":false},{"id":"CABLE Input (VB-Audio Virtual Cable)","name":"CABLE Input (VB-Audio Virtual Cable)","is_default":false,"is_selected":true}],"selected":"CABLE Input (VB-Audio Virtual Cable)"}"#
                    .to_owned(),
            ),
        )]
        .into_iter()
        .collect(),
    )
    .await;
    let core = Arc::new(crate::AppCore::new(Arc::new(Emitter)));
    let manifest = manifest(json!({
        "baseUrl": format!("http://{address}"),
        "auth": {"type": "bearer", "tokenSetting": "apiToken"}
    }));
    let scope = json!({"settings": {"apiToken": "tok-123"}});
    let body = core
        .fetch_declarative_json(&manifest, "GET", "/api/audio/devices", None, &scope)
        .await
        .unwrap();
    let options = super::super::option_sources::map_option_items(
        &body,
        Some("devices"),
        Some("id"),
        Some("name"),
    )
    .unwrap();
    assert_eq!(
        options,
        vec![
            json!({"value": "default", "label": "System Default"}),
            json!({"value": "CABLE Input (VB-Audio Virtual Cable)", "label": "CABLE Input (VB-Audio Virtual Cable)"}),
        ]
    );
    assert_eq!(
        super::super::option_sources::selected_option_value(&body, Some("devices"), Some("id"))
            .as_deref(),
        Some("CABLE Input (VB-Audio Virtual Cable)")
    );
    server.abort();
}

#[cfg(feature = "http")]
#[tokio::test]
async fn missing_option_endpoint_reports_its_status() {
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _message: crate::ipc::messages::HostMessage) {}
    }
    // No routes: every path answers 404 like an older server without the
    // audio API. The UI matches this prefix to tell "unsupported" apart
    // from "unreachable", so the format is pinned here.
    let (address, server) = stub_server(std::collections::HashMap::new()).await;
    let core = Arc::new(crate::AppCore::new(Arc::new(Emitter)));
    let manifest = manifest(json!({
        "baseUrl": format!("http://{address}"),
        "auth": {"type": "bearer", "tokenSetting": "apiToken"}
    }));
    let scope = json!({"settings": {"apiToken": "tok-123"}});
    let error = core
        .fetch_declarative_json(&manifest, "GET", "/api/audio/devices", None, &scope)
        .await
        .unwrap_err();
    assert!(error.starts_with("HTTP 404"), "unexpected error: {error}");
    server.abort();
}

#[tokio::test]
async fn probe_reports_unknown_plugins_without_fetching() {
    use std::sync::Mutex;
    struct Emitter {
        messages: Mutex<Vec<crate::ipc::messages::HostMessage>>,
    }
    impl crate::HostEmitter for Emitter {
        fn emit(&self, message: crate::ipc::messages::HostMessage) {
            self.messages.lock().unwrap().push(message);
        }
    }
    let emitter = Arc::new(Emitter {
        messages: Mutex::new(Vec::new()),
    });
    let core = Arc::new(crate::AppCore::new(emitter.clone()));
    core.probe_plugin_connection("definitely.not.installed")
        .await;
    let messages = emitter.messages.lock().unwrap();
    assert_eq!(messages.len(), 1);
    match &messages[0] {
        crate::ipc::messages::HostMessage::PluginConnectionResult { id, ok, error, .. } => {
            assert_eq!(id, "definitely.not.installed");
            assert!(!ok);
            assert!(error
                .as_deref()
                .unwrap_or_default()
                .contains("not installed"));
        }
        other => panic!("unexpected probe message: {other:?}"),
    }
}

#[cfg(feature = "http")]
#[tokio::test]
async fn fetch_errors_never_carry_tokens() {
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _message: crate::ipc::messages::HostMessage) {}
    }
    // A closed loopback port fails fast without touching the network.
    let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = closed.local_addr().unwrap();
    drop(closed);
    let core = Arc::new(crate::AppCore::new(Arc::new(Emitter)));
    let manifest = manifest(json!({
        "baseUrl": format!("http://{address}"),
        "auth": {"type": "query", "tokenSetting": "apiToken", "param": "token"}
    }));
    let scope = json!({"settings": {"apiToken": "tok-secret-123"}});
    let error = core
        .fetch_declarative_json(&manifest, "GET", "/v1/voices", None, &scope)
        .await
        .unwrap_err();
    assert!(
        !error.contains("tok-secret-123"),
        "token leaked into error: {error}"
    );
    assert!(
        error.contains(SECRET_SETTING_PLACEHOLDER),
        "token not redacted in error: {error}"
    );
}

/// Builds one endpoint from an action-style scope, mirroring how the host
/// resolves a declarative action's HTTP block.
fn build_action(
    config: Value,
    timeout_override_ms: Option<u64>,
    timeout_by_field: Option<Value>,
) -> Result<DeclarativeEndpoint, String> {
    let http = json!({"baseUrl": "http://localhost:17842"});
    let manifest = manifest(http.clone());
    let scope = json!({"config": config});
    build_declarative_request(&DeclarativeBuild {
        manifest: &manifest,
        http: &http,
        method: "POST",
        path_template: "/api/tts/play",
        extra_headers: None,
        body_template: None,
        timeout_override_ms,
        timeout_by_field: timeout_by_field.as_ref().and_then(Value::as_object),
        scope: &scope,
        broker: &broker(),
    })
}

#[test]
fn provider_aware_timeout_picks_a_slow_budget_above_the_generic_cap() {
    let spec = json!({
        "field": "provider",
        "defaultMs": 15000,
        "values": {"supertonic": 15000, "pocket-tts": 60000, "qwen3-gguf": 180000}
    });
    let build = |provider: &str| {
        build_action(
            json!({"provider": provider}),
            Some(15000),
            Some(spec.clone()),
        )
        .unwrap()
        .timeout_ms
    };
    assert_eq!(build("supertonic"), 15_000);
    assert_eq!(build("pocket-tts"), 60_000);
    // 180s exceeds the generic automation clamp (120s): the declarative
    // action ceiling is intentionally larger so slow synthesis can finish.
    assert_eq!(build("qwen3-gguf"), 180_000);
    // An unlisted provider uses the declared default, never the largest value.
    assert_eq!(build("mystery"), 15_000);
}

#[test]
fn provider_aware_timeout_takes_precedence_over_static_timeout() {
    // Static action timeout is 15s, but the provider table requests 180s.
    let endpoint = build_action(
        json!({"provider": "qwen3-gguf"}),
        Some(15_000),
        Some(json!({
            "field": "provider",
            "values": {"qwen3-gguf": 180_000}
        })),
    )
    .unwrap();
    assert_eq!(endpoint.timeout_ms, 180_000);
    // With no provider value and no default, resolution falls back to the
    // static action timeout rather than silently widening it.
    let endpoint = build_action(
        json!({"provider": "other"}),
        Some(20_000),
        Some(json!({"field": "provider", "values": {"qwen3-gguf": 180_000}})),
    )
    .unwrap();
    assert_eq!(endpoint.timeout_ms, 20_000);
}

#[test]
fn resolved_timeout_stays_bounded_and_defaults_without_a_table() {
    // A static budget above the ceiling is clamped, never honoured verbatim.
    let endpoint = build_action(json!({}), Some(1_000_000), None).unwrap();
    assert_eq!(endpoint.timeout_ms, 180_000);
    // No override and no table falls back to the built-in default.
    let endpoint = build_action(json!({}), None, None).unwrap();
    assert_eq!(endpoint.timeout_ms, 10_000);
}

#[cfg(feature = "http")]
#[tokio::test]
async fn repo_posts_send_nested_json_and_unique_execution_keys() {
    let _guard = REPO_EXECUTION_TEST_LOCK.lock().await;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut keys = Vec::new();
        for status in [200, 400, 401, 404, 409, 422, 429, 500, 501, 503, 504] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let header_end;
            loop {
                let mut chunk = [0; 4096];
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end = end + 4;
                    break;
                }
            }
            let headers = String::from_utf8_lossy(&bytes[..header_end]).to_lowercase();
            assert!(headers.starts_with("post /api/spawn/enemy "));
            let length: usize = headers
                .lines()
                .find_map(|l| l.strip_prefix("content-length: "))
                .unwrap()
                .parse()
                .unwrap();
            while bytes.len() < header_end + length {
                let mut chunk = [0; 4096];
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
            }
            let key = headers
                .lines()
                .find_map(|l| l.strip_prefix("idempotency-key: "))
                .unwrap()
                .to_owned();
            assert!(!keys.contains(&key));
            keys.push(key);
            let body: Value =
                serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
            assert_eq!(body["enemyId"], "fixture\"\\\n{{ untouched }}");
            assert_eq!(body["quantity"], 2);
            assert_eq!(body["position"], json!({"mode":"random_player"}));
            let response = r#"{"error":{"code":"FIXTURE_ERROR","message":"contract fixture","details":{"spawned":1}}}"#;
            socket.write_all(format!("HTTP/1.1 {status} Fixture\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}", response.len()).as_bytes()).await.unwrap();
        }
    });
    let core = Arc::new(crate::AppCore::new(Arc::new(Emitter)));
    let mut document: Value = serde_json::from_str(include_str!(
        "../../../../../examples/repo-game/plugin.json"
    ))
    .unwrap();
    document["http"]["baseUrl"] = json!(format!("http://{address}"));
    let manifest = PluginManifest::from_json_str(&document.to_string()).unwrap();
    let descriptor = &document["actionTypes"][0]["http"];
    let plugin = tiktools_plugin_loader::DiscoveredPlugin {
        manifest,
        directory: std::env::temp_dir(),
        source: tiktools_plugin_loader::PluginSource::Development,
        available: true,
        reason: None,
        running: true,
    };
    for status in [200, 400, 401, 404, 409, 422, 429, 500, 501, 503, 504] {
        let result = core.execute_declarative_action(&plugin, descriptor,
            &json!({"config":{"enemyId":"fixture\"\\\n{{ untouched }}","quantity":2,"positionMode":"random_player"}}),
            &json!({}), &mut Vec::new(), false).await;
        if status == 200 {
            assert!(result.is_ok(), "{result:?}");
        } else {
            let error = result.unwrap_err();
            assert!(error.contains(&status.to_string()), "{error}");
            assert!(error.contains("FIXTURE_ERROR"), "{error}");
        }
    }
    tokio::time::timeout(std::time::Duration::from_secs(10), server)
        .await
        .unwrap()
        .unwrap();
}

#[test]
fn repo_dynamic_catalogs_use_runtime_ids_and_names() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../../../examples/repo-game/plugin.json"
    ))
    .unwrap();
    for action in document["actionTypes"].as_array().unwrap() {
        if let Some(sources) = action["optionSources"].as_object() {
            for (_, spec) in sources {
                let root = spec["itemsPath"].as_str().unwrap();
                let body = json!({ root: [{"id":"modded-runtime-id", "name":"Runtime name"}] });
                let mapped = super::super::option_sources::map_option_items(
                    &body,
                    Some(root),
                    spec["valuePath"].as_str(),
                    spec["labelPath"].as_str(),
                )
                .unwrap();
                assert_eq!(
                    mapped,
                    vec![json!({"value":"modded-runtime-id","label":"Runtime name"})]
                );
                assert!(super::super::option_sources::map_option_items(
                    &json!({root:[]}),
                    Some(root),
                    Some("id"),
                    Some("name")
                )
                .unwrap()
                .is_empty());
            }
        }
    }
}

#[cfg(feature = "http")]
#[tokio::test]
async fn repo_bursts_reject_excess_work_without_queueing() {
    let _guard = REPO_EXECUTION_TEST_LOCK.lock().await;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut bytes = [0; 4096];
                let _ = socket.read(&mut bytes).await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}",
                    )
                    .await;
            });
        }
    });
    let core = Arc::new(crate::AppCore::new(Arc::new(Emitter)));
    let mut document: Value = serde_json::from_str(include_str!(
        "../../../../../examples/repo-game/plugin.json"
    ))
    .unwrap();
    document["http"]["baseUrl"] = json!(format!("http://{address}"));
    let manifest = PluginManifest::from_json_str(&document.to_string()).unwrap();
    let plugin = tiktools_plugin_loader::DiscoveredPlugin {
        manifest,
        directory: std::env::temp_dir(),
        source: tiktools_plugin_loader::PluginSource::Development,
        available: true,
        reason: None,
        running: true,
    };
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..32 {
        let core = core.clone();
        let plugin = plugin.clone();
        tasks.spawn(async move { core.execute_declarative_action(&plugin,
            &json!({"method":"POST","path":"/api/enemies/clear","bodyJson":{},"idempotency":{"enabled":true}}),
            &json!({}), &json!({}), &mut Vec::new(), false).await });
    }
    let mut accepted = 0;
    let mut rejected = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            Ok(_) => accepted += 1,
            Err(error) => {
                assert!(error.contains("limit"), "{error}");
                rejected += 1;
            }
        }
    }
    assert!(accepted <= 4);
    assert!(rejected >= 28);
    server.abort();
}

#[cfg(feature = "http")]
static REPO_EXECUTION_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(feature = "http")]
#[tokio::test]
async fn health_reports_api_reachability_separately_from_gameplay_readiness() {
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }
    let (address,server)=stub_server([
        ("/not-ready".to_owned(),(200,json!({"status":"ready","gameRunning":true,"inLevel":false,"isHost":true,"gameplayReady":false,"apiVersion":"1.0.0"}).to_string())),
        ("/ready".to_owned(),(200,json!({"status":"ready","gameRunning":true,"inLevel":true,"isHost":true,"gameplayReady":true,"apiVersion":"1.0.0"}).to_string())),
    ].into()).await;
    let root = std::env::temp_dir().join(format!("health-readiness-{}", fastrand::u64(..)));
    for (id, path) in [("repo.not-ready", "/not-ready"), ("repo.ready", "/ready")] {
        std::fs::create_dir_all(root.join(id)).unwrap();
        let mut document: Value = serde_json::from_str(include_str!(
            "../../../../../examples/repo-game/plugin.json"
        ))
        .unwrap();
        document["id"] = json!(id);
        document["actionTypes"] = json!([]);
        document["pages"] = json!([]);
        document["http"]["baseUrl"] = json!(format!("http://{address}"));
        document["http"]["health"]["path"] = json!(path);
        std::fs::write(root.join(id).join("plugin.json"), document.to_string()).unwrap();
    }
    let manager =
        tiktools_plugin_loader::PluginManager::new(vec![tiktools_plugin_loader::PluginRoot {
            path: root.clone(),
            source: tiktools_plugin_loader::PluginSource::Development,
        }]);
    manager.scan().unwrap();
    let mut core = crate::AppCore::new(Arc::new(Emitter));
    core.plugins = Arc::new(manager);
    let core = Arc::new(core);
    let unavailable = core
        .plugin_connection_check("repo.not-ready")
        .await
        .unwrap();
    assert!(unavailable.reachable);
    assert!(!unavailable.ok);
    assert_eq!(unavailable.gameplay_ready, Some(false));
    let ready = core.plugin_connection_check("repo.ready").await.unwrap();
    assert!(ready.reachable && ready.ok);
    assert_eq!(ready.gameplay_ready, Some(true));
    server.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(feature = "http")]
#[tokio::test]
async fn sonicboom_discovery_separates_authorization_from_model_readiness() {
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }
    for (required, status, provider_http, expected_ok) in [
        (false, "downloading", 200, true),
        (false, "loading", 200, true),
        (false, "failed", 200, true),
        (true, "ready", 401, false),
        (true, "loading", 200, true),
        (false, "ready", 403, false),
    ] {
        let info = json!({"auth":{"required":required}, "setup_complete":true,
            "model":{"status":status}, "features":{"playback":true}});
        let (address, server) = stub_server(
            [
                ("/api/info".to_owned(), (200, info.to_string())),
                (
                    "/api/tts/plugin/providers".to_owned(),
                    (provider_http, "[]".to_owned()),
                ),
            ]
            .into(),
        )
        .await;
        let root = std::env::temp_dir().join(format!("sonicboom-discovery-{}", fastrand::u64(..)));
        std::fs::create_dir_all(root.join("sonicboom.server")).unwrap();
        let document = json!({"schemaVersion":3, "id":"sonicboom.server", "name":"SonicBoom",
            "version":"1.0.0", "runtime":"declarative", "capabilities":["http.request"],
            "http":{"baseUrl":format!("http://{address}"), "auth":{"type":"bearer","tokenSetting":"apiToken"},
                "health":{"strategy":"sonicboom","path":"/api/info"}}});
        std::fs::write(
            root.join("sonicboom.server/plugin.json"),
            document.to_string(),
        )
        .unwrap();
        let manager =
            tiktools_plugin_loader::PluginManager::new(vec![tiktools_plugin_loader::PluginRoot {
                path: root.clone(),
                source: tiktools_plugin_loader::PluginSource::Development,
            }]);
        manager.scan().unwrap();
        let mut core = crate::AppCore::new(Arc::new(Emitter));
        core.plugins = Arc::new(manager);
        let check = Arc::new(core)
            .plugin_connection_check("sonicboom.server")
            .await
            .unwrap();
        assert!(check.reachable);
        assert_eq!(check.ok, expected_ok);
        let server_info = check.server_info.unwrap();
        assert_eq!(server_info["authRequired"], required);
        assert_eq!(server_info["authorized"], expected_ok);
        assert_eq!(server_info["modelStatus"], status);
        assert!(check.gameplay_ready.is_none());
        if !expected_ok {
            assert!(check.error.unwrap().contains(if required {
                "Sign in"
            } else {
                "local access"
            }));
        }
        server.abort();
        let _ = std::fs::remove_dir_all(root);
    }
}
