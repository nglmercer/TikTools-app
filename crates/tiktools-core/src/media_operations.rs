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

}
