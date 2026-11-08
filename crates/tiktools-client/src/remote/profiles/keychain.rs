use super::*;
/// Blocking credential-store seam. Implementations must not log values or
/// silently substitute plaintext files when their designated backend fails.
pub trait CredentialStore: Send + Sync {
    fn load(&self, key: &str) -> Result<Option<SecretString>, ClientError>;
    fn save(&self, key: &str, secret: &SecretString) -> Result<(), ClientError>;
    fn delete(&self, key: &str) -> Result<(), ClientError>;
}
pub struct OsCredentialStore;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl OsCredentialStore {
    fn entry(key: &str) -> Result<keyring::Entry, ClientError> {
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(credential_error());
        }
        #[cfg(target_os = "linux")]
        let builder = keyring::secret_service::default_credential_builder();
        #[cfg(target_os = "macos")]
        let builder = keyring::macos::default_credential_builder();
        #[cfg(target_os = "windows")]
        let builder = keyring::windows::default_credential_builder();
        let credential = builder
            .build(None, "TikTools LIVE remote v1", key)
            .map_err(|_| credential_error())?;
        Ok(keyring::Entry::new_with_credential(credential))
    }
}
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl CredentialStore for OsCredentialStore {
    fn load(&self, key: &str) -> Result<Option<SecretString>, ClientError> {
        match Self::entry(key)?.get_password() {
            Ok(value) => Ok(Some(SecretString::from(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(credential_error()),
        }
    }
    fn save(&self, key: &str, secret: &SecretString) -> Result<(), ClientError> {
        use secrecy::ExposeSecret;
        Self::entry(key)?
            .set_password(secret.expose_secret())
            .map_err(|_| credential_error())
    }
    fn delete(&self, key: &str) -> Result<(), ClientError> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(credential_error()),
        }
    }
}

// Remote networking can still be compiled on targets such as Android/Termux.
// Their profile backend must fail explicitly instead of referring to a
// platform keyring dependency that is not linked or using a plaintext store.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl CredentialStore for OsCredentialStore {
    fn load(&self, _: &str) -> Result<Option<SecretString>, ClientError> {
        Err(credential_error())
    }
    fn save(&self, _: &str, _: &SecretString) -> Result<(), ClientError> {
        Err(credential_error())
    }
    fn delete(&self, _: &str) -> Result<(), ClientError> {
        Err(credential_error())
    }
}
