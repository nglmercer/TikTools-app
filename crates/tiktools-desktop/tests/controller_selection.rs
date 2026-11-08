//! Actual binary rejection must happen before logging, locks or GUI startup.
use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
#[test]
fn controller_selection_rejects_host_options_and_secrets_before_host_files() {
    let root = std::env::temp_dir().join(format!(
        "tiktools-controller-startup-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(!root.exists());
    for args in [
        vec!["--controller", "--install-plugin", "not-executed.plugin"],
        vec!["--remote-profile"],
        vec!["--remote-profile=a", "--remote-profile=b"],
        vec![
            "--controller",
            "https://host.example/#private-invitation-marker",
        ],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_tiktools-desktop"))
            .args(args)
            .env("TIKTOOLS_HOME", &root)
            .env("TIKTOOLS_DATA_DIR", root.join("data"))
            .env("TIKTOOLS_LOG_DIR", root.join("logs"))
            .env("TIKTOOLS_PLUGINS_DIR", root.join("plugins"))
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&result.stderr).contains("private-invitation-marker"));
        assert!(!root.exists(), "controller rejection created host files");
    }
}
