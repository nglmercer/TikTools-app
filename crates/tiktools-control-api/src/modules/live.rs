use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::{control::LiveStatus, AppCore};

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

pub fn register(router: &mut ControlRouter) {
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
}
