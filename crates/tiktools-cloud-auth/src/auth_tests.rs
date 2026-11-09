use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    AccountState, AccountStatus, AuthError, CloudAuth, MemoryTokenStore, Persistence, TokenStore,
};
use secrecy::ExposeSecret;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tiktools-auth-{name}-{}-{suffix}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::AcqRel)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn join(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct MockState {
    account_id: String,
    tokens: HashSet<String>,
    used_codes: HashSet<String>,
    logouts: usize,
    expire_tokens: bool,
}

struct MockCloud {
    origin: String,
    task: tokio::task::JoinHandle<()>,
    hold: Arc<HeldRequest>,
}

#[derive(Default)]
struct HeldRequest {
    path: Mutex<Option<String>>,
    observed: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

impl MockCloud {
    async fn start(account_id: &str) -> Self {
        Self::start_with(account_id, false).await
    }

    async fn start_with(account_id: &str, expire_tokens: bool) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(MockState {
            account_id: account_id.to_owned(),
            tokens: HashSet::new(),
            used_codes: HashSet::new(),
            logouts: 0,
            expire_tokens,
        }));
        let hold = Arc::new(HeldRequest::default());
        let task = tokio::spawn({
            let hold = hold.clone();
            async move {
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        break;
                    };
                    let state = state.clone();
                    let hold = hold.clone();
                    tokio::spawn(async move {
                        let mut buffer = vec![0_u8; 16 * 1024];
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
                            .map_or_else(|| (target.as_str(), ""), |(path, query)| (path, query));
                        let headers: HashMap<String, String> = lines
                            .take_while(|line| !line.is_empty())
                            .filter_map(|line| line.split_once(':'))
                            .map(|(key, value)| {
                                (key.trim().to_ascii_lowercase(), value.trim().to_owned())
                            })
                            .collect();
                        let body_start =
                            request.find("\r\n\r\n").map(|index| index + 4).unwrap_or(0);
                        let body = request[body_start.min(request.len())..].to_owned();
                        let (status, payload) =
                            route(&method, path, query, &headers, &body, &state);
                        let should_hold = {
                            let mut wanted = hold.path.lock().unwrap();
                            if wanted.as_deref() == Some(path) {
                                *wanted = None;
                                true
                            } else {
                                false
                            }
                        };
                        if should_hold {
                            hold.observed.notify_one();
                            hold.release.notified().await;
                        }
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
        Self { origin, task, hold }
    }

    fn stop(self) {
        self.task.abort();
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    }
}

fn route(
    method: &str,
    path: &str,
    _query: &str,
    headers: &HashMap<String, String>,
    body: &str,
    state: &Arc<Mutex<MockState>>,
) -> (String, String) {
    let json = |status: u16, value: serde_json::Value| {
        (
            format!("{status} {}", reason(status)),
            serde_json::to_string(&value).unwrap(),
        )
    };
    let empty = |status: u16| (format!("{status} {}", reason(status)), String::new());
    let bearer = headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .to_owned();
    let mut state = state.lock().unwrap();
    match (method, path) {
        ("POST", "/api/v1/native/handoff/redeem") => {
            let value: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
            let code = value
                .get("code")
                .and_then(|code| code.as_str())
                .unwrap_or_default();
            if code.is_empty() || !state.used_codes.insert(code.to_owned()) {
                return json(400, serde_json::json!({"error": "bad or reused code"}));
            }
            // The client serializes PKCE with camelCase keys; accept exactly
            // the real wire contract and nothing looser.
            if value
                .get("codeVerifier")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .is_empty()
            {
                return json(400, serde_json::json!({"error": "missing verifier"}));
            }
            let token = format!("token-{code}");
            state.tokens.insert(token.clone());
            json(
                200,
                serde_json::json!({"sessionToken": token, "expiresAt": 9_999_999_999_u64}),
            )
        }
        ("GET", "/api/v1/native/me") => {
            if state.expire_tokens || !state.tokens.contains(&bearer) {
                return json(401, serde_json::json!({"error": "unauthorized"}));
            }
            json(
                200,
                serde_json::json!({"id": state.account_id, "email": "user@example.com"}),
            )
        }
        ("GET", "/api/v1/native/me/export") => {
            if state.expire_tokens || !state.tokens.contains(&bearer) {
                return json(401, serde_json::json!({"error": "unauthorized"}));
            }
            json(
                200,
                serde_json::json!({
                    "schemaVersion": 1,
                    "profile": {
                        "id": state.account_id,
                        "email": "user@example.com",
                        "name": "Test User",
                    },
                    "cloudRoles": [],
                    "publisherMemberships": [],
                    "publisherKeysRequested": [],
                    "releasesSubmitted": [],
                    "uploadsRequested": [],
                    "reviewsRecorded": [],
                    "publicationRequests": [],
                    "ownershipTransfers": [],
                    "auditEventsByActor": [],
                    "yanksRecorded": [],
                    "advisoriesRecorded": [],
                    "sync": {"schemaVersion": 1, "cursor": 0, "records": []},
                }),
            )
        }
        ("POST", "/api/v1/native/logout") => {
            if !state.tokens.contains(&bearer) {
                return json(401, serde_json::json!({"error": "unauthorized"}));
            }
            state.tokens.remove(&bearer);
            state.logouts += 1;
            empty(204)
        }
        // Test hook: expire every issued token from now on.
        ("POST", "/test/expire") => {
            state.expire_tokens = true;
            empty(200)
        }
        _ => json(404, serde_json::json!({"error": "not found"})),
    }
}

/// Split the loopback redirect and expected state out of the browser URL.
fn callback_parts(browser_url: &str) -> (String, String) {
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
    assert!(!expected.is_empty());
    (redirect, expected)
}

/// Park `complete_login` in the background, mirroring the real flow where the
/// browser callback arrives while the loopback receiver is already listening.
/// Delivering a callback with nobody accepting would block forever: the
/// kernel buffers the request, but the HTTP response is only written once
/// `receive()` accepts the connection. Call this after `begin_login` (and
/// any guard assertions on the pending state) and before `deliver_callback`.
fn spawn_complete(auth: &CloudAuth) -> tokio::task::JoinHandle<Result<AccountStatus, AuthError>> {
    let waiting = auth.clone();
    tokio::spawn(async move { waiting.complete_login().await })
}

/// Drive the loopback callback the way a browser would after sign-in. The
/// receiver must already be listening (see [`begin_with_waiter`]).
async fn deliver_callback(browser_url: &str, code: &str) {
    let (redirect, expected) = callback_parts(browser_url);
    let response = reqwest::get(format!("{redirect}?code={code}&state={expected}"))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

fn manager(dir: &TempDir, origin: Option<String>, store: Arc<dyn TokenStore>) -> CloudAuth {
    CloudAuth::new(origin, dir.join("cloud-account.json"), store)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn login_logout_round_trip() {
    let dir = TempDir::new("round-trip");
    let mock = MockCloud::start("user-1").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let auth = manager(&dir, Some(mock.origin.clone()), store.clone());

    let begun = auth.begin_login().await.unwrap();
    assert!(begun.browser_url.contains("/desktop/login"));
    assert!(auth.begin_login().await.is_err(), "second login must wait");
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "code-1").await;
    let status = waiter.await.unwrap().unwrap();
    assert_eq!(status.state, AccountState::SignedIn);
    assert_eq!(status.account_id.as_deref(), Some("user-1"));
    assert_eq!(status.persistence, Persistence::MemoryOnly);

    // Sign-in resolves the verified profile best-effort from /me/export.
    assert_eq!(status.email.as_deref(), Some("user@example.com"));
    assert_eq!(status.display_name.as_deref(), Some("Test User"));

    // The sidecar remembers the account but never the token.
    let sidecar = fs::read_to_string(dir.join("cloud-account.json")).unwrap();
    assert!(sidecar.contains("user-1"));
    assert!(!sidecar.contains("token-code-1"));

    // Status responses never carry token material.
    let serialized = serde_json::to_value(auth.status().await).unwrap();
    assert!(serialized.get("sessionToken").is_none());
    assert!(!serialized.to_string().contains("token-code-1"));

    let status = auth.logout().await.unwrap();
    assert_eq!(status.state, AccountState::SignedOut);
    assert!(store.load(&mock.origin, "user-1").unwrap().is_none());
    assert!(!dir.join("cloud-account.json").exists());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn captured_session_is_invalid_after_logout_and_same_account_login() {
    let dir = TempDir::new("session-identity");
    let mock = MockCloud::start("same-account").await;
    let auth = manager(
        &dir,
        Some(mock.origin.clone()),
        Arc::new(MemoryTokenStore::new()),
    );
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "first-session").await;
    waiter.await.unwrap().unwrap();
    let first = auth.session().await.unwrap();
    {
        assert!(auth
            .clone()
            .with_current_session_permit(&first, || ())
            .is_ok());
        let independent = manager(
            &dir,
            Some(mock.origin.clone()),
            Arc::new(MemoryTokenStore::new()),
        );
        assert!(independent
            .with_current_session_permit(&first, || ())
            .is_err());
    }
    assert!(auth.is_current_session(&first));
    assert!(auth.clone().is_current_session(&first));
    auth.logout().await.unwrap();
    assert!(!auth.is_current_session(&first));
    assert!(auth.with_current_session_permit(&first, || ()).is_err());
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "second-session").await;
    waiter.await.unwrap().unwrap();
    let second = auth.session().await.unwrap();
    assert_eq!(first.account_id(), second.account_id());
    assert!(!auth.is_current_session(&first));
    assert!(auth.is_current_session(&second));
    assert!(auth.with_current_session_permit(&first, || ()).is_err());
    assert!(auth.with_current_session_permit(&second, || ()).is_ok());
    auth.logout().await.unwrap();
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_permit_defers_logout_without_holding_account_mutex() {
    let dir = TempDir::new("session-permit");
    let mock = MockCloud::start("permit-account").await;
    let auth = manager(
        &dir,
        Some(mock.origin.clone()),
        Arc::new(MemoryTokenStore::new()),
    );
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "permit-session").await;
    waiter.await.unwrap().unwrap();
    let session = auth.session().await.unwrap();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let holding_auth = auth.clone();
    let holding_session = session.clone();
    let worker = tokio::task::spawn_blocking(move || {
        holding_auth
            .with_current_session_permit(&holding_session, || {
                ready_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
            .unwrap();
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), ready_rx)
        .await
        .unwrap()
        .unwrap();
    let logout_auth = auth.clone();
    let logout = tokio::spawn(async move { logout_auth.logout().await });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while auth.try_session_snapshot().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!logout.is_finished());
    // The identity permit does not own the account mutex; unrelated reads
    // stay responsive while logout waits asynchronously for the commit.
    let status = tokio::time::timeout(std::time::Duration::from_secs(1), auth.status())
        .await
        .unwrap();
    assert_eq!(status.state, AccountState::SignedIn);
    assert!(auth.with_current_session_permit(&session, || ()).is_err());
    // Cancellation while queued must release both the pending-writer slot
    // and transition flag without changing the still-current session.
    logout.abort();
    assert!(logout.await.unwrap_err().is_cancelled());
    assert!(auth.with_current_session_permit(&session, || ()).is_ok());
    let logout_auth = auth.clone();
    let logout = tokio::spawn(async move { logout_auth.logout().await });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while auth.try_session_snapshot().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!logout.is_finished());
    release_tx.send(()).unwrap();
    worker.await.unwrap();
    assert_eq!(
        logout.await.unwrap().unwrap().state,
        AccountState::SignedOut
    );
    assert!(auth.with_current_session_permit(&session, || ()).is_err());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn identity_is_unavailable_during_pending_and_completing_login() {
    let dir = TempDir::new("identity-login-transition");
    let mock = MockCloud::start("user-transition").await;
    let auth = manager(
        &dir,
        Some(mock.origin.clone()),
        Arc::new(MemoryTokenStore::new()),
    );
    let _begun = auth.begin_login().await.unwrap();
    assert!(auth.try_sync_identity().is_err());
    let waiter = spawn_complete(&auth);
    tokio::task::yield_now().await;
    assert!(auth.try_sync_identity().is_err());
    auth.cancel_login().await.unwrap();
    assert!(waiter.await.unwrap().is_err());
    assert!(matches!(auth.try_sync_identity(), Ok(None)));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_and_logout_during_redeem_do_not_resurrect_session() {
    for cancel in [true, false] {
        let dir = TempDir::new("held-redeem");
        let mock = MockCloud::start("user-held").await;
        let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
        let auth = manager(&dir, Some(mock.origin.clone()), store.clone());
        let begun = auth.begin_login().await.unwrap();
        *mock.hold.path.lock().unwrap() = Some("/api/v1/native/handoff/redeem".into());
        let waiter = spawn_complete(&auth);
        deliver_callback(
            &begun.browser_url,
            if cancel { "held-cancel" } else { "held-logout" },
        )
        .await;
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            mock.hold.observed.notified(),
        )
        .await
        .unwrap();
        if cancel {
            auth.cancel_login().await.unwrap();
        } else {
            auth.logout().await.unwrap();
        }
        mock.hold.release.notify_one();
        assert!(waiter.await.unwrap().is_err());
        assert!(auth.session().await.is_none());
        assert!(auth.try_sync_identity().unwrap().is_none());
        assert!(store.load(&mock.origin, "user-held").unwrap().is_none());
        assert!(!dir.join("cloud-account.json").exists());
        mock.stop();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_finishing_after_logout_cannot_install_session() {
    let dir = TempDir::new("held-restore");
    let mock = MockCloud::start("user-restored").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let original = manager(&dir, Some(mock.origin.clone()), store.clone());
    let begun = original.begin_login().await.unwrap();
    let waiter = spawn_complete(&original);
    deliver_callback(&begun.browser_url, "restore-race").await;
    waiter.await.unwrap().unwrap();
    let fresh = manager(&dir, Some(mock.origin.clone()), store.clone());
    *mock.hold.path.lock().unwrap() = Some("/api/v1/native/me".into());
    let restoring = {
        let fresh = fresh.clone();
        tokio::spawn(async move { fresh.restore().await })
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        mock.hold.observed.notified(),
    )
    .await
    .unwrap();
    fresh.logout().await.unwrap();
    mock.hold.release.notify_one();
    assert!(restoring.await.unwrap().is_err());
    assert!(fresh.session().await.is_none());
    assert!(fresh.try_sync_identity().unwrap().is_none());
    assert!(store.load(&mock.origin, "user-restored").unwrap().is_none());
    assert!(!dir.join("cloud-account.json").exists());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aborted_logout_does_not_leave_local_credentials_or_identity() {
    let dir = TempDir::new("held-logout");
    let mock = MockCloud::start("user-logout").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let auth = manager(&dir, Some(mock.origin.clone()), store.clone());
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "logout-race").await;
    waiter.await.unwrap().unwrap();
    *mock.hold.path.lock().unwrap() = Some("/api/v1/native/logout".into());
    let logout = {
        let auth = auth.clone();
        tokio::spawn(async move { auth.logout().await })
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        mock.hold.observed.notified(),
    )
    .await
    .unwrap();
    assert!(!dir.join("cloud-account.json").exists());
    assert!(store.load(&mock.origin, "user-logout").unwrap().is_none());
    logout.abort();
    assert!(logout.await.unwrap_err().is_cancelled());
    assert!(auth.session().await.is_none());
    assert!(auth.try_sync_identity().unwrap().is_none());
    mock.hold.release.notify_one();
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aborting_completion_does_not_leave_identity_permanently_busy() {
    let dir = TempDir::new("aborted-login");
    let mock = MockCloud::start("user-aborted").await;
    let auth = manager(
        &dir,
        Some(mock.origin.clone()),
        Arc::new(MemoryTokenStore::new()),
    );
    let begun = auth.begin_login().await.unwrap();
    *mock.hold.path.lock().unwrap() = Some("/api/v1/native/handoff/redeem".into());
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "abort-login").await;
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        mock.hold.observed.notified(),
    )
    .await
    .unwrap();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    assert!(matches!(auth.try_sync_identity(), Ok(None)));
    assert_eq!(auth.status().await.state, AccountState::SignedOut);
    assert!(auth.begin_login().await.is_ok());
    auth.cancel_login().await.unwrap();
    mock.hold.release.notify_one();
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logout_from_fresh_process_revokes_and_cleans_up() {
    let dir = TempDir::new("fresh-logout");
    let mock = MockCloud::start("user-11").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let auth = manager(&dir, Some(mock.origin.clone()), store.clone());

    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "code-11").await;
    let status = waiter.await.unwrap().unwrap();
    assert_eq!(status.state, AccountState::SignedIn);
    let token = store
        .load(&mock.origin, "user-11")
        .unwrap()
        .expect("login stores the token");

    // A fresh process holds no in-memory session; logout must still revoke
    // the server session and remove the stored credential.
    drop(auth);
    let fresh = manager(&dir, Some(mock.origin.clone()), store.clone());
    assert_eq!(fresh.status().await.state, AccountState::SignedOut);
    let status = fresh.logout().await.unwrap();
    assert_eq!(status.state, AccountState::SignedOut);
    assert!(store.load(&mock.origin, "user-11").unwrap().is_none());
    assert!(!dir.join("cloud-account.json").exists());

    // The revoked Bearer [REDACTED] no longer passes /me.
    let me = reqwest::Client::new()
        .get(format!("{}/api/v1/native/me", mock.origin))
        .bearer_auth(token.expose_secret())
        .send()
        .await
        .unwrap();
    assert_eq!(me.status().as_u16(), 401);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_reconnects_after_restart() {
    let dir = TempDir::new("restore");
    let mock = MockCloud::start("user-2").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let auth = manager(&dir, Some(mock.origin.clone()), store.clone());
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "code-2").await;
    waiter.await.unwrap().unwrap();

    // A fresh process with the same store and sidecar reconnects.
    let restarted = manager(&dir, Some(mock.origin.clone()), store);
    let status = restarted.restore().await.unwrap();
    assert_eq!(status.state, AccountState::SignedIn);
    assert_eq!(status.account_id.as_deref(), Some("user-2"));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_sessions_surface_on_restore() {
    let dir = TempDir::new("expired");
    let mock = MockCloud::start("user-3").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let auth = manager(&dir, Some(mock.origin.clone()), store.clone());
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "code-3").await;
    waiter.await.unwrap().unwrap();

    // Expire the token server-side: restore must fail closed with Expired.
    reqwest::Client::new()
        .post(format!("{}/test/expire", mock.origin))
        .send()
        .await
        .unwrap();
    let restarted = manager(&dir, Some(mock.origin.clone()), store);
    let error = restarted.restore().await.unwrap_err();
    assert!(matches!(error, AuthError::Expired), "{error:?}");
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_state_and_replay_fail_closed() {
    let dir = TempDir::new("state");
    let mock = MockCloud::start("user-4").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let auth = manager(&dir, Some(mock.origin.clone()), store);

    // A callback with a forged state is rejected at the loopback boundary
    // (HTTP 400) and the receiver keeps waiting for the real one.
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    let (redirect, _) = callback_parts(&begun.browser_url);
    let forged = reqwest::get(format!("{redirect}?code=code-4&state=forged"))
        .await
        .unwrap();
    assert_eq!(forged.status().as_u16(), 400);
    deliver_callback(&begun.browser_url, "code-4").await;
    waiter.await.unwrap().unwrap();
    auth.logout().await.unwrap();

    // A redeemed code is single-use: replaying it fails redemption.
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "code-5").await;
    waiter.await.unwrap().unwrap();
    auth.logout().await.unwrap();
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "code-5").await;
    let error = waiter.await.unwrap().unwrap_err();
    assert!(matches!(error, AuthError::Failed(_)), "{error:?}");
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_cancellation_aborts_login() {
    let dir = TempDir::new("cancel");
    let mock = MockCloud::start("user-6").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let auth = manager(&dir, Some(mock.origin.clone()), store);

    auth.begin_login().await.unwrap();
    let status = auth.cancel_login().await.unwrap();
    assert_eq!(status.state, AccountState::SignedOut);
    assert!(status.error.is_some());
    assert!(matches!(
        auth.complete_login().await,
        Err(AuthError::NoLoginInProgress)
    ));

    // Cancelling the blocked wait itself unblocks completion. The sleep
    // guarantees the spawned task is parked inside the loopback receiver.
    auth.begin_login().await.unwrap();
    let waiting = auth.clone();
    let completed = tokio::spawn(async move { waiting.complete_login().await });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    auth.cancel_login().await.unwrap();
    let error = completed.await.unwrap().unwrap_err();
    assert!(matches!(error, AuthError::Cancelled), "{error:?}");
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offline_restore_keeps_local_state_intact() {
    let dir = TempDir::new("offline");
    let mock = MockCloud::start("user-7").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let auth = manager(&dir, Some(mock.origin.clone()), store.clone());
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "code-7").await;
    waiter.await.unwrap().unwrap();
    let before = auth.status().await;
    assert!(before.account_id.is_some());
    mock.stop();

    // The origin is now unreachable: restore fails on the network, leaves
    // local state alone, and reports the outage.
    let restarted = manager(&dir, Some("http://127.0.0.1:9".to_owned()), store.clone());
    let error = restarted.restore().await.unwrap_err();
    assert!(matches!(error, AuthError::Failed(_)), "{error:?}");
    let status = restarted.status().await;
    assert_eq!(status.state, AccountState::SignedOut);
    assert!(
        status.error.is_none(),
        "a refused restore records no local state"
    );

    // Same dead origin as the sidecar: the network failure is explicit.
    let restarted = manager(&dir, before.origin.clone(), store);
    let error = restarted.restore().await.unwrap_err();
    assert!(
        matches!(&error, AuthError::Failed(message) if message.contains("unreachable")),
        "{error:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accounts_are_isolated_by_origin() {
    let dir = TempDir::new("isolation");
    let first = MockCloud::start("alice").await;
    let second = MockCloud::start("bob").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());

    let alice = manager(&dir, Some(first.origin.clone()), store.clone());
    let begun = alice.begin_login().await.unwrap();
    let waiter = spawn_complete(&alice);
    deliver_callback(&begun.browser_url, "code-a").await;
    waiter.await.unwrap().unwrap();

    let bob = manager(&dir, Some(second.origin.clone()), store.clone());
    let begun = bob.begin_login().await.unwrap();
    let waiter = spawn_complete(&bob);
    deliver_callback(&begun.browser_url, "code-b").await;
    waiter.await.unwrap().unwrap();

    // Logging out of one account leaves the other untouched.
    alice.logout().await.unwrap();
    assert!(store.load(&first.origin, "alice").unwrap().is_none());
    assert!(store.load(&second.origin, "bob").unwrap().is_some());
    assert_eq!(bob.status().await.state, AccountState::SignedIn);
    first.stop();
    second.stop();
}

struct FailingStore;

impl TokenStore for FailingStore {
    fn persistence(&self) -> Persistence {
        Persistence::MemoryOnly
    }

    fn load(
        &self,
        _origin: &str,
        _account_id: &str,
    ) -> Result<Option<secrecy::SecretString>, crate::TokenStoreError> {
        Err(crate::TokenStoreError::Unavailable)
    }

    fn store(
        &self,
        _origin: &str,
        _account_id: &str,
        _session: &cloud_client::Session,
    ) -> Result<(), crate::TokenStoreError> {
        Err(crate::TokenStoreError::Unavailable)
    }

    fn delete(&self, _origin: &str, _account_id: &str) -> Result<(), crate::TokenStoreError> {
        Err(crate::TokenStoreError::Unavailable)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn storage_failures_never_fall_back_to_files() {
    let dir = TempDir::new("storage");
    let mock = MockCloud::start("user-9").await;
    let store: Arc<dyn TokenStore> = Arc::new(FailingStore);
    let auth = manager(&dir, Some(mock.origin.clone()), store);

    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "code-9").await;
    let error = waiter.await.unwrap().unwrap_err();
    assert!(matches!(error, AuthError::StorageUnavailable), "{error:?}");
    // No session was kept and nothing was written anywhere.
    assert_eq!(auth.status().await.state, AccountState::SignedOut);
    assert!(!dir.join("cloud-account.json").exists());

    // The real OS adapter is at least exercised: it either works or reports
    // unavailable, never a silent file fallback.
    #[cfg(feature = "os-keychain")]
    {
        let os = crate::OsTokenStore::new();
        // Headless CI runners have no Secret Service, so `Unavailable` is a
        // valid answer; any other error means the adapter misbehaved.
        let probed = os.load(&mock.origin, "probe");
        assert!(
            matches!(probed, Ok(_) | Err(crate::TokenStoreError::Unavailable)),
            "keychain probe must answer or report unavailable, got: {probed:?}"
        );
        assert!(!dir.join("cloud-account.json").exists());
    }
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn begin_guards_and_unconfigured() {
    let dir = TempDir::new("guards");
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let auth = manager(&dir, None, store.clone());
    assert_eq!(auth.status().await.state, AccountState::Unconfigured);
    assert!(matches!(
        auth.begin_login().await,
        Err(AuthError::Unconfigured)
    ));
    assert!(matches!(auth.restore().await, Err(AuthError::Unconfigured)));

    let mock = MockCloud::start("user-10").await;
    let auth = manager(&dir, Some(mock.origin.clone()), store);
    // Configured but never signed in: a quiet "nothing to restore".
    assert!(matches!(
        auth.restore().await,
        Err(AuthError::NoSavedSession)
    ));
    auth.begin_login().await.unwrap();
    assert!(matches!(
        auth.begin_login().await,
        Err(AuthError::AlreadyInProgress)
    ));
    auth.cancel_login().await.unwrap();
    let begun = auth.begin_login().await.unwrap();
    let waiter = spawn_complete(&auth);
    deliver_callback(&begun.browser_url, "code-10").await;
    waiter.await.unwrap().unwrap();
    assert!(matches!(
        auth.begin_login().await,
        Err(AuthError::AlreadySignedIn)
    ));
    mock.stop();
}
