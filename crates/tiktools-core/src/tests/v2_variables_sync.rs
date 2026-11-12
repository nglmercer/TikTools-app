//! `variables` as a v2 sync collection against a recording server: staged only
//! when the instance announces it, secrets never staged, uploaded or listed.
use super::*;
use crate::control::{SyncV2StageVariablesParams, VariableInput, VariableKind};
use cloud_client::WorkspaceKey;
use serde_json::{json, Value};
use std::sync::Mutex;
use tiktools_cloud_auth::{CloudAuth, Persistence, TokenStore, TokenStoreError};
use tiktools_cloud_sync::WorkspaceCoordinator;
use tiktools_plugin_package::Sha256Digest;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const SECRET: &str = "hunter2-top-secret-value";

struct Store;
impl TokenStore for Store {
    fn persistence(&self) -> Persistence {
        Persistence::MemoryOnly
    }
    fn load(&self, _: &str, _: &str) -> Result<Option<secrecy::SecretString>, TokenStoreError> {
        Ok(Some(secrecy::SecretString::from("test-token".to_owned())))
    }
    fn store(&self, _: &str, _: &str, _: &cloud_client::Session) -> Result<(), TokenStoreError> {
        Ok(())
    }
    fn delete(&self, _: &str, _: &str) -> Result<(), TokenStoreError> {
        Ok(())
    }
}

fn capability_document(announce_variables: bool) -> Value {
    let mut document = json!({
        "apiVersion": 2, "instanceMode": "single", "userConfigurableRegistries": false,
        "sync": {
            "basePath": "/api/v2/sync", "schemaVersion": 2,
            "collections": ["plugin-selections","rule-templates","profiles","plugin-settings","preferences"],
            "maxRecords": 1000, "maxBytes": 8388608, "maxDocumentBytes": 262144,
            "conflictStatus": 409, "expiredCursorStatus": 410, "preconditionsRequired": true
        }
    });
    if announce_variables {
        document["sync"]["optionalCollections"] = json!(["variables"]);
    }
    document
}

/// `(method, target, body)` of every request the host made.
type Log = Arc<Mutex<Vec<(String, String, String)>>>;

