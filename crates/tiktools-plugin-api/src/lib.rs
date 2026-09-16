//! Runtime-neutral contracts shared by the host and installable plugins.
//!
//! This crate deliberately does not depend on the desktop, database, Tokio, or
//! TikTok crates. Plugin authors can use it without compiling TikTools itself.

pub mod abi;
pub mod capabilities;
pub mod manifest;
pub mod media;
pub mod protocol;
pub mod text;

pub use abi::{PluginBuffer, PluginInit, PluginStatus, TikToolsPluginApi};
pub use capabilities::{CapabilityId, CapabilitySet, PermissionSet};
pub use manifest::{
    PluginManifest, PluginProcessorDescriptor, PluginRuntimeKind, PluginSecurityModel, PluginTrust,
    ProcessorFailureMode, ProcessorInputDescriptor, ProcessorStage,
};
pub use media::{
    AudioOverlap, AudioPlayOptions, AudioPlaybackResult, MediaDirectoryRef, MediaFileRef,
    MediaKind, MediaPickerMode, MediaPickerOptions, MediaSelection, AUDIO_PLAY_INTENT,
    CAPABILITY_AUDIO_PLAY, CAPABILITY_MEDIA_PICK, CAPABILITY_MEDIA_READ, MEDIA_REFERENCE_VERSION,
};
pub use protocol::{
    read_frame, write_frame, CapabilityRequest, CapabilityResponse, FrameError, PluginRequest,
    PluginResponse, MAX_FRAME_BYTES, METHOD_CALL, METHOD_CAPABILITY_REQUEST,
    METHOD_CAPABILITY_RESPONSE,
};
pub use text::{
    compose_text, resolve_comment_tts, resolve_nickname_tts, strip_emoji, ResolvedText,
    TextComposition,
};

pub const TIKTOOLS_PLUGIN_ABI_VERSION: u32 = 1;
pub const TIKTOOLS_PLUGIN_PROTOCOL_VERSION: u32 = 1;
pub const TIKTOOLS_HOST_API_VERSION: &str = "1.0.0";
pub const TIKTOOLS_UI_API_VERSION: &str = "1.0.0";
