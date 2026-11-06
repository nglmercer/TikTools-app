//! Fetch a signed static manifest from the application's fixed update origin.
use std::time::Duration;

use semver::Version;
use thiserror::Error;

use crate::{download::verify, validate_manifest, Channel, ManifestError, ValidatedRelease};

const MAX_MANIFEST_BYTES: usize = 256 * 1024;
const MAX_SIGNATURE_BYTES: usize = 16 * 1024;

#[derive(Debug, Error)]
pub enum CheckError {
    #[error("update metadata request failed")]
    Network,
    #[error("update metadata exceeds the size limit")]
    TooLarge,
    #[error("update metadata signature is invalid")]
    InvalidSignature,
    #[error(transparent)]
    InvalidManifest(#[from] ManifestError),
}

/// A policy-validated release whose complete metadata was authenticated with
/// the configured application key. Only this type may initiate a download.
pub struct AuthenticatedRelease {
    pub(crate) release: ValidatedRelease,
    pub(crate) public_key: String,
}

impl AuthenticatedRelease {
    pub fn release(&self) -> &ValidatedRelease {
        &self.release
    }
}

async fn read_limited(
    client: &reqwest::Client,
    url: &str,
    limit: usize,
) -> Result<Vec<u8>, CheckError> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| CheckError::Network)?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(CheckError::Network);
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(CheckError::TooLarge);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| CheckError::Network)? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(CheckError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// The proposed GitHub Pages endpoints are intentionally not called at startup
/// until publication and an embedded application trust anchor are configured.
/// The sidecar signature authenticates version and asset mapping as well as
/// the payload signature in the unmodified upstream JSON schema.
pub async fn check_for_update(
    channel: Channel,
    target: &str,
    current: &Version,
    public_key: &str,
) -> Result<AuthenticatedRelease, CheckError> {
    let name = match channel {
        Channel::Stable => "stable",
        Channel::Beta => "beta",
    };
    let url = format!("https://nglmercer.github.io/TikTools-app/updates/{name}.json");
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("TikTools-app-updater")
        .build()
        .map_err(|_| CheckError::Network)?;
    let manifest = read_limited(&client, &url, MAX_MANIFEST_BYTES).await?;
    let signature = read_limited(&client, &format!("{url}.sig"), MAX_SIGNATURE_BYTES).await?;
    authenticate_manifest(&manifest, &signature, channel, target, current, public_key)
}

/// Authenticate the exact upstream JSON bytes before interpreting metadata.
pub fn authenticate_manifest(
    manifest: &[u8],
    signature: &[u8],
    channel: Channel,
    target: &str,
    current: &Version,
    public_key: &str,
) -> Result<AuthenticatedRelease, CheckError> {
    if manifest.len() > MAX_MANIFEST_BYTES || signature.len() > MAX_SIGNATURE_BYTES {
        return Err(CheckError::TooLarge);
    }
    let signature = std::str::from_utf8(signature).map_err(|_| CheckError::InvalidSignature)?;
    verify(manifest, signature, public_key).map_err(|_| CheckError::InvalidSignature)?;
    Ok(AuthenticatedRelease {
        release: validate_manifest(manifest, target, channel, current)?,
        public_key: public_key.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn serve(response: &'static [u8]) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).await;
            stream.write_all(response).await.unwrap();
        });
        format!("http://{addr}/")
    }

    #[tokio::test]
    async fn bounds_both_declared_and_streamed_metadata() {
        let client = reqwest::Client::new();
        let partial = serve(
            b"HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nConnection: close\r\n\r\n123",
        )
        .await;
        assert!(matches!(
            read_limited(&client, &partial, 10).await,
            Err(CheckError::Network)
        ));
        let too_long =
            serve(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\n1234567890")
                .await;
        assert!(matches!(
            read_limited(&client, &too_long, 4).await,
            Err(CheckError::TooLarge)
        ));
        let chunked = serve(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\n12345\r\n0\r\n\r\n").await;
        assert!(matches!(
            read_limited(&client, &chunked, 4).await,
            Err(CheckError::TooLarge)
        ));
        let truncated =
            serve(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\n123").await;
        assert!(matches!(
            read_limited(&client, &truncated, 20).await,
            Err(CheckError::Network)
        ));
    }
}
