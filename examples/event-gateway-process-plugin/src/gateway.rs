use std::{
    collections::HashMap,
    fs, io,
    net::IpAddr,
    path::PathBuf,
    sync::{mpsc as std_mpsc, Arc},
    thread::JoinHandle,
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use tiktools_plugin_sdk::{
    data_dir, DomainEventEnvelope, Plugin, PluginContext, PluginError, PluginResult,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream},
    sync::{broadcast, Notify},
    task::JoinSet,
};
use tokio_tungstenite::{
    tungstenite::{protocol::Role, Message},
    WebSocketStream,
};
use url::form_urlencoded;

const DEFAULT_PORT: u16 = 17_452;
const DEFAULT_BIND: &str = "127.0.0.1";
const DEFAULT_ORIGIN: &str = "https://widgets.tiktools.app";
const EVENT_CHANNEL_CAPACITY: usize = 256;
const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;
const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 1024 * 1024;
const AUTH_TIMEOUT: Duration = Duration::from_secs(5);
const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub port: u16,
    pub bind: IpAddr,
    pub allowed_origins: Vec<String>,
    pub token: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            bind: DEFAULT_BIND.parse().expect("default gateway bind is valid"),
            allowed_origins: vec![DEFAULT_ORIGIN.to_owned()],
            token: String::new(),
        }
    }
}

impl GatewayConfig {
    fn from_settings(settings: &Value) -> Result<Self, String> {
        let mut config = Self::default();
        let Some(object) = settings.as_object() else {
            return Err("gateway settings must be a JSON object".to_owned());
        };

        if let Some(port) = object.get("port") {
            let port = port
                .as_u64()
                .and_then(|value| u16::try_from(value).ok())
                .filter(|value| *value != 0)
                .ok_or_else(|| "port must be an integer from 1 to 65535".to_owned())?;
            config.port = port;
        }

        if let Some(bind) = object.get("bind") {
            let bind = bind
                .as_str()
                .ok_or_else(|| "bind must be a loopback IP address".to_owned())?;
            let bind = bind
                .parse::<IpAddr>()
                .map_err(|_| "bind must be a loopback IP address".to_owned())?;
            if !bind.is_loopback() {
                return Err(
                    "remote/LAN gateway binding is disabled; gateway is loopback-only (use 127.0.0.1 or ::1)".to_owned(),
                );
            }
            config.bind = bind;
        }

        if let Some(origins) = object.get("allowedOrigins") {
            let origins = origins
                .as_array()
                .ok_or_else(|| "allowedOrigins must be an array of origins".to_owned())?;
            if origins.len() > 64 {
                return Err("allowedOrigins has too many entries".to_owned());
            }
            config.allowed_origins = origins
                .iter()
                .map(|origin| {
                    let origin = origin
                        .as_str()
                        .map(str::trim)
                        .filter(|origin| !origin.is_empty() && origin.len() <= 2048)
                        .ok_or_else(|| "allowedOrigins contains an invalid origin".to_owned())?;
                    if origin == "*" {
                        return Err(
                            "allowedOrigins cannot use *; list browser origins explicitly"
                                .to_owned(),
                        );
                    }
                    Ok(origin.to_owned())
                })
                .collect::<Result<Vec<_>, String>>()?;
        }

        if let Some(token) = object.get("token") {
            if let Some(token) = token
                .as_str()
                .map(str::trim)
                .filter(|token| !token.is_empty())
            {
                if token.len() > 4096 {
                    return Err("token is too long".to_owned());
                }
                config.token = token.to_owned();
            }
        }
        if config.token.is_empty() {
            config.token = generated_token();
        }
        Ok(config)
    }
}

#[derive(Clone)]
struct GatewayState {
    config: Arc<GatewayConfig>,
    events: broadcast::Sender<DomainEventEnvelope>,
    server_shutdown: Arc<Notify>,
    clients_shutdown: Arc<Notify>,
}

impl GatewayState {
    fn new(config: GatewayConfig) -> Arc<Self> {
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Arc::new(Self {
            config: Arc::new(config),
            events,
            server_shutdown: Arc::new(Notify::new()),
            clients_shutdown: Arc::new(Notify::new()),
        })
    }

