//! Optional loopback browser transport. Owns no AppCore and connects only via IPC.
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        DefaultBodyLimit, Request, State,
    },
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rand::RngExt;
use serde_json::{json, Value};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tiktools_control_api::{
    owner_request_bytes, ControlClient, ControlEvent, MAX_OWNER_LIBRARY_REQUEST_BYTES,
    MAX_OWNER_RESPONSE_BYTES,
};
use tokio::{sync::Semaphore, task::JoinSet};

const PAIRING_RENEW_COOLDOWN: Duration = Duration::from_secs(30);
const SESSION_LIFETIME: Duration = Duration::from_secs(3600);
#[cfg(test)]
const MAX_FRAME: usize = 1024 * 1024;
/// Lifetime of a pairing code, in seconds. Bounds the in-memory check in
/// `pair` and the runtime-file readout the same way.
const PAIRING_TTL_SECS: u64 = 300;
/// Deterministic pairing override for automated login tests. Never set in
/// production: anyone holding the value can pair without the terminal.
const PAIRING_FIXTURE_ENV: &str = "TIKTOOLS_PAIRING_FIXTURE";

struct Auth {
    pairing: Option<String>,
    issued: Instant,
    session: Option<(String, Instant)>,
    attempts: u8,
    renewed_at: Option<Instant>,
}
struct App {
    root: PathBuf,
    asset_token: String,
    origin: String,
    authority: String,
    join_base: String,
    pairing_file: PathBuf,
    auth: Mutex<Auth>,
    clients: Arc<Semaphore>,
    stopping: tokio::sync::watch::Sender<bool>,
}

// Short-lived, attempt-limited human pairing only. Session and asset tokens
// continue to use the independent 256-bit secret() generator below.
fn pairing_code() -> String {
    format!("{:06}", rand::rng().random_range(0..1_000_000u32))
}

fn fixture_code_from(value: Option<std::ffi::OsString>) -> Result<Option<String>, String> {
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
fn mint_pairing() -> Result<String, String> {
    if let Some(code) = fixture_code_from(std::env::var_os(PAIRING_FIXTURE_ENV))? {
        eprintln!(
            "warning: {PAIRING_FIXTURE_ENV} is set; pairing uses a deterministic test code and is not secure"
        );
        return Ok(code);
    }
    Ok(pairing_code())
}

/// Readout of the active code for `web --print-code`, so agents and operators
/// can recover a missed terminal print without restarting the adapter. The
/// file is a readout only: reading never extends, consumes, or retries the
/// code, and the 5-minute TTL is enforced by the reader.
#[derive(serde::Serialize, serde::Deserialize)]
struct PairingSnapshot {
    code: String,
    issued_at_unix: u64,
    expires_in_secs: u64,
    pid: u32,
}

fn pairing_file(port: u16) -> PathBuf {
    tiktools_core::paths::AppPaths::from_environment()
        .temp
        .join(format!("tiktools-web-pairing-{port}.json"))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn write_pairing_file(path: &Path, snapshot: &PairingSnapshot) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let result = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut file = options.open(&tmp)?;
        file.write_all(
            serde_json::to_vec(snapshot)
                .map_err(std::io::Error::other)?
                .as_slice(),
        )?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn publish_pairing_code(path: &Path, code: &str) {
    let snapshot = PairingSnapshot {
        code: code.to_owned(),
        issued_at_unix: now_unix(),
        expires_in_secs: PAIRING_TTL_SECS,
        pid: std::process::id(),
    };
    if let Err(error) = write_pairing_file(path, &snapshot) {
        eprintln!("warning: could not publish pairing code for `web --print-code`: {error}");
    }
}

fn load_pairing_snapshot(path: &Path) -> Result<PairingSnapshot, String> {
    let bytes = std::fs::read(path).map_err(|_| "no active pairing code".to_string())?;
    let snapshot: PairingSnapshot =
        serde_json::from_slice(&bytes).map_err(|_| "no active pairing code".to_string())?;
    if snapshot.code.len() != 6 || !snapshot.code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("no active pairing code".to_string());
    }
    Ok(snapshot)
}

fn snapshot_remaining(snapshot: &PairingSnapshot, now: u64) -> Option<u64> {
    snapshot
        .issued_at_unix
        .saturating_add(snapshot.expires_in_secs)
        .checked_sub(now)
        .filter(|remaining| *remaining > 0)
}

fn server_listening(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(500),
    )
    .is_ok()
}

