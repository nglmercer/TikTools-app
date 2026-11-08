//! Host-owned media references and the audio/picker capability boundary.
//!
//! The core only deals in validated paths and serializable metadata. A
//! desktop implementation supplies the native picker and audio output; tests
//! can use the no-op implementation without pulling any GUI/audio dependency
//! into this crate.

use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    time::UNIX_EPOCH,
};

use thiserror::Error;
use tiktools_plugin_api::{
    AudioPlayOptions, AudioPlaybackResult, MediaDirectoryRef, MediaFileRef, MediaKind,
    MediaPickerMode, MediaPickerOptions, MediaSelection, MEDIA_REFERENCE_VERSION,
};

pub const AUDIO_EXTENSIONS: &[&str] = &[
    "aac", "flac", "m4a", "mp3", "mp4", "oga", "ogg", "opus", "wav", "webm",
];
pub const MAX_AUDIO_FILE_BYTES: u64 = 1_073_741_824;

pub type MediaHostFuture<T> = Pin<Box<dyn Future<Output = Result<T, MediaHostError>> + Send>>;

/// The only desktop functionality required by the framework-independent core.
/// Implementations must not copy the selected media file.
pub trait MediaHost: Send + Sync {
    /// Whether this host provides the default audio output sink.
    fn supports_audio(&self) -> bool {
        true
    }
    fn open_picker(&self, options: MediaPickerOptions) -> MediaHostFuture<Option<PathBuf>>;
    fn play_audio(
        &self,
        file: MediaFileRef,
        options: AudioPlayOptions,
    ) -> MediaHostFuture<AudioPlaybackResult>;
}

#[derive(Debug, Error)]
pub enum MediaHostError {
    #[error("media host is unavailable: {0}")]
    Unavailable(String),
    #[error("media host failed: {0}")]
    Failed(String),
}

#[derive(Debug, Error)]
pub enum MediaApiError {
    #[error(transparent)]
    Validation(#[from] MediaError),
    #[error(transparent)]
    Host(#[from] MediaHostError),
}

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("media path is empty")]
    EmptyPath,
    #[error("media path contains a NUL byte")]
    NulByte,
    #[error("media path is not valid UTF-8: {0}")]
    NonUtf8Path(PathBuf),
    #[error("media path does not exist: {0}")]
    NotFound(PathBuf),
    #[error("media path is not a regular file: {0}")]
    NotAFile(PathBuf),
    #[error("media path is not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("media path escapes the allowed relative directory: {0}")]
    OutsideRelativeRoot(PathBuf),
    #[error("audio file type is not supported: {0}")]
    UnsupportedAudioType(PathBuf),
    #[error("audio file is larger than the 1 GiB limit: {0}")]
    TooLarge(PathBuf),
    #[error("media reference version {0} is not supported")]
    UnsupportedReferenceVersion(u32),
    #[error("media option is invalid: {0}")]
    InvalidOption(&'static str),
    #[error("media path metadata could not be read: {0}")]
    Metadata(#[from] std::io::Error),
}

#[derive(Debug, Default)]
pub struct NoopMediaHost;

impl MediaHost for NoopMediaHost {
    fn supports_audio(&self) -> bool {
        false
    }
    fn open_picker(&self, _options: MediaPickerOptions) -> MediaHostFuture<Option<PathBuf>> {
        Box::pin(async {
            Err(MediaHostError::Unavailable(
                "native media dialogs are only available in the desktop host".to_owned(),
            ))
        })
    }

    fn play_audio(
        &self,
        _file: MediaFileRef,
        _options: AudioPlayOptions,
    ) -> MediaHostFuture<AudioPlaybackResult> {
        Box::pin(async {
            Err(MediaHostError::Unavailable(
                "audio output is only available in the desktop host".to_owned(),
            ))
        })
    }
}

/// Validates a file selected by the user and builds fresh metadata from the
/// filesystem. Only the path is retained; file bytes never enter TikTools.
pub fn media_file_ref(path: &Path, kind: MediaKind) -> Result<MediaFileRef, MediaError> {
    let canonical = canonical_file(path)?;
    let metadata = std::fs::metadata(&canonical)?;
    let size = metadata.len();
    if size > MAX_AUDIO_FILE_BYTES && kind == MediaKind::Audio {
        return Err(MediaError::TooLarge(canonical));
    }
    let extension = canonical
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if kind == MediaKind::Audio && !AUDIO_EXTENSIONS.contains(&extension.as_str()) {
        return Err(MediaError::UnsupportedAudioType(canonical));
    }
    let path_string = canonical
        .to_str()
        .ok_or_else(|| MediaError::NonUtf8Path(canonical.clone()))?
        .to_owned();
    let directory = canonical.parent().unwrap_or_else(|| Path::new(""));
    let directory = directory
        .to_str()
        .ok_or_else(|| MediaError::NonUtf8Path(canonical.clone()))?
        .to_owned();
    let name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| MediaError::NonUtf8Path(canonical.clone()))?
        .to_owned();
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .and_then(|value| u64::try_from(value.as_millis()).ok());

    Ok(MediaFileRef {
        reference_version: MEDIA_REFERENCE_VERSION,
        path: path_string,
        directory,
        name,
        extension: extension.clone(),
        kind: Some(kind),
        size_bytes: size,
        modified_at,
        mime_type: mime_type(kind, &extension).map(str::to_owned),
    })
}

/// Validates a path stored in an action. Absolute paths refer to the original
/// user file. Relative paths are supported only below the supplied app data
/// root, so an old action cannot escape through `../` after migration.
pub fn audio_file_ref_from_config(
    raw_path: &str,
    relative_root: &Path,
) -> Result<MediaFileRef, MediaError> {
    media_file_ref_from_config(raw_path, relative_root, MediaKind::Audio)
}

fn media_file_ref_from_config(
    raw_path: &str,
    relative_root: &Path,
    kind: MediaKind,
) -> Result<MediaFileRef, MediaError> {
    let raw_path = raw_path.trim();
    if raw_path.is_empty() {
        return Err(MediaError::EmptyPath);
    }
    if raw_path.contains('\0') {
        return Err(MediaError::NulByte);
    }
    let path = Path::new(raw_path);
    if path.is_absolute() {
        return media_file_ref(path, kind);
    }

    let root = std::fs::canonicalize(relative_root).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => MediaError::NotFound(relative_root.to_owned()),
        _ => MediaError::Metadata(error),
    })?;
    let candidate = root.join(path);
    let canonical =
        std::fs::canonicalize(&candidate).map_err(|_| MediaError::NotFound(candidate.clone()))?;
    if !canonical.starts_with(&root) {
        return Err(MediaError::OutsideRelativeRoot(canonical));
    }
    media_file_ref(&canonical, kind)
}

