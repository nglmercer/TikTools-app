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
    let (core, log, root, _) = fixture_server(announce_variables).await;
    (core, log, root)
}

#[derive(Default)]
struct VariablesServer {
    records: Mutex<std::collections::BTreeMap<String, Value>>,
    epoch: std::sync::atomic::AtomicU64,
    drop_next_write: std::sync::atomic::AtomicBool,
}

async fn fixture_server(
    announce_variables: bool,
) -> (Arc<AppCore>, Log, PathBuf, Arc<VariablesServer>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let log: Log = Arc::default();
    let seen = log.clone();
    let server = Arc::new(VariablesServer::default());
    server.epoch.store(7, std::sync::atomic::Ordering::SeqCst);
    let data = server.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let seen = seen.clone();
            let data = data.clone();
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
                    ("GET", "/api/v2/sync/write-epoch") => (
                        "200 OK",
                        json!({"writeEpoch": data.epoch.load(std::sync::atomic::Ordering::SeqCst).to_string(), "epochRequired": true}),
                    ),
                    ("POST", "/api/v2/sync/write-barrier") => (
                        "200 OK",
                        json!({"writeEpoch": (data.epoch.fetch_add(1, std::sync::atomic::Ordering::SeqCst)+1).to_string(), "epochRequired": true}),
                    ),
                    ("GET", t) if t.starts_with("/api/v2/sync/changes?") => (
                        "200 OK",
                        json!({"changes": [], "cursor": 0, "has_more": false}),
                    ),
                    (method @ ("GET" | "PUT" | "DELETE"), t)
                        if t.starts_with("/api/v2/sync/collections/variables/") =>
                    {
                        let key = t.rsplit('/').next().unwrap_or_default();
                        let mut records = data.records.lock().unwrap();
                        if method == "GET" {
                            match records.get(key) {
                                Some(record) => ("200 OK", record.clone()),
                                None => ("404 Not Found", json!({})),
                            }
                        } else {
                            let value: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                            let revision = records
                                .get(key)
                                .and_then(|record| record["revision"].as_u64())
                                .unwrap_or(0)
                                + 1;
                            let headers = text[..head_end].to_ascii_lowercase();
                            let expected = format!("if-match: \"{}\"", revision - 1);
                            if (revision > 1
                                && !headers.lines().any(|line| line.trim() == expected))
                                || (revision == 1 && !headers.contains("if-none-match: *"))
                            {
                                (
                                    "409 Conflict",
                                    records.get(key).cloned().unwrap_or(json!({})),
                                )
                            } else {
                                let record = json!({"collection":"variables","key":key,"revision":revision,"deleted":method == "DELETE","value": if method == "DELETE" {Value::Null} else {value}});
                                records.insert(key.into(), record.clone());
                                ("200 OK", record)
                            }
                        }
                    }
                    _ => ("404 Not Found", json!({})),
                };
                if (method == "PUT" || method == "DELETE")
                    && data
                        .drop_next_write
                        .swap(false, std::sync::atomic::Ordering::SeqCst)
                {
                    return;
                }
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
    (core, log, root, server)
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

async fn cache_variable(core: &Arc<AppCore>, name: &str, value: Option<Value>, revision: u64) {
    let session = core.cloud.auth().session().await.unwrap();
    let store = cloud_client::WorkspaceStore::open(
        core.db.paths().data.join("cloud-sync"),
        session.instance_origin(),
        session.account_id(),
    )
    .unwrap();
    let mut snapshot = store.load().unwrap().unwrap();
    let target = WorkspaceKey::new("variables", name).unwrap();
    let record: cloud_client::WorkspaceRecord = serde_json::from_value(json!({"collection":"variables","key":name,"revision":revision,"deleted":value.is_none(),"value":value})).unwrap();
    if let Some(entry) = snapshot
        .state
        .entries
        .iter_mut()
        .find(|entry| entry.target == target)
    {
        entry.remote = Some(record);
    } else {
        snapshot.state.entries.push(cloud_client::WorkspaceEntry {
            target,
            remote: Some(record),
            pending: None,
            next: None,
        });
    }
    store
        .save(Some(snapshot.generation), &snapshot.state)
        .unwrap();
}

