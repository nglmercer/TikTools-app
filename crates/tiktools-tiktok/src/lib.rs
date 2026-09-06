//! Native TikTok LIVE client used by the Rust application host.
//!
//! This crate deliberately knows nothing about Winit, Wry, SQLite, or the
//! frontend. It composes the browser-free transport from the pinned
//! `tiktok-signer` workspace:
//!
//! * `ttl-live-discovery` resolves rooms and reads room/gift metadata;
//! * `ttl-sign-embedded` signs the direct socket query in-process;
//! * `ttl-sign-headless` builds the transport backend;
//! * `ttl-live-ws` owns heartbeats, acknowledgements, and reconnection; and
//! * `ttl-live-events` normalises protobuf payloads into stable event values.
//!
//! Bun, Node, and a browser are not part of the live connection path.

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::{broadcast, oneshot, Mutex};

pub mod discovery;
pub mod events;
pub mod live;
pub mod protocol;

pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRequest {
    pub unique_id: String,
    pub session_cookie: String,
    #[serde(default)]
    pub room_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    pub room_id: Option<String>,
    pub title: Option<String>,
    pub creator: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub unique_id: String,
    pub room_id: String,
    pub title: String,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub viewers: u64,
    pub total_users: u64,
    pub gifts: Vec<GiftInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GiftInfo {
    pub id: String,
    pub name: String,
    pub diamond_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip)]
    pub streakable: bool,
}

#[derive(Debug, Clone)]
pub struct NativeTikTokConfig {
    /// Explicit bundle override. The signer bundle is intentionally loaded
    /// at runtime so a TikTok deployment can be updated without recompiling
    /// the desktop host.
    pub bundle_path: Option<PathBuf>,
    /// Cache location used when the bundle is downloaded from `bundle_url`.
    pub bundle_cache_path: Option<PathBuf>,
    pub bundle_url: String,
    pub reconnect: ttl_live_ws::ReconnectPolicy,
}

