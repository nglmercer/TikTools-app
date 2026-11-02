use std::{fmt, fs, path::Path};

use base64::{engine::general_purpose::STANDARD, Engine};
use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519},
};
use serde::{Deserialize, Serialize};
use tiktools_plugin_api::manifest::is_valid_publisher;

use crate::{PackageError, Sha256Digest};

pub const SIGNATURE_ALGORITHM: &str = "ed25519";
const SIGNATURE_FORMAT_VERSION: u32 = 1;
const ED25519_PUBLIC_KEY_LEN: usize = 32;
const ED25519_SIGNATURE_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KeyId(String);

impl KeyId {
    pub fn parse(value: &str) -> Result<Self, PackageError> {
        let bytes = value.as_bytes();
        let valid = (2..=64).contains(&bytes.len())
            && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
            && bytes.iter().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-')
            });
        valid
            .then(|| Self(value.to_owned()))
            .ok_or_else(|| PackageError::InvalidKey(format!("invalid key id `{value}`")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KeyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for KeyId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value).map_err(|error| error.to_string())
    }
}

impl From<KeyId> for String {
    fn from(value: KeyId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKey {
    pub key_id: KeyId,
    pub algorithm: String,
    pub public_key: String,
}

impl PublicKey {
    pub fn new(key_id: KeyId, raw: &[u8]) -> Result<Self, PackageError> {
        if raw.len() != ED25519_PUBLIC_KEY_LEN {
            return Err(PackageError::InvalidKey(
                "ed25519 public keys are 32 bytes".to_owned(),
            ));
        }
        Ok(Self {
            key_id,
            algorithm: SIGNATURE_ALGORITHM.to_owned(),
            public_key: STANDARD.encode(raw),
        })
    }

    pub fn raw(&self) -> Result<Vec<u8>, PackageError> {
        if self.algorithm != SIGNATURE_ALGORITHM {
            return Err(PackageError::InvalidKey(format!(
                "unsupported key algorithm `{}`",
                self.algorithm
            )));
        }
        let raw = STANDARD
            .decode(&self.public_key)
            .map_err(|error| PackageError::InvalidKey(error.to_string()))?;
        if raw.len() != ED25519_PUBLIC_KEY_LEN {
            return Err(PackageError::InvalidKey(
                "ed25519 public keys are 32 bytes".to_owned(),
            ));
        }
        Ok(raw)
    }

    pub fn fingerprint(&self) -> Result<Sha256Digest, PackageError> {
        Ok(Sha256Digest::of_bytes(&self.raw()?))
    }

    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), PackageError> {
        if signature.len() != ED25519_SIGNATURE_LEN {
            return Err(PackageError::SignatureMismatch);
        }
        UnparsedPublicKey::new(&ED25519, self.raw()?)
            .verify(message, signature)
            .map_err(|_| PackageError::SignatureMismatch)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SigningKeyFile {
    version: u32,
    algorithm: String,
    key_id: KeyId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    publisher: Option<String>,
    private_key: String,
}

pub struct SigningKey {
    key_id: KeyId,
    publisher: Option<String>,
    pkcs8: Vec<u8>,
    pair: Ed25519KeyPair,
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SigningKey")
            .field("key_id", &self.key_id)
            .field("publisher", &self.publisher)
            .finish_non_exhaustive()
    }
}

impl SigningKey {
    pub fn generate(key_id: KeyId, publisher: Option<String>) -> Result<Self, PackageError> {
        if let Some(publisher) = publisher
            .as_deref()
            .filter(|value| !is_valid_publisher(value))
        {
            return Err(PackageError::InvalidKey(format!(
                "invalid publisher `{publisher}`"
            )));
        }
        let document = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
            .map_err(|_| PackageError::InvalidKey("could not generate a key".to_owned()))?;
        Self::from_pkcs8(key_id, publisher, document.as_ref().to_vec())
    }

    pub fn from_pkcs8(
        key_id: KeyId,
        publisher: Option<String>,
        pkcs8: Vec<u8>,
    ) -> Result<Self, PackageError> {
        let pair = Ed25519KeyPair::from_pkcs8(&pkcs8)
            .map_err(|error| PackageError::InvalidKey(error.to_string()))?;
        Ok(Self {
            key_id,
            publisher,
            pkcs8,
            pair,
        })
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, PackageError> {
        let file: SigningKeyFile = serde_json::from_slice(bytes)
            .map_err(|error| PackageError::InvalidKey(error.to_string()))?;
        if file.version != SIGNATURE_FORMAT_VERSION || file.algorithm != SIGNATURE_ALGORITHM {
            return Err(PackageError::InvalidKey(
                "unsupported signing key format".to_owned(),
            ));
        }
        let pkcs8 = STANDARD
            .decode(&file.private_key)
            .map_err(|error| PackageError::InvalidKey(error.to_string()))?;
        Self::from_pkcs8(file.key_id, file.publisher, pkcs8)
    }

    pub fn load(path: &Path) -> Result<Self, PackageError> {
        Self::from_json(&fs::read(path)?)
    }

    pub fn to_json(&self) -> Vec<u8> {
        let file = SigningKeyFile {
            version: SIGNATURE_FORMAT_VERSION,
            algorithm: SIGNATURE_ALGORITHM.to_owned(),
            key_id: self.key_id.clone(),
            publisher: self.publisher.clone(),
            private_key: STANDARD.encode(&self.pkcs8),
        };
        let mut bytes = serde_json::to_vec_pretty(&file).unwrap_or_default();
        bytes.push(b'\n');
        bytes
    }

    pub fn key_id(&self) -> &KeyId {
        &self.key_id
    }

    pub fn publisher(&self) -> Option<&str> {
        self.publisher.as_deref()
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            key_id: self.key_id.clone(),
            algorithm: SIGNATURE_ALGORITHM.to_owned(),
            public_key: STANDARD.encode(self.pair.public_key().as_ref()),
        }
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.pair.sign(message).as_ref().to_vec()
    }

    pub fn sign_detached(&self, message: &[u8]) -> DetachedSignature {
        DetachedSignature {
            version: SIGNATURE_FORMAT_VERSION,
            algorithm: SIGNATURE_ALGORITHM.to_owned(),
            key_id: self.key_id.clone(),
            signature: STANDARD.encode(self.sign(message)),
        }
    }

    pub fn sign_package(&self, checksums: &[u8]) -> Result<PackageSignature, PackageError> {
        let publisher = self.publisher.clone().ok_or_else(|| {
            PackageError::InvalidKey("package signing keys must name a publisher".to_owned())
        })?;
        Ok(PackageSignature {
            version: SIGNATURE_FORMAT_VERSION,
            algorithm: SIGNATURE_ALGORITHM.to_owned(),
            key_id: self.key_id.clone(),
            publisher,
            signature: STANDARD.encode(self.sign(checksums)),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageSignature {
    pub version: u32,
    pub algorithm: String,
    pub key_id: KeyId,
    pub publisher: String,
    pub signature: String,
}

impl PackageSignature {
    pub fn parse(bytes: &[u8]) -> Result<Self, PackageError> {
        let signature: Self = serde_json::from_slice(bytes)
            .map_err(|error| PackageError::InvalidSignature(error.to_string()))?;
        if signature.version != SIGNATURE_FORMAT_VERSION
            || signature.algorithm != SIGNATURE_ALGORITHM
            || !is_valid_publisher(&signature.publisher)
        {
            return Err(PackageError::InvalidSignature(
                "unsupported signature format".to_owned(),
            ));
        }
        Ok(signature)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec_pretty(self).unwrap_or_default();
        bytes.push(b'\n');
        bytes
    }

    pub fn raw(&self) -> Result<Vec<u8>, PackageError> {
        decode_signature(&self.signature)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedSignature {
    pub version: u32,
    pub algorithm: String,
    pub key_id: KeyId,
    pub signature: String,
}

impl DetachedSignature {
    pub fn parse(bytes: &[u8]) -> Result<Self, PackageError> {
        let signature: Self = serde_json::from_slice(bytes)
            .map_err(|error| PackageError::InvalidSignature(error.to_string()))?;
        if signature.version != SIGNATURE_FORMAT_VERSION
            || signature.algorithm != SIGNATURE_ALGORITHM
        {
            return Err(PackageError::InvalidSignature(
                "unsupported signature format".to_owned(),
            ));
        }
        Ok(signature)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec_pretty(self).unwrap_or_default();
        bytes.push(b'\n');
        bytes
    }

    pub fn raw(&self) -> Result<Vec<u8>, PackageError> {
        decode_signature(&self.signature)
    }
}

fn decode_signature(value: &str) -> Result<Vec<u8>, PackageError> {
    let raw = STANDARD
        .decode(value)
        .map_err(|error| PackageError::InvalidSignature(error.to_string()))?;
    if raw.len() != ED25519_SIGNATURE_LEN {
        return Err(PackageError::InvalidSignature(
            "ed25519 signatures are 64 bytes".to_owned(),
        ));
    }
    Ok(raw)
}
