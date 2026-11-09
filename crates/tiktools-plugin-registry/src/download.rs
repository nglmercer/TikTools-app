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
    fixed_origin: bool,
}

impl Downloader {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        Self::new_with_policy(cache_dir, false)
    }

    /// Product downloader: redirects may not leave the initial Cloud origin.
    /// The caller must also validate that the initial URL is the configured
    /// Cloud instance; this restriction alone does not choose an origin.
    pub fn new_fixed(cache_dir: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        Self::new_with_policy(cache_dir, true)
    }

    fn new_with_policy(
        cache_dir: impl Into<PathBuf>,
        fixed_origin: bool,
    ) -> Result<Self, RegistryError> {
        let redirects = if fixed_origin {
            reqwest::redirect::Policy::custom(|attempt| {
                let same_origin = attempt.previous().first().is_some_and(|first| {
                    first.origin() == attempt.url().origin()
                        && attempt.url().username().is_empty()
                        && attempt.url().password().is_none()
                });
                if same_origin && attempt.previous().len() < 5 {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            })
        } else {
            reqwest::redirect::Policy::limited(5)
        };
        let client = tiktools_plugin_sdk::http::client_builder()
            .map_err(|message| RegistryError::http("", message))?
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(DOWNLOAD_TIMEOUT)
            .redirect(redirects)
            .user_agent(format!(
                "tiktools-plugin-registry/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|error| RegistryError::http("", error))?;
        let mut downloader = Self::with_client(client, cache_dir);
        downloader.fixed_origin = fixed_origin;
        Ok(downloader)
    }

    pub fn with_client(client: Client, cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            client,
            cache_dir: cache_dir.into(),
            max_bytes: MAX_DOWNLOAD_BYTES,
            fixed_origin: false,
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Whether this instance was built with the fixed-origin redirect policy.
    pub fn enforces_fixed_origin(&self) -> bool {
        self.fixed_origin
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
        let parsed = checked_url(request.url)?;
        if self.fixed_origin && (!parsed.username().is_empty() || parsed.password().is_some()) {
            return Err(RegistryError::FixedSources);
        }
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

#[cfg(test)]
mod fixed_tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn temp_cache() -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "tiktools-fixed-download-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ))
    }

    #[tokio::test]
    async fn fixed_downloader_refuses_cross_origin_redirect_without_contacting_it() {
        let foreign = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let foreign_addr = foreign.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let counter = requests.clone();
        let foreign_task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = foreign.accept().await {
                counter.fetch_add(1, Ordering::AcqRel);
                let _ = socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .await;
            }
        });
        let origin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin_addr = origin.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = origin.accept().await.unwrap();
            let mut buffer = [0u8; 2048];
            let _ = socket.read(&mut buffer).await;
            let response = format!("HTTP/1.1 302 Found\r\nLocation: http://{foreign_addr}/archive.plugin\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let cache = temp_cache();
        let downloader = Downloader::new_fixed(&cache).unwrap();
        let url = format!("http://{origin_addr}/packages/acme.plugin/a.plugin");
        let result = downloader
            .fetch(
                DownloadRequest {
                    url: &url,
                    expected_digest: None,
                    expected_size: None,
                },
                &|_, _| {},
                &AtomicBool::new(false),
            )
            .await;
        assert!(result.is_err());
        server.await.unwrap();
        assert_eq!(requests.load(Ordering::Acquire), 0);
        foreign_task.abort();
        let _ = fs::remove_dir_all(cache);
    }

    #[tokio::test]
    async fn fixed_downloader_allows_same_origin_redirect() {
        let origin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = origin.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for index in 0..2 {
                let (mut socket, _) = origin.accept().await.unwrap();
                let mut buffer = [0u8; 2048];
                let _ = socket.read(&mut buffer).await;
                let response: &[u8] = if index == 0 {
                    b"HTTP/1.1 302 Found\r\nLocation: /package.plugin\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                } else {
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
                };
                socket.write_all(response).await.unwrap();
            }
        });
        let cache = temp_cache();
        let downloader = Downloader::new_fixed(&cache).unwrap();
        let url = format!("http://{address}/archive.plugin");
        let result = downloader
            .fetch(
                DownloadRequest {
                    url: &url,
                    expected_digest: None,
                    expected_size: None,
                },
                &|_, _| {},
                &AtomicBool::new(false),
            )
            .await
            .unwrap();
        assert_eq!(fs::read(result.path).unwrap(), b"ok");
        server.await.unwrap();
        let _ = fs::remove_dir_all(cache);
    }
}