impl Default for NativeTikTokConfig {
    fn default() -> Self {
        Self {
            bundle_path: std::env::var_os("TIKTOOLS_TIKTOK_SIGNING_BUNDLE").map(PathBuf::from),
            bundle_cache_path: std::env::var_os("TIKTOOLS_TIKTOK_BUNDLE_CACHE").map(PathBuf::from),
            bundle_url: std::env::var("TIKTOOLS_TIKTOK_BUNDLE_URL").unwrap_or_else(|_| {
                "https://sf16-website-login.neutral.ttwstatic.com/obj/tiktok_web_login_static/webmssdk/1.0.0.388/webmssdk.js".to_owned()
            }),
            reconnect: ttl_live_ws::ReconnectPolicy {
                max_attempts: 5,
                initial_backoff: Duration::from_secs(2),
                max_backoff: Duration::from_secs(30),
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum TikTokError {
    #[error("TikTok creator handle is empty")]
    InvalidCreator,
    #[error("TikTok session is empty and guest bootstrap did not provide an identity")]
    EmptySession,
    #[error("TikTok discovery failed: {0}")]
    Discovery(String),
    #[error("TikTok signing bundle failed: {0}")]
    Bundle(String),
    #[error("TikTok signer failed: {0}")]
    Signer(String),
    #[error("TikTok live transport failed: {0}")]
    Transport(String),
    #[error("TikTok client task failed: {0}")]
    Task(String),
}

/// Events emitted by the native client. The core consumes this stream and
/// decides what belongs in the UI, points engine, automation, and plugins.
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Connected(ConnectionInfo),
    Event(events::TikToolsEvent),
    Reconnecting { attempt: u32, delay_ms: u64 },
    Disconnected { reason: String },
    Error { phase: ErrorPhase, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorPhase {
    Connect,
    Live,
}

pub trait TikTokClient: Send + Sync {
    fn connect<'a>(
        &'a self,
        request: ConnectRequest,
    ) -> BoxFuture<'a, Result<ConnectionInfo, TikTokError>>;
    fn disconnect<'a>(&'a self) -> BoxFuture<'a, Result<(), TikTokError>>;
    fn discover<'a>(
        &'a self,
        request: ConnectRequest,
    ) -> BoxFuture<'a, Result<DiscoveryResult, TikTokError>>;
}

/// The production client. All mutable socket state is owned by the Tokio
/// task; the public object is a cheap, thread-safe control handle.
pub struct NativeTikTokClient {
    config: NativeTikTokConfig,
    events: broadcast::Sender<ClientEvent>,
    active: Mutex<Option<ActiveConnection>>,
    generation: Arc<AtomicU64>,
}

struct ActiveConnection {
    stop: oneshot::Sender<()>,
}

impl NativeTikTokClient {
    pub fn new(config: NativeTikTokConfig) -> Self {
        ensure_tls_provider();
        let (events, _) = broadcast::channel(512);
        Self {
            config,
            events,
            active: Mutex::new(None),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ClientEvent> {
        self.events.subscribe()
    }

    pub async fn connect_native(
        &self,
        request: ConnectRequest,
    ) -> Result<ConnectionInfo, TikTokError> {
        self.disconnect_native().await?;

        let unique_id = clean_unique_id(&request.unique_id).ok_or(TikTokError::InvalidCreator)?;
        tracing::info!(creator = %unique_id, requested_room = ?request.room_id, "starting native TikTok connection");
        let cookies = resolve_session(&request.session_cookie).await?;
        tracing::debug!(creator = %unique_id, "TikTok session resolved");
        let preset = default_preset();
        let discovery = ttl_live_discovery::DiscoveryClient::new(&preset)
            .map_err(|error| TikTokError::Discovery(error.to_string()))?
            .with_session(cookies.to_cookie_string());

        let lookup = match request
            .room_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            Some(room_id) => {
                if !ttl_sign_core::room::is_usable_room_id(room_id) {
                    return Err(TikTokError::Discovery("room id is invalid".to_owned()));
                }
                None
            }
            None => Some(
                discovery
                    .room_lookup(&unique_id)
                    .await
                    .map_err(|error| TikTokError::Discovery(error.to_string()))?,
            ),
        };
        if let Some(lookup) = lookup.as_ref() {
            if !lookup.is_live() {
                return Err(TikTokError::Discovery(format!(
                    "@{} is not live (status {})",
                    unique_id, lookup.status
                )));
            }
        }
        let room_id = request
            .room_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| lookup.as_ref().map(|room| room.room_id.clone()))
            .ok_or_else(|| TikTokError::Discovery("TikTok did not return a room id".to_owned()))?;
        tracing::info!(creator = %unique_id, room_id = %room_id, "TikTok room resolved");

        // Metadata and gifts are useful but should not make a valid socket
        // unavailable when one of TikTok's JSON endpoints is temporarily
        // refused. The stream itself remains the source of live events.
        let room_info = discovery.room_info(&room_id).await.ok();
        let gifts = discovery.gift_list(&room_id).await.unwrap_or_default();
        tracing::debug!(room_id = %room_id, gifts = gifts.len(), "TikTok room metadata loaded");
        let bundle = load_signing_bundle(&self.config).await?;
        tracing::debug!(room_id = %room_id, "TikTok signing bundle loaded");
        let profile = ttl_sign_embedded::Profile {
            user_agent: Some(preset.user_agent()),
            cookie: Some(cookies.to_cookie_string()),
            ..ttl_sign_embedded::Profile::default()
        };
        let signer = tokio::task::spawn_blocking(move || {
            ttl_sign_embedded::EmbeddedSigner::new(bundle, profile)
                .map_err(|error| TikTokError::Signer(error.to_string()))
        })
        .await
        .map_err(|error| TikTokError::Task(error.to_string()))??;
        let backend = ttl_sign_headless::HeadlessBackend::new(
            ttl_sign_headless::HeadlessConfig::new(preset.clone(), cookies.clone()),
            Box::new(signer),
        )
        .map_err(|error| TikTokError::Signer(error.to_string()))?;
        let connection = ttl_live_ws::ReconnectingConnection::open(
            Arc::new(backend),
            &room_id,
            ttl_live_ws::ConnectConfig::default(),
            self.config.reconnect.clone(),
        )
        .await
        .map_err(|error| TikTokError::Transport(error.to_string()))?;
        tracing::info!(room_id = %room_id, "TikTok live WebSocket connected");

        let info = ConnectionInfo {
            unique_id: room_info
                .as_ref()
                .and_then(|info| {
                    (!info.owner.unique_id.is_empty()).then(|| info.owner.unique_id.clone())
                })
                .or_else(|| lookup.as_ref().map(|room| room.unique_id.clone()))
                .unwrap_or_else(|| unique_id.clone()),
            room_id: room_id.clone(),
            title: room_info
                .as_ref()
                .map(|info| info.title.clone())
                .filter(|title| !title.is_empty())
                .or_else(|| lookup.as_ref().map(|room| room.title.clone()))
                .unwrap_or_default(),
            nickname: room_info
                .as_ref()
                .map(|info| info.owner.nickname.clone())
                .filter(|nickname| !nickname.is_empty())
                .or_else(|| lookup.as_ref().map(|room| room.nickname.clone()))
                .unwrap_or_else(|| unique_id.clone()),
            avatar_url: room_info
                .as_ref()
                .map(|info| info.owner.avatar_url.clone())
                .filter(|url| !url.is_empty()),
            viewers: room_info
                .as_ref()
                .map(|info| info.viewer_count)
                .unwrap_or_default(),
            total_users: room_info
                .as_ref()
                .map(|info| info.total_viewers)
                .unwrap_or_default(),
            gifts: gifts.into_iter().map(gift_info).collect(),
        };

        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let (stop, stop_receiver) = oneshot::channel();
        {
            let mut active = self.active.lock().await;
            *active = Some(ActiveConnection { stop });
        }
        if self
            .events
            .send(ClientEvent::Connected(info.clone()))
            .is_err()
        {
            tracing::warn!("native TikTok connection has no event consumers");
        }
        let event_sender = self.events.clone();
        let gifts = info.gifts.clone();
        let generation_counter = Arc::clone(&self.generation);
        tokio::spawn(async move {
            run_connection(
                connection,
                stop_receiver,
                event_sender,
                generation_counter,
                generation,
                gifts,
            )
            .await;
        });
        Ok(info)
    }

    pub async fn disconnect_native(&self) -> Result<(), TikTokError> {
        self.generation.fetch_add(1, Ordering::AcqRel);
        let active = self.active.lock().await.take();
        if let Some(active) = active {
            let _ = active.stop.send(());
        }
        Ok(())
    }

    pub async fn live_channels(
        &self,
        session_cookie: &str,
    ) -> Result<Vec<discovery::LiveRoom>, TikTokError> {
        let cookies = resolve_session(session_cookie).await?;
        let preset = default_preset();
        let client = ttl_live_discovery::DiscoveryClient::new(&preset)
            .map_err(|error| TikTokError::Discovery(error.to_string()))?
            .with_session(cookies.to_cookie_string());
        client
            .live_channels("live")
            .await
            .map(|rooms| rooms.into_iter().map(Into::into).collect())
            .map_err(|error| TikTokError::Discovery(error.to_string()))
    }
}

impl Default for NativeTikTokClient {
    fn default() -> Self {
        Self::new(NativeTikTokConfig::default())
    }
}

impl TikTokClient for NativeTikTokClient {
    fn connect<'a>(
        &'a self,
        request: ConnectRequest,
    ) -> BoxFuture<'a, Result<ConnectionInfo, TikTokError>> {
        Box::pin(self.connect_native(request))
    }

