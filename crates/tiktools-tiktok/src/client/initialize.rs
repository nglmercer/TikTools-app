//! Session, room, signer and initial socket setup for one owned generation.
use super::{
    bundle::load_signing_bundle,
    config::default_preset,
    metadata::{clean_unique_id, gift_info, verified_room_creator},
    session::resolve_session,
};
use crate::{
    lifecycle, open_socket, retry, ConnectRequest, ConnectionInfo, ManagedConnection,
    NativeTikTokConfig, TikTokError,
};
use std::{sync::Arc, time::Duration};

pub(in super::super) async fn initialize_connection(
    config: NativeTikTokConfig,
    request: ConnectRequest,
    retry: retry::RetryContext,
) -> Result<(ManagedConnection, ConnectionInfo), TikTokError> {
    let unique_id = clean_unique_id(&request.unique_id).ok_or(TikTokError::InvalidCreator)?;
    tracing::info!(creator = %unique_id, requested_room = ?request.room_id, "starting native TikTok connection");
    let cookies = retry
        .run(|| resolve_session(&request.session_cookie))
        .await?;
    tracing::debug!(creator = %unique_id, "TikTok session resolved");
    let preset = default_preset();
    let discovery = ttl_live_discovery::DiscoveryClient::new(&preset)
        .map_err(TikTokError::from_discovery)?
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
            retry
                .run(|| async {
                    discovery
                        .room_lookup(&unique_id)
                        .await
                        .map_err(TikTokError::from_discovery)
                })
                .await?,
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
    let room_info = match lifecycle::interruptible(&retry.stop, async {
        discovery
            .room_info(&room_id)
            .await
            .map_err(TikTokError::from_discovery)
    })
    .await
    {
        Ok(info) => Some(info),
        Err(TikTokError::RateLimited { retry_after }) => {
            retry
                .cooldown
                .extend(retry_after.unwrap_or(Duration::from_secs(30)))
                .await;
            None
        }
        Err(TikTokError::Task(message)) => return Err(TikTokError::Task(message)),
        Err(_) => None,
    };
    retry.run(|| async { Ok(()) }).await?;
    let gifts = match lifecycle::interruptible(&retry.stop, async {
        discovery
            .gift_list(&room_id)
            .await
            .map_err(TikTokError::from_discovery)
    })
    .await
    {
        Ok(gifts) => gifts,
        Err(TikTokError::RateLimited { retry_after }) => {
            retry
                .cooldown
                .extend(retry_after.unwrap_or(Duration::from_secs(30)))
                .await;
            Vec::new()
        }
        Err(TikTokError::Task(message)) => return Err(TikTokError::Task(message)),
        Err(_) => Vec::new(),
    };
    tracing::debug!(room_id = %room_id, gifts = gifts.len(), "TikTok room metadata loaded");
    let bundle = retry.run(|| load_signing_bundle(&config)).await?;
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
    let backend: Arc<dyn ttl_sign_core::SignerBackend> = Arc::new(backend);
    let connection = retry
        .run(|| open_socket(backend.as_ref(), &room_id))
        .await?;
    tracing::info!(room_id = %room_id, "TikTok live WebSocket connected");

    let verified_creator =
        verified_room_creator(&unique_id, &room_id, room_info.as_ref(), lookup.as_ref());
    let info = ConnectionInfo {
        unique_id: verified_creator.clone().unwrap_or_else(|| {
            room_info
                .as_ref()
                .and_then(|info| {
                    (!info.owner.unique_id.is_empty()).then(|| info.owner.unique_id.clone())
                })
                .or_else(|| lookup.as_ref().map(|room| room.unique_id.clone()))
                .unwrap_or_else(|| unique_id.clone())
        }),
        verified_creator,
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

    Ok((
        ManagedConnection {
            connection: Some(connection),
            backend,
            room_id,
            retry,
            info: info.clone(),
        },
        info,
    ))
}
