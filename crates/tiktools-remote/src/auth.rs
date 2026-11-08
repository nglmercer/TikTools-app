use crate::{
    random_secret,
    state::{BrowserSecrets, State},
};
use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tiktools_collaboration::{hash_secret, Authorization};

pub(crate) const SESSION_COOKIE: &str = "__Host-tiktools-remote";
pub(crate) const TICKET_COOKIE: &str = "__Secure-tiktools-remote-ticket";

pub(crate) struct HttpError {
    pub status: StatusCode,
    pub code: &'static str,
}
impl HttpError {
    pub fn invalid() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_params",
        }
    }
    pub fn forbidden() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
        }
    }
    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
        }
    }
    pub fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal",
        }
    }
    pub fn overloaded() -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "overloaded",
        }
    }
    pub fn rate_limited() -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "rate_limited",
        }
    }
}
impl From<tiktools_collaboration::Error> for HttpError {
    fn from(error: tiktools_collaboration::Error) -> Self {
        use tiktools_collaboration::Error;
        let status = match error {
            Error::Disabled => StatusCode::SERVICE_UNAVAILABLE,
            Error::Invalid(_) => StatusCode::BAD_REQUEST,
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::PairingPending => StatusCode::ACCEPTED,
            Error::PairingRejected | Error::PairingExpired => StatusCode::UNAUTHORIZED,
            Error::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            code: error.code(),
        }
    }
}
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(json!({"error":{"code":self.code}}))).into_response();
        if self.status == StatusCode::TOO_MANY_REQUESTS {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, "60".parse().expect("static"));
        }
        response
    }
}

/// Cookie or Origin presence always selects the browser boundary. Native
/// credentials never provide an escape hatch from browser Origin/CSRF rules.
pub(crate) fn browser_mode(
    state: &State,
    headers: &HeaderMap,
    pairing: bool,
) -> Result<bool, HttpError> {
    let browser = headers.contains_key(header::COOKIE) || headers.contains_key(header::ORIGIN);
    if browser {
        if headers.contains_key(header::AUTHORIZATION) {
            return Err(HttpError::forbidden());
        }
        if headers.get_all(header::ORIGIN).iter().count() != 1 {
            return Err(HttpError::forbidden());
        }
        let origin = headers
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(HttpError::forbidden)?;
        if !state.origins.contains(origin) {
            return Err(HttpError::forbidden());
        }
    } else if pairing
        && headers
            .get("x-tiktools-client")
            .and_then(|v| v.to_str().ok())
            != Some("native")
    {
        return Err(HttpError::forbidden());
    }
    Ok(browser)
}

pub(crate) fn cookie(headers: &HeaderMap, name: &str) -> Result<Option<String>, HttpError> {
    let mut result = None;
    for value in headers.get_all(header::COOKIE) {
        let text = value.to_str().map_err(|_| HttpError::unauthorized())?;
        for part in text.split(';') {
            if let Some((key, value)) = part.trim().split_once('=') {
                if key == name {
                    if result.is_some() {
                        return Err(HttpError::unauthorized());
                    }
                    result = Some(value.to_owned());
                }
            }
        }
    }
    Ok(result)
}

pub(crate) struct Authentication {
    pub authority: Authorization,
    pub token: String,
    pub browser: bool,
}