async fn fixture(announce_variables: bool) -> (Arc<AppCore>, Log, PathBuf) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let log: Log = Arc::default();
    let seen = log.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let seen = seen.clone();
            tokio::spawn(async move {
                let mut bytes = Vec::new();
                let (head_end, length) = loop {
                    let mut buf = [0u8; 8192];
                    let Ok(n) = socket.read(&mut buf).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&bytes[..at]).to_ascii_lowercase();
                        let length = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length: "))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        break (at + 4, length);
                    }
                };
                while bytes.len() < head_end + length {
                    let mut buf = [0u8; 8192];
                    match socket.read(&mut buf).await {
                        Ok(n) if n > 0 => bytes.extend_from_slice(&buf[..n]),
                        _ => return,
                    }
                }
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let mut first = text.lines().next().unwrap_or_default().split_whitespace();
                let method = first.next().unwrap_or_default().to_owned();
                let target = first.next().unwrap_or_default().to_owned();
                let body = String::from_utf8_lossy(&bytes[head_end..]).into_owned();
                seen.lock()
                    .unwrap()
                    .push((method.clone(), target.clone(), body.clone()));
                let (status, value) = match (method.as_str(), target.as_str()) {
                    ("GET", "/api/v1/native/me") => ("200 OK", json!({"id": "alice"})),
                    ("GET", "/api/v2/capabilities") => {
                        ("200 OK", capability_document(announce_variables))
                    }
                    ("GET", "/api/v2/sync/write-epoch") => {
                        ("200 OK", json!({"writeEpoch": "7", "epochRequired": true}))
                    }
                    ("GET", t) if t.starts_with("/api/v2/sync/changes?") => (
                        "200 OK",
                        json!({"changes": [], "cursor": 0, "has_more": false}),
                    ),
                    ("PUT", t) if t.starts_with("/api/v2/sync/collections/variables/") => {
                        let key = t.rsplit('/').next().unwrap_or_default();
                        let value: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                        (
                            "200 OK",
                            json!({"collection":"variables","key":key,"revision":1,"deleted":false,"value":value}),
                        )
                    }
                    _ => ("404 Not Found", json!({})),
                };
                let payload = serde_json::to_vec(&value).unwrap();
                let head = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", payload.len());
                let _ = socket.write_all(head.as_bytes()).await;
                let _ = socket.write_all(&payload).await;
            });
        }
    });
    let (mut core, _, root) = core_with_fake_plugins(Arc::new(FakePluginState::default()), &[]);
    let paths = core.db.paths().clone();
    let sidecar = paths.data.join("variables-account.json");
    std::fs::write(
        &sidecar,
        serde_json::to_vec(&json!({"origin": origin, "account_id": "alice"})).unwrap(),
    )
    .unwrap();
    let auth = CloudAuth::new(Some(origin.clone()), sidecar, Arc::new(Store));
    auth.restore().await.unwrap();
    Arc::get_mut(&mut core).unwrap().cloud = Arc::new(
        crate::cloud_account::CloudAccount::with_test_auth(auth.clone()),
    );
    Arc::get_mut(&mut core).unwrap().sync =
        Arc::new(crate::cloud_sync::CloudSync::new(&paths, &auth));
    let sync = core.sync.coordinator().unwrap().clone();
    sync.set_enabled(true).unwrap();
    let sync_dir = paths.data.join("cloud-sync");
    let workspace = WorkspaceCoordinator::open(&sync_dir, origin.clone(), "alice".into()).unwrap();
    workspace.bootstrap(&[], None).unwrap();
    let store = cloud_client::WorkspaceStore::open(&sync_dir, &origin, "alice").unwrap();
    let mut snapshot = store.load().unwrap().unwrap();
    snapshot.state.mark_product_guarded();
    store
        .save(Some(snapshot.generation), &snapshot.state)
        .unwrap();
    let identity = Sha256Digest::of_bytes(&serde_json::to_vec(&(&origin, "alice")).unwrap());
    let scope = Sha256Digest::of_bytes(
        &serde_json::to_vec(&("tiktools-v2-consent", &origin, "alice")).unwrap(),
    );
    std::fs::write(
        sync_dir.join("cloud-sync.json"),
        serde_json::to_vec(
            &json!({"version":2,"enabled":true,"generation":42,"scope":scope.as_bytes()}),
        )
        .unwrap(),
    )
    .unwrap();
    let protocol_marker = sync_dir.join(format!("protocol-{identity}.json"));
    std::fs::write(&protocol_marker, serde_json::to_vec(&json!({"formatVersion":3,"origin":origin,"accountId":"alice","mode":"v2","candidateSha256":"a".repeat(64)})).unwrap()).unwrap();
    let product_marker = sync_dir.join(format!("v1-product-outbox-{identity}.json"));
    std::fs::write(
        &product_marker,
        serde_json::to_vec(&json!({"formatVersion":1,"origin":origin,"accountId":"alice"}))
            .unwrap(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [&protocol_marker, &product_marker] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
    (core, log, root)
}

fn variable(name: &str, kind: VariableKind, value: Value) -> VariableInput {
    VariableInput {
        name: name.into(),
        kind,
        value,
    }
}

fn mixed() -> SyncV2StageVariablesParams {
    SyncV2StageVariablesParams {
        variables: vec![
            variable("commandPort", VariableKind::Number, json!(46665)),
            variable("greeting", VariableKind::Text, json!("hola")),
            variable("apiKey", VariableKind::Secret, json!(SECRET)),
        ],
        remove: vec![],
    }
}

fn requests_to(log: &Log, prefix: &str) -> Vec<(String, String, String)> {
    log.lock()
        .unwrap()
        .iter()
        .filter(|(_, target, _)| target.starts_with(prefix))
        .cloned()
        .collect()
}

#[tokio::test]
async fn announced_variables_sync_but_a_secret_never_reaches_a_request_or_the_changes_list() {
    let (core, log, root) = fixture(true).await;
    let result = core.sync_v2_stage_variables(mixed()).await.unwrap();
    assert_eq!(result.staged, 2);
    assert_eq!(result.excluded_secrets, vec!["apiKey".to_owned()]);

    // Local, network-free listing: the two variables with values, no secret.
    let entries = core.sync_v2_changes_list().await.unwrap();
    let names: Vec<_> = entries
        .iter()
        .filter(|e| e.collection == "variables")
        .map(|e| e.key.as_str())
        .collect();
    assert_eq!(names, vec!["commandPort", "greeting"]);
    let port = entries.iter().find(|e| e.key == "commandPort").unwrap();
    let local = &port.pending.as_ref().unwrap().local;
    assert_eq!(local.value.as_ref().unwrap()["value"], 46665);
    assert_eq!(local.value.as_ref().unwrap()["type"], "number");
    let listed = serde_json::to_string(&entries).unwrap();
    assert!(!listed.contains(SECRET) && !listed.contains("apiKey"));

    // Upload, then check every byte the host sent.
    if let Err(e) = core.sync_v2_now().await {
        panic!("{e:?} {:#?}", log.lock().unwrap());
    }
    let puts = requests_to(&log, "/api/v2/sync/collections/variables/");
    let mut keys: Vec<_> = puts
        .iter()
        .map(|(_, t, _)| t.rsplit('/').next().unwrap().to_owned())
        .collect();
    keys.sort();
    assert_eq!(keys, vec!["commandPort", "greeting"]);
    for (method, target, body) in log.lock().unwrap().iter() {
        assert!(
            !body.contains(SECRET) && !target.contains(SECRET) && !target.contains("apiKey"),
            "{method} {target}"
        );
    }
    let (_, _, body) = puts
        .iter()
        .find(|(_, t, _)| t.ends_with("/commandPort"))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        json!({"schemaVersion":1,"name":"commandPort","type":"number","value":46665})
    );
    // And the durable workspace holds no secret either.
    let state = WorkspaceCoordinator::open(
        core.db.paths().data.join("cloud-sync"),
        puts.first()
            .map(|_| core.cloud.auth().last_identity().unwrap().0)
            .unwrap(),
        "alice".into(),
    )
    .unwrap()
    .snapshot()
    .unwrap()
    .state;
    assert!(state
        .entry(&WorkspaceKey::new("variables", "apiKey").unwrap())
        .is_none());
    assert!(!serde_json::to_string(&state).unwrap().contains(SECRET));
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn only_secrets_means_no_staging_and_no_network_at_all() {
    let (core, log, root) = fixture(false).await;
    let before = log.lock().unwrap().len();
    let result = core
        .sync_v2_stage_variables(SyncV2StageVariablesParams {
            variables: vec![variable("apiKey", VariableKind::Secret, json!(SECRET))],
            remove: vec![],
        })
        .await
        .unwrap();
    assert_eq!((result.staged, result.removed), (0, 0));
    assert_eq!(result.excluded_secrets, vec!["apiKey".to_owned()]);
    assert_eq!(log.lock().unwrap().len(), before);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn an_instance_that_does_not_announce_variables_stages_nothing() {
    let (core, log, root) = fixture(false).await;
    let error = core.sync_v2_stage_variables(mixed()).await.unwrap_err();
    assert_eq!(error.code, "capability_unavailable");
    assert_eq!(error.data.unwrap()["reason"], "variables_unsupported");
    let entries = core.sync_v2_changes_list().await.unwrap();
    assert!(entries.iter().all(|e| e.collection != "variables"));
    // Sync behaves exactly as before: nothing variables-related is ever sent.
    core.sync_v2_now().await.unwrap();
    assert!(requests_to(&log, "/api/v2/sync/collections/variables").is_empty());
    for (_, _, body) in log.lock().unwrap().iter() {
        assert!(!body.contains(SECRET));
    }
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}
