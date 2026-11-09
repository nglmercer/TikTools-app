//! Real HTTP and private files exercise the coordinator boundary, without
//! installing plugins, opening application databases, or using a v1 sync API.
use super::*;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
};

#[derive(Clone, Debug)]
struct Request {
    method: String,
    target: String,
    headers: BTreeMap<String, String>,
    body: Value,
}
struct Pause {
    entered: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
}
struct DurableProof {
    root: PathBuf,
    account: String,
    target: WorkspaceKey,
    edit: WorkspaceEdit,
}
#[derive(Default)]
struct Remote {
    cursors: BTreeMap<String, i64>,
    records: BTreeMap<(String, WorkspaceKey), WorkspaceRecord>,
    changes: BTreeMap<String, Vec<WorkspaceRecord>>,
    requests: Vec<Request>,
    lose_next_write: bool,
    conflict_next_write: bool,
    expire_changes: bool,
    barrier_count: i64,
    barrier_failure: bool,
    barrier_no_advance: bool,
    pause_next_write: Option<Pause>,
    pause_before_write: Option<Pause>,
    enforce_epoch: bool,
    rejected_stale: usize,
    proofs: Vec<DurableProof>,
}
impl Remote {
    fn put(
        &mut self,
        account: &str,
        target: WorkspaceKey,
        value: Option<Value>,
    ) -> WorkspaceRecord {
        if let Some(old) = self.records.get(&(account.into(), target.clone())) {
            if old.value == value {
                return old.clone();
            }
        }
        let cursor = self.cursors.entry(account.into()).or_default();
        *cursor += 1;
        let record = WorkspaceRecord {
            collection: target.collection.clone(),
            key: target.key.clone(),
            revision: *cursor,
            deleted: value.is_none(),
            value,
        };
        self.records
            .insert((account.into(), target), record.clone());
        self.changes
            .entry(account.into())
            .or_default()
            .push(record.clone());
        record
    }
    fn writes(&self) -> usize {
        self.requests
            .iter()
            .filter(|request| matches!(request.method.as_str(), "PUT" | "DELETE"))
            .count()
    }
}
struct Server {
    origin: String,
    remote: Arc<Mutex<Remote>>,
    task: JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn read_request(socket: &mut TcpStream) -> Request {
    let mut bytes = Vec::new();
    let end = loop {
        let mut chunk = [0; 4096];
        let n = socket.read(&mut chunk).await.unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&chunk[..n]);
        if let Some(index) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
            break index + 4;
        }
        assert!(bytes.len() < 16 * 1024);
    };
    let text = std::str::from_utf8(&bytes[..end]).unwrap();
    let mut lines = text.split("\r\n");
    let mut first = lines.next().unwrap().split_whitespace();
    let method = first.next().unwrap().to_owned();
    let target = first.next().unwrap().to_owned();
    let headers: BTreeMap<_, _> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let length = headers
        .get("content-length")
        .map(|v| v.parse::<usize>().unwrap())
        .unwrap_or(0);
    assert!(length <= 256 * 1024 + 1024);
    while bytes.len() < end + length {
        let mut chunk = [0; 4096];
        let n = socket.read(&mut chunk).await.unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&chunk[..n]);
    }
    let body = if length == 0 {
        Value::Null
    } else {
        serde_json::from_slice(&bytes[end..end + length]).unwrap()
    };
    Request {
        method,
        target,
        headers,
        body,
    }
}
async fn reply(socket: &mut TcpStream, status: u16, value: Value) {
    let bytes = serde_json::to_vec(&value).unwrap();
    let head=format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len());
    let _ = socket.write_all(head.as_bytes()).await;
    let _ = socket.write_all(&bytes).await;
}
async fn start_server() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let remote = Arc::new(Mutex::new(Remote::default()));
    let state = remote.clone();
    let server_origin = origin.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let state = state.clone();
            let origin = server_origin.clone();
            tokio::spawn(async move {
                let request = read_request(&mut socket).await;
                let before_write = if matches!(request.method.as_str(), "PUT" | "DELETE") {
                    state.lock().unwrap().pause_before_write.take()
                } else {
                    None
                };
                if let Some(pause) = before_write {
                    let _ = pause.entered.send(());
                    let _ = pause.release.await;
                }
                let (status, value, lose, pause) = {
                    let mut remote = state.lock().unwrap();
                    remote.requests.push(request.clone());
                    let account = match request.headers.get("authorization").map(String::as_str) {
                        Some("Bearer device-a" | "Bearer device-b") => "account-a",
                        Some("Bearer other-account") => "account-b",
                        _ => "unauthorized",
                    };
                    let url = url::Url::parse(&format!("{origin}{}", request.target)).unwrap();
                    let path = url.path();
                    if account == "unauthorized" {
                        (401, json!({}), false, None)
                    } else if path == "/api/v1/native/me" {
                        (200, json!({"id":account}), false, None)
                    } else if path == "/api/v2/sync/changes" {
                        if remote.expire_changes {
                            (410, json!({}), false, None)
                        } else {
                            let after = url
                                .query_pairs()
                                .find(|(k, _)| k == "cursor")
                                .unwrap()
                                .1
                                .parse::<i64>()
                                .unwrap();
                            let changes: Vec<_> = remote
                                .changes
                                .get(account)
                                .into_iter()
                                .flatten()
                                .filter(|r| r.revision > after)
                                .take(50)
                                .cloned()
                                .collect();
                            let cursor = changes.last().map_or(after, |r| r.revision);
                            let more = remote
                                .changes
                                .get(account)
                                .into_iter()
                                .flatten()
                                .any(|r| r.revision > cursor);
                            (
                                200,
                                json!({"changes":changes,"cursor":cursor,"has_more":more}),
                                false,
                                None,
                            )
                        }
                    } else if path == "/api/v2/sync/write-epoch" {
                        assert_eq!(request.method, "GET");
                        (
                            200,
                            json!({"writeEpoch":(7 + remote.barrier_count).to_string(), "epochRequired":true}),
                            false,
                            None,
                        )
                    } else if path == "/api/v2/sync/write-barrier" {
                        assert_eq!(request.method, "POST");
                        if remote.barrier_failure {
                            (503, json!({}), false, None)
                        } else {
                            if !remote.barrier_no_advance {
                                remote.barrier_count += 1;
                            }
                            (
                                200,
                                json!({"writeEpoch":(7 + remote.barrier_count).to_string()}),
                                false,
                                None,
                            )
                        }
                    } else if path == "/api/v2/sync/export" {
                        let records: Vec<_> = remote
                            .records
                            .iter()
                            .filter(|((owner, _), _)| owner == account)
                            .map(|(_, r)| r.clone())
                            .collect();
                        (
                            200,
                            json!({"schemaVersion":2,"cursor":remote.cursors.get(account).copied().unwrap_or(0),"records":records}),
                            false,
                            None,
                        )
                    } else if let Some(suffix) = path.strip_prefix("/api/v2/sync/collections/") {
                        let (collection, key) = suffix.split_once('/').unwrap();
                        let target = WorkspaceKey::new(collection, key).unwrap();
                        let old = remote
                            .records
                            .get(&(account.into(), target.clone()))
                            .cloned();
                        if request.method == "GET" {
                            match old {
                                Some(record) => (200, json!(record), false, None),
                                None => (404, json!({}), false, None),
                            }
                        } else if remote.enforce_epoch
                            && request.headers.get("x-workspace-write-epoch")
                                != Some(&(7 + remote.barrier_count).to_string())
                        {
                            remote.rejected_stale += 1;
                            (412, json!({}), false, None)
                        } else {
                            assert!(matches!(request.method.as_str(), "PUT" | "DELETE"));
                            if let Some(index) = remote
                                .proofs
                                .iter()
                                .position(|p| p.account == account && p.target == target)
                            {
                                let proof = remote.proofs.remove(index);
                                let durable =
                                    WorkspaceStore::open(&proof.root, origin.clone(), account)
                                        .unwrap()
                                        .load()
                                        .unwrap()
                                        .unwrap();
                                let pending = durable
                                    .state
                                    .entry(&target)
                                    .unwrap()
                                    .pending
                                    .as_ref()
                                    .unwrap();
                                assert_eq!(
                                    pending.status,
                                    WorkspacePendingStatus::Unconfirmed,
                                    "request was sent before its durable intent"
                                );
                                assert_eq!(
                                    pending.edit, proof.edit,
                                    "durable request source differs from actual request"
                                );
                                if durable.state.is_product_guarded() {
                                    let epoch = pending
                                        .write_epoch
                                        .as_ref()
                                        .expect("product epoch persisted before HTTP");
                                    assert_eq!(
                                        request.headers.get("x-workspace-write-epoch"),
                                        Some(&epoch.decimal())
                                    );
                                }
                                assert_eq!(
                                    pending.edit,
                                    if request.method == "DELETE" {
                                        WorkspaceEdit::Delete
                                    } else {
                                        WorkspaceEdit::Put(request.body.clone())
                                    }
                                );
                            }
                            let expected = old
                                .as_ref()
                                .map(|record| format!("\"{}\"", record.revision));
                            let matched = match expected {
                                Some(ref value) => {
                                    request.headers.get("if-match") == Some(value)
                                        && !request.headers.contains_key("if-none-match")
                                }
                                None => {
                                    request.headers.get("if-none-match").map(String::as_str)
                                        == Some("*")
                                        && !request.headers.contains_key("if-match")
                                }
                            };
                            if !matched || std::mem::take(&mut remote.conflict_next_write) {
                                let pause = remote.pause_next_write.take();
                                (409, json!({}), false, pause)
                            } else {
                                let record = remote.put(
                                    account,
                                    target,
                                    if request.method == "DELETE" {
                                        None
                                    } else {
                                        Some(request.body.clone())
                                    },
                                );
                                let lose = std::mem::take(&mut remote.lose_next_write);
                                let pause = remote.pause_next_write.take();
                                (200, json!(record), lose, pause)
                            }
                        }
                    } else {
                        panic!("unexpected HTTP route {path}");
                    }
                };
                if let Some(pause) = pause {
                    let _ = pause.entered.send(());
                    let _ = pause.release.await;
                }
                if !lose {
                    reply(&mut socket, status, value).await;
                }
            });
        }
    });
    Server {
        origin,
        remote,
        task,
    }
}
async fn connect(server: &Server, token: &str) -> Session {
    Session::connect(&server.origin, token.into())
        .await
        .unwrap()
}
fn coordinator(root: &Path, server: &Server, account: &str) -> WorkspaceCoordinator {
    WorkspaceCoordinator::open(root, server.origin.clone(), account.into()).unwrap()
}
// These low-level tests supply a trusted product fixture directly, without
// running the separate protocol cutover transaction.
fn guard_product_fixture(coordinator: &WorkspaceCoordinator) {
    let mut snapshot = coordinator.snapshot().unwrap();
    snapshot.state.mark_product_guarded();
    coordinator
        .store
        .save(Some(snapshot.generation), &snapshot.state)
        .unwrap();
}
fn key(collection: &str, name: &str) -> WorkspaceKey {
    WorkspaceKey::new(collection, name).unwrap()
}
fn preference(theme: &str) -> Value {
    json!({"schemaVersion":1,"theme":theme})
}
fn template() -> Value {
    json!({"templateVersion":1,"id":"gift-rule","title":"Gift rule","actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"Private local template"}}],"event":{"name":"Chat","enabled":false,"trigger":"tiktok.chat"}})
}
fn selection() -> PluginSelection {
    PluginSelection {
        schema_version: 1,
        plugin_id: "acme.plugin".into(),
        version: "1.2.3".into(),
        enabled: false,
    }
}
fn no_v1(server: &Server) {
    let remote = server.remote.lock().unwrap();
    assert!(remote
        .requests
        .iter()
        .all(|r| r.target == "/api/v1/native/me" || r.target.starts_with("/api/v2/sync/")));
    assert!(remote
        .requests
        .iter()
        .all(|r| !r.headers.contains_key("cookie")));
}
fn pending(coordinator: &WorkspaceCoordinator, target: &WorkspaceKey) -> WorkspaceEntry {
    coordinator
        .snapshot()
        .unwrap()
        .state
        .entry(target)
        .unwrap()
        .clone()
}

#[tokio::test]
async fn durable_product_mode_blocks_all_generic_routes_without_protocol_markers() {
    let temp = tempfile::tempdir().unwrap();
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let coord = coordinator(&temp.path().join("workspace"), &server, "account-a");
    coord.bootstrap(&[], None).unwrap();
    let target = key("preferences", "desktop");
    coord
        .stage(target.clone(), WorkspaceEdit::Put(preference("dark")))
        .unwrap();
    let review = coord.review(&session, &target).await.unwrap();
    guard_product_fixture(&coord);
    let before = coord.snapshot().unwrap();
    let requests = server.remote.lock().unwrap().requests.len();
    let is_blocked = |error| {
        matches!(
            error,
            WorkspaceCoordError::Protocol(crate::ProtocolFenceError::ProductOutboxRequired)
        )
    };
    assert!(is_blocked(coord.bootstrap(&[], None).unwrap_err()));
    assert!(is_blocked(
        coord
            .stage(target.clone(), WorkspaceEdit::Delete)
            .unwrap_err()
    ));
    assert!(is_blocked(coord.sync(&session).await.err().unwrap()));
    assert!(is_blocked(
        coord.review(&session, &target).await.err().unwrap()
    ));
    assert!(is_blocked(
        coord.resolve(&session, &review, true).await.unwrap_err()
    ));
    assert!(is_blocked(
        coord.rebuild_remote_cache(&session).await.unwrap_err()
    ));
    assert_eq!(server.remote.lock().unwrap().requests.len(), requests);
    assert_eq!(coord.snapshot().unwrap().state, before.state);
    assert_eq!(coord.snapshot().unwrap().generation, before.generation);
}

#[tokio::test]
async fn marked_product_partition_rejects_generic_paths_even_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let coord = coordinator(&root, &server, "account-a");
    coord.bootstrap(&[], None).unwrap();
    let target = key("preferences", "desktop");
    coord
        .stage(target.clone(), WorkspaceEdit::Put(preference("dark")))
        .unwrap();
    let prior_review = coord.review(&session, &target).await.unwrap();
    let gate = coord.protocol.try_cutover().unwrap();
    assert!(matches!(
        gate.require_product_outbox(&server.origin, "account-a"),
        Err(crate::ProtocolFenceError::UnsettledV2)
    ));
    drop(gate);
    coord.resolve(&session, &prior_review, false).await.unwrap();
    let before = coord.snapshot().unwrap();
    let gate = coord.protocol.try_cutover().unwrap();
    gate.require_product_outbox(&server.origin, "account-a")
        .unwrap();
    drop(gate);
    let requests = server.remote.lock().unwrap().requests.len();

    assert!(matches!(
        coord.stage(target.clone(), WorkspaceEdit::Put(preference("light"))),
        Err(WorkspaceCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    assert!(matches!(
        coord.sync(&session).await,
        Err(WorkspaceCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    assert!(matches!(
        coord.review(&session, &target).await,
        Err(WorkspaceCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    assert!(matches!(
        coord.resolve(&session, &prior_review, false).await,
        Err(WorkspaceCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    assert!(matches!(
        coord.rebuild_remote_cache(&session).await,
        Err(WorkspaceCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    let after = coord.snapshot().unwrap();
    assert_eq!(after.generation, before.generation);
    assert_eq!(after.state, before.state);
    assert_eq!(server.remote.lock().unwrap().requests.len(), requests);
    drop(coord);
    let reopened = coordinator(&root, &server, "account-a");
    assert!(matches!(
        reopened.stage(target, WorkspaceEdit::Put(preference("light"))),
        Err(WorkspaceCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    // The independent account partition is unaffected.
    coordinator(&root, &server, "account-b")
        .bootstrap(&[], None)
        .unwrap();
}

#[tokio::test]
async fn generic_network_operation_blocks_cutover_until_response_published() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let coord = coordinator(&root, &server, "account-a");
    coord.bootstrap(&[], None).unwrap();
    coord
        .stage(
            key("preferences", "desktop"),
            WorkspaceEdit::Put(preference("dark")),
        )
        .unwrap();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    server.remote.lock().unwrap().pause_next_write = Some(Pause {
        entered: entered_tx,
        release: release_rx,
    });
    let control = async {
        entered_rx.await.unwrap();
        assert!(matches!(
            coord.protocol.try_cutover(),
            Err(crate::ProtocolFenceError::Busy)
        ));
        release_tx.send(()).unwrap();
    };
    let (outcome, ()) = tokio::join!(coord.sync(&session), control);
    assert_eq!(outcome.unwrap().pushed, 1);
    let gate = coord.protocol.try_cutover().unwrap();
    gate.require_product_outbox(&server.origin, "account-a")
        .unwrap();
    drop(gate);
    assert!(matches!(
        coord.sync(&session).await,
        Err(WorkspaceCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
}

#[tokio::test]
async fn cancelled_generic_write_blocks_product_marker_and_preserves_review() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let coord = coordinator(&root, &server, "account-a");
    coord.bootstrap(&[], None).unwrap();
    let target = key("preferences", "desktop");
    coord
        .stage(target.clone(), WorkspaceEdit::Put(preference("dark")))
        .unwrap();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    server.remote.lock().unwrap().pause_next_write = Some(Pause {
        entered: entered_tx,
        release: release_rx,
    });
    let mut attempt = Box::pin(coord.sync(&session));
    tokio::select! {
        _result = &mut attempt => panic!("write returned before the held response"),
        result = entered_rx => result.unwrap(),
    }
    drop(attempt);
    assert_eq!(
        pending(&coord, &target).pending.unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
    let before = coord.snapshot().unwrap();
    let gate = coord.protocol.try_cutover().unwrap();
    assert!(matches!(
        gate.require_product_outbox(&server.origin, "account-a"),
        Err(crate::ProtocolFenceError::UnsettledV2)
    ));
    drop(gate);
    let after = coord.snapshot().unwrap();
    assert_eq!(after.generation, before.generation);
    assert_eq!(after.state, before.state);
    // The generic review path remains available rather than being trapped
    // by a newly installed product marker. A GET alone does not establish
    // that an earlier server handler can no longer commit.
    assert!(coord.review(&session, &target).await.is_ok());
    release_tx.send(()).unwrap();
}

struct ProductUploadFixture {
    generation: std::sync::atomic::AtomicU64,
    consent: u128,
    expire_eligibility: Option<Arc<std::sync::atomic::AtomicBool>>,
}

#[tokio::test]
async fn product_review_barrier_rejects_a_cancelled_write_that_arrives_late() {
    use std::sync::atomic::{AtomicU64, Ordering};

    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let temp = tempfile::tempdir().unwrap();
    let coordinator = Arc::new(coordinator(
        &temp.path().join("private"),
        &server,
        "account-a",
    ));
    coordinator.bootstrap(&[], None).unwrap();
    guard_product_fixture(&coordinator);
    let target = key("plugin-selections", "acme.plugin");
    let decision = crate::V2ProductDecision {
        origin: server.origin.clone(),
        account_id: "account-a".into(),
        config_generation: 42,
        source_generation: 7,
        plugin_id: "acme.plugin".into(),
        selection: Some(selection()),
    };
    coordinator
        .stage_product_decision(
            target.clone(),
            WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap()),
            7,
            &decision.digest().unwrap(),
        )
        .unwrap();
    let source = Arc::new(ProductUploadFixture {
        generation: AtomicU64::new(7),
        consent: 42,
        expire_eligibility: None,
    });
    let eligibility =
        WorkspaceEligibility::new(session.clone(), || true).with_test_publication_guard();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    {
        let mut remote = server.remote.lock().unwrap();
        remote.enforce_epoch = true;
        remote.pause_before_write = Some(Pause {
            entered: entered_tx,
            release: release_rx,
        });
    }
    let worker_coord = coordinator.clone();
    let worker_session = session.clone();
    let worker_source = source.clone();
    let worker_eligibility = eligibility.clone();
    let worker = tokio::spawn(async move {
        let admission = ProductUploadAdmission {
            source: worker_source.as_ref(),
            config_generation: 42,
        };
        worker_coord
            .sync_product_guarded(&worker_session, &worker_eligibility, &admission)
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), entered_rx)
        .await
        .unwrap()
        .unwrap();
    worker.abort();
    assert!(worker.await.err().unwrap().is_cancelled());
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
    let newer = crate::V2ProductDecision {
        source_generation: 8,
        ..decision.clone()
    };
    coordinator
        .stage_product_decision(
            target.clone(),
            WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap()),
            8,
            &newer.digest().unwrap(),
        )
        .unwrap();
    source.generation.store(8, Ordering::SeqCst);
    assert!(pending(&coordinator, &target).next.is_some());
    let review = coordinator
        .review_product_guarded(&session, &target, &eligibility)
        .await
        .unwrap();
    assert!(review.remote().is_none());
    release_tx.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if server.remote.lock().unwrap().rejected_stale == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the delayed request must be rejected after the barrier");
    assert!(server.remote.lock().unwrap().records.is_empty());
    let admission = ProductUploadAdmission {
        source: source.as_ref(),
        config_generation: 42,
    };
    coordinator
        .resolve_product_keep_local_guarded(&session, &review, &eligibility, &admission)
        .await
        .unwrap();
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Queued
    );
    assert_eq!(
        pending(&coordinator, &target)
            .pending
            .unwrap()
            .local_generation,
        2
    );
}

#[tokio::test]
async fn product_recovery_rebases_only_latest_staged_decision_without_replaying() {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    struct Source {
        generation: AtomicU64,
        enabled: AtomicBool,
    }
    impl crate::V2ProductUploadSource for Source {
        fn acquire_emission_guard(&self) -> Result<Box<dyn Send>, WorkspaceCoordError> {
            Ok(Box::new(()))
        }

        fn with_fenced_upload(
            &self,
            decision: &crate::V2ProductDecision,
            admit: &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
        ) -> Result<(), WorkspaceCoordError> {
            let mut expected = selection();
            expected.enabled = self.enabled.load(Ordering::SeqCst);
            if decision.source_generation != self.generation.load(Ordering::SeqCst)
                || decision.config_generation != 42
                || decision.selection.as_ref() != Some(&expected)
            {
                return Err(WorkspaceStateError::StaleReview.into());
            }
            admit()
        }
    }

    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let temp = tempfile::tempdir().unwrap();
    let coordinator = coordinator(&temp.path().join("private"), &server, "account-a");
    coordinator.bootstrap(&[], None).unwrap();
    guard_product_fixture(&coordinator);
    let target = key("plugin-selections", "acme.plugin");
    let mut decision = crate::V2ProductDecision {
        origin: server.origin.clone(),
        account_id: "account-a".into(),
        config_generation: 42,
        source_generation: 7,
        plugin_id: "acme.plugin".into(),
        selection: Some(selection()),
    };
    let first =
        WorkspaceEdit::Put(serde_json::to_value(decision.selection.as_ref().unwrap()).unwrap());
    coordinator
        .stage_product_decision(
            target.clone(),
            first.clone(),
            7,
            &decision.digest().unwrap(),
        )
        .unwrap();
    let eligibility =
        WorkspaceEligibility::new(session.clone(), || true).with_test_publication_guard();
    let source = Source {
        generation: AtomicU64::new(7),
        enabled: AtomicBool::new(false),
    };
    let admission = ProductUploadAdmission {
        source: &source,
        config_generation: 42,
    };
    server.remote.lock().unwrap().lose_next_write = true;
    assert!(coordinator
        .sync_product_guarded(&session, &eligibility, &admission)
        .await
        .is_err());
    assert_eq!(server.remote.lock().unwrap().writes(), 1);
    let original = pending(&coordinator, &target).pending.unwrap();
    assert_eq!(original.status, WorkspacePendingStatus::Unconfirmed);
    assert_eq!(original.edit, first);

    // A newer durable source decision cannot replace the frozen uncertain
    // request, but its own receipt is the only proof eligible for recovery.
    decision.source_generation = 8;
    decision.selection.as_mut().unwrap().enabled = true;
    let latest =
        WorkspaceEdit::Put(serde_json::to_value(decision.selection.as_ref().unwrap()).unwrap());
    let (latest_generation, staged) = coordinator
        .stage_product_decision(
            target.clone(),
            latest.clone(),
            8,
            &decision.digest().unwrap(),
        )
        .unwrap();
    assert!(staged);
    let entry = pending(&coordinator, &target);
    assert_eq!(entry.pending.as_ref(), Some(&original));
    assert_eq!(entry.next.as_ref().unwrap().edit, latest);
    // Old on-disk unconfirmed requests may lack an epoch. A fresh barrier
    // cannot retroactively prove which epoch such a request actually used.
    let saved = coordinator.snapshot().unwrap();
    let mut legacy = saved.state.clone();
    legacy
        .entries
        .iter_mut()
        .find(|entry| entry.target == target)
        .unwrap()
        .pending
        .as_mut()
        .unwrap()
        .write_epoch = None;
    let legacy = coordinator
        .store
        .save(Some(saved.generation), &legacy)
        .unwrap();
    let requests_before = server.remote.lock().unwrap().requests.len();
    assert!(coordinator
        .review_product_guarded(&session, &target, &eligibility)
        .await
        .is_err());
    assert_eq!(
        server.remote.lock().unwrap().requests.len(),
        requests_before
    );
    coordinator
        .store
        .save(Some(legacy.generation), &saved.state)
        .unwrap();
    server.remote.lock().unwrap().barrier_failure = true;
    let before_failed_barrier = coordinator.snapshot().unwrap();
    assert!(coordinator
        .review_product_guarded(&session, &target, &eligibility)
        .await
        .is_err());
    let after_failed_barrier = coordinator.snapshot().unwrap();
    assert_eq!(
        before_failed_barrier.generation,
        after_failed_barrier.generation
    );
    assert_eq!(before_failed_barrier.state, after_failed_barrier.state);
    server.remote.lock().unwrap().barrier_failure = false;
    server.remote.lock().unwrap().barrier_no_advance = true;
    assert!(coordinator
        .review_product_guarded(&session, &target, &eligibility)
        .await
        .is_err());
    assert_eq!(
        coordinator.snapshot().unwrap().state,
        before_failed_barrier.state
    );
    server.remote.lock().unwrap().barrier_no_advance = false;
    let review = coordinator
        .review_product_guarded(&session, &target, &eligibility)
        .await
        .unwrap();
    {
        let remote = server.remote.lock().unwrap();
        let barrier = remote
            .requests
            .iter()
            .rposition(|request| request.target == "/api/v2/sync/write-barrier")
            .unwrap();
        let read = remote
            .requests
            .iter()
            .rposition(|request| {
                request.method == "GET"
                    && request.target.contains("/collections/plugin-selections/")
            })
            .unwrap();
        assert!(barrier < read);
        assert_eq!(remote.writes(), 1);
    }
    assert_eq!(review.local_generation(), latest_generation);

    // The old source cannot approve even an otherwise-current review.
    let before = coordinator.snapshot().unwrap();
    assert!(matches!(
        coordinator
            .resolve_product_keep_local_guarded(&session, &review, &eligibility, &admission)
            .await,
        Err(WorkspaceCoordError::State(WorkspaceStateError::StaleReview))
    ));
    let after = coordinator.snapshot().unwrap();
    assert_eq!(before.generation, after.generation);
    assert_eq!(before.state, after.state);
    assert_eq!(server.remote.lock().unwrap().writes(), 1);

    source.generation.store(8, Ordering::SeqCst);
    source.enabled.store(true, Ordering::SeqCst);
    coordinator
        .resolve_product_keep_local_guarded(&session, &review, &eligibility, &admission)
        .await
        .unwrap();
    let entry = pending(&coordinator, &target);
    assert!(entry.next.is_none());
    let rebased = entry.pending.unwrap();
    assert_eq!(rebased.status, WorkspacePendingStatus::Queued);
    assert_eq!(rebased.local_generation, latest_generation);
    assert_eq!(rebased.edit, latest);
    let proof = coordinator
        .snapshot()
        .unwrap()
        .state
        .product_queued_proof(&target)
        .unwrap();
    assert_eq!(proof.source_generation(), 8);
    assert_eq!(proof.edit(), &latest);
    assert_eq!(
        server.remote.lock().unwrap().writes(),
        1,
        "review and rebase never resend"
    );
    // A later, separate dispatcher run may send the explicitly rebased value.
    assert_eq!(
        coordinator
            .sync_product_guarded(&session, &eligibility, &admission)
            .await
            .unwrap()
            .pushed,
        1
    );
    assert_eq!(server.remote.lock().unwrap().writes(), 2);
}

#[tokio::test]
async fn product_uncertain_keep_local_requires_fresh_review_source_and_publication_permit() {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let temp = tempfile::tempdir().unwrap();
    let coordinator = coordinator(&temp.path().join("private"), &server, "account-a");
    coordinator.bootstrap(&[], None).unwrap();
    guard_product_fixture(&coordinator);
    let target = key("plugin-selections", "acme.plugin");
    let decision = crate::V2ProductDecision {
        origin: server.origin.clone(),
        account_id: "account-a".into(),
        config_generation: 42,
        source_generation: 7,
        plugin_id: "acme.plugin".into(),
        selection: Some(selection()),
    };
    coordinator
        .stage_product_decision(
            target.clone(),
            WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap()),
            7,
            &decision.digest().unwrap(),
        )
        .unwrap();
    let current = Arc::new(AtomicBool::new(true));
    let deny_publication = Arc::new(AtomicBool::new(false));
    let check = current.clone();
    let guard_current = current.clone();
    let deny = deny_publication.clone();
    let eligibility =
        WorkspaceEligibility::new(session.clone(), move || check.load(Ordering::SeqCst))
            .with_publication_guard(move |publish| {
                if deny.swap(false, Ordering::SeqCst) {
                    guard_current.store(false, Ordering::SeqCst);
                }
                if !guard_current.load(Ordering::SeqCst) {
                    return Err(WorkspaceCoordError::Ineligible);
                }
                publish()
            })
            .with_test_emission_guard();
    let source = ProductUploadFixture {
        generation: AtomicU64::new(7),
        consent: 42,
        expire_eligibility: None,
    };
    let admission = ProductUploadAdmission {
        source: &source,
        config_generation: 42,
    };
    server.remote.lock().unwrap().lose_next_write = true;
    assert!(coordinator
        .sync_product_guarded(&session, &eligibility, &admission)
        .await
        .is_err());
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
    assert_eq!(server.remote.lock().unwrap().writes(), 1);

    // A logout/consent change at the actual rename must not publish the
    // newly fetched private record or change the uncertain ticket.
    let before = coordinator.snapshot().unwrap();
    deny_publication.store(true, Ordering::SeqCst);
    assert!(matches!(
        coordinator
            .review_product_guarded(&session, &target, &eligibility)
            .await,
        Err(WorkspaceCoordError::Ineligible)
    ));
    let after = coordinator.snapshot().unwrap();
    assert_eq!(after.generation, before.generation);
    assert_eq!(after.state, before.state);
    current.store(true, Ordering::SeqCst);
    let review = coordinator
        .review_product_guarded(&session, &target, &eligibility)
        .await
        .unwrap();
    assert!(review.remote().is_some());
    // Generic resolution cannot consume a product-bound review or drop it
    // as a Cloud-accept decision without a separate application journal.
    assert!(coordinator
        .resolve_guarded(&session, &review, false, &eligibility)
        .await
        .is_err());
    let reviewed = coordinator.snapshot().unwrap();
    source.generation.store(8, Ordering::SeqCst);
    assert!(coordinator
        .resolve_product_keep_local_guarded(&session, &review, &eligibility, &admission)
        .await
        .is_err());
    assert_eq!(coordinator.snapshot().unwrap().state, reviewed.state);
    source.generation.store(7, Ordering::SeqCst);
    let mut changed = selection();
    changed.enabled = true;
    server.remote.lock().unwrap().put(
        "account-a",
        target.clone(),
        Some(serde_json::to_value(changed).unwrap()),
    );
    assert!(matches!(
        coordinator
            .resolve_product_keep_local_guarded(&session, &review, &eligibility, &admission)
            .await,
        Err(WorkspaceCoordError::State(WorkspaceStateError::StaleReview))
    ));
    assert_eq!(coordinator.snapshot().unwrap().state, reviewed.state);
    let review = coordinator
        .review_product_guarded(&session, &target, &eligibility)
        .await
        .unwrap();
    let reviewed = coordinator.snapshot().unwrap();
    deny_publication.store(true, Ordering::SeqCst);
    assert!(matches!(
        coordinator
            .resolve_product_keep_local_guarded(&session, &review, &eligibility, &admission)
            .await,
        Err(WorkspaceCoordError::Ineligible)
    ));
    assert_eq!(coordinator.snapshot().unwrap().state, reviewed.state);
    current.store(true, Ordering::SeqCst);
    coordinator
        .resolve_product_keep_local_guarded(&session, &review, &eligibility, &admission)
        .await
        .unwrap();
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Queued
    );
    assert!(coordinator
        .snapshot()
        .unwrap()
        .state
        .product_queued_proof(&target)
        .is_some());
    assert_eq!(
        server.remote.lock().unwrap().writes(),
        1,
        "review and rebase must not resend"
    );
    assert_eq!(
        coordinator
            .sync_product_guarded(&session, &eligibility, &admission)
            .await
            .unwrap()
            .pushed,
        1
    );
    assert_eq!(server.remote.lock().unwrap().writes(), 2);
}
impl crate::V2ProductUploadSource for ProductUploadFixture {
    fn acquire_emission_guard(&self) -> Result<Box<dyn Send>, WorkspaceCoordError> {
        Ok(Box::new(()))
    }

    fn with_fenced_upload(
        &self,
        decision: &crate::V2ProductDecision,
        admit: &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
    ) -> Result<(), WorkspaceCoordError> {
        use std::sync::atomic::Ordering;
        if decision.source_generation != self.generation.load(Ordering::SeqCst)
            || decision.config_generation != self.consent
            || decision.plugin_id != "acme.plugin"
            || decision.selection.as_ref() != Some(&selection())
        {
            return Err(WorkspaceStateError::StaleReview.into());
        }
        if let Some(current) = &self.expire_eligibility {
            current.store(false, Ordering::SeqCst);
        }
        admit()
    }
}

#[tokio::test]
async fn product_upload_requires_exact_receipt_source_and_current_eligibility() {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let current = Arc::new(AtomicBool::new(true));
    let check = current.clone();
    let eligibility =
        WorkspaceEligibility::new(session.clone(), move || check.load(Ordering::SeqCst))
            .with_test_publication_guard();
    let source = ProductUploadFixture {
        generation: AtomicU64::new(7),
        consent: 42,
        expire_eligibility: None,
    };
    let admission = ProductUploadAdmission {
        source: &source,
        config_generation: 42,
    };
    let temp = tempfile::tempdir().unwrap();
    let coordinator = coordinator(&temp.path().join("private"), &server, "account-a");
    coordinator.bootstrap(&[], None).unwrap();
    let target = key("plugin-selections", "acme.plugin");
    let edit = WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap());
    // A generic queued edit has no trusted native receipt and cannot upload.
    coordinator.stage(target.clone(), edit.clone()).unwrap();
    guard_product_fixture(&coordinator);
    assert!(coordinator
        .sync_product_guarded(&session, &eligibility, &admission)
        .await
        .is_err());
    assert_eq!(server.remote.lock().unwrap().writes(), 0);
    let decision = crate::V2ProductDecision {
        origin: server.origin.clone(),
        account_id: "account-a".into(),
        config_generation: 42,
        source_generation: 7,
        plugin_id: "acme.plugin".into(),
        selection: Some(selection()),
    };
    coordinator
        .stage_product_decision(target.clone(), edit.clone(), 7, &decision.digest().unwrap())
        .unwrap();
    source.generation.store(8, Ordering::SeqCst);
    assert!(coordinator
        .sync_product_guarded(&session, &eligibility, &admission)
        .await
        .is_err());
    assert_eq!(server.remote.lock().unwrap().writes(), 0);
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Queued
    );
    source.generation.store(7, Ordering::SeqCst);
    let revoked = ProductUploadFixture {
        generation: AtomicU64::new(7),
        consent: 42,
        expire_eligibility: Some(current.clone()),
    };
    let revoked_admission = ProductUploadAdmission {
        source: &revoked,
        config_generation: 42,
    };
    assert!(matches!(
        coordinator
            .sync_product_guarded(&session, &eligibility, &revoked_admission)
            .await,
        Err(WorkspaceCoordError::Ineligible)
    ));
    assert_eq!(server.remote.lock().unwrap().writes(), 0);
    current.store(true, Ordering::SeqCst);
    assert_eq!(
        coordinator
            .sync_product_guarded(&session, &eligibility, &admission)
            .await
            .unwrap()
            .pushed,
        1
    );
    assert_eq!(server.remote.lock().unwrap().writes(), 1);
    // The product partition rejects a new generic decision outright.
    assert!(coordinator.stage(target.clone(), edit).is_err());
    assert_eq!(
        coordinator
            .sync_product_guarded(&session, &eligibility, &admission)
            .await
            .unwrap()
            .pushed,
        0
    );
    assert_eq!(server.remote.lock().unwrap().writes(), 1);
}

#[tokio::test]
async fn product_response_publication_requires_captured_session_and_consent() {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    // Exercise both the acknowledged and conflict response. The first mode
    // simulates logout while HTTP is in flight. The second invalidates the
    // permit only at the publication boundary, after the post-HTTP check.
    // The final mode confirms ordinary responses still commit normally.
    for conflict in [false, true] {
        for mode in 0..3 {
            let server = start_server().await;
            let session = Arc::new(connect(&server, "device-a").await);
            let temp = tempfile::tempdir().unwrap();
            let coordinator = coordinator(&temp.path().join("private"), &server, "account-a");
            coordinator.bootstrap(&[], None).unwrap();
            guard_product_fixture(&coordinator);
            let target = key("plugin-selections", "acme.plugin");
            let decision = crate::V2ProductDecision {
                origin: server.origin.clone(),
                account_id: "account-a".into(),
                config_generation: 42,
                source_generation: 7,
                plugin_id: "acme.plugin".into(),
                selection: Some(selection()),
            };
            coordinator
                .stage_product_decision(
                    target.clone(),
                    WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap()),
                    7,
                    &decision.digest().unwrap(),
                )
                .unwrap();
            let current = Arc::new(AtomicBool::new(true));
            let boundary = Arc::new(AtomicBool::new(false));
            let current_check = current.clone();
            let guard_current = current.clone();
            let guard_boundary = boundary.clone();
            let eligibility = WorkspaceEligibility::new(session.clone(), move || {
                current_check.load(Ordering::SeqCst)
            })
            .with_publication_guard(move |publish| {
                if guard_boundary.swap(false, Ordering::SeqCst) {
                    guard_current.store(false, Ordering::SeqCst);
                }
                if !guard_current.load(Ordering::SeqCst) {
                    return Err(WorkspaceCoordError::Ineligible);
                }
                publish()
            })
            .with_test_emission_guard();
            let source = ProductUploadFixture {
                generation: AtomicU64::new(7),
                consent: 42,
                expire_eligibility: None,
            };
            let admission = ProductUploadAdmission {
                source: &source,
                config_generation: 42,
            };
            let (entered_tx, entered_rx) = oneshot::channel();
            let (release_tx, release_rx) = oneshot::channel();
            {
                let mut remote = server.remote.lock().unwrap();
                remote.conflict_next_write = conflict;
                remote.pause_next_write = Some(Pause {
                    entered: entered_tx,
                    release: release_rx,
                });
            }
            let control = async {
                entered_rx.await.unwrap();
                let prepared = coordinator.snapshot().unwrap();
                assert_eq!(
                    pending(&coordinator, &target).pending.unwrap().status,
                    WorkspacePendingStatus::Unconfirmed
                );
                if mode == 0 {
                    current.store(false, Ordering::SeqCst);
                } else if mode == 1 {
                    boundary.store(true, Ordering::SeqCst);
                }
                release_tx.send(()).unwrap();
                prepared
            };
            let (result, prepared) = tokio::join!(
                coordinator.sync_product_guarded(&session, &eligibility, &admission),
                control
            );
            assert_eq!(server.remote.lock().unwrap().writes(), 1);
            if mode < 2 {
                assert!(matches!(result, Err(WorkspaceCoordError::Ineligible)));
                let after = coordinator.snapshot().unwrap();
                assert_eq!(after.generation, prepared.generation);
                assert_eq!(after.state, prepared.state);
                assert_eq!(
                    pending(&coordinator, &target).pending.unwrap().status,
                    WorkspacePendingStatus::Unconfirmed
                );
            } else {
                let result = result.unwrap();
                if conflict {
                    assert_eq!(result.conflicts, vec![target.clone()]);
                    assert_eq!(
                        pending(&coordinator, &target).pending.unwrap().status,
                        WorkspacePendingStatus::Conflict
                    );
                } else {
                    assert_eq!(result.pushed, 1);
                    assert!(pending(&coordinator, &target).pending.is_none());
                }
            }
        }
    }
}

struct BlockingProductSource {
    entered: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
    finished: Mutex<Option<oneshot::Sender<()>>>,
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_while_last_eligibility_check_is_blocked_does_not_send() {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let temp = tempfile::tempdir().unwrap();
    let coordinator = Arc::new(coordinator(
        &temp.path().join("private"),
        &server,
        "account-a",
    ));
    coordinator.bootstrap(&[], None).unwrap();
    guard_product_fixture(&coordinator);
    let target = key("plugin-selections", "acme.plugin");
    let decision = crate::V2ProductDecision {
        origin: server.origin.clone(),
        account_id: "account-a".into(),
        config_generation: 42,
        source_generation: 7,
        plugin_id: "acme.plugin".into(),
        selection: Some(selection()),
    };
    coordinator
        .stage_product_decision(
            target.clone(),
            WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap()),
            7,
            &decision.digest().unwrap(),
        )
        .unwrap();
    let source = Arc::new(ProductUploadFixture {
        generation: AtomicU64::new(7),
        consent: 42,
        expire_eligibility: None,
    });
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (checked_tx, checked_rx) = oneshot::channel();
    let entered = Mutex::new(Some(entered_tx));
    let release = Mutex::new(release_rx);
    let checked = Mutex::new(Some(checked_tx));
    let claimed = AtomicBool::new(false);
    let observed = coordinator.clone();
    let observed_target = target.clone();
    let worker = coordinator.clone();
    let task = tokio::spawn(async move {
        crate::v2_product_outbox::run_product_worker(move |cancelled, deadline| async move {
            let eligibility = WorkspaceEligibility::new(session.clone(), move || {
                let mut blocked_here = false;
                let allowed =
                    crate::v2_product_outbox::product_eligible(&cancelled, deadline, || {
                        let prepared = observed
                            .snapshot()
                            .ok()
                            .and_then(|snapshot| {
                                snapshot
                                    .state
                                    .entry(&observed_target)
                                    .and_then(|entry| entry.pending.as_ref())
                                    .map(|pending| {
                                        pending.status == WorkspacePendingStatus::Unconfirmed
                                    })
                            })
                            .unwrap_or(false);
                        if prepared && !claimed.swap(true, Ordering::AcqRel) {
                            blocked_here = true;
                            if let Some(entered) = entered.lock().unwrap().take() {
                                let _ = entered.send(());
                            }
                            release
                                .lock()
                                .unwrap()
                                .recv_timeout(std::time::Duration::from_secs(4))
                                .unwrap();
                        }
                        true
                    });
                if blocked_here {
                    if let Some(checked) = checked.lock().unwrap().take() {
                        let _ = checked.send(allowed);
                    }
                }
                allowed
            })
            .with_test_publication_guard();
            worker
                .sync_product_guarded(
                    &session,
                    &eligibility,
                    &ProductUploadAdmission {
                        source: source.as_ref(),
                        config_generation: 42,
                    },
                )
                .await
        })
        .await
    });

    tokio::time::timeout(std::time::Duration::from_secs(3), entered_rx)
        .await
        .expect("must reach the eligibility check after durable preparation")
        .unwrap();
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
    task.abort();
    let _ = task.await;
    release_tx.send(()).unwrap();
    let admitted = tokio::time::timeout(std::time::Duration::from_secs(3), checked_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !admitted,
        "a cancelled check must not use its stale pre-I/O observation"
    );
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if coordinator.store.try_operation_lock().unwrap().is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(server.remote.lock().unwrap().writes(), 0);
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
}
impl crate::V2ProductUploadSource for BlockingProductSource {
    fn acquire_emission_guard(&self) -> Result<Box<dyn Send>, WorkspaceCoordError> {
        Ok(Box::new(()))
    }

    fn with_fenced_upload(
        &self,
        _decision: &crate::V2ProductDecision,
        admit: &mut dyn FnMut() -> Result<(), WorkspaceCoordError>,
    ) -> Result<(), WorkspaceCoordError> {
        if let Some(entered) = self.entered.lock().unwrap().take() {
            let _ = entered.send(());
        }
        self.release
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(4))
            .unwrap();
        let result = admit();
        if let Some(finished) = self.finished.lock().unwrap().take() {
            let _ = finished.send(());
        }
        result
    }
}

#[tokio::test(flavor = "current_thread")]
async fn product_worker_cancellation_does_not_block_executor_or_send_after_source_unblocks() {
    use std::sync::atomic::Ordering;
    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let temp = tempfile::tempdir().unwrap();
    let coordinator = Arc::new(coordinator(
        &temp.path().join("private"),
        &server,
        "account-a",
    ));
    coordinator.bootstrap(&[], None).unwrap();
    guard_product_fixture(&coordinator);
    let target = key("plugin-selections", "acme.plugin");
    let decision = crate::V2ProductDecision {
        origin: server.origin.clone(),
        account_id: "account-a".into(),
        config_generation: 42,
        source_generation: 7,
        plugin_id: "acme.plugin".into(),
        selection: Some(selection()),
    };
    coordinator
        .stage_product_decision(
            target.clone(),
            WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap()),
            7,
            &decision.digest().unwrap(),
        )
        .unwrap();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (finished_tx, finished_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let source = Arc::new(BlockingProductSource {
        entered: Mutex::new(Some(entered_tx)),
        release: Mutex::new(release_rx),
        finished: Mutex::new(Some(finished_tx)),
    });
    let other = coordinator.clone();
    let task = tokio::spawn(async move {
        crate::v2_product_outbox::run_product_worker(move |cancelled, deadline| async move {
            let flag = cancelled.clone();
            let eligibility = WorkspaceEligibility::new(session.clone(), move || {
                !flag.load(Ordering::Acquire) && std::time::Instant::now() < deadline
            })
            .with_test_publication_guard();
            other
                .sync_product_guarded(
                    &session,
                    &eligibility,
                    &ProductUploadAdmission {
                        source: source.as_ref(),
                        config_generation: 42,
                    },
                )
                .await
        })
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), entered_rx)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        tokio::time::sleep(std::time::Duration::from_millis(10)),
    )
    .await
    .expect("slow source must not block the single-threaded executor");
    task.abort();
    let _ = task.await;
    release_tx.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), finished_rx)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if coordinator.store.try_operation_lock().unwrap().is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("workspace lease must be released after source unblocks");
    assert_eq!(server.remote.lock().unwrap().writes(), 0);
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Queued
    );
}

#[tokio::test(flavor = "current_thread")]
async fn product_worker_deadline_returns_while_source_blocked_without_sending_later() {
    use std::sync::atomic::Ordering;
    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let temp = tempfile::tempdir().unwrap();
    let coordinator = Arc::new(coordinator(
        &temp.path().join("private"),
        &server,
        "account-a",
    ));
    coordinator.bootstrap(&[], None).unwrap();
    guard_product_fixture(&coordinator);
    let target = key("plugin-selections", "acme.plugin");
    let decision = crate::V2ProductDecision {
        origin: server.origin.clone(),
        account_id: "account-a".into(),
        config_generation: 42,
        source_generation: 7,
        plugin_id: "acme.plugin".into(),
        selection: Some(selection()),
    };
    coordinator
        .stage_product_decision(
            target.clone(),
            WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap()),
            7,
            &decision.digest().unwrap(),
        )
        .unwrap();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (finished_tx, finished_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let source = Arc::new(BlockingProductSource {
        entered: Mutex::new(Some(entered_tx)),
        release: Mutex::new(release_rx),
        finished: Mutex::new(Some(finished_tx)),
    });
    let slots = Arc::new(tokio::sync::Semaphore::new(1));
    let available = slots.clone();
    let other = coordinator.clone();
    let task = tokio::spawn(async move {
        crate::v2_product_outbox::run_product_worker_with_slots(
            std::time::Duration::from_secs(1),
            slots,
            move |cancelled, deadline| async move {
                let flag = cancelled.clone();
                let eligibility = WorkspaceEligibility::new(session.clone(), move || {
                    !flag.load(Ordering::Acquire) && std::time::Instant::now() < deadline
                })
                .with_test_publication_guard();
                other
                    .sync_product_guarded(
                        &session,
                        &eligibility,
                        &ProductUploadAdmission {
                            source: source.as_ref(),
                            config_generation: 42,
                        },
                    )
                    .await
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), entered_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap(),
        Err(WorkspaceCoordError::TimedOut)
    ));
    assert_eq!(
        available.available_permits(),
        0,
        "blocked native worker retains capacity"
    );
    assert!(
        coordinator.store.try_operation_lock().unwrap().is_none(),
        "blocked native worker retains the workspace lease"
    );
    assert_eq!(server.remote.lock().unwrap().writes(), 0);
    release_tx.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), finished_rx)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if available.available_permits() > 0
                && coordinator.store.try_operation_lock().unwrap().is_some()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("worker capacity and workspace lease must return once source unblocks");
    assert_eq!(server.remote.lock().unwrap().writes(), 0);
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Queued
    );
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_product_worker_releases_http_lease_and_retains_unknown_result() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let temp = tempfile::tempdir().unwrap();
    let coordinator = Arc::new(coordinator(
        &temp.path().join("private"),
        &server,
        "account-a",
    ));
    coordinator.bootstrap(&[], None).unwrap();
    guard_product_fixture(&coordinator);
    let target = key("plugin-selections", "acme.plugin");
    let decision = crate::V2ProductDecision {
        origin: server.origin.clone(),
        account_id: "account-a".into(),
        config_generation: 42,
        source_generation: 7,
        plugin_id: "acme.plugin".into(),
        selection: Some(selection()),
    };
    coordinator
        .stage_product_decision(
            target.clone(),
            WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap()),
            7,
            &decision.digest().unwrap(),
        )
        .unwrap();
    let source = Arc::new(ProductUploadFixture {
        generation: AtomicU64::new(7),
        consent: 42,
        expire_eligibility: None,
    });
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    server.remote.lock().unwrap().pause_next_write = Some(Pause {
        entered: entered_tx,
        release: release_rx,
    });
    let other = coordinator.clone();
    let task = tokio::spawn(async move {
        crate::v2_product_outbox::run_product_worker(move |cancelled, deadline| async move {
            let flag = cancelled.clone();
            let eligibility = WorkspaceEligibility::new(session.clone(), move || {
                !flag.load(Ordering::Acquire) && std::time::Instant::now() < deadline
            })
            .with_test_publication_guard();
            other
                .sync_product_guarded(
                    &session,
                    &eligibility,
                    &ProductUploadAdmission {
                        source: source.as_ref(),
                        config_generation: 42,
                    },
                )
                .await
        })
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), entered_rx)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    let _ = task.await;
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if coordinator.store.try_operation_lock().unwrap().is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("cancelled HTTP must not retain the workspace lease");
    assert_eq!(server.remote.lock().unwrap().writes(), 1);
    assert_eq!(
        pending(&coordinator, &target).pending.unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
    let _ = release_tx.send(());
}

#[tokio::test]
async fn five_collections_two_devices_offline_restart_and_explicit_v1_bootstrap() {
    let server = start_server().await;
    let a = connect(&server, "device-a").await;
    let b = connect(&server, "device-b").await;
    let temp = tempfile::tempdir().unwrap();
    let root_a = temp.path().join("device-a-private");
    let root_b = temp.path().join("device-b-private");
    let ca = coordinator(&root_a, &server, "account-a");
    let mut legacy = a.new_state();
    legacy.cursor = 9001;
    legacy
        .stage("acme.plugin".into(), cloud_client::Edit::Put(selection()))
        .unwrap();
    let v1 = serde_json::to_vec(&legacy).unwrap();
    std::fs::write(root_a.join("sync-state.json"), &v1).unwrap();
    ca.bootstrap(&[selection()], Some(&legacy)).unwrap();
    assert_eq!(ca.snapshot().unwrap().state.cursor, 0);
    assert!(matches!(
        ca.bootstrap(&[], None),
        Err(WorkspaceCoordError::AlreadyBootstrapped)
    ));
    let values = vec![
        (key("rule-templates", "gift-rule"), template()),
        (
            key("profiles", "gift-pack"),
            json!({"profileVersion":1,"id":"gift-pack","name":"Gift pack","templates":[{"template":template()}]}),
        ),
        (
            key("plugin-settings", "acme.plugin"),
            json!({"schemaVersion":1,"pluginId":"acme.plugin","settings":{"fontSize":18}}),
        ),
        (key("preferences", "desktop"), preference("dark")),
    ];
    let network_before = server.remote.lock().unwrap().requests.len();
    for (target, value) in &values {
        ca.stage(target.clone(), WorkspaceEdit::Put(value.clone()))
            .unwrap();
    }
    assert_eq!(server.remote.lock().unwrap().requests.len(), network_before);
    drop(ca);
    let ca = coordinator(&root_a, &server, "account-a");
    assert_eq!(ca.snapshot().unwrap().state.pending_keys().len(), 5);
    {
        let mut remote = server.remote.lock().unwrap();
        for entry in &ca.snapshot().unwrap().state.entries {
            remote.proofs.push(DurableProof {
                root: root_a.clone(),
                account: "account-a".into(),
                target: entry.target.clone(),
                edit: entry.pending.as_ref().unwrap().edit.clone(),
            });
        }
    }
    let sent = ca.sync(&a).await.unwrap();
    assert_eq!(sent.pushed, 5);
    assert_eq!(sent.pending, 0);
    assert!(!sent.more_queued);
    assert!(server.remote.lock().unwrap().proofs.is_empty());
    assert_eq!(std::fs::read(root_a.join("sync-state.json")).unwrap(), v1);
    let cb = coordinator(&root_b, &server, "account-a");
    cb.bootstrap(&[], None).unwrap();
    let received = cb.sync(&b).await.unwrap();
    assert_eq!(received.pulled, 5);
    assert_eq!(received.pushed, 0);
    assert_eq!(received.remote.len(), 5);
    cb.stage(
        key("preferences", "desktop"),
        WorkspaceEdit::Put(preference("light")),
    )
    .unwrap();
    drop(cb);
    let cb = coordinator(&root_b, &server, "account-a");
    assert_eq!(cb.sync(&b).await.unwrap().pushed, 1);
    assert_eq!(ca.sync(&a).await.unwrap().pushed, 0);
    assert_eq!(
        pending(&ca, &key("preferences", "desktop"))
            .remote
            .unwrap()
            .value,
        Some(preference("light"))
    );
    let writes = server.remote.lock().unwrap().writes();
    assert_eq!(cb.sync(&b).await.unwrap().pushed, 0);
    assert_eq!(server.remote.lock().unwrap().writes(), writes);
    let removed = key("profiles", "gift-pack");
    cb.stage(removed.clone(), WorkspaceEdit::Delete).unwrap();
    assert_eq!(cb.sync(&b).await.unwrap().pushed, 1);
    ca.sync(&a).await.unwrap();
    let tombstone = pending(&ca, &removed).remote.unwrap();
    assert!(tombstone.deleted);
    assert!(tombstone.value.is_none());
    no_v1(&server);
}

#[tokio::test]
async fn lost_successful_reply_stays_unconfirmed_until_fresh_review_and_explicit_choice() {
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private-workspace");
    let target = key("preferences", "desktop");
    let c = coordinator(&root, &server, "account-a");
    c.bootstrap(&[], None).unwrap();
    c.stage(target.clone(), WorkspaceEdit::Put(preference("dark")))
        .unwrap();
    {
        let mut remote = server.remote.lock().unwrap();
        remote.lose_next_write = true;
        remote.proofs.push(DurableProof {
            root: root.clone(),
            account: "account-a".into(),
            target: target.clone(),
            edit: WorkspaceEdit::Put(preference("dark")),
        });
    }
    assert!(matches!(
        c.sync(&session).await,
        Err(WorkspaceCoordError::Transport(SyncError::Network))
    ));
    assert_eq!(
        pending(&c, &target).pending.unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
    drop(c);
    let c = coordinator(&root, &server, "account-a");
    let recovered = c.sync(&session).await.unwrap();
    assert_eq!(recovered.pushed, 0);
    assert_eq!(recovered.pending, 1);
    assert!(recovered.remote.is_empty());
    assert_eq!(server.remote.lock().unwrap().writes(), 1);
    let review = c.review(&session, &target).await.unwrap();
    assert_eq!(review.remote().unwrap().value, Some(preference("dark")));
    assert_eq!(
        review.entry().pending.as_ref().unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
    assert_eq!(server.remote.lock().unwrap().writes(), 1);
    c.resolve(&session, &review, false).await.unwrap();
    assert!(pending(&c, &target).pending.is_none());
    assert_eq!(c.sync(&session).await.unwrap().pushed, 0);
    assert_eq!(server.remote.lock().unwrap().writes(), 1);
    no_v1(&server);
}

#[tokio::test]
async fn expired_cursor_blocks_sync_until_explicit_readonly_rebuild_preserving_pending() {
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private-workspace");
    let target = key("preferences", "desktop");
    let c = coordinator(&root, &server, "account-a");
    c.bootstrap(&[], None).unwrap();
    c.stage(target.clone(), WorkspaceEdit::Put(preference("light")))
        .unwrap();
    {
        let mut remote = server.remote.lock().unwrap();
        remote.put("account-a", target.clone(), Some(preference("dark")));
        remote.expire_changes = true;
    }
    assert!(matches!(
        c.sync(&session).await,
        Err(WorkspaceCoordError::State(
            WorkspaceStateError::NeedsRebuild
        ))
    ));
    let old = c.snapshot().unwrap().state;
    assert!(old.needs_rebuild);
    assert_eq!(old.cursor, 0);
    let count = server.remote.lock().unwrap().requests.len();
    assert!(matches!(
        c.sync(&session).await,
        Err(WorkspaceCoordError::State(
            WorkspaceStateError::NeedsRebuild
        ))
    ));
    assert_eq!(server.remote.lock().unwrap().requests.len(), count);
    assert!(!server
        .remote
        .lock()
        .unwrap()
        .requests
        .iter()
        .any(|r| r.target == "/api/v2/sync/export"));
    c.rebuild_remote_cache(&session).await.unwrap();
    let rebuilt = c.snapshot().unwrap().state;
    assert!(!rebuilt.needs_rebuild);
    assert_eq!(rebuilt.cursor, 1);
    let entry = rebuilt.entry(&target).unwrap();
    assert_eq!(
        entry.pending.as_ref().unwrap().edit,
        old.entry(&target).unwrap().pending.as_ref().unwrap().edit
    );
    assert_eq!(
        entry.pending.as_ref().unwrap().status,
        WorkspacePendingStatus::Conflict
    );
    assert_eq!(
        entry.remote.as_ref().unwrap().value,
        Some(preference("dark"))
    );
    assert_eq!(server.remote.lock().unwrap().writes(), 0);
    server.remote.lock().unwrap().expire_changes = false;
    assert_eq!(c.sync(&session).await.unwrap().pushed, 0);
    no_v1(&server);
}

#[tokio::test]
async fn live_network_lock_keeps_local_stage_and_promotes_next_after_original_ack() {
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private-workspace");
    let target = key("preferences", "desktop");
    let c = coordinator(&root, &server, "account-a");
    c.bootstrap(&[], None).unwrap();
    c.stage(target.clone(), WorkspaceEdit::Put(preference("dark")))
        .unwrap();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    {
        let mut remote = server.remote.lock().unwrap();
        remote.pause_next_write = Some(Pause {
            entered: entered_tx,
            release: release_rx,
        });
        remote.proofs.push(DurableProof {
            root: root.clone(),
            account: "account-a".into(),
            target: target.clone(),
            edit: WorkspaceEdit::Put(preference("dark")),
        });
    }
    let other = coordinator(&root, &server, "account-a");
    let control = async {
        entered_rx.await.unwrap();
        let generation = other
            .stage(target.clone(), WorkspaceEdit::Put(preference("light")))
            .unwrap();
        let entry = pending(&other, &target);
        assert_eq!(
            entry.pending.as_ref().unwrap().edit,
            WorkspaceEdit::Put(preference("dark"))
        );
        assert_eq!(entry.next.as_ref().unwrap().local_generation, generation);
        assert!(matches!(
            other.sync(&session).await,
            Err(WorkspaceCoordError::Busy)
        ));
        assert!(matches!(
            other.review(&session, &target).await,
            Err(WorkspaceCoordError::Busy)
        ));
        assert!(matches!(
            other.bootstrap(&[], None),
            Err(WorkspaceCoordError::Busy)
        ));
        release_tx.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(c.sync(&session), control);
    let result = result.unwrap();
    assert_eq!(result.pushed, 1);
    assert_eq!(result.pending, 1);
    assert!(result.more_queued);
    let entry = pending(&c, &target);
    assert!(entry.next.is_none());
    let p = entry.pending.unwrap();
    assert_eq!(p.status, WorkspacePendingStatus::Queued);
    assert_eq!(p.base_revision, Some(1));
    assert_eq!(p.edit, WorkspaceEdit::Put(preference("light")));
    assert_eq!(c.sync(&session).await.unwrap().pushed, 1);
    assert_eq!(server.remote.lock().unwrap().writes(), 2);
    no_v1(&server);
}

#[tokio::test]
async fn conflict_reviews_fence_candidate_generation_live_remote_and_account_origin() {
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let other_session = connect(&server, "other-account").await;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private-workspace");
    let target = key("preferences", "desktop");
    server
        .remote
        .lock()
        .unwrap()
        .put("account-a", target.clone(), Some(preference("dark")));
    let c = coordinator(&root, &server, "account-a");
    c.bootstrap(&[], None).unwrap();
    c.sync(&session).await.unwrap();
    c.stage(target.clone(), WorkspaceEdit::Put(preference("light")))
        .unwrap();
    server
        .remote
        .lock()
        .unwrap()
        .put("account-a", target.clone(), Some(preference("system")));
    let conflict = c.sync(&session).await.unwrap();
    assert_eq!(conflict.pushed, 0);
    assert_eq!(conflict.conflicts, vec![target.clone()]);
    let review = c.review(&session, &target).await.unwrap();
    assert_eq!(
        review
            .entry()
            .pending
            .as_ref()
            .unwrap()
            .base_record
            .as_ref()
            .unwrap()
            .value,
        Some(preference("dark"))
    );
    c.stage(target.clone(), WorkspaceEdit::Put(preference("dark")))
        .unwrap();
    assert!(matches!(
        c.resolve(&session, &review, true).await,
        Err(WorkspaceCoordError::State(WorkspaceStateError::StaleReview))
    ));
    let review = c.review(&session, &target).await.unwrap();
    server
        .remote
        .lock()
        .unwrap()
        .put("account-a", target.clone(), Some(preference("light")));
    assert!(matches!(
        c.resolve(&session, &review, true).await,
        Err(WorkspaceCoordError::State(WorkspaceStateError::StaleReview))
    ));
    let review = c.review(&session, &target).await.unwrap();
    let before = server.remote.lock().unwrap().requests.len();
    assert!(matches!(
        c.resolve(&other_session, &review, true).await,
        Err(WorkspaceCoordError::State(
            WorkspaceStateError::WrongIdentity
        ))
    ));
    let other = coordinator(&root, &server, "account-b");
    other.bootstrap(&[], None).unwrap();
    assert!(matches!(
        other.resolve(&other_session, &review, true).await,
        Err(WorkspaceCoordError::State(
            WorkspaceStateError::WrongIdentity
        ))
    ));
    assert!(other.snapshot().unwrap().state.entries.is_empty());
    assert_eq!(server.remote.lock().unwrap().requests.len(), before);
    let other_origin = start_server().await;
    let other_origin_session = connect(&other_origin, "device-a").await;
    assert!(matches!(
        c.resolve(&other_origin_session, &review, true).await,
        Err(WorkspaceCoordError::State(
            WorkspaceStateError::WrongIdentity
        ))
    ));
    c.resolve(&session, &review, true).await.unwrap();
    assert_eq!(
        pending(&c, &target).pending.as_ref().unwrap().base_revision,
        Some(3)
    );
    assert_eq!(c.sync(&session).await.unwrap().pushed, 1);
    assert_eq!(
        pending(&c, &target).remote.as_ref().unwrap().value,
        Some(preference("dark"))
    );
    no_v1(&server);
}

#[tokio::test]
async fn same_generation_on_another_device_cannot_resolve_an_unreviewed_local_candidate() {
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let temp = tempfile::tempdir().unwrap();
    let target = key("preferences", "desktop");
    let a = coordinator(&temp.path().join("a-private"), &server, "account-a");
    let b = coordinator(&temp.path().join("b-private"), &server, "account-a");
    a.bootstrap(&[], None).unwrap();
    b.bootstrap(&[], None).unwrap();
    a.stage(target.clone(), WorkspaceEdit::Put(preference("dark")))
        .unwrap();
    b.stage(target.clone(), WorkspaceEdit::Put(preference("light")))
        .unwrap();
    let review = a.review(&session, &target).await.unwrap();
    let request_count = server.remote.lock().unwrap().requests.len();
    assert_eq!(
        review.local_generation(),
        b.snapshot()
            .unwrap()
            .state
            .local_generation(&target)
            .unwrap()
    );
    assert!(matches!(
        b.resolve(&session, &review, true).await,
        Err(WorkspaceCoordError::State(WorkspaceStateError::StaleReview))
    ));
    assert_eq!(server.remote.lock().unwrap().requests.len(), request_count);
    assert_eq!(
        pending(&b, &target).pending.unwrap().edit,
        WorkspaceEdit::Put(preference("light"))
    );
    assert_eq!(server.remote.lock().unwrap().writes(), 0);
}

#[tokio::test]
async fn deleting_an_unbased_absent_record_requires_a_live_get_but_no_write() {
    let server = start_server().await;
    let session = connect(&server, "device-a").await;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private-workspace");
    let target = key("preferences", "desktop");
    let c = coordinator(&root, &server, "account-a");
    assert!(matches!(
        c.stage(target.clone(), WorkspaceEdit::Delete),
        Err(WorkspaceCoordError::BootstrapRequired)
    ));
    c.bootstrap(&[], None).unwrap();
    c.stage(target.clone(), WorkspaceEdit::Delete).unwrap();
    let result = c.sync(&session).await.unwrap();
    assert_eq!(result.absent_deletes, 1);
    assert_eq!(result.pushed, 0);
    assert_eq!(result.pending, 0);
    assert!(c.snapshot().unwrap().state.entry(&target).is_none());
    let remote = server.remote.lock().unwrap();
    assert_eq!(remote.writes(), 0);
    assert_eq!(
        remote
            .requests
            .iter()
            .filter(|r| r.target == "/api/v2/sync/collections/preferences/desktop")
            .count(),
        1
    );
}

#[tokio::test]
async fn guarded_sync_stops_after_session_change_and_keeps_known_write_outcome() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let epoch = Arc::new(AtomicU64::new(1));
    let current = epoch.clone();
    let eligibility =
        WorkspaceEligibility::new(session.clone(), move || current.load(Ordering::SeqCst) == 1);
    let temp = tempfile::tempdir().unwrap();
    let c = coordinator(&temp.path().join("private"), &server, "account-a");
    c.bootstrap(&[], None).unwrap();
    let first = key("preferences", "desktop");
    let second = key("plugin-selections", "acme.plugin");
    // Sorted keys put plugin-selections before preferences.
    c.stage(first.clone(), WorkspaceEdit::Put(preference("dark")))
        .unwrap();
    c.stage(
        second.clone(),
        WorkspaceEdit::Put(serde_json::to_value(selection()).unwrap()),
    )
    .unwrap();
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    server.remote.lock().unwrap().pause_next_write = Some(Pause {
        entered: entered_tx,
        release: release_rx,
    });
    let control = async {
        entered_rx.await.unwrap();
        epoch.store(2, Ordering::SeqCst);
        release_tx.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(c.sync_guarded(&session, &eligibility), control);
    assert!(matches!(result, Err(WorkspaceCoordError::Ineligible)));
    assert_eq!(server.remote.lock().unwrap().writes(), 1);
    assert!(pending(&c, &second).pending.is_none());
    assert_eq!(
        pending(&c, &first).pending.unwrap().status,
        WorkspacePendingStatus::Queued
    );
    // Re-enabling under a fresh generation must not resurrect the old permit.
    let newer = epoch.clone();
    let fresh =
        WorkspaceEligibility::new(session.clone(), move || newer.load(Ordering::SeqCst) == 2);
    assert!(matches!(
        c.sync_guarded(&session, &eligibility).await,
        Err(WorkspaceCoordError::Ineligible)
    ));
    assert_eq!(c.sync_guarded(&session, &fresh).await.unwrap().pushed, 1);
    assert_eq!(server.remote.lock().unwrap().writes(), 2);
}

#[tokio::test]
async fn guarded_review_requires_original_permit_and_exact_session() {
    let server = start_server().await;
    let session = Arc::new(connect(&server, "device-a").await);
    let other_session = Arc::new(connect(&server, "device-b").await);
    let permit = WorkspaceEligibility::new(session.clone(), || true);
    let other_permit = WorkspaceEligibility::new(session.clone(), || true);
    let temp = tempfile::tempdir().unwrap();
    let c = coordinator(&temp.path().join("private"), &server, "account-a");
    c.bootstrap(&[], None).unwrap();
    let target = key("preferences", "desktop");
    c.stage(target.clone(), WorkspaceEdit::Put(preference("dark")))
        .unwrap();
    let review = c.review_guarded(&session, &target, &permit).await.unwrap();
    let before = server.remote.lock().unwrap().requests.len();
    assert!(matches!(
        c.resolve_guarded(&session, &review, false, &other_permit)
            .await,
        Err(WorkspaceCoordError::State(WorkspaceStateError::StaleReview))
    ));
    assert!(matches!(
        c.resolve_guarded(&other_session, &review, false, &permit)
            .await,
        Err(WorkspaceCoordError::Ineligible)
    ));
    assert!(matches!(
        c.resolve(&session, &review, false).await,
        Err(WorkspaceCoordError::State(WorkspaceStateError::StaleReview))
    ));
    assert_eq!(server.remote.lock().unwrap().requests.len(), before);
    c.resolve_guarded(&session, &review, false, &permit)
        .await
        .unwrap();
}
