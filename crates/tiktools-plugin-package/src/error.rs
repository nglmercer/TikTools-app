use std::io;

use thiserror::Error;
use tiktools_plugin_api::manifest::ManifestError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageErrorKind {
    InvalidPackage,
    Incompatible,
    Signature,
    UntrustedPublisher,
    Revoked,
    Io,
}

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("could not read plugin archive: {0}")]
    Archive(String),
    #[error("plugin packages must use the .plugin extension")]
    WrongExtension,
    #[error("plugin package is not a regular file")]
    NotAFile,
    #[error("plugin package exceeds the 512 MB limit")]
    ArchiveTooLarge,
    #[error("plugin archive expands beyond the 2 GB limit")]
    ExpandsTooLarge,
    #[error("plugin archive contains an invalid number of entries")]
    InvalidEntryCount,
    #[error("plugin archive contains too many files")]
    TooManyFiles,
    #[error("plugin archive contains unsafe path: {0}")]
    UnsafePath(String),
    #[error("plugin archive contains a duplicate path: {0}")]
    DuplicatePath(String),
    #[error("plugin archive contains a symbolic link")]
    Symlink,
    #[error("plugin archive contains an unsupported entry")]
    UnsupportedEntry,
    #[error("plugin archive contains a path traversal entry")]
    Traversal,
    #[error("plugin archive must contain plugin.json at its root")]
    MissingManifest,
    #[error("plugin archive contains multiple plugin.json manifests")]
    MultipleManifests,
    #[error("plugin manifest is not UTF-8")]
    ManifestEncoding,
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("plugin {0} is missing checksums.json")]
    MissingChecksums(String),
    #[error("checksums.json is invalid: {0}")]
    InvalidChecksums(String),
    #[error("checksum target is not a file: {0}")]
    MissingFile(String),
    #[error("checksum mismatch in {id}: {path}")]
    ChecksumMismatch { id: String, path: String },
    #[error("plugin checksum is missing for {0}")]
    UnlistedFile(String),
    #[error("signature.json is invalid: {0}")]
    InvalidSignature(String),
    #[error("plugin signature does not verify")]
    SignatureMismatch,
    #[error("signing key {0} is not trusted")]
    UnknownKey(String),
    #[error("signing key {0} has been revoked")]
    RevokedKey(String),
    #[error("signing key {0} is expired or not yet valid")]
    KeyExpired(String),
    #[error("signing key {key} may not sign {subject}")]
    KeyScope { key: String, subject: String },
    #[error("signing key file is invalid: {0}")]
    InvalidKey(String),
    #[error("{0}")]
    InvalidInput(String),
}

impl PackageError {
    pub fn kind(&self) -> PackageErrorKind {
        match self {
            Self::Io(_) => PackageErrorKind::Io,
            Self::Manifest(ManifestError::UnsupportedSchema(_))
            | Self::Manifest(ManifestError::InvalidField("protocolVersion" | "abiVersion")) => {
                PackageErrorKind::Incompatible
            }
            Self::InvalidSignature(_) | Self::SignatureMismatch | Self::InvalidKey(_) => {
                PackageErrorKind::Signature
            }
            Self::UnknownKey(_) | Self::KeyExpired(_) | Self::KeyScope { .. } => {
                PackageErrorKind::UntrustedPublisher
            }
            Self::RevokedKey(_) => PackageErrorKind::Revoked,
            _ => PackageErrorKind::InvalidPackage,
        }
    }

    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }
}

impl From<zip::result::ZipError> for PackageError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::Archive(error.to_string())
    }
}
