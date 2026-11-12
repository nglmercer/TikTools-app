use super::*;

impl Auth {
    pub(super) fn session_valid(&self) -> bool {
        match (&self.session, self.paired_at) {
            (Some((_, last)), Some(paired)) => {
                last.elapsed() < SESSION_LIFETIME && paired.elapsed() < SESSION_ABSOLUTE_LIFETIME
            }
            _ => false,
        }
    }

    /// Slide the idle window after a request proved possession of the cookie.
    pub(super) fn touch(&mut self) {
        if self.session_valid() {
            if let Some((_, last)) = self.session.as_mut() {
                *last = Instant::now();
            }
        }
    }

    /// Seconds until the idle or absolute window closes, whichever is first.
    pub(super) fn remaining_seconds(&self) -> u64 {
        let (Some((_, last)), Some(paired)) = (&self.session, self.paired_at) else {
            return 0;
        };
        let idle = SESSION_LIFETIME.saturating_sub(last.elapsed());
        let absolute = SESSION_ABSOLUTE_LIFETIME.saturating_sub(paired.elapsed());
        idle.min(absolute).as_secs()
    }
}

impl App {
    pub(super) fn cookie_name(&self) -> String {
        format!(
            "tiktools_local_{}",
            self.authority.rsplit(':').next().unwrap_or("local")
        )
    }

    pub(super) fn session_cookie(&self, token: &str) -> String {
        format!(
            "{}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={SESSION_COOKIE_MAX_AGE}",
            self.cookie_name()
        )
    }

    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        let Ok(auth) = self.auth.lock() else {
            return false;
        };
        if !auth.session_valid() {
            return false;
        }
        let Some((token, _)) = &auth.session else {
            return false;
        };
        cookie_matches(headers, &self.cookie_name(), token)
    }

    /// Validate the cookie and slide the idle window. Returns the token and the
    /// remaining lifetime so the caller can also refresh the cookie.
    pub(super) fn authorized_touch(&self, headers: &HeaderMap) -> Option<(String, u64)> {
        let mut auth = self.auth.lock().ok()?;
        if !auth.session_valid() {
            return None;
        }
        let token = auth.session.as_ref()?.0.clone();
        if !cookie_matches(headers, &self.cookie_name(), &token) {
            return None;
        }
        auth.touch();
        Some((token, auth.remaining_seconds()))
    }

    /// Keep the session alive while its socket is open. Returns false once the
    /// session is no longer valid so the caller can close the connection.
    pub(super) fn touch_session(&self) -> bool {
        let Ok(mut auth) = self.auth.lock() else {
            return false;
        };
        if !auth.session_valid() {
            return false;
        }
        auth.touch();
        true
    }
    pub(super) fn same_origin(&self, headers: &HeaderMap) -> bool {
        headers
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            == Some(self.origin.as_str())
    }
}

pub(super) fn secret() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn cookie_matches(headers: &HeaderMap, name: &str, token: &str) -> bool {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .any(|cookie| cookie.trim() == format!("{name}={token}"))
        })
}

/// Slide a live session without re-pairing. Same-origin only: a cross-site
/// request cannot keep a session alive, and the response reissues the cookie so
/// a long-lived tab never loses it to the browser's own expiry.
pub(super) async fn session_touch(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if !app.same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some((token, remaining)) = app.authorized_touch(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    (
        [(header::SET_COOKIE, app.session_cookie(&token))],
        Json(json!({"ok": true, "remainingSeconds": remaining})),
    )
        .into_response()
}
