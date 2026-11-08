//! Opt-in actual UI → local browser adapter → IPC owner → TLS controllers.
#![cfg(all(
    target_os = "linux",
    feature = "remote-control",
    feature = "remote-client",
    feature = "local-web"
))]
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tiktools_client::remote::{
    ClientKind, NativeCredentials, PairingPoll, RemoteClient, RemoteEndpoint, SecretString,
    TlsTrust,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdout, Command},
};

fn command(home: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_tiktools"));
    c.current_dir(home)
        .env("TIKTOOLS_HOME", home)
        .env("TIKTOOLS_CLOUD_ORIGIN", "")
        .env_remove("TIKTOOLS_SESSION_COOKIE");
    for (key, dir) in [
        ("TIKTOOLS_DATA_DIR", "data"),
        ("TIKTOOLS_PLUGINS_DIR", "plugins"),
        ("TIKTOOLS_PLUGIN_DATA_DIR", "plugin-data"),
        ("TIKTOOLS_BUILTIN_PLUGINS_DIR", "builtins"),
        ("TIKTOOLS_DEV_PLUGINS_DIR", "dev-plugins"),
        ("TIKTOOLS_LOG_DIR", "logs"),
        ("TIKTOOLS_TEMP_DIR", "temp"),
    ] {
        c.env(key, home.join(dir));
    }
    c.kill_on_drop(true).stderr(Stdio::null());
    c
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
    // `api call` prints the method result directly unless --include-meta is set.
    serde_json::from_slice::<Value>(&output.stdout).ok()
}
async fn rpc_meta(home: &Path, method: &str, params: Value) -> Value {
    let output = command(home)
        .args([
            "--json",
            "api",
            "call",
            method,
            &params.to_string(),
            "--confirm",
            "--include-meta",
        ])
        .output()
        .await
        .expect("metadata client did not run");
    assert!(output.status.success(), "metadata client failed");
    serde_json::from_slice(&output.stdout).expect("invalid metadata result")
}
fn port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
async fn event(lines: &mut Lines<BufReader<ChildStdout>>, name: &str) -> Value {
    let line = tokio::time::timeout(Duration::from_secs(25), lines.next_line())
        .await
        .expect("owner UI response timed out")
        .expect("owner UI pipe failed")
        .expect("owner UI driver ended early");
    let value: Value = serde_json::from_str(&line).expect("invalid owner UI protocol");
    assert_eq!(
        value["event"].as_str(),
        Some(name),
        "unexpected owner UI phase"
    );
    value
}
async fn send(child: &mut Child, value: Value) {
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&bytes)
        .await
        .unwrap();
}
async fn closed(remote: &RemoteClient) {
    tokio::time::timeout(Duration::from_secs(8), async {
        while !remote.is_closed() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("revoked controller stayed connected");
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires built dist/web, Bun and Playwright Chromium; run explicitly"]
async fn real_owner_ui_manages_two_tls_sessions_against_one_headless_core() {
    tokio::time::timeout(Duration::from_secs(100),async {
        let temp=tempfile::tempdir().unwrap();let home=temp.path().join("home");std::fs::create_dir(&home).unwrap();
        let repo=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
        let assets=repo.join("dist/web");assert!(assets.join("index.html").is_file());
        let mut host=command(&home).args(["host","--ipc"]).stdout(Stdio::null()).spawn().unwrap();
        let info=tokio::time::timeout(Duration::from_secs(15),async {loop {
            assert!(host.try_wait().unwrap().is_none(),"isolated owner host stopped during startup");
            if let Some(info)=rpc(&home,"system.info",json!({})).await {break info;}
            tokio::time::sleep(Duration::from_millis(50)).await;
        }}).await.expect("owner IPC did not start");
        assert_eq!(info["home"].as_str(),home.to_str());
        assert!(rpc(&home,"workflows.save",json!({"graph":{"id":"welcome","name":"Real owner workflow","enabled":false}})).await.is_some());
        for kind in ["action","event"] {
            assert!(rpc(&home,"automation.create",json!({"kind":kind,"record":{"id":"welcome","name":"Real owner behavior","enabled":true,"config":{"secret":"owner-only"}}})).await.is_some());
        }
        assert!(rpc(&home,"automation.create",json!({"record":{"id":"private","name":"Private label","enabled":true}})).await.is_some());
        let pair=rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
        let certificate=temp.path().join("cert.pem");let key=temp.path().join("key.pem");
        std::fs::write(&certificate,pair.cert.pem()).unwrap();std::fs::write(&key,pair.signing_key.serialize_pem()).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key,std::fs::Permissions::from_mode(0o600)).unwrap();
        let fingerprint:String=ring::digest::digest(&ring::digest::SHA256,pair.cert.der()).as_ref().iter().map(|b|format!("{b:02x}")).collect();
        let web_port=port();let mut remote_port=port();while remote_port==web_port{remote_port=port();}
        let owner_url=format!("http://127.0.0.1:{web_port}");let origin=format!("https://127.0.0.1:{remote_port}");
        let mut web=command(&home).args(["web","--assets",assets.to_str().unwrap(),"--port",&web_port.to_string()]).env("TIKTOOLS_PAIRING_FIXTURE","948231").stdout(Stdio::null()).spawn().unwrap();
        tokio::time::timeout(Duration::from_secs(10),async{loop{
            assert!(web.try_wait().unwrap().is_none(),"local owner browser adapter stopped");
            if tokio::net::TcpStream::connect(("127.0.0.1",web_port)).await.is_ok(){break;}
            tokio::time::sleep(Duration::from_millis(50)).await;
        }}).await.expect("owner browser adapter did not start");
        let mut driver=Command::new("bun").arg(repo.join("tests/helpers/owner-browser-driver.ts")).current_dir(&repo).kill_on_drop(true).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
        let mut lines=BufReader::new(driver.stdout.take().unwrap()).lines();
        send(&mut driver,json!({"ownerUrl":owner_url,"origin":origin,"bind":format!("127.0.0.1:{remote_port}"),"certificate":certificate,"key":key})).await;
        let mut sessions:Vec<(NativeCredentials,RemoteClient)>=Vec::new();
        for index in 0..2 {
            let invite=event(&mut lines,"invitation").await;
            let endpoint=RemoteEndpoint::new(&origin,TlsTrust::CertificateSha256{sha256:fingerprint.clone()}).await.unwrap();
            let mut pairing=endpoint.begin_pairing(invite["inviteId"].as_str().unwrap(),&SecretString::from(invite["secret"].as_str().unwrap().to_string()),&format!("Native controller {index}"),ClientKind::Agent).await.unwrap();
            send(&mut driver,json!({"type":"approve","requestId":pairing.request_id})).await;
            event(&mut lines,"approved").await;
            let credentials=match pairing.poll().await.unwrap(){PairingPoll::Approved(c)=>c,_=>panic!("owner did not approve the device")};
            let remote=RemoteClient::connect(&endpoint,&credentials,ClientKind::Agent).await.unwrap();
            assert_eq!(remote.call_value_with_meta("workflows.get",json!({"id":"welcome"}),None).await.unwrap().result["name"],"Real owner workflow");
            assert_eq!(remote.call_value_with_meta("system.shutdown",json!({}),None).await.unwrap_err().code,"forbidden");
            let status=remote.call_value_with_meta("automation.list",json!({}),None).await.unwrap();
            assert_eq!(status.result["events"].as_array().unwrap().len(),1);
            assert_eq!(status.result["actions"].as_array().unwrap().len(),1);
            assert!(status.result["events"][0]["definitionRedacted"].as_bool().unwrap());
            assert!(!status.result.to_string().contains("owner-only"));
            assert!(!status.result.to_string().contains("Private label"));
            sessions.push((credentials,remote));
            send(&mut driver,json!({"type":"connected"})).await;
        }
        assert_eq!(sessions[0].1.hello().host_instance_id,sessions[1].1.hello().host_instance_id);
        let metadata=tiktools_client::RequestMetadata { command_id:Some("browser-approved-stop".into()),host_instance_id:Some(sessions[0].1.hello().host_instance_id.clone()),expected_revision:Some("1".into()) };
        let stopped=sessions[0].1.call_value_with_meta("automation.disable",json!({"kind":"event","id":"welcome"}),Some(metadata)).await.unwrap();
        assert_eq!(stopped.result,json!({"id":"welcome","enabled":false}));
        assert_eq!(stopped.meta.unwrap().resource_versions[0].revision,"2");
        let status=sessions[1].1.call_value_with_meta("automation.get",json!({"kind":"event","id":"welcome"}),None).await.unwrap();
        assert_eq!(status.result["enabled"],false);
        assert_eq!(sessions[0].1.call_value_with_meta("collaboration.commands.get",json!({"commandId":"browser-approved-stop"}),None).await.unwrap().result["result"],json!({"id":"welcome","enabled":false}));
        send(&mut driver,json!({"type":"revoke","sessionId":sessions[0].0.session_id()})).await;
        event(&mut lines,"revoked").await;closed(&sessions[0].1).await;
        assert!(sessions[1].1.call_value_with_meta("workflows.get",json!({"id":"welcome"}),None).await.is_ok());
        send(&mut driver,json!({"type":"disable"})).await;event(&mut lines,"disabled").await;
        closed(&sessions[1].1).await;
        assert!(host.try_wait().unwrap().is_none(),"disabling collaboration stopped the owner host");
        assert_eq!(rpc(&home,"system.info",json!({})).await.unwrap()["home"],info["home"]);
        drop(driver.stdin.take());
        assert!(driver.wait().await.unwrap().success(),"real owner UI driver failed");
        let _=web.kill().await;let _=web.wait().await;
        let _=rpc(&home,"system.shutdown",json!({})).await;
        let _=host.kill().await;let _=host.wait().await;
    }).await.expect("real owner integration deadline exceeded");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires built dist/web, Bun and Playwright Chromium; run explicitly"]
async fn real_owner_automation_editors_preserve_conflicts_and_review_deleted_record_recreation() {
    tokio::time::timeout(Duration::from_secs(160), async {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        std::fs::create_dir_all(home.join("data")).unwrap();
        for name in ["tiktok-points.db", "tiktok-automation.db", "tiktok-analytics.db"] {
            std::fs::File::create(home.join("data").join(name)).unwrap();
        }
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
        let assets = repo.join("dist/web");
        assert!(assets.join("index.html").is_file());
        let mut host = command(&home).args(["host", "--ipc"]).stdout(Stdio::null()).spawn().unwrap();
        let info = tokio::time::timeout(Duration::from_secs(15), async { loop {
            assert!(host.try_wait().unwrap().is_none(), "isolated owner host stopped");
            if let Some(info) = rpc(&home, "system.info", json!({})).await { break info; }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }}).await.expect("owner IPC did not start");
        assert_eq!(info["home"].as_str(), home.to_str());
        let action = json!({"schemaVersion":2,"id":"same","name":"Shared action","enabled":false,"typeId":"core.log","config":{"message":"private fixture"}});
        let event_record = json!({"schemaVersion":1,"id":"same","name":"Shared event","enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"user","actionIds":["same"],"runMode":"all"});
        for (kind, record) in [("action", action), ("event", event_record)] {
            assert!(rpc(&home, "automation.create", json!({"kind":kind,"record":record})).await.is_some());
        }
        let web_port = port();
        let owner_url = format!("http://127.0.0.1:{web_port}");
        let mut web = command(&home).args(["web", "--assets", assets.to_str().unwrap(), "--port", &web_port.to_string()])
            .env("TIKTOOLS_PAIRING_FIXTURE", "948231").stdout(Stdio::null()).spawn().unwrap();
        tokio::time::timeout(Duration::from_secs(10), async { loop {
            assert!(web.try_wait().unwrap().is_none(), "owner browser adapter stopped");
            if tokio::net::TcpStream::connect(("127.0.0.1", web_port)).await.is_ok() { break; }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }}).await.expect("owner browser adapter did not start");
        let mut driver = Command::new("bun").arg(repo.join("tests/helpers/owner-editor-browser-driver.ts"))
            .current_dir(&repo).kill_on_drop(true).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
        let mut lines = BufReader::new(driver.stdout.take().unwrap()).lines();
        send(&mut driver, json!({"ownerUrl":owner_url})).await;
        event(&mut lines, "editors-open").await;
        send(&mut driver, json!({"type":"save-first"})).await;
        event(&mut lines, "first-saved").await;
        let first = rpc_meta(&home, "automation.get", json!({"kind":"event","id":"same"})).await;
        assert_eq!(first["result"]["name"], "First owner draft");
        assert_eq!(first["meta"]["resourceVersions"][0]["revision"], "2");
        send(&mut driver, json!({"type":"review-second"})).await;
        event(&mut lines, "reviewed").await;
        let reviewed = rpc_meta(&home, "automation.get", json!({"kind":"event","id":"same"})).await;
        assert_eq!(reviewed, first, "comparison/rebase must not submit another write");
        send(&mut driver, json!({"type":"save-second"})).await;
        event(&mut lines, "second-saved").await;
        let second = rpc_meta(&home, "automation.get", json!({"kind":"event","id":"same"})).await;
        assert_eq!(second["result"]["name"], "Second owner draft");
        assert_eq!(second["meta"]["resourceVersions"][0]["revision"], "3");
        assert!(rpc(&home, "automation.delete", json!({"kind":"event","id":"same"})).await.is_some());
        send(&mut driver, json!({"type":"deleted"})).await;
        event(&mut lines, "recreation-prepared").await;
        assert_eq!(rpc(&home, "automation.list", json!({"kind":"event"})).await.unwrap()["events"], json!([]));
        send(&mut driver, json!({"type":"recreate"})).await;
        event(&mut lines, "recreated").await;
        let recreated = rpc_meta(&home, "automation.get", json!({"kind":"event","id":"same"})).await;
        assert_eq!(recreated["result"]["name"], "Recovered after deletion");
        assert_eq!(recreated["meta"]["resourceVersions"][0]["revision"], "5");
        assert_eq!(recreated["result"]["enabled"], false);
        let untouched = rpc_meta(&home, "automation.get", json!({"kind":"action","id":"same"})).await;
        assert_eq!(untouched["meta"]["resourceVersions"][0]["scope"], "action");
        assert_eq!(untouched["meta"]["resourceVersions"][0]["revision"], "1");
        assert_eq!(rpc(&home, "automation.runs", json!({})).await.unwrap()["runs"], json!([]));
        assert!(host.try_wait().unwrap().is_none());
        drop(driver.stdin.take());
        assert!(driver.wait().await.unwrap().success(), "owner editor driver failed");
        let _ = web.kill().await; let _ = web.wait().await;
        let _ = rpc(&home, "system.shutdown", json!({})).await;
        let _ = host.kill().await; let _ = host.wait().await;
    }).await.expect("owner editor integration deadline exceeded");
}
