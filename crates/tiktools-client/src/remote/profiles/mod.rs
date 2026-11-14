//! Nonsecret profile files plus an explicitly selected credential backend.
pub(super) mod commands;
mod directory;
pub(super) mod drafts;
mod keychain;
#[cfg(feature = "peer")]
mod peer;
pub(super) mod proposals;
#[cfg(feature = "cloud-peer")]
mod rtc;
mod sealed;
use super::*;
pub use keychain::{CredentialStore, OsCredentialStore};
#[cfg(feature = "peer")]
pub use peer::PeerProfile;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteProfile {
    pub version: u32,
    pub name: String,
    pub profile_id: String,
    pub origin: String,
    pub host_id: String,
    pub principal_id: String,
    pub session_id: String,
    pub client_kind: ClientKind,
    pub trust: TlsTrust,
    pub credential_key: String,
    #[serde(default)]
    pub refresh_attempt: Option<String>,
    #[cfg(feature = "peer")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer: Option<PeerProfile>,
}
impl RemoteProfile {
    fn validate(&self) -> Result<(), ClientError> {
        #[cfg(feature = "peer")]
        let valid_version = self.version == 1 && self.peer.is_none()
            || matches!(self.version, 2 | 3) && self.peer.is_some();
        #[cfg(not(feature = "peer"))]
        let valid_version = self.version == 1;
        if !valid_version
            || !valid_name(&self.name)
            || !valid_id(&self.profile_id)
            || !valid_id(&self.host_id)
            || !valid_id(&self.principal_id)
            || !valid_id(&self.session_id)
            || self.client_kind == ClientKind::Browser
            || super::http::origin(&self.origin)?
                .origin()
                .ascii_serialization()
                != self.origin
            || self.refresh_attempt.as_ref().is_some_and(|v| !valid_id(v))
        {
            return Err(fail("profile_invalid", "remote profile is invalid"));
        }
        self.trust.validate()?;
        #[cfg(feature = "peer")]
        if let Some(peer) = &self.peer {
            peer.validate()?;
            if (self.version == 3) != peer.host_device_id.is_some()
                || self.version == 3
                    && (!peer.remote_address.addrs.is_empty()
                        || !peer.relay_urls.is_empty()
                        || self.refresh_attempt.is_some()
                        || !matches!(self.trust, TlsTrust::System))
            {
                return Err(fail("profile_invalid", "invalid WebRTC profile binding"));
            }
        }
        if self.credential_key != credential_key(self) {
            return Err(fail(
                "profile_invalid",
                "remote profile credential binding changed",
            ));
        }
        Ok(())
    }
}
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn credential_key(profile: &RemoteProfile) -> String {
    tiktools_collaboration::hash_secret(&format!(
        "TikTools remote v1\0{}\0{}\0{}\0{}\0{}",
        profile.profile_id,
        profile.origin,
        profile.host_id,
        profile.principal_id,
        profile.session_id
    ))
}

