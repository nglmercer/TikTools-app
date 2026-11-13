pub use crate::state::{AccountState, AccountStatus, Persistence};
#[cfg(feature = "os-keychain")]
pub use crate::store::OsTokenStore;
pub use crate::store::{MemoryTokenStore, TokenStore, TokenStoreError};

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use cloud_client::{LoginAttempt, LoginResult, Session, SyncError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{oneshot, Mutex, OwnedRwLockReadGuard, RwLock};

use crate::resolve_cloud_origin;

/// How long [`CloudAuth::complete_login`] waits for the browser callback.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);
const RESERVATION_TTL: Duration = Duration::from_secs(300);
const MAX_LOGIN_RESERVATIONS: usize = 128;
const MAX_ACCOUNT_SIDECAR_BYTES: u64 = 64 * 1024;

fn read_account_sidecar(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::{Error, ErrorKind, Read};
    let invalid = || {
        Error::new(
            ErrorKind::InvalidData,
            "account sidecar is not a bounded regular file",
        )
    };
    if !std::fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(invalid());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_ACCOUNT_SIDECAR_BYTES {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    file.take(MAX_ACCOUNT_SIDECAR_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ACCOUNT_SIDECAR_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}

// Publish via an exclusive, unpredictable sibling and sync it before
// replacing the sidecar. The callback permits failure testing before rename.
fn write_account_sidecar(
    path: &Path,
    bytes: &[u8],
    before_publish: impl FnOnce() -> std::io::Result<()>,
) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind, Write};
    if bytes.len() as u64 > MAX_ACCOUNT_SIDECAR_BYTES {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "account sidecar is too large",
        ));
    }
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    before_publish()?;
    temporary.persist(path).map_err(|error| error.error)?;
    #[cfg(unix)]
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}

/// Nonsecret sidecar remembering which account was signed in. The token
/// itself stays in the credential store; this file only lets restore find
/// the right keychain entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccountSidecar {
    origin: String,
    account_id: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
}

/// Verified, nonsecret account profile from `/me`. Cached in memory after
/// sign-in and in the sidecar across restarts; refreshed best-effort, so a
/// signed-in status without a profile is still valid.
#[derive(Debug, Clone, Default)]
struct AccountProfile {
    email: Option<String>,
    display_name: Option<String>,
}

/// Opaque same-process bearer token. The inner secret is exposed only as a
/// transient borrow for building an `Authorization` header; it is never
/// persisted, logged, or serialized.
#[derive(Debug, Clone)]
pub struct NativeToken(String);

impl NativeToken {
    /// Transient borrow for one request. Callers must mark the header
    /// sensitive and never store the result.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("cloud is disabled for this installation")]
    Unconfigured,
    #[error("no saved cloud session")]
    NoSavedSession,
    #[error("a login is already in progress")]
    AlreadyInProgress,
    #[error("no login is in progress")]
    NoLoginInProgress,
    #[error("already signed in; log out before switching accounts")]
    AlreadySignedIn,
    #[error("cloud login was cancelled or expired")]
    Cancelled,
    #[error("cloud login failed: {0}")]
    Failed(String),
    #[error("cloud session expired; sign in again")]
    Expired,
    #[error("the operating system credential store is unavailable")]
    StorageUnavailable,
}

impl AuthError {
    /// Short stable code for control responses; never carries secrets.
    pub fn code(&self) -> &'static str {
        match self {
            AuthError::Unconfigured => "unconfigured",
            AuthError::NoSavedSession => "no-saved-session",
            AuthError::AlreadyInProgress => "already-in-progress",
            AuthError::NoLoginInProgress => "no-login-in-progress",
            AuthError::AlreadySignedIn => "already-signed-in",
            AuthError::Cancelled => "cancelled",
            AuthError::Failed(_) => "failed",
            AuthError::Expired => "expired",
            AuthError::StorageUnavailable => "storage-unavailable",
        }
    }
}

struct PendingLogin {
    attempt: Option<LoginAttempt>,
    browser_url: String,
    cancel_rx: Option<oneshot::Receiver<()>>,
}

struct Inner {
    origin: Option<String>,
    session: Option<Arc<Session>>,
    profile: Option<AccountProfile>,
    pending: Option<PendingLogin>,
    cancel_tx: Option<oneshot::Sender<()>>,
    active_login_reservation: Option<String>,
    active_login_legacy: bool,
    login_reservations: HashMap<String, Instant>,
    last_error: Option<String>,
    epoch: u64,
    offline_identity_allowed: bool,
}

fn login_waiting(inner: &Inner) -> bool {
    inner.pending.is_some()
        || inner
            .cancel_tx
            .as_ref()
            .is_some_and(|sender| !sender.is_closed())
}

// Called only after a transition has established exclusive admission. When a
// complete future is aborted, its receiver closes without executing its
// normal cleanup. A live completion still owns a transition and cannot race
// this cleanup, including while it persists credentials after the callback.
fn cleanup_abandoned_login(inner: &mut Inner) {
    if inner.pending.is_none()
        && inner
            .cancel_tx
            .as_ref()
            .is_some_and(|sender| sender.is_closed())
    {
        inner.cancel_tx = None;
        inner.active_login_reservation = None;
        inner.active_login_legacy = false;
    }
}

fn check_login_reservation(inner: &Inner, operation: Option<&str>) -> Result<(), AuthError> {
    match (inner.active_login_reservation.as_deref(), operation) {
        (None, None) => Ok(()),
        (Some(current), Some(requested)) if current == requested => Ok(()),
        (Some(_), None) if inner.active_login_legacy => Ok(()),
        (_, Some(_)) => Err(AuthError::Cancelled),
        (Some(_), None) => Err(AuthError::AlreadyInProgress),
    }
}

fn fresh_login_id(inner: &Inner) -> Result<String, AuthError> {
    for _ in 0..4 {
        let mut random = [0u8; 16];
        getrandom::fill(&mut random)
            .map_err(|_| AuthError::Failed("could not create login reservation".into()))?;
        let id: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        if !inner.login_reservations.contains_key(&id)
            && inner.active_login_reservation.as_deref() != Some(&id)
        {
            return Ok(id);
        }
    }
    Err(AuthError::Failed(
        "could not create login reservation".into(),
    ))
}

use crate::origin::normalize_cloud_origin as normalized_origin;

/// Cloud account manager. Cheap to clone; state lives behind one async lock.
#[derive(Clone)]
pub struct CloudAuth {
    inner: Arc<Mutex<Inner>>,
    /// An async gate independent of the account mutex. Account transitions
    /// wait asynchronously while a bounded native commit owns a read lease.
    identity_gate: Arc<RwLock<()>>,
    store: Arc<dyn TokenStore>,
    sidecar: PathBuf,
    transitions: Arc<AtomicUsize>,
    context_observer: Option<Arc<dyn Fn() + Send + Sync>>,
}

