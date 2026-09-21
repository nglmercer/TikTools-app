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
