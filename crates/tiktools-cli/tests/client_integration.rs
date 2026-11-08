//! End-to-end typed-client tests over real transports.
//!
//! Connected tests spawn `tiktools host --ipc` under an isolated
//! `TIKTOOLS_HOME` (the IPC socket itself is home-scoped, so the test
//! process shares the same home) and verify the answering host is really
//! the child via `system.info` paths. Direct tests build an in-process
//! `ControlApi` the same way. All tests serialize on the process-wide
//! home override, and no test touches real user data.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tiktools_client::{ControlEvent, TikToolsClient};
use tiktools_control_api::modules::points::PointsAdjustParams;
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

fn unique_tag(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    format!("tiktools-test-{}-{}-{nanos}", std::process::id(), label)
}

/// Guards the process-wide `TIKTOOLS_HOME` override so parallel tests
/// never observe each other's home.
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Holds the env lock, points `TIKTOOLS_HOME` at a fresh temp home with
/// a unique Windows pipe name, and restores the previous values plus
/// removes the directory on drop.
struct HomeGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    path: PathBuf,
    previous_home: Option<std::ffi::OsString>,
    previous_ipc: Option<std::ffi::OsString>,
}

impl HomeGuard {
    fn set(label: &str) -> Self {
        let lock = env_lock().lock().expect("env lock");
        let path = std::env::temp_dir().join(unique_tag(label));
        std::fs::create_dir_all(&path).expect("create isolated home");
        let previous_home = std::env::var_os("TIKTOOLS_HOME");
        let previous_ipc = std::env::var_os("TIKTOOLS_IPC_NAME");
        std::env::set_var("TIKTOOLS_HOME", &path);
        std::env::set_var(
            "TIKTOOLS_IPC_NAME",
            format!("tiktools-test-{}-{}", std::process::id(), label),
        );
        Self {
            _lock: lock,
            path,
            previous_home,
            previous_ipc,
        }
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        match self.previous_home.take() {
            Some(value) => std::env::set_var("TIKTOOLS_HOME", value),
            None => std::env::remove_var("TIKTOOLS_HOME"),
        }
        match self.previous_ipc.take() {
            Some(value) => std::env::set_var("TIKTOOLS_IPC_NAME", value),
            None => std::env::remove_var("TIKTOOLS_IPC_NAME"),
        }
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

struct HostChild {
    child: std::process::Child,
}

impl HostChild {
    /// Spawns an isolated host under `home` and waits until it answers,
    /// verifying the answering host is this child (not a foreign
    /// already-running host on the same socket).
    async fn spawn(home: &Path) -> Self {
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_tiktools"))
            .args(["host", "--ipc"])
            .env("TIKTOOLS_HOME", home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn tiktools host --ipc");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        let client = loop {
            if tokio::time::Instant::now() >= deadline {
                let _ = child.kill();
                panic!("isolated host did not answer within 15s");
            }
            if let Ok(exit) = child.try_wait() {
                if exit.is_some() {
                    panic!("isolated host exited before answering (is another host running?)");
                }
            }
            match TikToolsClient::connect().await {
                Ok(client) => break client,
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        };
        // Prove the answering host is ours: its info paths must sit under
        // our temp home, never under the user's real home.
        let info = client
            .system_info()
            .await
            .expect("system.info on isolated host");
        let home_marker = home.to_string_lossy().into_owned();
        let info_text = serde_json::to_string(&info).unwrap_or_default();
        assert!(
            info_text.contains(&home_marker),
            "answering host is not the isolated child (a foreign host may be running)"
        );
        Self { child }
    }
}

impl Drop for HostChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct TestEmitter;

impl HostEmitter for TestEmitter {
    fn emit(&self, _message: HostMessage) {}
}

#[tokio::test]
async fn connected_concurrent_calls_share_clients() {
    let home = HomeGuard::set("host");
    let _host = HostChild::spawn(&home.path).await;
    let first = TikToolsClient::connect().await.expect("connect first");
    let second = TikToolsClient::connect().await.expect("connect second");
    let mut tasks = Vec::new();
    for index in 0..16 {
        let client = if index % 2 == 0 {
            first.clone()
        } else {
            second.clone()
        };
        tasks.push(tokio::spawn(async move {
            match index % 4 {
                0 => client.system_ping().await.map(|_| ()),
                1 => client.plugins_list().await.map(|_| ()),
                2 => client.points_config_get().await.map(|_| ()),
                _ => client.automation_context().await.map(|_| ()),
            }
        }));
    }
    for task in tasks {
        task.await.expect("task joins").expect("concurrent call ok");
    }
}

#[tokio::test]
async fn connected_subscribe_receives_domain_events() {
    let home = HomeGuard::set("host");
    let _host = HostChild::spawn(&home.path).await;
    let client = TikToolsClient::connect().await.expect("connect");
    let mut events = client.subscribe();
    client
        .points_adjust(PointsAdjustParams {
            expected_currency: None,
            unique_id: "verify-bot".to_owned(),
            delta: 1.0,
        })
        .await
        .expect("points.adjust triggers an event");
    let event = tokio::time::timeout(Duration::from_secs(10), events.recv())
        .await
        .expect("event arrives within budget")
        .expect("stream stays open");
    match event {
        ControlEvent::Domain(event) => assert_eq!(event.topic(), "points.changed"),
        ControlEvent::Reliable(_) => {
            panic!("local IPC must not receive remote subscription projections")
        }
        ControlEvent::Gap { lost } => panic!("unexpected gap of {lost} on a fresh stream"),
    }
}

#[tokio::test]
async fn direct_concurrent_calls_and_events() {
    let _home = HomeGuard::set("direct");
    let api = Arc::new(tiktools_control_api::ControlApi::new(Arc::new(
        AppCore::new(Arc::new(TestEmitter)),
    )));
    let client = TikToolsClient::direct(Arc::clone(&api));
    let mut tasks = Vec::new();
    for index in 0..16 {
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            match index % 4 {
                0 => client.system_ping().await.map(|_| ()),
                1 => client.workflows_list().await.map(|_| ()),
                2 => client.live_status().await.map(|_| ()),
                _ => client.processors_status().await.map(|_| ()),
            }
        }));
    }
    for task in tasks {
        task.await.expect("task joins").expect("direct call ok");
    }
    let mut events = client.subscribe();
    client
        .points_adjust(PointsAdjustParams {
            expected_currency: None,
            unique_id: "verify-bot".to_owned(),
            delta: 1.0,
        })
        .await
        .expect("points.adjust triggers an event");
    let event = tokio::time::timeout(Duration::from_secs(10), events.recv())
        .await
        .expect("event arrives within budget")
        .expect("stream stays open");
    match event {
        ControlEvent::Domain(event) => assert_eq!(event.topic(), "points.changed"),
        ControlEvent::Reliable(_) => {
            panic!("local IPC must not receive remote subscription projections")
        }
        ControlEvent::Gap { lost } => panic!("unexpected gap of {lost} on a fresh stream"),
    }
}

