//! Bounded download and signature verification compatible with packager 0.2.3.
//!
//! Upstream's download implementation buffers without a size limit. Keep the
//! limit here and use the same minisign verification algorithm and key format.
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use minisign_verify::{PublicKey, Signature};
use thiserror::Error;

use crate::AuthenticatedRelease;

pub const MAX_UPDATE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_KEY_OR_SIGNATURE_LENGTH: usize = 16 * 1024;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("no newer update is available")]
    NotAvailable,
    #[error("update request failed")]
    Network,
    #[error("update exceeds the download size limit")]
    TooLarge,
    #[error("update download was truncated")]
    Truncated,
    #[error("update signature verification failed")]
    InvalidSignature,
}

/// Only produced after verification. Its contents cannot be modified by a
/// caller before being handed to the platform installation layer.
pub struct VerifiedPackage {
    bytes: Vec<u8>,
}

impl VerifiedPackage {
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    // Kept private until the lifecycle integration can guarantee that the
    // updater's process-exiting install call happens after graceful shutdown.
    #[allow(dead_code)]
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

pub(crate) fn verify(bytes: &[u8], signature: &str, public_key: &str) -> Result<(), DownloadError> {
    if signature.len() > MAX_KEY_OR_SIGNATURE_LENGTH
        || public_key.len() > MAX_KEY_OR_SIGNATURE_LENGTH
    {
        return Err(DownloadError::InvalidSignature);
    }
    let decode = |value: &str| -> Result<String, DownloadError> {
        let bytes = STANDARD
            .decode(value.trim())
            .map_err(|_| DownloadError::InvalidSignature)?;
        String::from_utf8(bytes).map_err(|_| DownloadError::InvalidSignature)
    };
    let key =
        PublicKey::decode(&decode(public_key)?).map_err(|_| DownloadError::InvalidSignature)?;
    let signature =
        Signature::decode(&decode(signature)?).map_err(|_| DownloadError::InvalidSignature)?;
    key.verify(bytes, &signature, true)
        .map_err(|_| DownloadError::InvalidSignature)
}

/// Verify a Packager-format signature against a trusted public key.
/// This does not validate the content or grant permission to install it.
pub fn verify_signature(
    bytes: &[u8],
    signature: &str,
    public_key: &str,
) -> Result<(), DownloadError> {
    verify(bytes, signature, public_key)
}

/// Downloads the selected, policy-validated asset, then verifies its signature.
/// Uses the same trust anchor that authenticated the metadata.
pub async fn download_verified<F: FnMut(u64, Option<u64>), V: FnOnce()>(
    authenticated: &AuthenticatedRelease,
    mut progress: F,
    verifying: V,
) -> Result<VerifiedPackage, DownloadError> {
    let release = &authenticated.release;
    if !release.available {
        return Err(DownloadError::NotAvailable);
    }
    let url = release
        .release
        .download_url(&release.target)
        .map_err(|_| DownloadError::Network)?
        .clone();
    let signature = release
        .release
        .signature(&release.target)
        .map_err(|_| DownloadError::InvalidSignature)?
        .clone();
    let redirect = reqwest::redirect::Policy::custom(|attempt| {
        let url = attempt.url();
        if attempt.previous().len() >= 5
            || url.scheme() != "https"
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || !matches!(
                url.host_str(),
                Some(
                    "github.com"
                        | "release-assets.githubusercontent.com"
                        | "objects.githubusercontent.com"
                )
            )
        {
            attempt.stop()
        } else {
            attempt.follow()
        }
    });
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(redirect)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(600))
        .user_agent("TikTools-app-updater")
        .build()
        .map_err(|_| DownloadError::Network)?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| DownloadError::Network)?;
    verify_response(
        response,
        MAX_UPDATE_BYTES,
        &mut progress,
        verifying,
        &signature,
        &authenticated.public_key,
    )
    .await
}

/// Shared bounded-reader and authentication path, independent of transport setup.
pub(crate) async fn verify_response<F: FnMut(u64, Option<u64>), V: FnOnce()>(
    response: reqwest::Response,
    limit: u64,
    progress: &mut F,
    verifying: V,
    signature: &str,
    public_key: &str,
) -> Result<VerifiedPackage, DownloadError> {
    let bytes = read_update(response, limit, progress).await?;
    verifying();
    let signature = signature.to_owned();
    let key = public_key.to_owned();
    tokio::task::spawn_blocking(move || verified_package(bytes, &signature, &key))
        .await
        .map_err(|_| DownloadError::InvalidSignature)?
}

pub(crate) fn verified_package(
    bytes: Vec<u8>,
    signature: &str,
    public_key: &str,
) -> Result<VerifiedPackage, DownloadError> {
    verify(&bytes, signature, public_key)?;
    Ok(VerifiedPackage { bytes })
}

async fn read_update<F: FnMut(u64, Option<u64>)>(
    mut response: reqwest::Response,
    limit: u64,
    progress: &mut F,
) -> Result<Vec<u8>, DownloadError> {
    if !response.status().is_success() {
        return Err(DownloadError::Network);
    }
    let total = response.content_length();
    if total.is_some_and(|n| n > limit) {
        return Err(DownloadError::TooLarge);
    }
    let mut bytes = Vec::new();
    progress(0, total);
    while let Some(chunk) = response.chunk().await.map_err(|_| DownloadError::Network)? {
        if (bytes.len() as u64).saturating_add(chunk.len() as u64) > limit {
            return Err(DownloadError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
        progress(bytes.len() as u64, total);
    }
    if total.is_some_and(|n| n != bytes.len() as u64) {
        return Err(DownloadError::Truncated);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Public test vector from minisign-verify; not an application signing key.
    #[test]
    fn verifies_packager_encoding_and_rejects_tampering() {
        let key = STANDARD.encode("untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3");
        let sig = STANDARD.encode("untrusted comment: signature from minisign secret key\nRWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\ntrusted comment: timestamp:1555779966\tfile:test\nQtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==");
        assert!(verify(b"test", &sig, &key).is_ok());
        assert!(verify(b"Test", &sig, &key).is_err());
        assert!(verify(b"test", "garbage", &key).is_err());
        assert!(verify(b"test", &sig, "garbage").is_err());
    }

    #[tokio::test]
    async fn rejects_oversize_and_interrupted_downloads_without_faking_progress() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        async fn response(data: &'static [u8]) -> reqwest::Response {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = [0; 1024];
                let _ = stream.read(&mut request).await;
                let _ = stream.write_all(data).await;
            });
            reqwest::get(format!("http://{addr}/")).await.unwrap()
        }
        let chunked = response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\n12345\r\n0\r\n\r\n").await;
        assert!(matches!(
            read_update(chunked, 4, &mut |_, _| {}).await,
            Err(DownloadError::TooLarge)
        ));
        let truncated =
            response(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\n123")
                .await;
        assert!(read_update(truncated, 20, &mut |_, _| {}).await.is_err());
        let chunked = response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n3\r\n123\r\n0\r\n\r\n").await;
        let mut updates = Vec::new();
        let bytes = read_update(chunked, 4, &mut |n, total| updates.push((n, total)))
            .await
            .unwrap();
        assert_eq!(bytes, b"123");
        assert_eq!(updates.last(), Some(&(3, None)));
    }
}
