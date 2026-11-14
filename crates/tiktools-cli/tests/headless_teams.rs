//! Process-level CLI boundaries, independent of a real account or desktop window.
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn command(home: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tiktools"));
    command
        .args(args)
        .current_dir(home)
        .env("TIKTOOLS_HOME", home)
        .env("TIKTOOLS_CLOUD_ORIGIN", "")
        .env("TIKTOOLS_REMOTE_PROFILES_DIR", home.join("profiles"))
        .env_remove("TIKTOOLS_SESSION_COOKIE")
        .stdin(Stdio::null());
    command.output().expect("compiled CLI starts")
}

#[test]
fn artifact_metadata_never_needs_or_starts_a_host() {
    let home = tempfile::tempdir().unwrap();
    let output = command(home.path(), &["--build-info"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        value["features"]["remoteControl"],
        cfg!(feature = "remote-control")
    );
    assert!(!home.path().join("data").exists());
}

#[test]
fn teams_usage_errors_are_resolved_before_ipc_or_writes() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["teams", "create", "--name", "Unconfirmed"],
        vec!["teams", "host", "start", "--confirm", "--timeout", "0"],
        vec!["teams", "people", "search"],
    ] {
        assert_eq!(command(home.path(), &args).status.code(), Some(2));
    }
    assert!(!home.path().join("data").exists());
}

#[cfg(all(feature = "remote-client", feature = "cloud-auth"))]
#[test]
fn unattended_join_reports_authentication_without_prompting_or_local_runtime() {
    let home = tempfile::tempdir().unwrap();
    let output = command(
        home.path(),
        &[
            "--json",
            "teams",
            "join",
            "--profile",
            "agent",
            "--kind",
            "agent",
            "--invite-stdin",
            "--non-interactive",
            "--timeout",
            "1",
        ],
    );
    assert!(!output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "authentication_required");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Open this URL"));
    assert!(!home.path().join("data").exists());
}

#[cfg(all(feature = "remote-client", feature = "cloud-auth"))]
#[test]
fn controller_status_distinguishes_an_unverified_account_and_keeps_profiles_private() {
    let home = tempfile::tempdir().unwrap();
    let output = command(home.path(), &["--json", "remote", "cloud", "status"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["context"], "controller");
    assert_eq!(value["verified"], false);
    assert!(value["restoreError"]["code"].is_string());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(home.path().join("profiles"))
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
    assert!(!home.path().join("data").exists());
}

#[cfg(feature = "remote-client")]
#[test]
fn remote_stdio_requires_explicit_target_and_never_creates_a_host() {
    let home = tempfile::tempdir().unwrap();
    let output = command(home.path(), &["--json", "remote", "serve", "--stdio"]);
    assert!(!output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "invalid_params");
    assert!(!home.path().join("data").exists());
}
