pub(crate) mod aggregations;
mod app_state;
mod automation;
#[cfg(feature = "persistence")]
pub(crate) use automation::manifest as automation_manifest;
#[cfg(feature = "persistence")]
pub(crate) mod automation_approval_policy;
mod capabilities;
mod catalog;
#[cfg(feature = "persistence")]
pub(crate) mod competition_actions;
#[cfg(feature = "persistence")]
pub(crate) mod competition_events;
pub(crate) mod competitions;
pub(crate) mod declarative_http;
#[cfg(feature = "persistence")]
pub(crate) mod effects;
pub mod globals;
mod live;
mod media;
pub(crate) mod option_sources;
mod points;
pub(crate) use points::valid_viewer_id;
#[cfg(all(test, feature = "persistence"))]
pub(crate) use points::AdmissionPause;
#[cfg(feature = "native-tiktok")]
pub(crate) mod points_work;
mod script;
pub mod templates;
pub(crate) mod token_import;
pub(crate) mod token_provision;

pub use app_state::AppStateService;
#[cfg(feature = "persistence")]
pub(crate) use automation::approval::{
    ApprovalCandidate, RuntimeApprovalPermit, RuntimeOwnerRuleFence,
};
pub use automation::moderation::{
    moderation_penalty_action_record, moderation_penalty_event_record, validate_penalty_points,
    MODERATION_BLOCKED_PATH, MODERATION_POINTS_ACTION_TYPE, MODERATION_VIEWER_TEMPLATE,
};
pub(crate) use automation::read_event_path;
pub use automation::AutomationService;
#[cfg(feature = "persistence")]
pub(crate) use automation::RuntimeAuthority;
pub(crate) use automation::{MatchedAction, MatchedRecord};
pub(crate) use capabilities::PluginSettingsBinding;
pub use capabilities::{
    redact_secret_settings, secret_setting_keys, CapabilityBroker, CapabilityError,
    SECRET_SETTING_PLACEHOLDER,
};
pub use catalog::{builtin_action_types, builtin_node_catalog, builtin_translations};
pub use live::LiveService;
pub use media::{
    audio_file_ref_from_config, media_directory_ref, media_file_ref, media_selection_from_path,
    media_selection_from_path_with_kind, validate_audio_file_ref, validate_media_file_ref,
    validate_media_picker_options, MediaApiError, MediaError, MediaHost, MediaHostError,
    MediaHostFuture, NoopMediaHost, AUDIO_EXTENSIONS, MAX_AUDIO_FILE_BYTES,
};
#[cfg(feature = "persistence")]
pub(crate) use points::PointsWriteOp;
pub use points::{AwardOptions, PointAction, PointAward, PointsService, ViewerState};
pub use points::{PointsDiagnostics, DEFAULT_HOT_VIEWER_CAPACITY};
pub use script::ScriptService;
#[cfg(feature = "persistence")]
pub(crate) use script::SCOPED_SCRIPT_LIMITS;
