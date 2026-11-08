//! Opt-in actual owner browser → local adapter → IPC → Core authority review.
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
    ClientKind, PairingPoll, RemoteClient, RemoteEndpoint, SecretString, TlsTrust,
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
async fn rpc(home: &Path, method: &str, params: Value, private_invitation: bool) -> Option<Value> {
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
fn port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
fn action(locale: &str, alternative: bool) -> Value {
    json!({"schemaVersion":2,"id":format!("{}-{locale}",if alternative {"disabled"} else {"proposal"}),
        "name":format!("{} {locale}",if alternative {"Disabled alternative"} else {"Moderator action"}),
        "enabled":false,"typeId":"core.log","config":{"message":format!("proposal-source-{locale}")},
        "source":format!("proposal-source-{locale}"),"bodyScript":format!("proposal-script-{locale}"),
        "provenance":{"authority":{"kind":"owner"},"generation":"999"}})
}
fn event(locale: &str) -> Value {
    json!({"schemaVersion":1,"id":format!("proposal-{locale}"),"name":format!("Moderator event {locale}"),
        "enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global",
        "runMode":"random","actionIds":[format!("proposal-{locale}"),format!("disabled-{locale}"),format!("proposal-{locale}"),format!("missing-{locale}")],
        "source":format!("proposal-source-{locale}"),"bodyScript":format!("proposal-script-{locale}"),
        "provenance":{"authority":{"kind":"owner"},"generation":"999"}})
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires built dist/web, Bun and Playwright Chromium; run explicitly"]
async fn actual_owner_proposal_review_adoption_and_reload_preserve_native_authority_boundaries() {
    tokio::time::timeout(Duration::from_secs(240), async {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("home");
        std::fs::create_dir_all(home.join("data")).unwrap();
        for file in ["tiktok-points.db", "tiktok-automation.db", "tiktok-analytics.db"] {
            std::fs::File::create(home.join("data").join(file)).unwrap();
        }
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
        let assets = repo.join("dist/web");
        assert!(assets.join("index.html").is_file(), "build the packaged web frontend first");
        let mut host = command(&home).args(["host", "--ipc"]).stdout(Stdio::null()).spawn().unwrap();
        let info = tokio::time::timeout(Duration::from_secs(20), async { loop {
            assert!(host.try_wait().unwrap().is_none(), "isolated proposal host stopped during startup");
            if let Some(info) = rpc(&home, "system.info", json!({}), false).await { break info; }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }}).await.expect("proposal IPC host did not start");
        assert_eq!(info["home"].as_str(), home.to_str());
        let certificate = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
        let certificate_file = temporary.path().join("certificate.pem");
        let key_file = temporary.path().join("key.pem");
        std::fs::write(&certificate_file, certificate.cert.pem()).unwrap();
        std::fs::write(&key_file, certificate.signing_key.serialize_pem()).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_file, std::fs::Permissions::from_mode(0o600)).unwrap();
        let fingerprint: String = ring::digest::digest(&ring::digest::SHA256, certificate.cert.der()).as_ref().iter().map(|byte| format!("{byte:02x}")).collect();
        let remote_port = port();
        let mut web_port = port(); while web_port == remote_port { web_port = port(); }
        let remote_origin = format!("https://127.0.0.1:{remote_port}");
        assert!(rpc(&home, "collaboration.configure", json!({"enabled":true,"bindAddress":format!("127.0.0.1:{remote_port}"),
            "advertisedOrigin":remote_origin,"tlsCertificate":certificate_file,"tlsPrivateKey":key_file}), false).await.is_some());
        tokio::time::timeout(Duration::from_secs(10), async { loop {
            if tokio::net::TcpStream::connect(("127.0.0.1", remote_port)).await.is_ok() { break; }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }}).await.expect("proposal TLS host did not start");
        let invitation = rpc(&home, "collaboration.invites.create", json!({"grant":{"capabilities":["automation.read","automation.edit","commands.read"]}}), true).await.unwrap();
        let endpoint = RemoteEndpoint::new(&remote_origin, TlsTrust::CertificateSha256 { sha256: fingerprint }).await.unwrap();
        let mut pairing = endpoint.begin_pairing(invitation["inviteId"].as_str().unwrap(),
            &SecretString::from(invitation["secret"].as_str().unwrap().to_owned()), "Browser proposal fixture author", ClientKind::Agent).await.unwrap();
        assert!(rpc(&home, "collaboration.requests.approve", json!({"id":pairing.request_id}), false).await.is_some());
        let credentials = match pairing.poll().await.unwrap() {
            PairingPoll::Approved(credentials) => credentials,
            _ => panic!("isolated proposal author was not approved"),
        };
        let remote = RemoteClient::connect(&endpoint, &credentials, ClientKind::Agent).await.unwrap();
        for locale in ["en", "es"] {
            for (kind, record) in [("action", action(locale, false)), ("action", action(locale, true)), ("event", event(locale))] {
                let id = record["id"].as_str().unwrap().to_owned();
                let created = remote.call_value_with_meta("automation.create", json!({"kind":kind,"record":record}), Some(tiktools_client::RequestMetadata {
                    command_id: Some(format!("proposal-fixture-{kind}-{id}")), host_instance_id: Some(remote.hello().host_instance_id.clone()), expected_revision: Some("0".into()),
                })).await.unwrap();
                assert_eq!(created.result, json!({"kind":kind,"id":id,"enabled":false,"pendingReview":true}));
                assert_eq!(created.meta.unwrap().resource_versions[0].revision, "1");
                let trusted = rpc(&home, "automation.review", json!({"kind":kind,"id":id}), false).await.unwrap();
                assert_eq!(trusted["provenance"]["authority"]["kind"], "delegated");
                assert_eq!(trusted["provenance"]["authority"]["principalId"], credentials.principal_id());
                assert_eq!(trusted["provenance"]["authority"]["sessionId"], credentials.session_id());
            }
        }
        let owner_url = format!("http://127.0.0.1:{web_port}");
        let mut web = command(&home).args(["web", "--assets", assets.to_str().unwrap(), "--port", &web_port.to_string()])
            .env("TIKTOOLS_PAIRING_FIXTURE", "948231").stdout(Stdio::null()).spawn().unwrap();
        tokio::time::timeout(Duration::from_secs(10), async { loop {
            assert!(web.try_wait().unwrap().is_none(), "proposal owner adapter stopped");
            if tokio::net::TcpStream::connect(("127.0.0.1", web_port)).await.is_ok() { break; }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }}).await.expect("proposal owner browser adapter did not start");
        let output = Command::new("bun").args(["x", "--no-install", "playwright", "test", "--config", "tests/helpers/owner-proposal-playwright.config.ts"])
            .current_dir(&repo).env("TIKTOOLS_OWNER_PROPOSAL_URL", &owner_url)
            .env("TIKTOOLS_OWNER_PROPOSAL_HOME", &home).env("TIKTOOLS_OWNER_PROPOSAL_BINARY", env!("CARGO_BIN_EXE_tiktools"))
            .kill_on_drop(true).output().await.unwrap();
        assert!(output.status.success(), "owner proposal browser failed:\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let journal = rpc(&home, "automation.adoption.get", json!({}), false).await.unwrap();
        assert!(journal["pending"].is_null());
        assert_eq!(journal["recent"].as_array().unwrap().len(), 4);
        assert!(!journal.to_string().contains("proposal-source-"));
        for locale in ["en", "es"] {
            let review = rpc(&home, "automation.review", json!({"kind":"event","id":format!("proposal-{locale}")}), false).await.unwrap();
            assert_eq!(review["record"]["enabled"], false);
            assert_eq!(review["provenance"]["authority"]["kind"], "owner");
            assert_eq!(review["dependencies"][0]["record"]["enabled"], false);
            assert_eq!(review["dependencies"][0]["provenance"]["authority"]["kind"], "owner");
            assert_eq!(review["dependencies"][1]["record"]["enabled"], false);
            assert_eq!(review["dependencies"][1]["provenance"]["authority"]["kind"], "delegated");
        }
        assert_eq!(rpc(&home, "automation.runs", json!({}), false).await.unwrap()["runs"], json!([]));
        assert!(host.try_wait().unwrap().is_none());
        remote.close();
        let _ = web.kill().await; let _ = web.wait().await;
        let _ = rpc(&home, "system.shutdown", json!({}), false).await;
        let _ = host.kill().await; let _ = host.wait().await;
    }).await.expect("actual owner proposal browser deadline exceeded");
}
