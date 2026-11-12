use super::*;

pub(super) async fn upgrade(
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
    loop {
        let outgoing = tokio::select! {
            _ = stopping.changed() => break,
            _ = session_clock.tick() => {
                // An open, authenticated socket is activity: slide the idle
                // window so a working tab is never logged out. The absolute
                // ceiling still ends the session eventually.
                if !app.touch_session() || client.is_closed() { break; }
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
pub(super) fn bounded_owner_message(outgoing: Value) -> String {
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
