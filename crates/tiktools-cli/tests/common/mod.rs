//! Shared harness for the packaged owner browser integration tests: the
//! isolated `tiktools` command environment and the JSON-RPC call helper.

#![allow(dead_code)] // each browser test uses a different subset of the harness

use serde_json::Value;
use std::time::Duration;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::{Child, Command};

pub fn command(home: &Path) -> Command {
    command_with_cloud(home, "")
}
/// Like [`command`], with `cloud` as the Cloud origin the command talks to.
pub fn command_with_cloud(home: &Path, cloud: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tiktools"));
    command
        .current_dir(home)
        .env("TIKTOOLS_HOME", home)
        .env("TIKTOOLS_CLOUD_ORIGIN", cloud)
        .env_remove("TIKTOOLS_SESSION_COOKIE");
    for (key, directory) in [
        ("TIKTOOLS_DATA_DIR", "data"),
        ("TIKTOOLS_PLUGINS_DIR", "plugins"),
        ("TIKTOOLS_PLUGIN_DATA_DIR", "plugin-data"),
        ("TIKTOOLS_BUILTIN_PLUGINS_DIR", "builtins"),
        ("TIKTOOLS_DEV_PLUGINS_DIR", "dev-plugins"),
        ("TIKTOOLS_LOG_DIR", "logs"),
        ("TIKTOOLS_TEMP_DIR", "temp"),
    ] {
        command.env(key, home.join(directory));
    }
    command.kill_on_drop(true).stderr(Stdio::null());
    command
}
pub async fn rpc(
    home: &Path,
    method: &str,
    params: Value,
    private_invitation: bool,
) -> Option<Value> {
    let mut command = command(home);
    command.args([
        "--json",
        "api",
        "call",
        method,
        &params.to_string(),
        "--confirm",
    ]);
    // Only the isolated fixture receives this invitation; no secret is logged
    // or passed to the browser driver, shell environment, or process arguments.
    if private_invitation {
        command.arg("--secrets-visible");
    }
    let output = command.output().await.ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}
pub fn port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// An isolated home under `temporary` holding empty database files, and the
/// packaged web frontend and the repository root, returned as
/// `(home, assets, repo)`. Panics when `dist/web` has not been built.
pub fn packaged_home(temporary: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let home = temporary.join("home");
    std::fs::create_dir_all(home.join("data")).unwrap();
    for file in [
        "tiktok-points.db",
        "tiktok-automation.db",
        "tiktok-analytics.db",
    ] {
        std::fs::File::create(home.join("data").join(file)).unwrap();
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let assets = repo.join("dist/web");
    assert!(
        assets.join("index.html").is_file(),
        "build the packaged web frontend first"
    );
    (home, assets, repo)
}

/// Starts the packaged web adapter with `base` (an isolated `command(...)`),
/// serving `assets` on a free port, and waits until it accepts connections.
/// Returns the child and the owner URL the browser specs are pointed at.
pub async fn start_web_adapter(mut base: Command, assets: &Path, what: &str) -> (Child, String) {
    let port = port();
    let mut web = base
        .args(["web", "--assets"])
        .arg(assets)
        .args(["--port", &port.to_string()])
        .env("TIKTOOLS_PAIRING_FIXTURE", "948231")
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            assert!(web.try_wait().unwrap().is_none(), "{what} adapter stopped");
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{what} browser adapter did not start"));
    (web, format!("http://127.0.0.1:{port}"))
}