fn variable_document(name: &str, kind: &str, value: Value) -> Value {
    json!({"schemaVersion":1,"name":name,"type":kind,"value":value})
}

#[tokio::test]
async fn variables_review_is_local_stable_and_never_projects_local_secrets_or_credentials() {
    use crate::control::{RuntimeVariable, RuntimeVariableKind as Kind, RuntimeVariablesReplace};
    let (core, log, root) = fixture(true).await;
    let initial = core.runtime_variables_list().unwrap();
    core.runtime_variables_replace(RuntimeVariablesReplace {
        expected_revision: initial.revision,
        variables: [
            (
                "port".into(),
                RuntimeVariable {
                    kind: Kind::Number,
                    value: json!(1.0),
                },
            ),
            (
                "privateValue".into(),
                RuntimeVariable {
                    kind: Kind::Secret,
                    value: json!(SECRET),
                },
            ),
            (
                "credentialLooking".into(),
                RuntimeVariable {
                    kind: Kind::Text,
                    value: json!("Bearer private-review-test"),
                },
            ),
            (
                "localOnly".into(),
                RuntimeVariable {
                    kind: Kind::Boolean,
                    value: json!(true),
                },
            ),
        ]
        .into(),
    })
    .unwrap();
    cache_variable(
        &core,
        "port",
        Some(variable_document("port", "number", json!(1))),
        1,
    )
    .await;
    cache_variable(
        &core,
        "privateValue",
        Some(variable_document("privateValue", "text", json!(SECRET))),
        1,
    )
    .await;
    let before = log.lock().unwrap().len();
    let first = core.sync_v2_variables_review().await.unwrap();
    let again = core.sync_v2_variables_review().await.unwrap();
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&again).unwrap()
    );
    assert_eq!(
        first.changes.len(),
        3,
        "JSONB number equivalence must not create a difference"
    );
    let secret = first
        .changes
        .iter()
        .find(|change| change.name == "privateValue")
        .unwrap();
    assert!(secret.local_secret && secret.can_withdraw);
    assert!(
        !secret.can_send && !secret.can_apply && secret.local.is_none() && secret.cloud.is_none()
    );
    let blocked = first
        .changes
        .iter()
        .find(|change| change.name == "credentialLooking")
        .unwrap();
    assert!(blocked.blocked_local && blocked.local.is_none());
    let wire = serde_json::to_string(&first).unwrap();
    assert!(!wire.contains(SECRET) && !wire.contains("Bearer private-review-test"));
    assert_eq!(log.lock().unwrap().len(), before);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn variables_apply_adopts_only_the_reviewed_cached_value_and_is_one_time() {
    use crate::control::VariableSyncApply;
    let (core, log, root) = fixture(true).await;
    core.globals_set("port", "7").unwrap();
    core.globals_set("untouched", "keep").unwrap();
    cache_variable(
        &core,
        "port",
        Some(variable_document("port", "number", json!(9))),
        1,
    )
    .await;
    let changes = core.sync_v2_variables_review().await.unwrap().changes;
    let change = changes.iter().find(|change| change.name == "port").unwrap();
    assert!(change.can_apply);
    let before = log.lock().unwrap().len();
    core.sync_v2_variables_apply(VariableSyncApply {
        review: change.review.clone(),
    })
    .await
    .unwrap();
    assert_eq!(core.globals_get("port").unwrap(), "9");
    assert_eq!(
        core.runtime_variables_list().unwrap().variables["port"].kind,
        crate::control::RuntimeVariableKind::Number
    );
    assert_eq!(core.globals_get("untouched").unwrap(), "keep");
    assert!(core
        .sync_v2_variables_apply(VariableSyncApply {
            review: change.review.clone()
        })
        .await
        .is_err());
    assert!(!core
        .sync_v2_variables_review()
        .await
        .unwrap()
        .changes
        .iter()
        .any(|change| change.name == "port"));
    assert_eq!(
        log.lock().unwrap().len(),
        before,
        "application must not publish or fetch data"
    );
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn variables_apply_rejects_local_cache_consent_and_secret_changes() {
    use crate::control::{
        RuntimeVariable, RuntimeVariableKind, RuntimeVariablesReplace, VariableSyncApply,
    };
    let (core, _, root) = fixture(true).await;
    core.globals_set("port", "7").unwrap();
    cache_variable(
        &core,
        "port",
        Some(variable_document("port", "number", json!(9))),
        1,
    )
    .await;
    let token = core.sync_v2_variables_review().await.unwrap().changes[0]
        .review
        .clone();
    core.globals_set("port", "8").unwrap();
    assert!(core
        .sync_v2_variables_apply(VariableSyncApply { review: token })
        .await
        .is_err());
    let token = core.sync_v2_variables_review().await.unwrap().changes[0]
        .review
        .clone();
    cache_variable(
        &core,
        "port",
        Some(variable_document("port", "number", json!(10))),
        2,
    )
    .await;
    assert!(core
        .sync_v2_variables_apply(VariableSyncApply { review: token })
        .await
        .is_err());
    let token = core.sync_v2_variables_review().await.unwrap().changes[0]
        .review
        .clone();
    let local = core.runtime_variables_list().unwrap();
    core.runtime_variables_replace(RuntimeVariablesReplace {
        expected_revision: local.revision,
        variables: [(
            "port".into(),
            RuntimeVariable {
                kind: RuntimeVariableKind::Secret,
                value: json!(SECRET),
            },
        )]
        .into(),
    })
    .unwrap();
    assert!(core
        .sync_v2_variables_apply(VariableSyncApply { review: token })
        .await
        .is_err());
    assert_eq!(core.globals_get("port").unwrap(), SECRET);
    let review = core.sync_v2_variables_review().await.unwrap().changes[0]
        .review
        .clone();
    core.sync.coordinator().unwrap().set_enabled(false).unwrap();
    assert!(core
        .sync_v2_variables_apply(VariableSyncApply { review })
        .await
        .is_err());
    assert_eq!(core.globals_get("port").unwrap(), SECRET);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn variables_apply_rejects_same_account_relogin_and_accepts_reviewed_deletion() {
    use crate::control::VariableSyncApply;
    let (core, log, root) = fixture(true).await;
    core.globals_set("obsolete", "local").unwrap();
    cache_variable(&core, "obsolete", None, 1).await;
    let token = core.sync_v2_variables_review().await.unwrap().changes[0]
        .review
        .clone();
    let auth = core.cloud.auth();
    let origin = auth.session().await.unwrap().instance_origin();
    auth.logout().await.unwrap();
    std::fs::write(
        core.db.paths().data.join("variables-account.json"),
        serde_json::to_vec(&json!({"origin":origin,"account_id":"alice"})).unwrap(),
    )
    .unwrap();
    auth.restore().await.unwrap();
    let before = log.lock().unwrap().len();
    assert!(core
        .sync_v2_variables_apply(VariableSyncApply { review: token })
        .await
        .is_err());
    assert_eq!(core.globals_get("obsolete").unwrap(), "local");
    let token = core.sync_v2_variables_review().await.unwrap().changes[0]
        .review
        .clone();
    core.sync_v2_variables_apply(VariableSyncApply { review: token })
        .await
        .unwrap();
    assert!(core.globals_get("obsolete").is_err());
    assert_eq!(log.lock().unwrap().len(), before);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn variables_fetch_never_uploads_and_section_publication_retains_other_queues() {
    use crate::control::SyncSection;
    let (core, log, root) = fixture(true).await;
    core.sync_v2_stage_preferences(json!({"theme":"dark"}))
        .await
        .unwrap();
    core.sync_v2_stage_variables(SyncV2StageVariablesParams {
        variables: vec![variable("port", VariableKind::Number, json!(46665))],
        remove: vec![],
    })
    .await
    .unwrap();
    let before = core.sync_v2_changes_list().await.unwrap();
    let preferences = before
        .iter()
        .find(|entry| entry.collection == "preferences")
        .unwrap()
        .clone();
    core.sync_v2_variables_fetch().await.unwrap();
    assert!(!log
        .lock()
        .unwrap()
        .iter()
        .any(|(method, _, _)| method == "PUT" || method == "DELETE"));
    let pending = core.sync_v2_variables_review().await.unwrap().changes;
    assert!(
        !pending
            .iter()
            .find(|change| change.name == "port")
            .unwrap()
            .can_apply
    );
    let result = core.sync_v2_section(SyncSection::Variables).await.unwrap();
    assert_eq!(result.pushed, 1);
    assert!(!log
        .lock()
        .unwrap()
        .iter()
        .any(|(_, path, _)| path.contains("collections/preferences/")));
    let after = core.sync_v2_changes_list().await.unwrap();
    assert_eq!(
        after
            .iter()
            .find(|entry| entry.collection == "preferences")
            .unwrap(),
        &preferences
    );
    assert!(
        core.globals_get("port").is_err(),
        "publishing and downloading must not apply a value"
    );
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn variables_network_actions_require_the_optional_backend_collection() {
    let (core, log, root) = fixture(false).await;
    let fetch = core.sync_v2_variables_fetch().await.unwrap_err();
    assert_eq!(fetch.code, "capability_unavailable");
    let publish = core
        .sync_v2_section(crate::control::SyncSection::Variables)
        .await
        .unwrap_err();
    assert_eq!(publish.code, "capability_unavailable");
    assert!(!log
        .lock()
        .unwrap()
        .iter()
        .any(|(_, path, _)| path.starts_with("/api/v2/sync/")));
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn local_secret_classification_overrides_a_clients_shareable_type() {
    use crate::control::{RuntimeVariable, RuntimeVariableKind, RuntimeVariablesReplace};
    let (core, log, root) = fixture(true).await;
    let snapshot = core.runtime_variables_list().unwrap();
    core.runtime_variables_replace(RuntimeVariablesReplace {
        expected_revision: snapshot.revision,
        variables: [(
            "privateValue".into(),
            RuntimeVariable {
                kind: RuntimeVariableKind::Secret,
                value: json!(SECRET),
            },
        )]
        .into(),
    })
    .unwrap();
    let before = log.lock().unwrap().len();
    let result = core
        .sync_v2_stage_variables(SyncV2StageVariablesParams {
            variables: vec![variable("privateValue", VariableKind::Text, json!(SECRET))],
            remove: vec![],
        })
        .await
        .unwrap();
    assert_eq!(result.staged, 0);
    assert_eq!(result.excluded_secrets, vec!["privateValue"]);
    assert_eq!(
        log.lock().unwrap().len(),
        before,
        "no capability or upload request for a local secret"
    );
    assert!(core.sync_v2_changes_list().await.unwrap().is_empty());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn variables_type_damage_does_not_block_a_workspace_without_variable_records() {
    let (core, log, root) = fixture(true).await;
    core.globals_set("ordinary", "local").unwrap();
    core.db
        .save_app_state("runtime.variable.type.ordinary", "invalid")
        .unwrap();
    assert!(core.runtime_variables_list().is_err());
    let before = log.lock().unwrap().len();
    assert!(core.sync_v2_changes_list().await.unwrap().is_empty());
    assert_eq!(log.lock().unwrap().len(), before);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn marking_an_already_queued_variable_secret_blocks_upload_and_redacts_its_projection() {
    use crate::control::{RuntimeVariable, RuntimeVariableKind, RuntimeVariablesReplace};
    let (core, log, root) = fixture(true).await;
    let formerly_public = "value-was-public-now-local-only";
    core.sync_v2_stage_variables(SyncV2StageVariablesParams {
        variables: vec![variable(
            "privateValue",
            VariableKind::Text,
            json!(formerly_public),
        )],
        remove: vec![],
    })
    .await
    .unwrap();
    let snapshot = core.runtime_variables_list().unwrap();
    core.runtime_variables_replace(RuntimeVariablesReplace {
        expected_revision: snapshot.revision,
        variables: [(
            "privateValue".into(),
            RuntimeVariable {
                kind: RuntimeVariableKind::Secret,
                value: json!(formerly_public),
            },
        )]
        .into(),
    })
    .unwrap();
    let changes = core.sync_v2_changes_list().await.unwrap();
    assert_eq!(changes.len(), 1);
    assert!(changes[0].pending.as_ref().unwrap().local.value.is_none());
    assert!(!serde_json::to_string(&changes)
        .unwrap()
        .contains(formerly_public));
    assert!(core.sync_v2_now().await.is_err());
    assert!(requests_to(&log, "/api/v2/sync/collections/variables/").is_empty());
    assert!(!log
        .lock()
        .unwrap()
        .iter()
        .any(|(_, _, body)| body.contains(formerly_public)));
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_workspace_reads_do_not_need_product_mutation_admission() {
    let (core, log, root) = fixture(true).await;
    core.sync_v2_stage_variables(mixed()).await.unwrap();
    let requests_before = log.lock().unwrap().len();
    let sync = core.sync.coordinator().unwrap().clone();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writer = tokio::task::spawn_blocking(move || {
        sync.with_package_journal_admission(|| {
            entered_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(15))
                .unwrap();
        })
        .unwrap();
    });
    entered_rx.await.unwrap();
    let mut readers = tokio::task::JoinSet::new();
    for _ in 0..32 {
        let core = core.clone();
        readers.spawn(async move {
            let changes = core.sync_v2_changes_list().await?;
            let conflicts = core.sync_v2_conflict_list().await?;
            let selected = core.workspace_v2_selected().await?;
            Ok::<_, crate::control::OperationError>((changes.len(), conflicts.len(), selected))
        });
    }
    let mut results = Vec::new();
    while let Some(result) = readers.join_next().await {
        results.push(result.unwrap());
    }
    release_tx.send(()).unwrap();
    writer.await.unwrap();
    for result in results {
        assert_eq!(result.unwrap(), (2, 0, true));
    }
    assert_eq!(
        log.lock().unwrap().len(),
        requests_before,
        "lists must remain local-only"
    );
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn workspace_reads_reject_disabled_or_damaged_consent_without_network() {
    let (core, log, root) = fixture(true).await;
    core.sync_v2_stage_variables(mixed()).await.unwrap();
    let requests_before = log.lock().unwrap().len();
    let config_path = core.db.paths().data.join("cloud-sync/cloud-sync.json");
    let config = std::fs::read(&config_path).unwrap();
    core.sync.coordinator().unwrap().set_enabled(false).unwrap();
    assert!(!core.workspace_v2_selected().await.unwrap());
    assert!(core.sync_v2_changes_list().await.is_err());
    assert!(core.sync_v2_conflict_list().await.is_err());
    // Restore this fixture's consent, then simulate file loss/corruption.
    std::fs::write(&config_path, config).unwrap();
    assert_eq!(core.sync_v2_changes_list().await.unwrap().len(), 2);
    std::fs::write(&config_path, b"invalid").unwrap();
    assert!(core.workspace_v2_selected().await.is_err());
    assert!(core.sync_v2_changes_list().await.is_err());
    std::fs::remove_file(config_path).unwrap();
    assert!(core.sync_v2_conflict_list().await.is_err());
    assert_eq!(log.lock().unwrap().len(), requests_before);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn workspace_reads_never_return_another_origins_cached_values() {
    let (mut core, log, root) = fixture(true).await;
    core.sync_v2_stage_variables(mixed()).await.unwrap();
    let (other, other_log, other_root) = fixture(true).await;
    let requests_before = log.lock().unwrap().len();
    let other_requests_before = other_log.lock().unwrap().len();
    // A stale coordinator is still scoped to the first origin's workspace.
    Arc::get_mut(&mut core).unwrap().cloud = other.cloud.clone();
    assert!(core.workspace_v2_selected().await.is_err());
    assert!(core.sync_v2_changes_list().await.is_err());
    assert!(core.sync_v2_conflict_list().await.is_err());
    assert_eq!(log.lock().unwrap().len(), requests_before);
    assert_eq!(other_log.lock().unwrap().len(), other_requests_before);
    core.shutdown().await;
    other.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(other_root);
}

#[tokio::test]
async fn workspace_reads_never_restore_saved_credentials_implicitly() {
    let (mut core, log, root) = fixture(true).await;
    let origin = core.cloud.auth().session().await.unwrap().instance_origin();
    let saved_auth = CloudAuth::new(
        Some(origin),
        core.db.paths().data.join("variables-account.json"),
        Arc::new(Store),
    );
    Arc::get_mut(&mut core).unwrap().cloud = Arc::new(
        crate::cloud_account::CloudAccount::with_test_auth(saved_auth),
    );
    let before = log.lock().unwrap().len();
    assert!(core.sync_v2_changes_list().await.is_err());
    assert!(core.sync_v2_conflict_list().await.is_err());
    assert!(core.cloud.auth().session().await.is_none());
    assert_eq!(log.lock().unwrap().len(), before);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_read_discards_snapshot_after_same_account_session_replacement() {
    let (core, _, root) = fixture(true).await;
    core.sync_v2_stage_variables(mixed()).await.unwrap();
    let auth = core.cloud.auth();
    let original_session = auth.session().await.unwrap();
    let sync_dir = core.db.paths().data.join("cloud-sync");
    let store = cloud_client::WorkspaceStore::open(
        &sync_dir,
        original_session.instance_origin(),
        original_session.account_id(),
    )
    .unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writer = tokio::task::spawn_blocking(move || {
        store
            .try_with_locked_snapshot::<_, cloud_client::StoreError>(|_| {
                entered_tx.send(()).unwrap();
                release_rx
                    .recv_timeout(std::time::Duration::from_secs(15))
                    .unwrap();
                Ok(())
            })
            .unwrap()
            .unwrap();
    });
    entered_rx.await.unwrap();
    let (read_started_tx, read_started_rx) = tokio::sync::oneshot::channel();
    *core.sync.v2_read_started.lock().unwrap() = Some(read_started_tx);
    let read = {
        let core = core.clone();
        tokio::spawn(async move { core.sync_v2_changes_list().await })
    };
    // Observe admission directly: probing with an exclusive cutover lease
    // could itself make the nonblocking reader fail before reaching disk.
    tokio::time::timeout(std::time::Duration::from_secs(5), read_started_rx)
        .await
        .unwrap()
        .unwrap();
    // Reading files does not hold the account gate or delay logout.
    tokio::time::timeout(std::time::Duration::from_secs(2), auth.logout())
        .await
        .unwrap()
        .unwrap();
    std::fs::write(
        core.db.paths().data.join("variables-account.json"),
        serde_json::to_vec(
            &json!({"origin":original_session.instance_origin(),"account_id":"alice"}),
        )
        .unwrap(),
    )
    .unwrap();
    auth.restore().await.unwrap();
    assert!(!Arc::ptr_eq(
        &original_session,
        &auth.session().await.unwrap()
    ));
    release_tx.send(()).unwrap();
    writer.await.unwrap();
    let result = read.await.unwrap();
    assert!(
        result.is_err(),
        "old login must never expose its snapshot to a new session"
    );
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
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

async fn variable_review(core: &Arc<AppCore>, name: &str) -> crate::control::VariableSyncApply {
    crate::control::VariableSyncApply {
        review: core
            .sync_v2_variables_review()
            .await
            .unwrap()
            .changes
            .into_iter()
            .find(|change| change.name == name)
            .unwrap()
            .review,
    }
}

#[tokio::test]
async fn reviewed_variables_send_again_after_ack_and_leave_other_queues_untouched() {
    let (core, log, root) = fixture(true).await;
    core.globals_set("port", "first").unwrap();
    core.sync_v2_stage_variables(SyncV2StageVariablesParams {
        variables: vec![variable("unticked", VariableKind::Number, json!(2))],
        remove: vec![],
    })
    .await
    .unwrap();
    core.sync_v2_stage_preferences(json!({"theme":"dark"}))
        .await
        .unwrap();
    let before = core.sync_v2_changes_list().await.unwrap();
    let action = variable_review(&core, "port").await;
    let reused = action.clone();
    assert_eq!(core.sync_v2_variables_send(action).await.unwrap().pushed, 1);
    let count = log.lock().unwrap().len();
    assert!(core.sync_v2_variables_send(reused).await.is_err());
    assert_eq!(log.lock().unwrap().len(), count);
    core.globals_set("port", "second").unwrap();
    assert_eq!(
        core.sync_v2_variables_send(variable_review(&core, "port").await)
            .await
            .unwrap()
            .pushed,
        1
    );
    let after = core.sync_v2_changes_list().await.unwrap();
    for entry in before {
        assert_eq!(
            after
                .iter()
                .find(|candidate| candidate.collection == entry.collection
                    && candidate.key == entry.key),
            Some(&entry)
        );
    }
    let writes: Vec<_> = log
        .lock()
        .unwrap()
        .iter()
        .filter(|(method, _, _)| method == "PUT" || method == "DELETE")
        .cloned()
        .collect();
    assert_eq!(writes.len(), 2);
    assert!(writes
        .iter()
        .all(|(_, path, _)| path.ends_with("/variables/port")));
    assert!(writes[1].2.contains("second"));
    assert_eq!(core.globals_get("port").unwrap(), "second");
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn reviewed_variables_withdraw_secret_after_uncertain_upload_without_replaying_its_value() {
    use crate::control::{RuntimeVariable, RuntimeVariableKind, RuntimeVariablesReplace};
    let (core, log, root, server) = fixture_server(true).await;
    let formerly_public = "formerly-public-variable";
    core.globals_set("privateValue", formerly_public).unwrap();
    server
        .drop_next_write
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(core
        .sync_v2_variables_send(variable_review(&core, "privateValue").await)
        .await
        .is_err());
    let remote = server.records.lock().unwrap()["privateValue"].clone();
    cache_variable(&core, "privateValue", Some(remote["value"].clone()), 1).await;
    let local = core.runtime_variables_list().unwrap();
    core.runtime_variables_replace(RuntimeVariablesReplace {
        expected_revision: local.revision,
        variables: [(
            "privateValue".into(),
            RuntimeVariable {
                kind: RuntimeVariableKind::Secret,
                value: json!(formerly_public),
            },
        )]
        .into(),
    })
    .unwrap();
    let before = log.lock().unwrap().len();
    assert!(core
        .sync_v2_variables_send(variable_review(&core, "privateValue").await)
        .await
        .is_err());
    assert_eq!(
        log.lock().unwrap().len(),
        before,
        "a secret send must fail before network"
    );
    let review = core.sync_v2_variables_review().await.unwrap();
    assert!(!serde_json::to_string(&review)
        .unwrap()
        .contains(formerly_public));
    let result = core
        .sync_v2_variables_withdraw(variable_review(&core, "privateValue").await)
        .await
        .unwrap();
    assert_eq!(result.pushed, 1);
    assert_eq!(core.globals_get("privateValue").unwrap(), formerly_public);
    assert_eq!(
        core.runtime_variables_list().unwrap().variables["privateValue"].kind,
        RuntimeVariableKind::Secret
    );
    assert!(server.records.lock().unwrap()["privateValue"]["deleted"]
        .as_bool()
        .unwrap());
    assert!(core
        .sync_v2_variables_review()
        .await
        .unwrap()
        .changes
        .is_empty());
    let requests = log.lock().unwrap()[before..].to_vec();
    assert!(requests
        .iter()
        .any(|(method, path, _)| method == "POST" && path.ends_with("write-barrier")));
    assert!(requests.iter().any(|(method, _, _)| method == "DELETE"));
    assert!(requests
        .iter()
        .all(|(method, _, body)| method != "PUT" && !body.contains(formerly_public)));
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn reviewed_variables_apply_cancels_a_queued_edit_without_publishing() {
    let (core, log, root, server) = fixture_server(true).await;
    core.globals_set("port", "cloud-value").unwrap();
    core.sync_v2_variables_send(variable_review(&core, "port").await)
        .await
        .unwrap();
    core.globals_set("port", "local-value").unwrap();
    core.sync_v2_stage_variables(SyncV2StageVariablesParams {
        variables: vec![variable(
            "port",
            VariableKind::Text,
            json!("old-queued-value"),
        )],
        remove: vec![],
    })
    .await
    .unwrap();
    let before = log.lock().unwrap().len();
    core.sync_v2_variables_apply(variable_review(&core, "port").await)
        .await
        .unwrap();
    assert_eq!(log.lock().unwrap().len(), before);
    assert_eq!(core.globals_get("port").unwrap(), "cloud-value");
    assert!(core
        .sync_v2_variables_review()
        .await
        .unwrap()
        .changes
        .is_empty());
    assert!(core.sync_v2_changes_list().await.unwrap()[0]
        .pending
        .is_none());
    assert_eq!(server.records.lock().unwrap()["port"]["revision"], 1);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn reviewed_variables_reject_changed_local_or_target_and_missing_capability() {
    let (core, log, root) = fixture(true).await;
    core.globals_set("port", "first").unwrap();
    let old = variable_review(&core, "port").await;
    core.globals_set("port", "second").unwrap();
    let before = log.lock().unwrap().len();
    assert!(core.sync_v2_variables_send(old).await.is_err());
    assert_eq!(log.lock().unwrap().len(), before);
    let old = variable_review(&core, "port").await;
    cache_variable(
        &core,
        "port",
        Some(variable_document("port", "text", json!("remote"))),
        1,
    )
    .await;
    assert!(core.sync_v2_variables_send(old).await.is_err());
    assert!(!log
        .lock()
        .unwrap()
        .iter()
        .any(|(method, _, _)| method == "PUT" || method == "DELETE"));
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
    let (core, log, root) = fixture(false).await;
    core.globals_set("port", "first").unwrap();
    assert_eq!(
        core.sync_v2_variables_send(variable_review(&core, "port").await)
            .await
            .unwrap_err()
            .code,
        "capability_unavailable"
    );
    assert!(core.sync_v2_changes_list().await.unwrap().is_empty());
    assert!(!log
        .lock()
        .unwrap()
        .iter()
        .any(|(_, path, _)| path.starts_with("/api/v2/sync/")));
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn reviewed_variables_adopt_an_exact_cached_integer_without_javascript_rounding() {
    let (core, log, root) = fixture(true).await;
    core.globals_set("counter", "local text").unwrap();
    cache_variable(
        &core,
        "counter",
        Some(variable_document("counter", "number", json!(u64::MAX))),
        1,
    )
    .await;
    let before = log.lock().unwrap().len();
    let review = core.sync_v2_variables_review().await.unwrap();
    let wire = serde_json::to_value(&review).unwrap();
    assert_eq!(
        wire["changes"][0]["cloud"]["numberText"],
        u64::MAX.to_string()
    );
    core.sync_v2_variables_apply(variable_review(&core, "counter").await)
        .await
        .unwrap();
    assert_eq!(core.globals_get("counter").unwrap(), u64::MAX.to_string());
    assert_eq!(log.lock().unwrap().len(), before);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}