    fn publish(&self, event: DomainEventEnvelope) {
        let _ = self.events.send(event);
    }
}

pub struct EventGatewayPlugin {
    state: Option<Arc<GatewayState>>,
    server_thread: Option<JoinHandle<()>>,
}

impl Default for EventGatewayPlugin {
    fn default() -> Self {
        Self {
            state: None,
            server_thread: None,
        }
    }
}

impl Plugin for EventGatewayPlugin {
    fn initialize(&mut self, _context: &PluginContext) -> PluginResult<()> {
        let (config, settings_path) = load_config()?;
        persist_generated_token(&config, settings_path.as_ref())?;
        let state = GatewayState::new(config);
        let (ready_sender, ready_receiver) = std_mpsc::channel();
        let server_state = Arc::clone(&state);
        let thread = std::thread::Builder::new()
            .name("tiktools-event-gateway".to_owned())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready_sender
                            .send(Err(format!("could not start gateway runtime: {error}")));
                        return;
                    }
                };
                let result = runtime.block_on(run_server(server_state, ready_sender));
                if let Err(error) = result {
                    eprintln!("event-gateway server stopped: {error}");
                }
            })
            .map_err(|error| {
                PluginError::other(format!("could not start gateway thread: {error}"))
            })?;

        match ready_receiver.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                self.state = Some(state);
                self.server_thread = Some(thread);
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(PluginError::other(error))
            }
            Err(error) => {
                state.server_shutdown.notify_one();
                let join_error = thread.join().err().map(|_| "; gateway thread panicked");
                Err(PluginError::other(format!(
                    "gateway did not become ready: {error}{}",
                    join_error.unwrap_or_default()
                )))
            }
        }
    }

    fn event(&mut self, _context: &PluginContext, event: DomainEventEnvelope) -> PluginResult<()> {
        if let Some(state) = self.state.as_ref() {
            state.publish(event);
        }
        Ok(())
    }

    fn shutdown(&mut self, _context: &PluginContext) -> PluginResult<()> {
        if let Some(state) = self.state.take() {
            // Queue a permit for the listener even if shutdown races the
            // listener's first `select`; the listener fans out a wake to all
            // active HTTP/WebSocket connections before it exits.
            state.server_shutdown.notify_one();
        }
        if let Some(thread) = self.server_thread.take() {
            thread
                .join()
                .map_err(|_| PluginError::other("gateway server thread panicked"))?;
        }
        Ok(())
    }
}

fn load_config() -> PluginResult<(GatewayConfig, Option<PathBuf>)> {
    let data_directory = data_dir()?;
    fs::create_dir_all(&data_directory).map_err(|error| {
        PluginError::other(format!("could not create plugin data directory: {error}"))
    })?;
    let path = data_directory.join("settings.json");
    let settings = match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str::<Value>(&contents).map_err(|error| {
            PluginError::other(format!("gateway settings are not valid JSON: {error}"))
        })?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => json!({}),
        Err(error) => {
            return Err(PluginError::other(format!(
                "could not read gateway settings: {error}"
            )))
        }
    };
    GatewayConfig::from_settings(&settings)
        .map(|config| (config, Some(path)))
        .map_err(PluginError::other)
}

fn persist_generated_token(config: &GatewayConfig, path: Option<&PathBuf>) -> PluginResult<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let existing = fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    if existing
        .get("token")
        .and_then(Value::as_str)
        .is_some_and(|token| !token.trim().is_empty())
    {
        return Ok(());
    }
    let mut settings = existing;
    settings.insert("token".to_owned(), Value::String(config.token.clone()));
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&Value::Object(settings)).unwrap(),
    )
    .map_err(|error| {
        PluginError::other(format!("could not save generated gateway token: {error}"))
    })?;
    fs::rename(&temporary, path).map_err(|error| {
        PluginError::other(format!("could not commit gateway settings: {error}"))
    })?;
    Ok(())
}

