use super::*;
#[test]
fn aggregate_subscription_limit_is_atomic_across_requests() {
    let mut topics = HashSet::new();
    apply_topics(
        &mut topics,
        crate::event::PluginUiSubscription::Subscribe(
            (0..32).map(|n| format!("plugin.{n}")).collect(),
        ),
    )
    .unwrap();
    assert!(apply_topics(
        &mut topics,
        crate::event::PluginUiSubscription::Subscribe(vec!["plugin.more".to_owned()])
    )
    .is_err());
    assert_eq!(topics.len(), 32);
    apply_topics(
        &mut topics,
        crate::event::PluginUiSubscription::Unsubscribe(vec!["plugin.0".to_owned()]),
    )
    .unwrap();
    apply_topics(
        &mut topics,
        crate::event::PluginUiSubscription::Subscribe(vec!["plugin.more".to_owned()]),
    )
    .unwrap();
    assert_eq!(topics.len(), 32);
}
#[test]
fn deferred_close_is_fenced_even_if_a_native_window_id_is_reused() {
    let id = WindowId::from(1);
    assert!(close_owner_matches(Some(id), Some(5), id, 5));
    assert!(!close_owner_matches(Some(id), Some(5), id, 6));
    assert!(close_owner_matches(None, None, id, 6));
}

#[test]
fn ui_ids_follow_the_identifier_rule() {
    assert!(is_valid_ui_id("sonicboom.server"));
    assert!(is_valid_ui_id("plugin-a_b.c"));
    assert!(!is_valid_ui_id(""));
    assert!(!is_valid_ui_id("9lives"));
    assert!(!is_valid_ui_id("../escape"));
    assert!(!is_valid_ui_id("has space"));
    assert!(!is_valid_ui_id(&"x".repeat(129)));
}

#[test]
fn host_notifications_route_by_plugin_topic() {
    let message = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "event",
        "params": {
            "topic": "plugin.event",
            "data": { "pluginId": "owner", "eventType": "speech.state", "event": {} },
        },
    })
    .to_string();
    let (plugin_id, topic, data) = parse_host_notification(&message).expect("routes");
    assert_eq!(plugin_id, "owner");
    assert_eq!(topic, "plugin.event");
    assert_eq!(data["eventType"], "speech.state");

    // Non-plugin topics never reach plugin windows.
    let other = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "event",
        "params": { "topic": "live.event", "data": { "pluginId": "owner" } },
    })
    .to_string();
    assert!(parse_host_notification(&other).is_none());
    // RPC responses and malformed input are ignored.
    assert!(parse_host_notification(r#"{"type":"rpc-response"}"#).is_none());
    assert!(parse_host_notification("not json").is_none());
}

#[test]
fn teardown_topics_close_windows_without_affecting_delivery() {
    assert!(is_teardown_topic("plugin.uninstalled"));
    assert!(is_teardown_topic("plugin.stopped"));
    assert!(!is_teardown_topic("plugin.started"));
    assert!(!is_teardown_topic("plugin.event"));
    assert!(!is_teardown_topic("plugin.settings-changed"));
    // Forwarding without windows is a no-op, never a panic.
    let mut windows = PluginUiWindows::default();
    windows.forward_host_message(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": {
                "topic": "plugin.uninstalled",
                "data": {"pluginId": "gone.plugin"},
            },
        })
        .to_string(),
    );
}

#[test]
fn init_script_forwards_refresh_and_devtools_shortcuts() {
    // F5 / Ctrl+R reload in-page; F12 / Ctrl+Shift+I ask the host to
    // open devtools. The reload path needs no transport; the devtools
    // path reuses the captured `ipc` handle so the broker allowlist is
    // never involved.
    assert!(INIT_SCRIPT.contains("F5"));
    assert!(INIT_SCRIPT.contains("F12"));
    assert!(INIT_SCRIPT.contains("location.reload"));
    assert!(INIT_SCRIPT.contains("open-devtools"));
}

#[test]
fn init_script_installs_the_narrow_surface_only() {
    // The transport is read but never deleted or overwritten: on
    // WebKitGTK `window.ipc` is a non-configurable property installed
    // before init scripts, so touching it throws in strict mode and
    // aborts the whole script (leaving `window.tiktools` missing).
    assert!(!INIT_SCRIPT.contains("delete window.ipc"));
    assert!(!INIT_SCRIPT.contains("window.ipc = "));
    assert!(!INIT_SCRIPT.contains("window.ipc= "));
    assert!(INIT_SCRIPT.contains("window.ipc && typeof window.ipc.postMessage"));
    assert!(INIT_SCRIPT.contains("window.tiktools = {"));
    assert!(INIT_SCRIPT.contains("apiVersion: 1"));
    assert!(INIT_SCRIPT.contains("__tiktools_broker_push__"));
    // Transport diagnostics for native debugging (see also the Rust
    // `transport = "native"` open log and broker rejection logs).
    assert!(INIT_SCRIPT.contains("__tiktools_transport__"));
    assert!(INIT_SCRIPT.contains("'native'"));
    assert!(INIT_SCRIPT.contains("'unavailable'"));
}

#[test]
fn teardown_plan_destroys_the_child_before_the_parent() {
    // Pure pin of the destruction order: the child WebView dies
    // first, the display sync flushes its destroy, and only then is
    // the parent window handle destroyed. This orders the Rust calls;
    // it cannot prove the X11/GTK behavior, which the native smoke
    // test (`smoke::native_open_close_cycles`) exercises for real.
    assert_eq!(
        teardown_plan(false),
        [
            TeardownStep::DropWebView,
            TeardownStep::SyncDisplay,
            TeardownStep::DropWindow,
        ]
    );
    // A server-destroyed parent is forgotten, never destroyed twice —
    // but the WebView still drops first with the same sync.
    assert_eq!(
        teardown_plan(true),
        [
            TeardownStep::DropWebView,
            TeardownStep::SyncDisplay,
            TeardownStep::ForgetWindow,
        ]
    );
}

#[test]
fn close_requests_queue_exactly_one_teardown() {
    let mut state = PluginWindowState::Open;
    assert!(state.begin_close());
    assert_eq!(state, PluginWindowState::Closing);
    // A duplicate request while closing queues nothing.
    assert!(!state.begin_close());
    assert_eq!(state, PluginWindowState::Closing);
}

#[test]
fn manager_close_paths_are_harmless_without_windows() {
    let mut windows = PluginUiWindows::default();
    // Double closes, missing windows, and late broker traffic never
    // panic, recreate entries, or evaluate script anywhere.
    windows.close("ghost.plugin", "main");
    windows.close("ghost.plugin", "main");
    windows.close_plugin("ghost.plugin");
    windows.close_plugin("ghost.plugin");
    windows.respond(
        "ghost.plugin",
        "main",
        r#"{"ok":true}"#.into(),
        WindowId::from(0),
        0,
        None,
    );
    windows.apply_subscription(
        "ghost.plugin",
        "main",
        crate::event::PluginUiSubscription::Subscribe(vec!["plugin.event".to_owned()]),
        WindowId::from(0),
        0,
    );
    windows.deliver_event("ghost.plugin", "plugin.event", &Value::Null);
    windows.forward_host_message(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": {
                "topic": "plugin.stopped",
                "data": {"pluginId": "ghost.plugin"},
            },
        })
        .to_string(),
    );
    assert!(windows.windows.is_empty());
}
