//! Reference Twitch live-platform plugin.
//!
//! The plugin owns only anonymous read-only Twitch transport, reconnects,
//! IRC parsing and provider-to-canonical mapping. All feed, points, automation,
//! identity, replay and analytics authority remains in the TikTools host.

mod irc;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Map, Value};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tiktools_live::{
    Capabilities, ChannelInput, LiveEventInput, LiveIngestBatch, LivePayload,
    PlatformIngestMessage, ViewerInput, INGEST_EVENT_TYPE, MAX_BATCH_MESSAGES,
};
use tiktools_plugin_sdk::{
    data_dir, run_process_plugin, ActionCall, ActionResult, Plugin, PluginContext, PluginError,
    PluginEvent, PluginResult, PollResult,
};
use tokio_util::sync::CancellationToken;

const STREAM_KEY: &str = "main";
const MAX_PENDING_MESSAGES: usize = 1024;
// Twitch IRC frames are small, but the host also bounds every plugin event
// payload. Batching fewer messages keeps one poll response comfortably below
// that transport limit even with long chat text and metadata.
const PLUGIN_BATCH_MESSAGES: usize = 32;
// Stay comfortably below the host's 64 KiB plugin-event transport bound even
// when IRC metadata is large. Oversized single messages can never be accepted
// downstream, so they are accounted as an explicit provider gap instead.
const MAX_INGEST_BATCH_BYTES: usize = 48 * 1024;
const MAX_INGEST_MESSAGE_BYTES: usize = 32 * 1024;

fn encoded_len(message: &PlatformIngestMessage) -> usize {
    serde_json::to_vec(message).map_or(usize::MAX, |bytes| bytes.len())
}

#[derive(Default)]
struct SharedQueue {
    messages: VecDeque<PlatformIngestMessage>,
    lost: u64,
}

impl SharedQueue {
    fn push(&mut self, message: PlatformIngestMessage) {
        if encoded_len(&message) > MAX_INGEST_MESSAGE_BYTES {
            self.lost = self.lost.saturating_add(1);
            return;
        }
        if self.messages.len() >= MAX_PENDING_MESSAGES {
            self.lost = self.lost.saturating_add(1);
            return;
        }
        if self.lost > 0 {
            let lost = std::mem::take(&mut self.lost);
            self.messages.push_back(PlatformIngestMessage::Gap {
                stream: STREAM_KEY.into(),
                lost,
            });
        }
        self.messages.push_back(message);
    }
}

type Queue = Arc<Mutex<SharedQueue>>;

fn push(queue: &Queue, message: PlatformIngestMessage) {
    if let Ok(mut shared) = queue.lock() {
        shared.push(message);
    }
}

#[derive(Default)]
struct TwitchPlugin {
    runtime: Mutex<Option<tokio::runtime::Runtime>>,
    stop: Mutex<Option<CancellationToken>>,
    queue: Queue,
    status: Arc<Mutex<String>>,
    worker_alive: Arc<AtomicBool>,
}

impl TwitchPlugin {
    fn set_status(&self, status: impl Into<String>) {
        if let Ok(mut slot) = self.status.lock() {
            *slot = status.into();
        }
    }

    fn start(&self, channel: String) -> PluginResult<()> {
        {
            let runtime = self
                .runtime
                .lock()
                .map_err(|error| PluginError::other(error.to_string()))?;
            if runtime.is_some() && self.worker_alive.load(Ordering::SeqCst) {
                return Err(PluginError::other("Twitch connection is already running"));
            }
        }
        if self
            .runtime
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))?
            .is_some()
        {
            self.stop_connection()?;
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|error| PluginError::other(error.to_string()))?;
        let stop = CancellationToken::new();
        let queue = self.queue.clone();
        let worker_stop = stop.clone();
        let worker_alive = self.worker_alive.clone();
        let status = self.status.clone();
        worker_alive.store(true, Ordering::SeqCst);
        runtime.spawn(async move {
            run_connection_loop(channel, queue, worker_stop, status).await;
            worker_alive.store(false, Ordering::SeqCst);
        });
        *self
            .runtime
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))? = Some(runtime);
        *self
            .stop
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))? = Some(stop);
        self.set_status("connecting");
        Ok(())
    }

    fn stop_connection(&self) -> PluginResult<()> {
        self.worker_alive.store(false, Ordering::SeqCst);
        if let Some(stop) = self
            .stop
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))?
            .take()
        {
            stop.cancel();
        }
        if let Some(runtime) = self
            .runtime
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))?
            .take()
        {
            runtime.shutdown_timeout(Duration::from_secs(5));
        }
        self.set_status("stopped");
        Ok(())
    }
}

