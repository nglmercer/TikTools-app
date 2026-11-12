//! Host media picker and audio operations with capability validation.
use super::*;

impl AppCore {
    /// Opens the host-owned native picker and returns a validated reference to
    /// the existing file/directory. The result contains metadata and a
    /// canonical path only; the host never copies the selected bytes.
    pub async fn open_media_picker(
        &self,
        options: MediaPickerOptions,
    ) -> Result<Option<MediaSelection>, MediaApiError> {
        validate_media_picker_options(&options)?;
        let mode = options.mode;
        let kind = options.kind;
        let path = self.media.open_picker(options).await?;
        path.map(|path| media_selection_from_path_with_kind(&path, mode, kind))
            .transpose()
            .map_err(MediaApiError::from)
    }

    /// Validates a media reference immediately before playback and then hands
    /// the canonical path to the desktop audio backend. This is the only core
    /// entry point used by automation and runtime plugins for local audio.
    pub async fn play_audio(
        &self,
        file: MediaFileRef,
        options: AudioPlayOptions,
    ) -> Result<AudioPlaybackResult, MediaApiError> {
        if !options.volume.is_finite() {
            return Err(MediaApiError::Validation(MediaError::InvalidOption(
                "volume",
            )));
        }
        let file = validate_audio_file_ref(&file, self.db.paths().data.as_path())?;
        self.media
            .play_audio(
                file,
                AudioPlayOptions {
                    volume: options.volume.clamp(0.0, 1.0),
                    ..options
                },
            )
            .await
            .map_err(MediaApiError::from)
    }
}
