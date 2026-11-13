use crate::{
    codec::{Decoder, MAX_PACKET_BYTES},
    Error, Framed,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use webrtc::{
    data_channel::{DataChannel, DataChannelEvent, RTCDataChannelInit},
    peer_connection::{
        PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler, RTCConfigurationBuilder,
        RTCIceCandidateInit, RTCIceGatheringState, RTCIceServer, RTCPeerConnectionIceEvent,
        RTCPeerConnectionState, RTCSessionDescription,
    },
    runtime::TokioRuntime,
};

const LABEL: &str = "tiktools-control";
const PROTOCOL: &str = "tiktools/collaboration/1";

/// No implicit public STUN/TURN provider. Operators supply their ICE services.
#[derive(Clone)]
pub struct Config {
    pub bind_addresses: Vec<String>,
    pub ice_servers: Vec<IceServer>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addresses: vec!["0.0.0.0:0".into()],
            ice_servers: vec![],
        }
    }
}
/// Intentionally does not implement Debug: TURN credentials are private.
#[derive(Clone)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Signal {
    Offer {
        sdp: String,
    },
    Answer {
        sdp: String,
    },
    Candidate {
        candidate: String,
        #[serde(rename = "sdpMid")]
        sdp_mid: Option<String>,
        #[serde(rename = "sdpMLineIndex")]
        sdp_mline_index: Option<u16>,
        #[serde(rename = "usernameFragment")]
        username_fragment: Option<String>,
    },
    EndOfCandidates,
}
impl Signal {
    pub(crate) fn valid(&self) -> bool {
        let field = |s: &Option<String>, max| {
            s.as_ref()
                .is_none_or(|s| s.len() <= max && !s.chars().any(char::is_control))
        };
        match self {
            Self::Offer { sdp } | Self::Answer { sdp } => {
                sdp.len() <= 65536
                    && !sdp.contains('\0')
                    && (sdp.starts_with("v=0\r\n") || sdp.starts_with("v=0\n"))
                    && sdp.lines().any(|line| line.starts_with("a=fingerprint:"))
            }
            Self::Candidate {
                candidate,
                sdp_mid,
                sdp_mline_index,
                username_fragment,
            } => {
                candidate.starts_with("candidate:")
                    && candidate.len() <= 2048
                    && !candidate.chars().any(char::is_control)
                    && field(sdp_mid, 64)
                    && sdp_mline_index.is_none_or(|index| index < 32)
                    && field(username_fragment, 256)
            }
            Self::EndOfCandidates => true,
        }
    }
}
struct Handler {
    signals: mpsc::Sender<Signal>,
    channels: mpsc::Sender<Framed>,
    accepted: AtomicBool,
    local_candidates: AtomicUsize,
    connected: AtomicBool,
    established: CancellationToken,
    stop: CancellationToken,
}
impl Handler {
    async fn install(&self, channel: Arc<dyn DataChannel>) {
        let valid = channel.label().await.is_ok_and(|s| s == LABEL)
            && channel.protocol().await.is_ok_and(|s| s == PROTOCOL)
            && channel.ordered().await.unwrap_or(false)
            && channel
                .max_packet_life_time()
                .await
                .is_ok_and(|v| v.is_none())
            && channel.max_retransmits().await.is_ok_and(|v| v.is_none());
        if !valid || self.accepted.swap(true, Ordering::AcqRel) {
            self.stop.cancel();
            let _ = channel.close().await;
            return;
        }
        let stop = self.stop.clone();
        let ready = self.channels.clone();
        let established = self.established.clone();
        tokio::spawn(async move {
            let (tx, rx) = mpsc::channel(4);
            let mut framed = Some(Framed {
                channel: channel.clone(),
                packets: rx,
                stop: stop.clone(),
                decoder: Decoder::default(),
                sending: false,
            });
            loop {
                let event = tokio::select! { biased; _=stop.cancelled()=>break, event=channel.poll()=>event };
                match event {
                    Some(DataChannelEvent::OnOpen) => {
                        established.cancel();
                        if let Some(stream) = framed.take() {
                            if ready.try_send(stream).is_err() {
                                break;
                            }
                        }
                    }
                    Some(DataChannelEvent::OnMessage(message)) => {
                        if message.is_string || message.data.len() > MAX_PACKET_BYTES {
                            break;
                        }
                        tokio::select! { biased; _=stop.cancelled()=>break, sent=tx.send(message.data.to_vec())=>if sent.is_err(){break;} }
                    }
                    Some(
                        DataChannelEvent::OnClose
                        | DataChannelEvent::OnClosing
                        | DataChannelEvent::OnError,
                    )
                    | None => break,
                    _ => {}
                }
            }
            stop.cancel();
            let _ = channel.close().await;
        });
    }
    fn signal(&self, signal: Signal) {
        if self.signals.try_send(signal).is_err() {
            self.stop.cancel();
        }
    }
}
#[async_trait::async_trait]
impl PeerConnectionEventHandler for Handler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        // SDP is sequence zero. Reserve one of the remaining 64 positions for
        // end-of-candidates; extra interfaces/servers cannot poison the stream.
        if self.local_candidates.fetch_add(1, Ordering::Relaxed) >= 63 {
            return;
        }
        match event.candidate.to_json() {
            Ok(c) => self.signal(Signal::Candidate {
                candidate: c.candidate,
                sdp_mid: c.sdp_mid,
                sdp_mline_index: c.sdp_mline_index,
                username_fragment: c.username_fragment,
            }),
            Err(_) => self.stop.cancel(),
        }
    }
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        if state == RTCIceGatheringState::Complete {
            self.signal(Signal::EndOfCandidates);
        }
    }
    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        self.connected.store(
            state == RTCPeerConnectionState::Connected,
            Ordering::Release,
        );
        if matches!(
            state,
            RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed
        ) {
            self.stop.cancel();
        }
    }
    async fn on_data_channel(&self, channel: Arc<dyn DataChannel>) {
        self.install(channel).await;
    }
}
/// One reliable ordered DataChannel per authenticated negotiation. The caller
/// retains this owner while using the stream and signs/validates every signal.
pub struct Peer {
    connection: Arc<dyn PeerConnection>,
    handler: Arc<Handler>,
    signals: mpsc::Receiver<Signal>,
    channels: mpsc::Receiver<Framed>,
    phase: u8,
    route: Arc<AtomicU8>,
}
/// Drive signaling and channel readiness together without concurrent mutable
/// borrows or polling timeouts that discard a partially received event.
pub enum Event {
    Signal(Signal),
    Connected(Framed),
}
impl Peer {
    pub async fn new(config: Config) -> Result<Self, Error> {
        if config.bind_addresses.is_empty()
            || config.bind_addresses.len() > 8
            || config.ice_servers.len() > 8
        {
            return Err(Error::Configuration);
        }
        let mut servers = Vec::new();
        for server in config.ice_servers {
            if server.urls.is_empty()
                || server.urls.len() > 8
                || server.username.len() > 256
                || server.credential.len() > 1024
                || server.urls.iter().any(|url| {
                    url.len() > 2048
                        || !["stun:", "stuns:", "turn:", "turns:"]
                            .iter()
                            .any(|scheme| url.starts_with(scheme))
                })
            {
                return Err(Error::Configuration);
            }
            servers.push(RTCIceServer {
                urls: server.urls,
                username: server.username,
                credential: server.credential,
            });
        }
        let (signal_tx, signals) = mpsc::channel(64);
        let (channel_tx, channels) = mpsc::channel(1);
        let handler = Arc::new(Handler {
            signals: signal_tx,
            channels: channel_tx,
            accepted: AtomicBool::new(false),
            local_candidates: AtomicUsize::new(0),
            connected: AtomicBool::new(false),
            established: CancellationToken::new(),
            stop: CancellationToken::new(),
        });
        let settings = webrtc::peer_connection::SettingEngineBuilder::default()
            .with_sctp_max_receive_buffer_size(256 * 1024)
            .build();
        let connection: Arc<dyn PeerConnection> = Arc::new(
            PeerConnectionBuilder::new()
                .with_configuration(
                    RTCConfigurationBuilder::default()
                        .with_ice_servers(servers)
                        .build(),
                )
                .with_setting_engine(settings)
                .with_data_channel_send_buffer_limit(256 * 1024)
                .with_runtime(Arc::new(TokioRuntime))
                .with_handler(handler.clone())
                .with_udp_addrs(config.bind_addresses)
                .build()
                .await
                .map_err(|_| Error::Configuration)?,
        );
        let route = Arc::new(AtomicU8::new(0));
        crate::route::track(
            Arc::downgrade(&connection),
            route.clone(),
            handler.stop.clone(),
        );
        let weak = Arc::downgrade(&connection);
        let stop = handler.stop.clone();
        let established = handler.established.clone();
        tokio::spawn(async move {
            tokio::select! {
                biased;
                _=stop.cancelled()=>{},
                _=established.cancelled()=>{stop.cancelled().await;},
                _=tokio::time::sleep(std::time::Duration::from_secs(60))=>{stop.cancel();},
            }
            if let Some(connection) = weak.upgrade() {
                let _ = connection.close().await;
            }
        });
        Ok(Self {
            connection,
            handler,
            signals,
            channels,
            phase: 0,
            route,
        })
    }
    pub fn route(&self) -> Option<crate::Route> {
        if self.handler.stop.is_cancelled() || !self.handler.connected.load(Ordering::Acquire) {
            return None;
        }
        match self.route.load(Ordering::Acquire) {
            1 => Some(crate::Route::Direct),
            2 => Some(crate::Route::Relay),
            _ => None,
        }
    }
    pub async fn offer(&mut self) -> Result<Signal, Error> {
        if self.phase != 0 {
            return Err(Error::Negotiation);
        }
        self.phase = 1;
        let channel = self
            .connection
            .create_data_channel(
                LABEL,
                Some(RTCDataChannelInit {
                    protocol: PROTOCOL.into(),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|_| Error::Transport)?;
        self.handler.install(channel).await;
        let description = self
            .connection
            .create_offer(None)
            .await
            .map_err(|_| Error::Transport)?;
        let sdp = description.sdp.clone();
        self.connection
            .set_local_description(description)
            .await
            .map_err(|_| Error::Transport)?;
        Ok(Signal::Offer { sdp })
    }
    pub async fn apply(&mut self, signal: Signal) -> Result<Option<Signal>, Error> {
        if !signal.valid() {
            return Err(Error::Negotiation);
        }
        match signal {
            Signal::Offer { sdp } if self.phase == 0 => {
                self.phase = 2;
                self.connection
                    .set_remote_description(
                        RTCSessionDescription::offer(sdp).map_err(|_| Error::Negotiation)?,
                    )
                    .await
                    .map_err(|_| Error::Negotiation)?;
                let answer = self
                    .connection
                    .create_answer(None)
                    .await
                    .map_err(|_| Error::Transport)?;
                let sdp = answer.sdp.clone();
                self.connection
                    .set_local_description(answer)
                    .await
                    .map_err(|_| Error::Transport)?;
                Ok(Some(Signal::Answer { sdp }))
            }
            Signal::Answer { sdp } if self.phase == 1 => {
                self.phase = 2;
                self.connection
                    .set_remote_description(
                        RTCSessionDescription::answer(sdp).map_err(|_| Error::Negotiation)?,
                    )
                    .await
                    .map_err(|_| Error::Negotiation)?;
                Ok(None)
            }
            Signal::Candidate {
                candidate,
                sdp_mid,
                sdp_mline_index,
                username_fragment,
            } if self.phase == 2 => {
                self.connection
                    .add_ice_candidate(RTCIceCandidateInit {
                        candidate,
                        sdp_mid,
                        sdp_mline_index,
                        username_fragment,
                        ..Default::default()
                    })
                    .await
                    .map_err(|_| Error::Negotiation)?;
                Ok(None)
            }
            Signal::EndOfCandidates if self.phase == 2 => Ok(None),
            _ => Err(Error::Negotiation),
        }
    }
    pub async fn next_event(&mut self) -> Result<Event, Error> {
        tokio::select! {
            biased;
            _=self.handler.stop.cancelled()=>Err(Error::Transport),
            signal=self.signals.recv()=>signal.map(Event::Signal).ok_or(Error::Transport),
            stream=self.channels.recv()=>stream.map(Event::Connected).ok_or(Error::Transport),
        }
    }
    pub async fn next_signal(&mut self) -> Result<Signal, Error> {
        tokio::select! { biased; _=self.handler.stop.cancelled()=>Err(Error::Transport), signal=self.signals.recv()=>signal.ok_or(Error::Transport) }
    }
    pub async fn stream(&mut self) -> Result<Framed, Error> {
        tokio::select! { biased; _=self.handler.stop.cancelled()=>Err(Error::Transport), stream=self.channels.recv()=>stream.ok_or(Error::Transport) }
    }
    pub async fn close(&self) {
        self.handler.stop.cancel();
        let _ = self.connection.close().await;
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.handler.stop.cancel();
    }
}

#[cfg(test)]
mod tests;