impl Drop for TwitchPlugin {
    fn drop(&mut self) {
        let _ = self.stop_connection();
    }
}

fn configured_value(config: &Map<String, Value>, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            let plugin_id = std::env::var("TIKTOOLS_PLUGIN_ID").ok()?;
            let path = data_dir().ok()?.join(plugin_id).join("settings.json");
            let value: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
}

impl Plugin for TwitchPlugin {
    fn action(&mut self, _context: &PluginContext, call: ActionCall) -> PluginResult<ActionResult> {
        match call.action_type() {
            Some("tiktools.twitch.connect") => {
                let channel = configured_value(call.config(), "channel")
                    .ok_or_else(|| PluginError::other("channel is required"))?
                    .to_ascii_lowercase();
                if !valid_login(&channel) {
                    return Err(PluginError::other("channel must be a Twitch login name"));
                }
                self.start(channel)?;
                Ok(ActionResult::summary(
                    "Twitch anonymous read-only connection started",
                ))
            }
            Some("tiktools.twitch.disconnect") => {
                self.stop_connection()?;
                push(
                    &self.queue,
                    PlatformIngestMessage::SessionEnd {
                        stream: STREAM_KEY.into(),
                        error: None,
                    },
                );
                Ok(ActionResult::summary("Twitch connection stopped"))
            }
            Some("tiktools.twitch.status") => {
                let status = self
                    .status
                    .lock()
                    .map_err(|error| PluginError::other(error.to_string()))?
                    .clone();
                let running = self.worker_alive.load(Ordering::SeqCst);
                Ok(ActionResult::summary(format!(
                    "Twitch: {} ({})",
                    if running { "running" } else { "stopped" },
                    if status.is_empty() { "idle".into() } else { status }
                )))
            }
            other => Err(PluginError::other(format!("unknown action {other:?}"))),
        }
    }

    fn poll(&mut self, _context: &PluginContext) -> PluginResult<PollResult> {
        let mut result = PollResult::default();
        loop {
            let messages = {
                let mut shared = self
                    .queue
                    .lock()
                    .map_err(|error| PluginError::other(error.to_string()))?;
                if shared.messages.is_empty() {
                    Vec::new()
                } else {
                    let max_count = MAX_BATCH_MESSAGES.min(PLUGIN_BATCH_MESSAGES);
                    // JSON array/comma/envelope overhead is small but nonzero.
                    // Measure before draining so a boundary batch remains
                    // queued instead of being rejected wholesale by the host.
                    let mut encoded_bytes = 64usize;
                    let mut count = 0usize;
                    for message in shared.messages.iter().take(max_count) {
                        let message_bytes = encoded_len(message).saturating_add(1);
                        if encoded_bytes.saturating_add(message_bytes) > MAX_INGEST_BATCH_BYTES {
                            break;
                        }
                        encoded_bytes += message_bytes;
                        count += 1;
                    }
                    shared.messages.drain(..count).collect()
                }
            };
            if messages.is_empty() {
                break;
            }
            result.events.push(PluginEvent::new(
                INGEST_EVENT_TYPE,
                LiveIngestBatch::new(messages),
            )?);
            if result.events.len() >= tiktools_plugin_sdk::POLL_MAX_EVENTS_PER_RESPONSE {
                break;
            }
        }
        Ok(result)
    }

