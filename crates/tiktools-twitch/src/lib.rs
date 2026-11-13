//! Twitch IRC over TLS WebSocket. Read-only: `chat:read` user OAuth token.
//! https://dev.twitch.tv/docs/chat/irc/
//! https://dev.twitch.tv/docs/authentication/validate-tokens/
mod irc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::time::Duration;
use tiktools_live::*;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
};
use tokio_util::sync::CancellationToken;

pub struct TwitchAdapter {
    http: reqwest::Client,
    validate_url: String,
    socket_url: String,
}
impl TwitchAdapter {
    pub fn new() -> Result<Self, LiveError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| {
                LiveError::new("unavailable", "Could not initialize Twitch HTTP client")
            })?;
        Ok(Self {
            http,
            validate_url: "https://id.twitch.tv/oauth2/validate".into(),
            socket_url: "wss://irc-ws.chat.twitch.tv:443".into(),
        })
    }
    async fn validate(&self, token: &str) -> Result<TokenIdentity, LiveError> {
        let response = self
            .http
            .get(&self.validate_url)
            .header("Authorization", format!("OAuth {token}"))
            .send()
            .await
            .map_err(|_| {
                LiveError::new("transport", "Twitch token validation could not be reached")
            })?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let seconds = response
                .headers()
                .get("Retry-After")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60)
                .clamp(1, 3600);
            return Err(LiveError {
                code: "rate_limited".into(),
                message: "Twitch token validation is rate limited".into(),
                retry_after_ms: Some(seconds * 1000),
            });
        }
        if response.status().is_server_error() {
            return Err(LiveError::new(
                "transport",
                "Twitch token validation is temporarily unavailable",
            ));
        }
        if !response.status().is_success() {
            return Err(LiveError::new(
                "unauthenticated",
                "Twitch access token was rejected; sign in again",
            ));
        }
        let identity: TokenIdentity = response.json().await.map_err(|_| {
            LiveError::new(
                "invalid_response",
                "Invalid Twitch token validation response",
            )
        })?;
        if !valid_login(&identity.login)
            || identity.user_id.is_empty()
            || !identity.scopes.iter().any(|s| s == "chat:read")
            || identity.expires_in < 60
        {
            return Err(LiveError::new(
                "unauthenticated",
                "A valid Twitch user token with chat:read permission is required",
            ));
        }
        Ok(identity)
    }
    async fn session(&self, request: &AdapterRequest, sink: &EventSink) -> Result<(), LiveError> {
        let token = request
            .credential
            .expose()
            .strip_prefix("oauth:")
            .unwrap_or(request.credential.expose());
        if token.is_empty()
            || token.len() > 4096
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
        {
            return Err(LiveError::new(
                "unauthenticated",
                "Invalid Twitch OAuth token",
            ));
        }
        let identity = self.validate(token).await?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(1024 * 1024))
            .max_frame_size(Some(1024 * 1024));
        let (mut socket, _) = tokio::time::timeout(
            Duration::from_secs(20),
            connect_async_with_config(&self.socket_url, Some(config), false),
        )
        .await
        .map_err(|_| LiveError::new("transport", "Twitch connection timed out"))?
        .map_err(|_| LiveError::new("transport", "Could not connect to Twitch chat"))?;
        let login = identity.login;
        let channel = request.target.trim_start_matches('#').to_ascii_lowercase();
        for command in [
            "CAP REQ :twitch.tv/tags twitch.tv/commands\r\n".to_owned(),
            format!("PASS oauth:{token}\r\n"),
            format!("NICK {login}\r\n"),
            format!("JOIN #{channel}\r\n"),
        ] {
            socket
                .send(Message::Text(command.into()))
                .await
                .map_err(|_| {
                    LiveError::new("transport", "Twitch authentication could not be sent")
                })?;
        }
        let mut ready = false;
        let join_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        let token_lifetime =
            Duration::from_secs(identity.expires_in.min(3600).saturating_sub(30).max(1));
        let mut validate =
            tokio::time::interval_at(tokio::time::Instant::now() + token_lifetime, token_lifetime);
        loop {
            let next = tokio::select! {
                result = tokio::time::timeout(Duration::from_secs(300), socket.next()) => result.map_err(|_| LiveError::new("transport", "Twitch chat heartbeat timed out"))?,
                () = tokio::time::sleep_until(join_deadline), if !ready => return Err(LiveError::new("transport", "Twitch channel join timed out")),
                _ = validate.tick() => { self.validate(token).await?; continue; }
            };
            let message = next
                .ok_or_else(|| LiveError::new("transport", "Twitch closed the chat connection"))?
                .map_err(|_| LiveError::new("transport", "Twitch chat connection interrupted"))?;
            let text = match message {
                Message::Text(text) => text,
                Message::Ping(bytes) => {
                    socket.send(Message::Pong(bytes)).await.map_err(|_| {
                        LiveError::new("transport", "Could not acknowledge Twitch heartbeat")
                    })?;
                    continue;
                }
                Message::Close(_) => {
                    return Err(LiveError::new(
                        "transport",
                        "Twitch closed the chat connection",
                    ))
                }
                _ => continue,
            };
            for line in text.split('\n').filter(|s| !s.is_empty()) {
                let Some(m) = irc::parse(line) else {
                    sink.gap(1).await;
                    continue;
                };
                if m.command == "PING" {
                    socket
                        .send(Message::Text(
                            format!("PONG :{}\r\n", m.trailing.unwrap_or("tmi.twitch.tv")).into(),
                        ))
                        .await
                        .map_err(|_| {
                            LiveError::new("transport", "Could not acknowledge Twitch heartbeat")
                        })?;
                    continue;
                }
                if m.command == "RECONNECT" {
                    return Err(LiveError::new("transport", "Twitch requested reconnection"));
                }
                if m.command == "NOTICE" {
                    let notice = m.tags.get("msg-id").map(String::as_str).unwrap_or("");
                    let text = m.trailing.unwrap_or("");
                    if text.contains("authentication failed")
                        || text.contains("Improperly formatted auth")
                        || text.contains("Login unsuccessful")
                    {
                        return Err(LiveError::new(
                            "unauthenticated",
                            "Twitch IRC authentication failed; sign in again",
                        ));
                    }
                    if matches!(notice, "msg_banned" | "msg_channel_suspended" | "tos_ban") {
                        return Err(LiveError::new(
                            "forbidden",
                            "Twitch denied access to this channel",
                        ));
                    }
                    if notice.contains("ratelimit") {
                        return Err(LiveError {
                            code: "rate_limited".into(),
                            message: "Twitch chat is rate limited".into(),
                            retry_after_ms: Some(60_000),
                        });
                    }
                }
                if m.target != Some(format!("#{channel}").as_str()) {
                    continue;
                }
                if m.command == "ROOMSTATE" {
                    if ready {
                        continue;
                    }
                    let room =
                        m.tags
                            .get("room-id")
                            .filter(|s| !s.is_empty())
                            .ok_or_else(|| {
                                LiveError::new(
                                    "invalid_response",
                                    "Twitch did not verify the channel ID",
                                )
                            })?;
                    sink.connected(ChannelIdentity {
                        platform: PlatformId::Twitch,
                        id: room.clone(),
                        handle: channel.clone(),
                        room_id: room.clone(),
                    })
                    .await?;
                    ready = true;
                    continue;
                }
                if !ready {
                    continue;
                }
                if let Some((user, payload, extensions)) = normalize(&m) {
                    let at = m
                        .tags
                        .get("tmi-sent-ts")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or_else(now_millis);
                    if let Some(bits) = m
                        .tags
                        .get("bits")
                        .and_then(|s| s.parse::<u64>().ok())
                        .filter(|n| *n > 0)
                    {
                        sink.event(
                            m.tags.get("id").cloned(),
                            at,
                            false,
                            user.clone(),
                            LivePayload::Gift {
                                id: "bits".into(),
                                name: "Bits".into(),
                                count: 1,
                                unit: "twitch_bits".into(),
                                amount: bits.to_string(),
                            },
                            extensions.clone(),
                        )
                        .await?;
                    }
                    sink.event(
                        m.tags.get("id").filter(|s| !s.is_empty()).cloned(),
                        at,
                        false,
                        user,
                        payload,
                        extensions,
                    )
                    .await?;
                }
            }
        }
    }
}
impl Adapter for TwitchAdapter {
    fn normalize_target(&self, target: &str) -> Result<String, LiveError> {
        let target = target.trim_start_matches('#').to_ascii_lowercase();
        if !valid_login(&target) {
            return Err(LiveError::new(
                "invalid_params",
                "Twitch channel must be a login name",
            ));
        }
        Ok(target)
    }
    fn descriptor(&self) -> PlatformDescriptor {
        PlatformDescriptor {
            platform: PlatformId::Twitch,
            name: "Twitch".into(),
            capabilities: Capabilities {
                chat: true,
                subscriptions: true,
                gifts: true,
                ..Default::default()
            },
            authentication: "oauth-token".into(),
        }
    }
    fn run<'a>(
        &'a self,
        request: AdapterRequest,
        sink: EventSink,
        stop: CancellationToken,
    ) -> BoxFuture<'a, Result<(), LiveError>> {
        Box::pin(async move {
            if !valid_login(request.target.trim_start_matches('#')) {
                return Err(LiveError::new(
                    "invalid_params",
                    "Twitch channel must be a login name",
                ));
            }
            for attempt in 0..=5_u32 {
                let failure = tokio::select! { biased; () = stop.cancelled() => return Ok(()), result = self.session(&request, &sink) => match result { Ok(()) => return Ok(()), Err(e) => e } };
                if !matches!(failure.code.as_str(), "transport" | "rate_limited") || attempt == 5 {
                    return Err(failure);
                }
                // Also bounds authentication/join rate across reconnects.
                let delay = failure
                    .retry_after_ms
                    .unwrap_or((2000_u64 << attempt).min(60_000));
                sink.reconnecting(attempt + 1, delay).await;
                tokio::select! { biased; () = stop.cancelled() => return Ok(()), () = tokio::time::sleep(Duration::from_millis(delay)) => {} }
            }
            unreachable!()
        })
    }
}
#[derive(Deserialize)]
struct TokenIdentity {
    login: String,
    user_id: String,
    scopes: Vec<String>,
    expires_in: u64,
}
fn valid_login(login: &str) -> bool {
    !login.is_empty()
        && login.len() <= 25
        && login
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn normalize(
    m: &irc::Message<'_>,
) -> Option<(Option<ViewerIdentity>, LivePayload, Map<String, Value>)> {
    let id = m.tags.get("user-id").filter(|s| !s.is_empty())?;
    let handle = m.prefix.and_then(|p| p.split('!').next()).unwrap_or("");
    let user = ViewerIdentity {
        platform: PlatformId::Twitch,
        id: id.clone(),
        handle: handle.into(),
        display_name: m
            .tags
            .get("display-name")
            .cloned()
            .unwrap_or_else(|| handle.into()),
        avatar_url: None,
    };
    // Retain only selected public platform metadata, never raw auth/server data.
    let mut ext = Map::new();
    ext.insert("twitch".into(), json!({"badges":m.tags.get("badges"),"emotes":m.tags.get("emotes"),"color":m.tags.get("color"),"bits":m.tags.get("bits")}));
    let payload = match m.command {
        "PRIVMSG" => LivePayload::Chat {
            comment: m.trailing.unwrap_or("").into(),
            emotes: vec![],
        },
        "USERNOTICE" => match m.tags.get("msg-id").map(String::as_str) {
            Some("sub" | "resub" | "subgift" | "anonsubgift") => LivePayload::Subscription {
                tier: m.tags.get("msg-param-sub-plan").cloned(),
                months: m
                    .tags
                    .get("msg-param-cumulative-months")
                    .and_then(|s| s.parse().ok()),
                gifted: matches!(
                    m.tags.get("msg-id").map(String::as_str),
                    Some("subgift" | "anonsubgift")
                ),
            },
            _ => return None,
        },
        _ => return None,
    };
    Some((Some(user), payload, ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chat_identity_and_bits_are_not_converted_to_diamonds() {
        let m = irc::parse("@id=msg-1;user-id=42;display-name=Alice;bits=100 :alice!a@b PRIVMSG #room :cheer100 hello").unwrap();
        let (user, payload, extensions) = normalize(&m).unwrap();
        assert_eq!(user.unwrap().key(), "twitch:2:42");
        assert!(matches!(payload, LivePayload::Chat { .. }));
        assert_eq!(extensions["twitch"]["bits"], "100");
        assert!(extensions["twitch"].get("diamonds").is_none());
    }
    #[test]
    fn rejects_command_injection_and_unidentified_messages() {
        assert!(!valid_login("room\r\nJOIN #other"));
        assert!(normalize(&irc::parse(":alice!a@b PRIVMSG #room :hello").unwrap()).is_none());
    }

    async fn validation_server(
        status: &str,
        body: Value,
        extra: &str,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/validate", listener.local_addr().unwrap());
        let body = body.to_string();
        let status = status.to_owned();
        let extra = extra.to_owned();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0u8; 8192];
            let n = socket.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..n])
                .to_ascii_lowercase()
                .contains("authorization: oauth fixture-token"));
            socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        });
        (url, task)
    }
    #[tokio::test]
    async fn websocket_fixture_authenticates_handles_ping_and_emits_typed_events() {
        use std::sync::Arc;
        let (validate_url, http) = validation_server(
            "200 OK",
            json!({"login":"reader","user_id":"1","scopes":["chat:read"],"expires_in":3600}),
            "",
        )
        .await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let socket_url = format!("ws://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
            let mut commands = String::new();
            for _ in 0..4 {
                commands.push_str(socket.next().await.unwrap().unwrap().to_text().unwrap());
            }
            assert!(commands.contains("PASS oauth:fixture-token\r\n"));
            assert!(commands.contains("NICK reader\r\n"));
            assert!(commands.contains("JOIN #channel\r\n"));
            socket
                .send(Message::Text("PING :tmi.twitch.tv\r\n".into()))
                .await
                .unwrap();
            assert_eq!(
                socket.next().await.unwrap().unwrap().to_text().unwrap(),
                "PONG :tmi.twitch.tv\r\n"
            );
            socket.send(Message::Text(concat!(
                "@room-id=99 :tmi.twitch.tv ROOMSTATE #channel\r\n",
                "@id=chat-1;room-id=99;user-id=42;display-name=Alice;bits=100;tmi-sent-ts=1000 :alice!a@b PRIVMSG #channel :cheer100 hello\r\n",
                "@id=sub-1;room-id=99;user-id=42;display-name=Alice;msg-id=sub;msg-param-sub-plan=1000 :tmi.twitch.tv USERNOTICE #channel :hello\r\n"
            ).into())).await.unwrap();
            let _ = socket.next().await;
        });
        let adapter = TwitchAdapter {
            http: reqwest::Client::builder().no_proxy().build().unwrap(),
            validate_url,
            socket_url,
        };
        let manager = ConnectionManager::default();
        manager.register(Arc::new(adapter)).unwrap();
        let mut events = manager.take_events().unwrap();
        let snapshot = manager
            .connect(
                PlatformId::Twitch,
                AdapterRequest {
                    target: "Channel".into(),
                    room_id: None,
                    credential: Credential::new("fixture-token".into()),
                },
            )
            .await
            .unwrap();
        let mut received = vec![];
        while received.len() < 3 {
            if let ManagedEvent::Event(event) =
                tokio::time::timeout(Duration::from_secs(5), events.recv())
                    .await
                    .unwrap()
                    .unwrap()
            {
                received.push(event);
            }
        }
        assert!(received.iter().all(|e| e.channel.id == "99"
            && e.platform == PlatformId::Twitch
            && e.user.as_ref().unwrap().id == "42"));
        assert!(received.iter().any(|e|matches!(&e.payload,LivePayload::Gift {unit,amount,..} if unit=="twitch_bits" && amount=="100")));
        assert!(received.iter().any(
            |e| matches!(&e.payload,LivePayload::Chat {comment,..} if comment=="cheer100 hello")
        ));
        assert!(received
            .iter()
            .any(|e| matches!(&e.payload, LivePayload::Subscription { gifted: false, .. })));
        assert!(!snapshot.capabilities.moderation);
        assert!(!snapshot.capabilities.likes);
        manager.disconnect(&snapshot.connection_id).await.unwrap();
        http.await.unwrap();
        server.await.unwrap();
    }
    #[tokio::test]
    async fn rejects_missing_scopes_and_honors_retry_after() {
        use std::sync::Arc;
        for limited in [false, true] {
            let (url, http) = if limited {
                validation_server("429 Too Many Requests", json!({}), "Retry-After: 17\r\n").await
            } else {
                validation_server(
                    "200 OK",
                    json!({"login":"reader","user_id":"1","scopes":[],"expires_in":3600}),
                    "",
                )
                .await
            };
            let adapter = TwitchAdapter {
                http: reqwest::Client::builder().no_proxy().build().unwrap(),
                validate_url: url,
                socket_url: "ws://127.0.0.1:1".into(),
            };
            let manager = ConnectionManager::default();
            manager.register(Arc::new(adapter)).unwrap();
            let mut events = manager.take_events().unwrap();
            let s = manager
                .connect(
                    PlatformId::Twitch,
                    AdapterRequest {
                        target: "channel".into(),
                        room_id: None,
                        credential: Credential::new("fixture-token".into()),
                    },
                )
                .await
                .unwrap();
            loop {
                if let ManagedEvent::State(s) =
                    tokio::time::timeout(Duration::from_secs(3), events.recv())
                        .await
                        .unwrap()
                        .unwrap()
                {
                    if limited && s.state == ConnectionState::Reconnecting {
                        assert_eq!(s.retry_after_ms, Some(17000));
                        break;
                    }
                    if !limited && s.state == ConnectionState::Error {
                        assert_eq!(s.error.unwrap().code, "unauthenticated");
                        break;
                    }
                }
            }
            assert!(!serde_json::to_string(&manager.snapshots())
                .unwrap()
                .contains("fixture-token"));
            manager.disconnect(&s.connection_id).await.unwrap();
            http.await.unwrap();
        }
    }
}
