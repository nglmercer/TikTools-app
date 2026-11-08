#![cfg(feature = "remote-client")]
use std::{
    path::PathBuf,
    process::{Command, Output},
};
struct Home(PathBuf);
impl Home {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "tt-remote-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tiktools"))
            .args(args)
            .env("TIKTOOLS_HOME", &self.0)
            .env(
                "TIKTOOLS_IPC_NAME",
                format!("tt-no-fallback-{}", std::process::id()),
            )
            .output()
            .unwrap()
    }
    fn no_host_data(&self) {
        assert!(!self.0.join("data").exists());
        assert!(!self.0.join("plugins").exists());
        assert!(!self.0.join("plugin-data").exists());
    }
}
impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn invalid_remote_combinations_never_start_a_runtime_or_sandbox() {
    let home = Home::new();
    for args in [
        vec!["--remote-profile", "desk", "--standalone", "live", "status"],
        vec!["--remote-profile", "desk", "host", "--ipc"],
        vec!["--remote-profile", "desk", "rpc", "--stdio"],
        vec!["--remote-profile", "desk", "api", "verify", "--sandbox"],
        vec!["--remote-profile", "desk", "web", "--assets", "missing"],
    ] {
        assert_eq!(home.run(&args).status.code(), Some(2));
        home.no_host_data();
    }
}
#[test]
fn missing_remote_profile_fails_without_local_ipc_or_direct_fallback() {
    let home = Home::new();
    let output = home.run(&["--json", "--remote-profile", "missing", "live", "status"]);
    assert_ne!(output.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "profile_not_found");
    home.no_host_data();
}
#[test]
fn profile_inventory_and_rejected_secret_arguments_create_no_host_data_or_secret_logs() {
    let home = Home::new();
    let list = home.run(&["--json", "remote", "list"]);
    assert!(list.status.success());
    let value: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    assert!(value["profiles"].as_array().unwrap().is_empty());
    home.no_host_data();
    let output = home.run(&[
        "remote",
        "pair",
        "--url",
        "https://127.0.0.1",
        "--profile",
        "desk",
        "--secret",
        "private-test-marker",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-test-marker"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private-test-marker"));
    home.no_host_data();
}
