//! Host side of the Cloud remote-control relay (v3).
//!
//! The PC dials out over WSS, so controllers never reach it directly. It confirms
//! every grant the owner approved, admits a command only when the grant is active
//! and the operation is allowed for its role, and executes only the read
//! operations implemented here. Every other operation is answered as unsupported.

use std::{
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tiktools_cloud_auth::CloudAuth;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use crate::{
    cloud_teams::{self, CloudTeams},
    control::OperationError,
    AppCore,
};

const MAX_FRAME_BYTES: usize = 128 * 1024;
const MAX_RESULT_BYTES: usize = 64 * 1024;
const PING_EVERY: Duration = Duration::from_secs(20);
const READY_TIMEOUT: Duration = Duration::from_secs(10);
const BACKOFF_MIN: Duration = Duration::from_secs(2);
const BACKOFF_MAX: Duration = Duration::from_secs(60);

const READ_OPERATIONS: &[&str] = &[
    "live.status",
    "workflow.list",
    "competition.list",
    "points.read",
    "automation.list",
    "command.outcome",
];
const MODERATION_OPERATIONS: &[&str] = &[
    "workflow.disable",
    "workflow.delete",
    "points.adjust",
    "automation.toggle",
    "automation.delete",
    "competition.control",
];

/// The only operations this host executes. Everything else is answered unsupported.
const EXECUTABLE: &[&str] = &["live.status"];

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteHostStatus {
    /// `stopped`, `connecting`, `ready` or `retrying`.
    pub state: String,
    pub host_id: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Default)]
struct LinkInner {
    status: Mutex<Option<RemoteHostStatus>>,
    cancel: Mutex<Option<CancellationToken>>,
}

/// Owns the single outbound link of this PC. Starting twice keeps the first link.
#[derive(Default)]
pub struct RemoteHostLink {
    inner: Arc<LinkInner>,
}

impl LinkInner {
    fn set(&self, state: &str, host_id: Option<&str>, error: Option<String>) {
        let mut status = self
            .status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *status = Some(RemoteHostStatus {
            state: state.to_owned(),
            host_id: host_id.map(str::to_owned),
            last_error: error,
        });
    }
}

impl RemoteHostLink {
    pub fn status(&self) -> RemoteHostStatus {
        self.inner
            .status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .unwrap_or(RemoteHostStatus {
                state: "stopped".into(),
                host_id: None,
                last_error: None,
            })
    }

    /// Starts the link for the team this PC serves. `core` is held weakly so the
    /// link never keeps the application alive on its own.
    pub(crate) fn start(
        &self,
        account: Arc<crate::cloud_account::CloudAccount>,
        teams: Arc<CloudTeams>,
        core: Weak<AppCore>,
    ) -> Result<(), OperationError> {
        let binding = teams.host_binding()?.ok_or_else(|| {
            OperationError::invalid("enroll this PC as a team host before serving a team")
        })?;
        let mut cancel_slot = self
            .inner
            .cancel
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if cancel_slot.is_some() {
            return Ok(());
        }
        let cancel = CancellationToken::new();
        *cancel_slot = Some(cancel.clone());
        drop(cancel_slot);
        self.inner.set("connecting", Some(&binding.0), None);
        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            run(&inner, account, teams, core, binding, cancel).await;
        });
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(cancel) = self
            .inner
            .cancel
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            cancel.cancel();
        }
        self.inner.set("stopped", None, None);
    }
}

async fn run(
    link: &LinkInner,
    account: Arc<crate::cloud_account::CloudAccount>,
    teams: Arc<CloudTeams>,
    core: Weak<AppCore>,
    binding: (String, String),
    cancel: CancellationToken,
) {
    let (host_id, device_id) = binding;
    let mut backoff = BACKOFF_MIN;
    loop {
        if cancel.is_cancelled() {
            break;
        }
        link.set("connecting", Some(&host_id), None);
        let outcome = session(
            link,
            account.auth(),
            &teams,
            &core,
            &host_id,
            &device_id,
            &cancel,
        )
        .await;
        if cancel.is_cancelled() {
            break;
        }
        let message = match outcome {
            Ok(()) => "the relay closed the connection".to_owned(),
            Err(error) => error.message,
        };
        link.set("retrying", Some(&host_id), Some(message));
        tokio::select! {
            _ = sleep(backoff) => {}
            _ = cancel.cancelled() => break,
        }
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }
    link.set("stopped", None, None);
}

