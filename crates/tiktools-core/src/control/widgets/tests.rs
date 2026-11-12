#[test]
fn copied_design_preserves_text_templates_and_hidden_lines() {
    let result = super::portable_design(
        r#"{"text":{"title":"","message":"Thanks {{name}}!","unknown":"drop"}}"#,
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"text":{"title":"","message":"Thanks {{name}}!"}})
    );
}
#[test]
fn copied_design_only_contains_portable_style_tokens() {
    let result = super::portable_design(
        r##"{"background":"#112233aa","accent":"url(evil)","radius":99,"token":"secret"}"##,
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"background": "#112233aa", "radius": 48.0})
    );
    assert!(super::portable_design("broken").is_none());
    assert!(super::portable_design("{}").is_none());
}
#[test]
fn copied_design_covers_layout_avatar_badge_and_hidden_text() {
    let result = super::portable_design(
        r##"{"hiddenText":["title","name","title","evil"],"borderColor":"#2a2a33","borderWidth":99,"shadow":false,"opacity":80,"padding":24,"gap":12,"align":"center","width":100,"avatar":{"visible":false,"size":64,"radius":12,"borderColor":"#ffffff","borderWidth":3,"evil":1},"badge":{"color":"#00dce8","fontSize":14,"fontWeight":750,"letterSpacing":2},"align2":"drop"}"##,
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "hiddenText": ["title", "name"],
            "borderColor": "#2a2a33",
            "borderWidth": 8.0,
            "shadow": false,
            "opacity": 80.0,
            "padding": 24.0,
            "gap": 12.0,
            "align": "center",
            "width": 240.0,
            "avatar": {"visible": false, "size": 64.0, "radius": 12.0, "borderColor": "#ffffff", "borderWidth": 3.0},
            "badge": {"color": "#00dce8", "fontSize": 14.0, "fontWeight": 800.0, "letterSpacing": 2.0},
        })
    );
    assert!(super::portable_design(r#"{"hiddenText":[]}"#).is_none());
    assert!(super::portable_design(r#"{"hiddenText":"title"}"#).is_none());
    assert!(super::portable_design(r#"{"align":"diagonal"}"#).is_none());
}
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tiktools_plugin_loader::{PluginManager, PluginRoot, PluginSource};

struct NullEmitter;

impl crate::HostEmitter for NullEmitter {
    fn emit(&self, _message: crate::HostMessage) {}
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "tiktools-widgets-test-{label}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ))
}

fn gateway_manifest() -> String {
    r#"{"schemaVersion": 3, "id": "tiktools.event-gateway", "name": "Event Gateway", "version": "1.0.0", "runtime": "declarative", "capabilities": [], "permissions": []}"#.to_owned()
}

/// AppCore with a discovered (but never running) gateway plugin and a
/// temp-rooted capability broker. The caller writes settings files
/// under `<root>/tiktools.event-gateway/settings.json`.
fn core_with_gateway() -> (AppCore, std::path::PathBuf) {
    let root = temp_root("core");
    let package = root.join("packages").join("gateway");
    std::fs::create_dir_all(&package).expect("test plugin dir");
    std::fs::write(package.join("plugin.json"), gateway_manifest()).expect("manifest");
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.join("packages"),
        source: PluginSource::Development,
    }]);
    manager.scan().expect("test scan");
    assert!(manager.get(GATEWAY_PLUGIN_ID).is_some());
    let mut core = AppCore::new(Arc::new(NullEmitter));
    core.plugins = Arc::new(manager);
    core.capabilities = Arc::new(crate::services::CapabilityBroker::new(root.join("data")));
    (core, root)
}

fn write_settings(root: &std::path::Path, values: &Value) {
    let dir = root.join("data").join(GATEWAY_PLUGIN_ID);
    std::fs::create_dir_all(&dir).expect("settings dir");
    std::fs::write(
        dir.join("settings.json"),
        serde_json::to_vec(values).unwrap(),
    )
    .expect("settings file");
}

#[test]
fn state_resolution_covers_every_widgets_tab_state() {
    use WidgetsState::*;
    // (installed, enabled, available, running, credential, probe) -> state.
    assert_eq!(
        AppCore::resolve_widgets_state(false, true, true, true, true, true, None),
        Missing
    );
    // Present in the catalog but install-flagged off reads as missing.
    assert_eq!(
        AppCore::resolve_widgets_state(true, false, true, true, false, false, None),
        Missing
    );
    assert_eq!(
        AppCore::resolve_widgets_state(true, true, false, true, false, false, None),
        Disabled
    );
    assert_eq!(
        AppCore::resolve_widgets_state(true, true, true, false, false, false, None),
        Disabled
    );
    assert_eq!(
        AppCore::resolve_widgets_state(true, true, true, true, false, false, None),
        Stopped
    );
    // A running gateway without a stored credential cannot mint URLs.
    assert_eq!(
        AppCore::resolve_widgets_state(true, true, true, true, true, false, None),
        CredentialUnavailable
    );
    assert_eq!(
        AppCore::resolve_widgets_state(
            true,
            true,
            true,
            true,
            true,
            true,
            Some(ProbeOutcome::NotListening)
        ),
        Starting
    );
    assert_eq!(
        AppCore::resolve_widgets_state(
            true,
            true,
            true,
            true,
            true,
            true,
            Some(ProbeOutcome::BadGateway)
        ),
        Unreachable
    );
    assert_eq!(
        AppCore::resolve_widgets_state(
            true,
            true,
            true,
            true,
            true,
            true,
            Some(ProbeOutcome::AssetsMissing)
        ),
        AssetsMissing
    );
    assert_eq!(
        AppCore::resolve_widgets_state(
            true,
            true,
            true,
            true,
            true,
            true,
            Some(ProbeOutcome::Ready)
        ),
        Ready
    );
}

#[test]
fn widget_credential_rejects_placeholders_and_blanks() {
    let (core, root) = core_with_gateway();
    for (values, expected) in [
        (serde_json::json!({}), None),
        (serde_json::json!({"widgetToken": ""}), None),
        (serde_json::json!({"widgetToken": "   "}), None),
        // The redaction placeholder is never a valid credential, even
        // if it somehow ends up stored.
        (
            serde_json::json!({"widgetToken": SECRET_SETTING_PLACEHOLDER}),
            None,
        ),
        (serde_json::json!({"widgetToken": "x".repeat(4097)}), None),
        (
            serde_json::json!({"widgetToken": "  ttw_widget  "}),
            Some("ttw_widget".to_owned()),
        ),
    ] {
        assert_eq!(core.gateway_widget_credential(&values), expected);
    }
    assert_eq!(
        core.gateway_port(&serde_json::json!({})),
        GATEWAY_DEFAULT_PORT
    );
    assert_eq!(
        core.gateway_port(&serde_json::json!({"port": 19999})),
        19999
    );
    assert_eq!(
        core.gateway_port(&serde_json::json!({"port": 0})),
        GATEWAY_DEFAULT_PORT
    );
    assert_eq!(
        core.gateway_port(&serde_json::json!({"port": "bad"})),
        GATEWAY_DEFAULT_PORT
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn copy_rejects_bad_widgets_and_missing_credentials_before_clipboard() {
    let (core, root) = core_with_gateway();
    // Unknown widget kinds fail without touching settings or clipboard.
    assert!(core.widgets_copy_obs_url("overlay").is_err());
    assert!(core.widgets_copy_obs_url("").is_err());
    // No stored credential: unavailable, never a placeholder URL.
    write_settings(&root, &serde_json::json!({"port": 17452}));
    let error = core.widgets_copy_obs_url("follow").unwrap_err();
    assert!(error.to_string().contains("credential"));
    assert!(!error.to_string().contains(SECRET_SETTING_PLACEHOLDER));
    write_settings(
        &root,
        &serde_json::json!({"widgetToken": SECRET_SETTING_PLACEHOLDER}),
    );
    assert!(core.widgets_copy_obs_url("gift").is_err());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn copy_with_stored_credential_needs_a_display() {
    let (core, root) = core_with_gateway();
    write_settings(
        &root,
        &serde_json::json!({"widgetToken": "ttw_test_credential"}),
    );
    let result = core.widgets_copy_obs_url("follow");
    if std::env::var_os("TIKTOOLS_TEST_CLIPBOARD").is_none() {
        // Headless CI has no clipboard: the op must fail gracefully
        // (typed error, no panic, no secret in the message), and must
        // not write to any real clipboard. Set
        // TIKTOOLS_TEST_CLIPBOARD=1 to assert the round-trip instead.
        if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
            let error = result.unwrap_err();
            assert!(error.to_string().contains("clipboard"));
            assert!(!error.to_string().contains("ttw_test_credential"));
        }
    } else {
        result.expect("clipboard round-trip");
        #[cfg(not(target_os = "android"))]
        {
            let mut clipboard = arboard::Clipboard::new().expect("clipboard");
            let pasted = clipboard.get_text().expect("pasted text");
            assert!(pasted.contains("/widgets/follow/#token="));
        }
    }
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn status_reports_lifecycle_states_without_a_process() {
    let (core, root) = core_with_gateway();
    // Discovered, default activation, never running: stopped, with the
    // configured (or default) port echoed for the UI.
    write_settings(&root, &serde_json::json!({"port": 19999}));
    let status = block_on_current_thread(core.widgets_status()).unwrap();
    assert_eq!(status.state, WidgetsState::Stopped);
    assert_eq!(status.port, 19999);
    // Disabled plugin: disabled, no probe attempted.
    core.set_plugin_activation(GATEWAY_PLUGIN_ID, true, false);
    let status = block_on_current_thread(core.widgets_status()).unwrap();
    assert_eq!(status.state, WidgetsState::Disabled);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn status_reports_missing_for_unknown_plugins() {
    // An empty plugin root keeps the test hermetic: the
    // operator's real plugin directory can carry an installed
    // gateway, which would resolve to Stopped instead of
    // Missing on any machine that has run the app.
    let root = temp_root("missing");
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.join("packages"),
        source: PluginSource::Development,
    }]);
    manager.scan().expect("test scan");
    let mut core = AppCore::new(Arc::new(NullEmitter));
    core.plugins = Arc::new(manager);
    let status = block_on_current_thread(core.widgets_status()).unwrap();
    assert_eq!(status.state, WidgetsState::Missing);
    assert_eq!(status.port, GATEWAY_DEFAULT_PORT);
}

#[test]
fn fragment_encoding_matches_browser_unreserved_set() {
    assert_eq!(percent_encode_fragment("ttw_abc123"), "ttw_abc123");
    assert_eq!(percent_encode_fragment("a b+c"), "a%20b%2Bc");
    assert_eq!(percent_encode_fragment("/#?"), "%2F%23%3F");
    assert_eq!(percent_encode_fragment(""), "");
}

#[test]
fn status_line_parsing_rejects_garbage() {
    assert_eq!(parse_http_status(b"HTTP/1.1 200 OK\r\n"), Some(200));
    assert_eq!(parse_http_status(b"HTTP/1.0 404 Not Found\n"), Some(404));
    assert_eq!(parse_http_status(b""), None);
    assert_eq!(parse_http_status(b"not http at all"), None);
    assert_eq!(parse_http_status(b"HTTP/1.1 nope\r\n"), None);
    assert_eq!(parse_http_status(b"\xff\xfe binary"), None);
}

/// Minimal single-threaded block_on so status/probe tests need no
/// runtime fixture. The futures only touch loopback TCP.
fn block_on_current_thread<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime")
        .block_on(future)
}

/// Serves canned `(path-prefix, status)` answers on loopback until
/// `hits` connections complete. Returns the bound port.
fn serve_canned(responses: Vec<(&'static str, u16)>, hits: usize) -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let port = listener.local_addr().expect("port").port();
    std::thread::spawn(move || {
        listener.set_nonblocking(false).expect("blocking listener");
        for _ in 0..hits {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            use std::io::{Read, Write};
            let mut request = vec![0u8; 4096];
            let Ok(read) = stream.read(&mut request) else {
                continue;
            };
            let head = String::from_utf8_lossy(&request[..read]);
            let path = head.split_whitespace().nth(1).unwrap_or("/");
            let status = responses
                .iter()
                .find(|(prefix, _)| path.starts_with(prefix))
                .map(|(_, status)| *status)
                .unwrap_or(404);
            let reason = if status == 200 { "OK" } else { "Not Found" };
            let body = if status == 200 { "ok" } else { "missing" };
            let _ = write!(
                stream,
                "HTTP/1.0 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    port
}

#[test]
fn probe_classifies_listeners_and_refusals() {
    // Nothing listening: refused fast (TEST-NET port stays closed).
    let outcome = block_on_current_thread(probe_gateway("127.0.0.1".parse().unwrap(), 9));
    assert_eq!(outcome, ProbeOutcome::NotListening);
    // Healthy gateway shape: /health plus every bundle answers 200.
    let port = serve_canned(
        vec![("/health", 200), ("/widgets/", 200)],
        1 + VALID_WIDGETS.len(),
    );
    let outcome = block_on_current_thread(probe_gateway("127.0.0.1".parse().unwrap(), port));
    assert_eq!(outcome, ProbeOutcome::Ready);
    // Health OK but bundles missing: assets missing, not unreachable.
    let port = serve_canned(vec![("/health", 200)], 2);
    let outcome = block_on_current_thread(probe_gateway("127.0.0.1".parse().unwrap(), port));
    assert_eq!(outcome, ProbeOutcome::AssetsMissing);
    // Wrong answers on a live socket: bad gateway, not starting.
    let port = serve_canned(vec![("/health", 500)], 1);
    let outcome = block_on_current_thread(probe_gateway("127.0.0.1".parse().unwrap(), port));
    assert_eq!(outcome, ProbeOutcome::BadGateway);
}
