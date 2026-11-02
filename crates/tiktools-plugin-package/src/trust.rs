use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{DetachedSignature, KeyId, PackageError, PackageSignature, PublicKey};

const KEYS_DOCUMENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyRole {
    Root,
    Index,
    Publisher,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedKey {
    #[serde(flatten)]
    pub key: PublicKey,
    pub role: KeyRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_before: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_after: Option<u64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub revoked: bool,
}

impl TrustedKey {
    pub fn new(key: PublicKey, role: KeyRole) -> Self {
        Self {
            key,
            role,
            publisher: None,
            scopes: Vec::new(),
            not_before: None,
            not_after: None,
            revoked: false,
        }
    }

    pub fn publisher(mut self, publisher: impl Into<String>) -> Self {
        self.publisher = Some(publisher.into());
        self
    }

    pub fn is_valid_at(&self, now: u64) -> bool {
        self.not_before.is_none_or(|start| now >= start)
            && self.not_after.is_none_or(|end| now <= end)
    }

    pub fn allows_package(&self, package_id: &str) -> bool {
        let Some(publisher) = self.publisher.as_deref() else {
            return false;
        };
        if self.scopes.is_empty() {
            return package_id
                .strip_prefix(publisher)
                .is_some_and(|rest| rest.starts_with('.') && rest.len() > 1);
        }
        self.scopes
            .iter()
            .any(|scope| match scope.strip_suffix('*') {
                Some(prefix) => package_id.starts_with(prefix) && package_id.len() > prefix.len(),
                None => scope == package_id,
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeysDocument {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<u64>,
    #[serde(default)]
    pub keys: Vec<TrustedKey>,
}

impl KeysDocument {
    pub fn new(keys: Vec<TrustedKey>) -> Self {
        Self {
            version: KEYS_DOCUMENT_VERSION,
            expires: None,
            keys,
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, PackageError> {
        let document: Self = serde_json::from_slice(bytes)
            .map_err(|error| PackageError::InvalidKey(error.to_string()))?;
        if document.version != KEYS_DOCUMENT_VERSION {
            return Err(PackageError::InvalidKey(format!(
                "unsupported keys document version {}",
                document.version
            )));
        }
        Ok(document)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec_pretty(self).unwrap_or_default();
        bytes.push(b'\n');
        bytes
    }

    pub fn is_expired(&self, now: u64) -> bool {
        self.expires.is_some_and(|expires| now > expires)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedSignature {
    pub publisher: String,
    pub key_id: KeyId,
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct TrustStore {
    keys: BTreeMap<KeyId, (TrustedKey, String)>,
}

impl TrustStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: TrustedKey, source: impl Into<String>) {
        self.keys
            .insert(key.key.key_id.clone(), (key, source.into()));
    }

    pub fn extend_delegations(&mut self, document: &KeysDocument, source: &str) {
        for key in document.keys.iter().filter(|key| key.role != KeyRole::Root) {
            let pinned_root = self
                .keys
                .get(&key.key.key_id)
                .is_some_and(|(existing, _)| existing.role == KeyRole::Root);
            if !pinned_root {
                self.insert(key.clone(), source);
            }
        }
    }

    pub fn remove_source(&mut self, source: &str) {
        self.keys.retain(|_, (_, owner)| owner != source);
    }

    pub fn get(&self, key_id: &KeyId) -> Option<&TrustedKey> {
        self.keys.get(key_id).map(|(key, _)| key)
    }

    pub fn keys(&self) -> impl Iterator<Item = (&TrustedKey, &str)> {
        self.keys
            .values()
            .map(|(key, source)| (key, source.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn verify_package(
        &self,
        signature: &PackageSignature,
        checksums: &[u8],
        package_id: &str,
        now: u64,
    ) -> Result<VerifiedSignature, PackageError> {
        let (key, source) = self.usable(&signature.key_id, KeyRole::Publisher, now)?;
        if key.publisher.as_deref() != Some(signature.publisher.as_str())
            || !key.allows_package(package_id)
        {
            return Err(PackageError::KeyScope {
                key: signature.key_id.to_string(),
                subject: package_id.to_owned(),
            });
        }
        key.key.verify(checksums, &signature.raw()?)?;
        Ok(VerifiedSignature {
            publisher: signature.publisher.clone(),
            key_id: signature.key_id.clone(),
            source: source.to_owned(),
        })
    }

    pub fn verify_detached(
        &self,
        signature: &DetachedSignature,
        message: &[u8],
        role: KeyRole,
        now: u64,
    ) -> Result<&TrustedKey, PackageError> {
        let (key, _) = self.usable(&signature.key_id, role, now)?;
        key.key.verify(message, &signature.raw()?)?;
        Ok(key)
    }

    fn usable(
        &self,
        key_id: &KeyId,
        role: KeyRole,
        now: u64,
    ) -> Result<(&TrustedKey, &str), PackageError> {
        let (key, source) = self
            .keys
            .get(key_id)
            .ok_or_else(|| PackageError::UnknownKey(key_id.to_string()))?;
        if key.revoked {
            return Err(PackageError::RevokedKey(key_id.to_string()));
        }
        if !key.is_valid_at(now) {
            return Err(PackageError::KeyExpired(key_id.to_string()));
        }
        if key.role != role {
            return Err(PackageError::KeyScope {
                key: key_id.to_string(),
                subject: format!("{role:?}").to_ascii_lowercase(),
            });
        }
        Ok((key, source.as_str()))
    }
}