/// A read admission for one captured live session, held across an async
/// operation. The session and the gate remain alive until the permit drops.
/// This is not an HTTP cancellation or server-side completion guarantee.
#[must_use = "dropping the permit ends the session admission"]
pub struct SessionEmissionPermit {
    _gate: OwnedRwLockReadGuard<()>,
    _session: Arc<Session>,
}

/// Optional advisory notifications around identity transitions. The observer
/// receives no credentials and cannot veto authentication.
struct ContextInvalidation(Option<Arc<dyn Fn() + Send + Sync>>);
impl ContextInvalidation {
    fn notify(&self) {
        if let Some(observer) = &self.0 {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| observer()));
        }
    }
}
impl Drop for ContextInvalidation {
    fn drop(&mut self) {
        self.notify();
    }
}

struct TransitionGuard(Arc<AtomicUsize>);
impl TransitionGuard {
    fn begin(transitions: &Arc<AtomicUsize>) -> Self {
        transitions.fetch_add(1, Ordering::SeqCst);
        Self(transitions.clone())
    }
}
impl Drop for TransitionGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

struct PendingCredentialGuard {
    store: Arc<dyn TokenStore>,
    origin: String,
    account: String,
    armed: bool,
}
impl Drop for PendingCredentialGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.store.delete(&self.origin, &self.account);
        }
    }
}

/// The in-memory account is being updated; do not fall back to a potentially
/// stale persisted identity while this is returned.
#[derive(Debug, Error)]
#[error("cloud account state is busy")]
pub struct SessionBusy;

#[cfg(test)]
mod session_snapshot_tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn sidecar_publish_ignores_old_fixed_temp_symlink_and_preserves_old_file_on_failure() {
        use std::os::unix::fs::symlink;
        let dir = std::env::temp_dir().join(format!(
            "tiktools-auth-publish-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("account.json");
        let victim = dir.join("victim");
        std::fs::write(&path, b"old sidecar").unwrap();
        std::fs::write(&victim, b"untouched").unwrap();
        symlink(&victim, path.with_extension("json.tmp")).unwrap();
        assert!(
            write_account_sidecar(&path, b"new sidecar", || Err(std::io::Error::other(
                "injected failure"
            )))
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"old sidecar");
        assert_eq!(std::fs::read(&victim).unwrap(), b"untouched");
        write_account_sidecar(&path, b"new sidecar", || Ok(())).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new sidecar");
        assert_eq!(std::fs::read(&victim).unwrap(), b"untouched");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unsafe_or_oversized_sidecar_cannot_block_offline_identity_or_logout() {
        use std::os::unix::{ffi::OsStrExt, fs::symlink};
        let dir = std::env::temp_dir().join(format!(
            "tiktools-auth-sidecar-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("account.json");
        let target = dir.join("target.json");
        std::fs::write(
            &target,
            br#"{"origin":"http://127.0.0.1:8471","account_id":"alice"}"#,
        )
        .unwrap();
        let auth = CloudAuth::new(
            Some("http://127.0.0.1:8471".into()),
            path.clone(),
            Arc::new(MemoryTokenStore::new()),
        );
        symlink(&target, &path).unwrap();
        assert!(auth.try_sync_identity().unwrap().is_none());
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, vec![b'x'; MAX_ACCOUNT_SIDECAR_BYTES as usize + 1]).unwrap();
        assert!(auth.try_sync_identity().unwrap().is_none());
        std::fs::remove_file(&path).unwrap();
        let cpath = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
        let worker_auth = auth.clone();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let identity = worker_auth.with_sync_identity_permit(|identity| identity);
            let _ = done_tx.send(identity);
        });
        assert!(done_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap()
            .is_none());
        tokio::time::timeout(Duration::from_secs(2), auth.logout())
            .await
            .unwrap()
            .unwrap();
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remembered_identity_permit_releases_mutex_and_fences_logout() {
        let path = std::env::temp_dir().join(format!(
            "tiktools-auth-identity-permit-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            br#"{"origin":"http://127.0.0.1:8471","account_id":"alice"}"#,
        )
        .unwrap();
        let auth = CloudAuth::new(
            Some("http://127.0.0.1:8471".into()),
            path.clone(),
            Arc::new(MemoryTokenStore::new()),
        );
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker_auth = auth.clone();
        let worker = std::thread::spawn(move || {
            worker_auth
                .with_sync_identity_permit(|identity| {
                    assert_eq!(
                        identity,
                        Some(("http://127.0.0.1:8471".into(), "alice".into()))
                    );
                    assert!(
                        worker_auth.inner.try_lock().is_ok(),
                        "account mutex held during callback"
                    );
                    assert!(worker_auth.identity_gate.try_write().is_err());
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                })
                .unwrap();
        });
        entered_rx.await.unwrap();
        let logout_auth = auth.clone();
        let mut logout = tokio::spawn(async move { logout_auth.logout().await });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), &mut logout)
                .await
                .is_err()
        );
        assert!(auth.with_sync_identity_permit(|_| ()).is_err());
        release_tx.send(()).unwrap();
        worker.join().unwrap();
        logout.await.unwrap().unwrap();
        assert!(matches!(
            auth.with_sync_identity_permit(|identity| identity),
            Ok(None)
        ));
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn contended_account_is_distinct_from_signed_out() {
        let auth = CloudAuth::new(
            None,
            PathBuf::from("unused-test-sidecar"),
            Arc::new(MemoryTokenStore::new()),
        );
        assert!(matches!(auth.try_session_snapshot(), Ok(None)));
        let locked = auth.inner.lock().await;
        assert!(matches!(auth.try_session_snapshot(), Err(SessionBusy)));
        drop(locked);
        assert!(matches!(auth.try_session_snapshot(), Ok(None)));
    }

    fn configured() -> CloudAuth {
        CloudAuth::new(
            Some("http://127.0.0.1:8471".into()),
            PathBuf::from("unused-test-sidecar"),
            Arc::new(MemoryTokenStore::new()),
        )
    }

    #[tokio::test]
    async fn local_read_waits_for_mutex_but_rejects_pending_transition_and_same_account_replacement(
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut request = Vec::new();
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let mut chunk = [0; 1024];
                    let read = socket.read(&mut chunk).await.unwrap();
                    assert!(read > 0, "test request ended before its headers");
                    request.extend_from_slice(&chunk[..read]);
                    assert!(
                        request.len() <= 8192,
                        "test request headers exceed the limit"
                    );
                }
                let body = r#"{"id":"alice"}"#;
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let session = Arc::new(
            Session::connect_secret(&origin, "fake-token".to_owned().into())
                .await
                .unwrap(),
        );
        let auth = configured();
        auth.inner.lock().await.session = Some(session.clone());
        let locked = auth.inner.lock().await;
        let read = auth.with_current_session_read(&session, || 7);
        tokio::pin!(read);
        assert!(tokio::time::timeout(Duration::from_millis(10), &mut read)
            .await
            .is_err());
        drop(locked);
        assert_eq!(read.await.unwrap(), 7);
        let pending = TransitionGuard::begin(&auth.transitions);
        assert!(auth
            .with_current_session_read(&session, || panic!("transition exposed cached values"))
            .await
            .is_err());
        drop(pending);
        let replacement = Arc::new(
            Session::connect_secret(&origin, "fake-token".to_owned().into())
                .await
                .unwrap(),
        );
        auth.inner.lock().await.session = Some(replacement.clone());
        assert!(auth
            .with_current_session_read(&session, || panic!("old session exposed cached values"))
            .await
            .is_err());
        assert_eq!(
            auth.with_current_session_read(&replacement, || 9)
                .await
                .unwrap(),
            9
        );
        auth.inner.lock().await.session = None;
        assert!(auth
            .with_current_session_read(&replacement, || panic!(
                "signed-out read exposed cached values"
            ))
            .await
            .is_err());
        server.abort();
    }