async fn session(
    link: &LinkInner,
    auth: &CloudAuth,
    teams: &CloudTeams,
    core: &Weak<AppCore>,
    host_id: &str,
    device_id: &str,
    cancel: &CancellationToken,
) -> Result<(), OperationError> {
    let ticket = teams.issue_ticket(auth, host_id, device_id).await?;
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
        || ready.get("role").and_then(Value::as_str) != Some("host")
        || ready.get("hostId").and_then(Value::as_str) != Some(host_id)
    {
        return Err(OperationError::unavailable("the relay refused this host"));
    }
    link.set("ready", Some(host_id), None);
    confirm_pending(&mut socket, auth, host_id).await?;

    let mut ping = tokio::time::interval(PING_EVERY);
    ping.tick().await;
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = socket.close(None).await;
                return Ok(());
            }
            _ = ping.tick() => {
                socket.send(Message::Text(json!({"type": "ping"}).to_string().into())).await
                    .map_err(|_| OperationError::unavailable("the relay connection was lost"))?;
            }
            frame = socket.next() => {
                let Some(frame) = frame else { return Ok(()) };
                let frame = frame.map_err(|_| OperationError::unavailable("the relay connection was lost"))?;
                match frame {
                    Message::Text(text) => {
                        if text.len() > MAX_FRAME_BYTES {
                            return Err(OperationError::unavailable("the relay sent an oversized frame"));
                        }
                        let value: Value = serde_json::from_str(text.as_str())
                            .map_err(|_| OperationError::unavailable("the relay sent an unreadable frame"))?;
                        match value.get("type").and_then(Value::as_str) {
                            Some("grants.changed") => confirm_pending(&mut socket, auth, host_id).await?,
                            Some("command") => {
                                handle_command(&mut socket, auth, host_id, core, &value).await?;
                            }
                            _ => {}
                        }
                    }
                    Message::Close(_) => return Ok(()),
                    Message::Ping(payload) => {
                        socket.send(Message::Pong(payload)).await
                            .map_err(|_| OperationError::unavailable("the relay connection was lost"))?;
                    }
                    Message::Pong(_) => {}
                    _ => return Err(OperationError::unavailable("the relay sent a binary frame")),
                }
            }
        }
    }
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn next_json(socket: &mut Socket) -> Result<Value, OperationError> {
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) if text.len() <= MAX_FRAME_BYTES => {
                return serde_json::from_str(text.as_str()).map_err(|_| {
                    OperationError::unavailable("the relay sent an unreadable frame")
                });
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            _ => {
                return Err(OperationError::unavailable(
                    "the relay did not confirm the session",
                ))
            }
        }
    }
}

/// Confirms every grant the owner approved, at the exact revision read from the Cloud.
async fn confirm_pending(
    socket: &mut Socket,
    auth: &CloudAuth,
    host_id: &str,
) -> Result<(), OperationError> {
    let grants = cloud_teams::grants(auth, host_id).await?;
    for grant in grants
        .items
        .iter()
        .filter(|grant| grant.status == "pendingHost")
    {
        socket
            .send(Message::Text(
                json!({"type": "grant.confirmed", "grantId": grant.id, "revision": grant.revision})
                    .to_string()
                    .into(),
            ))
            .await
            .map_err(|_| OperationError::unavailable("the relay connection was lost"))?;
    }
    Ok(())
}

async fn handle_command(
    socket: &mut Socket,
    auth: &CloudAuth,
    host_id: &str,
    core: &Weak<AppCore>,
    frame: &Value,
) -> Result<(), OperationError> {
    let command_id = frame
        .get("commandId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if !valid_command_id(&command_id) {
        return Ok(());
    }
    let operation = frame
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let session_id = frame
        .get("sessionId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let args_ok = frame.get("args").is_some_and(Value::is_object);
    let expires = frame
        .get("expiresAt")
        .and_then(Value::as_str)
        .and_then(parse_utc_seconds);

    send_result(socket, &command_id, "running", None, None).await?;
    let outcome = match (expires, now_seconds()) {
        (None, _) => Err("invalidArguments"),
        (Some(expires), now) if expires <= now => Err("expired"),
        _ if !args_ok => Err("invalidArguments"),
        _ => admit(auth, host_id, &session_id, &operation).await,
    };
    match outcome {
        Err(code) => {
            let status = if code == "expired" {
                "expired"
            } else {
                "failed"
            };
            send_result(socket, &command_id, status, Some(code), None).await
        }
        Ok(()) if !EXECUTABLE.contains(&operation.as_str()) => {
            send_result(socket, &command_id, "failed", Some("unsupported"), None).await
        }
        Ok(()) => match execute(core, &operation).await {
            Ok(result)
                if serde_json::to_vec(&result)
                    .map(|b| b.len())
                    .unwrap_or(usize::MAX)
                    <= MAX_RESULT_BYTES =>
            {
                send_result(socket, &command_id, "completed", None, Some(result)).await
            }
            Ok(_) => send_result(socket, &command_id, "failed", Some("resultTooLarge"), None).await,
            Err(code) => send_result(socket, &command_id, "failed", Some(code), None).await,
        },
    }
}

/// Re-reads the grant behind the session and checks it at execution time.
async fn admit(
    auth: &CloudAuth,
    host_id: &str,
    session_id: &str,
    operation: &str,
) -> Result<(), &'static str> {
    let sessions = cloud_teams::sessions(auth, host_id)
        .await
        .map_err(|_| "unavailable")?;
    let session = sessions
        .items
        .iter()
        .find(|session| session.id == session_id)
        .ok_or("forbidden")?;
    let grants = cloud_teams::grants(auth, host_id)
        .await
        .map_err(|_| "unavailable")?;
    let grant = grants
        .items
        .iter()
        .find(|grant| grant.id == session.grant_id && grant.revision == session.grant_revision)
        .ok_or("stale")?;
    if grant.status != "active" {
        return Err("forbidden");
    }
    if allowed(&grant.role, &grant.policy, operation) {
        Ok(())
    } else {
        Err("forbidden")
    }
}