fn generated_token() -> String {
    format!("ttk-{:016x}{:016x}", fastrand::u64(..), fastrand::u64(..))
}

async fn run_server(
    state: Arc<GatewayState>,
    ready_sender: std_mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    let listener = TcpListener::bind((state.config.bind, state.config.port))
        .await
        .map_err(|error| {
            let message = format!(
                "could not bind {}:{}: {error}",
                state.config.bind, state.config.port
            );
            let _ = ready_sender.send(Err(message.clone()));
            message
        })?;
    let _ = ready_sender.send(Ok(()));
    eprintln!(
        "event-gateway listening on http://{}:{}/",
        state.config.bind, state.config.port
    );

    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = state.server_shutdown.notified() => break,
            accepted = listener.accept() => match accepted {
                Ok((stream, _address)) => {
                    let state = Arc::clone(&state);
                    connections.spawn(async move {
                        if let Err(error) = handle_connection(stream, state).await {
                            eprintln!("event-gateway connection failed: {error}");
                        }
                    });
                }
                Err(error) => eprintln!("event-gateway accept failed: {error}"),
            },
        }
    }
    state.clients_shutdown.notify_waiters();
    // A client may still be blocked before its HTTP headers arrive. Abort
    // every connection after broadcasting shutdown so plugin shutdown never
    // waits indefinitely on an idle socket.
    connections.shutdown().await;
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            eprintln!("event-gateway connection task failed: {error}");
        }
    }
    Ok(())
}

async fn handle_connection(stream: TcpStream, state: Arc<GatewayState>) -> io::Result<()> {
    let mut stream = BufStream::new(stream);
    let request = read_http_request(&mut stream).await?;
    let Some(request) = request else {
        return Ok(());
    };
    let (method, target, headers) = request;
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let origin = headers.get("origin").cloned();

    if !origin_allowed(origin.as_deref(), &state.config) {
        return write_http_response(
            &mut stream,
            403,
            "Forbidden",
            "text/plain; charset=utf-8",
            b"origin not allowed\n",
            origin.as_deref(),
            &state.config,
        )
        .await;
    }
    if method == "OPTIONS" {
        return write_http_response(
            &mut stream,
            204,
            "No Content",
            "text/plain; charset=utf-8",
            b"",
            origin.as_deref(),
            &state.config,
        )
        .await;
    }

    match (method.as_str(), path) {
        ("GET", "/health") => {
            write_http_response(
                &mut stream,
                200,
                "OK",
                "application/json; charset=utf-8",
                br#"{"ok":true,"service":"tiktools.event-gateway"}
"#,
                origin.as_deref(),
                &state.config,
            )
            .await
        }
        ("GET", "/events") => {
            if !authorized_http(&headers, query, &state.config) {
                return write_http_response(
                    &mut stream,
                    401,
                    "Unauthorized",
                    "text/plain; charset=utf-8",
                    b"authentication required\n",
                    origin.as_deref(),
                    &state.config,
                )
                .await;
            }
            let topics = query_topics(query);
            stream_events(stream, state, topics, origin.as_deref()).await
        }
        ("GET", "/ws") => websocket_connection(stream, state, headers, origin.as_deref()).await,
        _ => {
            write_http_response(
                &mut stream,
                404,
                "Not Found",
                "text/plain; charset=utf-8",
                b"not found\n",
                origin.as_deref(),
                &state.config,
            )
            .await
        }
    }
}

type HttpRequest = (String, String, HashMap<String, String>);

async fn read_http_request(stream: &mut BufStream<TcpStream>) -> io::Result<Option<HttpRequest>> {
    let mut bytes = Vec::with_capacity(1024);
    loop {
        let mut line = Vec::new();
        let read = stream.read_until(b'\n', &mut line).await?;
        if read == 0 {
            return Ok(None);
        }
        if bytes.len() + line.len() > MAX_HTTP_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP headers are too large",
            ));
        }
        let is_end = line == b"\r\n" || line == b"\n";
        bytes.extend_from_slice(&line);
        if is_end {
            break;
        }
    }
    parse_http_request(&bytes).map(Some)
}