    #[tokio::test]
    async fn observer_follows_restore_and_aborted_completion_across_clones() {
        let auth = configured();
        let transitions = auth.transitions.clone();
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let wake = Arc::new(tokio::sync::Notify::new());
        let observer_seen = seen.clone();
        let observer_wake = wake.clone();
        let auth = auth.with_context_observer(Arc::new(move || {
            observer_seen
                .lock()
                .unwrap()
                .push(transitions.load(Ordering::SeqCst));
            observer_wake.notify_one();
        }));
        assert!(matches!(
            auth.clone().restore().await,
            Err(AuthError::NoSavedSession)
        ));
        assert_eq!(*seen.lock().unwrap(), vec![1, 0]);
        seen.lock().unwrap().clear();
        let id = auth.reserve_login().await.unwrap();
        auth.begin_login_for(Some(&id)).await.unwrap();
        let clone = auth.clone();
        let task = tokio::spawn(async move { clone.complete_login_for(Some(&id)).await });
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if !seen.lock().unwrap().is_empty() {
                    break;
                }
                wake.notified().await;
            }
        })
        .await
        .unwrap();
        task.abort();
        let _ = task.await;
        assert_eq!(*seen.lock().unwrap(), vec![1, 0]);
    }

    #[tokio::test]
    async fn login_reservations_are_independent_and_stale_cancel_does_not_stop_newer_login() {
        let auth = configured();
        let first = auth.reserve_login().await.unwrap();
        let second = auth.reserve_login().await.unwrap();
        assert_ne!(first, second);
        auth.cancel_login_for(Some(&first)).await.unwrap();
        assert!(matches!(
            auth.begin_login_for(Some(&first)).await,
            Err(AuthError::Cancelled)
        ));
        auth.begin_login_for(Some(&second)).await.unwrap();
        auth.cancel_login_for(Some(&first)).await.unwrap();
        assert!(matches!(
            auth.complete_login_for(Some(&first)).await,
            Err(AuthError::Cancelled)
        ));
        assert!(matches!(
            auth.cancel_login().await,
            Err(AuthError::AlreadyInProgress)
        ));
        assert!(login_waiting(&*auth.inner.lock().await));
        auth.cancel_login_for(Some(&second)).await.unwrap();
        assert!(!login_waiting(&*auth.inner.lock().await));
    }

    #[tokio::test]
    async fn cancel_during_begin_fences_late_callback_installation() {
        let auth = configured();
        let id = auth.reserve_login().await.unwrap();
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        let other = auth.clone();
        let attempt_id = id.clone();
        let task = tokio::spawn(async move {
            other
                .begin_login_after(Some(&attempt_id), async move {
                    let _ = entered_tx.send(());
                    let _ = release_rx.await;
                })
                .await
        });
        entered_rx.await.unwrap();
        auth.cancel_login_for(Some(&id)).await.unwrap();
        release_tx.send(()).unwrap();
        assert!(matches!(task.await.unwrap(), Err(AuthError::Cancelled)));
        assert!(!login_waiting(&*auth.inner.lock().await));
        auth.begin_login().await.unwrap();
        auth.cancel_login().await.unwrap();
    }

    #[tokio::test]
    async fn aborted_begin_does_not_block_an_independent_login() {
        let auth = configured();
        let id = auth.reserve_login().await.unwrap();
        let (entered_tx, entered_rx) = oneshot::channel();
        let (_release_tx, release_rx) = oneshot::channel::<()>();
        let other = auth.clone();
        let task = tokio::spawn(async move {
            other
                .begin_login_after(Some(&id), async move {
                    let _ = entered_tx.send(());
                    let _ = release_rx.await;
                })
                .await
        });
        entered_rx.await.unwrap();
        task.abort();
        let _ = task.await;
        let next = auth.reserve_login().await.unwrap();
        auth.begin_login_for(Some(&next)).await.unwrap();
        auth.cancel_login_for(Some(&next)).await.unwrap();
    }

    #[tokio::test]
    async fn aborted_complete_releases_the_active_operation() {
        let auth = configured();
        let id = auth.reserve_login().await.unwrap();
        auth.begin_login_for(Some(&id)).await.unwrap();
        let other = auth.clone();
        let task_id = id.clone();
        let task = tokio::spawn(async move { other.complete_login_for(Some(&task_id)).await });
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if auth.inner.lock().await.pending.is_none() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        task.abort();
        let _ = task.await;
        let status = auth.status().await;
        assert_eq!(status.state, AccountState::SignedOut);
        assert_eq!(status.login_operation_id, None);
        let next = auth.reserve_login().await.unwrap();
        auth.begin_login_for(Some(&next)).await.unwrap();
        auth.cancel_login_for(Some(&next)).await.unwrap();
    }

    #[tokio::test]
    async fn status_keeps_operation_visible_while_completion_persists() {
        let auth = configured();
        let id = auth.reserve_login().await.unwrap();
        auth.begin_login_for(Some(&id)).await.unwrap();
        // Model the part of completion after the callback receiver was
        // consumed but before its transition/persistence has finished.
        let live = TransitionGuard::begin(&auth.transitions);
        {
            let mut inner = auth.inner.lock().await;
            let pending = inner.pending.take().unwrap();
            drop(pending);
        }
        let status = auth.status().await;
        assert_eq!(status.state, AccountState::SigningIn);
        assert_eq!(status.login_operation_id.as_deref(), Some(id.as_str()));
        assert!(matches!(
            auth.reserve_login().await,
            Err(AuthError::AlreadyInProgress)
        ));
        auth.cancel_login_for(Some(&id)).await.unwrap();
        drop(live);
        assert_eq!(auth.status().await.state, AccountState::SignedOut);
    }

    #[tokio::test]
    async fn observed_cancel_fences_a_completion_that_keeps_running() {
        let auth = configured();
        let id = auth.reserve_login().await.unwrap();
        auth.begin_login_for(Some(&id)).await.unwrap();
        let other = auth.clone();
        let complete_id = id.clone();
        let task = tokio::spawn(async move { other.complete_login_for(Some(&complete_id)).await });
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if auth.inner.lock().await.pending.is_none() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let observed = auth.status().await.login_operation_id.unwrap();
        assert_eq!(observed, id);
        auth.cancel_login_for(Some(&observed)).await.unwrap();
        assert!(task.await.unwrap().is_err());
        let next = auth.reserve_login().await.unwrap();
        auth.begin_login_for(Some(&next)).await.unwrap();
        auth.cancel_login_for(Some(&observed)).await.unwrap();
        assert_eq!(
            auth.status().await.login_operation_id.as_deref(),
            Some(next.as_str())
        );
        auth.cancel_login_for(Some(&next)).await.unwrap();
    }

    #[tokio::test]
    async fn observed_legacy_login_id_cannot_cancel_a_later_correlated_attempt() {
        let auth = configured();
        auth.begin_login().await.unwrap();
        let old = auth.status().await.login_operation_id.unwrap();
        auth.cancel_login().await.unwrap();
        let next = auth.reserve_login().await.unwrap();
        auth.begin_login_for(Some(&next)).await.unwrap();
        auth.cancel_login_for(Some(&old)).await.unwrap();
        assert_eq!(
            auth.status().await.login_operation_id.as_deref(),
            Some(next.as_str())
        );
        assert!(matches!(
            auth.cancel_login().await,
            Err(AuthError::AlreadyInProgress)
        ));
        auth.cancel_login_for(Some(&next)).await.unwrap();
    }

    #[tokio::test]
    async fn lost_reservations_are_bounded_and_expire_without_starting_login() {
        let auth = configured();
        let mut first = String::new();
        for index in 0..MAX_LOGIN_RESERVATIONS {
            let id = auth.reserve_login().await.unwrap();
            if index == 0 {
                first = id;
            }
        }
        assert!(matches!(
            auth.reserve_login().await,
            Err(AuthError::Failed(_))
        ));
        assert!(!login_waiting(&*auth.inner.lock().await));
        {
            let mut inner = auth.inner.lock().await;
            for expiry in inner.login_reservations.values_mut() {
                *expiry = Instant::now();
            }
        }
        let next = auth.reserve_login().await.unwrap();
        assert!(matches!(
            auth.begin_login_for(Some(&first)).await,
            Err(AuthError::Cancelled)
        ));
        auth.begin_login_for(Some(&next)).await.unwrap();
        auth.cancel_login_for(Some(&next)).await.unwrap();
    }
}

