//! TCP listener, connection spawning, and route dispatch.

use super::auth::{authorized_http, origin_allowed};
use super::http::{read_http_request, stream_events, write_http_response};
use super::state::GatewayState;
use super::topics::query_topics;
use super::websocket::websocket_connection;
use std::io;
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use tokio::io::BufStream;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

/// A client that never finishes its request headers holds its socket
/// only for this long before the connection is dropped.
const HEADER_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Finished connection tasks are reaped on this cadence (and after
/// every accept) so the handle list stays proportional to the live
/// connection count instead of total connections served.
const RECONCILE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

pub(crate) async fn run_server(
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

    let mut connections: Vec<JoinHandle<()>> = Vec::new();
    let mut reconcile = tokio::time::interval(RECONCILE_INTERVAL);
    loop {
        tokio::select! {
            biased;
            _ = state.server_shutdown.notified() => break,
            _ = reconcile.tick() => reconcile_connections(&mut connections),
            accepted = listener.accept() => match accepted {
                Ok((stream, _address)) => {
                    reconcile_connections(&mut connections);
                    let Ok(permit) = Arc::clone(&state.connection_permits).try_acquire_owned() else {
                        eprintln!("event-gateway connection limit reached; dropping connection");
                        continue;
                    };
                    let state = Arc::clone(&state);
                    connections.push(tokio::spawn(async move {
                        let _permit = permit;
                        if let Err(error) = handle_connection(stream, state).await {
                            eprintln!("event-gateway connection failed: {error}");
                        }
                    }));
                }
                Err(error) => eprintln!("event-gateway accept failed: {error}"),
            },
        }
    }
    state.clients_shutdown.notify_waiters();
    // A client may still be blocked before its HTTP headers arrive. Abort
    // every connection after broadcasting shutdown so plugin shutdown never
    // waits indefinitely on an idle socket.
    for handle in &connections {
        handle.abort();
    }
    for handle in connections {
        let _ = handle.await;
    }
    Ok(())
}

/// Drops finished connection tasks. Only finished handles are
/// removed, so no live task is ever detached; per-connection errors
/// are already logged inside the task, and panics surface through
/// the default panic hook.
pub(crate) fn reconcile_connections(connections: &mut Vec<JoinHandle<()>>) {
    connections.retain(|handle| !handle.is_finished());
}

async fn handle_connection(stream: TcpStream, state: Arc<GatewayState>) -> io::Result<()> {
    let mut stream = BufStream::new(stream);
    let request = tokio::time::timeout(HEADER_READ_TIMEOUT, read_http_request(&mut stream))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "HTTP request headers timed out"))??;
    let Some(request) = request else {
        return Ok(());
    };
    let (method, target, headers, version) = request;
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
            if !authorized_http(&headers, &state.config) {
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
            let topics = match query_topics(query) {
                Ok(topics) => topics,
                Err(error) => {
                    return write_http_response(
                        &mut stream,
                        400,
                        "Bad Request",
                        "text/plain; charset=utf-8",
                        format!("{error}\n").as_bytes(),
                        origin.as_deref(),
                        &state.config,
                    )
                    .await;
                }
            };
            stream_events(stream, state, topics, origin.as_deref()).await
        }
        ("GET", "/ws") => {
            websocket_connection(stream, state, headers, origin.as_deref(), version).await
        }
        ("GET", path) if super::widgets::is_widget_route(path) => {
            super::widgets::serve_widget_request(&mut stream, state, path, origin.as_deref()).await
        }
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
