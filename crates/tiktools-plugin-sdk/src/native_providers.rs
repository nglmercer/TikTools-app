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

/// HTTP client for provider downloads: rustls, per-request timeout,
/// default redirect policy. Integrity pins authenticate every byte, so
/// transport is untrusted by design.
pub fn provider_client() -> Result<Client, NativeProviderError> {
    Client::builder()
        .timeout(FETCH_TIMEOUT)
        .user_agent(format!(
            "tiktools-native-providers/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|error| NativeProviderError::Http {
            url: String::new(),
            message: format!("could not build HTTP client: {error}"),
        })
}

/// Download a URL with a byte cap. The `content-length` header is
/// checked before reading and the running total while streaming, so a
/// lying server cannot exhaust memory.
async fn fetch_capped(
    client: &Client,
    url: &str,
    limit: u64,
) -> Result<Vec<u8>, NativeProviderError> {
    let response = client.get(url).send().await.map_err(|error| {
        if error.is_timeout() {
            NativeProviderError::Http {
                url: url.to_owned(),
                message: format!("request timed out after {}s", FETCH_TIMEOUT.as_secs()),
            }
        } else {
            NativeProviderError::Http {
                url: url.to_owned(),
                message: error.to_string(),
            }
        }
    })?;
    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        return Err(NativeProviderError::NotFound(url.to_owned()));
    }
    if !status.is_success() {
        return Err(NativeProviderError::HttpStatus {
            url: url.to_owned(),
            status: status.as_u16(),
        });
    }
    if let Some(declared) = response.content_length() {
        if declared > limit {
            return Err(NativeProviderError::TooLarge {
                url: url.to_owned(),
                limit,
            });
        }
    }
    let mut bytes = Vec::new();
    let mut response = response;
    loop {
        let chunk = response
            .chunk()
            .await
            .map_err(|error| NativeProviderError::Http {
                url: url.to_owned(),
                message: error.to_string(),
            })?;
        let Some(chunk) = chunk else {
            break;
        };
        bytes.extend_from_slice(&chunk);
        if bytes.len() as u64 > limit {
            return Err(NativeProviderError::TooLarge {
                url: url.to_owned(),
                limit,
            });
        }
    }
    Ok(bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(ALPHABET[(byte >> 4) as usize] as char);
        text.push(ALPHABET[(byte & 0x0f) as usize] as char);
    }
    text
}

/// Hex SHA-256 digest rendered as a lockfile pin (`sha256:<hex>`).
fn sha256_pin(bytes: &[u8]) -> String {
    format!("sha256:{}", hex_encode(&Sha256::digest(bytes)))
}

/// Verify `bytes` against a `sha256:<hex>` pin. Malformed pins fail
/// closed: a pin that cannot be parsed never matches.
fn verify_pin(bytes: &[u8], pin: &str) -> bool {
    let Some(expected) = pin.strip_prefix("sha256:") else {
        return false;
    };
    expected.len() == 64 && hex_encode(&Sha256::digest(bytes)) == expected
}

/// Verify `bytes` against an npm `integrity` value
/// (`sha512-<base64>` or `sha256-<base64>`). Unknown algorithms and
/// malformed values fail closed.
fn verify_integrity(bytes: &[u8], integrity: &str) -> bool {
    use base64::Engine;
    let Some((algorithm, encoded)) = integrity.split_once('-') else {
        return false;
    };
    let Ok(expected) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return false;
    };
    match algorithm {
        "sha512" => Sha512::digest(bytes).as_slice() == expected.as_slice(),
        "sha256" => Sha256::digest(bytes).as_slice() == expected.as_slice(),
        _ => false,
    }
}

/// One extracted npm package file: path relative to the package root
/// plus its bytes.
struct ExtractedFile {
    relative: String,
    bytes: Vec<u8>,
}

/// Extract an npm tarball (`package/...` entries) into memory. Rejects
/// absolute paths, `..` segments, symlinks, hardlinks, non-regular
/// files, and anything outside `package/`; enforces file-count and
/// byte caps. Never writes: callers decide what lands on disk.
fn extract_npm_tarball(
    package: &str,
    tarball: &[u8],
) -> Result<Vec<ExtractedFile>, NativeProviderError> {
    let tar_error = |message: String| NativeProviderError::Tar {
        package: package.to_owned(),
        message,
    };
    let gzip = flate2::read::GzDecoder::new(tarball);
    let mut archive = tar::Archive::new(gzip);
    let entries = archive
        .entries()
        .map_err(|error| tar_error(format!("unreadable archive: {error}")))?;
    let mut files = Vec::new();
    let mut total: u64 = 0;
    for entry in entries {
        let mut entry = entry.map_err(|error| tar_error(format!("unreadable entry: {error}")))?;
        if files.len() >= MAX_EXTRACTED_FILES {
            return Err(tar_error(format!(
                "archive holds more than {MAX_EXTRACTED_FILES} files"
            )));
        }
        if !entry.header().entry_type().is_file() {
            return Err(tar_error("archive holds a non-regular file".to_owned()));
        }
        let path: PathBuf = entry
            .path()
            .map_err(|error| tar_error(format!("undecodable entry path: {error}")))?
            .into_owned();
        let mut components = path.components();
        if !matches!(components.next(), Some(Component::Normal(first)) if first == "package") {
            return Err(tar_error(format!(
                "entry {} escapes the package root",
                path.display()
            )));
        }
        let mut relative = PathBuf::new();
        for component in components {
            let Some(name) = (match component {
                Component::Normal(name) => Some(name),
                _ => None,
            }) else {
                return Err(tar_error(format!(
                    "entry {} is not a safe relative path",
                    path.display()
                )));
            };
            relative.push(name);
        }
        if relative.as_os_str().is_empty() {
            return Err(tar_error("archive holds an empty entry path".to_owned()));
        }
        let relative = relative.display().to_string().replace('\\', "/");
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| tar_error(format!("unreadable entry {relative}: {error}")))?;
        total += bytes.len() as u64;
        if total > MAX_EXTRACTED_BYTES {
            return Err(tar_error(format!(
                "archive extracts to more than {MAX_EXTRACTED_BYTES} bytes"
            )));
        }
        files.push(ExtractedFile { relative, bytes });
    }
    if files.is_empty() {
        return Err(tar_error("archive holds no files".to_owned()));
    }
    Ok(files)
}