fn parse_http_request(bytes: &[u8]) -> io::Result<HttpRequest> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "HTTP headers are not UTF-8"))?;
    let mut lines = text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP request line"))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP method"))?;
    let target = request_parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP target"))?;
    let version = request_parts.next().unwrap_or_default();
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported HTTP version",
        ));
    }
    let mut headers = HashMap::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
    }
    Ok((method.to_ascii_uppercase(), target.to_owned(), headers))
}

fn origin_allowed(origin: Option<&str>, config: &GatewayConfig) -> bool {
    origin.is_none_or(|origin| {
        config
            .allowed_origins
            .iter()
            .any(|allowed| allowed == origin)
    })
}

fn authorization_token(headers: &HashMap<String, String>) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
}

fn authorized_http(headers: &HashMap<String, String>, query: &str, config: &GatewayConfig) -> bool {
    authorization_token(headers).is_some_and(|token| token == config.token)
        || form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| key == "token")
            .is_some_and(|(_, token)| token == config.token)
}

fn query_topics(query: &str) -> Vec<String> {
    let topics = form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == "topics")
        .map(|(_, value)| value.into_owned())
        .unwrap_or_else(|| "*".to_owned());
    let mut parsed = Vec::new();
    for topic in topics
        .split(',')
        .map(str::trim)
        .filter(|topic| !topic.is_empty())
    {
        if !tiktools_plugin_sdk::tiktools_plugin_api::manifest::is_valid_event_subscription(topic) {
            return Vec::new();
        }
        parsed.push(topic.to_owned());
    }
    if parsed.is_empty() {
        vec!["*".to_owned()]
    } else {
        parsed
    }
}