    fn disconnect<'a>(&'a self) -> BoxFuture<'a, Result<(), TikTokError>> {
        Box::pin(self.disconnect_native())
    }

    fn discover<'a>(
        &'a self,
        request: ConnectRequest,
    ) -> BoxFuture<'a, Result<DiscoveryResult, TikTokError>> {
        Box::pin(async move {
            let unique_id =
                clean_unique_id(&request.unique_id).ok_or(TikTokError::InvalidCreator)?;
            let cookies = resolve_session(&request.session_cookie).await?;
            let preset = default_preset();
            let client = ttl_live_discovery::DiscoveryClient::new(&preset)
                .map_err(|error| TikTokError::Discovery(error.to_string()))?
                .with_session(cookies.to_cookie_string());
            let lookup = client
                .room_lookup(&unique_id)
                .await
                .map_err(|error| TikTokError::Discovery(error.to_string()))?;
            let creator = client.room_info(&lookup.room_id).await.ok().map(|info| {
                serde_json::json!({
                    "roomId": info.room_id,
                    "title": info.title,
                    "status": info.status,
                    "createTime": info.create_time,
                    "viewerCount": info.viewer_count,
                    "totalViewers": info.total_viewers,
                    "likeCount": info.like_count,
                    "commentCount": info.comment_count,
                    "shareCount": info.share_count,
                    "followCount": info.follow_count,
                    "coverUrl": info.cover_url,
                    "shareUrl": info.share_url,
                    "owner": {
                        "id": info.owner.id,
                        "uniqueId": info.owner.unique_id,
                        "nickname": info.owner.nickname,
                        "secUid": info.owner.sec_uid,
                        "avatarUrl": info.owner.avatar_url,
                        "followerCount": info.owner.follower_count,
                        "followingCount": info.owner.following_count,
                    },
                })
            });
            Ok(DiscoveryResult {
                room_id: Some(lookup.room_id),
                title: Some(lookup.title),
                creator,
            })
        })
    }
}