#[derive(Clone)]
pub struct ProfileManager {
    directory: PathBuf,
    store: Arc<dyn CredentialStore>,
    #[cfg(feature = "peer")]
    peer_endpoints: Arc<tokio::sync::Mutex<peer::EndpointCache>>,
    #[cfg(feature = "cloud-peer")]
    pub(super) cloud_context: Arc<std::sync::Mutex<Option<super::cloud_peer::MaintenanceContext>>>,
}
impl ProfileManager {
    pub async fn check_store(&self) -> Result<(), ClientError> {
        let store = self.store.clone();
        let key = tiktools_collaboration::hash_secret(&uuid::Uuid::new_v4().to_string());
        tokio::task::spawn_blocking(move || store.load(&key).map(|_| ()))
            .await
            .map_err(|_| credential_error())?
    }
    pub fn new(directory: PathBuf, store: Arc<dyn CredentialStore>) -> Result<Self, ClientError> {
        if !directory.is_absolute() {
            return Err(fail(
                "profile_invalid",
                "profile directory must be absolute",
            ));
        }
        Ok(Self {
            directory,
            store,
            #[cfg(feature = "peer")]
            peer_endpoints: Arc::new(tokio::sync::Mutex::new(std::collections::BTreeMap::new())),
            #[cfg(feature = "cloud-peer")]
            cloud_context: Arc::new(std::sync::Mutex::new(None)),
        })
    }
    pub fn system() -> Result<Self, ClientError> {
        Self::new(default_directory()?, Arc::new(OsCredentialStore))
    }
    /// Create the private profile root before another service creates a child
    /// (notably the Cloud account sidecar). Existing unsafe roots fail closed.
    pub fn prepare_directory(&self) -> Result<(), ClientError> {
        directory::check_directory(&self.directory, true)
    }
    pub fn directory(&self) -> &std::path::Path {
        &self.directory
    }
    async fn lease(&self, name: &str, create: bool) -> Result<directory::Lease, ClientError> {
        let directory = self.directory.clone();
        let name = name.to_owned();
        tokio::task::spawn_blocking(move || directory::Lease::open(directory, &name, create))
            .await
            .map_err(|_| fail("profile_unavailable", "profile task failed"))?
    }
    pub async fn list(&self) -> Result<Vec<RemoteProfile>, ClientError> {
        let directory = self.directory.clone();
        tokio::task::spawn_blocking(move || directory::list(&directory))
            .await
            .map_err(|_| fail("profile_unavailable", "profile task failed"))?
    }
    pub async fn get(&self, name: &str) -> Result<RemoteProfile, ClientError> {
        self.lease(name, false)
            .await?
            .profile
            .ok_or_else(|| fail("profile_not_found", "remote profile does not exist"))
    }
    pub async fn ensure_new(&self, name: &str) -> Result<(), ClientError> {
        let lease = self.lease(name, true).await?;
        if lease.profile.is_some() {
            return Err(fail(
                "profile_exists",
                "remote profile already exists; changes must be explicit",
            ));
        }
        Ok(())
    }
    pub async fn install(
        &self,
        name: &str,
        endpoint: &RemoteEndpoint,
        credentials: NativeCredentials,
        kind: ClientKind,
    ) -> Result<RemoteProfile, ClientError> {
        endpoint.verify_host(&credentials).await?;
        let lease = self.lease(name, true).await?;
        if lease.profile.is_some() {
            return Err(fail(
                "profile_exists",
                "remote profile already exists; changes must be explicit",
            ));
        }
        let mut profile = RemoteProfile {
            version: 1,
            name: name.into(),
            profile_id: Uuid::new_v4().to_string(),
            origin: endpoint.origin.clone(),
            host_id: credentials.host_id.clone(),
            principal_id: credentials.principal_id.clone(),
            session_id: credentials.session_id.clone(),
            client_kind: kind,
            trust: endpoint.trust.clone(),
            credential_key: String::new(),
            refresh_attempt: None,
            #[cfg(feature = "peer")]
            peer: None,
        };
        profile.credential_key = credential_key(&profile);
        profile.validate()?;
        let secret = sealed::encode(&profile, &credentials, None)?;
        let store = self.store.clone();
        let key = profile.credential_key.clone();
        tokio::task::spawn_blocking(move || store.save(&key, &secret))
            .await
            .map_err(|_| credential_error())??;
        let saved = profile.clone();
        let stored = tokio::task::spawn_blocking(move || lease.write(&saved))
            .await
            .map_err(|_| profile_error())?;
        if let Err(error) = stored {
            let store = self.store.clone();
            let key = profile.credential_key.clone();
            let _ = tokio::task::spawn_blocking(move || store.delete(&key)).await;
            return Err(error);
        }
        Ok(profile)
    }
    /// The nonsecret durable intent fences ambiguous one-use refreshes. A
    /// saved new keychain generation repairs a crash before clearing intent.
    pub async fn open(&self, name: &str) -> Result<RemoteClient, ClientError> {
        #[cfg(feature = "cloud-peer")]
        {
            let lease = self.lease(name, false).await?;
            if lease
                .profile
                .as_ref()
                .is_some_and(|profile| profile.peer.is_some())
            {
                // No bearer rotation exists here: cancellation should drop the
                // live WebRTC negotiation, not leave a background connection.
                return self.open_rtc_leased(lease).await;
            }
        }
        let manager = self.clone();
        let name = name.to_owned();
        // A cancelled caller must not abandon a known successful refresh
        // between the HTTP reply and its keychain/profile writes.
        tokio::spawn(async move { manager.open_inner(&name).await })
            .await
            .map_err(|_| profile_error())?
    }
    async fn open_inner(&self, name: &str) -> Result<RemoteClient, ClientError> {
        let lease = self.lease(name, false).await?;
        let mut profile = lease
            .profile
            .clone()
            .ok_or_else(|| fail("profile_not_found", "remote profile does not exist"))?;
        #[cfg(feature = "cloud-peer")]
        if profile.peer.is_some() {
            return self.open_rtc_leased(lease).await;
        }
        #[cfg(all(feature = "peer", not(feature = "cloud-peer")))]
        if profile.peer.is_some() {
            if profile.version != 2 {
                return Err(fail(
                    "cloud_unavailable",
                    "WebRTC profiles require Cloud support",
                ));
            }
            return self.open_peer_profile(&profile).await;
        }
        let endpoint = RemoteEndpoint::new(&profile.origin, profile.trust.clone()).await?;
        if endpoint.info().await?.host_id != profile.host_id {
            return Err(fail(
                "host_changed",
                "remote host identity changed; no credential was loaded",
            ));
        }
        let store = self.store.clone();
        let key = profile.credential_key.clone();
        let secret = tokio::task::spawn_blocking(move || store.load(&key))
            .await
            .map_err(|_| credential_error())??
            .ok_or_else(|| {
                fail(
                    "reauthentication_required",
                    "remote credentials are missing; pair again",
                )
            })?;
        let (mut credentials, marker) = sealed::decode(&profile, &secret)?;
        if let Some(attempt) = &profile.refresh_attempt {
            if marker.as_ref() != Some(attempt) {
                return Err(fail(
                    "reauthentication_required",
                    "an earlier refresh was not confirmed; the old token will not be retried",
                ));
            }
            profile.refresh_attempt = None;
            let p = profile.clone();
            let l = lease.clone_for_write();
            tokio::task::spawn_blocking(move || l.write(&p))
                .await
                .map_err(|_| profile_error())??;
        }
        if credentials.expires_at <= now() {
            return Err(fail(
                "reauthentication_required",
                "remote session expired; pair again",
            ));
        }
        if credentials.access_expires_at <= now() + 30_000 {
            let attempt = Uuid::new_v4().to_string();
            profile.refresh_attempt = Some(attempt.clone());
            let p = profile.clone();
            let l = lease.clone_for_write();
            tokio::task::spawn_blocking(move || l.write(&p))
                .await
                .map_err(|_| profile_error())??;
            credentials = endpoint.rotate_credentials(&credentials).await?;
            let secret = sealed::encode(&profile, &credentials, Some(attempt))?;
            let store = self.store.clone();
            let key = profile.credential_key.clone();
            tokio::task::spawn_blocking(move || store.save(&key, &secret))
                .await
                .map_err(|_| credential_error())??;
            profile.refresh_attempt = None;
            let p = profile.clone();
            let l = lease.clone_for_write();
            tokio::task::spawn_blocking(move || l.write(&p))
                .await
                .map_err(|_| profile_error())??;
        }
        // Keep the process lease until the connection is negotiated, so two
        // callers cannot rotate a credential while this one is being opened.
        let mut client =
            RemoteClient::connect(&endpoint, &credentials, profile.client_kind).await?;
        client.profile_binding = Some((self.clone(), name.to_owned()));
        Ok(client)
    }
    pub async fn forget(&self, name: &str) -> Result<(), ClientError> {
        self.forget_matching(name, None).await
    }

