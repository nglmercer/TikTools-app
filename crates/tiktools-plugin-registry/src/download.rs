use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime},
};

use reqwest::{header, Client, StatusCode};
use tiktools_plugin_package::{Sha256Digest, PACKAGE_EXTENSION};

use crate::{url_policy::checked_url, RegistryError};

pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone)]
pub struct DownloadRequest<'a> {
    pub url: &'a str,
    pub expected_digest: Option<&'a Sha256Digest>,
    pub expected_size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Downloaded {
    pub path: PathBuf,
    pub digest: Sha256Digest,
    pub size: u64,
    pub cached: bool,
}

#[derive(Debug, Clone)]
pub struct Downloader {
    client: Client,
    cache_dir: PathBuf,
    max_bytes: u64,
}

impl Downloader {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        let client = tiktools_plugin_sdk::http::client_builder()
            .map_err(|message| RegistryError::http("", message))?
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(DOWNLOAD_TIMEOUT)
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(format!(
                "tiktools-plugin-registry/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|error| RegistryError::http("", error))?;
        Ok(Self::with_client(client, cache_dir))
    }

    pub fn with_client(client: Client, cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            client,
            cache_dir: cache_dir.into(),
            max_bytes: MAX_DOWNLOAD_BYTES,
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn cached_path(&self, digest: &Sha256Digest) -> PathBuf {
        let hex = digest.as_hex();
        self.cache_dir
            .join("sha256")
            .join(&hex[..2])
            .join(format!("{hex}.{PACKAGE_EXTENSION}"))
    }

    pub async fn fetch(
        &self,
        request: DownloadRequest<'_>,
        progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
        cancel: &AtomicBool,
    ) -> Result<Downloaded, RegistryError> {
        checked_url(request.url)?;
        if let Some(digest) = request.expected_digest {
            let cached = self.cached_path(digest);
            if let Some(hit) = self.cache_hit(&cached, digest, request.expected_size)? {
                progress(hit.size, Some(hit.size));
                return Ok(hit);
            }
        }
        let partial_dir = self.cache_dir.join("partial");
        fs::create_dir_all(&partial_dir)?;
        let partial = partial_dir.join(format!(
            "{}.part",
            Sha256Digest::of_bytes(request.url.as_bytes()).as_hex()
        ));
        let result = self.stream(&request, &partial, progress, cancel).await;
        let size = match result {
            Ok(size) => size,
            Err(error) => {
                if !matches!(error, RegistryError::Http { .. } | RegistryError::Cancelled) {
                    let _ = fs::remove_file(&partial);
                }
                return Err(error);
            }
        };
        if let Some(expected) = request.expected_size {
            if size != expected {
                let _ = fs::remove_file(&partial);
                return Err(RegistryError::SizeMismatch {
                    expected,
                    actual: size,
                });
            }
        }
        let digest = Sha256Digest::of_file(&partial)?;
        if let Some(expected) = request.expected_digest {
            if &digest != expected {
                let _ = fs::remove_file(&partial);
                return Err(RegistryError::DigestMismatch {
                    expected: expected.to_string(),
                    actual: digest.to_string(),
                });
            }
        }
        let destination = self.cached_path(&digest);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        if destination.exists() {
            fs::remove_file(&partial)?;
        } else {
            fs::rename(&partial, &destination)?;
        }
        Ok(Downloaded {
            path: destination,
            digest,
            size,
            cached: false,
        })
    }

    pub fn prune(&self, max_total_bytes: u64) -> Result<u64, RegistryError> {
        let root = self.cache_dir.join("sha256");
        let mut entries = Vec::new();
        if root.is_dir() {
            for shard in fs::read_dir(&root)? {
                let shard = shard?.path();
                if !shard.is_dir() {
                    continue;
                }
                for file in fs::read_dir(&shard)? {
                    let path = file?.path();
                    let metadata = fs::metadata(&path)?;
                    if metadata.is_file() {
                        let used = metadata
                            .accessed()
                            .or_else(|_| metadata.modified())
                            .unwrap_or(SystemTime::UNIX_EPOCH);
                        entries.push((used, metadata.len(), path));
                    }
                }
            }
        }
        let mut total: u64 = entries.iter().map(|(_, size, _)| size).sum();
        entries.sort_by_key(|(used, _, _)| *used);
        let mut freed = 0;
        for (_, size, path) in entries {
            if total <= max_total_bytes {
                break;
            }
            fs::remove_file(&path)?;
            total = total.saturating_sub(size);
            freed += size;
        }
        Ok(freed)
    }

    fn cache_hit(
        &self,
        path: &Path,
        digest: &Sha256Digest,
        expected_size: Option<u64>,
    ) -> Result<Option<Downloaded>, RegistryError> {
        let Ok(metadata) = fs::metadata(path) else {
            return Ok(None);
        };
        if expected_size.is_some_and(|size| size != metadata.len()) {
            fs::remove_file(path)?;
            return Ok(None);
        }
        if &Sha256Digest::of_file(path)? != digest {
            fs::remove_file(path)?;
            return Ok(None);
        }
        Ok(Some(Downloaded {
            path: path.to_owned(),
            digest: digest.clone(),
            size: metadata.len(),
            cached: true,
        }))
    }

    async fn stream(
        &self,
        request: &DownloadRequest<'_>,
        partial: &Path,
        progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
        cancel: &AtomicBool,
    ) -> Result<u64, RegistryError> {
        let existing = fs::metadata(partial)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let mut builder = self.client.get(request.url);
        if existing > 0 {
            builder = builder.header(header::RANGE, format!("bytes={existing}-"));
        }
        let mut response = builder
            .send()
            .await
            .map_err(|error| RegistryError::http(request.url, error))?;
        let status = response.status();
        let resumed = status == StatusCode::PARTIAL_CONTENT && existing > 0;
        if !status.is_success() {
            if status == StatusCode::RANGE_NOT_SATISFIABLE {
                let _ = fs::remove_file(partial);
            }
            return Err(RegistryError::Status {
                url: request.url.to_owned(),
                status: status.as_u16(),
            });
        }
        let offset = if resumed { existing } else { 0 };
        let total = response
            .content_length()
            .map(|length| length + offset)
            .or(request.expected_size);
        if total.is_some_and(|total| total > self.max_bytes) {
            return Err(RegistryError::TooLarge(self.max_bytes));
        }
        let mut file = if resumed {
            fs::OpenOptions::new().append(true).open(partial)?
        } else {
            fs::File::create(partial)?
        };
        let mut written = offset;
        progress(written, total);
        loop {
            if cancel.load(Ordering::Acquire) {
                file.flush()?;
                return Err(RegistryError::Cancelled);
            }
            let chunk = response
                .chunk()
                .await
                .map_err(|error| RegistryError::http(request.url, error))?;
            let Some(chunk) = chunk else {
                break;
            };
            written += chunk.len() as u64;
            if written > self.max_bytes {
                drop(file);
                let _ = fs::remove_file(partial);
                return Err(RegistryError::TooLarge(self.max_bytes));
            }
            file.write_all(&chunk)?;
            progress(written, total);
        }
        file.flush()?;
        file.sync_all().or_else(|error| match error.kind() {
            io::ErrorKind::Unsupported => Ok(()),
            _ => Err(error),
        })?;
        Ok(written)
    }
}
