//! Host side of the Cloud remote-control relay (v3).
//!
//! The PC dials out over WSS, so controllers never reach it directly. It confirms
//! every grant the owner approved, admits a command only when the grant is active
//! and the operation is allowed for its role, and executes only the read
//! operations implemented here, as bounded summaries. Every other operation is
//! answered as unsupported: the moderation and draft operations would have to run
//! under a local collaboration principal and journal, which a Cloud grant is not
//! yet mapped to (see docs/CLOUD-UNIFIED.md).

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
const EXECUTABLE: &[&str] = &[
    "live.status",
    "workflow.list",
    "automation.list",
    "competition.list",
    "points.read",
];

/// Items returned by one list operation, and the longest text kept from any field.
const LIST_CAP: usize = 50;
const TEXT_CAP: usize = 200;
/// The only fields a summary copies. Definitions can embed URLs, headers or
/// credentials; none of that leaves the PC.
const SUMMARY_FIELDS: &[&str] = &[
    "id", "name", "title", "enabled", "kind", "trigger", "status",
];

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
    // The relay sends the expiry as integer milliseconds since the Unix epoch.
    let expires = frame.get("expiresAt").and_then(Value::as_i64);

    send_result(socket, &command_id, "running", None, None).await?;
    let outcome = match (expires, now_millis()) {
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
        Ok(()) => {
            match execute(core, &operation, frame.get("args").unwrap_or(&Value::Null)).await {
                Ok(result)
                    if serde_json::to_vec(&result)
                        .map(|b| b.len())
                        .unwrap_or(usize::MAX)
                        <= MAX_RESULT_BYTES =>
                {
                    send_result(socket, &command_id, "completed", None, Some(result)).await
                }
                Ok(_) => {
                    send_result(socket, &command_id, "failed", Some("resultTooLarge"), None).await
                }
                Err(code) => send_result(socket, &command_id, "failed", Some(code), None).await,
            }
        }
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

async fn execute(
    core: &Weak<AppCore>,
    operation: &str,
    args: &Value,
) -> Result<Value, &'static str> {
    let core = core.upgrade().ok_or("unavailable")?;
    match operation {
        "live.status" => serde_json::to_value(core.live_status()).map_err(|_| "internal"),
        "workflow.list" => {
            let items = core.workflow_list().map_err(|_| "unavailable")?;
            Ok(summarize(&items))
        }
        "automation.list" => {
            use crate::control::AutomationKind;
            let kinds = match args.get("kind").and_then(Value::as_str) {
                None => vec![AutomationKind::Event, AutomationKind::Action],
                Some("event") => vec![AutomationKind::Event],
                Some("action") => vec![AutomationKind::Action],
                Some(_) => return Err("invalidArguments"),
            };
            let items: Vec<Value> = kinds
                .into_iter()
                .flat_map(|kind| core.automation_list(kind))
                .collect();
            Ok(summarize(&items))
        }
        "competition.list" => {
            let definitions = core.competition_list().map_err(|_| "unavailable")?;
            let items: Vec<Value> = definitions
                .iter()
                .filter_map(|definition| serde_json::to_value(definition).ok())
                .collect();
            Ok(summarize(&items))
        }
        "points.read" => {
            if let Some(viewer) = args.get("uniqueId") {
                let viewer = viewer.as_str().ok_or("invalidArguments")?;
                return core.points_viewer(viewer).map_err(|error| {
                    if error.code == "not_found" {
                        "notFound"
                    } else {
                        "invalidArguments"
                    }
                });
            }
            let limit = match args.get("limit") {
                None => LIST_CAP as i64,
                Some(value) => value
                    .as_i64()
                    .filter(|limit| (1..=LIST_CAP as i64).contains(limit))
                    .ok_or("invalidArguments")?,
            };
            Ok(json!({"items": core.points_leaderboard(Some(limit)), "limit": limit}))
        }
        _ => Err("unsupported"),
    }
}

/// Reduces records to the allowed scalar fields, bounded in count and text length.
fn summarize(items: &[Value]) -> Value {
    let rows: Vec<Value> = items
        .iter()
        .take(LIST_CAP)
        .map(|item| {
            let mut row = serde_json::Map::new();
            for key in SUMMARY_FIELDS {
                match item.get(*key) {
                    Some(Value::String(text)) => {
                        let text: String = text.chars().take(TEXT_CAP).collect();
                        row.insert((*key).to_owned(), Value::String(text));
                    }
                    Some(value @ (Value::Bool(_) | Value::Number(_))) => {
                        row.insert((*key).to_owned(), value.clone());
                    }
                    _ => {}
                }
            }
            Value::Object(row)
        })
        .collect();
    json!({"items": rows, "total": items.len(), "truncated": items.len() > LIST_CAP})
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

pub(crate) fn socket_url(origin: &str) -> Result<url::Url, OperationError> {
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

pub(crate) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn summaries_keep_only_allowed_scalars_and_bound_size() {
        let secret = "https://example.test/hook?token=SECRET";
        let items: Vec<Value> = (0..60)
            .map(|index| {
                json!({
                    "id": format!("w-{index}"),
                    "name": "x".repeat(500),
                    "enabled": true,
                    "graph": {"nodes": [{"url": secret}]},
                    "headers": {"authorization": "Bearer SECRET"},
                    "trigger": {"nested": "object is dropped"},
                })
            })
            .collect();
        let summary = summarize(&items);
        assert_eq!(summary["items"].as_array().unwrap().len(), LIST_CAP);
        assert_eq!(summary["total"], 60);
        assert_eq!(summary["truncated"], true);
        let first = &summary["items"][0];
        assert_eq!(first["name"].as_str().unwrap().chars().count(), TEXT_CAP);
        assert_eq!(first["enabled"], true);
        assert!(first.get("graph").is_none() && first.get("trigger").is_none());
        assert!(!summary.to_string().contains("SECRET"));
    }

    #[test]
    fn only_read_operations_are_executable() {
        for operation in EXECUTABLE {
            assert!(
                allowed("observer", &json!({}), operation),
                "{operation} must be a read operation"
            );
        }
        for operation in MODERATION_OPERATIONS {
            assert!(!EXECUTABLE.contains(operation), "{operation}");
        }
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