    fn shutdown(&mut self, _context: &PluginContext) -> PluginResult<()> {
        self.stop_connection()
    }
}

async fn run_connection_loop(channel: String, queue: Queue, stop: CancellationToken, status: Arc<Mutex<String>>) {
    for attempt in 0..=5_u32 {
        if stop.is_cancelled() {
            break;
        }
        let failure = tokio::select! {
            biased;
            () = stop.cancelled() => break,
            result = session(&channel, &queue, &stop, &status) => result,
        };
        if let Err(error) = failure {
            let retry = (2_000_u64 << attempt).min(60_000);
            if attempt == 5 || error.code != "transport" {
                if let Ok(mut slot) = status.lock() {
                    *slot = format!("failed: {}", error.message);
                }
                push(
                    &queue,
                    PlatformIngestMessage::SessionEnd {
                        stream: STREAM_KEY.into(),
                        error: Some(tiktools_live::PlatformError {
                            code: error.code,
                            message: error.message,
                        }),
                    },
                );
                break;
            }
            if let Ok(mut slot) = status.lock() {
                *slot = format!("reconnecting: {}", error.message);
            }
            push(
                &queue,
                PlatformIngestMessage::SessionState {
                    stream: STREAM_KEY.into(),
                    state: tiktools_live::SessionState::Reconnecting,
                    reconnect_attempt: attempt + 1,
                    retry_after_ms: Some(retry),
                    error: None,
                },
            );
            tokio::select! {
                biased;
                () = stop.cancelled() => break,
                () = tokio::time::sleep(Duration::from_millis(retry)) => {}
            }
        } else {
            break;
        }
    }
}

#[derive(Debug)]
struct TwitchError {
    code: String,
    message: String,
}

impl TwitchError {
    fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

fn anonymous_login() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let suffix = now
        .as_secs()
        .wrapping_mul(1_000_000_000)
        .wrapping_add(u64::from(now.subsec_nanos()))
        .wrapping_add(u64::from(std::process::id()))
        % 10_000_000_000_000_000;
    format!("justinfan{suffix:016}")
}

// This reference intentionally uses Twitch's legacy anonymous `justinfan` login.
// Production providers should replace it with authenticated IRC or EventSub.
fn registration_commands(channel: &str, nick: &str) -> [String; 3] {
    [
        "CAP REQ :twitch.tv/tags twitch.tv/commands\r\n".to_owned(),
        format!("NICK {nick}\r\n"),
        format!("JOIN #{channel}\r\n"),
    ]
}

