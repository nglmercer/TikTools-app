//! Read-only room browsing/discovery and the boxed public client adapter.
use super::{
    config::default_preset, errors::session_failure, metadata::clean_unique_id,
    session::resolve_session,
};
use crate::{
    discovery, BoxFuture, ConnectRequest, ConnectionInfo, DiscoveryResult, NativeTikTokClient,
    TikTokClient, TikTokError,
};
use std::time::Duration;

impl NativeTikTokClient {
    /// Reuses the SDK's unsigned, browser-free room lookup. Offline is a
    /// normal observation; transport/auth failures remain errors.
    pub async fn creator_live_room(&self, creator: &str) -> Result<Option<String>, TikTokError> {
        self.check_cooldown().await?;
        let creator = clean_unique_id(creator).ok_or(TikTokError::InvalidCreator)?;
        let discovery = ttl_live_discovery::DiscoveryClient::new(&default_preset())
            .map_err(TikTokError::from_discovery)?;
        match discovery.room_lookup(&creator).await {
            Ok(room) => Ok(room.is_live().then_some(room.room_id)),
            Err(ttl_live_discovery::DiscoveryError::NoRoom(_)) => Ok(None),
            Err(error) => {
                let error = TikTokError::from_discovery(error);
                self.remember_failure(&error).await;
                Err(error)
            }
        }
    }

    /// Browse one page of currently-live rooms without connecting. Read-only
    /// and guest-capable: an empty cookie falls through to the guest session.
    /// The caller controls pagination and selects a creator explicitly.
    pub async fn live_channels_page(
        &self,
        session_cookie: &str,
        keyword: &str,
        offset: usize,
    ) -> Result<Vec<discovery::LiveRoom>, discovery::DiscoveryFailure> {
        self.check_cooldown().await.map_err(session_failure)?;
        let cookies = match resolve_session(session_cookie).await {
            Ok(cookies) => cookies,
            Err(error) => {
                self.remember_failure(&error).await;
                return Err(session_failure(error));
            }
        };
        let client = ttl_live_discovery::DiscoveryClient::new(&default_preset())
            .map_err(discovery::DiscoveryFailure::from_error)?
            .with_session(cookies.to_cookie_string());
        match client.live_channels_page(keyword, offset).await {
            Ok(rooms) => Ok(rooms.into_iter().map(Into::into).collect()),
            Err(error) => {
                let failure = discovery::DiscoveryFailure::from_error(error);
                if failure.kind == discovery::DiscoveryFailureKind::RateLimited {
                    self.cooldown.extend(Duration::from_secs(30)).await;
                }
                Err(failure)
            }
        }
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
            self.check_cooldown().await?;
            let unique_id =
                clean_unique_id(&request.unique_id).ok_or(TikTokError::InvalidCreator)?;
            let cookies = resolve_session(&request.session_cookie).await?;
            let preset = default_preset();
            let client = ttl_live_discovery::DiscoveryClient::new(&preset)
                .map_err(TikTokError::from_discovery)?
                .with_session(cookies.to_cookie_string());
            let lookup = client
                .room_lookup(&unique_id)
                .await
                .map_err(TikTokError::from_discovery)?;
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
