//! Controller side of the remote-control relay (v3).
//!
//! A controller never reaches a PC directly. For each command the app opens one
//! bounded WSS session with a controller ticket, sends the command, and waits for the
//! host's result. The relay has the final say on admission; the host repeats its own
//! checks. If no result arrives, the outcome is unknown and the caller must read the
//! command receipt before retrying anything.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tiktools_cloud_auth::CloudAuth;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

use crate::{
    cloud_remote_host::{now_millis, socket_url},
    cloud_teams::{self, CloudTeams},
    control::OperationError,
};

const MAX_ARGS_BYTES: usize = 16 * 1024;
const MAX_RESULT_BYTES: usize = 64 * 1024;
const COMMAND_LIFETIME_MS: i64 = 30_000;
const RESULT_TIMEOUT: Duration = Duration::from_secs(40);
const READY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandRunParams {
    pub host_id: String,
    pub operation: String,
    #[serde(default)]
    pub args: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutcome {
    pub command_id: String,
    /// The host's terminal status: `completed`, `failed`, `cancelled`, `expired` or `unknown`.
    pub status: String,
    pub code: Option<String>,
    pub result: Option<Value>,
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

fn valid_operation(value: &str) -> bool {
    (1..=64).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_lowercase() || b == b'.')
}

/// Sends one command to a team and returns the host's result. Fails closed on any
/// ambiguity: a missing result is reported as an unknown outcome, never as success.
pub async fn run_command(
    auth: &CloudAuth,
    teams: &CloudTeams,
    params: CommandRunParams,
) -> Result<CommandOutcome, OperationError> {
    let CommandRunParams {
        host_id,
        operation,
        args,
    } = params;
    if !valid_operation(&operation) {
        return Err(OperationError::invalid("the operation name is not valid"));
    }
    let args = args.unwrap_or_else(|| json!({}));
    if !args.is_object()
        || serde_json::to_vec(&args)
            .map(|b| b.len())
            .unwrap_or(usize::MAX)
            > MAX_ARGS_BYTES
    {
        return Err(OperationError::invalid(
            "arguments must be an object of at most 16 KiB",
        ));
    }
    let controller = teams.controller_device()?;
    let ticket = teams
        .issue_ticket_as(auth, "controller", &host_id, &controller)
        .await?;
    let (origin, _) = cloud_teams::credentials(auth).await?;
    let url = socket_url(&origin)?;
    let (mut socket, _) = timeout(
        Duration::from_secs(15),
        tokio_tungstenite::connect_async(url.as_str()),
    )
    .await
    .map_err(|_| OperationError::unavailable("the relay did not answer"))?
    .map_err(|_| OperationError::unavailable("could not reach the relay"))?;

    socket
        .send(Message::Text(
            json!({"type": "auth", "ticket": ticket}).to_string().into(),
        ))
        .await
        .map_err(|_| OperationError::unavailable("could not send the ticket"))?;
    let ready = timeout(READY_TIMEOUT, next_json(&mut socket))
        .await
        .map_err(|_| OperationError::unavailable("the relay did not confirm the session"))??;
    if ready.get("type").and_then(Value::as_str) != Some("ready")
        || ready.get("role").and_then(Value::as_str) != Some("controller")
        || ready.get("hostId").and_then(Value::as_str) != Some(host_id.as_str())
    {
        return Err(OperationError::forbidden(
            "the relay did not admit this controller to that team",
        ));
    }
    let grant_revision = ready
        .get("grantRevision")
        .and_then(Value::as_i64)
        .ok_or_else(|| OperationError::internal("the relay sent no grant revision"))?;

    let command_id = format!("cmd-{}", uuid::Uuid::new_v4().simple());
    let frame = json!({
        "type": "command",
        "commandId": command_id,
        "operation": operation,
        "args": args,
        "policyRevision": grant_revision,
        // The relay expects integer milliseconds and admits at most 60 seconds ahead.
        "expiresAt": now_millis() + COMMAND_LIFETIME_MS,
    });
    socket
        .send(Message::Text(frame.to_string().into()))
        .await
        .map_err(|_| OperationError::unavailable("the relay connection was lost"))?;

    match timeout(RESULT_TIMEOUT, wait_for_result(&mut socket, &command_id)).await {
        Ok(outcome) => outcome,
        Err(_) => Ok(CommandOutcome {
            command_id,
            status: "unknown".into(),
            code: Some("noResult".into()),
            result: None,
        }),
    }
}

async fn wait_for_result(
    socket: &mut Socket,
    command_id: &str,
) -> Result<CommandOutcome, OperationError> {
    loop {
        let frame = next_json(socket).await?;
        match frame.get("type").and_then(Value::as_str) {
            Some("command.accepted" | "command.status" | "pong") => continue,
            Some("command.rejected")
                if frame.get("commandId").and_then(Value::as_str) == Some(command_id) =>
            {
                let code = frame
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("rejected");
                return Err(OperationError::forbidden(format!(
                    "the relay refused the command: {code}"
                )));
            }
            Some("result")
                if frame.get("commandId").and_then(Value::as_str) == Some(command_id) =>
            {
                let status = frame
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned();
                let code = frame.get("code").and_then(Value::as_str).map(str::to_owned);
                let result = frame.get("result").cloned();
                if result.as_ref().is_some_and(|value| {
                    serde_json::to_vec(value)
                        .map(|b| b.len())
                        .unwrap_or(usize::MAX)
                        > MAX_RESULT_BYTES
                }) {
                    return Err(OperationError::unavailable(
                        "the host's result is too large to show",
                    ));
                }
                return Ok(CommandOutcome {
                    command_id: command_id.to_owned(),
                    status,
                    code,
                    result,
                });
            }
            Some("closed") => {
                let reason = frame
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("closed");
                return Err(OperationError::unavailable(format!(
                    "the session ended: {reason}"
                )));
            }
            Some("error") => {
                let code = frame.get("code").and_then(Value::as_str).unwrap_or("error");
                return Err(OperationError::unavailable(format!(
                    "the relay reported {code}"
                )));
            }
            _ => continue,
        }
    }
}

async fn next_json(socket: &mut Socket) -> Result<Value, OperationError> {
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) if text.len() <= 128 * 1024 => {
                return serde_json::from_str(text.as_str()).map_err(|_| {
                    OperationError::unavailable("the relay sent an unreadable frame")
                });
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            _ => return Err(OperationError::unavailable("the relay connection was lost")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_names_follow_the_relay_alphabet() {
        assert!(valid_operation("live.status"));
        assert!(valid_operation("automation.draft.submit"));
        assert!(!valid_operation(""));
        assert!(!valid_operation("Live.Status"));
        assert!(!valid_operation("live status"));
        assert!(!valid_operation(&"a".repeat(65)));
    }
}