/// Readout for a separate CLI process. Fails closed when the file is missing,
/// malformed, expired, or the adapter is no longer listening.
fn current_pairing_code_at(path: &Path, port: u16) -> Result<(String, u64), String> {
    let snapshot = load_pairing_snapshot(path)?;
    let remaining = snapshot_remaining(&snapshot, now_unix())
        .ok_or_else(|| "pairing code expired".to_string())?;
    if !server_listening(port) {
        return Err("local browser adapter is not running".to_string());
    }
    Ok((snapshot.code, remaining))
}

fn current_pairing_code(port: u16) -> Result<(String, u64), String> {
    current_pairing_code_at(&pairing_file(port), port)
}

fn secret() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl App {
    fn cookie_name(&self) -> String {
        format!(
            "tiktools_local_{}",
            self.authority.rsplit(':').next().unwrap_or("local")
        )
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        let Ok(auth) = self.auth.lock() else {
            return false;
        };
        let Some((token, since)) = &auth.session else {
            return false;
        };
        since.elapsed() < SESSION_LIFETIME
            && headers
                .get(header::COOKIE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| {
                    value
                        .split(';')
                        .any(|cookie| cookie.trim() == format!("{}={token}", self.cookie_name()))
                })
    }
    fn same_origin(&self, headers: &HeaderMap) -> bool {
        headers
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            == Some(self.origin.as_str())
    }
}

pub async fn run(args: &[String], standalone: bool) -> i32 {
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("usage: tiktools web --assets <directory> [--port N] [--join-origin URL]\n       tiktools web --print-code [--port N]\nLocal browser adapter; start tiktools host --ipc first.\n--print-code prints the active six-digit pairing code without restarting the adapter.");
        return 0;
    }
    if args.iter().any(|arg| arg == "--print-code") {
        return match parse_print_code_options(args) {
            Ok(port) => print_pairing_code(port),
            Err(error) => {
                eprintln!("tiktools web: {error}");
                2
            }
        };
    }
    let (assets, port, join_origin) = match parse_options(args, standalone) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("tiktools web: {error}");
            return 2;
        }
    };
    match start(assets, port, join_origin).await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("tiktools web: {error}");
            3
        }
    }
}

/// `web --print-code [--port N]`: read the active pairing code back without
/// restarting the adapter. Only `--port` may accompany `--print-code`.
fn parse_print_code_options(args: &[String]) -> Result<u16, String> {
    let mut port: u16 = 8787;
    let mut options = args.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--print-code" => {}
            "--port" => {
                port = options
                    .next()
                    .ok_or("--port requires a number")?
                    .parse()
                    .map_err(|_| "invalid port")?;
            }
            _ => {
                return Err(format!(
                    "unknown option: {option}; use web --print-code [--port N]"
                ));
            }
        }
    }
    Ok(port)
}

fn print_pairing_code(port: u16) -> i32 {
    match current_pairing_code(port) {
        Ok((code, remaining)) => {
            // Bare code on stdout for agents and scripts; the human note goes
            // to stderr. Use it immediately: the code is single-use.
            println!("{code}");
            eprintln!("one-use pairing code (single use, valid for {remaining}s); never store it");
            0
        }
        Err(error) => {
            eprintln!("tiktools web --print-code: {error}");
            3
        }
    }
}

