#[test]
fn owner_source_responses_can_exceed_request_frames_but_remain_bounded() {
    let data = "review-source".repeat(MAX_FRAME / 6);
    let outgoing = json!({"type":"rpc-response","response":{"jsonrpc":"2.0","id":17,"result":{"source":data}}});
    let encoded = bounded_owner_message(outgoing);
    assert!(encoded.len() > MAX_FRAME && encoded.len() < MAX_OWNER_RESPONSE_BYTES);
    let decoded: Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded["response"]["id"], 17);
    assert_eq!(decoded["response"]["result"]["source"], data);
    assert_eq!(MAX_FRAME, 1024 * 1024);
}

#[test]
fn oversized_owner_rpc_fails_the_original_id_and_oversized_event_reports_a_gap() {
    let data = "private-source".repeat(MAX_OWNER_RESPONSE_BYTES / 10);
    let encoded = bounded_owner_message(
        json!({"type":"rpc-response","response":{"id":18,"result":{"source":data}}}),
    );
    assert!(encoded.len() < MAX_OWNER_RESPONSE_BYTES);
    let decoded: Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded["response"]["id"], 18);
    assert_eq!(decoded["response"]["error"]["code"], "too_large");
    assert!(decoded["response"].get("result").is_none());
    assert!(!encoded.contains("private-source"));
    let event = bounded_owner_message(
        json!({"jsonrpc":"2.0","method":"event","params":{"source":"x".repeat(MAX_OWNER_RESPONSE_BYTES)}}),
    );
    let decoded: Value = serde_json::from_str(&event).unwrap();
    assert_eq!(decoded["method"], "event.gap");
    assert_eq!(decoded["params"]["lost"], 1);
}
use super::*;

#[cfg(unix)]
#[tokio::test]
async fn plugin_assets_resolve_through_a_symlinked_root_and_stay_confined() {
    let base = std::env::temp_dir().join(format!(
        "tiktools-web-assets-{}-{}",
        std::process::id(),
        now_unix()
    ));
    let real = base.join("real");
    std::fs::create_dir_all(real.join("assets")).unwrap();
    std::fs::write(real.join("index.html"), "<html></html>").unwrap();
    std::fs::write(real.join("assets/app.js"), "").unwrap();
    std::fs::write(base.join("secret.txt"), "nope").unwrap();
    std::os::unix::fs::symlink(base.join("secret.txt"), real.join("leak.txt")).unwrap();
    let linked = base.join("linked");
    std::os::unix::fs::symlink(&real, &linked).unwrap();

    // The host reports the root as configured, here through a symlink.
    let resolved = resolve_plugin_asset(&linked, "index.html").await.unwrap();
    assert_eq!(resolved, real.canonicalize().unwrap().join("index.html"));
    assert!(resolve_plugin_asset(&linked, "assets/app.js")
        .await
        .is_some());
    assert!(resolve_plugin_asset(&linked, "assets").await.is_none());
    assert!(resolve_plugin_asset(&linked, "missing.js").await.is_none());
    assert!(resolve_plugin_asset(&linked, "leak.txt").await.is_none());
    assert!(resolve_plugin_asset(&base.join("absent"), "index.html")
        .await
        .is_none());
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn usage_rejects_public_binding_and_standalone_before_connecting() {
    assert!(parse_options(&[], false).is_err());
    assert!(parse_options(&["--assets".into(), "dist/web".into()], true).is_err());
    assert!(parse_options(
        &[
            "--assets".into(),
            "dist/web".into(),
            "--host".into(),
            "0.0.0.0".into()
        ],
        false
    )
    .is_err());
    assert!(parse_options(
        &[
            "--assets".into(),
            "dist/web".into(),
            "--port".into(),
            "70000".into()
        ],
        false
    )
    .is_err());
    let (_, port, join_origin) = parse_options(
        &[
            "--assets".into(),
            "dist/web".into(),
            "--port".into(),
            "0".into(),
        ],
        false,
    )
    .unwrap();
    assert_eq!(port, 0);
    assert_eq!(join_origin, None);
}

#[test]
fn join_origin_stays_on_loopback_http() {
    let (_, _, join_origin) = parse_options(
        &[
            "--assets".into(),
            "dist/web".into(),
            "--join-origin".into(),
            "http://127.0.0.1:46201/".into(),
        ],
        false,
    )
    .unwrap();
    assert_eq!(join_origin.as_deref(), Some("http://127.0.0.1:46201"));
    for bad in [
        "https://127.0.0.1:46201",
        "http://192.168.1.20:46201",
        "http://example.com:46201",
        "http://127.0.0.1:0",
        "http://127.0.0.1:notaport",
        "http://127.0.0.1:46201/path",
        "http://127.0.0.1:46201?code=123456",
    ] {
        assert!(
            parse_options(
                &[
                    "--assets".into(),
                    "dist/web".into(),
                    "--join-origin".into(),
                    bad.into()
                ],
                false,
            )
            .is_err(),
            "rejects {bad}"
        );
    }
    assert_eq!(
        join_link("http://127.0.0.1:46201", "413616"),
        "http://127.0.0.1:46201/#pair=413616"
    );
}

fn state(root: PathBuf) -> Arc<App> {
    state_with_pairing_file(root, test_pairing_file())
}

/// Unique readout path per test so parallel tests never share a file.
fn test_pairing_file() -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "tiktools-web-pairing-test-{}-{unique}.json",
        std::process::id()
    ))
}

