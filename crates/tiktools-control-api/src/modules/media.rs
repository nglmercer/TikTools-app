use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::AppCore;
use tiktools_plugin_api::{AudioPlaybackResult, MediaKind, MediaSelection};

use crate::{error::ApiError, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaValidateParams {
    pub path: String,
    #[serde(default)]
    pub kind: Option<MediaKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlayParams {
    pub path: String,
    #[serde(default)]
    pub kind: Option<MediaKind>,
    #[serde(default)]
    pub volume: Option<f32>,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<MediaValidateParams, MediaSelection, _, _>(
        "media.validate",
        "Validates a media path and returns its canonical reference",
        false,
        |core: Arc<AppCore>, params: MediaValidateParams| async move {
            core.media_validate(&params.path, params.kind.unwrap_or_default())
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<MediaPlayParams, AudioPlaybackResult, _, _>(
        "media.play",
        "Plays a local audio file through the host audio backend",
        true,
        |core: Arc<AppCore>, params: MediaPlayParams| async move {
            core.media_play(&params.path, params.kind, params.volume)
                .await
                .map_err(ApiError::from)
        },
    );
}
