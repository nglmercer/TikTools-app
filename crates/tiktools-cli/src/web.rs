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
use rand::Rng;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tiktools_control_api::{ControlClient, ControlEvent};
use tokio::{sync::Semaphore, task::JoinSet};

const SESSION_LIFETIME: Duration = Duration::from_secs(3600);
const MAX_FRAME: usize = 1024 * 1024;

struct Auth {
    pairing: Option<String>,
    issued: Instant,
    session: Option<(String, Instant)>,
    attempts: u8,
}
struct App {
    root: PathBuf,
    asset_token: String,
    origin: String,
    authority: String,
    auth: Mutex<Auth>,
    clients: Arc<Semaphore>,
    stopping: tokio::sync::watch::Sender<bool>,
}

// Short-lived, attempt-limited human pairing only. Session and asset tokens
// continue to use the independent 256-bit secret() generator below.
fn pairing_code() -> String {
    format!("{:06}", rand::rng().random_range(0..1_000_000u32))
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
        println!("usage: tiktools web --assets <directory> [--port N]\nLocal browser adapter; start tiktools host --ipc first.");
        return 0;
    }
    let (assets, port) = match parse_options(args, standalone) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("tiktools web: {error}");
            return 2;
        }
    };
    match start(assets, port).await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("tiktools web: {error}");
            3
        }
    }
}

fn parse_options(args: &[String], standalone: bool) -> Result<(PathBuf, u16), String> {
    if standalone {
        return Err("web connects to the existing host; omit --standalone".into());
    }
    let mut assets = None;
    let mut port: u16 = 8787;
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
            _ => {
                return Err(format!(
                    "unknown option: {option}; use web --assets <directory> [--port N]"
                ))
            }
        }
    }
    Ok((assets.ok_or("--assets is required")?, port))
}

async fn start(assets: PathBuf, port: u16) -> Result<(), String> {
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
    let pairing = pairing_code();
    eprintln!(
        "TikTools local browser: {origin}\nOne-use pairing code (valid 5 minutes): {pairing}"
    );
    let state = Arc::new(App {
        root,
        asset_token: secret(),
        origin,
        authority,
        auth: Mutex::new(Auth {
            pairing: Some(pairing),
            issued: Instant::now(),
            session: None,
            attempts: 0,
        }),
        clients: Arc::new(Semaphore::new(8)),
        stopping: tokio::sync::watch::channel(false).0,
    });
    let shutdown_state = state.clone();
    let router = Router::new()
        .route("/__local/pair", post(pair))
        .route("/__local/pair.js", get(pair_script))
        .route("/__local/ws", get(upgrade))
        .fallback(get(asset))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn_with_state(state.clone(), boundary))
        .with_state(state);
    axum::serve(listener, router)
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
        .map_err(|error| error.to_string())
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
    let session = secret();
    let cookie = format!(
        "{}={session}; HttpOnly; SameSite=Strict; Path=/; Max-Age=3600",
        app.cookie_name()
    );
    auth.session = Some((session, Instant::now()));
    ([(header::SET_COOKIE, cookie)], Json(json!({"ok": true}))).into_response()
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
    ws.max_message_size(MAX_FRAME)
        .max_frame_size(MAX_FRAME)
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
                if calls.len() >= 32 {
                    rpc_error(id, "overloaded", "too many pending calls")
                } else if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") || method.is_empty() || !(id.is_string() || id.is_number()) {
                    rpc_error(id, "invalid_request", "expected a JSON-RPC request with an id")
                } else {
                    let client = client.clone();
                    calls.spawn(async move {
                        match client.call_value(&method, request.get("params").cloned().unwrap_or(json!({}))).await {
                            Ok(result) => json!({"type": "rpc-response", "response": {"jsonrpc": "2.0", "id": id, "result": result}}),
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
                    Ok(ControlEvent::Gap { lost }) => json!({"jsonrpc":"2.0", "method":"event.gap", "params":{"lost":lost}}),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(lost)) => json!({"jsonrpc":"2.0", "method":"event.gap", "params":{"lost":lost}}),
                    Err(_) => break,
                }
            }
        };
        let text = outgoing.to_string();
        if text.len() > MAX_FRAME {
            break;
        }
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
#result { min-height: 1.65em; margin-top: 16px; color: #ffb3b8; font-size: 13px; }
@media (max-width: 360px) { body { padding: 24px; border-radius: 18px; } }
</style></head><body><h1>TikTools local browser</h1><p>Enter the six-digit, one-use pairing code printed by the Termux adapter. It expires after five minutes.</p><form id="pair"><label>Pairing code <input id="code" type="text" inputmode="numeric" pattern="[0-9]{6}" minlength="6" maxlength="6" autocomplete="one-time-code" required></label><button>Connect</button></form><p id="result" role="status"></p><script src="/__local/pair.js" defer></script></body></html>"#;
const PAIR_SCRIPT: &str = r#"document.getElementById('pair').addEventListener('submit',async(event)=>{event.preventDefault();const input=document.getElementById('code');const code=input.value;input.value='';try{const response=await fetch('/__local/pair',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({code})});if(response.ok){location.replace('/');return;}document.getElementById('result').textContent='Pairing failed or expired. Restart the adapter for a new code.';}catch{document.getElementById('result').textContent='Adapter unavailable. Check Termux.';}});"#;

#[cfg(test)]
mod tests {
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
        let (_, port) = parse_options(
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
    }

    fn state(root: PathBuf) -> Arc<App> {
        Arc::new(App {
            root,
            asset_token: secret(),
            origin: "http://127.0.0.1:8787".into(),
            authority: "127.0.0.1:8787".into(),
            auth: Mutex::new(Auth {
                pairing: Some("012345".into()),
                issued: Instant::now(),
                session: None,
                attempts: 0,
            }),
            clients: Arc::new(Semaphore::new(8)),
            stopping: tokio::sync::watch::channel(false).0,
        })
    }
    fn origin() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, "http://127.0.0.1:8787".parse().unwrap());
        headers
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
}
