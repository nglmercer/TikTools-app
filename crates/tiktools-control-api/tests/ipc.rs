//! Real host/client behavior over local IPC (§15) and the one-runtime
//! ownership proof (§17): two clients observe the same `AppCore` state.
//!
//! Both tests isolate `TIKTOOLS_HOME` (plus `TIKTOOLS_IPC_NAME` on Windows)
//! and serialize on a process-local mutex because the home is resolved
//! from process-global environment.

use std::{sync::Arc, time::Duration};

use serde_json::{json, Value};
use tiktools_control_api::{ControlApi, ControlClient};
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

// Async-aware: the guard intentionally spans awaits so the isolated home
// stays stable for the whole test.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct NullEmitter;

impl HostEmitter for NullEmitter {
    fn emit(&self, _message: HostMessage) {}
}

async fn lock_env() -> tokio::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().await
}

/// Points the process at a fresh isolated home. The caller must hold the
/// env lock for the whole test.
fn isolated_home(tag: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let home = std::env::temp_dir().join(format!(
        "tiktools-ipc-test-{}-{}-{tag}",
        std::process::id(),
        unique
    ));
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var(
        "TIKTOOLS_IPC_NAME",
        format!("tiktools-test-{}-{tag}", std::process::id()),
    );
    home
}

async fn within<F, T>(what: &str, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(Duration::from_secs(30), future)
        .await
        .unwrap_or_else(|_| panic!("{what} timed out"))
}

