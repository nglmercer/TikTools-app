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
use tokio::task::JoinSet;

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
