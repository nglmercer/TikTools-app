//! Cookie parsing and guest bootstrap without buffering the response body.
use super::config::default_preset;
use crate::{retry, TikTokError};
use std::time::Duration;

pub(in super::super) async fn resolve_session(
    raw: &str,
) -> Result<ttl_sign_core::CookieJar, TikTokError> {
    let raw = raw.trim();
    if !raw.is_empty() {
        let jar = ttl_sign_core::CookieJar::parse(raw);
        if !jar.is_empty() {
            return Ok(jar);
        }
    }
    bootstrap_guest_session().await
}

pub(in super::super) async fn bootstrap_guest_session(
) -> Result<ttl_sign_core::CookieJar, TikTokError> {
    let response = tiktools_plugin_sdk::http::client_builder()
        .map_err(TikTokError::Discovery)?
        .user_agent(default_preset().user_agent())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| TikTokError::RetryableTransport(error.to_string()))?
        .get("https://www.tiktok.com/live")
        .header("accept-language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|error| TikTokError::RetryableTransport(error.to_string()))?;
    let status = response.status();
    if status.as_u16() == 429 {
        return Err(TikTokError::RateLimited {
            retry_after: retry::retry_after(
                response.headers().get(reqwest::header::RETRY_AFTER),
                std::time::SystemTime::now(),
            ),
        });
    }
    let mut cookies = Vec::new();
    for value in response.headers().get_all(reqwest::header::SET_COOKIE) {
        if let Ok(value) = value.to_str() {
            if let Some(pair) = value.split(';').next() {
                cookies.push(pair.to_owned());
            }
        }
    }
    // The bootstrap response body is not used. Dropping the response avoids
    // buffering an attacker-controlled document merely to collect cookies.
    drop(response);
    if status.is_server_error() {
        return Err(TikTokError::RetryableTransport(format!(
            "guest bootstrap returned HTTP {}",
            status.as_u16()
        )));
    }
    if !status.is_success() {
        return Err(TikTokError::Discovery(format!(
            "guest bootstrap returned HTTP {}",
            status.as_u16()
        )));
    }
    let header = cookies.join("; ");
    let jar = ttl_sign_core::CookieJar::parse(&header);
    if jar.get("ttwid").is_none() || jar.is_empty() {
        return Err(TikTokError::EmptySession);
    }
    Ok(jar)
}
