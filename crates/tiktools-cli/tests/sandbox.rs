//! A caller's explicit directory overrides must never escape sandbox verify.
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn sandbox_ignores_production_path_overrides_and_development_plugins() {
    let root = std::env::temp_dir().join(format!(
        "tt-sandbox-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("data")).unwrap();
    fs::write(
        root.join("data/tiktok-automation.db"),
        b"caller legacy database sentinel",
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_tiktools"));
    command
        .args(["--json", "api", "verify", "--sandbox"])
        .current_dir(&root);
    for key in [
        "TIKTOOLS_HOME",
        "TIKTOOLS_DATA_DIR",
        "TIKTOOLS_PLUGINS_DIR",
        "TIKTOOLS_PLUGIN_DATA_DIR",
        "TIKTOOLS_LOG_DIR",
        "TIKTOOLS_TEMP_DIR",
        "TIKTOOLS_BUILTIN_PLUGINS_DIR",
        "TIKTOOLS_DEV_PLUGINS_DIR",
    ] {
        command.env(key, root.join(key));
    }
    command.env("TIKTOOLS_SESSION_COOKIE", "sandbox-test-secret");
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("sandbox-test-secret"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("sandbox-test-secret"));
    assert_eq!(
        fs::read_dir(&root).unwrap().count(),
        1,
        "sandbox wrote to caller paths"
    );
    assert_eq!(
        fs::read(root.join("data/tiktok-automation.db")).unwrap(),
        b"caller legacy database sentinel"
    );
    fs::remove_dir_all(root).unwrap();
}
