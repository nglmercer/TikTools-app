//! Desktop implementations of the framework-independent media capability.
//!
//! The picker returns a path only; no file is copied into the application or
//! plugin directories. Playback is not a host feature: it belongs to an
//! installed audio provider plugin, which the core routes `audio.play` to.

use std::path::{Path, PathBuf};

use tiktools_core::{MediaHost, MediaHostError, MediaHostFuture};
use tiktools_plugin_api::{MediaKind, MediaPickerMode, MediaPickerOptions};

#[derive(Default)]
pub struct DesktopMediaHost;

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
