//! Typed `media.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::media::{
    AudioPlaybackResult, MediaPickParams, MediaPickResult, MediaPlayParams, MediaSelection,
    MediaValidateParams,
};
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[MEDIA_VALIDATE, MEDIA_PLAY, MEDIA_PICK];

const MEDIA_VALIDATE: &str = "media.validate";
const MEDIA_PLAY: &str = "media.play";
const MEDIA_PICK: &str = "media.pick";

impl TikToolsClient {
    /// Validates a media path and returns its canonical reference.
    /// RPC method `media.validate`.
    pub async fn media_validate(
        &self,
        params: MediaValidateParams,
    ) -> Result<MediaSelection, ClientError> {
        self.call(MEDIA_VALIDATE, params).await
    }
    /// Plays a local audio file through the host audio backend.
    /// RPC method `media.play`.
    pub async fn media_play(
        &self,
        params: MediaPlayParams,
    ) -> Result<AudioPlaybackResult, ClientError> {
        self.call(MEDIA_PLAY, params).await
    }
    /// Opens the native file dialog (desktop hosts only).
    /// RPC method `media.pick`.
    pub async fn media_pick(
        &self,
        params: MediaPickParams,
    ) -> Result<MediaPickResult, ClientError> {
        self.call(MEDIA_PICK, params).await
    }
}
