//! Coordinator tests against a mock Cloud speaking the real sync protocol.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore, TokenStore};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{SyncCoordError, SyncCoordinator};

struct Stored {
    revision: i64,
    deleted: bool,
    value: Option<serde_json::Value>,
}

struct MockInner {
    account_id: String,
    tokens: HashSet<String>,
    codes: HashSet<String>,
    records: BTreeMap<String, Stored>,
    cursor: i64,
    expire_once: bool,
    fail_sync: bool,
}

struct MockSync {
    origin: String,
    task: tokio::task::JoinHandle<()>,
}

impl MockSync {
    async fn start(account_id: &str) -> Self {
        Self::start_seeded(account_id, 0, HashSet::new()).await
    }

    async fn start_on(account_id: &str, port: u16) -> Self {
        Self::start_seeded(account_id, port, HashSet::new()).await
    }

    async fn start_seeded(account_id: &str, port: u16, tokens: HashSet<String>) -> Self {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let inner = Arc::new(Mutex::new(MockInner {
            account_id: account_id.to_owned(),
            tokens,
            codes: HashSet::new(),
            records: BTreeMap::new(),
            cursor: 0,
            expire_once: false,
            fail_sync: false,
        }));
        let task = tokio::spawn({
            let inner = inner.clone();
            async move {
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        break;
                    };
                    let inner = inner.clone();
                    tokio::spawn(async move {
                        let mut buffer = vec![0_u8; 32 * 1024];
                        let Ok(read) = socket.read(&mut buffer).await else {
                            return;
                        };
                        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                        let mut lines = request.lines();
                        let head = lines.next().unwrap_or_default().to_owned();
                        let mut parts = head.split_whitespace();
                        let method = parts.next().unwrap_or_default().to_owned();
                        let target = parts.next().unwrap_or("/").to_owned();
                        let (path, query) = target
                            .split_once('?')
                            .map_or_else(|| (target.as_str(), ""), |(p, q)| (p, q));
                        let headers: HashMap<String, String> = lines
                            .take_while(|line| !line.is_empty())
                            .filter_map(|line| line.split_once(':'))
                            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
                            .collect();
                        let body_start = request.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
                        let body = request[body_start.min(request.len())..].to_owned();
                        let (status, payload) =
                            route(&method, path, query, &headers, &body, &inner);
                        let head = format!(
                            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                            payload.len()
                        );
                        let _ = socket.write_all(head.as_bytes()).await;
                        let _ = socket.write_all(payload.as_bytes()).await;
                    });
                }
            }
        });
        Self { origin, task }
    }

    fn stop(self) {
        self.task.abort();
    }
}

fn json(status: u16, value: serde_json::Value) -> (String, String) {
    (
        format!("{status} {}", reason(status)),
        serde_json::to_string(&value).unwrap(),
    )
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        410 => "Gone",
        500 => "Internal Server Error",
        _ => "Error",
    }
}

