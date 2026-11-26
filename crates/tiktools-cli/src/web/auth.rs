use super::*;

impl Session {
    pub(super) fn alive(&self) -> bool {
        self.last.elapsed() < SESSION_LIFETIME
            && self.paired_at.elapsed() < SESSION_ABSOLUTE_LIFETIME
    }

    fn remaining_seconds(&self) -> u64 {
        let idle = SESSION_LIFETIME.saturating_sub(self.last.elapsed());
        let absolute = SESSION_ABSOLUTE_LIFETIME.saturating_sub(self.paired_at.elapsed());
        idle.min(absolute).as_secs()
    }
}

impl Auth {
    pub(super) fn session_valid(&self) -> bool {
        self.sessions.iter().any(Session::alive)
    }

    pub(super) fn clear_sessions(&mut self) {
        self.sessions.clear();
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

    fn session_index(&self, auth: &Auth, headers: &HeaderMap) -> Option<usize> {
        let name = self.cookie_name();
        auth.sessions
            .iter()
            .position(|session| session.alive() && cookie_matches(headers, &name, &session.token))
    }

    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        match self.auth.lock() {
            Ok(auth) => self.session_index(&auth, headers).is_some(),
            Err(_) => false,
        }
    }

    /// Token of the live session the cookie identifies. The websocket binds
    /// its heartbeat to it so one browser's activity never keeps another
    /// browser's idle window open.
    pub(super) fn authorized_token(&self, headers: &HeaderMap) -> Option<String> {
        let auth = self.auth.lock().ok()?;
        let index = self.session_index(&auth, headers)?;
        Some(auth.sessions[index].token.clone())
    }

    /// Validate the cookie, slide that session's idle window and return its
    /// token and remaining lifetime so the caller can reissue the cookie.
    pub(super) fn authorized_touch(&self, headers: &HeaderMap) -> Option<(String, u64)> {
        let mut auth = self.auth.lock().ok()?;
        let index = self.session_index(&auth, headers)?;
        let session = &mut auth.sessions[index];
        session.last = Instant::now();
        Some((session.token.clone(), session.remaining_seconds()))
    }

    /// Keep one session alive while its socket is open. Returns false once
    /// that session is gone or expired so the caller closes the connection.
    pub(super) fn touch_session_token(&self, token: &str) -> bool {
        let Ok(mut auth) = self.auth.lock() else {
            return false;
        };
        let Some(session) = auth
            .sessions
            .iter_mut()
            .find(|session| session.alive() && session.token == token)
        else {
            return false;
        };
        session.last = Instant::now();
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

/// Compare two fixed-length secrets without short-circuiting on the first
/// differing byte. The reset token is read from a 0600 file by the CLI.
pub(super) fn token_matches(provided: &str, expected: &str) -> bool {
    let (provided, expected) = (provided.as_bytes(), expected.as_bytes());
    if provided.len() != expected.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in provided.iter().zip(expected) {
        difference |= left ^ right;
    }
    difference == 0
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