fn parse_options(
    args: &[String],
    standalone: bool,
) -> Result<(PathBuf, u16, Option<String>), String> {
    if standalone {
        return Err("web connects to the existing host; omit --standalone".into());
    }
    let mut assets = None;
    let mut port: u16 = 8787;
    let mut join_origin = None;
    let mut options = args.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--assets" => {
                assets = Some(PathBuf::from(
                    options.next().ok_or("--assets requires a directory")?,
                ))
            }
            "--port" => {
                port = options
                    .next()
                    .ok_or("--port requires a number")?
                    .parse()
                    .map_err(|_| "invalid port")?
            }
            "--join-origin" => {
                join_origin = Some(normalize_join_origin(
                    options.next().ok_or("--join-origin requires a URL")?,
                )?);
            }
            _ => {
                return Err(format!(
                    "unknown option: {option}; use web --assets <directory> [--port N] [--join-origin URL]"
                ))
            }
        }
    }
    Ok((assets.ok_or("--assets is required")?, port, join_origin))
}

/// Join links are terminal convenience only: keep them on loopback HTTP so a
/// printed one-click URL can never point a browser at a remote host.
fn normalize_join_origin(value: &str) -> Result<String, String> {
    let base = value.trim_end_matches('/');
    let (host, port_text) = base
        .strip_prefix("http://127.0.0.1:")
        .map(|port| ("127.0.0.1", port))
        .or_else(|| {
            base.strip_prefix("http://localhost:")
                .map(|port| ("localhost", port))
        })
        .ok_or("join origin must be an http://127.0.0.1:<port> or http://localhost:<port> URL")?;
    if port_text.is_empty() || !port_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("invalid join-origin port".into());
    }
    let port: u16 = port_text.parse().map_err(|_| "invalid join-origin port")?;
    if port == 0 {
        return Err("invalid join-origin port".into());
    }
    Ok(format!("http://{host}:{port}"))
}

fn join_link(join_base: &str, pairing: &str) -> String {
    format!("{join_base}/#pair={pairing}")
}

async fn start(assets: PathBuf, port: u16, join_origin: Option<String>) -> Result<(), String> {
    let root = tokio::fs::canonicalize(assets)
        .await
        .map_err(|error| error.to_string())?;
    if !root.join("index.html").is_file() {
        return Err("assets must contain index.html".into());
    }
    let _probe = ControlClient::connect()
        .await
        .map_err(|_| "host_unavailable: start tiktools host --ipc first")?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|error| error.to_string())?;
    let authority = format!(
        "127.0.0.1:{}",
        listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port()
    );
    let origin = format!("http://{authority}");
    let pairing = mint_pairing()?;
    let pairing_path = pairing_file(port);
    publish_pairing_code(&pairing_path, &pairing);
    let join_base = join_origin.unwrap_or_else(|| origin.clone());
    eprintln!(
        "TikTools local browser: {origin}\nOne-use pairing code (valid 5 minutes): {pairing}\nOne-click join: {} (same one-use code, valid 5 minutes)",
        join_link(&join_base, &pairing)
    );
    let state = Arc::new(App {
        root,
        asset_token: secret(),
        origin,
        authority,
        join_base,
        pairing_file: pairing_path.clone(),
        auth: Mutex::new(Auth {
            pairing: Some(pairing),
            issued: Instant::now(),
            session: None,
            attempts: 0,
            renewed_at: None,
        }),
        clients: Arc::new(Semaphore::new(8)),
        stopping: tokio::sync::watch::channel(false).0,
    });
    let shutdown_state = state.clone();
    let pairing_path_for_shutdown = pairing_path;
    let router = Router::new()
        .route("/__local/pair", post(pair))
        .route("/__local/pair/renew", post(renew_pairing))
        .route("/__local/pair.js", get(pair_script))
        .route("/__local/ws", get(upgrade))
        .fallback(get(asset))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn_with_state(state.clone(), boundary))
        .with_state(state);
    let result = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            #[cfg(unix)]
            {
                if let Ok(mut signal) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                {
                    tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = signal.recv() => {} }
                    let _ = shutdown_state.stopping.send(true);
                    return;
                }
            }
            let _ = tokio::signal::ctrl_c().await;
            let _ = shutdown_state.stopping.send(true);
        })
        .await
        .map_err(|error| error.to_string());
    // Best-effort readout cleanup. A stale file is harmless: the reader still
    // enforces the TTL and checks that the adapter is listening.
    let _ = std::fs::remove_file(&pairing_path_for_shutdown);
    result
}

