//! Declarative native-library providers for `napi-vm` plugins.
//!
//! A `nativeLibs` manifest entry pins one package version and names its
//! provider (`npm` by default, `github` for release assets). This module
//! resolves those declarations to verified bytes:
//!
//! - [`pin_native_lib`] contacts the provider, downloads the artifacts,
//!   and records content hashes into a [`NativeLibsLockfile`]. Pinning is
//!   the explicit trust moment: the lockfile diff is human reviewable and
//!   committed next to `plugin.json`.
//! - [`fetch_and_stage_native_libs`] replays a lockfile: it downloads the
//!   same bytes, re-verifies every hash, keeps only the requested
//!   target's `.node` binary, and stages the tree through
//!   [`native_stage`](crate::native_stage), so provider trees get the
//!   same validation as local checkouts.
//!
//! Fail-closed rules: exact version pins only (no ranges), every
//! downloaded byte verified against the lockfile before staging, unknown
//! targets rejected (never guessed), and no lockfile entry means no
//! download. The providers never run package scripts and never consult
//! anything outside the declared manifest entry plus its lockfile pins.

mod archive;
mod github;
mod integrity;
mod lockfile;
mod npm;
mod staging;
mod transport;

use archive::extract_npm_tarball;
use github::{fetch_github, pin_github};
#[cfg(test)]
use integrity::verify_pin;
use integrity::{sha256_pin, verify_asset, verify_integrity};
use lockfile::check_lock_matches;
pub use lockfile::{read_lockfile, write_lockfile};
#[cfg(test)]
use npm::node_serves_target;
use npm::{fetch_npm, pin_npm};
use staging::write_source_file;
pub use staging::{fetch_and_stage_native_libs, resolve_target};
pub use transport::provider_client;
use transport::{fetch_capped, trim_base};

use std::{
    collections::BTreeMap,
    io::Read,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use tiktools_plugin_api::{
    manifest::{current_napi_target, is_known_napi_target, NAPI_TARGETS},
    NativeLibDeclaration, NativeLibProvider, PluginManifest,
};

use crate::native_stage::{stage_native_package, NativeStageRequest};

/// Official npm registry. Overridable per call for mirrors and tests.
pub const DEFAULT_NPM_REGISTRY: &str = "https://registry.npmjs.org";
/// Official GitHub web base for release-asset downloads.
pub const DEFAULT_GITHUB_BASE: &str = "https://github.com";
/// Lockfile name, committed next to `plugin.json` and shipped in the
/// archive so the installer can replay it offline from the manifest.
pub const NATIVE_LIBS_LOCKFILE: &str = "native-libs.lock.json";
/// Lockfile schema version this module reads and writes.
pub const LOCKFILE_VERSION: u32 = 1;

/// Largest registry-metadata document accepted (JSON, small by nature).
const MAX_METADATA_BYTES: u64 = 1024 * 1024;
/// Largest single download accepted (tarballs, `.node` assets).
const MAX_DOWNLOAD_BYTES: u64 = 64 * 1024 * 1024;
/// Largest npm package extraction accepted overall.
const MAX_EXTRACTED_BYTES: u64 = 256 * 1024 * 1024;
/// Most files accepted from one npm tarball.
const MAX_EXTRACTED_FILES: usize = 4096;
/// Per-request network timeout.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Fetch bases. Tests point these at a local stub server; production
/// uses [`ProviderEndpoints::officials`].
#[derive(Debug, Clone)]
pub struct ProviderEndpoints {
    pub npm_registry: String,
    pub github_base: String,
}

impl Default for ProviderEndpoints {
    fn default() -> Self {
        Self::officials()
    }
}

impl ProviderEndpoints {
    pub fn officials() -> Self {
        Self {
            npm_registry: DEFAULT_NPM_REGISTRY.to_owned(),
            github_base: DEFAULT_GITHUB_BASE.to_owned(),
        }
    }
}

/// One pinned package: the immutable resolution of a `nativeLibs`
/// entry. `tarball`/`integrity` authenticate npm downloads;
/// `repo`/`tag`/`binary` locate GitHub release assets; `assets` pins
/// every staged filename to its SHA-256 (`sha256:<hex>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedNativeLib {
    pub version: String,
    pub provider: NativeLibProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tarball: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    #[serde(default)]
    pub assets: BTreeMap<String, String>,
}

/// Committed lockfile: package name to its pinned resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeLibsLockfile {
    pub version: u32,
    #[serde(default)]
    pub packages: BTreeMap<String, LockedNativeLib>,
}

