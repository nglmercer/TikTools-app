//! Typed `live.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::live::{LiveConnectParams, LivePickParams, LiveStatus};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[LIVE_CONNECT, LIVE_PICK, LIVE_DISCONNECT, LIVE_STATUS];

const LIVE_CONNECT: &str = "live.connect";
const LIVE_PICK: &str = "live.pick";
const LIVE_DISCONNECT: &str = "live.disconnect";
const LIVE_STATUS: &str = "live.status";

impl TikToolsClient {
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
}
