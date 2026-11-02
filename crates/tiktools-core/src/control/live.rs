//! Live connection lifecycle and live status operations.

#[cfg(feature = "native-tiktok")]
use super::check_session_cookie_len;
use super::OperationError;
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

/// One currently-live room returned by TikTok's browser-free live search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveChannel {
    pub unique_id: String,
    /// Kept a string on purpose: TikTok room ids exceed the JavaScript safe
    /// integer range and would be corrupted by a JSON number.
    pub room_id: String,
    pub nickname: String,
    pub title: String,
    pub viewers: u64,
}

/// A bounded page of live rooms. `nextOffset` is absent when the page was
/// empty; an empty page is not proof that nobody is broadcasting.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveChannelsResult {
    pub rooms: Vec<LiveChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
    pub fetched_at: u64,
    pub source: String,
}

/// Source marker for [`LiveChannelsResult`]; never a claim about TikTok as a whole.
#[cfg(feature = "native-tiktok")]
pub const LIVE_SEARCH_SOURCE: &str = "tiktok-live-search";

#[cfg(any(test, feature = "native-tiktok"))]
const LIVE_CHANNELS_KEYWORD_FALLBACK: &str = "live";
#[cfg(any(test, feature = "native-tiktok"))]
const LIVE_CHANNELS_KEYWORD_MAX: usize = 64;
#[cfg(feature = "native-tiktok")]
const LIVE_CHANNELS_OFFSET_MAX: u32 = 10_000;
#[cfg(feature = "native-tiktok")]
const LIVE_CHANNELS_LIMIT_DEFAULT: u32 = 20;
#[cfg(feature = "native-tiktok")]
const LIVE_CHANNELS_LIMIT_MAX: u32 = 50;

/// Trim an optional keyword, fall back to `live` when empty, and cap its
/// length. This is a browse hint, not an identity: an over-long value is
/// truncated rather than rejected.
#[cfg(any(test, feature = "native-tiktok"))]
fn normalize_live_keyword(keyword: Option<String>) -> String {
    let trimmed = keyword.unwrap_or_default();
    let trimmed = trimmed.trim();
    if trimmed.is_empty() {
        return LIVE_CHANNELS_KEYWORD_FALLBACK.to_owned();
    }
    trimmed.chars().take(LIVE_CHANNELS_KEYWORD_MAX).collect()
}

#[cfg(feature = "native-tiktok")]
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default()
}

