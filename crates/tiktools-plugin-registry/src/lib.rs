mod client;
pub mod cloud;
mod download;
mod error;
mod index;
mod model;
mod resolver;
mod source;
mod url_policy;

#[cfg(test)]
mod cloud_tests;
#[cfg(test)]
mod tests;

pub use client::{
    RegistryClient, RegistryStatus, RegistryTrustGuard, SearchEntry, SearchQuery, SearchResults,
};
pub use cloud::{
    cloud_source, generation_files_url, CloudGeneration, OFFICIAL_REGISTRY_NAME,
    STAGING_REGISTRY_NAME,
};
pub use download::{DownloadRequest, Downloaded, Downloader, MAX_DOWNLOAD_BYTES};
pub use error::RegistryError;
pub use index::{IndexWriter, IndexedRelease};
pub use model::{
    shard_path, Advisory, AdvisorySeverity, Artifact, CatalogEntry, CatalogPage, LiveReleaseStatus,
    PackageDocument, RegistryConfigDocument, Release, RootDocument, ANY_TARGET, CHANNEL_BETA,
    CHANNEL_STABLE, REGISTRY_PROTOCOL_VERSION,
};
pub use resolver::{
    compatible_artifact, latest_compatible, resolve, Rejection, RejectionReason, ResolveRequest,
    ResolvedRelease,
};
pub use source::{AutoUpdate, RegistriesConfig, RegistryDefaults, RegistrySource};
pub use url_policy::{is_allowed_url, join_url};