async fn connect_retry() -> ControlClient {
    for _ in 0..100 {
        if let Ok(client) = ControlClient::connect().await {
            return client;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("could not connect to the test IPC host");
}

#[tokio::test]
async fn ipc_roundtrip_serves_discover_and_domain_methods() {
    let _env = lock_env().await;
    let home = isolated_home("roundtrip");
    let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let server = tokio::spawn({
        let api = Arc::new(ControlApi::new(core.clone()));
        async move { tiktools_control_api::run_ipc_shared(api).await }
    });

    let client = within("connect", connect_retry()).await;

    let discovered: Value = within("rpc.discover", client.call_value("rpc.discover", json!({})))
        .await
        .expect("rpc.discover works");
    let methods = discovered
        .get("methods")
        .and_then(Value::as_array)
        .expect("discover returns a methods array");
    for expected in [
        "rpc.discover",
        "plugins.list",
        "points.adjust",
        "system.snapshot",
        "system.shutdown",
        "workflows.list",
        "media.pick",
    ] {
        assert!(
            methods
                .iter()
                .any(|method| method.get("name").and_then(Value::as_str) == Some(expected)),
            "discover lists {expected}"
        );
    }

    let plugins: Value = within("plugins.list", client.call_value("plugins.list", json!({})))
        .await
        .expect("plugins.list works");
    assert!(
        plugins.get("plugins").and_then(Value::as_array).is_some(),
        "unexpected plugins.list shape: {plugins}"
    );

    let award: Value = within(
        "points.adjust",
        client.call_value("points.adjust", json!({"uniqueId": "alice", "delta": 10.0})),
    )
    .await
    .expect("points.adjust works");
    assert_eq!(
        award.get("totalPoints").and_then(Value::as_f64),
        Some(10.0),
        "unexpected award shape: {award}"
    );

    // Desktop-only capabilities refuse on a headless host instead of
    // being emulated.
    let pick = within("media.pick", client.call_value("media.pick", json!({}))).await;
    assert_eq!(
        pick.as_ref().err().map(|error| error.code.as_str()),
        Some("capability_unavailable"),
        "headless media.pick must refuse: {pick:?}"
    );

    let snapshot: Value = within(
        "system.snapshot",
        client.call_value("system.snapshot", json!({})),
    )
    .await
    .expect("system.snapshot works");
    assert!(
        snapshot.get("live").is_some() && snapshot.get("health").is_some(),
        "unexpected snapshot shape: {snapshot}"
    );
    assert!(
        !snapshot
            .to_string()
            .to_ascii_lowercase()
            .contains("sessioncookie"),
        "snapshot must stay secret-safe"
    );

    let _shutdown: Value = within(
        "system.shutdown",
        client.call_value("system.shutdown", json!({})),
    )
    .await
    .expect("system.shutdown works");
    assert!(core.is_shutdown(), "shutdown reaches the core");
    within("server exit", server)
        .await
        .expect("server task panicked")
        .expect("server task failed");

    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn two_clients_share_one_runtime_state() {
    let _env = lock_env().await;
    let home = isolated_home("ownership");
    let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let server = tokio::spawn({
        let api = Arc::new(ControlApi::new(core.clone()));
        async move { tiktools_control_api::run_ipc_shared(api).await }
    });

    let mut first = within("connect", connect_retry()).await;
    // The Windows pipe server accepts one instance at a time, so the
    // second connection also retries until the next instance exists.
    let mut second = within("second connect", connect_retry()).await;

    // Mutate through client 1 only; there is exactly one AppCore, so both
    // clients must observe the write.
    let award: Value = within(
        "points.adjust",
        first.call_value("points.adjust", json!({"uniqueId": "alice", "delta": 10.0})),
    )
    .await
    .expect("points.adjust works");
    assert_eq!(
        award.get("totalPoints").and_then(Value::as_f64),
        Some(10.0),
        "unexpected award shape: {award}"
    );

    for (name, client) in [("first", &mut first), ("second", &mut second)] {
        let viewer: Value = within(
            "points.viewer.get",
            client.call_value("points.viewer.get", json!({"uniqueId": "alice"})),
        )
        .await
        .unwrap_or_else(|error| panic!("{name} client reads the viewer: {error}"));
        assert_eq!(
            viewer
                .get("viewer")
                .and_then(|viewer| viewer.get("points"))
                .and_then(Value::as_f64),
            Some(10.0),
            "{name} client sees the shared state: {viewer}"
        );
    }

    let _shutdown: Value = within(
        "system.shutdown",
        first.call_value("system.shutdown", json!({})),
    )
    .await
    .expect("system.shutdown works");
    within("server exit", server)
        .await
        .expect("server task panicked")
        .expect("server task failed");

    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn concurrent_calls_share_one_connection() {
    let _env = lock_env().await;
    let home = isolated_home("concurrent");
    let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let server = tokio::spawn({
        let api = Arc::new(ControlApi::new(core.clone()));
        async move { tiktools_control_api::run_ipc_shared(api).await }
    });

    let client = within("connect", connect_retry()).await;
    // Ten overlapping RPCs on one connection: every id must resolve to
    // its own response, never to a sibling's.
    let mut tasks = Vec::new();
    for index in 0..10 {
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            within(
                "points.adjust",
                client.call_value(
                    "points.adjust",
                    json!({"uniqueId": format!("viewer-{index}"), "delta": 1.0}),
                ),
            )
            .await
            .expect("concurrent call works")
        }));
    }
    let mut seen = Vec::new();
    for task in tasks {
        let award = within("join", task).await.expect("task panicked");
        seen.push(
            award
                .get("uniqueId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        );
    }
    seen.sort();
    assert_eq!(seen.len(), 10, "every concurrent call resolves: {seen:?}");

    let _shutdown: Value = within(
        "system.shutdown",
        client.call_value("system.shutdown", json!({})),
    )
    .await
    .expect("system.shutdown works");
    within("server exit", server)
        .await
        .expect("server task panicked")
        .expect("server task failed");

    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn event_subscription_receives_domain_events_without_blocking_rpc() {
    let _env = lock_env().await;
    let home = isolated_home("events");
    let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let server = tokio::spawn({
        let api = Arc::new(ControlApi::new(core.clone()));
        async move { tiktools_control_api::run_ipc_shared(api).await }
    });

    let client = within("connect", connect_retry()).await;
    let mut events = client.subscribe();

    // A state mutation publishes `points.changed`; the subscriber must see
    // it while RPC responses keep flowing on the same connection.
    let award: Value = within(
        "points.adjust",
        client.call_value("points.adjust", json!({"uniqueId": "bob", "delta": 5.0})),
    )
    .await
    .expect("points.adjust works");
    assert_eq!(
        award.get("totalPoints").and_then(Value::as_f64),
        Some(5.0),
        "unexpected award shape: {award}"
    );
    let event = within("event recv", events.recv())
        .await
        .expect("event channel open");
    assert_eq!(
        event.topic(),
        "points.changed",
        "unexpected event: {event:?}"
    );

    // RPC still works after events interleaved on the stream.
    let ping: Value = within("system.ping", client.call_value("system.ping", json!({})))
        .await
        .expect("ping after events works");
    assert_eq!(ping.get("ok"), Some(&Value::Bool(true)));

    let _shutdown: Value = within(
        "system.shutdown",
        client.call_value("system.shutdown", json!({})),
    )
    .await
    .expect("system.shutdown works");
    within("server exit", server)
        .await
        .expect("server task panicked")
        .expect("server task failed");

    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn processors_status_returns_typed_shape() {
    let _env = lock_env().await;
    let home = isolated_home("processors");
    let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let server = tokio::spawn({
        let api = Arc::new(ControlApi::new(core.clone()));
        async move { tiktools_control_api::run_ipc_shared(api).await }
    });

    let client = within("connect", connect_retry()).await;
    // Exact contract: `{ processors: [...] }`, never a bare array.
    let status: Value = within(
        "processors.status",
        client.call_value("processors.status", json!({})),
    )
    .await
    .expect("processors.status works");
    let processors = status.get("processors").and_then(Value::as_array);
    assert!(
        processors.is_some(),
        "processors.status must be {{ processors: [...] }}: {status}"
    );

    let _shutdown: Value = within(
        "system.shutdown",
        client.call_value("system.shutdown", json!({})),
    )
    .await
    .expect("system.shutdown works");
    within("server exit", server)
        .await
        .expect("server task panicked")
        .expect("server task failed");

    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn second_owner_fails_while_first_holds_endpoint() {
    let _env = lock_env().await;
    let home = isolated_home("owner");
    let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let first = tokio::spawn({
        let api = Arc::new(ControlApi::new(core.clone()));
        async move { tiktools_control_api::run_ipc_shared(api).await }
    });
    let _client = within("connect", connect_retry()).await;

    // A second server on the same endpoint must fail instead of stealing
    // or sharing it. Unix enforces this at bind; Windows at the mutex.
    let second_core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let second = tiktools_control_api::run_ipc_shared(Arc::new(ControlApi::new(second_core))).await;
    assert!(
        second.is_err(),
        "second control host must not own the endpoint"
    );

    core.shutdown().await;
    within("server exit", first)
        .await
        .expect("server task panicked")
        .expect("server task failed");

    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn connect_to_idle_endpoint_returns_host_unavailable() {
    let _env = lock_env().await;
    let home = isolated_home("idle");
    // No server is started: after the bounded retry budget the client must
    // report `host_unavailable`, never hang or invent a runtime.
    let error = match ControlClient::connect().await {
        Ok(_) => panic!("idle endpoint must refuse"),
        Err(error) => error,
    };
    assert_eq!(error.code, "host_unavailable", "unexpected code: {error:?}");

    let _ = std::fs::remove_dir_all(&home);
}
