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
        ControlEvent::Gap { lost } => panic!("unexpected gap of {lost} on a fresh stream"),
    }
}
