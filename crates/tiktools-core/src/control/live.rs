//! Live connection lifecycle and live status operations.

#[cfg(feature = "native-tiktok")]
use super::check_session_cookie_len;
use super::OperationError;
use crate::events::DomainEvent;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveStatus {
    pub connected: bool,
    pub unique_id: Option<String>,
    pub room_id: Option<String>,
    pub connection_id: Option<String>,
    pub native: bool,
}

impl AppCore {
    // ------------------------------------------------------------------
    // Live.
    // ------------------------------------------------------------------

    pub fn live_status(&self) -> LiveStatus {
        let connected = self.live.is_connected();
        #[cfg(feature = "native-tiktok")]
        let native = true;
        #[cfg(not(feature = "native-tiktok"))]
        let native = false;
        #[cfg(feature = "persistence")]
        let context = read_or_recover(&self.connection_context, "connection context").clone();
        #[cfg(not(feature = "persistence"))]
        let context: Option<LiveContext> =
            read_or_recover(&self.connection_context, "connection context").clone();
        LiveStatus {
            connected,
            unique_id: context.as_ref().map(|context| context.unique_id.clone()),
            room_id: context.as_ref().map(|context| context.room_id.clone()),
            connection_id: context.map(|context| context.connection_id),
            native,
        }
    }

    #[cfg(feature = "native-tiktok")]
    pub async fn live_connect(
        self: &Arc<Self>,
        unique_id: String,
        session_cookie: String,
        room_id: Option<String>,
    ) -> Result<LiveStatus, OperationError> {
        let unique_id = clean_unique_id(&unique_id)
            .ok_or_else(|| OperationError::invalid("uniqueId must not be empty"))?;
        check_session_cookie_len(&session_cookie)?;
        if room_id.as_ref().is_some_and(|room| room.len() > 64) {
            return Err(OperationError::invalid("roomId is too long (max 64)"));
        }
        self.start_live_event_pump();
        self.publish_disconnected_event().await;
        self.live.disconnect().await;
        let info = self
            .live
            .connect(ConnectRequest {
                unique_id,
                session_cookie,
                room_id,
            })
            .await
            .map_err(|error| OperationError::unavailable(error.to_string()))?;
        self.events.publish_domain(DomainEvent::LiveConnected {
            unique_id: Some(info.unique_id.clone()),
            room_id: Some(info.room_id.clone()),
        });
        self.events.publish_domain(DomainEvent::CreatorChanged {
            unique_id: Some(info.unique_id.clone()),
        });
        Ok(self.live_status())
    }

    #[cfg(not(feature = "native-tiktok"))]
    pub async fn live_connect(
        self: &Arc<Self>,
        _unique_id: String,
        _session_cookie: String,
        _room_id: Option<String>,
    ) -> Result<LiveStatus, OperationError> {
        Err(OperationError::unavailable(
            "the native TikTok client is disabled in this build",
        ))
    }

    pub async fn live_disconnect(self: &Arc<Self>) -> LiveStatus {
        self.publish_disconnected_event().await;
        self.live.disconnect().await;
        self.events.publish_domain(DomainEvent::LiveDisconnected);
        self.events
            .publish_domain(DomainEvent::CreatorChanged { unique_id: None });
        self.live_status()
    }

    /// Picks the top live room for a session cookie and connects to it.
    /// Room choice is deterministic (first of the viewer-ordered rooms).
    #[cfg(feature = "native-tiktok")]
    pub async fn live_pick(
        self: &Arc<Self>,
        session_cookie: String,
    ) -> Result<LiveStatus, OperationError> {
        check_session_cookie_len(&session_cookie)?;
        self.start_live_event_pump();
        let mut rooms = self
            .live
            .live_channels(&session_cookie)
            .await
            .map_err(|error| OperationError::unavailable(error.to_string()))?;
        if rooms.is_empty() {
            return Err(OperationError::unavailable(
                "TikTok returned no live rooms.",
            ));
        }
        let room = rooms.remove(0);
        self.live_connect(room.unique_id, session_cookie, Some(room.room_id))
            .await
    }

    #[cfg(not(feature = "native-tiktok"))]
    pub async fn live_pick(
        self: &Arc<Self>,
        _session_cookie: String,
    ) -> Result<LiveStatus, OperationError> {
        Err(OperationError::unavailable(
            "the native TikTok client is disabled in this build",
        ))
    }
}
