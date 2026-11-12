//! Verified sessions, sensitive request headers and bounded HTTP response decoding.
use super::*;

impl Session {
    /// Explicitly connects and verifies the token. `origin` must be a trusted HTTPS origin;
    /// plain HTTP is accepted only for loopback IP literals for local development.
    pub async fn connect(origin: &str, token: String) -> Result<Self, SyncError> {
        Self::connect_secret(origin, SecretString::from(token)).await
    }

    pub async fn connect_secret(origin: &str, token: SecretString) -> Result<Self, SyncError> {
        let url = parse_origin(origin)?;
        if token.expose_secret().is_empty() || token.expose_secret().len() > 4096 {
            return Err(SyncError::Configuration);
        }
        let mut value = HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))
            .map_err(|_| SyncError::Configuration)?;
        value.set_sensitive(true);
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| SyncError::Configuration)?;
        let mut session = Self {
            origin: url,
            client,
            token,
            account_id: String::new(),
        };
        let profile: Profile = session.get("api/v1/native/me", &[]).await?;
        if profile.id.is_empty() || profile.id.len() > 256 {
            return Err(SyncError::Protocol);
        }
        session.account_id = profile.id;
        Ok(session)
    }

    /// Expose only to an OS credential store; never log or serialize the token.
    pub fn token_for_keychain(&self) -> &SecretString {
        &self.token
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Canonical operator instance identity, suitable for account-scoped storage.
    pub fn instance_origin(&self) -> String {
        self.origin.origin().ascii_serialization()
    }

    pub fn new_state(&self) -> SyncState {
        SyncState::new(
            self.origin.origin().ascii_serialization(),
            self.account_id.clone(),
        )
    }

    pub(super) fn verify_account(&self, state: &SyncState) -> Result<(), SyncError> {
        if state.account_id != self.account_id
            || state.origin != self.origin.origin().ascii_serialization()
        {
            return Err(SyncError::WrongAccount);
        }
        Ok(())
    }

    pub(super) fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, SyncError> {
        let url = self
            .origin
            .join(path)
            .map_err(|_| SyncError::Configuration)?;
        let mut auth = HeaderValue::from_str(&format!("Bearer {}", self.token.expose_secret()))
            .map_err(|_| SyncError::Configuration)?;
        auth.set_sensitive(true);
        Ok(self.client.request(method, url).header(AUTHORIZATION, auth))
    }

    pub(super) async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, SyncError> {
        let response = self
            .request(reqwest::Method::GET, path)?
            .query(params)
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        decode(response).await
    }
}

pub(super) fn parse_origin(origin: &str) -> Result<Url, SyncError> {
    let url = Url::parse(origin).map_err(|_| SyncError::Configuration)?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || url.scheme() == "http" && loopback)
        || url.host().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(SyncError::Configuration);
    }
    Ok(url)
}

pub(super) fn retry_delay(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let values: Vec<_> = headers
        .get_all(reqwest::header::RETRY_AFTER)
        .iter()
        .collect();
    if values.len() != 1 {
        return None;
    }
    values[0]
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()
        .filter(|value| (1..=86_400).contains(value))
}

pub(super) async fn decode<T: for<'de> Deserialize<'de>>(
    response: Response,
) -> Result<T, SyncError> {
    decode_with_limit(response, MAX_RESPONSE).await
}

pub(super) async fn decode_with_limit<T: for<'de> Deserialize<'de>>(
    mut response: Response,
    max_bytes: usize,
) -> Result<T, SyncError> {
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(SyncError::Unauthorized),
        StatusCode::CONFLICT => return Err(SyncError::Conflict),
        StatusCode::GONE => return Err(SyncError::CursorExpired),
        StatusCode::TOO_MANY_REQUESTS => {
            return Err(SyncError::RateLimited {
                retry_after_seconds: retry_delay(response.headers()),
            })
        }
        s if !s.is_success() => return Err(SyncError::Http(s.as_u16())),
        _ => (),
    }
    if response
        .content_length()
        .is_some_and(|n| n > max_bytes as u64)
    {
        return Err(SyncError::Protocol);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| SyncError::Network)? {
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(SyncError::Protocol);
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| SyncError::Protocol)
}