pub fn media_selection_from_path(
    path: &Path,
    mode: MediaPickerMode,
) -> Result<MediaSelection, MediaError> {
    media_selection_from_path_with_kind(path, mode, MediaKind::Audio)
}

pub fn media_selection_from_path_with_kind(
    path: &Path,
    mode: MediaPickerMode,
    kind: MediaKind,
) -> Result<MediaSelection, MediaError> {
    match mode {
        MediaPickerMode::File => Ok(MediaSelection::File {
            file: media_file_ref(path, kind)?,
        }),
        MediaPickerMode::Directory => Ok(MediaSelection::Directory {
            directory: media_directory_ref(path)?,
        }),
    }
}

/// Validates the public picker contract before it reaches a native dialog.
/// This keeps process/VM callers from passing unbounded strings or path-like
/// extension values into a platform backend.
pub fn validate_media_picker_options(options: &MediaPickerOptions) -> Result<(), MediaError> {
    if options
        .title
        .as_deref()
        .is_some_and(|value| value.len() > 256 || value.contains('\0'))
    {
        return Err(MediaError::InvalidOption("title"));
    }
    if options
        .initial_directory
        .as_deref()
        .is_some_and(|value| value.len() > 4_096 || value.contains('\0'))
    {
        return Err(MediaError::InvalidOption("initialDirectory"));
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
        return Err(MediaError::InvalidOption("extensions"));
    }
    Ok(())
}

pub fn media_directory_ref(path: &Path) -> Result<MediaDirectoryRef, MediaError> {
    let canonical =
        std::fs::canonicalize(path).map_err(|_| MediaError::NotFound(path.to_owned()))?;
    if !canonical.is_dir() {
        return Err(MediaError::NotADirectory(canonical));
    }
    let path_string = canonical
        .to_str()
        .ok_or_else(|| MediaError::NonUtf8Path(canonical.clone()))?
        .to_owned();
    let name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path_string.as_str())
        .to_owned();
    Ok(MediaDirectoryRef {
        reference_version: MEDIA_REFERENCE_VERSION,
        path: path_string,
        name,
    })
}

