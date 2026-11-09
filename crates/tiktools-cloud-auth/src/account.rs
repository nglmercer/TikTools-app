pub use crate::state::{AccountState, AccountStatus, Persistence};
#[cfg(feature = "os-keychain")]
pub use crate::store::OsTokenStore;
pub use crate::store::{MemoryTokenStore, TokenStore, TokenStoreError};

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use cloud_client::{LoginAttempt, LoginResult, Session, SyncError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{oneshot, Mutex};

use crate::resolve_cloud_origin;

/// How long [`CloudAuth::complete_login`] waits for the browser callback.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);

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

fn normalized_origin(raw: &str) -> Option<String> {
    let url = url::Url::parse(raw).ok()?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || url.scheme() == "http" && loopback)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(url.origin().ascii_serialization())
}

/// Cloud account manager. Cheap to clone; state lives behind one async lock.
#[derive(Clone)]
pub struct CloudAuth {
    inner: Arc<Mutex<Inner>>,
    store: Arc<dyn TokenStore>,
    sidecar: PathBuf,
    transitions: Arc<AtomicUsize>,
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
}

impl CloudAuth {
    fn sync_identity_locked(&self, inner: &Inner) -> Option<(String, String)> {
        let configured = normalized_origin(inner.origin.as_deref()?)?;
        if let Some(session) = &inner.session {
            return (session.instance_origin() == configured)
                .then(|| (configured, session.account_id().to_owned()));
        }
        if !inner.offline_identity_allowed {
            return None;
        }
        let (origin, account) = self.last_identity()?;
        (normalized_origin(&origin).as_deref() == Some(configured.as_str()))
            .then_some((configured, account))
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
                last_error: None,
                epoch: 0,
                offline_identity_allowed: true,
            })),
            store,
            sidecar,
            transitions: Arc::new(AtomicUsize::new(0)),
        }
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
        let inner = self.inner.lock().await;
        let state = if login_waiting(&inner) {
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
        if self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        let inner = self.inner.try_lock().map_err(|_| SessionBusy)?;
        if login_waiting(&inner) {
            return Err(SessionBusy);
        }
        let identity = self.sync_identity_locked(&inner);
        if self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        Ok(identity)
    }

    /// Run a bounded synchronous local operation while the account identity
    /// remains locked. The closure must not await or call back into CloudAuth.
    pub fn with_sync_identity<R>(
        &self,
        operation: impl FnOnce(Option<(String, String)>) -> R,
    ) -> Result<R, SessionBusy> {
        if self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        let inner = self.inner.try_lock().map_err(|_| SessionBusy)?;
        if login_waiting(&inner) || self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
        let identity = self.sync_identity_locked(&inner);
        if self.transitions.load(Ordering::SeqCst) != 0 {
            return Err(SessionBusy);
        }
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

    /// Last signed-in `(origin, account_id)` from the nonsecret sidecar, if
    /// any. Needs no live session, so offline staging can target the right
    /// identity; the sync engine still re-validates on use.
    pub fn last_identity(&self) -> Option<(String, String)> {
        std::fs::read(&self.sidecar)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<AccountSidecar>(&bytes).ok())
            .map(|sidecar| (sidecar.origin, sidecar.account_id))
    }

    /// Bind the loopback receiver first, then hand out the URL to open in
    /// the system browser. The attempt (PKCE verifier, state) never leaves
    /// this process. Fails when signed in (explicit logout first) or when a
    /// login is already running.
    pub async fn begin_login(&self) -> Result<LoginBegin, AuthError> {
        let _transition = TransitionGuard::begin(&self.transitions);
        if self.transitions.load(Ordering::SeqCst) != 1 {
            return Err(AuthError::AlreadyInProgress);
        }
        let (origin, epoch) = {
            let inner = self.inner.lock().await;
            if inner.session.is_some() {
                return Err(AuthError::AlreadySignedIn);
            }
            if login_waiting(&inner) {
                return Err(AuthError::AlreadyInProgress);
            }
            (
                inner.origin.clone().ok_or(AuthError::Unconfigured)?,
                inner.epoch,
            )
        };
        let attempt = LoginAttempt::begin(&origin)
            .await
            .map_err(|_| AuthError::Failed("could not start the login callback".to_owned()))?;
        let browser_url = attempt.browser_url().to_string();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        {
            let mut inner = self.inner.lock().await;
            if inner.session.is_some() || login_waiting(&inner) || inner.epoch != epoch {
                return Err(AuthError::AlreadyInProgress);
            }
            inner.epoch = inner
                .epoch
                .checked_add(1)
                .ok_or_else(|| AuthError::Failed("account generation exhausted".into()))?;
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
        let url = {
            let inner = self.inner.lock().await;
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
        let _transition = TransitionGuard::begin(&self.transitions);
        let (attempt, cancel_rx, origin, epoch) = {
            let mut inner = self.inner.lock().await;
            let mut pending = inner.pending.take().ok_or(AuthError::NoLoginInProgress)?;
            let attempt = pending.attempt.take().ok_or(AuthError::NoLoginInProgress)?;
            let cancel_rx = pending.cancel_rx.take();
            let origin = inner.origin.clone().ok_or(AuthError::Unconfigured)?;
            (attempt, cancel_rx, origin, inner.epoch)
        };
        let cancel_rx = cancel_rx.unwrap_or_else(|| oneshot::channel::<()>().1);
        let result = attempt.complete(LOGIN_TIMEOUT, cancel_rx).await;
        let finished = self.finish_login(origin, result, epoch).await;
        let mut inner = self.inner.lock().await;
        if inner.epoch == epoch {
            inner.cancel_tx = None;
        }
        drop(inner);
        finished?;
        Ok(self.status().await)
    }

    /// Abort an in-flight login and drop its secrets, including a wait
    /// already blocked inside [`CloudAuth::complete_login`].
    pub async fn cancel_login(&self) -> Result<AccountStatus, AuthError> {
        let _transition = TransitionGuard::begin(&self.transitions);
        let mut inner = self.inner.lock().await;
        let had_pending = inner.pending.take().is_some();
        let had_waiter = inner.cancel_tx.take().map(|sender| sender.send(()));
        if !had_pending && had_waiter.is_none() {
            return Err(AuthError::NoLoginInProgress);
        }
        inner.epoch = inner
            .epoch
            .checked_add(1)
            .ok_or_else(|| AuthError::Failed("account generation exhausted".into()))?;
        inner.last_error = Some("login cancelled".to_owned());
        drop(inner);
        Ok(self.status().await)
    }

    /// Reconnect from the credential store after a restart. Verifies the
    /// token against `/me`, so expiry surfaces here instead of later.
    /// Refuses when the configured origin changed since sign-in.
    pub async fn restore(&self) -> Result<AccountStatus, AuthError> {
        let _transition = TransitionGuard::begin(&self.transitions);
        if self.transitions.load(Ordering::SeqCst) != 1 {
            return Err(AuthError::AlreadyInProgress);
        }
        self.restore_inner().await
    }

    async fn restore_inner(&self) -> Result<AccountStatus, AuthError> {
        let sidecar: AccountSidecar = {
            let Ok(bytes) = std::fs::read(&self.sidecar) else {
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
        Ok(self.status().await)
    }

    /// Wipe local credentials before attempting best-effort server revocation.
    /// Cancellation of the network operation cannot restore the local session.
    pub async fn logout(&self) -> Result<AccountStatus, AuthError> {
        let _transition = TransitionGuard::begin(&self.transitions);
        let (configured_origin, session) = {
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
            inner.last_error = None;
            (inner.origin.clone(), session)
        };
        let sidecar = std::fs::read(&self.sidecar)
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
        let mut inner = self.inner.lock().await;
        if inner.epoch != epoch || inner.session.is_some() {
            return Err(AuthError::Cancelled);
        }
        if let Some(parent) = self.sidecar.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let temporary = self.sidecar.with_extension("json.tmp");
        if serde_json::to_vec(&sidecar)
            .ok()
            .and_then(|bytes| std::fs::write(&temporary, bytes).ok())
            .and_then(|()| std::fs::rename(&temporary, &self.sidecar).ok())
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