#[tokio::test]
async fn cli_explicit_command_metadata_preserves_receipts_and_retries_over_ipc() {
    let home = HomeGuard::set("command-meta");
    let _host = HostChild::spawn(&home.path).await;
    let client = TikToolsClient::connect().await.unwrap();
    let instance = client
        .collaboration_status()
        .await
        .unwrap()
        .host_instance_id;
    let params =
        r#"{"graph":{"id":"cli-workflow","name":"CLI","enabled":false,"nodes":[],"edges":[]}}"#;
    let invoke = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_tiktools"))
            .args(args)
            .env("TIKTOOLS_HOME", &home.path)
            .output()
            .unwrap()
    };
    let arguments = [
        "api",
        "call",
        "workflows.save",
        params,
        "--command-id",
        "cli-command",
        "--host-instance-id",
        &instance,
        "--expected-revision",
        "0",
        "--include-meta",
        "--format",
        "json",
    ];
    let first = invoke(&arguments);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first["meta"]["command"]["commandId"], "cli-command");
    assert_eq!(first["meta"]["resourceVersions"][0]["revision"], "1");
    let retry = invoke(&arguments);
    assert!(retry.status.success());
    let retry: serde_json::Value = serde_json::from_slice(&retry.stdout).unwrap();
    assert_eq!(retry, first);
    let conflict = invoke(&[
        "api",
        "call",
        "workflows.save",
        params,
        "--expected-revision",
        "0",
        "--format",
        "json",
    ]);
    assert_eq!(conflict.status.code(), Some(1));
    let conflict: serde_json::Value = serde_json::from_slice(&conflict.stdout).unwrap();
    assert_eq!(conflict["error"]["code"], "revision_conflict");
    assert_eq!(
        conflict["error"]["data"]["resourceVersions"][0]["revision"],
        "1"
    );
    let legacy = invoke(&[
        "api",
        "call",
        "workflows.get",
        r#"{"id":"cli-workflow"}"#,
        "--format",
        "json",
    ]);
    assert!(legacy.status.success());
    let legacy: serde_json::Value = serde_json::from_slice(&legacy.stdout).unwrap();
    assert_eq!(legacy["id"], "cli-workflow");
    assert!(legacy.get("meta").is_none());
    let dry = invoke(&[
        "api",
        "call",
        "workflows.save",
        params,
        "--command-id",
        "dry-command",
        "--host-instance-id",
        &instance,
        "--expected-revision",
        "1",
        "--dry-run",
        "--include-meta",
        "--format",
        "json",
    ]);
    assert!(dry.status.success());
    assert_eq!(
        client
            .collaboration_commands_get(tiktools_control_api::modules::collaboration::CommandGet {
                command_id: "dry-command".into(),
                principal_id: None
            })
            .await
            .unwrap_err()
            .code,
        "not_found"
    );
    let outcome = invoke(&[
        "api",
        "call",
        "collaboration.commands.get",
        r#"{"commandId":"cli-command"}"#,
        "--format",
        "json",
    ]);
    assert!(outcome.status.success());
    let outcome: serde_json::Value = serde_json::from_slice(&outcome.stdout).unwrap();
    assert_eq!(outcome["command"]["state"], "succeeded");
    let point_args = [
        "api",
        "call",
        "points.adjust",
        r#"{"uniqueId":"cli-viewer","delta":2}"#,
        "--command-id",
        "cli-points",
        "--host-instance-id",
        &instance,
        "--include-meta",
        "--format",
        "json",
    ];
    let first = invoke(&point_args);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first["result"]["totalPoints"], 2.0);
    assert_eq!(first["meta"]["resourceVersions"][0]["kind"], "viewer");
    let retry = invoke(&point_args);
    assert!(retry.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&retry.stdout).unwrap(),
        first
    );
    let cas = invoke(&[
        "api",
        "call",
        "points.adjust",
        r#"{"uniqueId":"cli-viewer","delta":2}"#,
        "--command-id",
        "wrong-cas",
        "--host-instance-id",
        &instance,
        "--expected-revision",
        "1",
        "--format",
        "json",
    ]);
    assert_eq!(cas.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&cas.stdout).unwrap()["error"]["code"],
        "invalid_params"
    );
    let missing_cas = invoke(&[
        "api",
        "call",
        "workflows.save",
        params,
        "--command-id",
        "missing-cas",
        "--host-instance-id",
        &instance,
        "--format",
        "json",
    ]);
    assert_eq!(missing_cas.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&missing_cas.stdout).unwrap()["error"]["code"],
        "invalid_params"
    );
    assert_eq!(
        client
            .collaboration_commands_get(tiktools_control_api::modules::collaboration::CommandGet {
                command_id: "missing-cas".into(),
                principal_id: None
            })
            .await
            .unwrap_err()
            .code,
        "not_found"
    );
    let point_outcome = invoke(&[
        "api",
        "call",
        "collaboration.commands.get",
        r#"{"commandId":"cli-points"}"#,
        "--format",
        "json",
    ]);
    assert!(point_outcome.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&point_outcome.stdout).unwrap()["result"]
            ["totalPoints"],
        2.0
    );
}