async fn write_http_response(
    stream: &mut BufStream<TcpStream>,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    origin: Option<&str>,
    config: &GatewayConfig,
) -> io::Result<()> {
    let cors = cors_headers(origin, config);
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n{cors}\r\n",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

fn cors_headers(origin: Option<&str>, config: &GatewayConfig) -> String {
    let Some(origin) = origin.filter(|origin| {
        config
            .allowed_origins
            .iter()
            .any(|allowed| allowed == origin)
    }) else {
        return String::new();
    };
    format!(
        "Access-Control-Allow-Origin: {origin}\r\nAccess-Control-Allow-Headers: Authorization, Content-Type\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nVary: Origin\r\n"
    )
}

async fn stream_events(
    mut stream: BufStream<TcpStream>,
    state: Arc<GatewayState>,
    topics: Vec<String>,
    origin: Option<&str>,
) -> io::Result<()> {
    let cors = cors_headers(origin, &state.config);
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson; charset=utf-8\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n{cors}\r\n"
    );
    stream.write_all(headers.as_bytes()).await?;
    stream.flush().await?;
    let mut receiver = state.events.subscribe();
    loop {
        tokio::select! {
            biased;
            _ = state.clients_shutdown.notified() => break,
            event = receiver.recv() => match event {
                Ok(event) if matches_topics(&topics, &event.topic) => {
                    write_ndjson(&mut stream, &event).await?;
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(lost)) => {
                    let gap = DomainEventEnvelope::new("event.gap", json!({"lost": lost, "resync": true}));
                    if matches_topics(&topics, &gap.topic) {
                        write_ndjson(&mut stream, &gap).await?;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    Ok(())
}

async fn write_ndjson<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    event: &DomainEventEnvelope,
) -> io::Result<()> {
    let mut line = serde_json::to_vec(event).map_err(io::Error::other)?;
    line.push(b'\n');
    writer.write_all(&line).await?;
    writer.flush().await
}

fn matches_topics(topics: &[String], topic: &str) -> bool {
    topics.iter().any(|subscription| {
        tiktools_plugin_sdk::tiktools_plugin_api::event_subscription_matches(subscription, topic)
    })
}

async fn websocket_connection(
    mut stream: BufStream<TcpStream>,
    state: Arc<GatewayState>,
    headers: HashMap<String, String>,
    origin: Option<&str>,
) -> io::Result<()> {
    if headers
        .get("upgrade")
        .is_none_or(|value| !value.eq_ignore_ascii_case("websocket"))
    {
        return write_http_response(
            &mut stream,
            400,
            "Bad Request",
            "text/plain; charset=utf-8",
            b"WebSocket upgrade required\n",
            origin,
            &state.config,
        )
        .await;
    }
    let Some(key) = headers.get("sec-websocket-key") else {
        return write_http_response(
            &mut stream,
            400,
            "Bad Request",
            "text/plain; charset=utf-8",
            b"Sec-WebSocket-Key required\n",
            origin,
            &state.config,
        )
        .await;
    };
    let accept = websocket_accept_key(key);
    let cors = cors_headers(origin, &state.config);
    let handshake = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n{cors}\r\n"
    );
    stream.write_all(handshake.as_bytes()).await?;
    stream.flush().await?;

    let mut websocket = WebSocketStream::from_raw_socket(stream, Role::Server, None).await;
    let mut receiver = state.events.subscribe();
    let mut authenticated =
        authorization_token(&headers).is_some_and(|token| token == state.config.token);
    let mut topics = if authenticated {
        vec!["*".to_owned()]
    } else {
        Vec::new()
    };
    let authentication_deadline = tokio::time::sleep(AUTH_TIMEOUT);
    tokio::pin!(authentication_deadline);

    loop {
        if !authenticated {
            tokio::select! {
                biased;
                _ = state.clients_shutdown.notified() => break,
                _ = &mut authentication_deadline => {
                    let _ = websocket.send(Message::Close(None)).await;
                    break;
                }
                message = websocket.next() => {
                    match message {
                        Some(Ok(message)) => match handle_websocket_control(message, &state.config, &mut authenticated, &mut topics, &mut websocket).await {
                            Ok(true) => {}
                            Ok(false) => break,
                            Err(error) => return Err(error),
                        },
                        Some(Err(error)) => return Err(io::Error::other(error)),
                        None => break,
                    }
                }
            }
            continue;
        }

        tokio::select! {
            biased;
            _ = state.clients_shutdown.notified() => {
                let _ = websocket.send(Message::Close(None)).await;
                break;
            }
            message = websocket.next() => {
                match message {
                    Some(Ok(message)) => match handle_websocket_control(message, &state.config, &mut authenticated, &mut topics, &mut websocket).await {
                        Ok(true) => {}
                        Ok(false) => break,
                        Err(error) => return Err(error),
                    },
                    Some(Err(error)) => return Err(io::Error::other(error)),
                    None => break,
                }
            }
            event = receiver.recv() => match event {
                Ok(event) if matches_topics(&topics, &event.topic) => {
                    send_websocket_json(&mut websocket, &event).await?;
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(lost)) => {
                    let gap = DomainEventEnvelope::new("event.gap", json!({"lost": lost, "resync": true}));
                    if matches_topics(&topics, &gap.topic) {
                        send_websocket_json(&mut websocket, &gap).await?;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    Ok(())
}

async fn handle_websocket_control<S>(
    message: Message,
    config: &GatewayConfig,
    authenticated: &mut bool,
    topics: &mut Vec<String>,
    websocket: &mut WebSocketStream<S>,
) -> io::Result<bool>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    match message {
        Message::Ping(payload) => {
            websocket
                .send(Message::Pong(payload))
                .await
                .map_err(io::Error::other)?;
            Ok(true)
        }
        Message::Pong(_) => Ok(true),
        Message::Close(_) => Ok(false),
        Message::Text(text) => {
            if text.len() > MAX_WEBSOCKET_MESSAGE_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "WebSocket message too large",
                ));
            }
            handle_websocket_json(text.as_ref(), config, authenticated, topics, websocket).await
        }
        Message::Binary(bytes) => {
            if bytes.len() > MAX_WEBSOCKET_MESSAGE_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "WebSocket message too large",
                ));
            }
            handle_websocket_json(
                std::str::from_utf8(&bytes).unwrap_or_default(),
                config,
                authenticated,
                topics,
                websocket,
            )
            .await
        }
        Message::Frame(_) => Ok(true),
    }
}

async fn handle_websocket_json<S>(
    text: &str,
    config: &GatewayConfig,
    authenticated: &mut bool,
    topics: &mut Vec<String>,
    websocket: &mut WebSocketStream<S>,
) -> io::Result<bool>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let value = serde_json::from_str::<Value>(text).map_err(|error| {
        io::Error::new(io::ErrorKind::InvalidData, format!("invalid JSON: {error}"))
    })?;
    match value.get("type").and_then(Value::as_str) {
        Some("auth") => {
            let token = value
                .get("token")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if token != config.token {
                let _ = websocket
                    .send(Message::Text(
                        json!({"type": "error", "error": "authentication failed"})
                            .to_string()
                            .into(),
                    ))
                    .await;
                return Ok(false);
            }
            *authenticated = true;
            *topics = vec!["*".to_owned()];
            websocket
                .send(Message::Text(
                    json!({"type": "authenticated"}).to_string().into(),
                ))
                .await
                .map_err(io::Error::other)?;
            Ok(true)
        }
        Some("subscribe") if *authenticated => {
            let Some(values) = value.get("topics").and_then(Value::as_array) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "topics must be an array",
                ));
            };
            let mut next = Vec::new();
            for value in values {
                let topic = value.as_str().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "topic must be a string")
                })?;
                if !tiktools_plugin_sdk::tiktools_plugin_api::manifest::is_valid_event_subscription(
                    topic,
                ) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid topic subscription",
                    ));
                }
                next.push(topic.to_owned());
            }
            *topics = next;
            websocket
                .send(Message::Text(
                    json!({"type": "subscribed", "topics": topics})
                        .to_string()
                        .into(),
                ))
                .await
                .map_err(io::Error::other)?;
            Ok(true)
        }
        Some("ping") if *authenticated => {
            websocket
                .send(Message::Text(json!({"type": "pong"}).to_string().into()))
                .await
                .map_err(io::Error::other)?;
            Ok(true)
        }
        _ => {
            let _ = websocket
                .send(Message::Text(
                    json!({"type": "error", "error": "authenticate first"})
                        .to_string()
                        .into(),
                ))
                .await;
            Ok(!*authenticated)
        }
    }
}

