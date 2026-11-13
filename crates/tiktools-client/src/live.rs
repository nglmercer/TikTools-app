//! Typed `live.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::live::{
    LiveChannelsParams, LiveChannelsResult, LiveConnectParams, LiveMonitorStartParams,
    LiveMonitorStatus, LivePickParams, LiveStatus,
};
pub use tiktools_control_api::modules::live::{
    PlatformConnectParams, PlatformConnectionSnapshot, PlatformCredentialDelete,
    PlatformCredentialParams, PlatformCredentialRef, PlatformDescriptor, PlatformDisconnectParams,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[
    LIVE_CONNECT,
    LIVE_PICK,
    LIVE_DISCONNECT,
    LIVE_STATUS,
    LIVE_CHANNELS,
    LIVE_MONITOR_START,
    LIVE_MONITOR_STOP,
    LIVE_MONITOR_STATUS,
    "live.platforms",
    "live.connections",
    "live.platform.connect",
    "live.platform.disconnect",
    "live.platform.credentials.store",
    "live.platform.credentials.delete",
    "live.platform.analytics",
];

const LIVE_CONNECT: &str = "live.connect";
const LIVE_PICK: &str = "live.pick";
const LIVE_DISCONNECT: &str = "live.disconnect";
const LIVE_STATUS: &str = "live.status";
const LIVE_CHANNELS: &str = "live.channels";
const LIVE_MONITOR_START: &str = "live.monitor.start";
const LIVE_MONITOR_STOP: &str = "live.monitor.stop";
const LIVE_MONITOR_STATUS: &str = "live.monitor.status";

impl TikToolsClient {
    pub async fn live_platforms(&self) -> Result<Vec<PlatformDescriptor>, ClientError> {
        self.call("live.platforms", Empty::default()).await
    }
    pub async fn live_connections(&self) -> Result<Vec<PlatformConnectionSnapshot>, ClientError> {
        self.call("live.connections", Empty::default()).await
    }
    pub async fn live_platform_connect(
        &self,
        params: PlatformConnectParams,
    ) -> Result<PlatformConnectionSnapshot, ClientError> {
        self.call("live.platform.connect", params).await
    }
    pub async fn live_platform_disconnect(
        &self,
        params: PlatformDisconnectParams,
    ) -> Result<(), ClientError> {
        self.call("live.platform.disconnect", params).await
    }
    pub async fn live_platform_store_credential(
        &self,
        params: PlatformCredentialParams,
    ) -> Result<PlatformCredentialRef, ClientError> {
        self.call("live.platform.credentials.store", params).await
    }
    pub async fn live_platform_delete_credential(
        &self,
        params: PlatformCredentialDelete,
    ) -> Result<(), ClientError> {
        self.call("live.platform.credentials.delete", params).await
    }
    pub async fn live_platform_analytics(&self) -> Result<Vec<serde_json::Value>, ClientError> {
        self.call("live.platform.analytics", Empty::default()).await
    }
    /// Connects the native TikTok live client.
    /// RPC method `live.connect`.
    pub async fn live_connect(&self, params: LiveConnectParams) -> Result<LiveStatus, ClientError> {
        self.call(LIVE_CONNECT, params).await
    }
    /// Connects to the top live room for a session cookie.
    /// RPC method `live.pick`.
    pub async fn live_pick(&self, params: LivePickParams) -> Result<LiveStatus, ClientError> {
        self.call(LIVE_PICK, params).await
    }
    /// Disconnects the live client.
    /// RPC method `live.disconnect`.
    pub async fn live_disconnect(&self) -> Result<LiveStatus, ClientError> {
        self.call(LIVE_DISCONNECT, Empty::default()).await
    }
    /// Live connection status.
    /// RPC method `live.status`.
    pub async fn live_status(&self) -> Result<LiveStatus, ClientError> {
        self.call(LIVE_STATUS, Empty::default()).await
    }
    /// Browses currently live rooms without connecting.
    /// RPC method `live.channels`.
    pub async fn live_channels(
        &self,
        params: LiveChannelsParams,
    ) -> Result<LiveChannelsResult, ClientError> {
        self.call(LIVE_CHANNELS, params).await
    }
    /// Monitors a creator and connects when the LIVE starts.
    /// RPC method `live.monitor.start`.
    pub async fn live_monitor_start(
        &self,
        params: LiveMonitorStartParams,
    ) -> Result<LiveMonitorStatus, ClientError> {
        self.call(LIVE_MONITOR_START, params).await
    }
    /// Stops monitoring and disconnects its LIVE session.
    /// RPC method `live.monitor.stop`.
    pub async fn live_monitor_stop(&self) -> Result<LiveMonitorStatus, ClientError> {
        self.call(LIVE_MONITOR_STOP, Empty::default()).await
    }
    /// Current native creator monitor state.
    /// RPC method `live.monitor.status`.
    pub async fn live_monitor_status(&self) -> Result<LiveMonitorStatus, ClientError> {
        self.call(LIVE_MONITOR_STATUS, Empty::default()).await
    }
}
