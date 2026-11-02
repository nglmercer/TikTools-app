mod archive;
mod checksums;
mod digest;
mod error;
mod pack;
mod signing;
mod trust;

#[cfg(test)]
mod tests;

pub use archive::{
    ExtractedPackage, PackageArchive, PackageInspection, MAX_ARCHIVE_BYTES, MAX_CHANGELOG_BYTES,
    MAX_EXTRACTED_BYTES, MAX_FILES, MAX_MANIFEST_BYTES,
};
pub use checksums::Checksums;
pub use digest::{Sha256Digest, Sha256Hasher};
pub use error::{PackageError, PackageErrorKind};
pub use pack::{pack, sign_archive, PackOutput, PackRequest};
pub use signing::{
    DetachedSignature, KeyId, PackageSignature, PublicKey, SigningKey, SIGNATURE_ALGORITHM,
};
pub use trust::{KeyRole, KeysDocument, TrustStore, TrustedKey, VerifiedSignature};

pub const MANIFEST_FILE: &str = "plugin.json";
pub const CHECKSUMS_FILE: &str = "checksums.json";
pub const SIGNATURE_FILE: &str = "signature.json";
pub const PACKAGE_EXTENSION: &str = "plugin";