/// Mirrors the role table the relay uses. The host repeats the check because it owns the outcome.
pub fn allowed(role: &str, policy: &Value, operation: &str) -> bool {
    match role {
        "observer" => READ_OPERATIONS.contains(&operation),
        "moderator" => {
            READ_OPERATIONS.contains(&operation) || MODERATION_OPERATIONS.contains(&operation)
        }
        "automationEditor" => {
            READ_OPERATIONS.contains(&operation) || operation == "automation.draft.submit"
        }
        "custom" => policy
            .get("allowedOperations")
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(operation))),
        _ => false,
    }
}

async fn execute(core: &Weak<AppCore>, operation: &str) -> Result<Value, &'static str> {
    let core = core.upgrade().ok_or("unavailable")?;
    match operation {
        "live.status" => serde_json::to_value(core.live_status()).map_err(|_| "internal"),
        _ => Err("unsupported"),
    }
}

async fn send_result(
    socket: &mut Socket,
    command_id: &str,
    status: &str,
    code: Option<&str>,
    result: Option<Value>,
) -> Result<(), OperationError> {
    let mut frame = json!({"type": "result", "commandId": command_id, "status": status});
    if let Some(code) = code {
        frame["code"] = json!(code);
    }
    if let Some(result) = result {
        frame["result"] = result;
    }
    socket
        .send(Message::Text(frame.to_string().into()))
        .await
        .map_err(|_| OperationError::unavailable("the relay connection was lost"))
}

fn valid_command_id(value: &str) -> bool {
    (8..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
}

fn socket_url(origin: &str) -> Result<url::Url, OperationError> {
    let mut url =
        url::Url::parse(origin).map_err(|_| OperationError::unavailable("invalid Cloud origin"))?;
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        _ => return Err(OperationError::unavailable("invalid Cloud origin")),
    };
    url.set_scheme(scheme)
        .map_err(|_| OperationError::unavailable("invalid Cloud origin"))?;
    url.set_path("/api/v3/remote/socket");
    Ok(url)
}

fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Parses the server's `to_rfc3339()` output in UTC, e.g. `2026-11-10T12:00:00+00:00`.
pub fn parse_utc_seconds(value: &str) -> Option<i64> {
    let (date, rest) = value.split_once('T')?;
    let time = rest
        .strip_suffix("+00:00")
        .or_else(|| rest.strip_suffix('Z'))?;
    let time = time.split('.').next()?;
    let mut d = date.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let mut t = time.split(':').map(|p| p.parse::<i64>().ok());
    let (hh, mm, ss) = (t.next()??, t.next()??, t.next()??);
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    // Days from civil (Howard Hinnant), proleptic Gregorian.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + hh * 3600 + mm * 60 + ss)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_utc_values_parse_to_epoch_seconds() {
        assert_eq!(parse_utc_seconds("1970-01-01T00:00:00+00:00"), Some(0));
        assert_eq!(
            parse_utc_seconds("2026-11-10T12:00:00.123456+00:00"),
            Some(1_794_312_000)
        );
        assert_eq!(
            parse_utc_seconds("2026-11-10T12:00:00Z"),
            Some(1_794_312_000)
        );
        assert_eq!(parse_utc_seconds("2026-13-10T12:00:00Z"), None);
        assert_eq!(parse_utc_seconds("not a time"), None);
    }

    #[test]
    fn role_table_admits_only_declared_operations() {
        assert!(allowed("observer", &json!({}), "live.status"));
        assert!(!allowed("observer", &json!({}), "points.adjust"));
        assert!(allowed("moderator", &json!({}), "points.adjust"));
        assert!(!allowed(
            "automationEditor",
            &json!({}),
            "automation.toggle"
        ));
        assert!(allowed(
            "custom",
            &json!({"allowedOperations": ["live.status"]}),
            "live.status"
        ));
        assert!(!allowed("custom", &json!({}), "live.status"));
        assert!(!allowed("admin", &json!({}), "live.status"));
    }

    #[test]
    fn command_ids_follow_the_relay_alphabet() {
        assert!(valid_command_id("cmd-2026.11_10:01"));
        assert!(!valid_command_id("short"));
        assert!(!valid_command_id("has space in it"));
    }

    #[test]
    fn socket_url_switches_to_websocket_schemes() {
        assert_eq!(
            socket_url("https://cloud.example").unwrap().as_str(),
            "wss://cloud.example/api/v3/remote/socket"
        );
        assert_eq!(
            socket_url("http://127.0.0.1:8080").unwrap().as_str(),
            "ws://127.0.0.1:8080/api/v3/remote/socket"
        );
        assert!(socket_url("ftp://cloud.example").is_err());
    }
}