pub fn validate_media_file_ref(
    file: &MediaFileRef,
    relative_root: &Path,
) -> Result<MediaFileRef, MediaError> {
    if file.reference_version != MEDIA_REFERENCE_VERSION {
        return Err(MediaError::UnsupportedReferenceVersion(
            file.reference_version,
        ));
    }
    media_file_ref_from_config(
        &file.path,
        relative_root,
        file.kind.unwrap_or(MediaKind::Other),
    )
}

/// Validates a reference specifically for the audio output capability. A
/// missing kind is treated as audio for compatibility with old saved actions
/// that stored only a path.
pub fn validate_audio_file_ref(
    file: &MediaFileRef,
    relative_root: &Path,
) -> Result<MediaFileRef, MediaError> {
    if file.reference_version != MEDIA_REFERENCE_VERSION {
        return Err(MediaError::UnsupportedReferenceVersion(
            file.reference_version,
        ));
    }
    audio_file_ref_from_config(&file.path, relative_root)
}

fn canonical_file(path: &Path) -> Result<PathBuf, MediaError> {
    let canonical = std::fs::canonicalize(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => MediaError::NotFound(path.to_owned()),
        _ => MediaError::Metadata(error),
    })?;
    if !canonical.is_file() {
        return Err(MediaError::NotAFile(canonical));
    }
    Ok(canonical)
}

fn mime_type(kind: MediaKind, extension: &str) -> Option<&'static str> {
    if kind != MediaKind::Audio {
        return None;
    }
    Some(match extension {
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "mp4" => "audio/mp4",
        "mp3" => "audio/mpeg",
        "oga" | "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        "webm" => "audio/webm",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn fixture_root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tiktools-media-{suffix}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn builds_a_reference_without_copying_the_file() {
        let root = fixture_root();
        let path = root.join("alert.wav");
        fs::write(&path, b"not decoded here").unwrap();
        let reference = media_file_ref(&path, MediaKind::Audio).unwrap();
        assert_eq!(
            reference.path,
            fs::canonicalize(&path).unwrap().to_string_lossy()
        );
        assert_eq!(
            reference.directory,
            fs::canonicalize(&root).unwrap().to_string_lossy()
        );
        assert_eq!(reference.size_bytes, 16);
        assert!(path.is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_relative_path_escape_and_unsupported_audio() {
        let root = fixture_root();
        fs::write(root.join("alert.txt"), b"no").unwrap();
        assert!(matches!(
            audio_file_ref_from_config("alert.txt", &root),
            Err(MediaError::UnsupportedAudioType(_))
        ));
        assert!(matches!(
            audio_file_ref_from_config("../alert.wav", &root),
            Err(MediaError::NotFound(_) | MediaError::OutsideRelativeRoot(_))
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn serializable_file_ref_metadata_is_rebuilt_on_validation() {
        let root = fixture_root();
        let path = root.join("alert.mp3");
        fs::write(&path, b"mp3").unwrap();
        let minimal = MediaFileRef::from_path(path.to_string_lossy());
        let validated = validate_audio_file_ref(&minimal, &root).unwrap();
        assert_eq!(validated.kind, Some(MediaKind::Audio));
        assert_eq!(validated.extension, "mp3");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_mp4_as_an_audio_container_reference() {
        let root = fixture_root();
        let path = root.join("alert.MP4");
        fs::write(&path, b"mp4 audio container").unwrap();

        let reference = media_file_ref(&path, MediaKind::Audio).unwrap();

        assert_eq!(reference.extension, "mp4");
        assert_eq!(reference.mime_type.as_deref(), Some("audio/mp4"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn generic_media_validation_preserves_non_audio_kind() {
        let root = fixture_root();
        let path = root.join("clip.mp4");
        fs::write(&path, b"video").unwrap();
        let reference = MediaFileRef {
            kind: Some(MediaKind::Video),
            ..MediaFileRef::from_path(path.to_string_lossy())
        };
        let validated = validate_media_file_ref(&reference, &root).unwrap();
        assert_eq!(validated.kind, Some(MediaKind::Video));
        assert_eq!(validated.extension, "mp4");
        let _ = fs::remove_dir_all(root);
    }
}
