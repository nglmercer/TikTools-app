//! WSS relay transport for the installable remote-control plugin.
//!
//! The plugin never receives a bearer token. It asks the host for:
//!   - `cloud.status` → nonsecret `origin` of the configured Cloud;
//!   - `remote.controller.ticket` → a short-lived single-use relay ticket
//!     minted by the host with its own Cloud session.
//!
//! With only those two values it opens `/api/v3/remote/socket`, authenticates
//! with the ticket, and forwards admitted `command` frames to the host through
//! `remote.command.dispatch`.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tiktools_plugin_sdk::host_remote_execute;
use tokio::sync::oneshot;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
};

use crate::GatewayConfig;

const READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

fn socket_url(origin: &str) -> Result<String, String> {
    let origin = origin.trim_end_matches('/');
    let scheme = if origin.starts_with("https://") { "wss" } else { "ws" };
    let rest = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))
        .ok_or_else(|| "unsupported Cloud origin".to_owned())?;
    Ok(format!("{scheme}://{rest}/api/v3/remote/socket"))
}

/// Run the WSS relay loop until `stop` fires or the socket closes.
pub async fn run(config: GatewayConfig, stop: oneshot::Receiver<()>) -> Result<(), String> {
    let status = host_remote_execute("cloud.status", json!({})).map_err(|e| e.to_string())?;
    let origin = status
        .get("origin")
        .and_then(Value::as_str)
        .ok_or_else(|| "Cloud is not configured on the host".to_owned())?;
    let url = socket_url(origin)?;

    let issued = host_remote_execute(
        "remote.controller.ticket",
        json!({"hostId": config.host_id, "deviceId": config.device_id}),
    )
    .map_err(|e| e.to_string())?;
    let ticket = issued
        .get("ticket")
        .and_then(Value::as_str)
        .ok_or_else(|| "host did not issue a relay ticket".to_owned())?;

    let request = url
        .into_client_request()
        .map_err(|e| format!("invalid relay URL: {e}"))?;
    let (mut socket, _) = connect_async(request)
        .await
        .map_err(|e| format!("could not reach relay: {e}"))?;

    socket
        .send(Message::Text(json!({"type": "auth", "ticket": ticket}).to_string().into()))
        .await
        .map_err(|e| e.to_string())?;

    let ready = tokio::time::timeout(READY_TIMEOUT, socket.next())
        .await
        .map_err(|_| "relay did not confirm the ticket".to_owned())?
        .ok_or_else(|| "relay closed before ready".to_owned())?
        .map_err(|e| e.to_string())?;
    let ready: Value = match ready {
        Message::Text(text) => serde_json::from_str(&text).map_err(|e| e.to_string())?,
        _ => return Err("relay sent a non-text ready frame".to_owned()),
    };
    if ready.get("type").and_then(Value::as_str) != Some("ready")
        || ready.get("role").and_then(Value::as_str) != Some("controller")
    {
        return Err("relay refused the controller session".to_owned());
    }

    let mut stop = stop;
    loop {
        tokio::select! {
            _ = &mut stop => {
                let _ = socket.close(None).await;
                break;
            }
            message = socket.next() => {
                let Some(message) = message else { break };
                let message = message.map_err(|e| e.to_string())?;
                let Message::Text(text) = message else { continue };
                let frame: Value = match serde_json::from_str(&text) {
                    Ok(frame) => frame,
                    Err(_) => continue,
                };
                match frame.get("type").and_then(Value::as_str) {
                    Some("login.code") => {
                        // Display-only: forwarded to the plugin UI as an event
                        // by the host; never logged or persisted here.
                    }
                    Some("command") => {
                        let command_id = frame
                            .get("commandId")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                        let operation = frame
                            .get("operation")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                        let args = frame.get("args").cloned().unwrap_or(json!({}));
                        let result = host_remote_execute(
                            "remote.command.dispatch",
                            json!({"commandId": command_id, "operation": operation, "args": args}),
                        );
                        let response = match result {
                            Ok(value) => value,
                            Err(error) => json!({"ok": false, "error": error.to_string()}),
                        };
                        socket
                            .send(Message::Text(
                                json!({"type": "result", "commandId": command_id, "result": response})
                                    .to_string()
                                    .into(),
                            ))
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    Some("result") | Some("grants.changed") | Some("ack") => {}
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn _assert_send(_: Arc<GatewayConfig>) {}
