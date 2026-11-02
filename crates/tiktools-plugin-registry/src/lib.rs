mod client;
mod download;
mod error;
mod index;
mod model;
mod resolver;
mod source;
mod url_policy;

#[cfg(test)]
mod tests;

pub use client::{RegistryClient, RegistryStatus, SearchEntry, SearchQuery, SearchResults};
pub use download::{DownloadRequest, Downloaded, Downloader, MAX_DOWNLOAD_BYTES};
pub use error::RegistryError;
pub use index::{IndexWriter, IndexedRelease};
pub use model::{
    shard_path, Advisory, AdvisorySeverity, Artifact, CatalogEntry, CatalogPage, PackageDocument,
    RegistryConfigDocument, Release, RootDocument, ANY_TARGET, CHANNEL_BETA, CHANNEL_STABLE,
    REGISTRY_PROTOCOL_VERSION,
};
pub use resolver::{
    latest_compatible, resolve, Rejection, RejectionReason, ResolveRequest, ResolvedRelease,
};
pub use source::{AutoUpdate, RegistriesConfig, RegistryDefaults, RegistrySource};
pub use url_policy::{is_allowed_url, join_url};