impl CloudAuth {
    // Caller holds the identity read gate. Snapshot account state briefly,
    // read a remembered sidecar without the account mutex, then revalidate.
    fn try_sync_identity_guarded(&self) -> Result<Option<(String, String)>, SessionBusy> {
        if self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        let inner = self.inner.try_lock().map_err(|_| SessionBusy)?;
        if login_waiting(&inner) || self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        let origin = inner.origin.clone();
        let session = inner.session.clone();
        let offline_allowed = inner.offline_identity_allowed;
        let epoch = inner.epoch;
        drop(inner);
        let identity = origin
            .as_deref()
            .and_then(normalized_origin)
            .and_then(|configured| {
                if let Some(session) = &session {
                    return (session.instance_origin() == configured)
                        .then(|| (configured, session.account_id().to_owned()));
                }
                if !offline_allowed {
                    return None;
                }
                let (saved_origin, account) = self.last_identity()?;
                (normalized_origin(&saved_origin).as_deref() == Some(configured.as_str()))
                    .then_some((configured, account))
            });
        let inner = self.inner.try_lock().map_err(|_| SessionBusy)?;
        let same_session = match (&session, &inner.session) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        if inner.epoch != epoch
            || inner.origin != origin
            || inner.offline_identity_allowed != offline_allowed
            || !same_session
            || login_waiting(&inner)
            || self.transitions.load(Ordering::SeqCst) != 0
        {
            return Err(SessionBusy);
        }
        Ok(identity)
    }
    /// Build from an explicit origin and a token store. `None` disables
    /// login and reports unconfigured.
    pub fn new(origin: Option<String>, sidecar: PathBuf, store: Arc<dyn TokenStore>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                origin,
                session: None,
                profile: None,
                pending: None,
                cancel_tx: None,
                active_login_reservation: None,
                active_login_legacy: false,
                login_reservations: HashMap::new(),
                last_error: None,
                epoch: 0,
                offline_identity_allowed: true,
            })),
            identity_gate: Arc::new(RwLock::new(())),
            store,
            sidecar,
            transitions: Arc::new(AtomicUsize::new(0)),
            context_observer: None,
        }
    }

    /// Install an advisory, nonsecret identity-change notification before
    /// cloning this manager. It provides no cross-process or delivery fence.
    pub fn with_context_observer(mut self, observer: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.context_observer = Some(observer);
        self
    }

    fn context_invalidation(&self) -> ContextInvalidation {
        ContextInvalidation(self.context_observer.clone())
    }

    /// Build from the resolved Cloud origin ([`resolve_cloud_origin`]).
    /// Only an explicit opt-out reports unconfigured.
    pub fn from_environment(data_dir: PathBuf, store: Arc<dyn TokenStore>) -> Self {
        Self::new(
            resolve_cloud_origin(),
            data_dir.join("cloud-account.json"),
            store,
        )
    }

    pub async fn status(&self) -> AccountStatus {
        let mut inner = self.inner.lock().await;
        // While the lock is held a new completion cannot consume pending. A
        // live completion has already raised the transition count; a closed
        // receiver with no transition is therefore an abandoned attempt.
        if self.transitions.load(Ordering::SeqCst) == 0 {
            cleanup_abandoned_login(&mut inner);
        }
        let state = if login_waiting(&inner) || inner.active_login_reservation.is_some() {
            AccountState::SigningIn
        } else if inner.session.is_some() {
            AccountState::SignedIn
        } else if inner.origin.is_some() {
            AccountState::SignedOut
        } else {
            AccountState::Unconfigured
        };
        AccountStatus {
            origin: inner.origin.clone(),
            state,
            login_operation_id: if state == AccountState::SigningIn {
                inner.active_login_reservation.clone()
            } else {
                None
            },
            account_id: inner
                .session
                .as_ref()
                .map(|session| session.account_id().to_owned()),
            email: inner
                .profile
                .as_ref()
                .and_then(|profile| profile.email.clone()),
            display_name: inner
                .profile
                .as_ref()
                .and_then(|profile| profile.display_name.clone()),
            persistence: self.store.persistence(),
            error: inner.last_error.clone(),
        }
    }

    /// Best-effort profile refresh from the bounded `/me/export` snapshot.
    /// Never fails sign-in: a missing or unreadable profile leaves the
    /// cached (or absent) profile untouched.
    async fn refresh_profile(session: &Session) -> Option<AccountProfile> {
        let export = session.export_account().await.ok()?;
        Some(AccountProfile {
            email: Some(export.profile.email),
            display_name: export.profile.name,
        })
    }

    /// The live native session, if signed in. Shares ownership so sync and
    /// other native callers reuse one session without re-authenticating;
    /// the token itself never leaves the [`Session`] handle.
    pub async fn session(&self) -> Option<Arc<Session>> {
        self.inner.lock().await.session.clone()
    }

    /// Bearer [REDACTED] for same-process native Cloud API calls only (e.g. the
    /// publisher flow). The secret never leaves process memory: callers must
    /// not persist, log, or forward it, and must mark auth headers sensitive.
    pub async fn native_token(&self) -> Option<NativeToken> {
        use secrecy::ExposeSecret;
        let inner = self.inner.lock().await;
        let token = inner.session.as_ref()?.token_for_keychain();
        Some(NativeToken(token.expose_secret().to_owned()))
    }

    /// Non-blocking share of the live session, if signed in. Lock holds are
    /// microsecond-scale (never across an await), so a missed lock just
    /// means another task is mid-update; callers treat that as absent.
    pub fn try_session(&self) -> Option<Arc<Session>> {
        self.inner.try_lock().ok()?.session.clone()
    }

    /// Non-blocking snapshot that distinguishes a signed-out account from a
    /// temporarily contended account update.
    pub fn try_session_snapshot(&self) -> Result<Option<Arc<Session>>, SessionBusy> {
        if self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        let inner = self.inner.try_lock().map_err(|_| SessionBusy)?;
        if login_waiting(&inner) || self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        Ok(inner.session.clone())
    }

    /// Capture a live or remembered identity without treating a running login,
    /// restore or logout as a signed-out offline state.
    pub fn try_sync_identity(&self) -> Result<Option<(String, String)>, SessionBusy> {
        let _gate = self.identity_gate.try_read().map_err(|_| SessionBusy)?;
        self.try_sync_identity_guarded()
    }

    /// Run a bounded synchronous local operation while the account identity
    /// remains locked. The closure must not await or call back into CloudAuth.
    pub fn with_sync_identity<R>(
        &self,
        operation: impl FnOnce(Option<(String, String)>) -> R,
    ) -> Result<R, SessionBusy> {
        let _gate = self.identity_gate.try_read().map_err(|_| SessionBusy)?;
        let identity = self.try_sync_identity_guarded()?;
        let inner = self.inner.try_lock().map_err(|_| SessionBusy)?;
        if login_waiting(&inner) || self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        let result = operation(identity);
        drop(inner);
        Ok(result)
    }

    /// Hold a nonblocking identity permit while a synchronous native commit
    /// uses a live or remembered account. The account mutex is released
    /// before the callback; login, restore and logout wait on the identity
    /// gate. The callback must not await, do network I/O or synchronously
    /// wait for an account transition. Run disk work on a blocking thread.
    /// This protects only this manager and its clones, not other processes.
    pub fn with_sync_identity_permit<R>(
        &self,
        operation: impl FnOnce(Option<(String, String)>) -> R,
    ) -> Result<R, SessionBusy> {
        let _gate = self.identity_gate.try_read().map_err(|_| SessionBusy)?;
        let identity = self.try_sync_identity_guarded()?;
        Ok(operation(identity))
    }

    /// Execute a bounded synchronous commit only while this exact session is
    /// installed. No network or asynchronous work may occur in the closure.
    pub fn with_current_session<R>(
        &self,
        captured: &Arc<Session>,
        operation: impl FnOnce() -> R,
    ) -> Result<R, SessionBusy> {
        if self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        let inner = self.inner.try_lock().map_err(|_| SessionBusy)?;
        if login_waiting(&inner)
            || !inner
                .session
                .as_ref()
                .is_some_and(|live| Arc::ptr_eq(live, captured))
            || self.transitions.load(Ordering::SeqCst) != 0
        {
            return Err(SessionBusy);
        }
        Ok(operation())
    }

    /// Return an already-read local value while this exact session is still
    /// installed. Unlike write admission, a simultaneous reader briefly
    /// holding the account mutex is awaited rather than reported as a conflict.
    /// The callback must only return in-memory data: no I/O, await or reentry.
    /// Pending transitions fail closed, including replacement by the same account.
    pub async fn with_current_session_read<R>(
        &self,
        captured: &Arc<Session>,
        operation: impl FnOnce() -> R,
    ) -> Result<R, SessionBusy> {
        if self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        let inner = self.inner.lock().await;
        if login_waiting(&inner)
            || !inner
                .session
                .as_ref()
                .is_some_and(|live| Arc::ptr_eq(live, captured))
            || self.transitions.load(Ordering::SeqCst) != 0
        {
            return Err(SessionBusy);
        }
        Ok(operation())
    }

    /// Whether this exact captured session is still installed. Account IDs
    /// alone cannot distinguish logout followed by login to the same account.
    /// A contended lock fails closed; callers must also fence their opt-in
    /// generation and recheck after asynchronous work.
    pub fn is_current_session(&self, captured: &Arc<Session>) -> bool {
        if self.transitions.load(Ordering::SeqCst) != 0 {
            return false;
        }
        self.inner
            .try_lock()
            .ok()
            .and_then(|inner| {
                inner.session.as_ref().map(|live| {
                    Arc::ptr_eq(live, captured)
                        && !login_waiting(&inner)
                        && self.transitions.load(Ordering::SeqCst) == 0
                })
            })
            .unwrap_or(false)
    }

    /// Run a bounded synchronous local commit while this exact session is
    /// stable. The account mutex is used only for the initial comparison and
    /// is released before the callback. The private gate cannot escape the
    /// callback; returning a future does not extend its protection. A pending
    /// transition fails admission, while a later transition waits asynchronously.
    /// The callback must not perform network work or synchronously wait for
    /// a transition of this manager, which needs the exclusive gate. Run
    /// potentially slow disk operations off async executor threads. This fences this manager
    /// and its clones, not separately constructed managers or processes.
    pub fn with_current_session_permit<R>(
        &self,
        captured: &Arc<Session>,
        operation: impl FnOnce() -> R,
    ) -> Result<R, SessionBusy> {
        let _gate = self.identity_gate.try_read().map_err(|_| SessionBusy)?;
        if !self.is_current_session(captured) {
            return Err(SessionBusy);
        }
        Ok(operation())
    }

    /// Attempt a nonblocking admission for an asynchronous operation using
    /// this exact live session. The returned owned permit keeps this manager's
    /// identity gate shared until it is dropped; an account transition that
    /// starts after admission waits for it, while an earlier transition or a
    /// contended lock fails closed. Account ID equality is insufficient:
    /// replacing a session for the same account invalidates the old capture.
    ///
    /// Unlike the bounded synchronous callback above, the caller can hold
    /// this permit across `await`. The caller must enforce its own deadline,
    /// release it on cancellation, and must not wait for a transition of this
    /// manager while holding it. A later transition may make other eligibility
    /// checks fail while it waits. No account mutex is held by the permit.
    /// This coordinates only this manager and its clones, not another manager
    /// or process. Dropping it after cancellation does not prevent a server
    /// from committing a request already received; recovery needs its own
    /// server-side fence. This primitive is not itself permission to sync.
    pub fn try_session_emission_permit(
        &self,
        captured: &Arc<Session>,
    ) -> Result<SessionEmissionPermit, SessionBusy> {
        let gate = self
            .identity_gate
            .clone()
            .try_read_owned()
            .map_err(|_| SessionBusy)?;
        if !self.is_current_session(captured) {
            return Err(SessionBusy);
        }
        Ok(SessionEmissionPermit {
            _gate: gate,
            _session: captured.clone(),
        })
    }

    /// Last signed-in `(origin, account_id)` from the nonsecret sidecar, if
    /// any. Needs no live session, so offline staging can target the right
    /// identity; the sync engine still re-validates on use.
    pub fn last_identity(&self) -> Option<(String, String)> {
        read_account_sidecar(&self.sidecar)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<AccountSidecar>(&bytes).ok())
            .map(|sidecar| (sidecar.origin, sidecar.account_id))
    }

    /// Create an independent, bounded and expiring reservation. It cannot
    /// start a browser callback until a subsequent begin consumes it.
    pub async fn reserve_login(&self) -> Result<String, AuthError> {
        let _transition = TransitionGuard::begin(&self.transitions);
        if self.transitions.load(Ordering::SeqCst) != 1 {
            return Err(AuthError::AlreadyInProgress);
        }
        let _identity = self.identity_gate.write().await;
        let mut inner = self.inner.lock().await;
        if self.transitions.load(Ordering::SeqCst) != 1 {
            return Err(AuthError::AlreadyInProgress);
        }
        cleanup_abandoned_login(&mut inner);
        if inner.session.is_some() {
            return Err(AuthError::AlreadySignedIn);
        }
        if inner.origin.is_none() {
            return Err(AuthError::Unconfigured);
        }
        if login_waiting(&inner) {
            return Err(AuthError::AlreadyInProgress);
        }
        let now = Instant::now();
        inner.login_reservations.retain(|_, expires| *expires > now);
        if inner.login_reservations.len() >= MAX_LOGIN_RESERVATIONS {
            return Err(AuthError::Failed("too many login reservations".into()));
        }
        let id = fresh_login_id(&inner)?;
        inner
            .login_reservations
            .insert(id.clone(), now + RESERVATION_TTL);
        Ok(id)
    }

    pub async fn begin_login(&self) -> Result<LoginBegin, AuthError> {
        self.begin_login_for(None).await
    }

    /// Bind begin to a reservation when provided, so cancellation can fence a
    /// request that has not arrived or whose response was lost.
    pub async fn begin_login_for(&self, operation: Option<&str>) -> Result<LoginBegin, AuthError> {
        self.begin_login_after(operation, std::future::ready(()))
            .await
    }

    async fn begin_login_after<F: std::future::Future<Output = ()>>(
        &self,
        operation: Option<&str>,
        after_reservation: F,
    ) -> Result<LoginBegin, AuthError> {
        let _transition = TransitionGuard::begin(&self.transitions);
        if self.transitions.load(Ordering::SeqCst) != 1 {
            return Err(AuthError::AlreadyInProgress);
        }
        let (origin, epoch) = {
            let _identity = self.identity_gate.write().await;
            let mut inner = self.inner.lock().await;
            if self.transitions.load(Ordering::SeqCst) != 1 {
                return Err(AuthError::AlreadyInProgress);
            }
            cleanup_abandoned_login(&mut inner);
            if inner.session.is_some() {
                return Err(AuthError::AlreadySignedIn);
            }
            if login_waiting(&inner) {
                return Err(AuthError::AlreadyInProgress);
            }
            if inner.active_login_reservation.is_some() {
                return Err(AuthError::AlreadyInProgress);
            }
            if let Some(id) = operation {
                if !inner
                    .login_reservations
                    .get(id)
                    .is_some_and(|expires| *expires > Instant::now())
                {
                    return Err(AuthError::Cancelled);
                }
            }
            (
                inner.origin.clone().ok_or(AuthError::Unconfigured)?,
                inner.epoch,
            )
        };
        after_reservation.await;
        let attempt = LoginAttempt::begin(&origin)
            .await
            .map_err(|_| AuthError::Failed("could not start the login callback".to_owned()))?;
        let browser_url = attempt.browser_url().to_string();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        {
            let _identity = self.identity_gate.write().await;
            let mut inner = self.inner.lock().await;
            if inner.session.is_some() || login_waiting(&inner) || inner.epoch != epoch {
                return Err(AuthError::AlreadyInProgress);
            }
            if inner.active_login_reservation.is_some() {
                return Err(AuthError::AlreadyInProgress);
            }
            let next_epoch = inner
                .epoch
                .checked_add(1)
                .ok_or_else(|| AuthError::Failed("account generation exhausted".into()))?;
            if let Some(id) = operation {
                if !inner
                    .login_reservations
                    .remove(id)
                    .is_some_and(|expires| expires > Instant::now())
                {
                    return Err(AuthError::Cancelled);
                }
                inner.active_login_reservation = Some(id.to_owned());
                inner.active_login_legacy = false;
            } else {
                inner.active_login_reservation = Some(fresh_login_id(&inner)?);
                inner.active_login_legacy = true;
            }
            inner.epoch = next_epoch;
            inner.last_error = None;
            inner.pending = Some(PendingLogin {
                attempt: Some(attempt),
                browser_url: browser_url.clone(),
                cancel_rx: Some(cancel_rx),
            });
            inner.cancel_tx = Some(cancel_tx);
        }
        Ok(LoginBegin { browser_url })
    }

    /// Open the system browser at the pending login URL. Requires the
    /// `system-browser` feature; otherwise callers show the URL from
    /// [`CloudAuth::begin_login`] for manual opening.
    pub async fn open_browser(&self) -> Result<(), AuthError> {
        self.open_browser_for(None).await
    }

    pub async fn open_browser_for(&self, operation: Option<&str>) -> Result<(), AuthError> {
        let url = {
            let inner = self.inner.lock().await;
            check_login_reservation(&inner, operation)?;
            inner
                .pending
                .as_ref()
                .map(|pending| pending.browser_url.clone())
                .ok_or(AuthError::NoLoginInProgress)?
        };
        #[cfg(feature = "system-browser")]
        {
            open::that(&url)
                .map_err(|_| AuthError::Failed("could not open the system browser".to_owned()))
        }
        #[cfg(not(feature = "system-browser"))]
        {
            let _ = url;
            Err(AuthError::Failed(
                "system browser support is not compiled in".to_owned(),
            ))
        }
    }

    /// Wait for the validated browser callback and redeem the one-time code,
    /// then verify the session and persist it. Consumes the pending attempt;
    /// a concurrent [`CloudAuth::cancel_login`] aborts the wait.
    pub async fn complete_login(&self) -> Result<AccountStatus, AuthError> {
        self.complete_login_for(None).await
    }

    pub async fn complete_login_for(
        &self,
        operation: Option<&str>,
    ) -> Result<AccountStatus, AuthError> {
        // Declare the invalidation first so its final notification happens
        // after the transition guard releases, including on future abort.
        let invalidation = self.context_invalidation();
        let _transition = TransitionGuard::begin(&self.transitions);
        invalidation.notify();
        let (attempt, cancel_rx, origin, epoch) = {
            let _identity = self.identity_gate.write().await;
            let mut inner = self.inner.lock().await;
            check_login_reservation(&inner, operation)?;
            let mut pending = inner.pending.take().ok_or(AuthError::NoLoginInProgress)?;
            let attempt = pending.attempt.take().ok_or(AuthError::NoLoginInProgress)?;
            let cancel_rx = pending.cancel_rx.take();
            let origin = inner.origin.clone().ok_or(AuthError::Unconfigured)?;
            (attempt, cancel_rx, origin, inner.epoch)
        };
        let cancel_rx = cancel_rx.unwrap_or_else(|| oneshot::channel::<()>().1);
        let result = attempt.complete(LOGIN_TIMEOUT, cancel_rx).await;
        let finished = self.finish_login(origin, result, epoch).await;
        let identity = self.identity_gate.write().await;
        let mut inner = self.inner.lock().await;
        if inner.epoch == epoch {
            inner.cancel_tx = None;
            inner.active_login_reservation = None;
            inner.active_login_legacy = false;
        }
        drop(inner);
        drop(identity);
        finished?;
        Ok(self.status().await)
    }

    /// Abort an in-flight login and drop its secrets, including a wait
    /// already blocked inside [`CloudAuth::complete_login`].
    pub async fn cancel_login(&self) -> Result<AccountStatus, AuthError> {
        self.cancel_login_for(None).await
    }

    pub async fn cancel_login_for(
        &self,
        operation: Option<&str>,
    ) -> Result<AccountStatus, AuthError> {
        let _transition = TransitionGuard::begin(&self.transitions);
        let identity = self.identity_gate.write().await;
        let mut inner = self.inner.lock().await;
        if operation.is_none()
            && inner.active_login_reservation.is_some()
            && !inner.active_login_legacy
        {
            return Err(AuthError::AlreadyInProgress);
        }
        // A scoped retry is idempotent and never cancels a newer attempt.
        if operation.is_some() && inner.active_login_reservation.as_deref() != operation {
            if let Some(id) = operation {
                inner.login_reservations.remove(id);
            }
            drop(inner);
            drop(identity);
            return Ok(self.status().await);
        }
        let had_reservation = inner.active_login_reservation.take().is_some();
        inner.active_login_legacy = false;
        let had_pending = inner.pending.take().is_some();
        let had_waiter = inner.cancel_tx.take().map(|sender| sender.send(()));
        if !had_reservation && !had_pending && had_waiter.is_none() {
            return Err(AuthError::NoLoginInProgress);
        }
        inner.epoch = inner
            .epoch
            .checked_add(1)
            .ok_or_else(|| AuthError::Failed("account generation exhausted".into()))?;
        inner.last_error = Some("login cancelled".to_owned());
        drop(inner);
        drop(identity);
        Ok(self.status().await)
    }

    /// Reconnect from the credential store after a restart. Verifies the
    /// token against `/me`, so expiry surfaces here instead of later.
    /// Refuses when the configured origin changed since sign-in.
    pub async fn restore(&self) -> Result<AccountStatus, AuthError> {
        let invalidation = self.context_invalidation();
        let _transition = TransitionGuard::begin(&self.transitions);
        invalidation.notify();
        if self.transitions.load(Ordering::SeqCst) != 1 {
            return Err(AuthError::AlreadyInProgress);
        }
        self.restore_inner().await
    }

    async fn restore_inner(&self) -> Result<AccountStatus, AuthError> {
        let sidecar: AccountSidecar = {
            let Ok(bytes) = read_account_sidecar(&self.sidecar) else {
                // Nothing to restore is a normal signed-out start, distinct
                // from a disabled Cloud.
                return Err(if self.inner.lock().await.origin.is_some() {
                    AuthError::NoSavedSession
                } else {
                    AuthError::Unconfigured
                });
            };
            serde_json::from_slice(&bytes)
                .map_err(|_| AuthError::Failed("saved account is unreadable".to_owned()))?
        };
        let epoch = {
            let inner = self.inner.lock().await;
            if login_waiting(&inner) || inner.session.is_some() {
                return Err(AuthError::AlreadyInProgress);
            }
            let configured = inner
                .origin
                .as_deref()
                .and_then(normalized_origin)
                .ok_or(AuthError::Unconfigured)?;
            if normalized_origin(&sidecar.origin).as_deref() != Some(configured.as_str()) {
                return Err(AuthError::Failed(
                    "saved account belongs to a different cloud origin".to_owned(),
                ));
            }
            inner.epoch
        };
        let token = self
            .store
            .load(&sidecar.origin, &sidecar.account_id)
            .map_err(|_| AuthError::StorageUnavailable)?
            .ok_or(AuthError::NoSavedSession)?;
        let session = Session::connect_secret(&sidecar.origin, token)
            .await
            .map_err(map_session_error)?;
        if session.account_id() != sidecar.account_id {
            return Err(AuthError::Failed(
                "saved account does not match the stored credential".to_owned(),
            ));
        }
        // Prefer a fresh profile; fall back to the sidecar cache so an
        // offline restore still shows the last known account details.
        let profile = Self::refresh_profile(&session).await.or_else(|| {
            if sidecar.email.is_none() && sidecar.display_name.is_none() {
                None
            } else {
                Some(AccountProfile {
                    email: sidecar.email.clone(),
                    display_name: sidecar.display_name.clone(),
                })
            }
        });
        let identity = self.identity_gate.write().await;
        let mut inner = self.inner.lock().await;
        if inner.epoch != epoch || login_waiting(&inner) || inner.session.is_some() {
            return Err(AuthError::Cancelled);
        }
        inner.cancel_tx = None;
        inner.origin = Some(sidecar.origin);
        inner.session = Some(Arc::new(session));
        inner.profile = profile;
        inner.offline_identity_allowed = true;
        inner.last_error = None;
        drop(inner);
        drop(identity);
        Ok(self.status().await)
    }

    /// Wipe local credentials before attempting best-effort server revocation.
    /// Cancellation of the network operation cannot restore the local session.
    pub async fn logout(&self) -> Result<AccountStatus, AuthError> {
        let invalidation = self.context_invalidation();
        let _transition = TransitionGuard::begin(&self.transitions);
        invalidation.notify();
        let (configured_origin, session) = {
            let _identity = self.identity_gate.write().await;
            let mut inner = self.inner.lock().await;
            inner.epoch = inner
                .epoch
                .checked_add(1)
                .ok_or_else(|| AuthError::Failed("account generation exhausted".into()))?;
            if let Some(cancel) = inner.cancel_tx.take() {
                let _ = cancel.send(());
            }
            let session = inner.session.take();
            inner.profile = None;
            inner.offline_identity_allowed = false;
            inner.pending = None;
            inner.active_login_reservation = None;
            inner.active_login_legacy = false;
            inner.login_reservations.clear();
            inner.last_error = None;
            (inner.origin.clone(), session)
        };
        let sidecar = read_account_sidecar(&self.sidecar)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<AccountSidecar>(&bytes).ok());
        let remembered = sidecar.filter(|v| {
            configured_origin
                .as_deref()
                .is_none_or(|origin| origin == v.origin)
        });
        let identity = session
            .as_ref()
            .map(|s| (s.instance_origin(), s.account_id().to_owned()))
            .or_else(|| {
                remembered
                    .as_ref()
                    .map(|s| (s.origin.clone(), s.account_id.clone()))
            });
        let mut warning = None;
        let mut local_cleanup_failed = false;
        // Load a revocation credential into memory first, then synchronously
        // remove local persistence before the first cancellable network await.
        let token = if session.is_none() {
            match identity
                .as_ref()
                .map(|(origin, account)| self.store.load(origin, account))
            {
                Some(Ok(token)) => token,
                Some(Err(_)) => {
                    warning = Some("signed out locally; credential store unavailable".into());
                    local_cleanup_failed = true;
                    None
                }
                None => None,
            }
        } else {
            None
        };
        if let Some((origin, account)) = &identity {
            if self.store.delete(origin, account).is_err() {
                warning = Some("signed out locally; credential store unavailable".into());
                local_cleanup_failed = true;
            }
        }
        if let Err(error) = std::fs::remove_file(&self.sidecar) {
            if error.kind() != std::io::ErrorKind::NotFound {
                warning = Some("signed out locally; account state could not be removed".into());
                local_cleanup_failed = true;
            }
        }
        let revocation = if let Some(session) = session {
            Some(session)
        } else if let (Some((origin, account)), Some(token)) = (&identity, token) {
            match Session::connect_secret(origin, token).await {
                Ok(session) if session.account_id() == account => Some(Arc::new(session)),
                Ok(_) => {
                    warning = Some("signed out locally; stored account did not match".into());
                    None
                }
                Err(SyncError::Unauthorized) => None,
                Err(_) => {
                    warning = Some("signed out locally; server revocation is unknown".into());
                    None
                }
            }
        } else {
            None
        };
        if let Some(session) = revocation {
            if let Err(error) = session.logout().await {
                warning = Some(format!(
                    "signed out locally; server revocation is unknown ({error})"
                ));
            }
        }
        let mut inner = self.inner.lock().await;
        inner.last_error = warning;
        drop(inner);
        if local_cleanup_failed {
            return Err(AuthError::StorageUnavailable);
        }
        Ok(self.status().await)
    }

    async fn finish_login(
        &self,
        origin: String,
        result: Result<LoginResult, cloud_client::SyncError>,
        epoch: u64,
    ) -> Result<AccountStatus, AuthError> {
        let LoginResult { session, .. } = result.map_err(map_session_error)?;
        let account_id = session.account_id().to_owned();
        let mut credential = {
            let _identity = self.identity_gate.write().await;
            let mut inner = self.inner.lock().await;
            if inner.epoch != epoch {
                return Err(AuthError::Cancelled);
            }
            if let Err(error) = self.store.store(&origin, &account_id, &session) {
                inner.last_error = Some(store_error_text(&error));
                return Err(AuthError::StorageUnavailable);
            }
            PendingCredentialGuard {
                store: self.store.clone(),
                origin: origin.clone(),
                account: account_id.clone(),
                armed: true,
            }
        };
        let profile = Self::refresh_profile(&session).await;
        let sidecar = AccountSidecar {
            origin: origin.clone(),
            account_id: account_id.clone(),
            email: profile.as_ref().and_then(|profile| profile.email.clone()),
            display_name: profile
                .as_ref()
                .and_then(|profile| profile.display_name.clone()),
        };
        let identity = self.identity_gate.write().await;
        let mut inner = self.inner.lock().await;
        if inner.epoch != epoch || inner.session.is_some() {
            return Err(AuthError::Cancelled);
        }
        if serde_json::to_vec(&sidecar)
            .ok()
            .and_then(|bytes| write_account_sidecar(&self.sidecar, &bytes, || Ok(())).ok())
            .is_none()
        {
            return Err(AuthError::Failed(
                "could not remember the signed-in account".to_owned(),
            ));
        }
        inner.origin = Some(origin);
        inner.session = Some(Arc::new(session));
        inner.profile = profile;
        inner.offline_identity_allowed = true;
        inner.last_error = None;
        credential.armed = false;
        drop(inner);
        drop(identity);
        Ok(self.status().await)
    }
}

fn store_error_text(error: &TokenStoreError) -> String {
    match error {
        TokenStoreError::Unavailable => "credential store unavailable".to_owned(),
        TokenStoreError::WrongIdentity => "credential belongs to a different account".to_owned(),
        TokenStoreError::Configuration => "credential store misconfigured".to_owned(),
    }
}

fn map_session_error(error: cloud_client::SyncError) -> AuthError {
    use cloud_client::SyncError as E;
    match error {
        E::LoginCancelled => AuthError::Cancelled,
        E::Unauthorized => AuthError::Expired,
        E::Network => AuthError::Failed("cloud is unreachable".to_owned()),
        E::Configuration | E::Protocol => AuthError::Failed("cloud login failed".to_owned()),
        other => AuthError::Failed(format!("cloud request failed ({other})")),
    }
}

/// What [`CloudAuth::begin_login`] hands out: only the public browser URL.
/// No secret; safe to display or copy.
#[derive(Debug, Clone)]
pub struct LoginBegin {
    pub browser_url: String,
}
