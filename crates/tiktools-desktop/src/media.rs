//! Desktop implementations of the framework-independent media capability.
//!
//! The picker returns a path only. Audio playback streams the selected file
//! directly; no file is copied into the application or plugin directories.
//! The rodio engine lives in the `tiktools-audio-rodio` crate of the plugins
//! repository; this module only adapts it to the host media interface.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tiktools_audio_rodio::{AudioEngine, Overlap};
use tiktools_core::{MediaHost, MediaHostError, MediaHostFuture};
use tiktools_plugin_api::{
    AudioOverlap, AudioPlayOptions, AudioPlaybackResult, MediaKind, MediaPickerMode,
    MediaPickerOptions,
};

#[derive(Default)]
pub struct DesktopMediaHost {
    audio: Arc<AudioEngine>,
}

impl MediaHost for DesktopMediaHost {
    fn open_picker(&self, options: MediaPickerOptions) -> MediaHostFuture<Option<PathBuf>> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || pick_media(options))
                .await
                .map_err(|error| {
                    MediaHostError::Failed(format!("media picker task failed: {error}"))
                })?
        })
    }

    fn play_audio(
        &self,
        file: tiktools_plugin_api::MediaFileRef,
        options: AudioPlayOptions,
    ) -> MediaHostFuture<AudioPlaybackResult> {
        let audio = Arc::clone(&self.audio);
        Box::pin(async move {
            tokio::task::spawn_blocking(move || play_audio(&audio, &file, options))
                .await
                .map_err(|error| MediaHostError::Failed(format!("audio task failed: {error}")))?
        })
    }
}

fn pick_media(options: MediaPickerOptions) -> Result<Option<PathBuf>, MediaHostError> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(title) = options.title.filter(|value| !value.trim().is_empty()) {
        dialog = dialog.set_title(title);
    }
    if let Some(directory) = options
        .initial_directory
        .as_deref()
        .map(Path::new)
        .filter(|path| path.is_dir())
    {
        dialog = dialog.set_directory(directory);
    }
    if !options.extensions.is_empty() {
        let filter_name = match options.kind {
            MediaKind::Audio => "Audio files",
            MediaKind::Video => "Video files",
            MediaKind::Image => "Image files",
            MediaKind::Other => "Supported files",
        };
        dialog = dialog.add_filter(filter_name, &options.extensions);
    }
    let selected = match options.mode {
        MediaPickerMode::File => dialog.pick_file(),
        MediaPickerMode::Directory => dialog.pick_folder(),
    };
    Ok(selected)
}

fn play_audio(
    audio: &AudioEngine,
    file: &tiktools_plugin_api::MediaFileRef,
    options: AudioPlayOptions,
) -> Result<AudioPlaybackResult, MediaHostError> {
    let overlap = match options.overlap {
        AudioOverlap::Allow => Overlap::Allow,
        AudioOverlap::Restart => Overlap::Restart,
        AudioOverlap::Drop => Overlap::Drop,
    };
    let outcome = audio
        .play_file(Path::new(&file.path), options.volume, overlap)
        .map_err(|error| MediaHostError::Failed(error.to_string()))?;
    Ok(AudioPlaybackResult {
        played: outcome.played,
        reason: outcome.reason.map(str::to_owned),
        active_players: outcome.active_players,
    })
}
