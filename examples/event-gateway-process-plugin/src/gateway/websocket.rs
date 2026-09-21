//! WebSocket sessions, control messages, and event streaming.

use super::auth::{authorization_token, tokens_equal};
use super::config::GatewayConfig;
use super::http::{cors_headers, write_http_response};
use super::state::GatewayState;
use super::topics::matches_topics;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tiktools_plugin_sdk::DomainEventEnvelope;
use tokio::io::AsyncWriteExt;
use tokio::io::BufStream;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::{protocol::Role, Message};
use tokio_tungstenite::WebSocketStream;

const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 1024 * 1024;

const AUTH_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) async fn websocket_connection(
    mut stream: BufStream<TcpStream>,
    state: Arc<GatewayState>,
    headers: HashMap<String, String>,
    origin: Option<&str>,
    version: http::Version,
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
    if !headers.contains_key("sec-websocket-key") {
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
    }
    let response = match handshake_response(&headers, version) {
        Ok(response) => response,
        Err(error) => {
            eprintln!("event-gateway rejecting WebSocket handshake: {error}");
            return write_http_response(
                &mut stream,
                400,
                "Bad Request",
                "text/plain; charset=utf-8",
                b"invalid WebSocket handshake\n",
                origin,
                &state.config,
            )
            .await;
        }
    };
    // The 101 response carries the library-computed status and headers
    // (Connection, Upgrade, Sec-WebSocket-Accept) plus our CORS headers.
    let mut handshake = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status().as_u16(),
        response
            .status()
            .canonical_reason()
            .unwrap_or("Switching Protocols")
    );
    for (name, value) in response.headers() {
        handshake.push_str(&format!("{}: {}\r\n", name, value.to_str().unwrap_or("")));
    }
    handshake.push_str(&cors_headers(origin, &state.config));
    handshake.push_str("\r\n");
    stream.write_all(handshake.as_bytes()).await?;
    stream.flush().await?;

    let mut websocket = WebSocketStream::from_raw_socket(stream, Role::Server, None).await;
    let mut receiver = state.events.subscribe();
    let mut authenticated =
        authorization_token(&headers).is_some_and(|token| tokens_equal(token, &state.config.token));
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
            if !tokens_equal(token, &config.token) {
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
                send_control_error(websocket, "topics must be an array").await?;
                return Ok(true);
            };
            let mut next = Vec::new();
            for value in values {
                let Some(topic) = value.as_str() else {
                    send_control_error(websocket, "topic must be a string").await?;
                    return Ok(true);
                };
                if !tiktools_plugin_sdk::tiktools_plugin_api::manifest::is_valid_event_subscription(
                    topic,
                ) {
                    send_control_error(websocket, "invalid topic subscription").await?;
                    return Ok(true);
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

async fn send_control_error<S>(websocket: &mut WebSocketStream<S>, message: &str) -> io::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let text = json!({"type": "error", "error": message}).to_string();
    websocket
        .send(Message::Text(text.into()))
        .await
        .map_err(io::Error::other)
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

/// Builds the 101 Switching Protocols response through tungstenite's
/// server handshake. HTTP version, method, Connection/Upgrade headers,
/// Sec-WebSocket-Version, key validity, and the accept-key calculation
/// are all owned by the library; only routing-relevant pre-checks and
/// CORS headers stay here.
pub(crate) fn handshake_response(
    headers: &HashMap<String, String>,
    version: http::Version,
) -> Result<http::Response<()>, String> {
    let mut builder = http::Request::builder()
        .method(http::Method::GET)
        .uri("/ws")
        .version(version);
    for (name, value) in headers {
        let (Ok(name), Ok(value)) = (
            name.parse::<http::header::HeaderName>(),
            value.parse::<http::header::HeaderValue>(),
        ) else {
            continue;
        };
        builder = builder.header(name, value);
    }
    let request = builder
        .body(())
        .map_err(|error| format!("invalid WebSocket handshake: {error}"))?;
    tokio_tungstenite::tungstenite::handshake::server::create_response(&request)
        .map_err(|error| format!("invalid WebSocket handshake: {error}"))
}
