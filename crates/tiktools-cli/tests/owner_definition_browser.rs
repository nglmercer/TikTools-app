//! Actual packaged Owner browser → local adapter → IPC → native definition journal.
#![cfg(all(target_os = "linux", feature = "local-web"))]
use serde_json::{json, Value};
use std::{path::Path, process::Stdio, time::Duration};
mod common;
use common::command;
use tokio::process::Command;

async fn rpc(home: &Path, method: &str, params: Value) -> Option<Value> {
    common::rpc(home, method, params, false).await
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
        let (home, assets, repo) = common::packaged_home(temporary.path());
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

        let (mut web, owner_url) =
            common::start_web_adapter(command(&home), &assets, "definition owner").await;
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