async fn session(
    channel: &str,
    queue: &Queue,
    stop: &CancellationToken,
    status: &Arc<Mutex<String>>,
) -> Result<(), TwitchError> {
    let config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(1024 * 1024))
        .max_frame_size(Some(1024 * 1024));
    let (mut socket, _) = tokio::time::timeout(
        Duration::from_secs(20),
        tokio_tungstenite::connect_async_with_config(
            "wss://irc-ws.chat.twitch.tv:443",
            Some(config),
            false,
        ),
    )
    .await
    .map_err(|_| TwitchError::new("transport", "Twitch connection timed out"))?
    .map_err(|_| TwitchError::new("transport", "Could not connect to Twitch chat"))?;
    for command in registration_commands(channel, &anonymous_login()) {
        socket
            .send(tokio_tungstenite::tungstenite::Message::Text(command.into()))
            .await
            .map_err(|_| TwitchError::new("transport", "Twitch registration could not be sent"))?;
    }
    let mut ready = false;
    loop {
        let next = tokio::select! {
            biased;
            () = stop.cancelled() => return Ok(()),
            result = tokio::time::timeout(Duration::from_secs(300), socket.next()) => result.map_err(|_| TwitchError::new("transport", "Twitch chat heartbeat timed out"))?,
        };
        let message = next
            .ok_or_else(|| TwitchError::new("transport", "Twitch closed the chat connection"))?
            .map_err(|_| TwitchError::new("transport", "Twitch chat connection interrupted"))?;
        let text = match message {
            tokio_tungstenite::tungstenite::Message::Text(text) => text,
            tokio_tungstenite::tungstenite::Message::Ping(bytes) => {
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Pong(bytes))
                    .await
                    .map_err(|_| TwitchError::new("transport", "Could not acknowledge Twitch heartbeat"))?;
                continue;
            }
            tokio_tungstenite::tungstenite::Message::Close(_) => {
                return Err(TwitchError::new("transport", "Twitch closed the chat connection"));
            }
            _ => continue,
        };
        for line in text.split('\n').filter(|line| !line.is_empty()) {
            let Some(message) = irc::parse(line) else {
                push(
                    queue,
                    PlatformIngestMessage::Gap {
                        stream: STREAM_KEY.into(),
                        lost: 1,
                    },
                );
                continue;
            };
            if message.command == "PING" {
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        format!("PONG :{}\r\n", message.trailing.unwrap_or("tmi.twitch.tv")).into(),
                    ))
                    .await
                    .map_err(|_| TwitchError::new("transport", "Could not acknowledge Twitch heartbeat"))?;
                continue;
            }
            if message.command == "RECONNECT" {
                return Err(TwitchError::new("transport", "Twitch requested reconnection"));
            }
            if message.target != Some(format!("#{channel}").as_str()) {
                continue;
            }
            if message.command == "ROOMSTATE" {
                if ready {
                    continue;
                }
                let room = message
                    .tags
                    .get("room-id")
                    .filter(|room| !room.is_empty())
                    .ok_or_else(|| TwitchError::new("invalid_response", "Twitch did not verify the channel ID"))?;
                if let Ok(mut slot) = status.lock() {
                    *slot = format!("connected to #{channel}");
                }
                push(
                    queue,
                    PlatformIngestMessage::SessionStart {
                        stream: STREAM_KEY.into(),
                        channel: ChannelInput {
                            id: room.clone(),
                            handle: channel.to_owned(),
                            room_id: room.clone(),
                        },
                        capabilities: Capabilities {
                            chat: true,
                            subscriptions: true,
                            gifts: true,
                            ..Default::default()
                        },
                    },
                );
                ready = true;
                continue;
            }
            if !ready {
                continue;
            }
            if let Some((user, payload, extensions)) = normalize(&message) {
                let occurred_at = message
                    .tags
                    .get("tmi-sent-ts")
                    .and_then(|value| value.parse().ok());
                if let Some(bits) = message
                    .tags
                    .get("bits")
                    .and_then(|value| value.parse::<u64>().ok())
                    .filter(|bits| *bits > 0)
                {
                    push(
                        queue,
                        event_message(
                            message.tags.get("id").cloned(),
                            occurred_at,
                            user.clone(),
                            LivePayload::Gift {
                                id: "bits".into(),
                                name: "Bits".into(),
                                count: 1,
                                unit: "twitch_bits".into(),
                                amount: bits.to_string(),
                            },
                            extensions.clone(),
                        ),
                    );
                }
                push(
                    queue,
                    event_message(
                        message.tags.get("id").filter(|id| !id.is_empty()).cloned(),
                        occurred_at,
                        user,
                        payload,
                        extensions,
                    ),
                );
            }
        }
    }
}

fn event_message(
    source_event_id: Option<String>,
    occurred_at: Option<u64>,
    user: ViewerInput,
    payload: LivePayload,
    extensions: Map<String, Value>,
) -> PlatformIngestMessage {
    PlatformIngestMessage::Event {
        stream: STREAM_KEY.into(),
        event: LiveEventInput {
            source_event_id,
            occurred_at,
            is_history: false,
            user: Some(user),
            payload,
            extensions,
        },
    }
}

