//! Transport for the existing Cloud signaling endpoint. Tickets are obtained
//! through the normal enrolled-device proof API; no bearer or private key is
//! placed in a URL. This socket does not grant business-RPC authority.
use crate::{signaling::ForwardedSignal, Error};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::{
    net::TcpStream,
    time::{Instant, Interval},
};
use tokio_tungstenite::{
    tungstenite::{protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};
use uuid::Uuid;
mod relay;
pub use relay::ForwardedAnnouncement;

const LIMIT: usize = 128 * 1024;
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Host,
    Controller,
}
pub struct Ready {
    pub host_id: Uuid,
    pub session_id: Option<Uuid>,
    pub grant_revision: Option<i64>,
}
pub enum Event {
    UnsupportedCommand {
        command_id: String,
    },
    RelayOpen(Box<ForwardedAnnouncement>),
    RelayRecord(crate::relay::io::Record),
    RelayAccepted {
        tunnel_id: Uuid,
        sequence: Option<i64>,
    },
    Signal(Box<ForwardedSignal>),
    Accepted {
        negotiation_id: Uuid,
        sequence: u32,
    },
    GrantsChanged,
    Rejected {
        negotiation_id: Option<Uuid>,
        tunnel_id: Option<Uuid>,
        code: String,
    },
}
pub struct Connection {
    socket: Socket,
    ready: Ready,
    heartbeat: Interval,
    last_received: Instant,
    sending: bool,
}
fn socket_url(origin: &str) -> Result<url::Url, Error> {
    let mut url = url::Url::parse(origin).map_err(|_| Error::Configuration)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err(Error::Configuration);
    }
    match url.scheme() {
        "https" => {
            url.set_scheme("wss").map_err(|_| Error::Configuration)?;
        }
        "http"
            if url.host_str().is_some_and(|host| {
                host.parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            }) =>
        {
            url.set_scheme("ws").map_err(|_| Error::Configuration)?;
        }
        _ => return Err(Error::Configuration),
    }
    url.set_path("/api/v3/remote/socket");
    Ok(url)
}
fn ready(value: Value, role: Role, host: Uuid) -> Result<Ready, Error> {
    if value["type"] != "ready"
        || value["role"]
            != match role {
                Role::Host => "host",
                Role::Controller => "controller",
            }
        || value["hostId"] != host.to_string()
    {
        return Err(Error::Negotiation);
    }
    match role {
        Role::Host => Ok(Ready {
            host_id: host,
            session_id: None,
            grant_revision: None,
        }),
        Role::Controller => Ok(Ready {
            host_id: host,
            session_id: Some(
                value["sessionId"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .ok_or(Error::Negotiation)?,
            ),
            grant_revision: Some(
                value["grantRevision"]
                    .as_i64()
                    .filter(|n| *n > 0)
                    .ok_or(Error::Negotiation)?,
            ),
        }),
    }
}
impl Connection {
    pub async fn connect(
        origin: &str,
        ticket: &str,
        role: Role,
        host: Uuid,
    ) -> Result<Self, Error> {
        if !(32..=4096).contains(&ticket.len()) || ticket.chars().any(char::is_control) {
            return Err(Error::Configuration);
        }
        let url = socket_url(origin)?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(LIMIT))
            .max_frame_size(Some(LIMIT))
            .write_buffer_size(16 * 1024)
            .max_write_buffer_size(512 * 1024);
        let (mut socket, _) = tokio::time::timeout(
            Duration::from_secs(15),
            tokio_tungstenite::connect_async_with_config(url.as_str(), Some(config), true),
        )
        .await
        .map_err(|_| Error::Transport)?
        .map_err(|_| Error::Transport)?;
        let admitted = tokio::time::timeout(Duration::from_secs(10), async {
            socket
                .send(Message::Text(
                    json!({"type":"auth","ticket":ticket}).to_string().into(),
                ))
                .await
                .map_err(|_| Error::Transport)?;
            loop {
                match socket.next().await {
                    Some(Ok(Message::Text(text))) => {
                        break ready(
                            serde_json::from_str(&text).map_err(|_| Error::Negotiation)?,
                            role,
                            host,
                        )
                    }
                    Some(Ok(Message::Ping(bytes))) => socket
                        .send(Message::Pong(bytes))
                        .await
                        .map_err(|_| Error::Transport)?,
                    _ => return Err(Error::Transport),
                }
            }
        })
        .await
        .map_err(|_| Error::Transport)??;
        let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        Ok(Self {
            socket,
            ready: admitted,
            heartbeat,
            last_received: Instant::now(),
            sending: false,
        })
    }
    pub fn ready(&self) -> &Ready {
        &self.ready
    }
    /// Cancellation after emission poisons the socket: a signed sequence is
    /// never silently retried. Start a new negotiation after an uncertain send.
    pub async fn send(&mut self, frame: &Value) -> Result<(), Error> {
        let allowed = matches!(
            frame["type"].as_str(),
            Some("rtc.signal" | "relay.open" | "relay.data" | "relay.close")
        ) || (frame["type"] == "result" && self.ready.session_id.is_none());
        if self.sending || !allowed {
            return Err(Error::Negotiation);
        }
        let text = serde_json::to_string(frame).map_err(|_| Error::Negotiation)?;
        if text.len() > LIMIT {
            return Err(Error::Frame);
        }
        self.sending = true;
        tokio::time::timeout(
            Duration::from_secs(5),
            self.socket.send(Message::Text(text.into())),
        )
        .await
        .map_err(|_| Error::Transport)?
        .map_err(|_| Error::Transport)?;
        self.sending = false;
        Ok(())
    }
    pub async fn next(&mut self) -> Result<Event, Error> {
        if self.sending {
            return Err(Error::Transport);
        }
        loop {
            let message = tokio::select! {
                _=self.heartbeat.tick()=>{
                    if self.last_received.elapsed()>=Duration::from_secs(45) {return Err(Error::Transport);}
                    tokio::time::timeout(Duration::from_secs(5),self.socket.send(Message::Text(json!({"type":"ping"}).to_string().into()))).await.map_err(|_|Error::Transport)?.map_err(|_|Error::Transport)?;
                    continue;
                },
                message=self.socket.next()=>message.ok_or(Error::Transport)?.map_err(|_|Error::Transport)?,
            };
            self.last_received = Instant::now();
            match message {
                Message::Text(text) => {
                    let value: Value =
                        serde_json::from_str(&text).map_err(|_| Error::Negotiation)?;
                    match value["type"].as_str() {
                        // Older Cloud command requests are not native RPC.
                        // Reject them explicitly without dropping active TLS
                        // tunnels multiplexed over this host socket.
                        Some("command") if self.ready.session_id.is_none() => {
                            let command_id = value["commandId"]
                                .as_str()
                                .filter(|id| {
                                    (8..=128).contains(&id.len())
                                        && id.bytes().all(|b| {
                                            b.is_ascii_alphanumeric() || b"-_.:".contains(&b)
                                        })
                                })
                                .ok_or(Error::Negotiation)?
                                .to_owned();
                            return Ok(Event::UnsupportedCommand { command_id });
                        }
                        Some("ack") if self.ready.session_id.is_none() => {}
                        Some("relay.open" | "relay.data" | "relay.close" | "relay.accepted") => {
                            return relay::parse(value, &self.ready)
                        }
                        Some("rtc.signal") => {
                            let frame: ForwardedSignal =
                                serde_json::from_value(value).map_err(|_| Error::Negotiation)?;
                            if frame.host_id != self.ready.host_id
                                || self
                                    .ready
                                    .session_id
                                    .is_some_and(|id| id != frame.session_id)
                                || self.ready.session_id.is_some()
                                    != (frame.from_device_id == frame.host_device_id)
                                || frame.grant_revision <= 0
                                || frame.sequence > 64
                                || !frame.signal.valid()
                                || frame.signature.len() != 86
                                || frame.from_endpoint_id.len() != 64
                                || !frame
                                    .from_endpoint_id
                                    .bytes()
                                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                            {
                                return Err(Error::Negotiation);
                            }
                            return Ok(Event::Signal(Box::new(frame)));
                        }
                        Some("rtc.accepted") => {
                            return Ok(Event::Accepted {
                                negotiation_id: value["negotiationId"]
                                    .as_str()
                                    .and_then(|s| s.parse().ok())
                                    .ok_or(Error::Negotiation)?,
                                sequence: value["sequence"]
                                    .as_u64()
                                    .and_then(|n| u32::try_from(n).ok())
                                    .filter(|n| *n <= 64)
                                    .ok_or(Error::Negotiation)?,
                            })
                        }
                        Some("grants.changed") => return Ok(Event::GrantsChanged),
                        Some("error") => {
                            let code = value["code"]
                                .as_str()
                                .filter(|s| s.len() <= 64)
                                .ok_or(Error::Negotiation)?
                                .to_owned();
                            let negotiation_id = match value.get("negotiationId") {
                                None | Some(Value::Null) => None,
                                Some(Value::String(s)) => {
                                    Some(s.parse().map_err(|_| Error::Negotiation)?)
                                }
                                _ => return Err(Error::Negotiation),
                            };
                            return Ok(Event::Rejected {
                                negotiation_id,
                                tunnel_id: match value.get("tunnelId") {
                                    None | Some(Value::Null) => None,
                                    Some(Value::String(s)) => {
                                        Some(s.parse().map_err(|_| Error::Negotiation)?)
                                    }
                                    _ => return Err(Error::Negotiation),
                                },
                                code,
                            });
                        }
                        Some("pong") => {}
                        _ => return Err(Error::Negotiation),
                    }
                }
                Message::Ping(bytes) => {
                    tokio::time::timeout(
                        Duration::from_secs(5),
                        self.socket.send(Message::Pong(bytes)),
                    )
                    .await
                    .map_err(|_| Error::Transport)?
                    .map_err(|_| Error::Transport)?;
                }
                Message::Pong(_) => {}
                _ => return Err(Error::Transport),
            }
        }
    }
    pub async fn close(&mut self) {
        let _ = tokio::time::timeout(Duration::from_secs(1), self.socket.close(None)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signaling_requires_secure_origin_or_numeric_loopback() {
        assert_eq!(
            socket_url("https://cloud.example").unwrap().as_str(),
            "wss://cloud.example/api/v3/remote/socket"
        );
        assert!(socket_url("http://127.0.0.1:9000").is_ok());
        for origin in [
            "http://cloud.example",
            "https://user:secret@cloud.example",
            "https://cloud.example/path",
            "https://cloud.example?ticket=secret",
            "wss://cloud.example",
            "http://localhost",
        ] {
            assert!(socket_url(origin).is_err());
        }
    }
    #[test]
    fn ready_binds_host_role_and_grant() {
        let host = Uuid::new_v4();
        let valid = json!({"type":"ready","role":"controller","hostId":host,"sessionId":Uuid::new_v4(),"grantRevision":1});
        assert!(ready(valid.clone(), Role::Controller, host).is_ok());
        assert!(ready(valid.clone(), Role::Host, host).is_err());
        assert!(ready(valid.clone(), Role::Controller, Uuid::new_v4()).is_err());
        let mut invalid = valid;
        invalid["grantRevision"] = json!(0);
        assert!(ready(invalid, Role::Controller, host).is_err());
    }
    #[tokio::test]
    async fn socket_authenticates_and_handles_heartbeat_before_signal_ack() {
        let host = Uuid::new_v4();
        let negotiation = Uuid::new_v4();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            let auth = socket.next().await.unwrap().unwrap().into_text().unwrap();
            let value: Value = serde_json::from_str(&auth).unwrap();
            assert_eq!(value["type"], "auth");
            assert_eq!(value["ticket"], "t".repeat(32));
            socket.send(Message::Text(json!({"type":"ready","role":"controller","hostId":host,"sessionId":Uuid::new_v4(),"grantRevision":7}).to_string().into())).await.unwrap();
            loop {
                let frame = socket.next().await.unwrap().unwrap().into_text().unwrap();
                let value: Value = serde_json::from_str(&frame).unwrap();
                if value["type"] == "ping" {
                    socket
                        .send(Message::Text(json!({"type":"pong"}).to_string().into()))
                        .await
                        .unwrap();
                    continue;
                }
                assert_eq!(value["type"], "rtc.signal");
                socket
                    .send(Message::Text(
                        json!({"type":"rtc.accepted","negotiationId":negotiation,"sequence":0})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                break;
            }
        });
        let mut client = Connection::connect(&origin, &"t".repeat(32), Role::Controller, host)
            .await
            .unwrap();
        assert_eq!(client.ready().grant_revision, Some(7));
        assert!(
            tokio::time::timeout(Duration::from_millis(10), client.next())
                .await
                .is_err()
        );
        assert!(client
            .send(&json!({"type":"result","commandId":"not-a-host"}))
            .await
            .is_err());
        client.send(&json!({"type":"rtc.signal"})).await.unwrap();
        assert!(
            matches!(client.next().await.unwrap(),Event::Accepted{negotiation_id,sequence:0} if negotiation_id==negotiation)
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn legacy_commands_are_rejected_without_closing_the_shared_host_socket() {
        let host = Uuid::new_v4();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
            socket.next().await.unwrap().unwrap();
            socket
                .send(Message::Text(
                    json!({"type":"ready","role":"host","hostId":host})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
            socket.send(Message::Text(json!({"type":"command","commandId":"old-request-1","operation":"points.adjust",
                "args":{"uniqueId":"alice","delta":999}}).to_string().into())).await.unwrap();
            loop {
                let text = socket.next().await.unwrap().unwrap().into_text().unwrap();
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["type"] == "ping" {
                    socket
                        .send(Message::Text(json!({"type":"pong"}).to_string().into()))
                        .await
                        .unwrap();
                    continue;
                }
                assert_eq!(
                    value,
                    json!({"type":"result","commandId":"old-request-1","status":"failed",
                    "code":"unsupportedOperation","result":null})
                );
                socket
                    .send(Message::Text(
                        json!({"type":"ack","commandId":"old-request-1","status":"failed"})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                socket
                    .send(Message::Text(
                        json!({"type":"grants.changed"}).to_string().into(),
                    ))
                    .await
                    .unwrap();
                break;
            }
        });
        let mut client = Connection::connect(&origin, &"t".repeat(32), Role::Host, host)
            .await
            .unwrap();
        let Event::UnsupportedCommand { command_id } = client.next().await.unwrap() else {
            panic!("legacy request must not become native RPC");
        };
        client
            .send(
                &json!({"type":"result","commandId":command_id,"status":"failed",
            "code":"unsupportedOperation","result":null}),
            )
            .await
            .unwrap();
        assert!(matches!(client.next().await.unwrap(), Event::GrantsChanged));
        server.await.unwrap();
    }
}
