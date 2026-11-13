use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use tiktools_core::control::{LiveChannelsResult, LiveStatus};
pub use tiktools_core::control::{
    PlatformConnectParams, PlatformConnectionSnapshot, PlatformCredentialParams,
    PlatformCredentialRef, PlatformDescriptor, PlatformDisconnectParams, PlatformId,
};
pub use tiktools_core::live_monitor::LiveMonitorStatus;
use tiktools_core::AppCore;

use crate::{error::ApiError, modules::Empty, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveConnectParams {
    pub unique_id: String,
    pub session_cookie: String,
    #[serde(default)]
    pub room_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LivePickParams {
    pub session_cookie: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveChannelsParams {
    /// Search hint; trimmed and capped by the host. Defaults to `live`.
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub offset: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
    /// Optional signed-in session. Empty uses TikTok's guest session and is
    /// never logged or returned.
    #[serde(default)]
    pub session_cookie: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveMonitorStartParams {
    pub unique_id: String,
    #[serde(default)]
    pub session_cookie: String,
    #[serde(default)]
    pub poll_interval_seconds: Option<u64>,
}

pub fn register(router: &mut ControlRouter) {
    register_platforms(router);
    router.register_typed::<LiveMonitorStartParams, LiveMonitorStatus, _, _>(
        "live.monitor.start",
        "Monitors a creator and connects when LIVE",
        true,
        |core: Arc<AppCore>, params: LiveMonitorStartParams| async move {
            core.live_monitor_start(
                params.unique_id,
                params.session_cookie,
                params.poll_interval_seconds,
            )
            .await
            .map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, LiveMonitorStatus, _, _>(
        "live.monitor.stop", "Stops monitoring and disconnects its LIVE session", true,
        |core: Arc<AppCore>, _: Empty| async move { Ok::<_, ApiError>(core.live_monitor_stop().await) },
    );
    router.register_typed::<Empty, LiveMonitorStatus, _, _>(
        "live.monitor.status",
        "Current native creator monitor state",
        false,
        |core: Arc<AppCore>, _: Empty| async move { Ok::<_, ApiError>(core.live_monitor_status()) },
    );
    router.register_typed::<LiveConnectParams, LiveStatus, _, _>(
        "live.connect",
        "Connects the native TikTok live client",
        true,
        |core: Arc<AppCore>, params: LiveConnectParams| async move {
            core.live_connect(params.unique_id, params.session_cookie, params.room_id)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<LivePickParams, LiveStatus, _, _>(
        "live.pick",
        "Connects to the top live room for a session cookie",
        true,
        |core: Arc<AppCore>, params: LivePickParams| async move {
            core.live_pick(params.session_cookie)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, LiveStatus, _, _>(
        "live.disconnect",
        "Disconnects the live client",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<LiveStatus, ApiError>(core.live_disconnect().await)
        },
    );
    router.register_typed::<Empty, LiveStatus, _, _>(
        "live.status",
        "Live connection status",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<LiveStatus, ApiError>(core.live_status())
        },
    );
    router.register_typed::<LiveChannelsParams, LiveChannelsResult, _, _>(
        "live.channels",
        "Browses currently live rooms without connecting",
        false,
        |core: Arc<AppCore>, params: LiveChannelsParams| async move {
            core.live_channels(
                params.keyword,
                params.offset,
                params.limit,
                params.session_cookie,
            )
            .await
            .map_err(ApiError::from)
        },
    );
}

fn register_platforms(router: &mut ControlRouter) {
    use tiktools_core::control::{
        PlatformConnectParams, PlatformConnectionSnapshot, PlatformCredentialParams,
        PlatformCredentialRef, PlatformDescriptor, PlatformDisconnectParams,
    };
    router.register_typed::<Empty, Vec<PlatformDescriptor>, _, _>(
        "live.platforms",
        "Installed live platform capabilities",
        false,
        |core: Arc<AppCore>, _: Empty| async move {
            Ok::<_, ApiError>(core.live_platform_descriptors())
        },
    );
    router.register_typed::<Empty, Vec<PlatformConnectionSnapshot>, _, _>(
        "live.connections",
        "Managed connections across platforms",
        false,
        |core: Arc<AppCore>, _: Empty| async move {
            Ok::<_, ApiError>(core.live_platform_connections())
        },
    );
    router.register_typed::<PlatformConnectParams, PlatformConnectionSnapshot, _, _>(
        "live.platform.connect",
        "Connect a platform using stored credentials",
        true,
        |core: Arc<AppCore>, params| async move {
            core.live_platform_connect(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PlatformDisconnectParams, (), _, _>(
        "live.platform.disconnect",
        "Disconnect one managed connection",
        true,
        |core: Arc<AppCore>, params| async move {
            core.live_platform_disconnect(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PlatformCredentialParams, PlatformCredentialRef, _, _>(
        "live.platform.credentials.store",
        "Store a private platform credential",
        true,
        |core: Arc<AppCore>, params| async move {
            core.live_platform_store_credential(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<PlatformCredentialDelete, (), _, _>(
        "live.platform.credentials.delete",
        "Forget a stored platform credential",
        true,
        |core: Arc<AppCore>, params: PlatformCredentialDelete| async move {
            core.live_platform_delete_credential(params.platform, &params.credential_ref)
                .await
                .map_err(ApiError::from)
        },
    );
    #[cfg(feature = "persistence")]
    router.register_typed::<Empty, Vec<serde_json::Value>, _, _>(
        "live.platform.analytics",
        "Recent daily activity by platform and channel",
        false,
        |core: Arc<AppCore>, _: Empty| async move {
            tokio::task::spawn_blocking(move || core.db.platform_analytics())
                .await
                .map_err(|_| {
                    ApiError::from(tiktools_core::control::OperationError::unavailable(
                        "Analytics task failed",
                    ))
                })?
                .map_err(|_| {
                    ApiError::from(tiktools_core::control::OperationError::unavailable(
                        "Platform analytics unavailable",
                    ))
                })
        },
    );
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformCredentialDelete {
    pub platform: tiktools_core::control::PlatformId,
    pub credential_ref: String,
}