/// What [`fetch_and_stage_native_libs`] staged for one library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NativeFetchReport {
    pub package: String,
    pub version: String,
    pub target: String,
    pub root: String,
    pub files: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum NativeProviderError {
    #[error("provider request failed for {url}: {message}")]
    Http { url: String, message: String },
    #[error("provider request for {url} returned HTTP {status}")]
    HttpStatus { url: String, status: u16 },
    #[error("provider has no such artifact: {0}")]
    NotFound(String),
    #[error("provider response for {url} exceeds the {limit}-byte cap")]
    TooLarge { url: String, limit: u64 },
    #[error("invalid provider metadata for {package}: {message}")]
    InvalidMetadata { package: String, message: String },
    #[error("integrity mismatch for {what}: download does not match its pin")]
    IntegrityMismatch { what: String },
    #[error("lockfile has no pin for {file} of {package}; re-pin with --pin")]
    PinMissing { package: String, file: String },
    #[error("downloaded {file} of {package} does not match its lockfile pin")]
    PinMismatch { package: String, file: String },
    #[error("lockfile entry for {package} does not match its manifest declaration: {message}")]
    LockMismatch { package: String, message: String },
    #[error("lockfile {path} uses unsupported schema version {version}")]
    LockVersion { path: String, version: u32 },
    #[error("lockfile has no entry for {0}; pin it with --pin first")]
    LockMissing(String),
    #[error("unknown napi target {target}; expected one of {}", NAPI_TARGETS.join(", "))]
    UnknownTarget { target: String },
    #[error("package {package} ships no binary for target {target}")]
    NoBinaryForTarget { package: String, target: String },
    #[error("package manifest names {found}, expected {expected}")]
    PackageNameMismatch { expected: String, found: String },
    #[error("native library {package} has no nativeAddons entry to stage into")]
    MissingAddonRoot { package: String },
    #[error("invalid npm tarball for {package}: {message}")]
    Tar { package: String, message: String },
    #[error("provider found no .node assets for {package}")]
    NoAssetsFound { package: String },
    #[error("provider IO failed for {path}: {message}")]
    Io { path: String, message: String },
    #[error("staging fetched package {package} failed: {message}")]
    Stage { package: String, message: String },
}

fn io_error(path: &Path, error: std::io::Error) -> NativeProviderError {
    NativeProviderError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}

fn metadata_error(package: &str, message: String) -> NativeProviderError {
    NativeProviderError::InvalidMetadata {
        package: package.to_owned(),
        message,
    }
}

/// Resolve one declaration against its provider and record the content
/// hashes. This is the trust moment: review the lockfile diff before
/// committing it.
pub async fn pin_native_lib(
    client: &Client,
    endpoints: &ProviderEndpoints,
    declaration: &NativeLibDeclaration,
) -> Result<LockedNativeLib, NativeProviderError> {
    match declaration.provider {
        NativeLibProvider::Npm => pin_npm(client, endpoints, declaration).await,
        NativeLibProvider::Github => pin_github(client, endpoints, declaration).await,
    }
}

/// Fetch one locked library for `target` into `dest` (created when
/// missing). The manifest declaration and its lockfile entry must agree
/// exactly; every downloaded byte is verified before it lands on disk.
pub async fn fetch_native_lib(
    client: &Client,
    endpoints: &ProviderEndpoints,
    declaration: &NativeLibDeclaration,
    locked: &LockedNativeLib,
    target: &str,
    dest: &Path,
) -> Result<Vec<String>, NativeProviderError> {
    if !is_known_napi_target(target) {
        return Err(NativeProviderError::UnknownTarget {
            target: target.to_owned(),
        });
    }
    check_lock_matches(&declaration.package, declaration, locked)?;
    std::fs::create_dir_all(dest).map_err(|error| io_error(dest, error))?;
    match declaration.provider {
        NativeLibProvider::Npm => fetch_npm(client, declaration, locked, target, dest).await,
        NativeLibProvider::Github => {
            fetch_github(client, endpoints, declaration, locked, target, dest).await
        }
    }
}

/// Inputs for [`fetch_and_stage_native_libs`].
pub struct FetchAndStage<'a> {
    pub client: &'a Client,
    pub endpoints: &'a ProviderEndpoints,
    pub manifest: &'a PluginManifest,
    pub lockfile: &'a NativeLibsLockfile,
    pub target: &'a str,
    pub plugin_dir: &'a Path,
    pub work_dir: &'a Path,
    pub overwrite: bool,
    pub provider_filter: Option<NativeLibProvider>,
}

#[cfg(test)]
mod tests;
