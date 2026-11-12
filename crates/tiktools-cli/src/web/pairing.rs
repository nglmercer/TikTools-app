use super::*;

// Short-lived, attempt-limited human pairing only. Session and asset tokens
// continue to use the independent 256-bit secret() generator below.
pub(super) fn pairing_code() -> String {
    format!("{:06}", rand::rng().random_range(0..1_000_000u32))
}

pub(super) fn fixture_code_from(
    value: Option<std::ffi::OsString>,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let code = value
        .to_str()
        .ok_or_else(|| format!("{PAIRING_FIXTURE_ENV} must be a six-digit code"))?;
    if code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(Some(code.to_owned()))
    } else {
        Err(format!("{PAIRING_FIXTURE_ENV} must be a six-digit code"))
    }
}

/// Active code source. The fixture override keeps automated login tests
/// deterministic; a loud warning marks every process running with it.
pub(super) fn mint_pairing() -> Result<String, String> {
    if let Some(code) = fixture_code_from(std::env::var_os(PAIRING_FIXTURE_ENV))? {
        eprintln!(
            "warning: {PAIRING_FIXTURE_ENV} is set; pairing uses a deterministic test code and is not secure"
        );
        return Ok(code);
    }
    Ok(pairing_code())
}

pub(super) async fn pair(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if !app.same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(mut auth) = app.auth.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if auth.attempts >= 8
        || auth.issued.elapsed() > Duration::from_secs(300)
        || auth.pairing.is_none()
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    auth.attempts += 1;
    let candidate = body.get("code").and_then(Value::as_str);
    if !candidate
        .is_some_and(|code| code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()))
        || candidate != auth.pairing.as_deref()
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    auth.pairing = None;
    // The spent code must not linger in the readout file.
    let _ = std::fs::remove_file(&app.pairing_file);
    let session = secret();
    let cookie = app.session_cookie(&session);
    let now = Instant::now();
    auth.session = Some((session, now));
    auth.paired_at = Some(now);
    ([(header::SET_COOKIE, cookie)], Json(json!({"ok": true}))).into_response()
}

// Renewal changes only the human code. It never returns credentials or resets
// the shared attempt counter, and cannot invalidate a live paired session.
pub(super) async fn renew_pairing(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if !app.same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(mut auth) = app.auth.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if auth.session_valid() {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error":"already_paired"})),
        )
            .into_response();
    }
    if auth.attempts >= 8 {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error":"pairing_locked"})),
        )
            .into_response();
    }
    if let Some(renewed) = auth.renewed_at {
        let remaining = PAIRING_RENEW_COOLDOWN.saturating_sub(renewed.elapsed());
        if !remaining.is_zero() {
            let retry_after = remaining.as_secs() + 1;
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, retry_after.to_string())],
                Json(json!({"error":"cooldown", "retryAfterSeconds":retry_after})),
            )
                .into_response();
        }
    }
    // Draw uniformly from all values except the previous pending code, so a
    // successful rotation always invalidates the old code without retry loops.
    // The test fixture keeps its deterministic value across renewals.
    let code = match fixture_code_from(std::env::var_os(PAIRING_FIXTURE_ENV)) {
        Ok(Some(fixed)) => fixed,
        Ok(None) => {
            if let Some(previous) = auth
                .pairing
                .as_deref()
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|value| *value < 1_000_000)
            {
                let draw = rand::rng().random_range(0..999_999u32);
                format!("{:06}", draw + u32::from(draw >= previous))
            } else {
                pairing_code()
            }
        }
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error).into_response();
        }
    };
    eprintln!(
        "One-use pairing code (valid 5 minutes): {code}\nOne-click join: {} (same one-use code, valid 5 minutes)",
        join_link(&app.join_base, &code)
    );
    auth.pairing = Some(code.clone());
    auth.issued = Instant::now();
    auth.renewed_at = Some(Instant::now());
    publish_pairing_code(&app.pairing_file, &code);
    (
        StatusCode::OK,
        Json(json!({"ok":true,"expiresInSeconds":PAIRING_TTL_SECS,"retryAfterSeconds":30})),
    )
        .into_response()
}
