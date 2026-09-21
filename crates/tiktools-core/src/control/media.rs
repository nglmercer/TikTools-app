//! Media validation, playback, and picker operations.

use super::OperationError;
use crate::services::media_file_ref;
use crate::*;
use tiktools_plugin_api::MediaKind;

impl AppCore {
    // ------------------------------------------------------------------
    // Media.
    // ------------------------------------------------------------------

    pub fn media_validate(
        &self,
        path: &str,
        kind: MediaKind,
    ) -> Result<MediaSelection, OperationError> {
        let trimmed = path.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 {
            return Err(OperationError::invalid("path must be 1..=4096 characters"));
        }
        let file = media_file_ref(std::path::Path::new(trimmed), kind)
            .map_err(|error| OperationError::invalid(error.to_string()))?;
        Ok(MediaSelection::File { file })
    }

    pub async fn media_play(
        &self,
        path: &str,
        kind: Option<MediaKind>,
        volume: Option<f32>,
    ) -> Result<AudioPlaybackResult, OperationError> {
        let trimmed = path.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 {
            return Err(OperationError::invalid("path must be 1..=4096 characters"));
        }
        let volume = volume.unwrap_or(1.0);
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(OperationError::invalid("volume must be within 0.0..=1.0"));
        }
        let file = media_file_ref(std::path::Path::new(trimmed), kind.unwrap_or_default())
            .map_err(|error| OperationError::invalid(error.to_string()))?;
        self.play_audio(
            file,
            AudioPlayOptions {
                volume,
                ..AudioPlayOptions::default()
            },
        )
        .await
        .map_err(|error| OperationError::unavailable(error.to_string()))
    }

    /// Desktop-only native file dialog. Headless hosts return
    /// `capability_unavailable`; callers must not emulate a dialog.
    pub async fn media_pick(
        &self,
        options: MediaPickerOptions,
    ) -> Result<Option<MediaSelection>, OperationError> {
        if options
            .title
            .as_ref()
            .is_some_and(|title| title.len() > 256)
        {
            return Err(OperationError::invalid("title is too long (max 256)"));
        }
        if options
            .initial_directory
            .as_ref()
            .is_some_and(|directory| directory.len() > 4096)
        {
            return Err(OperationError::invalid(
                "initialDirectory is too long (max 4096)",
            ));
        }
        if options.extensions.len() > 32
            || options.extensions.iter().any(|extension| {
                extension.is_empty()
                    || extension.len() > 16
                    || !extension.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '_')
                    })
            })
        {
            return Err(OperationError::invalid(
                "extensions must be at most 32 alphanumeric tokens",
            ));
        }
        match self.open_media_picker(options).await {
            Ok(selection) => Ok(selection),
            Err(MediaApiError::Validation(error)) => {
                Err(OperationError::invalid(error.to_string()))
            }
            Err(MediaApiError::Host(MediaHostError::Unavailable(message))) => {
                Err(OperationError::capability_unavailable(message))
            }
            Err(MediaApiError::Host(MediaHostError::Failed(message))) => {
                Err(OperationError::unavailable(message))
            }
        }
    }
}
