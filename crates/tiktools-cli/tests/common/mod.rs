//! Shared harness for the packaged owner browser integration tests: the
//! isolated `tiktools` command environment and the JSON-RPC call helper.

use serde_json::Value;
use std::{path::Path, process::Stdio};
use tokio::process::Command;

pub fn command(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tiktools"));
    command
        .current_dir(home)
        .env("TIKTOOLS_HOME", home)
        .env("TIKTOOLS_CLOUD_ORIGIN", "")
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