/// Registry path segment for a package: scoped slashes escape as
/// `%2F` (`@scope/pkg` -> `@scope%2Fpkg`).
fn registry_package_path(package: &str) -> String {
    package.replace('/', "%2F")
}

fn trim_base(base: &str) -> &str {
    base.trim_end_matches('/')
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

async fn pin_npm(
    client: &Client,
    endpoints: &ProviderEndpoints,
    declaration: &NativeLibDeclaration,
) -> Result<LockedNativeLib, NativeProviderError> {
    let package = &declaration.package;
    let url = format!(
        "{}/{}/{}",
        trim_base(&endpoints.npm_registry),
        registry_package_path(package),
        declaration.version,
    );
    let body = fetch_capped(client, &url, MAX_METADATA_BYTES).await?;
    let metadata: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|error| metadata_error(package, format!("metadata is not JSON: {error}")))?;
    let name = metadata
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if name != package {
        return Err(metadata_error(
            package,
            format!("registry answered for package {name:?}"),
        ));
    }
    let answered = metadata
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if answered != declaration.version {
        return Err(metadata_error(
            package,
            format!("registry answered version {answered:?}"),
        ));
    }
    let dist = metadata
        .get("dist")
        .ok_or_else(|| metadata_error(package, "metadata has no dist object".to_owned()))?;
    let tarball = dist
        .get("tarball")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| metadata_error(package, "metadata has no dist.tarball".to_owned()))?;
    let integrity = dist
        .get("integrity")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| metadata_error(package, "metadata has no dist.integrity".to_owned()))?;
    if tarball.is_empty() || integrity.is_empty() {
        return Err(metadata_error(
            package,
            "metadata dist fields are empty".to_owned(),
        ));
    }
    let archive = fetch_capped(client, tarball, MAX_DOWNLOAD_BYTES).await?;
    if !verify_integrity(&archive, integrity) {
        return Err(NativeProviderError::IntegrityMismatch {
            what: format!("{package} tarball"),
        });
    }
    let files = extract_npm_tarball(package, &archive)?;
    let manifest = files
        .iter()
        .find(|file| file.relative == "package.json")
        .ok_or_else(|| NativeProviderError::Tar {
            package: package.clone(),
            message: "archive holds no package.json".to_owned(),
        })?;
    let manifest_json: serde_json::Value =
        serde_json::from_slice(&manifest.bytes).map_err(|error| NativeProviderError::Tar {
            package: package.clone(),
            message: format!("package.json is not JSON: {error}"),
        })?;
    let found = manifest_json
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if found != package {
        return Err(NativeProviderError::PackageNameMismatch {
            expected: package.clone(),
            found: found.to_owned(),
        });
    }
    let mut assets = BTreeMap::new();
    for file in files.iter().filter(|file| file.relative.ends_with(".node")) {
        assets.insert(file.relative.clone(), sha256_pin(&file.bytes));
    }
    if assets.is_empty() {
        return Err(NativeProviderError::NoAssetsFound {
            package: package.clone(),
        });
    }
    Ok(LockedNativeLib {
        version: declaration.version.clone(),
        provider: NativeLibProvider::Npm,
        tarball: Some(tarball.to_owned()),
        integrity: Some(integrity.to_owned()),
        repo: None,
        tag: None,
        binary: None,
        assets,
    })
}

/// Companion sources every GitHub release must carry next to the
/// `.node` assets: the loader entry and its types.
const GITHUB_COMPANION_FILES: [&str; 2] = ["index.js", "index.d.ts"];

fn github_asset_url(endpoints: &ProviderEndpoints, repo: &str, tag: &str, asset: &str) -> String {
    format!(
        "{}/{repo}/releases/download/{tag}/{asset}",
        trim_base(&endpoints.github_base),
    )
}

async fn pin_github(
    client: &Client,
    endpoints: &ProviderEndpoints,
    declaration: &NativeLibDeclaration,
) -> Result<LockedNativeLib, NativeProviderError> {
    let package = &declaration.package;
    let (Some(repo), Some(tag), Some(binary)) = (
        declaration.repo.as_deref(),
        declaration.tag.as_deref(),
        declaration.binary.as_deref(),
    ) else {
        return Err(metadata_error(
            package,
            "github declaration needs repo, tag, and binary".to_owned(),
        ));
    };
    let mut assets = BTreeMap::new();
    for companion in GITHUB_COMPANION_FILES {
        let url = github_asset_url(endpoints, repo, tag, companion);
        let bytes = fetch_capped(client, &url, MAX_DOWNLOAD_BYTES).await?;
        assets.insert(companion.to_owned(), sha256_pin(&bytes));
    }
    for target in NAPI_TARGETS {
        let asset = format!("{binary}.{target}.node");
        let url = github_asset_url(endpoints, repo, tag, &asset);
        match fetch_capped(client, &url, MAX_DOWNLOAD_BYTES).await {
            Ok(bytes) => {
                assets.insert(asset, sha256_pin(&bytes));
            }
            Err(NativeProviderError::NotFound(_)) => continue,
            Err(error) => return Err(error),
        }
    }
    if !assets.keys().any(|name| name.ends_with(".node")) {
        return Err(NativeProviderError::NoAssetsFound {
            package: package.clone(),
        });
    }
    Ok(LockedNativeLib {
        version: declaration.version.clone(),
        provider: NativeLibProvider::Github,
        tarball: None,
        integrity: None,
        repo: Some(repo.to_owned()),
        tag: Some(tag.to_owned()),
        binary: Some(binary.to_owned()),
        assets,
    })
}