async fn run_connection(
    mut connection: ttl_live_ws::ReconnectingConnection,
    mut stop: oneshot::Receiver<()>,
    event_sender: broadcast::Sender<ClientEvent>,
    generation: Arc<AtomicU64>,
    connection_generation: u64,
    gifts: Vec<GiftInfo>,
) {
    tracing::debug!(connection_generation, "TikTok live event task started");
    let gifts = gifts
        .into_iter()
        .map(|gift| (gift.id.clone(), gift))
        .collect::<std::collections::HashMap<_, _>>();
    loop {
        tokio::select! {
            _ = &mut stop => {
                connection.close().await;
                tracing::debug!(connection_generation, "TikTok live event task stopped");
                return;
            }
            message = connection.next_message() => match message {
                Some(Ok(message)) => {
                    tracing::debug!(
                        log_id = message.log_id,
                        bytes = message.payload.len(),
                        "TikTok live frame received"
                    );
                    match ttl_live_events::decode_batch(&message.payload) {
                    Ok(batch) => {
                        tracing::debug!(events = batch.events.len(), "TikTok live frame decoded");
                        for decoded in batch.events {
                            if generation.load(Ordering::Acquire) != connection_generation {
                                let _ = connection.close().await;
                                tracing::debug!(connection_generation, "discarding stale TikTok live connection");
                                return;
                            }
                            tracing::debug!(method = %decoded.raw.method, "TikTok live event decoded");
                            let event = events::TikToolsEvent::from_decoded(decoded, &gifts);
                            if event_sender.send(ClientEvent::Event(event)).is_err() {
                                tracing::debug!("TikTok live event has no consumers");
                            }
                        }
                    }
                    Err(error) => {
                        let _ = event_sender.send(ClientEvent::Error {
                            phase: ErrorPhase::Live,
                            message: format!("could not decode TikTok event batch: {error}"),
                        });
                    }
                    }
                }
                Some(Err(error)) => {
                    let _ = event_sender.send(ClientEvent::Error {
                        phase: ErrorPhase::Live,
                        message: error.to_string(),
                    });
                    let _ = event_sender.send(ClientEvent::Disconnected { reason: error.to_string() });
                    tracing::warn!(%error, connection_generation, "TikTok live event task ended after a transport error");
                    return;
                }
                None => {
                    let _ = event_sender.send(ClientEvent::Disconnected { reason: "TikTok closed the live stream".to_owned() });
                    tracing::info!(connection_generation, "TikTok live event task ended because the stream closed");
                    return;
                }
            }
        }
    }
}

async fn resolve_session(raw: &str) -> Result<ttl_sign_core::CookieJar, TikTokError> {
    let raw = raw.trim();
    if !raw.is_empty() {
        let jar = ttl_sign_core::CookieJar::parse(raw);
        if !jar.is_empty() {
            return Ok(jar);
        }
    }
    bootstrap_guest_session().await
}

