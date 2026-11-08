//! Packaged Owner Vue → authenticated local adapter → IPC → atomic library writer.
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
    for (key, folder) in [
        ("TIKTOOLS_DATA_DIR", "data"),
        ("TIKTOOLS_PLUGINS_DIR", "plugins"),
        ("TIKTOOLS_PLUGIN_DATA_DIR", "plugin-data"),
        ("TIKTOOLS_BUILTIN_PLUGINS_DIR", "builtins"),
        ("TIKTOOLS_DEV_PLUGINS_DIR", "dev-plugins"),
        ("TIKTOOLS_LOG_DIR", "logs"),
        ("TIKTOOLS_TEMP_DIR", "temp"),
    ] {
        command.env(key, home.join(folder));
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
    json!({"id":format!("library-delete-action-{locale}"),"name":format!("Delete library action {locale}"),"enabled":false,"typeId":"core.log","config":{"message":format!("private-library-delete-source-{locale}")}})
}
fn event(locale: &str, external: bool) -> Value {
    let kind = if external { "external" } else { "delete" };
    json!({"id":format!("library-{kind}-event-{locale}"),"name":format!("{kind} library event {locale}"),"enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all","actionIds":[format!("library-delete-action-{locale}")]})
}
fn template(locale: &str) -> Value {
    json!({"templateVersion":1,"id":format!("library-seed-{locale}"),"title":{"default":format!("Library seed {locale}")},"description":{"default":"Private local library fixture"},"icon":"plugin","actions":[{"name":format!("Applied library action {locale}"),"typeId":"core.log","enabled":false,"config":{"message":format!("private-library-seed-source-{locale}"),"weight":1.0,"tolerance":0.000001,"tiny":1e-7,"":"bmp","𐀀":"supplementary"}}],"event":{"name":format!("Applied library event {locale}"),"enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all"},"custom":true})
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires built dist/web, Bun and Playwright Chromium; run explicitly"]
async fn actual_owner_library_transactions_recover_original_identity_and_preserve_newer_candidates()
{
    tokio::time::timeout(Duration::from_secs(360),async {
        let temporary=tempfile::tempdir().unwrap();let home=temporary.path().join("home");
        std::fs::create_dir_all(home.join("data")).unwrap();
        for file in ["tiktok-points.db","tiktok-automation.db","tiktok-analytics.db"] {std::fs::File::create(home.join("data").join(file)).unwrap();}
        let repo=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();let assets=repo.join("dist/web");
        assert!(assets.join("index.html").is_file(),"build packaged frontend first");
        let mut host=command(&home).args(["host","--ipc"]).stdout(Stdio::null()).spawn().unwrap();
        let info=tokio::time::timeout(Duration::from_secs(20),async {loop {
            assert!(host.try_wait().unwrap().is_none(),"library host stopped at startup");
            if let Some(info)=rpc(&home,"system.info",json!({})).await {break info;}
            tokio::time::sleep(Duration::from_millis(50)).await;
        }}).await.expect("library IPC host failed to start");
        assert_eq!(info["home"].as_str(),home.to_str());
        let mut packs=Vec::new();let mut templates=Vec::new();
        for locale in ["en","es"] {
            for (kind,record) in [("action",action(locale)),("event",event(locale,false)),("event",event(locale,true))] {
                assert!(rpc(&home,"automation.create",json!({"kind":kind,"record":record})).await.is_some());
            }
            packs.push(json!({"id":format!("delete-library-{locale}"),"name":format!("Delete library profile {locale}"),"description":"Delete with an outside reference","eventIds":[format!("library-delete-event-{locale}")],"actionIds":[format!("library-delete-action-{locale}")]}));
            templates.push(template(locale));
        }
        for (key,value) in [("behavior.profiles.packs",json!(packs).to_string()),("behavior.profiles.active","default".into()),("behavior.templates.custom",json!(templates).to_string())] {
            assert!(rpc(&home,"app.state.set",json!({"key":key,"value":value})).await.is_some());
        }
        let initial=rpc(&home,"automation.library.get",json!({})).await.unwrap();
        assert!(initial["pending"].is_null());assert_eq!(initial["recent"],json!([]),"fixture seed is outside native UI intents");
        let web_port=port();let owner_url=format!("http://127.0.0.1:{web_port}");
        let mut web=command(&home).args(["web","--assets",assets.to_str().unwrap(),"--port",&web_port.to_string()]).env("TIKTOOLS_PAIRING_FIXTURE","948231").stdout(Stdio::null()).spawn().unwrap();
        tokio::time::timeout(Duration::from_secs(10),async {loop {
            assert!(web.try_wait().unwrap().is_none(),"library adapter stopped");
            if tokio::net::TcpStream::connect(("127.0.0.1",web_port)).await.is_ok(){break;}
            tokio::time::sleep(Duration::from_millis(25)).await;
        }}).await.expect("library adapter failed to listen");
        let output=Command::new("bun").args(["x","--no-install","playwright","test","--config","tests/helpers/owner-library-playwright.config.ts"]).current_dir(&repo)
            .env("TIKTOOLS_OWNER_LIBRARY_URL",&owner_url).env("TIKTOOLS_OWNER_LIBRARY_HOME",&home).env("TIKTOOLS_OWNER_LIBRARY_BINARY",env!("CARGO_BIN_EXE_tiktools"))
            .kill_on_drop(true).output().await.unwrap();
        assert!(output.status.success(),"Owner library browser failed:\n{}\n{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
        let state=rpc(&home,"automation.library.get",json!({})).await.unwrap();
        assert!(state["pending"].is_null());let recent=state["recent"].as_array().unwrap();assert_eq!(recent.len(),16);
        let mut ids=std::collections::BTreeSet::new();
        for row in recent {assert_eq!(row["outcome"],"succeeded");let id=row["intent"]["commandId"].as_str().unwrap();assert!(id.starts_with("owner-library-"));assert_eq!(id.len(),"owner-library-".len()+32);assert!(ids.insert(id));assert_eq!(row["intent"]["hostInstanceId"],state["hostInstanceId"]);}
        for operation in ["create-profile","apply-template","import-profile","assign-record","import-templates","delete-template","switch-profile","delete-profile"] {
            assert_eq!(recent.iter().filter(|r|r["intent"]["operation"]==operation).count(),2,"{operation}");
        }
        assert!(!state.to_string().contains("private-library-"));assert!(!state.to_string().contains("acknowledgement"));
        let snapshot=rpc(&home,"automation.library.snapshot",json!({})).await.unwrap();
        assert_eq!(snapshot["catalog"]["activeId"],"default");assert_eq!(snapshot["catalog"]["templates"].as_array().unwrap().len(),2);
        for locale in ["en","es"] {
            assert!(!snapshot["catalog"]["packs"].as_array().unwrap().iter().any(|p|p["id"]==format!("delete-library-{locale}")));
            assert!(!snapshot["definitions"]["actions"].as_array().unwrap().iter().any(|r|r["id"]==format!("library-delete-action-{locale}")));
            assert!(!snapshot["definitions"]["events"].as_array().unwrap().iter().any(|r|r["id"]==format!("library-delete-event-{locale}")));
            let external=snapshot["definitions"]["events"].as_array().unwrap().iter().find(|r|r["id"]==format!("library-external-event-{locale}")).unwrap();assert_eq!(external["actionIds"],json!([]));
        }
        assert!(host.try_wait().unwrap().is_none());let _=web.kill().await;let _=web.wait().await;
        let _=rpc(&home,"system.shutdown",json!({})).await;let _=host.kill().await;let _=host.wait().await;
    }).await.expect("actual library browser deadline exceeded");
}