async fn boundary(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    if request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(app.authority.as_str())
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let plugin_asset = request
        .uri()
        .path()
        .starts_with(&format!("/__plugins/{}/", app.asset_token));
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    headers.insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
    headers.insert(header::CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data: https:; frame-ancestors 'self'; base-uri 'none'; form-action 'self'".parse().unwrap());
    if plugin_asset {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "null".parse().unwrap());
        headers.insert(header::CONTENT_SECURITY_POLICY, "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'none'; frame-ancestors 'self'; sandbox allow-scripts".parse().unwrap());
    }
    response
}

async fn pair(
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
    let cookie = format!(
        "{}={session}; HttpOnly; SameSite=Strict; Path=/; Max-Age=3600",
        app.cookie_name()
    );
    auth.session = Some((session, Instant::now()));
    ([(header::SET_COOKIE, cookie)], Json(json!({"ok": true}))).into_response()
}

// Renewal changes only the human code. It never returns credentials or resets
// the shared attempt counter, and cannot invalidate a live paired session.
async fn renew_pairing(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if !app.same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(mut auth) = app.auth.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if auth
        .session
        .as_ref()
        .is_some_and(|(_, issued)| issued.elapsed() < SESSION_LIFETIME)
    {
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

async fn upgrade(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !app.same_origin(&headers) || !app.authorized(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Ok(permit) = app.clients.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    let client = match ControlClient::connect().await {
        Ok(client) => client,
        Err(_) => return (StatusCode::SERVICE_UNAVAILABLE, "host_unavailable").into_response(),
    };
    ws.max_message_size(MAX_OWNER_LIBRARY_REQUEST_BYTES)
        .max_frame_size(MAX_OWNER_LIBRARY_REQUEST_BYTES)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            socket_loop(socket, client, app).await;
        })
}

async fn socket_loop(mut socket: WebSocket, client: ControlClient, app: Arc<App>) {
    let mut stopping = app.stopping.subscribe();
    if *stopping.borrow() {
        return;
    }
    let mut events = client.subscribe();
    let mut calls = JoinSet::new();
    let mut session_clock = tokio::time::interval(Duration::from_secs(10));
    let started = Instant::now();
    loop {
        let outgoing = tokio::select! {
            _ = stopping.changed() => break,
            _ = session_clock.tick() => {
                let valid = app.auth.lock().ok().and_then(|auth| auth.session.as_ref().map(|(_, since)| since.elapsed() < SESSION_LIFETIME)).unwrap_or(false);
                if !valid || client.is_closed() || started.elapsed() >= SESSION_LIFETIME { break; }
                continue;
            }
            message = socket.recv() => {
                let raw = match message {
                    Some(Ok(Message::Text(raw))) => raw,
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
                    _ => break,
                };
                let Ok(request) = serde_json::from_str::<Value>(&raw) else { break; };
                let id = request.get("id").cloned().unwrap_or(Value::Null);
                let method = request.get("method").and_then(Value::as_str).unwrap_or("").to_string();
                if raw.len() > owner_request_bytes(&method) {
                    rpc_error(id, "too_large", "request exceeds its method size limit")
                } else if calls.len() >= 32 {
                    rpc_error(id, "overloaded", "too many pending calls")
                } else if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") || method.is_empty() || !(id.is_string() || id.is_number()) {
                    rpc_error(id, "invalid_request", "expected a JSON-RPC request with an id")
                } else {
                    let client = client.clone();
                    calls.spawn(async move {
                        let metadata = match request.get("meta").filter(|meta|!meta.is_null()) {
                            Some(meta) => match serde_json::from_value::<tiktools_control_api::RequestMetadata>(meta.clone()) {
                                Ok(meta) => Some(meta),
                                Err(_) => return rpc_error(id, "invalid_params", "invalid request metadata"),
                            },
                            None => None,
                        };
                        match client.call_value_with_meta(&method, request.get("params").cloned().unwrap_or(json!({})), metadata).await {
                            Ok(result) => {
                                let mut response = json!({"type": "rpc-response", "response": {"jsonrpc": "2.0", "id": id, "result": result.result}});
                                if let Some(meta) = result.meta { response["response"]["meta"] = json!(meta); }
                                response
                            },
                            Err(error) => {
                                let mut response = rpc_error(id, &error.code, &error.message);
                                if let Some(data) = error.data { response["response"]["error"]["data"] = data; }
                                response
                            },
                        }
                    });
                    continue;
                }
            }
            result = calls.join_next(), if !calls.is_empty() => {
                match result { Some(Ok(value)) => value, _ => break }
            }
            event = events.recv() => {
                match event {
                    Ok(ControlEvent::Domain(event)) => tiktools_control_api::event_notification(&event),
                    Ok(ControlEvent::Reliable(event)) => json!({"jsonrpc":"2.0","method":"event","params":event}),
                    Ok(ControlEvent::Gap { lost }) => json!({"jsonrpc":"2.0", "method":"event.gap", "params":{"lost":lost}}),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(lost)) => json!({"jsonrpc":"2.0", "method":"event.gap", "params":{"lost":lost}}),
                    Err(_) => break,
                }
            }
        };
        let text = bounded_owner_message(outgoing);
        if !matches!(
            tokio::time::timeout(
                Duration::from_secs(5),
                socket.send(Message::Text(text.into()))
            )
            .await,
            Ok(Ok(()))
        ) {
            break;
        }
    }
    calls.abort_all();
}

