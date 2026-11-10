//! Packaged Vue -> local adapter -> IPC -> native catalog -> disposable HTTP.
#![cfg(all(target_os = "linux", feature = "local-web", debug_assertions))]

use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    process::Command,
};

const GENERATION: i64 = 9_007_199_254_740_993;
const SUMMARY_CURSOR: &str = "eyJraW5kIjoic3VtbWFyeSIsImdlbmVyYXRpb24iOjkwMDcxOTkyNTQ3NDA5OTMsInEiOiIiLCJ0YXJnZXQiOm51bGwsImxhc3QiOiJjYXRhbG9nLnA0OSJ9";
const VERSION_CURSOR: &str = "eyJraW5kIjoidmVyc2lvbnMiLCJnZW5lcmF0aW9uIjo5MDA3MTk5MjU0NzQwOTkzLCJpZCI6ImNhdGFsb2cucDAwIiwidGFyZ2V0IjpudWxsLCJ2ZXJzaW9uIjoiMS4wLjArYnVpbGQ0OSIsImxhc3RfdGFyZ2V0IjoiYW55In0";

fn command(home: &Path, cloud: &str) -> Command {
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

async fn rpc(home: &Path, cloud: &str, method: &str, params: Value) -> Option<Value> {
    let output = command(home, cloud)
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

fn plugin(index: u8) -> Value {
    let id = format!("catalog.p{index:02}");
    let withdrawn = index == 50;
    json!({
        "id": id,
        "latestStableVersion": if withdrawn { None } else { Some("1.0.0") },
        "metadataPath": format!("/api/v1/registry/generations/{GENERATION}/files/index/ca/ta/catalog.p{index:02}.json"),
    })
}

fn version(index: u8, withdrawn: bool) -> Value {
    json!({
        "version":format!("1.0.0+build{index:02}"), "target":"any", "sha256":"a".repeat(64),
        "size":"9007199254740995", "yanked":withdrawn, "keyRevoked":withdrawn,
        "publisherApproved":!withdrawn,
        "advisory":if withdrawn { Some(json!({"severity":"high","message":"Fixture advisory","url":null})) } else { None }
    })
}

async fn catalog_fixture() -> (String, Arc<Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let saved = requests.clone();
    let server = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let saved = saved.clone();
            tokio::spawn(async move {
                let mut request = Vec::new();
                loop {
                    let mut buffer = [0_u8; 2048];
                    let Ok(Ok(count)) =
                        tokio::time::timeout(Duration::from_secs(5), socket.read(&mut buffer))
                            .await
                    else {
                        return;
                    };
                    if count == 0 {
                        return;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                        break;
                    }
                    if request.len() > 16 * 1024 {
                        return;
                    }
                }
                let request = String::from_utf8_lossy(&request).to_string();
                let line = request.lines().next().unwrap_or_default().to_string();
                let catalog = line.starts_with("GET /api/v2/catalog/");
                if catalog {
                    saved.lock().unwrap().push(request);
                }
                let (status, body) = if line
                    .starts_with("GET /api/v2/catalog/pagination-capabilities")
                {
                    ("200 OK", json!({"schemaVersion":1,"summaryPath":"/api/v2/catalog/plugin-summaries","versionPagePath":"/api/v2/catalog/plugins/{id}/version-pages","maxItems":50,"maxBytes":524288,"versionOrder":"ascii-version-target"}).to_string())
                } else if line.starts_with("GET /api/v2/catalog/plugin-summaries?") {
                    let second = line.contains(SUMMARY_CURSOR);
                    let items = if second {
                        vec![plugin(50)]
                    } else {
                        (0..50).map(plugin).collect()
                    };
                    ("200 OK", json!({"generation":GENERATION.to_string(),"items":items,"nextCursor":if second {None} else {Some(SUMMARY_CURSOR)}}).to_string())
                } else if line.starts_with("GET /api/v2/catalog/plugins/catalog.p")
                    && line.contains("/version-pages?")
                {
                    let id = line.split('/').nth(5).unwrap_or_default();
                    let withdrawn = id == "catalog.p50";
                    let second = line.contains(VERSION_CURSOR);
                    let items = if withdrawn {
                        vec![version(0, true)]
                    } else if second {
                        vec![version(50, false)]
                    } else {
                        (0..50).map(|i| version(i, false)).collect()
                    };
                    ("200 OK", json!({"generation":GENERATION.to_string(),"id":id,"metadataPath":format!("/api/v1/registry/generations/{GENERATION}/files/index/ca/ta/{id}.json"),"items":items,"nextCursor":if withdrawn || second { None } else {Some(VERSION_CURSOR)}}).to_string())
                } else {
                    ("404 Not Found", "{}".to_string())
                };
                let header = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = socket.write_all(header.as_bytes()).await;
                let _ = socket.write_all(body.as_bytes()).await;
            });
        }
    });
    (origin, requests, server)
}

fn port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires built dist/web, Bun and Playwright Chromium; run explicitly"]
async fn packaged_cloud_catalog_uses_native_http_and_pinned_pagination() {
    tokio::time::timeout(Duration::from_secs(240), async {
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
            "build the packaged frontend first"
        );
        let (cloud, requests, server) = catalog_fixture().await;
        let mut host = command(&home, &cloud)
            .args(["host", "--ipc"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let info = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                assert!(
                    host.try_wait().unwrap().is_none(),
                    "isolated catalog host stopped"
                );
                if let Some(info) = rpc(&home, &cloud, "system.info", json!({})).await {
                    break info;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("catalog IPC host did not start");
        assert_eq!(info["home"].as_str(), home.to_str());
        let web_port = port();
        let owner_url = format!("http://127.0.0.1:{web_port}");
        let mut web = command(&home, &cloud)
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
                assert!(web.try_wait().unwrap().is_none(), "catalog adapter stopped");
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
        .expect("catalog browser adapter did not start");
        let output = Command::new("bun")
            .args([
                "x",
                "--no-install",
                "playwright",
                "test",
                "--config",
                "tests/helpers/cloud-catalog-playwright.config.ts",
            ])
            .current_dir(&repo)
            .env("TIKTOOLS_CLOUD_CATALOG_BROWSER_URL", &owner_url)
            .kill_on_drop(true)
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "catalog browser failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        {
            let captured = requests.lock().unwrap();
            assert!(captured.len() >= 6, "expected real native catalog requests");
            assert!(captured.iter().any(|request| request
                .lines()
                .next()
                .unwrap_or_default()
                .contains(SUMMARY_CURSOR)
                && request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .contains("generation=9007199254740993")));
            assert!(captured.iter().any(|request| request
                .lines()
                .next()
                .unwrap_or_default()
                .contains(VERSION_CURSOR)
                && request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .contains("generation=9007199254740993")));
            assert!(
                captured
                    .iter()
                    .all(|request| !request.to_ascii_lowercase().contains("\nauthorization:")),
                "catalog requests must be anonymous"
            );
        }
        assert!(host.try_wait().unwrap().is_none());
        let _ = web.kill().await;
        let _ = web.wait().await;
        let _ = rpc(&home, &cloud, "system.shutdown", json!({})).await;
        let _ = host.kill().await;
        let _ = host.wait().await;
        server.abort();
    })
    .await
    .expect("actual catalog browser deadline exceeded");
}
