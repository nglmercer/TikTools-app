//! Token storage seam. Production uses the OS credential store;
//! tests and degraded runs use memory.

use std::sync::Mutex;

use cloud_client::Session;
use secrecy::SecretString;
use thiserror::Error;

use crate::state::Persistence;

#[derive(Debug, Error)]
pub enum TokenStoreError {
    #[error("credential store unavailable")]
    Unavailable,
    #[error("credential belongs to a different account")]
    WrongIdentity,
    #[error("credential store misconfigured")]
    Configuration,
}

/// Stores session tokens. Implementations must never write tokens to files,
/// logs, or anything but their designated backend. All methods are
/// synchronous; [`OsTokenStore`] documents its blocking contract.
pub trait TokenStore: Send + Sync {
    fn persistence(&self) -> Persistence;
    fn load(&self, origin: &str, account_id: &str)
        -> Result<Option<SecretString>, TokenStoreError>;
    fn store(
        &self,
        origin: &str,
        account_id: &str,
        session: &Session,
    ) -> Result<(), TokenStoreError>;
    fn delete(&self, origin: &str, account_id: &str) -> Result<(), TokenStoreError>;
}

/// OS credential store via the pinned cloud-client adapter. Every operation
/// can block on the OS; call off the UI thread. Only compiled with the
/// `os-keychain` feature; without it there is no file fallback.
#[cfg(feature = "os-keychain")]
pub struct OsTokenStore;

#[cfg(feature = "os-keychain")]
impl OsTokenStore {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "os-keychain")]
impl Default for OsTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "os-keychain")]
fn map_keychain(error: cloud_client::KeychainError) -> TokenStoreError {
    use cloud_client::KeychainError as E;
    match error {
        E::Unavailable => TokenStoreError::Unavailable,
        E::WrongIdentity => TokenStoreError::WrongIdentity,
        E::Configuration => TokenStoreError::Configuration,
    }
}

#[cfg(feature = "os-keychain")]
impl TokenStore for OsTokenStore {
    fn persistence(&self) -> Persistence {
        Persistence::Keychain
    }

    fn load(
        &self,
        origin: &str,
        account_id: &str,
    ) -> Result<Option<SecretString>, TokenStoreError> {
        let keychain = cloud_client::Keychain::new(origin, account_id).map_err(map_keychain)?;
        keychain.load().map_err(map_keychain)
    }

    fn store(
        &self,
        origin: &str,
        account_id: &str,
        session: &Session,
    ) -> Result<(), TokenStoreError> {
        let keychain = cloud_client::Keychain::new(origin, account_id).map_err(map_keychain)?;
        keychain.store(session).map_err(map_keychain)
    }

    fn delete(&self, origin: &str, account_id: &str) -> Result<(), TokenStoreError> {
        let keychain = cloud_client::Keychain::new(origin, account_id).map_err(map_keychain)?;
        keychain.delete().map_err(map_keychain)
    }
}

/// Process-memory store. Nothing survives a restart; used for tests and
/// reported as degraded persistence, never as a silent file fallback.
#[derive(Default)]
pub struct MemoryTokenStore {
    entries: Mutex<Vec<(String, String, SecretString)>>,
}

impl MemoryTokenStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TokenStore for MemoryTokenStore {
    fn persistence(&self) -> Persistence {
        Persistence::MemoryOnly
    }

    fn load(
        &self,
        origin: &str,
        account_id: &str,
    ) -> Result<Option<SecretString>, TokenStoreError> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| TokenStoreError::Unavailable)?;
        Ok(entries
            .iter()
            .find(|(entry_origin, entry_account, _)| {
                entry_origin == origin && entry_account == account_id
            })
            .map(|(_, _, token)| token.clone()))
    }

    fn store(
        &self,
        origin: &str,
        account_id: &str,
        session: &Session,
    ) -> Result<(), TokenStoreError> {
        if session.account_id() != account_id {
            return Err(TokenStoreError::WrongIdentity);
        }
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| TokenStoreError::Unavailable)?;
        entries.retain(|(entry_origin, entry_account, _)| {
            entry_origin != origin || entry_account != account_id
        });
        entries.push((
            origin.to_owned(),
            account_id.to_owned(),
            session.token_for_keychain().clone(),
        ));
        Ok(())
    }

    fn delete(&self, origin: &str, account_id: &str) -> Result<(), TokenStoreError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| TokenStoreError::Unavailable)?;
        entries.retain(|(entry_origin, entry_account, _)| {
            entry_origin != origin || entry_account != account_id
        });
        Ok(())
    }
}
