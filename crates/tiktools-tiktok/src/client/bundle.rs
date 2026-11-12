//! Signer source overrides, bounded downloads and cache replacement.
use super::config::default_preset;
use crate::{retry, NativeTikTokConfig, TikTokError};
use std::{path::Path, time::Duration};

pub(in super::super) async fn load_signing_bundle(
    config: &NativeTikTokConfig,
) -> Result<String, TikTokError> {
    if let Some(path) = config.bundle_path.as_deref() {
        return read_bundle(path).await;
    }
    if let Some(path) = config.bundle_cache_path.as_deref() {
        if path.is_file() {
            return read_bundle(path).await;
        }
    }
    let client = tiktools_plugin_sdk::http::client_builder()
        .map_err(TikTokError::Bundle)?
        .user_agent(default_preset().user_agent())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| TikTokError::Bundle(error.to_string()))?;
    let response = client
        .get(&config.bundle_url)
        .send()
        .await
        .map_err(|error| TikTokError::Bundle(error.to_string()))?;
    if response.status().as_u16() == 429 {
        return Err(TikTokError::RateLimited {
            retry_after: retry::retry_after(
                response.headers().get(reqwest::header::RETRY_AFTER),
                std::time::SystemTime::now(),
            ),
        });
    }
    if response.status().is_server_error() {
        return Err(TikTokError::RetryableTransport(format!(
            "bundle endpoint returned HTTP {}",
            response.status().as_u16()
        )));
    }
    if !response.status().is_success() {
        return Err(TikTokError::Bundle(format!(
            "bundle download returned HTTP {}",
            response.status().as_u16()
        )));
    }
    let bytes = read_limited_response(response, 8 * 1024 * 1024).await?;
    if bytes.is_empty() {
        return Err(TikTokError::Bundle(
            "downloaded bundle has an invalid size".to_owned(),
        ));
    }
    let source = String::from_utf8(bytes.to_vec())
        .map_err(|_| TikTokError::Bundle("downloaded bundle is not UTF-8".to_owned()))?;
    if let Some(path) = config.bundle_cache_path.as_deref() {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| TikTokError::Bundle(error.to_string()))?;
        }
        let temporary = path.with_extension("download");
        let written = match tokio::fs::write(&temporary, source.as_bytes()).await {
            Ok(()) => tokio::fs::rename(&temporary, path).await,
            Err(error) => Err(error),
        };
        if let Err(error) = written {
            // Never leave a partial `*.download` file behind the cache.
            let _ = tokio::fs::remove_file(&temporary).await;
            return Err(TikTokError::Bundle(error.to_string()));
        }
    }
    Ok(source)
}

pub(in super::super) async fn read_limited_response(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, TikTokError> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(TikTokError::Bundle(format!(
            "downloaded response exceeds the {max_bytes} byte limit"
        )));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| TikTokError::Bundle(error.to_string()))?
    {
        if bytes.len().saturating_add(chunk.len()) > max_bytes {
            return Err(TikTokError::Bundle(format!(
                "downloaded response exceeds the {max_bytes} byte limit"
            )));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub(in super::super) async fn read_bundle(path: &Path) -> Result<String, TikTokError> {
    let display = path.display().to_string();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || std::fs::read_to_string(&path))
        .await
        .map_err(|error| TikTokError::Bundle(error.to_string()))?
        .map_err(|error| TikTokError::Bundle(format!("{display}: {error}")))
}