/// Map a classified discovery failure onto the control-plane error codes the
/// UI branches on: sign in, slow down, retry, or give up.
#[cfg(feature = "native-tiktok")]
fn discovery_failure(error: tiktools_tiktok::discovery::DiscoveryFailure) -> OperationError {
    use tiktools_tiktok::discovery::DiscoveryFailureKind as Kind;
    let code = match error.kind {
        Kind::AuthRequired => "unauthenticated",
        Kind::RateLimited => "rate_limited",
        Kind::Transport => "timeout",
        Kind::InvalidResponse => "invalid_response",
        Kind::Refused => "unavailable",
    };
    OperationError {
        code,
        message: error.message,
        data: None,
    }
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
        let context = recover_rwlock_read(&self.connection_context, "connection context").clone();
        #[cfg(not(feature = "persistence"))]
        let context: Option<LiveContext> =
            recover_rwlock_read(&self.connection_context, "connection context").clone();
        #[cfg(feature = "native-tiktok")]
        let context = context.filter(|context| {
            context.connection_id
                == format!("connection-{}", self.live.connection_snapshot().generation)
        });
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
        let _operation = self.live_monitor.operation.lock().await;
        self.stop_live_monitor().await;
        drop(_operation);
        self.live_connect_monitored(unique_id, session_cookie, room_id)
            .await
    }

    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn live_connect_monitored(
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
        let (generation, info) = self
            .live
            .connect_generation(ConnectRequest {
                unique_id,
                session_cookie,
                room_id,
            })
            .await
            .map_err(|error| OperationError::unavailable(error.to_string()))?;
        // The ordered transport event worker publishes lifecycle topics.
        // Return the connection values directly even if persistence/UI
        // initialization is still queued; it must not duplicate topics.
        let snapshot = self.live.connection_snapshot();
        if !self.live.is_connected() || snapshot.generation != generation {
            return Ok(self.live_status());
        }
        Ok(LiveStatus {
            connected: true,
            unique_id: Some(info.unique_id),
            room_id: Some(info.room_id),
            connection_id: Some(format!("connection-{}", generation)),
            native: true,
        })
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
        let _operation = self.live_monitor.operation.lock().await;
        self.stop_live_monitor().await;
        self.live.disconnect().await;
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
        let _operation = self.live_monitor.operation.lock().await;
        self.stop_live_monitor().await;
        drop(_operation);
        self.start_live_event_pump();
        let (generation, info) = self
            .live
            .pick_generation(session_cookie)
            .await
            .map_err(|error| OperationError::unavailable(error.to_string()))?;
        if self.live.connection_snapshot().generation != generation || !self.live.is_connected() {
            return Ok(self.live_status());
        }
        Ok(LiveStatus {
            connected: true,
            unique_id: Some(info.unique_id),
            room_id: Some(info.room_id),
            connection_id: Some(format!("connection-{generation}")),
            native: true,
        })
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

    /// Browse currently-live rooms without connecting. Read-only: it never
    /// starts a session, awards points, or fires automation. A missing cookie
    /// falls through to TikTok's guest session.
    #[cfg(feature = "native-tiktok")]
    pub async fn live_channels(
        self: &Arc<Self>,
        keyword: Option<String>,
        offset: Option<u32>,
        limit: Option<u32>,
        session_cookie: String,
    ) -> Result<LiveChannelsResult, OperationError> {
        let keyword = normalize_live_keyword(keyword);
        let offset = offset.unwrap_or(0);
        if offset > LIVE_CHANNELS_OFFSET_MAX {
            return Err(OperationError::invalid(format!(
                "offset must be at most {LIVE_CHANNELS_OFFSET_MAX}"
            )));
        }
        let limit = limit.unwrap_or(LIVE_CHANNELS_LIMIT_DEFAULT);
        if limit == 0 || limit > LIVE_CHANNELS_LIMIT_MAX {
            return Err(OperationError::invalid(format!(
                "limit must be between 1 and {LIVE_CHANNELS_LIMIT_MAX}"
            )));
        }
        let page = self
            .live
            .live_channels_page(&session_cookie, &keyword, offset as usize)
            .await
            .map_err(discovery_failure)?;
        // `nextOffset` reflects the raw page width so paging never skips a
        // room when `limit` is smaller than one upstream page.
        let page_len = page.len() as u32;
        let rooms = page
            .into_iter()
            .take(limit as usize)
            .map(|room| LiveChannel {
                unique_id: room.unique_id,
                room_id: room.room_id,
                nickname: room.nickname,
                title: room.title,
                viewers: room.viewers,
            })
            .collect();
        Ok(LiveChannelsResult {
            rooms,
            next_offset: (page_len > 0).then_some(offset.saturating_add(page_len)),
            fetched_at: now_millis(),
            source: LIVE_SEARCH_SOURCE.to_owned(),
        })
    }

    #[cfg(not(feature = "native-tiktok"))]
    pub async fn live_channels(
        self: &Arc<Self>,
        _keyword: Option<String>,
        _offset: Option<u32>,
        _limit: Option<u32>,
        _session_cookie: String,
    ) -> Result<LiveChannelsResult, OperationError> {
        Err(OperationError::unavailable(
            "the native TikTok client is disabled in this build",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_live_keyword;

    #[test]
    fn keyword_defaults_to_live_when_absent_or_blank() {
        assert_eq!(normalize_live_keyword(None), "live");
        assert_eq!(normalize_live_keyword(Some("   ".to_owned())), "live");
    }

    #[test]
    fn keyword_is_trimmed_but_preserved() {
        assert_eq!(
            normalize_live_keyword(Some("  music  ".to_owned())),
            "music"
        );
    }

    #[test]
    fn keyword_is_capped_without_splitting_a_character() {
        let long = "é".repeat(200);
        let capped = normalize_live_keyword(Some(long));
        assert_eq!(capped.chars().count(), 64);
    }
}