/// Read and validate a lockfile. Unknown schema versions fail closed:
/// an unreadable lockfile never degrades into an unpinned download.
pub fn read_lockfile(path: &Path) -> Result<NativeLibsLockfile, NativeProviderError> {
    let bytes = std::fs::read(path).map_err(|error| io_error(path, error))?;
    let lockfile: NativeLibsLockfile =
        serde_json::from_slice(&bytes).map_err(|error| NativeProviderError::Io {
            path: path.display().to_string(),
            message: format!("lockfile is not valid JSON: {error}"),
        })?;
    if lockfile.version != LOCKFILE_VERSION {
        return Err(NativeProviderError::LockVersion {
            path: path.display().to_string(),
            version: lockfile.version,
        });
    }
    Ok(lockfile)
}

/// Write a lockfile: pretty JSON with a trailing newline, deterministic
/// key order (`BTreeMap`), so diffs stay reviewable.
pub fn write_lockfile(
    path: &Path,
    lockfile: &NativeLibsLockfile,
) -> Result<(), NativeProviderError> {
    let mut text =
        serde_json::to_string_pretty(lockfile).map_err(|error| NativeProviderError::Io {
            path: path.display().to_string(),
            message: format!("lockfile is not serializable: {error}"),
        })?;
    text.push('\n');
    std::fs::write(path, text).map_err(|error| io_error(path, error))
}

fn check_lock_matches(
    package: &str,
    declaration: &NativeLibDeclaration,
    locked: &LockedNativeLib,
) -> Result<(), NativeProviderError> {
    let mismatch = |message: &str| NativeProviderError::LockMismatch {
        package: package.to_owned(),
        message: message.to_owned(),
    };
    if locked.provider != declaration.provider {
        return Err(mismatch("provider changed; re-pin"));
    }
    if locked.version != declaration.version {
        return Err(mismatch("version changed; re-pin"));
    }
    if declaration.provider == NativeLibProvider::Github
        && (locked.repo.as_deref() != declaration.repo.as_deref()
            || locked.tag.as_deref() != declaration.tag.as_deref()
            || locked.binary.as_deref() != declaration.binary.as_deref())
    {
        return Err(mismatch("repo, tag, or binary changed; re-pin"));
    }
    Ok(())
}

fn verify_asset(
    package: &str,
    locked: &LockedNativeLib,
    file: &str,
    bytes: &[u8],
) -> Result<(), NativeProviderError> {
    let Some(pin) = locked.assets.get(file) else {
        return Err(NativeProviderError::PinMissing {
            package: package.to_owned(),
            file: file.to_owned(),
        });
    };
    if !verify_pin(bytes, pin) {
        return Err(NativeProviderError::PinMismatch {
            package: package.to_owned(),
            file: file.to_owned(),
        });
    }
    Ok(())
}

fn write_source_file(dest: &Path, relative: &str, bytes: &[u8]) -> Result<(), NativeProviderError> {
    let path = dest.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| io_error(&path, error))?;
    }
    std::fs::write(&path, bytes).map_err(|error| io_error(&path, error))
}

/// Whether a top-level `.node` filename serves `target`
/// (`{stem}.{target}.node`, dot boundary required).
fn node_serves_target(file: &str, target: &str) -> bool {
    if file.contains('/') || !file.ends_with(".node") {
        return false;
    }
    file.strip_suffix(".node").is_some_and(|stem| {
        stem.len() > target.len()
            && stem.ends_with(target)
            && stem.as_bytes()[stem.len() - target.len() - 1] == b'.'
    })
}

async fn fetch_npm(
    client: &Client,
    declaration: &NativeLibDeclaration,
    locked: &LockedNativeLib,
    target: &str,
    dest: &Path,
) -> Result<Vec<String>, NativeProviderError> {
    let package = &declaration.package;
    let (Some(tarball), Some(integrity)) = (locked.tarball.as_deref(), locked.integrity.as_deref())
    else {
        return Err(NativeProviderError::LockMismatch {
            package: package.clone(),
            message: "npm entry lacks its tarball pin; re-pin".to_owned(),
        });
    };
    let archive = fetch_capped(client, tarball, MAX_DOWNLOAD_BYTES).await?;
    if !verify_integrity(&archive, integrity) {
        return Err(NativeProviderError::IntegrityMismatch {
            what: format!("{package} tarball"),
        });
    }
    let files = extract_npm_tarball(package, &archive)?;
    let mut staged = Vec::new();
    let mut saw_target_binary = false;
    for file in &files {
        if file.relative.ends_with(".node") {
            if !node_serves_target(&file.relative, target) {
                continue;
            }
            saw_target_binary = true;
            verify_asset(package, locked, &file.relative, &file.bytes)?;
        }
        write_source_file(dest, &file.relative, &file.bytes)?;
        staged.push(file.relative.clone());
    }
    if !saw_target_binary {
        return Err(NativeProviderError::NoBinaryForTarget {
            package: package.clone(),
            target: target.to_owned(),
        });
    }
    if !staged.iter().any(|file| file == "package.json") {
        return Err(NativeProviderError::Tar {
            package: package.clone(),
            message: "archive holds no package.json".to_owned(),
        });
    }
    Ok(staged)
}