    /// Forget only the exact profile the user reviewed. The comparison and
    /// deletion share the cross-process profile lease, so a replacement under
    /// the same display name cannot be deleted by an old confirmation.
    pub async fn forget_reviewed(
        &self,
        name: &str,
        profile_id: &str,
        origin: &str,
        host_id: &str,
    ) -> Result<(), ClientError> {
        self.forget_matching(name, Some((profile_id, origin, host_id)))
            .await
    }

    async fn forget_matching(
        &self,
        name: &str,
        expected: Option<(&str, &str, &str)>,
    ) -> Result<(), ClientError> {
        let lease = self.lease(name, false).await?;
        let profile = lease
            .profile
            .clone()
            .ok_or_else(|| fail("profile_not_found", "remote profile does not exist"))?;
        if expected.is_some_and(|(id, origin, host)| {
            profile.profile_id != id || profile.origin != origin || profile.host_id != host
        }) {
            return Err(fail("stale_view", "the reviewed remote profile changed"));
        }
        let root = self.directory.clone();
        let p = profile.clone();
        let journal = tokio::task::spawn_blocking(move || commands::Journal::open(&root, &p))
            .await
            .map_err(|_| profile_error())??;
        if journal.view().pending.is_some() {
            return Err(fail(
                "command_pending",
                "review the pending command before forgetting this profile",
            ));
        }
        let store = self.store.clone();
        #[cfg(feature = "peer")]
        let peer_key = profile.peer.as_ref().map(|peer| peer.key_path.clone());
        let key = profile.credential_key;
        #[cfg(feature = "peer")]
        let has_bearer = profile.peer.is_none();
        #[cfg(not(feature = "peer"))]
        let has_bearer = true;
        if has_bearer {
            tokio::task::spawn_blocking(move || store.delete(&key))
                .await
                .map_err(|_| credential_error())??;
        }
        tokio::task::spawn_blocking(move || {
            journal.remove_resolved()?;
            lease.remove()
        })
        .await
        .map_err(|_| profile_error())??;
        #[cfg(feature = "peer")]
        if let Some(key) = peer_key {
            let in_use = self.list().await?.iter().any(|profile| {
                profile
                    .peer
                    .as_ref()
                    .is_some_and(|peer| peer.key_path == key)
            });
            if !in_use {
                self.peer_endpoints.lock().await.remove(&key);
            }
        }
        Ok(())
    }
    pub async fn set_trust(
        &self,
        name: &str,
        trust: TlsTrust,
    ) -> Result<RemoteProfile, ClientError> {
        trust.validate()?;
        let lease = self.lease(name, false).await?;
        let mut profile = lease
            .profile
            .clone()
            .ok_or_else(|| fail("profile_not_found", "remote profile does not exist"))?;
        #[cfg(feature = "peer")]
        if profile.peer.is_some() {
            return Err(fail(
                "invalid_params",
                "peer identity trust does not use HTTPS certificate overrides",
            ));
        }
        let endpoint = RemoteEndpoint::new(&profile.origin, trust.clone()).await?;
        if endpoint.info().await?.host_id != profile.host_id {
            return Err(fail(
                "host_changed",
                "trust update cannot replace the paired host identity",
            ));
        }
        profile.trust = trust;
        let saved = profile.clone();
        tokio::task::spawn_blocking(move || lease.write(&saved))
            .await
            .map_err(|_| profile_error())??;
        Ok(profile)
    }
}
fn credential_error() -> ClientError {
    fail(
        "credential_store_unavailable",
        "operating system credential store is unavailable; no file fallback is used",
    )
}
fn profile_error() -> ClientError {
    fail(
        "profile_unavailable",
        "remote profile storage is unavailable",
    )
}
fn default_directory() -> Result<PathBuf, ClientError> {
    if let Some(path) = std::env::var_os("TIKTOOLS_REMOTE_PROFILES_DIR") {
        let path = PathBuf::from(path);
        return if path.is_absolute() {
            Ok(path)
        } else {
            Err(fail(
                "profile_invalid",
                "TIKTOOLS_REMOTE_PROFILES_DIR must be absolute",
            ))
        };
    }
    if let Some(root) = std::env::var_os("TIKTOOLS_HOME") {
        let root = PathBuf::from(root);
        if root.is_absolute() {
            return Ok(root.join("remote-profiles"));
        }
        return Err(fail(
            "profile_invalid",
            "TIKTOOLS_HOME must be absolute for remote profiles",
        ));
    }
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base =
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")));
    base.filter(|p| p.is_absolute())
        .map(|p| p.join("TikTools/remote-profiles"))
        .ok_or_else(profile_error)
}
#[cfg(test)]
mod tests;
