//! HTTP request parsing, responses, and NDJSON event streaming.

use super::config::GatewayConfig;
use super::state::GatewayState;
use super::topics::matches_topics;
use serde_json::json;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use tiktools_plugin_sdk::DomainEventEnvelope;
use tokio::io::BufStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;

pub(crate) type HttpRequest = (String, String, HashMap<String, String>);

pub(crate) async fn read_http_request(
    stream: &mut BufStream<TcpStream>,
) -> io::Result<Option<HttpRequest>> {
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

pub(crate) fn parse_http_request(bytes: &[u8]) -> io::Result<HttpRequest> {
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

pub(crate) async fn write_http_response(
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

pub(crate) fn cors_headers(origin: Option<&str>, config: &GatewayConfig) -> String {
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

pub(crate) async fn stream_events(
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
