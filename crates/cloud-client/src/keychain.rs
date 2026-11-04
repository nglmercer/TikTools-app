//! Explicit OS credential storage; never falls back to a file or mock store.
use crate::{parse_origin, Session};
use keyring::Entry;
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use thiserror::Error;

const SERVICE: &str = "TikTools Cloud v1";
#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("invalid account or cloud origin")]
    Configuration,
    #[error("the keychain entry belongs to a different account or origin")]
    WrongIdentity,
    #[error("the operating system credential store is unavailable")]
    Unavailable,
}

/// Blocking operations: call off the UI thread. No secret is kept in this struct.
pub struct Keychain {
    entry: Entry,
    origin: String,
    account_id: String,
}
impl Keychain {
    pub fn new(origin: &str, account_id: &str) -> Result<Self, KeychainError> {
        let url = parse_origin(origin).map_err(|_| KeychainError::Configuration)?;
        if account_id.is_empty() || account_id.len() > 256 {
            return Err(KeychainError::Configuration);
        }
        let origin = url.origin().ascii_serialization();
        let key = identity_key(&origin, account_id);
        // Select the native backend directly; another library cannot replace it
        // through keyring's process-global default builder.
        #[cfg(target_os = "linux")]
        let builder = keyring::secret_service::default_credential_builder();
        #[cfg(target_os = "macos")]
        let builder = keyring::macos::default_credential_builder();
        #[cfg(target_os = "windows")]
        let builder = keyring::windows::default_credential_builder();
        let credential = builder
            .build(None, SERVICE, &key)
            .map_err(|_| KeychainError::Unavailable)?;
        let entry = Entry::new_with_credential(credential);
        Ok(Self {
            entry,
            origin,
            account_id: account_id.to_owned(),
        })
    }
    pub fn store(&self, session: &Session) -> Result<(), KeychainError> {
        if session.origin.origin().ascii_serialization() != self.origin
            || session.account_id != self.account_id
        {
            return Err(KeychainError::WrongIdentity);
        }
        self.entry
            .set_password(session.token.expose_secret())
            .map_err(|_| KeychainError::Unavailable)
    }
    pub fn load(&self) -> Result<Option<SecretString>, KeychainError> {
        match self.entry.get_password() {
            Ok(value) => Ok(Some(SecretString::from(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(KeychainError::Unavailable),
        }
    }
    pub fn delete(&self) -> Result<(), KeychainError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(KeychainError::Unavailable),
        }
    }
}
fn identity_key(origin: &str, account_id: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(origin.as_bytes());
    hash.update([0]);
    hash.update(account_id.as_bytes());
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn namespaces_are_separate() {
        assert_ne!(
            identity_key("https://a.example", "one"),
            identity_key("https://b.example", "one")
        );
        assert_ne!(
            identity_key("https://a.example", "one"),
            identity_key("https://a.example", "two")
        );
    }
    #[test]
    fn storage_uses_the_credential_backend() {
        // The mock is test-only and is never selected by our production target features.
        let entry = Entry::new_with_credential(Box::new(keyring::mock::MockCredential::default()));
        let store = Keychain {
            entry,
            origin: "https://cloud.example".into(),
            account_id: "user".into(),
        };
        assert!(store.load().unwrap().is_none());
        let session = Session {
            origin: url::Url::parse("https://cloud.example").unwrap(),
            client: reqwest::Client::new(),
            token: SecretString::from("test-secret".to_string()),
            account_id: "user".into(),
        };
        store.store(&session).unwrap();
        assert_eq!(
            store.load().unwrap().unwrap().expose_secret(),
            "test-secret"
        );
        store.delete().unwrap();
        assert!(store.load().unwrap().is_none());
        let wrong = Keychain {
            entry: Entry::new_with_credential(Box::new(keyring::mock::MockCredential::default())),
            origin: "https://other.example".into(),
            account_id: "user".into(),
        };
        assert!(matches!(
            wrong.store(&session),
            Err(KeychainError::WrongIdentity)
        ));
    }
}
