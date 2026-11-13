#[cfg(not(feature = "native-tiktok"))]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "native-tiktok")]
use std::sync::Arc;

#[cfg(feature = "native-tiktok")]
use tiktools_tiktok::{
    ConnectRequest, ConnectionInfo, NativeTikTokClient, NativeTikTokConfig, TikTokError,
};

/// Owns the live transport without exposing a desktop or WebView type.
pub struct LiveService {
    #[cfg(not(feature = "native-tiktok"))]
    connected: AtomicBool,
    #[cfg(feature = "native-tiktok")]
    client: Arc<tiktools_tiktok::adapter::TikTokAdapter>,
    #[cfg(all(test, feature = "native-tiktok"))]
    snapshot_override: std::sync::RwLock<Option<tiktools_tiktok::ConnectionSnapshot>>,
}

impl Default for LiveService {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveService {
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn adapter(&self) -> Arc<dyn tiktools_live::Adapter> {
        self.client.clone()
    }
    pub fn new() -> Self {
        Self::with_native_config(NativeConfig::default())
    }

    #[cfg(feature = "native-tiktok")]
    pub fn with_native_config(config: NativeTikTokConfig) -> Self {
        Self {
            client: Arc::new(tiktools_tiktok::adapter::TikTokAdapter(Arc::new(
                NativeTikTokClient::new(config),
            ))),
            #[cfg(test)]
            snapshot_override: std::sync::RwLock::new(None),
        }
    }

    #[cfg(not(feature = "native-tiktok"))]
    pub fn with_native_config(_config: NativeConfig) -> Self {
        Self {
            connected: AtomicBool::new(false),
        }
    }

    pub fn is_connected(&self) -> bool {
        #[cfg(feature = "native-tiktok")]
        {
            self.connection_snapshot().state == tiktools_tiktok::ConnectionState::Connected
        }
        #[cfg(not(feature = "native-tiktok"))]
        {
            self.connected.load(Ordering::Acquire)
        }
    }

    #[cfg(feature = "native-tiktok")]
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<tiktools_tiktok::ClientEvent> {
        self.client.subscribe()
    }

    #[cfg(feature = "native-tiktok")]
    pub(crate) fn subscribe_stamped_with_loss(&self) -> tiktools_tiktok::TrackedReceiver {
        self.client.subscribe_stamped_with_loss()
    }

    #[cfg(feature = "native-tiktok")]
    pub fn connection_snapshot(&self) -> tiktools_tiktok::ConnectionSnapshot {
        #[cfg(test)]
        if let Some(snapshot) = *self.snapshot_override.read().unwrap() {
            return snapshot;
        }
        self.client.connection_snapshot()
    }

    #[cfg(all(test, feature = "native-tiktok"))]
    pub(crate) fn set_connection_snapshot_for_test(
        &self,
        snapshot: tiktools_tiktok::ConnectionSnapshot,
    ) {
        *self.snapshot_override.write().unwrap() = Some(snapshot);
    }

    #[cfg(feature = "native-tiktok")]
    pub async fn connect(&self, request: ConnectRequest) -> Result<ConnectionInfo, TikTokError> {
        self.client.connect_native(request).await
    }

    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn connect_generation(
        &self,
        request: ConnectRequest,
    ) -> Result<(u64, ConnectionInfo), TikTokError> {
        self.client.connect_generation(request).await
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn pick_generation(
        &self,
        session_cookie: String,
    ) -> Result<(u64, ConnectionInfo), TikTokError> {
        self.client.pick_generation(session_cookie).await
    }

    #[cfg(feature = "native-tiktok")]
    pub async fn live_channels_page(
        &self,
        session_cookie: &str,
        keyword: &str,
        offset: usize,
    ) -> Result<
        Vec<tiktools_tiktok::discovery::LiveRoom>,
        tiktools_tiktok::discovery::DiscoveryFailure,
    > {
        self.client
            .live_channels_page(session_cookie, keyword, offset)
            .await
    }

    #[cfg(feature = "native-tiktok")]
    pub async fn creator_live_room(&self, creator: &str) -> Result<Option<String>, TikTokError> {
        self.client.creator_live_room(creator).await
    }

    pub async fn disconnect(&self) {
        #[cfg(feature = "native-tiktok")]
        if let Err(error) = self.client.disconnect_native().await {
            tracing::debug!(%error, "native TikTok client disconnect failed");
        }
        #[cfg(not(feature = "native-tiktok"))]
        self.connected.store(false, Ordering::Release);
    }
}

#[cfg(feature = "native-tiktok")]
pub type NativeConfig = NativeTikTokConfig;

#[cfg(not(feature = "native-tiktok"))]
#[derive(Default)]
pub struct NativeConfig;
