use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::AppCore;
use tiktools_plugin_api::{
    AudioPlaybackResult, MediaKind, MediaPickerMode, MediaPickerOptions, MediaSelection,
};

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaPickParams {
    #[serde(default)]
    pub mode: Option<MediaPickerMode>,
    #[serde(default)]
    pub kind: Option<MediaKind>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub initial_directory: Option<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaPickResult {
    pub selection: Option<MediaSelection>,
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
    router.register_typed_full::<MediaPickParams, MediaPickResult, _, _>(
        "media.pick",
        "Opens the native file dialog (desktop hosts only)",
        true,
        false,
        true,
        |core: Arc<AppCore>, params: MediaPickParams| async move {
            let options = MediaPickerOptions {
                mode: params.mode.unwrap_or_default(),
                kind: params.kind.unwrap_or_default(),
                title: params.title,
                initial_directory: params.initial_directory,
                extensions: params.extensions,
            };
            core.media_pick(options)
                .await
                .map(|selection| MediaPickResult { selection })
                .map_err(ApiError::from)
        },
    );
}