async fn bootstrap_guest_session() -> Result<ttl_sign_core::CookieJar, TikTokError> {
    let response = reqwest::Client::builder()
        .user_agent(default_preset().user_agent())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| TikTokError::Discovery(error.to_string()))?
        .get("https://www.tiktok.com/live")
        .header("accept-language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|error| TikTokError::Discovery(error.to_string()))?;
    let status = response.status();
    let mut cookies = Vec::new();
    for value in response.headers().get_all(reqwest::header::SET_COOKIE) {
        if let Ok(value) = value.to_str() {
            if let Some(pair) = value.split(';').next() {
                cookies.push(pair.to_owned());
            }
        }
    }
    // The bootstrap response body is not used. Dropping the response avoids
    // buffering an attacker-controlled document merely to collect cookies.
    drop(response);
    if !status.is_success() {
        return Err(TikTokError::Discovery(format!(
            "guest bootstrap returned HTTP {}",
            status.as_u16()
        )));
    }
    let header = cookies.join("; ");
    let jar = ttl_sign_core::CookieJar::parse(&header);
    if jar.get("ttwid").is_none() || jar.is_empty() {
        return Err(TikTokError::EmptySession);
    }
    Ok(jar)
}

async fn load_signing_bundle(config: &NativeTikTokConfig) -> Result<String, TikTokError> {
    if let Some(path) = config.bundle_path.as_deref() {
        return read_bundle(path).await;
    }
    if let Some(path) = config.bundle_cache_path.as_deref() {
        if path.is_file() {
            return read_bundle(path).await;
        }
    }
    let client = reqwest::Client::builder()
        .user_agent(default_preset().user_agent())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| TikTokError::Bundle(error.to_string()))?;
    let response = client
        .get(&config.bundle_url)
        .send()
        .await
        .map_err(|error| TikTokError::Bundle(error.to_string()))?;
    if !response.status().is_success() {
        return Err(TikTokError::Bundle(format!(
            "bundle download returned HTTP {}",
            response.status().as_u16()
        )));
    }
    let bytes = read_limited_response(response, 8 * 1024 * 1024).await?;
    if bytes.is_empty() {
        return Err(TikTokError::Bundle(
            "downloaded bundle has an invalid size".to_owned(),
        ));
    }
    let source = String::from_utf8(bytes.to_vec())
        .map_err(|_| TikTokError::Bundle("downloaded bundle is not UTF-8".to_owned()))?;
    if let Some(path) = config.bundle_cache_path.as_deref() {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| TikTokError::Bundle(error.to_string()))?;
        }
        let temporary = path.with_extension("download");
        tokio::fs::write(&temporary, source.as_bytes())
            .await
            .map_err(|error| TikTokError::Bundle(error.to_string()))?;
        tokio::fs::rename(&temporary, path)
            .await
            .map_err(|error| TikTokError::Bundle(error.to_string()))?;
    }
    Ok(source)
}

async fn read_limited_response(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, TikTokError> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(TikTokError::Bundle(format!(
            "downloaded response exceeds the {max_bytes} byte limit"
        )));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| TikTokError::Bundle(error.to_string()))?
    {
        if bytes.len().saturating_add(chunk.len()) > max_bytes {
            return Err(TikTokError::Bundle(format!(
                "downloaded response exceeds the {max_bytes} byte limit"
            )));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn read_bundle(path: &Path) -> Result<String, TikTokError> {
    let display = path.display().to_string();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || std::fs::read_to_string(&path))
        .await
        .map_err(|error| TikTokError::Bundle(error.to_string()))?
        .map_err(|error| TikTokError::Bundle(format!("{display}: {error}")))
}

fn ensure_tls_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        // reqwest and tokio-tungstenite can enable different rustls providers
        // through Cargo feature unification. Selecting ring here keeps the
        // library safe when it is embedded without the desktop binary.
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

fn gift_info(gift: ttl_sign_core::Gift) -> GiftInfo {
    let streakable = gift.is_streakable();
    GiftInfo {
        id: gift.id.to_string(),
        name: gift.name,
        diamond_count: gift.diamond_count,
        icon_url: (!gift.icon_url.is_empty()).then_some(gift.icon_url),
        streakable,
    }
}

fn clean_unique_id(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('@');
    (!value.is_empty()).then_some(value.to_owned())
}

fn default_preset() -> ttl_sign_core::Preset {
    #[cfg(target_os = "windows")]
    let device = ttl_sign_core::DevicePreset::chrome_windows();
    #[cfg(target_os = "macos")]
    let device = ttl_sign_core::DevicePreset::chrome_macos();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let device = ttl_sign_core::DevicePreset::chrome_linux();
    ttl_sign_core::Preset::new(
        device,
        ttl_sign_core::LocationPreset::us_east(),
        ttl_sign_core::ScreenPreset::FHD,
    )
}
