//! Optional Cloud account for TikTools Desktop.
//!
//! The account is strictly opt-in: without an explicit Cloud origin nothing
//! here runs and every local feature keeps working. Login reuses the pinned
//! `cloud-client` flow (system browser, PKCE, loopback callback, one-time
//! redemption) instead of inventing a second OAuth dance.
//!
//! Token handling rules, enforced by construction:
//!
//! - Session tokens live in the OS credential store ([`OsTokenStore`]) or in
//!   process memory for the current run only. They are never written to
//!   files, logs, SQLite, or the frontend.
//! - Only nonsecret state ([`AccountStatus`]: origin, account id, verified
//!   profile, login phase, persistence health, error text) crosses into
//!   control responses and the UI.
//! - Bearer credentials are used only on native API routes inside
//!   `cloud-client`.

mod state;
mod store;

#[cfg(test)]
mod auth_tests;

pub use state::{AccountState, AccountStatus, Persistence};
#[cfg(feature = "os-keychain")]
pub use store::OsTokenStore;
pub use store::{MemoryTokenStore, TokenStore, TokenStoreError};

use std::{path::PathBuf, sync::Arc, time::Duration};

use cloud_client::{LoginAttempt, LoginResult, Session};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{oneshot, Mutex};

/// Environment override for the Cloud origin (development and staging).
/// Production builds carry no compiled-in origin: none configured means the
/// account section reports unconfigured and login is unavailable.
pub const ENV_CLOUD_ORIGIN: &str = "TIKTOOLS_CLOUD_ORIGIN";

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
    #[error("cloud authentication is not configured: set {ENV_CLOUD_ORIGIN}")]
    Unconfigured,
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
}

/// Cloud account manager. Cheap to clone; state lives behind one async lock.
#[derive(Clone)]
pub struct CloudAuth {
    inner: Arc<Mutex<Inner>>,
    store: Arc<dyn TokenStore>,
    sidecar: PathBuf,
}