fn state_with_pairing_file(root: PathBuf, pairing_file: PathBuf) -> Arc<App> {
    Arc::new(App {
        root,
        asset_token: secret(),
        origin: "http://127.0.0.1:8787".into(),
        authority: "127.0.0.1:8787".into(),
        join_base: "http://127.0.0.1:8787".into(),
        pairing_file,
        auth: Mutex::new(Auth {
            pairing: Some("012345".into()),
            issued: Instant::now(),
            session: None,
            paired_at: None,
            attempts: 0,
            renewed_at: None,
        }),
        clients: Arc::new(Semaphore::new(8)),
        stopping: tokio::sync::watch::channel(false).0,
    })
}

fn write_snapshot(path: &Path, code: &str, issued_at_unix: u64) {
    write_pairing_file(
        path,
        &PairingSnapshot {
            code: code.into(),
            issued_at_unix,
            expires_in_secs: PAIRING_TTL_SECS,
            pid: std::process::id(),
        },
    )
    .unwrap();
}
fn origin() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::ORIGIN, "http://127.0.0.1:8787".parse().unwrap());
    headers
}
fn session_headers(app: &App, token: &str) -> HeaderMap {
    let mut headers = origin();
    headers.insert(
        header::COOKIE,
        format!("{}={token}", app.cookie_name()).parse().unwrap(),
    );
    headers
}
#[tokio::test]
async fn pairing_issues_a_cookie_that_outlives_the_idle_window() {
    let app = state(PathBuf::new());
    let response = pair(State(app.clone()), origin(), Json(json!({"code":"012345"}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie.contains(&format!("Max-Age={SESSION_COOKIE_MAX_AGE}")));
    assert!(cookie.contains("HttpOnly"));
}
#[tokio::test]
async fn activity_slides_the_idle_window_but_not_the_absolute_ceiling() {
    let app = state(PathBuf::new());
    let response = pair(State(app.clone()), origin(), Json(json!({"code":"012345"}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let token = app.auth.lock().unwrap().session.as_ref().unwrap().0.clone();
    // Nearly idle: an authenticated touch renews it.
    app.auth.lock().unwrap().session.as_mut().unwrap().1 =
        Instant::now() - (SESSION_LIFETIME - Duration::from_secs(5));
    assert!(app.authorized(&session_headers(&app, &token)));
    let touched = session_touch(State(app.clone()), session_headers(&app, &token)).await;
    assert_eq!(touched.status(), StatusCode::OK);
    let body = axum::body::to_bytes(touched.into_body(), 4096)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert!(body["remainingSeconds"].as_u64().unwrap() > 3000);
    assert!(
        app.auth
            .lock()
            .unwrap()
            .session
            .as_ref()
            .unwrap()
            .1
            .elapsed()
            < Duration::from_secs(1)
    );
    // Idle again: the touch endpoint refuses and never revives the session.
    app.auth.lock().unwrap().session.as_mut().unwrap().1 = Instant::now() - SESSION_LIFETIME;
    assert!(!app.authorized(&session_headers(&app, &token)));
    assert_eq!(
        session_touch(State(app.clone()), session_headers(&app, &token))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    // Activity cannot outlive the absolute pairing ceiling.
    app.auth.lock().unwrap().session.as_mut().unwrap().1 = Instant::now();
    app.auth.lock().unwrap().paired_at = Some(Instant::now() - SESSION_ABSOLUTE_LIFETIME);
    assert!(!app.touch_session());
    assert_eq!(
        session_touch(State(app.clone()), session_headers(&app, &token))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn session_touch_requires_same_origin_and_the_session_cookie() {
    let app = state(PathBuf::new());
    pair(State(app.clone()), origin(), Json(json!({"code":"012345"}))).await;
    let token = app.auth.lock().unwrap().session.as_ref().unwrap().0.clone();
    assert_eq!(
        session_touch(State(app.clone()), HeaderMap::new())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut foreign = session_headers(&app, &token);
    foreign.insert(header::ORIGIN, "https://example.invalid".parse().unwrap());
    assert_eq!(
        session_touch(State(app.clone()), foreign).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        session_touch(State(app.clone()), origin()).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        session_touch(State(app.clone()), session_headers(&app, "not-the-token"))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn renewal_is_origin_checked_and_does_not_return_or_reuse_the_code() {
    let app = state(PathBuf::new());
    assert_eq!(
        renew_pairing(State(app.clone()), HeaderMap::new())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut foreign = origin();
    foreign.insert(header::ORIGIN, "https://example.invalid".parse().unwrap());
    assert_eq!(
        renew_pairing(State(app.clone()), foreign).await.status(),
        StatusCode::FORBIDDEN
    );
    {
        let mut auth = app.auth.lock().unwrap();
        auth.attempts = 2;
        auth.issued = Instant::now() - Duration::from_secs(301);
    }
    let response = renew_pairing(State(app.clone()), origin()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({"ok":true,"expiresInSeconds":300,"retryAfterSeconds":30})
    );
    let code = {
        let auth = app.auth.lock().unwrap();
        assert_eq!(auth.attempts, 2);
        assert!(auth.issued.elapsed() < Duration::from_secs(1));
        auth.pairing.clone().unwrap()
    };
    assert_ne!(code, "012345");
    assert_eq!(code.len(), 6);
    assert_eq!(
        pair(State(app.clone()), origin(), Json(json!({"code":"012345"})))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        pair(State(app), origin(), Json(json!({"code":code})))
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn renewal_cooldown_is_shared_and_cannot_reset_a_lockout() {
    let app = state(PathBuf::new());
    let (first, second) = tokio::join!(
        renew_pairing(State(app.clone()), origin()),
        renew_pairing(State(app.clone()), origin())
    );
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(second.headers().contains_key(header::RETRY_AFTER));
    let code = app.auth.lock().unwrap().pairing.clone();
    {
        let mut auth = app.auth.lock().unwrap();
        auth.attempts = 8;
        auth.renewed_at = Some(Instant::now() - Duration::from_secs(31));
    }
    let response = renew_pairing(State(app.clone()), origin()).await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap()["error"],
        "pairing_locked"
    );
    assert_eq!(app.auth.lock().unwrap().pairing, code);
}

#[tokio::test]
async fn renewal_preserves_a_live_session_and_allows_an_expired_session_to_pair_again() {
    let app = state(PathBuf::new());
    assert_eq!(
        pair(State(app.clone()), origin(), Json(json!({"code":"012345"})))
            .await
            .status(),
        StatusCode::OK
    );
    let token = app.auth.lock().unwrap().session.as_ref().unwrap().0.clone();
    assert_eq!(
        renew_pairing(State(app.clone()), origin()).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(app.auth.lock().unwrap().session.as_ref().unwrap().0, token);
    app.auth.lock().unwrap().session.as_mut().unwrap().1 = Instant::now() - SESSION_LIFETIME;
    assert_eq!(
        renew_pairing(State(app.clone()), origin()).await.status(),
        StatusCode::OK
    );
    assert_eq!(app.auth.lock().unwrap().attempts, 1);
    assert!(app.auth.lock().unwrap().pairing.is_some());
}

#[test]
fn human_pairing_is_six_digits_while_other_secrets_stay_strong() {
    for _ in 0..64 {
        let code = pairing_code();
        assert_eq!(code.len(), 6);
        assert!(code.bytes().all(|byte| byte.is_ascii_digit()));
    }
    assert_eq!(secret().len(), 64);
}

#[tokio::test]
async fn malformed_codes_consume_attempts_and_leading_zeros_are_preserved() {
    let app = state(PathBuf::new());
    for candidate in [
        json!(12345),
        json!("12345"),
        json!("0123456"),
        json!("٠١٢٣٤٥"),
        Value::Null,
    ] {
        let response = pair(
            State(app.clone()),
            origin(),
            Json(json!({"code":candidate})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    assert_eq!(app.auth.lock().unwrap().attempts, 5);
    assert_eq!(
        pair(State(app), origin(), Json(json!({"code":"012345"})))
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn pairing_requires_origin_is_one_use_and_cookie_expires() {
    let app = state(PathBuf::new());
    assert_eq!(
        pair(
            State(app.clone()),
            HeaderMap::new(),
            Json(json!({"code":"012345"}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        pair(State(app.clone()), origin(), Json(json!({"code":"wrong"})))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = pair(State(app.clone()), origin(), Json(json!({"code":"012345"}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie.contains("HttpOnly; SameSite=Strict"));
    let session_token = cookie.split(';').next().unwrap().split_once('=').unwrap().1;
    assert_eq!(session_token.len(), 64);
    assert!(session_token.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        cookie.split(';').next().unwrap().parse().unwrap(),
    );
    assert!(app.authorized(&headers));
    assert_eq!(
        pair(State(app.clone()), origin(), Json(json!({"code":"012345"})))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    app.auth.lock().unwrap().session.as_mut().unwrap().1 = Instant::now() - SESSION_LIFETIME;
    assert!(!app.authorized(&headers));
}
#[tokio::test]
async fn pairing_attempts_and_time_are_bounded() {
    let app = state(PathBuf::new());
    for _ in 0..8 {
        let _ = pair(State(app.clone()), origin(), Json(json!({"code":"wrong"}))).await;
    }
    assert_eq!(
        pair(State(app.clone()), origin(), Json(json!({"code":"012345"})))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let app = state(PathBuf::new());
    app.auth.lock().unwrap().issued = Instant::now() - Duration::from_secs(301);
    assert_eq!(
        pair(State(app), origin(), Json(json!({"code":"012345"})))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn assets_reject_traversal_and_symlink_escape() {
    let root = std::env::temp_dir().join(format!("tt-web-{}", secret()));
    tokio::fs::create_dir(&root).await.unwrap();
    tokio::fs::write(root.join("index.html"), "<html><head></head></html>")
        .await
        .unwrap();
    let app = state(root.clone());
    app.auth.lock().unwrap().session = Some(("test-session".into(), Instant::now()));
    app.auth.lock().unwrap().paired_at = Some(Instant::now());
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        "tiktools_local_8787=test-session".parse().unwrap(),
    );
    for path in ["/../Cargo.toml", "/%2e%2e/Cargo.toml", "/a%2fb"] {
        let request = Request::builder().uri(path).body(Body::empty()).unwrap();
        assert_eq!(
            asset(State(app.clone()), headers.clone(), request)
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.parent().unwrap(), root.join("outside")).unwrap();
        let request = Request::builder()
            .uri("/outside/")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            asset(State(app.clone()), headers.clone(), request)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let request = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = asset(State(app), headers, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    assert!(std::str::from_utf8(&bytes)
        .unwrap()
        .contains("tiktools-transport"));
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[test]
fn fixture_override_accepts_only_six_digits() {
    assert_eq!(fixture_code_from(None), Ok(None));
    assert_eq!(
        fixture_code_from(Some("012345".into())),
        Ok(Some("012345".to_string()))
    );
    for bad in ["", "12345", "1234567", "abcdef", " 12345", "12 345"] {
        assert!(
            fixture_code_from(Some(bad.into())).is_err(),
            "rejects {bad}"
        );
    }
}

#[test]
fn print_code_options_default_port_and_reject_unknown() {
    assert_eq!(parse_print_code_options(&["--print-code".into()]), Ok(8787));
    assert_eq!(
        parse_print_code_options(&["--print-code".into(), "--port".into(), "4599".into()]),
        Ok(4599)
    );
    assert!(parse_print_code_options(&["--print-code".into(), "--assets".into()]).is_err());
    assert!(parse_print_code_options(&["--print-code".into(), "--port".into()]).is_err());
    assert!(
        parse_print_code_options(&["--print-code".into(), "--port".into(), "nope".into()]).is_err()
    );
}

#[test]
fn pairing_file_roundtrip_enforces_shape_and_ttl() {
    let path = test_pairing_file();
    let now = now_unix();
    write_snapshot(&path, "123456", now);
    let snapshot = load_pairing_snapshot(&path).unwrap();
    assert_eq!(snapshot.code, "123456");
    assert_eq!(snapshot.expires_in_secs, PAIRING_TTL_SECS);
    assert!(snapshot_remaining(&snapshot, now).is_some_and(|left| left <= PAIRING_TTL_SECS));
    assert_eq!(snapshot_remaining(&snapshot, now + PAIRING_TTL_SECS), None);
    let _ = std::fs::remove_file(&path);
    // Missing, malformed, and non-numeric files all fail closed.
    assert!(load_pairing_snapshot(&path).is_err());
    std::fs::write(&path, b"not json").unwrap();
    assert!(load_pairing_snapshot(&path).is_err());
    write_snapshot(&path, "abc", now);
    assert!(load_pairing_snapshot(&path).is_err());
    let _ = std::fs::remove_file(&path);
}

#[cfg(unix)]
#[test]
fn pairing_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let path = test_pairing_file();
    write_snapshot(&path, "123456", now_unix());
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn readout_fails_closed_without_server_and_opens_with_it() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let path = test_pairing_file();
    write_snapshot(&path, "123456", now_unix());
    // A live listener proves the adapter behind the port is up, so the
    // readout opens even though nothing here speaks the pair protocol.
    let (code, remaining) = current_pairing_code_at(&path, port).unwrap();
    assert_eq!(code, "123456");
    assert!(remaining > 0 && remaining <= PAIRING_TTL_SECS);
    // Expired codes stay rejected while the server is still up.
    write_snapshot(
        &path,
        "123456",
        now_unix().saturating_sub(PAIRING_TTL_SECS + 1),
    );
    assert!(current_pairing_code_at(&path, port).is_err());
    drop(listener);
    write_snapshot(&path, "123456", now_unix());
    assert!(current_pairing_code_at(&path, port).is_err());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn renew_publishes_code_and_pair_consumes_it() {
    let path = test_pairing_file();
    let app = state_with_pairing_file(PathBuf::new(), path.clone());
    let response = renew_pairing(State(app.clone()), origin()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let code = app.auth.lock().unwrap().pairing.clone().unwrap();
    assert_ne!(code, "012345");
    assert_eq!(load_pairing_snapshot(&path).unwrap().code, code);
    let response = pair(State(app.clone()), origin(), Json(json!({"code":code}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        !path.exists(),
        "a spent code must not linger in the readout"
    );
}