/// Minimal `package.json` for a GitHub-assembled tree. Releases carry
/// no manifest, so the provider writes one in the napi-rs shape
/// (`binaryName`, plus a `files` allowlist matching what was fetched).
fn github_package_manifest(declaration: &NativeLibDeclaration, binary: &str) -> String {
    let manifest = serde_json::json!({
        "name": declaration.package,
        "version": declaration.version,
        "main": "index.js",
        "types": "index.d.ts",
        "files": ["index.js", "index.d.ts", "*.node"],
        "napi": {"binaryName": binary},
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&manifest).unwrap_or_default()
    )
}

async fn fetch_github(
    client: &Client,
    endpoints: &ProviderEndpoints,
    declaration: &NativeLibDeclaration,
    locked: &LockedNativeLib,
    target: &str,
    dest: &Path,
) -> Result<Vec<String>, NativeProviderError> {
    let package = &declaration.package;
    let (Some(repo), Some(tag), Some(binary)) = (
        locked.repo.as_deref(),
        locked.tag.as_deref(),
        locked.binary.as_deref(),
    ) else {
        return Err(NativeProviderError::LockMismatch {
            package: package.clone(),
            message: "github entry lacks its repo pins; re-pin".to_owned(),
        });
    };
    // The lockfile was checked against the declaration, so its
    // coordinates are authoritative here.
    let mut wanted = Vec::with_capacity(GITHUB_COMPANION_FILES.len() + 1);
    wanted.extend(GITHUB_COMPANION_FILES.iter().map(ToString::to_string));
    wanted.push(format!("{binary}.{target}.node"));
    let mut staged = Vec::with_capacity(wanted.len() + 1);
    for file in &wanted {
        if !locked.assets.contains_key(file) {
            if file.ends_with(".node") {
                return Err(NativeProviderError::NoBinaryForTarget {
                    package: package.clone(),
                    target: target.to_owned(),
                });
            }
            return Err(NativeProviderError::PinMissing {
                package: package.clone(),
                file: file.clone(),
            });
        }
        let url = github_asset_url(endpoints, repo, tag, file);
        let bytes = fetch_capped(client, &url, MAX_DOWNLOAD_BYTES).await?;
        verify_asset(package, locked, file, &bytes)?;
        write_source_file(dest, file, &bytes)?;
        staged.push(file.clone());
    }
    write_source_file(
        dest,
        "package.json",
        github_package_manifest(declaration, binary).as_bytes(),
    )?;
    staged.push("package.json".to_owned());
    Ok(staged)
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

/// Resolve `target`: an explicit triple, or the host triple when
/// omitted. Unknown triples fail; nothing is ever guessed.
pub fn resolve_target(target: Option<&str>) -> Result<String, NativeProviderError> {
    match target {
        Some(triple) if is_known_napi_target(triple) => Ok(triple.to_owned()),
        Some(triple) => Err(NativeProviderError::UnknownTarget {
            target: triple.to_owned(),
        }),
        None => Ok(current_napi_target()),
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

/// Fetch every `nativeLibs` entry (optionally filtered to one provider)
/// for `target` and stage each tree into its `nativeAddons` root under
/// `plugin_dir`. Each library needs a lockfile entry and a same-package
/// `nativeAddons` twin; anything missing fails the whole operation
/// before anything is staged. `work_dir` holds transient downloads and
/// is cleaned afterwards.
pub async fn fetch_and_stage_native_libs(
    plan: &FetchAndStage<'_>,
) -> Result<Vec<NativeFetchReport>, NativeProviderError> {
    if !is_known_napi_target(plan.target) {
        return Err(NativeProviderError::UnknownTarget {
            target: plan.target.to_owned(),
        });
    }
    let selected = |entry: &&NativeLibDeclaration| {
        plan.provider_filter
            .is_none_or(|provider| entry.provider == provider)
    };
    // Validate the full plan before touching the plugin directory.
    for declaration in plan.manifest.native_libs.iter().filter(selected) {
        let locked = plan
            .lockfile
            .packages
            .get(&declaration.package)
            .ok_or_else(|| NativeProviderError::LockMissing(declaration.package.clone()))?;
        check_lock_matches(&declaration.package, declaration, locked)?;
        if !plan
            .manifest
            .native_addons
            .iter()
            .any(|addon| addon.package == declaration.package)
        {
            return Err(NativeProviderError::MissingAddonRoot {
                package: declaration.package.clone(),
            });
        }
    }
    let fetch_root = plan.work_dir.join("tiktools-native-fetch");
    let _ = std::fs::remove_dir_all(&fetch_root);
    let mut reports = Vec::new();
    for declaration in plan.manifest.native_libs.iter().filter(selected) {
        let locked = plan
            .lockfile
            .packages
            .get(&declaration.package)
            .expect("lockfile entry validated above");
        let root = plan
            .manifest
            .native_addons
            .iter()
            .find(|addon| addon.package == declaration.package)
            .expect("addon root validated above")
            .root
            .clone();
        let source = fetch_root.join(&declaration.package);
        let _ = std::fs::remove_dir_all(&source);
        let staged = fetch_native_lib(
            plan.client,
            plan.endpoints,
            declaration,
            locked,
            plan.target,
            &source,
        )
        .await
        .and_then(|_| {
            stage_native_package(&NativeStageRequest {
                package: &declaration.package,
                source: &source,
                plugin_dir: plan.plugin_dir,
                root: &root,
                overwrite: plan.overwrite,
            })
            .map_err(|error| NativeProviderError::Stage {
                package: declaration.package.clone(),
                message: error.to_string(),
            })
        });
        let _ = std::fs::remove_dir_all(&source);
        let report = staged?;
        reports.push(NativeFetchReport {
            package: declaration.package.clone(),
            version: declaration.version.clone(),
            target: plan.target.to_owned(),
            root,
            files: report.files,
        });
    }
    let _ = std::fs::remove_dir_all(&fetch_root);
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        task::JoinHandle,
    };

    static STUB_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn scratch(label: &str) -> PathBuf {
        let id = STUB_COUNTER.fetch_add(1, Ordering::AcqRel);
        let root = std::env::temp_dir().join(format!(
            "tiktools-native-providers-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    /// Minimal stub origin. Routes are filled after binding (metadata
    /// documents embed the base URL); absent paths 404. Mirrors the
    /// declarative-HTTP test stub: one read per connection,
    /// `connection: close` framing.
    async fn stub_origin() -> (
        String,
        Arc<std::sync::Mutex<BTreeMap<String, Vec<u8>>>>,
        JoinHandle<()>,
    ) {
        let routes = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
        let serving = routes.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let routes = serving.clone();
                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 8192];
                    let Ok(read) = socket.read(&mut buffer).await else {
                        return;
                    };
                    let request = String::from_utf8_lossy(&buffer[..read]);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");
                    let path = path.split('?').next().unwrap_or(path);
                    let (status, body) = routes
                        .lock()
                        .ok()
                        .and_then(|routes| routes.get(path).cloned())
                        .map_or_else(|| (404, b"not found".to_vec()), |body| (200, body));
                    let reason = if status == 200 { "OK" } else { "Not Found" };
                    let header = format!(
                        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/octet-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = socket.write_all(header.as_bytes()).await;
                    let _ = socket.write_all(&body).await;
                });
            }
        });
        (base, routes, task)
    }

    /// Build a gzip tarball from `(archive_path, bytes)` entries.
    fn make_tarball(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut builder = tar::Builder::new(encoder);
        for (path, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *bytes).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn ssri_sha512(bytes: &[u8]) -> String {
        use base64::Engine;
        format!(
            "sha512-{}",
            base64::engine::general_purpose::STANDARD.encode(Sha512::digest(bytes))
        )
    }

    fn npm_fixture_tarball() -> Vec<u8> {
        make_tarball(&[
            (
                "package/package.json",
                br#"{"name":"fixture-pkg","version":"1.2.3","main":"index.js","files":["index.js","*.node"]}"#,
            ),
            ("package/index.js", b"module.exports = {};"),
            ("package/node-stem.linux-x64-gnu.node", b"gnu-bytes"),
            ("package/node-stem.linux-x64-musl.node", b"musl-bytes"),
        ])
    }

    fn npm_declaration() -> NativeLibDeclaration {
        NativeLibDeclaration {
            package: "fixture-pkg".to_owned(),
            version: "1.2.3".to_owned(),
            provider: NativeLibProvider::Npm,
            repo: None,
            tag: None,
            binary: None,
        }
    }

    fn github_declaration() -> NativeLibDeclaration {
        NativeLibDeclaration {
            package: "gh-pkg".to_owned(),
            version: "0.3.0".to_owned(),
            provider: NativeLibProvider::Github,
            repo: Some("owner/gh-pkg".to_owned()),
            tag: Some("v0.3.0".to_owned()),
            binary: Some("node-stem".to_owned()),
        }
    }

    #[test]
    fn target_matcher_requires_dot_boundary_and_top_level() {
        assert!(node_serves_target(
            "node-stem.linux-x64-gnu.node",
            "linux-x64-gnu"
        ));
        assert!(!node_serves_target(
            "node-stem.linux-x64-musl.node",
            "linux-x64-gnu"
        ));
        // No dot boundary: a longer triple ending in the target name.
        assert!(!node_serves_target(
            "stemxlinux-x64-gnu.node",
            "linux-x64-gnu"
        ));
        // Subdirectory binaries never match: the loader selects top-level.
        assert!(!node_serves_target(
            "dist/node-stem.linux-x64-gnu.node",
            "linux-x64-gnu"
        ));
        assert!(!node_serves_target("index.js", "linux-x64-gnu"));
    }

    #[test]
    fn integrity_and_pin_verifiers_fail_closed() {
        let bytes = b"payload";
        assert!(verify_integrity(bytes, &ssri_sha512(bytes)));
        assert!(!verify_integrity(bytes, "sha512-broken!!!"));
        assert!(!verify_integrity(bytes, "md5-AAAAAAAAAAAAAAAAAAAAAA=="));
        assert!(!verify_integrity(bytes, "not-an-integrity-value"));
        assert!(!verify_integrity(b"other", &ssri_sha512(bytes)));
        let pin = sha256_pin(bytes);
        assert!(verify_pin(bytes, &pin));
        assert!(!verify_pin(b"other", &pin));
        assert!(!verify_pin(bytes, "sha256:xyz"));
        assert!(!verify_pin(bytes, "md5:d41d8cd98f00b204e9800998ecf8427e"));
    }

    #[test]
    fn lockfile_roundtrips_and_rejects_versions() {
        let root = scratch("lock");
        let path = root.join(NATIVE_LIBS_LOCKFILE);
        let mut packages = BTreeMap::new();
        packages.insert(
            "pkg".to_owned(),
            LockedNativeLib {
                version: "1.0.0".to_owned(),
                provider: NativeLibProvider::Npm,
                tarball: Some("https://example.invalid/pkg.tgz".to_owned()),
                integrity: Some("sha512-AAAA".to_owned()),
                repo: None,
                tag: None,
                binary: None,
                assets: BTreeMap::from([(
                    "node-stem.linux-x64-gnu.node".to_owned(),
                    sha256_pin(b"x"),
                )]),
            },
        );
        let lockfile = NativeLibsLockfile {
            version: LOCKFILE_VERSION,
            packages,
        };
        write_lockfile(&path, &lockfile).unwrap();
        assert_eq!(read_lockfile(&path).unwrap(), lockfile);
        std::fs::write(&path, r#"{"version":999,"packages":{}}"#).unwrap();
        assert!(matches!(
            read_lockfile(&path),
            Err(NativeProviderError::LockVersion { version: 999, .. })
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Patch the first header's name field to an evil path the safe
    /// builder API refuses to emit, then repair the header checksum so
    /// the archive parses and the extractor's own guards are tested.
    fn make_evil_tarball(evil: &str) -> Vec<u8> {
        let placeholder = "package/placeholder-padded-to-cover-evil-paths.node";
        assert!(evil.len() < placeholder.len());
        let tarball = make_tarball(&[(placeholder, b"x" as &[u8])]);
        // Decompress, patch the raw header, recompress.
        let mut decoder = flate2::read::GzDecoder::new(tarball.as_slice());
        let mut raw = Vec::new();
        use std::io::Read;
        decoder.read_to_end(&mut raw).unwrap();
        assert!(raw.len() >= 512);
        raw[..placeholder.len()].fill(0);
        raw[..evil.len()].copy_from_slice(evil.as_bytes());
        // Recompute the ustar checksum: sum with the chksum field blanked.
        raw[148..156].fill(b' ');
        let sum: u32 = raw[..512].iter().map(|byte| *byte as u32).sum();
        let checksum = format!("{sum:06o}\0 ");
        raw[148..156].copy_from_slice(checksum.as_bytes());
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        use std::io::Write;
        encoder.write_all(&raw).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn tarball_extraction_rejects_evil_entries() {
        // Baseline: the fixture extracts.
        let files = extract_npm_tarball("pkg", &npm_fixture_tarball()).unwrap();
        assert_eq!(files.len(), 4);
        // Prefix escapes are expressible through the safe builder.
        let tarball = make_tarball(&[("elsewhere/file.node", b"x" as &[u8])]);
        assert!(extract_npm_tarball("pkg", &tarball).is_err());
        // Traversal and absolute paths need raw headers: the builder
        // refuses to emit them, so patch the bytes directly.
        for evil in [
            "package/../escape.node",
            "/absolute.node",
            "package/sub/../../escape.node",
        ] {
            let tarball = make_evil_tarball(evil);
            assert!(
                extract_npm_tarball("pkg", &tarball).is_err(),
                "{evil} should be rejected"
            );
        }
        // Symlinks are non-regular files: rejected without following.
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_cksum();
        builder
            .append_link(&mut header, "package/link.node", "package/real.node")
            .unwrap();
        let tarball = builder.into_inner().unwrap().finish().unwrap();
        assert!(extract_npm_tarball("pkg", &tarball).is_err());
    }

    #[tokio::test]
    async fn npm_pin_then_fetch_roundtrip() {
        let tarball = npm_fixture_tarball();
        let integrity = ssri_sha512(&tarball);
        let (base, routes, task) = stub_origin().await;
        routes.lock().unwrap().extend([
            (
                "/fixture-pkg/1.2.3".to_owned(),
                serde_json::to_vec(&serde_json::json!({
                    "name": "fixture-pkg",
                    "version": "1.2.3",
                    "dist": {
                        "tarball": format!("{base}/tarballs/fixture-pkg-1.2.3.tgz"),
                        "integrity": integrity,
                    },
                }))
                .unwrap(),
            ),
            ("/tarballs/fixture-pkg-1.2.3.tgz".to_owned(), tarball),
        ]);
        let endpoints = ProviderEndpoints {
            npm_registry: base.clone(),
            github_base: base,
        };
        let client = provider_client().unwrap();
        let declaration = npm_declaration();
        let locked = pin_native_lib(&client, &endpoints, &declaration)
            .await
            .unwrap();
        assert_eq!(locked.version, "1.2.3");
        assert_eq!(locked.provider, NativeLibProvider::Npm);
        assert_eq!(locked.assets.len(), 2);
        assert_eq!(
            locked.assets.get("node-stem.linux-x64-gnu.node").unwrap(),
            &sha256_pin(b"gnu-bytes")
        );

        let root = scratch("npm-fetch");
        let dest = root.join("source");
        let staged = fetch_native_lib(
            &client,
            &endpoints,
            &declaration,
            &locked,
            "linux-x64-gnu",
            &dest,
        )
        .await
        .unwrap();
        assert!(staged.contains(&"package.json".to_owned()));
        assert!(staged.contains(&"node-stem.linux-x64-gnu.node".to_owned()));
        assert!(!staged.iter().any(|file| file.contains("musl")));
        assert!(!dest.join("node-stem.linux-x64-musl.node").exists());

        // A target the tarball does not serve fails instead of staging
        // a binary-less tree.
        let missing = fetch_native_lib(
            &client,
            &endpoints,
            &declaration,
            &locked,
            "win32-x64-msvc",
            &root.join("missing"),
        )
        .await;
        assert!(
            matches!(missing, Err(NativeProviderError::NoBinaryForTarget { .. })),
            "{missing:?}"
        );
        // Unknown triples never reach the network.
        assert!(matches!(
            fetch_native_lib(
                &client,
                &endpoints,
                &declaration,
                &locked,
                "fuchsia-arm64",
                &root.join("nope"),
            )
            .await,
            Err(NativeProviderError::UnknownTarget { .. })
        ));
        task.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn github_pin_then_fetch_roundtrip() {
        let (base, routes, task) = stub_origin().await;
        let prefix = "/owner/gh-pkg/releases/download/v0.3.0";
        routes.lock().unwrap().extend([
            (
                format!("{prefix}/index.js"),
                b"module.exports = {};".to_vec(),
            ),
            (format!("{prefix}/index.d.ts"), b"export {};".to_vec()),
            (
                format!("{prefix}/node-stem.linux-x64-gnu.node"),
                b"gnu-bytes".to_vec(),
            ),
            (
                format!("{prefix}/node-stem.win32-x64-msvc.node"),
                b"win-bytes".to_vec(),
            ),
        ]);
        let endpoints = ProviderEndpoints {
            npm_registry: base.clone(),
            github_base: base,
        };
        let client = provider_client().unwrap();
        let declaration = github_declaration();
        // Pinning probes every known target; absent assets 404 and are
        // skipped, the rest are hashed.
        let locked = pin_native_lib(&client, &endpoints, &declaration)
            .await
            .unwrap();
        assert_eq!(locked.provider, NativeLibProvider::Github);
        assert_eq!(locked.assets.len(), 4);
        assert_eq!(
            locked.assets.get("index.js").unwrap(),
            &sha256_pin(b"module.exports = {};")
        );

        let root = scratch("gh-fetch");
        let dest = root.join("source");
        let staged = fetch_native_lib(
            &client,
            &endpoints,
            &declaration,
            &locked,
            "linux-x64-gnu",
            &dest,
        )
        .await
        .unwrap();
        assert_eq!(staged.len(), 4);
        assert!(dest.join("node-stem.linux-x64-gnu.node").is_file());
        assert!(!dest.join("node-stem.win32-x64-msvc.node").exists());
        let manifest = std::fs::read_to_string(dest.join("package.json")).unwrap();
        assert!(manifest.contains("\"binaryName\": \"node-stem\""));
        assert!(manifest.contains("\"name\": \"gh-pkg\""));

        // An unpinned target fails before any download for it.
        let missing = fetch_native_lib(
            &client,
            &endpoints,
            &declaration,
            &locked,
            "darwin-arm64",
            &root.join("missing"),
        )
        .await;
        assert!(
            matches!(missing, Err(NativeProviderError::NoBinaryForTarget { .. })),
            "{missing:?}"
        );
        task.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn fetch_rejects_tampered_bytes() {
        // npm: the tarball no longer matches its integrity pin.
        let tarball = npm_fixture_tarball();
        let integrity = ssri_sha512(&tarball);
        let (base, routes, task) = stub_origin().await;
        routes.lock().unwrap().extend([
            (
                "/fixture-pkg/1.2.3".to_owned(),
                serde_json::to_vec(&serde_json::json!({
                    "name": "fixture-pkg",
                    "version": "1.2.3",
                    "dist": {
                        "tarball": format!("{base}/t.tgz"),
                        "integrity": integrity,
                    },
                }))
                .unwrap(),
            ),
            ("/t.tgz".to_owned(), tarball),
        ]);
        let endpoints = ProviderEndpoints {
            npm_registry: base.clone(),
            github_base: base,
        };
        let client = provider_client().unwrap();
        let declaration = npm_declaration();
        let locked = pin_native_lib(&client, &endpoints, &declaration)
            .await
            .unwrap();
        routes
            .lock()
            .unwrap()
            .insert("/t.tgz".to_owned(), b"tampered".to_vec());
        let root = scratch("tamper");
        let result = fetch_native_lib(
            &client,
            &endpoints,
            &declaration,
            &locked,
            "linux-x64-gnu",
            &root.join("npm"),
        )
        .await;
        assert!(
            matches!(result, Err(NativeProviderError::IntegrityMismatch { .. })),
            "{result:?}"
        );

        // github: one asset no longer matches its pin.
        let prefix = "/o/p/releases/download/v1";
        routes.lock().unwrap().extend([
            (format!("{prefix}/index.js"), b"js".to_vec()),
            (format!("{prefix}/index.d.ts"), b"dts".to_vec()),
            (
                format!("{prefix}/node-stem.linux-x64-gnu.node"),
                b"node".to_vec(),
            ),
        ]);
        let declaration = NativeLibDeclaration {
            package: "p".to_owned(),
            version: "1.0.0".to_owned(),
            provider: NativeLibProvider::Github,
            repo: Some("o/p".to_owned()),
            tag: Some("v1".to_owned()),
            binary: Some("node-stem".to_owned()),
        };
        let locked = pin_native_lib(&client, &endpoints, &declaration)
            .await
            .unwrap();
        routes.lock().unwrap().insert(
            format!("{prefix}/node-stem.linux-x64-gnu.node"),
            b"tampered".to_vec(),
        );
        let result = fetch_native_lib(
            &client,
            &endpoints,
            &declaration,
            &locked,
            "linux-x64-gnu",
            &root.join("gh"),
        )
        .await;
        assert!(
            matches!(result, Err(NativeProviderError::PinMismatch { .. })),
            "{result:?}"
        );
        task.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn fetch_rejects_lock_drift_and_missing_entries() {
        let (base, routes, task) = stub_origin().await;
        routes.lock().unwrap().extend([
            (
                "/fixture-pkg/1.2.3".to_owned(),
                serde_json::to_vec(&serde_json::json!({
                    "name": "fixture-pkg",
                    "version": "1.2.3",
                    "dist": {
                        "tarball": format!("{base}/t.tgz"),
                        "integrity": ssri_sha512(&npm_fixture_tarball()),
                    },
                }))
                .unwrap(),
            ),
            ("/t.tgz".to_owned(), npm_fixture_tarball()),
        ]);
        let endpoints = ProviderEndpoints {
            npm_registry: base.clone(),
            github_base: base,
        };
        let client = provider_client().unwrap();
        let declaration = npm_declaration();
        let locked = pin_native_lib(&client, &endpoints, &declaration)
            .await
            .unwrap();
        let root = scratch("drift");
        // A version bump without re-pinning fails before downloading.
        let bumped = NativeLibDeclaration {
            version: "1.2.4".to_owned(),
            ..declaration.clone()
        };
        assert!(matches!(
            fetch_native_lib(
                &client,
                &endpoints,
                &bumped,
                &locked,
                "linux-x64-gnu",
                &root.join("bumped"),
            )
            .await,
            Err(NativeProviderError::LockMismatch { .. })
        ));
        // A provider flip without re-pinning fails the same way.
        let flipped = NativeLibDeclaration {
            provider: NativeLibProvider::Github,
            repo: Some("o/p".to_owned()),
            tag: Some("v1".to_owned()),
            binary: Some("node-stem".to_owned()),
            ..declaration.clone()
        };
        assert!(matches!(
            fetch_native_lib(
                &client,
                &endpoints,
                &flipped,
                &locked,
                "linux-x64-gnu",
                &root.join("flipped"),
            )
            .await,
            Err(NativeProviderError::LockMismatch { .. })
        ));
        // Orchestration without a lockfile entry fails before staging.
        let manifest = PluginManifest::from_json_str(
            r#"{"schemaVersion":3,"id":"xx","name":"X","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js","nativeAddons":[{"package":"fixture-pkg","root":"node_modules/fixture-pkg"}],"nativeLibs":[{"package":"fixture-pkg","version":"1.2.3"}]}"#,
        )
        .unwrap();
        let empty = NativeLibsLockfile {
            version: LOCKFILE_VERSION,
            packages: BTreeMap::new(),
        };
        let plugin_dir = root.join("plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        assert!(matches!(
            fetch_and_stage_native_libs(&FetchAndStage {
                client: &client,
                endpoints: &endpoints,
                manifest: &manifest,
                lockfile: &empty,
                target: "linux-x64-gnu",
                plugin_dir: &plugin_dir,
                work_dir: &root.join("work"),
                overwrite: true,
                provider_filter: None,
            })
            .await,
            Err(NativeProviderError::LockMissing(_))
        ));
        assert!(!plugin_dir.join("node_modules").exists());
        task.abort();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn orchestration_stages_into_addon_roots() {
        let tarball = npm_fixture_tarball();
        let (base, routes, task) = stub_origin().await;
        routes.lock().unwrap().extend([
            (
                "/fixture-pkg/1.2.3".to_owned(),
                serde_json::to_vec(&serde_json::json!({
                    "name": "fixture-pkg",
                    "version": "1.2.3",
                    "dist": {
                        "tarball": format!("{base}/t.tgz"),
                        "integrity": ssri_sha512(&tarball),
                    },
                }))
                .unwrap(),
            ),
            ("/t.tgz".to_owned(), tarball),
        ]);
        let endpoints = ProviderEndpoints {
            npm_registry: base.clone(),
            github_base: base,
        };
        let client = provider_client().unwrap();
        let manifest = PluginManifest::from_json_str(
            r#"{"schemaVersion":3,"id":"xx","name":"X","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js","nativeAddons":[{"package":"fixture-pkg","root":"node_modules/fixture-pkg"}],"nativeLibs":[{"package":"fixture-pkg","version":"1.2.3"}]}"#,
        )
        .unwrap();
        let locked = pin_native_lib(&client, &endpoints, &manifest.native_libs[0])
            .await
            .unwrap();
        let lockfile = NativeLibsLockfile {
            version: LOCKFILE_VERSION,
            packages: BTreeMap::from([("fixture-pkg".to_owned(), locked)]),
        };
        let root = scratch("orchestrate");
        let plugin_dir = root.join("plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        let reports = fetch_and_stage_native_libs(&FetchAndStage {
            client: &client,
            endpoints: &endpoints,
            manifest: &manifest,
            lockfile: &lockfile,
            target: "linux-x64-gnu",
            plugin_dir: &plugin_dir,
            work_dir: &root.join("work"),
            overwrite: false,
            provider_filter: None,
        })
        .await
        .unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].package, "fixture-pkg");
        assert_eq!(reports[0].target, "linux-x64-gnu");
        assert_eq!(reports[0].root, "node_modules/fixture-pkg");
        // The staged tree went through native_stage validation: name
        // matched, the target binary is present, the twin is not.
        let staged = plugin_dir.join("node_modules/fixture-pkg");
        assert!(staged.join("package.json").is_file());
        assert!(staged.join("node-stem.linux-x64-gnu.node").is_file());
        assert!(!staged.join("node-stem.linux-x64-musl.node").exists());
        // Transient downloads are cleaned.
        assert!(!root.join("work/tiktools-native-fetch").exists());

        // A library without a nativeAddons twin fails before staging.
        let unrooted = PluginManifest::from_json_str(
            r#"{"schemaVersion":3,"id":"xx","name":"X","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js","nativeLibs":[{"package":"fixture-pkg","version":"1.2.3"}]}"#,
        )
        .unwrap();
        assert!(matches!(
            fetch_and_stage_native_libs(&FetchAndStage {
                client: &client,
                endpoints: &endpoints,
                manifest: &unrooted,
                lockfile: &lockfile,
                target: "linux-x64-gnu",
                plugin_dir: &root.join("plugin2"),
                work_dir: &root.join("work2"),
                overwrite: false,
                provider_filter: None,
            })
            .await,
            Err(NativeProviderError::MissingAddonRoot { .. })
        ));
        task.abort();
        let _ = std::fs::remove_dir_all(&root);
    }
}
