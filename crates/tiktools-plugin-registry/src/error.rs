use std::io;

use thiserror::Error;
use tiktools_plugin_package::PackageError;

use crate::Rejection;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("registry `{0}` is not configured")]
    UnknownRegistry(String),
    #[error("cloud registry at {origin} has no published generation yet")]
    NoPublishedGeneration { origin: String },
    #[error("registry `{0}` already exists")]
    DuplicateRegistry(String),
    #[error("registry name `{0}` is invalid")]
    InvalidName(String),
    #[error("plugin `{0}` was not found in any configured registry")]
    NotFound(String),
    #[error("could not reach {url}: {message}")]
    Http { url: String, message: String },
    #[error("registry returned HTTP {status} for {url}")]
    Status { url: String, status: u16 },
    #[error("registry document {path} is invalid: {message}")]
    InvalidDocument { path: String, message: String },
    #[error("registry document {path} failed verification: {source}")]
    Signature {
        path: String,
        #[source]
        source: PackageError,
    },
    #[error("registry protocol {0} is not supported")]
    UnsupportedProtocol(u32),
    #[error("registry metadata expired; refresh the registry")]
    Expired,
    #[error(
        "downloaded file does not match the published digest (expected {expected}, got {actual})"
    )]
    DigestMismatch { expected: String, actual: String },
    #[error(
        "downloaded file does not match the published size (expected {expected}, got {actual})"
    )]
    SizeMismatch { expected: u64, actual: u64 },
    #[error("download exceeds the {0} byte limit")]
    TooLarge(u64),
    #[error("download was cancelled")]
    Cancelled,
    #[error("only https URLs are allowed (http is accepted for localhost): {0}")]
    InsecureUrl(String),
    #[error("no compatible release of {id}")]
    NoCompatibleRelease {
        id: String,
        rejections: Vec<Rejection>,
    },
    #[error(transparent)]
    Package(#[from] PackageError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl RegistryError {
    pub(crate) fn http(url: &str, error: impl std::fmt::Display) -> Self {
        Self::Http {
            url: url.to_owned(),
            message: error.to_string(),
        }
    }

    pub(crate) fn document(path: &str, message: impl std::fmt::Display) -> Self {
        Self::InvalidDocument {
            path: path.to_owned(),
            message: message.to_string(),
        }
    }
}
