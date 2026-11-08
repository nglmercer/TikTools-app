//! Actual packaged Owner browser → local adapter → IPC → native definition journal.
#![cfg(all(target_os = "linux", feature = "local-web"))]
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::process::Command;

fn command(home: &Path) -> Command {
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
async fn rpc(home: &Path, method: &str, params: Value) -> Option<Value> {
    let output = command(home)
        .args([
            "--json",
            "api",
            "call",
            method,
            &params.to_string(),
            "--confirm",
        ])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}
fn port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
fn action(locale: &str) -> Value {
    json!({
        "id":format!("definition-action-{locale}"),
        "name":format!("Owner definition action {locale}"),
        "enabled":false,"typeId":"core.log",
        "config":{"message":format!("definition-source-{locale}")},
    })
}
fn event(locale: &str) -> Value {
    json!({
        "id":format!("definition-event-{locale}"),
        "name":format!("Owner definition event {locale}"),
        "enabled":false,"trigger":"tiktok.chat","filters":[],
        "cooldownMs":0,"cooldownScope":"global","runMode":"all",
        "actionIds":[format!("definition-action-{locale}")],
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires built dist/web, Bun and Playwright Chromium; run explicitly"]
async fn actual_owner_definition_writes_reload_and_recovery_keep_original_results_and_newer_drafts()
{
    tokio::time::timeout(Duration::from_secs(300), async {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("home");
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
        let mut host = command(&home)
            .args(["host", "--ipc"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let info = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                assert!(
                    host.try_wait().unwrap().is_none(),
                    "isolated definition host stopped during startup"
                );
                if let Some(info) = rpc(&home, "system.info", json!({})).await {
                    break info;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("definition IPC host did not start");
        assert_eq!(info["home"].as_str(), home.to_str());
        for locale in ["en", "es"] {
            for (kind, record) in [("action", action(locale)), ("event", event(locale))] {
                assert!(rpc(
                    &home,
                    "automation.create",
                    json!({"kind":kind,"record":record})
                )
                .await
                .is_some());
            }
        }
        let initial = rpc(&home, "automation.definition.get", json!({}))
            .await
            .unwrap();
        assert!(initial["pending"].is_null());
        assert_eq!(
            initial["recent"],
            json!([]),
            "fixture seed writes are separate from native UI intents"
        );

        let web_port = port();
        let owner_url = format!("http://127.0.0.1:{web_port}");
        let mut web = command(&home)
            .args([
                "web",
                "--assets",
                assets.to_str().unwrap(),
                "--port",
                &web_port.to_string(),
            ])
            .env("TIKTOOLS_PAIRING_FIXTURE", "948231")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                assert!(
                    web.try_wait().unwrap().is_none(),
                    "definition owner adapter stopped"
                );
                if tokio::net::TcpStream::connect(("127.0.0.1", web_port))
                    .await
                    .is_ok()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("definition owner browser adapter did not start");
        let output = Command::new("bun")
            .args([
                "x",
                "--no-install",
                "playwright",
                "test",
                "--config",
                "tests/helpers/owner-definition-playwright.config.ts",
            ])
            .current_dir(&repo)
            .env("TIKTOOLS_OWNER_DEFINITION_URL", &owner_url)
            .env("TIKTOOLS_OWNER_DEFINITION_HOME", &home)
            .env(
                "TIKTOOLS_OWNER_DEFINITION_BINARY",
                env!("CARGO_BIN_EXE_tiktools"),
            )
            .kill_on_drop(true)
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "owner definition browser failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let journal = rpc(&home, "automation.definition.get", json!({}))
            .await
            .unwrap();
        assert!(journal["pending"].is_null());
        let recent = journal["recent"].as_array().unwrap();
        assert_eq!(recent.len(), 16);
        assert_eq!(
            recent
                .iter()
                .filter(|row| row["outcome"] == "succeeded")
                .count(),
            16
        );
        assert!(!journal.to_string().contains("definition-source-"));
        assert!(!journal.to_string().contains("acknowledgement"));
        let mut ids = std::collections::BTreeSet::new();
        for row in recent {
            let id = row["intent"]["commandId"].as_str().unwrap();
            assert!(id.starts_with("owner-definition-"));
            assert_eq!(id.len(), "owner-definition-".len() + 32);
            assert!(ids.insert(id));
            assert_eq!(row["intent"]["hostInstanceId"], journal["hostInstanceId"]);
        }
        assert!(host.try_wait().unwrap().is_none());
        let _ = web.kill().await;
        let _ = web.wait().await;
        let _ = rpc(&home, "system.shutdown", json!({})).await;
        let _ = host.kill().await;
        let _ = host.wait().await;
    })
    .await
    .expect("actual Owner definition browser deadline exceeded");
}