pub(crate) async fn authenticate(
    state: &Arc<State>,
    headers: &HeaderMap,
    require_origin: bool,
) -> Result<Authentication, HttpError> {
    let browser = if require_origin {
        browser_mode(state, headers, false)?
    } else {
        let cookies = headers.contains_key(header::COOKIE);
        if headers.contains_key(header::ORIGIN) {
            browser_mode(state, headers, false)?
        } else {
            if cookies && headers.contains_key(header::AUTHORIZATION) {
                return Err(HttpError::forbidden());
            }
            if headers
                .get("sec-fetch-site")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|site| site != "same-origin" && site != "none")
            {
                return Err(HttpError::forbidden());
            }
            cookies
        }
    };
    let token = if browser {
        cookie(headers, SESSION_COOKIE)?.ok_or_else(HttpError::unauthorized)?
    } else {
        if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
            return Err(HttpError::unauthorized());
        }
        headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(HttpError::unauthorized)?
            .to_owned()
    };
    let service = Arc::clone(&state.service);
    let credential = token.clone();
    let authority =
        tokio::task::spawn_blocking(move || service.authenticate_live(&credential, browser, true))
            .await
            .map_err(|_| HttpError::internal())??;
    state.rate(&authority.snapshot()?.session_id)?;
    Ok(Authentication {
        authority,
        token,
        browser,
    })
}

pub(crate) fn csrf(state: &State, auth: &Authentication) -> Result<String, HttpError> {
    let identity = auth.authority.snapshot()?;
    let mut browser = state.browser.lock().map_err(|_| HttpError::internal())?;
    let now = Instant::now();
    browser.retain(|_, entry| entry.expires > now);
    let key = hash_secret(&auth.token);
    if browser.len() >= 1024 && !browser.contains_key(&key) {
        return Err(HttpError::overloaded());
    }
    let entry = browser.entry(key).or_insert(BrowserSecrets {
        csrf: random_secret()?,
        session_id: identity.session_id,
        expires: now + Duration::from_secs(24 * 3600),
        tickets: Default::default(),
    });
    entry.expires = now + Duration::from_secs(24 * 3600);
    Ok(entry.csrf.clone())
}

pub(crate) fn check_csrf(
    state: &State,
    auth: &Authentication,
    headers: &HeaderMap,
) -> Result<(), HttpError> {
    if !auth.browser {
        return Ok(());
    }
    let browser = state.browser.lock().map_err(|_| HttpError::internal())?;
    let entry = browser
        .get(&hash_secret(&auth.token))
        .filter(|entry| entry.expires > Instant::now())
        .ok_or_else(HttpError::forbidden)?;
    let input = headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(HttpError::forbidden)?;
    if !bool::from(input.as_bytes().ct_eq(entry.csrf.as_bytes())) {
        return Err(HttpError::forbidden());
    }
    Ok(())
}

pub(crate) fn issue_ticket(state: &State, auth: &Authentication) -> Result<String, HttpError> {
    let identity = auth.authority.snapshot()?;
    let mut browser = state.browser.lock().map_err(|_| HttpError::internal())?;
    let entry = browser
        .get_mut(&hash_secret(&auth.token))
        .filter(|entry| entry.expires > Instant::now() && entry.session_id == identity.session_id)
        .ok_or_else(HttpError::forbidden)?;
    entry.tickets.retain(|_, expiry| *expiry > Instant::now());
    if entry.tickets.len() >= 4 {
        return Err(HttpError::overloaded());
    }
    let ticket = random_secret()?;
    entry.tickets.insert(
        hash_secret(&ticket),
        Instant::now() + Duration::from_secs(30),
    );
    Ok(ticket)
}

pub(crate) fn consume_ticket(
    state: &State,
    auth: &Authentication,
    headers: &HeaderMap,
) -> Result<(), HttpError> {
    let ticket = cookie(headers, TICKET_COOKIE)?.ok_or_else(HttpError::unauthorized)?;
    let identity = auth.authority.snapshot()?;
    let mut browser = state.browser.lock().map_err(|_| HttpError::internal())?;
    let entry = browser
        .get_mut(&hash_secret(&auth.token))
        .filter(|entry| entry.session_id == identity.session_id)
        .ok_or_else(HttpError::unauthorized)?;
    let expiry = entry
        .tickets
        .remove(&hash_secret(&ticket))
        .ok_or_else(HttpError::unauthorized)?;
    if expiry <= Instant::now() {
        return Err(HttpError::unauthorized());
    }
    Ok(())
}
