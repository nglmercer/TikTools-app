//! JSON-safe media references and host media capability messages.
//!
//! A media reference is metadata plus a canonical path. It is deliberately a
//! reference to an existing user file: the host never copies the file into a
//! plugin directory or into the application data directory. Untrusted
//! runtimes must pass the reference back to the host capability broker instead
//! of opening the path themselves.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MEDIA_REFERENCE_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MediaKind {
    #[default]
    Audio,
    Video,
    Image,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaFileRef {
    #[serde(default = "default_media_reference_version")]
    pub reference_version: u32,
    pub path: String,
    #[serde(default)]
    pub directory: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub extension: String,
    #[serde(default)]
    pub kind: Option<MediaKind>,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub modified_at: Option<u64>,
    #[serde(default)]
    pub mime_type: Option<String>,
}

impl MediaFileRef {
    /// Creates a minimal reference for a host capability request. The host
    /// fills metadata and validates the path before it is opened or played.
    pub fn from_path(path: impl Into<String>) -> Self {
        Self {
            reference_version: MEDIA_REFERENCE_VERSION,
            path: path.into(),
            directory: String::new(),
            name: String::new(),
            extension: String::new(),
            kind: None,
            size_bytes: 0,
            modified_at: None,
            mime_type: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaDirectoryRef {
    #[serde(default = "default_media_reference_version")]
    pub reference_version: u32,
    pub path: String,
    #[serde(default)]
    pub name: String,
}

impl MediaDirectoryRef {
    pub fn from_path(path: impl Into<String>) -> Self {
        Self {
            reference_version: MEDIA_REFERENCE_VERSION,
            path: path.into(),
            name: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum MediaPickerMode {
    #[default]
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MediaPickerOptions {
    #[serde(default)]
    pub mode: MediaPickerMode,
    #[serde(default)]
    pub kind: MediaKind,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub initial_directory: Option<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
}

impl Default for MediaPickerOptions {
    fn default() -> Self {
        Self {
            mode: MediaPickerMode::File,
            kind: MediaKind::Audio,
            title: None,
            initial_directory: None,
            extensions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MediaSelection {
    File { file: MediaFileRef },
    Directory { directory: MediaDirectoryRef },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AudioOverlap {
    #[default]
    Allow,
    Restart,
    Drop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioPlayOptions {
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub overlap: AudioOverlap,
}

impl Default for AudioPlayOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            overlap: AudioOverlap::Allow,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AudioPlaybackResult {
    pub played: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub active_players: usize,
}

pub const AUDIO_PLAY_INTENT: &str = "playAudio";
pub const CAPABILITY_MEDIA_PICK: &str = "media.pick";
pub const CAPABILITY_MEDIA_READ: &str = "media.read";
pub const CAPABILITY_AUDIO_PLAY: &str = "audio.play";

fn default_media_reference_version() -> u32 {
    MEDIA_REFERENCE_VERSION
}

fn default_volume() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_selection_is_json_safe_and_does_not_embed_file_bytes() {
        let selection = MediaSelection::File {
            file: MediaFileRef {
                reference_version: MEDIA_REFERENCE_VERSION,
                path: "/music/alert.wav".to_owned(),
                directory: "/music".to_owned(),
                name: "alert.wav".to_owned(),
                extension: "wav".to_owned(),
                kind: Some(MediaKind::Audio),
                size_bytes: 12,
                modified_at: Some(42),
                mime_type: Some("audio/wav".to_owned()),
            },
        };
        let json = serde_json::to_value(&selection).unwrap();
        assert_eq!(json["type"], "file");
        assert_eq!(json["file"]["path"], "/music/alert.wav");
        assert!(json.get("bytes").is_none());
    }

    #[test]
    fn minimal_file_reference_can_be_created_by_a_process_plugin() {
        let reference: MediaFileRef =
            serde_json::from_value(serde_json::json!({"path":"/music/alert.wav"})).unwrap();
        assert_eq!(reference.reference_version, MEDIA_REFERENCE_VERSION);
        assert_eq!(reference.path, "/music/alert.wav");
    }
}
