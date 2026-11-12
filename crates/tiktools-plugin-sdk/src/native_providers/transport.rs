use super::*;

/// HTTP client for provider downloads: rustls, per-request timeout,
/// default redirect policy. Integrity pins authenticate every byte, so
/// transport is untrusted by design.
pub fn provider_client() -> Result<Client, NativeProviderError> {
    crate::http::client_builder()
        .map_err(|message| NativeProviderError::Http {
            url: String::new(),
            message,
        })?
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
pub(super) async fn fetch_capped(
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

pub(super) fn trim_base(base: &str) -> &str {
    base.trim_end_matches('/')
}