fn rpc_error(id: Value, code: &str, message: &str) -> Value {
    json!({"type":"rpc-response", "response":{"jsonrpc":"2.0", "id":id, "error":{"code":code, "message":message}}})
}

/// The loopback Owner adapter carries complete source reviews. Request limits
/// and the separately negotiated remote gateway frame limits stay unchanged.
fn bounded_owner_message(outgoing: Value) -> String {
    let text = outgoing.to_string();
    if text.len() <= MAX_OWNER_RESPONSE_BYTES {
        return text;
    }
    let bounded = if let Some(id) = outgoing
        .get("response")
        .and_then(|response| response.get("id"))
    {
        rpc_error(
            id.clone(),
            "too_large",
            "local owner response exceeds its bounded size limit",
        )
    } else {
        json!({"jsonrpc":"2.0","method":"event.gap","params":{"lost":1,"resync":true}})
    };
    let encoded = bounded.to_string();
    if encoded.len() <= MAX_OWNER_RESPONSE_BYTES {
        encoded
    } else {
        rpc_error(
            Value::Null,
            "too_large",
            "local owner response exceeds its bounded size limit",
        )
        .to_string()
    }
}

async fn asset(State(app): State<Arc<App>>, headers: HeaderMap, request: Request) -> Response {
    if request.uri().path().starts_with("/__plugins/") {
        return plugin_asset(app, headers, request).await;
    }
    if !app.authorized(&headers) {
        return (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            PAIR_PAGE,
        )
            .into_response();
    }
    let raw = request.uri().path();
    // URL decoding is deliberately disallowed for static paths in the first adapter.
    if raw.contains('%') || raw.contains('\\') || raw.split('/').any(|part| part == "..") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let path = app.root.join(raw.trim_start_matches('/'));
    let path = if raw == "/" {
        app.root.join("index.html")
    } else {
        path
    };
    let path = match tokio::fs::canonicalize(path).await {
        Ok(path) if path.starts_with(&app.root) && path.is_file() => path,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let mime = match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    };
    if !matches!(tokio::fs::metadata(&path).await, Ok(meta) if meta.len() <= 32 * 1024 * 1024) {
        return StatusCode::NOT_FOUND.into_response();
    }
    if path == app.root.join("index.html") {
        return match tokio::fs::read_to_string(path).await {
            Ok(html) => (
                [(header::CONTENT_TYPE, mime)],
                html.replacen(
                    "<head>",
                    &format!("<head><meta name=\"tiktools-transport\" content=\"local-web\"><meta name=\"tiktools-plugin-assets\" content=\"/__plugins/{}/\">", app.asset_token),
                    1,
                ),
            )
                .into_response(),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        };
    }
    match tokio::fs::read(path).await {
        Ok(bytes) if bytes.len() <= 32 * 1024 * 1024 => {
            ([(header::CONTENT_TYPE, mime)], Body::from(bytes)).into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

// The opaque iframe receives an asset-only random prefix, never the HTTP session
// cookie. This lets module requests omit credentials while the broker remains
// its only way to request control operations. Tokens expire with the session.
async fn plugin_asset(app: Arc<App>, headers: HeaderMap, request: Request) -> Response {
    let valid_session = app
        .auth
        .lock()
        .ok()
        .and_then(|auth| {
            auth.session
                .as_ref()
                .map(|(_, issued)| issued.elapsed() < SESSION_LIFETIME)
        })
        .unwrap_or(false);
    let prefix = format!("/__plugins/{}/", app.asset_token);
    let Some(rest) = request.uri().path().strip_prefix(&prefix) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !valid_session
        || headers
            .get(header::ORIGIN)
            .is_some_and(|value| value != "null" && value != app.origin.as_str())
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some((id, file)) = rest.split_once('/') else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        || file.is_empty()
        || file.contains('%')
        || file.contains('\\')
        || file
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let client = match ControlClient::connect().await {
        Ok(client) => client,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let assets: tiktools_control_api::modules::plugins::PluginUiAssetsResult = match client
        .call("plugins.ui.assets", json!({"pluginId":id}))
        .await
    {
        Ok(assets) => assets,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let root = PathBuf::from(assets.asset_root);
    let canonical = match tokio::fs::canonicalize(root.join(file)).await {
        Ok(path) if path.starts_with(&root) && path.is_file() => path,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    if !matches!(tokio::fs::metadata(&canonical).await, Ok(meta) if meta.len() <= 8 * 1024 * 1024) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mime = match canonical
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        _ => "application/octet-stream",
    };
    match tokio::fs::read(canonical).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, mime)], Body::from(bytes)).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn pair_script() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/javascript")], PAIR_SCRIPT)
}

const PAIR_PAGE: &str = r#"<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1"><title>TikTools pairing</title><style>
:root { color-scheme: dark; font-family: "Segoe UI", system-ui, -apple-system, sans-serif; background: #17191f; color: #f2f3f5; }
* { box-sizing: border-box; }
html { min-height: 100%; display: grid; place-items: center; padding: 32px 16px; }
body { width: 100%; max-width: 420px; margin: 0; padding: 32px; border: 1px solid #3f4350; border-radius: 24px; background: #25272f; box-shadow: 0 24px 72px rgba(0, 0, 0, .28); }
h1 { margin: 0 0 12px; font-size: clamp(25px, 6vw, 30px); font-weight: 650; line-height: 1.2; letter-spacing: -.035em; }
p { margin: 0; color: #b3b7c3; font-size: 14px; line-height: 1.65; }
form { margin-top: 28px; }
label { display: block; color: #e0e3eb; font-size: 13px; font-weight: 600; }
#code { display: block; width: 100%; height: 68px; margin-top: 10px; padding: 14px 10px; border: 1px solid #555a6a; border-radius: 12px; outline: none; background: #1b1d23; color: #fff; font: 500 clamp(24px, 7vw, 32px)/1.2 ui-monospace, monospace; font-variant-numeric: tabular-nums; text-align: center; letter-spacing: .32em; text-indent: .32em; caret-color: #9bb2ff; }
#code:focus { border-color: #8ba7ff; box-shadow: 0 0 0 3px rgba(109, 140, 240, .2); }
button { display: block; width: 100%; min-height: 48px; margin-top: 20px; padding: 12px 20px; border: 1px solid #7892ef; border-radius: 12px; background: #4967cf; color: #fff; font-family: inherit; font-size: 15px; font-weight: 600; line-height: 1.4; cursor: pointer; }
button:focus-visible { outline: 2px solid #9bb2ff; outline-offset: 4px; }
button:active { background: #3652b5; }
@media (hover: hover) { button:hover { background: #5576e5; } }
#renew { margin-top: 10px; border-color: #555a6a; background: transparent; color: #c5cddd; }
#renew:hover { background: #31343e; }
button:disabled { opacity: .55; cursor: default; }
#result[data-kind="success"] { color: #a5deb6; }
#result[data-kind="info"] { color: #b3c6ff; }
#result { min-height: 1.65em; margin-top: 16px; color: #ffb3b8; font-size: 13px; }
@media (max-width: 360px) { body { padding: 24px; border-radius: 18px; } }
</style></head><body><h1>TikTools local browser</h1><p>Enter the six-digit, one-use pairing code printed by the adapter. It expires after five minutes. The terminal one-click join link fills it in automatically.</p><form id="pair"><label>Pairing code <input id="code" type="text" inputmode="numeric" pattern="[0-9]{6}" minlength="6" maxlength="6" autocomplete="one-time-code" required></label><button>Connect</button></form><button id="renew" type="button">Request new code</button><p id="result" role="status"></p><script src="/__local/pair.js" defer></script></body></html>"#;
const PAIR_SCRIPT: &str = r#"(() => {
  const form = document.getElementById('pair');
  const input = document.getElementById('code');
  const result = document.getElementById('result');
  const renew = document.getElementById('renew');
  let retryTimer;
  const show = (text, kind = 'error') => { result.textContent = text; result.dataset.kind = kind; };
  const cooldown = seconds => {
    clearTimeout(retryTimer);
    renew.disabled = true;
    retryTimer = setTimeout(() => { renew.disabled = false; }, Math.min(300, Math.max(1, seconds)) * 1000);
  };
  const submitCode = async code => {
    input.value = '';
    try {
      const response = await fetch('/__local/pair', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ code }) });
      if (response.ok) { location.replace('/'); return; }
      show('Incorrect or expired code. Try again or request a new code.');
    } catch { show('Adapter unavailable. Check the terminal where the adapter is running.'); }
  };
  form.addEventListener('submit', event => {
    event.preventDefault();
    submitCode(input.value);
  });
  // One-click terminal links carry the one-use code in the URL fragment, which
  // browsers never send to the server. Clear it immediately, then pair once.
  const auto = (location.hash || '').match(/#pair=([0-9]{6})(?:&|$)/);
  if (auto) {
    try { history.replaceState(null, '', location.pathname); } catch {}
    show('Signing in with the terminal link…', 'info');
    submitCode(auto[1]);
  }
  renew.addEventListener('click', async () => {
    renew.disabled = true; input.value = '';
    show('Requesting a new code…', 'info');
    try {
      const response = await fetch('/__local/pair/renew', { method: 'POST' });
      const body = await response.json();
      if (response.ok) {
        show('New code printed in the terminal. Enter it here within five minutes.', 'success');
        cooldown(30);
      } else if (body.error === 'cooldown') {
        const seconds = Number(body.retryAfterSeconds) || 30;
        show(`Please wait ${seconds} seconds before requesting another code.`, 'info');
        cooldown(seconds);
      } else if (body.error === 'pairing_locked') {
        show('Too many attempts. Restart the adapter to pair again.');
      } else if (body.error === 'already_paired') {
        show('Already paired. Reload this page, or restart the adapter to pair another browser.');
      } else {
        renew.disabled = false; show('Could not request a code. Check the terminal where the adapter is running.');
      }
    } catch { renew.disabled = false; show('Adapter unavailable. Check the terminal where the adapter is running.'); }
  });
})();"#;

#[cfg(test)]
mod tests {
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
            parse_print_code_options(&["--print-code".into(), "--port".into(), "nope".into()])
                .is_err()
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
}