impl CloudAuth {
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
            })),
            store,
            sidecar,
        }
    }

    /// Build from the `TIKTOOLS_CLOUD_ORIGIN` environment. Empty or missing
    /// means unconfigured.
    pub fn from_environment(data_dir: PathBuf, store: Arc<dyn TokenStore>) -> Self {
        let origin = std::env::var(ENV_CLOUD_ORIGIN)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        Self::new(origin, data_dir.join("cloud-account.json"), store)
    }

    pub async fn status(&self) -> AccountStatus {
        let inner = self.inner.lock().await;
        let state = if inner.pending.is_some() {
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
            email: inner.profile.as_ref().and_then(|profile| profile.email.clone()),
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
        let origin = {
            let inner = self.inner.lock().await;
            if inner.session.is_some() {
                return Err(AuthError::AlreadySignedIn);
            }
            if inner.pending.is_some() {
                return Err(AuthError::AlreadyInProgress);
            }
            inner.origin.clone().ok_or(AuthError::Unconfigured)?
        };
        let attempt = LoginAttempt::begin(&origin)
            .await
            .map_err(|_| AuthError::Failed("could not start the login callback".to_owned()))?;
        let browser_url = attempt.browser_url().to_string();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        {
            let mut inner = self.inner.lock().await;
            if inner.session.is_some() || inner.pending.is_some() {
                return Err(AuthError::AlreadyInProgress);
            }
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
        let (attempt, cancel_rx, origin) = {
            let mut inner = self.inner.lock().await;
            let mut pending = inner.pending.take().ok_or(AuthError::NoLoginInProgress)?;
            let attempt = pending.attempt.take().ok_or(AuthError::NoLoginInProgress)?;
            let cancel_rx = pending.cancel_rx.take();
            let origin = inner.origin.clone().ok_or(AuthError::Unconfigured)?;
            (attempt, cancel_rx, origin)
        };
        let cancel_rx = cancel_rx.unwrap_or_else(|| oneshot::channel::<()>().1);
        let result = attempt.complete(LOGIN_TIMEOUT, cancel_rx).await;
        {
            let mut inner = self.inner.lock().await;
            inner.cancel_tx = None;
        }
        self.finish_login(origin, result).await
    }

    /// Abort an in-flight login and drop its secrets, including a wait
    /// already blocked inside [`CloudAuth::complete_login`].
    pub async fn cancel_login(&self) -> Result<AccountStatus, AuthError> {
        let mut inner = self.inner.lock().await;
        let had_pending = inner.pending.take().is_some();
        let had_waiter = inner.cancel_tx.take().map(|sender| sender.send(()));
        if !had_pending && had_waiter.is_none() {
            return Err(AuthError::NoLoginInProgress);
        }
        inner.last_error = Some("login cancelled".to_owned());
        drop(inner);
        Ok(self.status().await)
    }

    /// Reconnect from the credential store after a restart. Verifies the
    /// token against `/me`, so expiry surfaces here instead of later.
    /// Refuses when the configured origin changed since sign-in.
    pub async fn restore(&self) -> Result<AccountStatus, AuthError> {
        let sidecar: AccountSidecar = {
            let bytes = std::fs::read(&self.sidecar).map_err(|_| AuthError::Unconfigured)?;
            serde_json::from_slice(&bytes)
                .map_err(|_| AuthError::Failed("saved account is unreadable".to_owned()))?
        };
        {
            let inner = self.inner.lock().await;
            if let Some(origin) = inner.origin.as_deref() {
                if origin != sidecar.origin {
                    return Err(AuthError::Failed(
                        "saved account belongs to a different cloud origin".to_owned(),
                    ));
                }
            }
        }
        let token = self
            .store
            .load(&sidecar.origin, &sidecar.account_id)
            .map_err(|_| AuthError::StorageUnavailable)?
            .ok_or(AuthError::Unconfigured)?;
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
        inner.origin = Some(sidecar.origin);
        inner.session = Some(Arc::new(session));
        inner.profile = profile;
        inner.last_error = None;
        drop(inner);
        Ok(self.status().await)
    }

    /// Revoke on the server, then always wipe local credentials and state.
    /// A network failure leaves revocation unknown but still signs out
    /// locally; the warning is surfaced in the returned status. A fresh
    /// process holds no in-memory session, so logout reconnects from the
    /// store first: otherwise the server session would survive and the
    /// stored credential would be left behind.
    pub async fn logout(&self) -> Result<AccountStatus, AuthError> {
        // Best-effort reconnect; local cleanup below proceeds even when the
        // network or the stored credential is gone.
        let restore_error = if self.inner.lock().await.session.is_none() {
            self.restore().await.err()
        } else {
            None
        };
        let (origin, account_id, session) = {
            let mut inner = self.inner.lock().await;
            let session = inner.session.take();
            inner.profile = None;
            inner.pending = None;
            inner.cancel_tx = None;
            inner.last_error = None;
            let account_id = session
                .as_ref()
                .map(|session| session.account_id().to_owned());
            (inner.origin.clone(), account_id, session)
        };
        let mut warning = None;
        if let Some(session) = &session {
            if let Err(error) = session.logout().await {
                warning = Some(format!(
                    "signed out locally; server revocation is unknown ({error})"
                ));
            }
        }
        if let (Some(origin), Some(account_id)) = (origin, account_id) {
            let _ = self.store.delete(&origin, &account_id);
        }
        if session.is_none() {
            // No live session (restore failed or nothing was stored). A
            // leftover sidecar still names a stored credential: remove it so
            // logout never orphans tokens. A foreign origin's credential is
            // left alone, mirroring `restore`'s origin check.
            if let Ok(bytes) = std::fs::read(&self.sidecar) {
                if let Ok(sidecar) = serde_json::from_slice::<AccountSidecar>(&bytes) {
                    let origin_ok = self
                        .inner
                        .lock()
                        .await
                        .origin
                        .as_deref()
                        .is_none_or(|origin| origin == sidecar.origin);
                    if origin_ok {
                        let _ = self.store.delete(&sidecar.origin, &sidecar.account_id);
                        let _ = std::fs::remove_file(&self.sidecar);
                        if !matches!(
                            restore_error,
                            None | Some(AuthError::Unconfigured) | Some(AuthError::Expired)
                        ) {
                            warning = Some(format!(
                                "signed out locally; server revocation is unknown ({})",
                                restore_error
                                    .as_ref()
                                    .map_or("unreachable cloud", AuthError::code)
                            ));
                        }
                    }
                }
            }
        }
        let _ = std::fs::remove_file(&self.sidecar);
        let mut inner = self.inner.lock().await;
        inner.last_error = warning;
        drop(inner);
        Ok(self.status().await)
    }

    async fn finish_login(
        &self,
        origin: String,
        result: Result<LoginResult, cloud_client::SyncError>,
    ) -> Result<AccountStatus, AuthError> {
        let LoginResult { session, .. } = result.map_err(map_session_error)?;
        let account_id = session.account_id().to_owned();
        if let Err(error) = self.store.store(&origin, &account_id, &session) {
            let mut inner = self.inner.lock().await;
            inner.last_error = Some(store_error_text(&error));
            drop(inner);
            return Err(AuthError::StorageUnavailable);
        }
        let profile = Self::refresh_profile(&session).await;
        let sidecar = AccountSidecar {
            origin: origin.clone(),
            account_id: account_id.clone(),
            email: profile.as_ref().and_then(|profile| profile.email.clone()),
            display_name: profile
                .as_ref()
                .and_then(|profile| profile.display_name.clone()),
        };
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
            let _ = self.store.delete(&origin, &account_id);
            return Err(AuthError::Failed(
                "could not remember the signed-in account".to_owned(),
            ));
        }
        let mut inner = self.inner.lock().await;
        inner.origin = Some(origin);
        inner.session = Some(Arc::new(session));
        inner.profile = profile;
        inner.last_error = None;
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