fn normalize(message: &irc::Message<'_>) -> Option<(ViewerInput, LivePayload, Map<String, Value>)> {
    let id = message.tags.get("user-id").filter(|id| !id.is_empty())?;
    let handle = message
        .prefix
        .and_then(|prefix| prefix.split('!').next())
        .unwrap_or("");
    let user = ViewerInput {
        id: id.clone(),
        handle: handle.into(),
        display_name: message
            .tags
            .get("display-name")
            .cloned()
            .unwrap_or_else(|| handle.into()),
        avatar_url: None,
    };
    let mut extensions = Map::new();
    extensions.insert(
        "twitch".into(),
        json!({
            "badges":message.tags.get("badges"),
            "emotes":message.tags.get("emotes"),
            "color":message.tags.get("color"),
            "bits":message.tags.get("bits"),
        }),
    );
    let payload = match message.command {
        "PRIVMSG" => LivePayload::Chat {
            comment: message.trailing.unwrap_or("").into(),
        },
        "USERNOTICE" => match message.tags.get("msg-id").map(String::as_str) {
            Some("sub" | "resub" | "subgift" | "anonsubgift") => LivePayload::Subscription {
                tier: message.tags.get("msg-param-sub-plan").cloned(),
                months: message
                    .tags
                    .get("msg-param-cumulative-months")
                    .and_then(|value| value.parse().ok()),
                gifted: matches!(
                    message.tags.get("msg-id").map(String::as_str),
                    Some("subgift" | "anonsubgift")
                ),
            },
            _ => return None,
        },
        _ => return None,
    };
    Some((user, payload, extensions))
}

fn valid_login(login: &str) -> bool {
    !login.is_empty()
        && login.len() <= 25
        && login
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn main() -> PluginResult<()> {
    tiktools_tls::ensure_default_crypto_provider();
    run_process_plugin::<TwitchPlugin>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_identity_and_bits_are_not_converted_to_diamonds() {
        let message = irc::parse("@id=msg-1;user-id=42;display-name=Alice;bits=100 :alice!a@b PRIVMSG #room :cheer100 hello").unwrap();
        let (user, payload, extensions) = normalize(&message).unwrap();
        assert_eq!(user.id, "42");
        assert!(matches!(payload, LivePayload::Chat { .. }));
        assert_eq!(extensions["twitch"]["bits"], "100");
        assert!(extensions["twitch"].get("diamonds").is_none());
    }

    #[test]
    fn plugin_manifest_is_valid_and_does_not_request_oauth() {
        let manifest = tiktools_plugin_sdk::tiktools_plugin_api::PluginManifest::from_json_str(
            include_str!("../plugin.json"),
        )
        .unwrap();
        assert_eq!(manifest.id, "tiktools.twitch");
        let settings = manifest.settings_schema.unwrap();
        assert!(settings["properties"].get("channel").is_some());
        assert!(settings["properties"].get("oauthToken").is_none());
    }

    #[test]
    fn anonymous_registration_is_read_only_and_uses_no_password() {
        let nick = anonymous_login();
        assert!(nick.starts_with("justinfan"));
        assert_eq!(nick.len(), 25);
        assert!(valid_login(&nick));
        let commands = registration_commands("room", &nick);
        assert_eq!(
            commands[0],
            "CAP REQ :twitch.tv/tags twitch.tv/commands\r\n"
        );
        assert_eq!(commands[1], format!("NICK {nick}\r\n"));
        assert_eq!(commands[2], "JOIN #room\r\n");
        assert!(commands
            .iter()
            .all(|command| !command.starts_with("PASS") && !command.contains("oauth")));
    }

    #[test]
    fn rejects_command_injection_and_unidentified_messages() {
        assert!(!valid_login("room\r\nJOIN #other"));
        assert!(normalize(&irc::parse(":alice!a@b PRIVMSG #room :hello").unwrap()).is_none());
    }
}