async fn send_websocket_json<S>(
    websocket: &mut WebSocketStream<S>,
    value: &DomainEventEnvelope,
) -> io::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let text = serde_json::to_string(value).map_err(io::Error::other)?;
    websocket
        .send(Message::Text(text.into()))
        .await
        .map_err(io::Error::other)
}

fn websocket_accept_key(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());
    BASE64.encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_loopback_only() {
        let config = GatewayConfig::from_settings(&json!({})).unwrap();
        assert_eq!(config.port, DEFAULT_PORT);
        assert!(config.bind.is_loopback());
        assert_eq!(config.allowed_origins, vec![DEFAULT_ORIGIN]);
        assert!(!config.token.is_empty());
    }

    #[test]
    fn remote_binding_is_rejected() {
        let error = GatewayConfig::from_settings(&json!({"bind": "0.0.0.0"})).unwrap_err();
        assert!(error.contains("loopback"));
    }

    #[test]
    fn websocket_accept_key_matches_rfc_example() {
        assert_eq!(
            websocket_accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn request_parser_keeps_auth_and_origin_headers() {
        let request = parse_http_request(
            b"GET /events?topics=live.%2A HTTP/1.1\r\nOrigin: https://widgets.tiktools.app\r\nAuthorization: Bearer abc\r\n\r\n",
        )
        .unwrap();
        assert_eq!(request.0, "GET");
        assert_eq!(request.1, "/events?topics=live.%2A");
        assert_eq!(request.2["origin"], "https://widgets.tiktools.app");
        assert_eq!(request.2["authorization"], "Bearer abc");
    }

    #[test]
    fn topic_queries_are_wildcard_matched() {
        let topics = query_topics("topics=live.%2A%2Cplugin.%2A");
        assert!(matches_topics(&topics, "live.event"));
        assert!(matches_topics(&topics, "plugin.started"));
        assert!(!matches_topics(&topics, "points.changed"));
    }
}