fn route(
    method: &str,
    path: &str,
    query: &str,
    headers: &HashMap<String, String>,
    body: &str,
    state: &Arc<Mutex<MockInner>>,
) -> (String, String) {
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default()
        .to_owned();
    let mut state = state.lock().unwrap();
    // Test hooks never need auth.
    if (method, path) == ("POST", "/test/expire-cursor") {
        // One-shot: the next stale-cursor poll gets 410, then the server
        // behaves again, mirroring real retention expiry.
        state.expire_once = true;
        return ("200 OK".to_owned(), String::new());
    }
    if (method, path) == ("GET", "/test/export-tokens") {
        let tokens: Vec<&String> = state.tokens.iter().collect();
        return json(200, serde_json::json!({"tokens": tokens}));
    }
    if (method, path) == ("POST", "/test/fail-sync") {
        state.fail_sync = true;
        return ("200 OK".to_owned(), String::new());
    }
    if (method, path) == ("POST", "/test/recover") {
        state.expire_once = false;
        state.fail_sync = false;
        return ("200 OK".to_owned(), String::new());
    }
    if (method, path) == ("POST", "/api/v1/native/handoff/redeem") {
        let value: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
        let code = value
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or_default();
        if code.is_empty() || !state.codes.insert(code.to_owned()) {
            return json(400, serde_json::json!({"error": "bad or reused code"}));
        }
        if value
            .get("codeVerifier")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .is_empty()
        {
            return json(400, serde_json::json!({"error": "missing verifier"}));
        }
        let token = format!("token-{code}");
        state.tokens.insert(token.clone());
        return json(
            200,
            serde_json::json!({"sessionToken": token, "expiresAt": 9_999_999_999_u64}),
        );
    }
    if !state.tokens.contains(&bearer) {
        return json(401, serde_json::json!({"error": "unauthorized"}));
    }
    if state.fail_sync && path.starts_with("/api/v1/native/sync") {
        return json(500, serde_json::json!({"error": "boom"}));
    }
    match (method, path) {
        ("GET", "/api/v1/native/me") => json(
            200,
            serde_json::json!({"id": state.account_id, "email": "user@example.com"}),
        ),
        ("GET", "/api/v1/native/sync/changes") => {
            let cursor: i64 = query
                .split('&')
                .find_map(|p| p.strip_prefix("cursor="))
                .and_then(|n| n.parse().ok())
                .unwrap_or(-1);
            if cursor < 0 {
                return json(400, serde_json::json!({"error": "bad cursor"}));
            }
            if state.expire_once && cursor < state.cursor {
                state.expire_once = false;
                return ("410 Gone".to_owned(), String::new());
            }
            let mut changes: Vec<(i64, String, Stored)> = state
                .records
                .iter()
                .filter(|(_, r)| r.revision > cursor)
                .map(|(k, r)| {
                    (
                        r.revision,
                        k.clone(),
                        Stored {
                            revision: r.revision,
                            deleted: r.deleted,
                            value: r.value.clone(),
                        },
                    )
                })
                .collect();
            changes.sort_by_key(|(rev, _, _)| *rev);
            let payload: Vec<serde_json::Value> = changes
                .into_iter()
                .map(|(_, key, r)| {
                    serde_json::json!({
                        "collection": "plugin-selections",
                        "key": key,
                        "revision": r.revision,
                        "deleted": r.deleted,
                        "value": r.value,
                    })
                })
                .collect();
            json(
                200,
                // NOTE: the engine's Page struct uses snake_case with no
                // rename_all; only this endpoint speaks snake_case.
                serde_json::json!({"changes": payload, "cursor": state.cursor, "has_more": false}),
            )
        }
        ("GET", "/api/v1/native/sync/head") => {
            json(200, serde_json::json!({"cursor": state.cursor}))
        }
        ("GET", "/api/v1/native/sync/collections/plugin-selections") => {
            let after = query
                .split('&')
                .find_map(|p| p.strip_prefix("after="))
                .unwrap_or_default();
            let mut keys: Vec<&String> = state
                .records
                .keys()
                .filter(|k| k.as_str() > after)
                .collect();
            keys.sort();
            let page: Vec<&String> = keys.into_iter().take(2).collect();
            let next_after = if page.len() == 2 && state.records.keys().any(|k| k > page[1]) {
                Some(page[1].clone())
            } else {
                None
            };
            let records: Vec<serde_json::Value> = page
                .iter()
                .map(|key| {
                    let r = &state.records[*key];
                    serde_json::json!({
                        "collection": "plugin-selections",
                        "key": key,
                        "revision": r.revision,
                        "deleted": r.deleted,
                        "value": r.value,
                    })
                })
                .collect();
            json(
                200,
                serde_json::json!({"records": records, "nextAfter": next_after}),
            )
        }
        _ if path.starts_with("/api/v1/native/sync/collections/plugin-selections/") => {
            let key = path
                .strip_prefix("/api/v1/native/sync/collections/plugin-selections/")
                .unwrap_or_default()
                .to_owned();
            if key.is_empty() {
                return json(400, serde_json::json!({"error": "missing key"}));
            }
            let if_match = headers
                .get("if-match")
                .map(|v| v.trim_matches('"').to_owned());
            let if_none = headers.get("if-none-match").cloned();
            let current = state.records.get(&key).map(|r| r.revision);
            let matches = |want: &Option<String>| {
                want.as_ref()
                    .and_then(|w| w.parse::<i64>().ok())
                    .zip(current)
                    .is_some_and(|(w, rev)| w == rev)
            };
            // Preconditions mirror the sync wire contract: If-Match must name
            // the current revision, If-None-Match forbids an existing record.
            if method == "PUT" {
                let conflict = match (&current, &if_match, &if_none) {
                    (Some(_), _, Some(_)) => true,
                    (Some(_), Some(_), _) => !matches(&if_match),
                    (None, Some(_), _) => true,
                    _ => false,
                };
                if conflict {
                    return json(409, serde_json::json!({"error": "revision mismatch"}));
                }
                let value: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
                if value.get("pluginId").and_then(|v| v.as_str()) != Some(key.as_str()) {
                    return json(400, serde_json::json!({"error": "key mismatch"}));
                }
                state.cursor += 1;
                let revision = state.cursor;
                state.records.insert(
                    key.clone(),
                    Stored {
                        revision,
                        deleted: false,
                        value: Some(value),
                    },
                );
                return json(
                    200,
                    serde_json::json!({
                        "collection": "plugin-selections",
                        "key": key,
                        "revision": revision,
                        "deleted": false,
                        "value": state.records[&key].value,
                    }),
                );
            }
            if method == "DELETE" {
                let conflict = match (&current, &if_match) {
                    (Some(_), Some(_)) => !matches(&if_match),
                    (None, Some(_)) => true,
                    _ => false,
                };
                if conflict {
                    return json(409, serde_json::json!({"error": "revision mismatch"}));
                }
                state.cursor += 1;
                let revision = state.cursor;
                state.records.insert(
                    key.clone(),
                    Stored {
                        revision,
                        deleted: true,
                        value: None,
                    },
                );
                return json(
                    200,
                    serde_json::json!({
                        "collection": "plugin-selections",
                        "key": key,
                        "revision": revision,
                        "deleted": true,
                        "value": serde_json::Value::Null,
                    }),
                );
            }
            json(404, serde_json::json!({"error": "not found"}))
        }
        _ => json(404, serde_json::json!({"error": "not found"})),
    }
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tiktools-sync-{name}-{}-{suffix}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Drive the loopback callback while `complete_login` is parked, mirroring
/// the real browser flow.
async fn deliver(browser_url: &str, code: &str) {
    let parsed = url::Url::parse(browser_url).unwrap();
    let redirect = parsed
        .query_pairs()
        .find(|(key, _)| key == "redirectUri")
        .map(|(_, value)| value.to_string())
        .unwrap();
    let expected = parsed
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .unwrap();
    let response = reqwest::get(format!("{redirect}?code={code}&state={expected}"))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

async fn login(auth: &CloudAuth, code: &str) {
    let begun = auth.begin_login().await.unwrap();
    let waiting = auth.clone();
    let waiter = tokio::spawn(async move { waiting.complete_login().await });
    deliver(&begun.browser_url, code).await;
    waiter.await.unwrap().unwrap();
}

fn coordinator(
    dir: &TempDir,
    mock: &MockSync,
    name: &str,
    store: Arc<dyn TokenStore>,
) -> (CloudAuth, SyncCoordinator) {
    let auth = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join(&format!("{name}-account.json")),
        store,
    );
    let sync = SyncCoordinator::new(auth.clone(), dir.join(&format!("{name}-sync"))).unwrap();
    sync.set_enabled(true).unwrap();
    (auth, sync)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_devices_share_selections() {
    let dir = TempDir::new("two-devices");
    let mock = MockSync::start("user-1").await;
    let store_a: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let store_b: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let (auth_a, sync_a) = coordinator(&dir, &mock, "a", store_a);
    let (auth_b, sync_b) = coordinator(&dir, &mock, "b", store_b);
    login(&auth_a, "code-a1").await;
    login(&auth_b, "code-b1").await;
    assert!(sync_a
        .record_local_change("acme.player", "1.2.0", true)
        .unwrap());
    let outcome = sync_a.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);

    let outcome = sync_b.sync_now().await.unwrap();
    let remote: HashMap<_, _> = outcome
        .remote
        .into_iter()
        .map(|r| (r.key, r.selection))
        .collect();
    let selection = remote.get("acme.player").unwrap().as_ref().unwrap();
    assert_eq!(selection.version, "1.2.0");
    assert!(selection.enabled);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offline_edits_survive_and_upload_later() {
    let dir = TempDir::new("offline");
    let port = 18771;
    let mock = MockSync::start_on("user-2", port).await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-off1").await;
    // A restarted server keeps its issued tokens (durable auth store); carry
    // them over so the resume below exercises sync, not re-login.
    let exported: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/test/export-tokens", mock.origin))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tokens: HashSet<String> = exported
        .get("tokens")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    mock.stop();

    // Offline: staging works, syncing fails, nothing is lost.
    assert!(sync
        .record_local_change("acme.offline", "0.9.0", false)
        .unwrap());
    let error = sync.sync_now().await.unwrap_err();
    assert!(matches!(error, SyncCoordError::Engine(_)), "{error:?}");
    assert_eq!(sync.status().await.pending, 1);

    // Reconnect: the pending edit uploads on the next run.
    let mock = MockSync::start_seeded("user-2", port, tokens).await;
    let outcome = sync.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    assert_eq!(sync.status().await.pending, 0);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conflicting_changes_keep_local_on_choice() {
    let dir = TempDir::new("conflict-keep");
    let mock = MockSync::start("user-3").await;
    let (auth_a, sync_a) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth_a, "code-c1").await;
    let auth_b = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join("b-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_b, "code-c2").await;
    let sync_b = SyncCoordinator::new(auth_b.clone(), dir.join("b-sync")).unwrap();
    sync_b.set_enabled(true).unwrap();

    // Both sides edit the same key while apart; A uploads first.
    assert!(sync_a
        .record_local_change("acme.shared", "1.0.0", true)
        .unwrap());
    assert!(sync_b
        .record_local_change("acme.shared", "1.0.0", false)
        .unwrap());
    sync_a.sync_now().await.unwrap();
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.conflicts, vec!["acme.shared".to_owned()]);

    // Keep local: rebase over the remote revision and upload.
    sync_b.keep_local("acme.shared").unwrap();
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    assert!(outcome.conflicts.is_empty());
    let outcome = sync_a.sync_now().await.unwrap();
    let remote: HashMap<_, _> = outcome
        .remote
        .into_iter()
        .map(|r| (r.key, r.selection))
        .collect();
    assert!(!remote.get("acme.shared").unwrap().as_ref().unwrap().enabled);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conflicting_changes_take_remote_on_choice() {
    let dir = TempDir::new("conflict-take");
    let mock = MockSync::start("user-4").await;
    let (auth_a, sync_a) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth_a, "code-d1").await;
    let auth_b = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join("b-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_b, "code-d2").await;
    let sync_b = SyncCoordinator::new(auth_b.clone(), dir.join("b-sync")).unwrap();
    sync_b.set_enabled(true).unwrap();

    assert!(sync_a
        .record_local_change("acme.shared", "2.0.0", true)
        .unwrap());
    assert!(sync_b
        .record_local_change("acme.shared", "2.0.0", false)
        .unwrap());
    sync_a.sync_now().await.unwrap();
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.conflicts, vec!["acme.shared".to_owned()]);

    // Take remote: the local edit is dropped and the server value applies.
    let remote = sync_b.take_remote("acme.shared").unwrap().unwrap();
    assert!(remote.enabled);
    assert_eq!(sync_b.status().await.pending, 0);
    let outcome = sync_b.sync_now().await.unwrap();
    assert!(outcome.conflicts.is_empty());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_cursor_rebuilds_from_snapshot() {
    let dir = TempDir::new("expiry");
    let mock = MockSync::start("user-5").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-e1").await;
    assert!(sync
        .record_local_change("acme.keep", "1.0.0", true)
        .unwrap());
    sync.sync_now().await.unwrap();

    reqwest::Client::new()
        .post(format!("{}/test/expire-cursor", mock.origin))
        .send()
        .await
        .unwrap();
    assert!(sync
        .record_local_change("acme.late", "1.0.0", true)
        .unwrap());
    let outcome = sync.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    let keys: HashSet<_> = outcome.remote.into_iter().map(|r| r.key).collect();
    assert!(keys.contains("acme.keep"));
    assert!(keys.contains("acme.late"));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_propagates_as_tombstone() {
    let dir = TempDir::new("delete");
    let mock = MockSync::start("user-6").await;
    let (auth_a, sync_a) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth_a, "code-f1").await;
    let auth_b = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join("b-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_b, "code-f2").await;
    let sync_b = SyncCoordinator::new(auth_b.clone(), dir.join("b-sync")).unwrap();
    sync_b.set_enabled(true).unwrap();

    assert!(sync_a
        .record_local_change("acme.gone", "1.0.0", true)
        .unwrap());
    sync_a.sync_now().await.unwrap();
    sync_b.sync_now().await.unwrap();

    assert!(sync_a.record_local_removal("acme.gone").unwrap());
    sync_a.sync_now().await.unwrap();
    let outcome = sync_b.sync_now().await.unwrap();
    let tombstoned = outcome
        .remote
        .into_iter()
        .find(|r| r.key == "acme.gone")
        .unwrap();
    assert!(tombstoned.selection.is_none());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_pulls_are_idempotent() {
    let dir = TempDir::new("dupes");
    let mock = MockSync::start("user-7").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-g1").await;
    assert!(sync
        .record_local_change("acme.idem", "3.1.0", true)
        .unwrap());
    let first = sync.sync_now().await.unwrap();
    let second = sync.sync_now().await.unwrap();
    assert_eq!(second.pulled, 0);
    assert_eq!(second.pushed, 0);
    assert_eq!(first.remote, second.remote);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_failure_keeps_pending_and_retries() {
    let dir = TempDir::new("failure");
    let mock = MockSync::start("user-8").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-h1").await;
    assert!(sync
        .record_local_change("acme.retry", "1.0.0", true)
        .unwrap());

    reqwest::Client::new()
        .post(format!("{}/test/fail-sync", mock.origin))
        .send()
        .await
        .unwrap();
    let error = sync.sync_now().await.unwrap_err();
    assert!(matches!(error, SyncCoordError::Engine(_)), "{error:?}");
    assert_eq!(sync.status().await.pending, 1);
    assert!(sync.status().await.last_error.is_some());

    reqwest::Client::new()
        .post(format!("{}/test/recover", mock.origin))
        .send()
        .await
        .unwrap();
    let outcome = sync.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    assert!(sync.status().await.last_error.is_none());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn account_switch_starts_fresh_and_isolates() {
    let dir = TempDir::new("switch");
    let alice = MockSync::start("alice").await;
    let shared = dir.join("shared-sync");
    let auth_a = CloudAuth::new(
        Some(alice.origin.clone()),
        dir.join("a-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_a, "code-i1").await;
    let sync_a = SyncCoordinator::new(auth_a.clone(), &shared).unwrap();
    sync_a.set_enabled(true).unwrap();
    assert!(sync_a
        .record_local_change("acme.alice", "1.0.0", true)
        .unwrap());
    sync_a.sync_now().await.unwrap();
    alice.stop();

    // Same device directory, different account: the stale file is replaced,
    // never merged, and the new account syncs cleanly.
    let bob = MockSync::start("bob").await;
    let auth_b = CloudAuth::new(
        Some(bob.origin.clone()),
        dir.join("b-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_b, "code-i2").await;
    let sync_b = SyncCoordinator::new(auth_b.clone(), &shared).unwrap();
    assert!(sync_b.is_enabled());
    let outcome = sync_b.sync_now().await.unwrap();
    assert!(outcome.remote.is_empty());
    assert!(sync_b
        .record_local_change("acme.bob", "1.0.0", true)
        .unwrap());
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    bob.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_and_outcomes_carry_no_secrets() {
    let dir = TempDir::new("secrets");
    let mock = MockSync::start("user-9").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-j1").await;
    assert!(sync
        .record_local_change("acme.clean", "1.0.0", true)
        .unwrap());
    let outcome = sync.sync_now().await.unwrap();
    let status = sync.status().await;
    // The coordinator only ever handles selections and status: assert the
    // exact serialized surface carries no credential material.
    let status_text = serde_json::to_value(&status)
        .unwrap()
        .to_string()
        .to_lowercase();
    assert!(!status_text.contains("token"), "{status_text}");
    for remote in &outcome.remote {
        assert!(!remote.key.to_lowercase().contains("token"));
        if let Some(selection) = &remote.selection {
            let text = serde_json::to_value(selection)
                .unwrap()
                .to_string()
                .to_lowercase();
            assert!(!text.contains("token"), "{text}");
        }
    }
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_now_restores_stored_session() {
    let dir = TempDir::new("restore-sync");
    let mock = MockSync::start("user-11").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let sidecar = dir.join("shared-account.json");

    // First "process": log in, then drop every in-memory handle.
    let auth_a = CloudAuth::new(Some(mock.origin.clone()), sidecar.clone(), store.clone());
    login(&auth_a, "code-r1").await;
    drop(auth_a);

    // Second "process": new handles over the same sidecar and keychain.
    // Staging works without a live session; sync reconnects on demand.
    let auth_b = CloudAuth::new(Some(mock.origin.clone()), sidecar, store);
    assert!(auth_b.session().await.is_none());
    let sync_b = SyncCoordinator::new(auth_b.clone(), dir.join("b-sync")).unwrap();
    sync_b.set_enabled(true).unwrap();
    assert!(sync_b
        .record_local_change("acme.restored", "1.0.0", true)
        .unwrap());
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    assert!(auth_b.session().await.is_some());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disabled_sync_stages_and_syncs_nothing() {
    let dir = TempDir::new("optout");
    let mock = MockSync::start("user-10").await;
    let auth = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join("a-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth, "code-k1").await;
    let sync = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    assert!(!sync.is_enabled());
    assert!(!sync.record_local_change("acme.off", "1.0.0", true).unwrap());
    assert!(matches!(
        sync.sync_now().await.unwrap_err(),
        SyncCoordError::Disabled
    ));
    sync.set_enabled(true).unwrap();
    assert!(sync.record_local_change("acme.off", "1.0.0", true).unwrap());
    sync.set_enabled(false).unwrap();
    assert!(matches!(
        sync.sync_now().await.unwrap_err(),
        SyncCoordError::Disabled
    ));
    // The staged edit waits; re-enabling resumes without data loss.
    sync.set_enabled(true).unwrap();
    let outcome = sync.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_sync_releases_the_running_flag() {
    let dir = TempDir::new("cancelled");
    let mock = MockSync::start("user-cancel").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-cancel").await;
    assert!(sync
        .record_local_change("acme.cancel", "1.0.0", true)
        .unwrap());

    // Swap the Cloud for a listener that accepts but never answers, so the
    // run parks on the network while holding the running flag.
    let address = mock.origin.trim_start_matches("http://").to_owned();
    mock.stop();
    // The aborted accept loop releases the port asynchronously.
    let stalled = loop {
        match tokio::net::TcpListener::bind(&address).await {
            Ok(listener) => break listener,
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(10)).await,
        }
    };
    let stall = tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((socket, _)) = stalled.accept().await {
            held.push(socket);
        }
    });

    let mut run = Box::pin(sync.sync_now());
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sync.status().await.syncing {
        assert!(tokio::time::Instant::now() < deadline, "sync never started");
        tokio::select! {
            result = &mut run => panic!("sync finished against a stalled Cloud: {result:?}"),
            () = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
        }
    }
    // Drop the parked run the way the control plane's request timeout does.
    drop(run);

    assert!(!sync.status().await.syncing);
    // A fresh run gets past the guard (and parks again) instead of failing
    // fast with `AlreadyRunning`.
    let retry = tokio::time::timeout(std::time::Duration::from_millis(200), sync.sync_now()).await;
    assert!(retry.is_err(), "retry did not reach the network: {retry:?}");
    stall.abort();
}
