//! Headless operations API: the single authoritative operation path.
//!
//! Every client (WebView IPC, CLI, JSON stdio, local IPC, tests, agents) is
//! expected to go through these value-returning operations. Unlike the
//! `PageMessage` handlers, these methods never emit [`HostMessage`]s; the
//! WebView layer emits UI messages after calling the same operations.
//!
//! [`HostMessage`]: crate::ipc::messages::HostMessage

mod analytics;
mod automation;
mod owner_approval_review;
pub use owner_approval_review::{
    OwnerApprovalBlockReason, OwnerApprovalEligibility, OwnerApprovalReviewParams,
    OwnerApprovalReviewResult, OwnerApprovalStatus, OwnerApprovalStatusKind, OwnerApprovalSubject,
};
pub(crate) mod automation_approval;
pub use automation_approval::{ApprovalActivateParams, ApprovalEnablement, ApprovalRevokeParams};
pub(crate) mod automation_adoption;
pub use automation_adoption::{
    OwnerAdoptionIntent, OwnerAdoptionOutcome, OwnerAdoptionRecord, OwnerAdoptionReview,
    OwnerAdoptionState, OwnerAdoptionSubmit,
};
pub(crate) mod owner_approval;
pub use owner_approval::{
    OwnerApprovalIntent, OwnerApprovalOperation, OwnerApprovalOutcome, OwnerApprovalReceipt,
    OwnerApprovalRecord, OwnerApprovalReviewUnknown, OwnerApprovalState, OwnerApprovalSubmit,
};
pub(crate) mod owner_definition;
pub use owner_definition::{
    OwnerDefinitionAcknowledgement, OwnerDefinitionIntent, OwnerDefinitionOperation,
    OwnerDefinitionOutcome, OwnerDefinitionReceipt, OwnerDefinitionRecord,
    OwnerDefinitionReviewUnknown, OwnerDefinitionState, OwnerDefinitionSubmission,
    OwnerDefinitionSubmit,
};
pub(crate) mod owner_library;
pub use owner_library::{
    owner_library_canonical_json, owner_library_command_hash, owner_library_command_params,
    owner_library_definition_digest, owner_library_digest, owner_library_versions_digest,
    validate_owner_library_submit, OwnerLibraryAcknowledgement, OwnerLibraryCatalog,
    OwnerLibraryEntry, OwnerLibraryIntent, OwnerLibraryMutation, OwnerLibraryOperation,
    OwnerLibraryOutcome, OwnerLibraryPack, OwnerLibraryProfile, OwnerLibraryReceipt,
    OwnerLibraryRecord, OwnerLibraryReviewUnknown, OwnerLibrarySnapshot, OwnerLibraryState,
    OwnerLibrarySubmission, OwnerLibrarySubmit, OWNER_LIBRARY_MAX_BYTES,
    OWNER_LIBRARY_MAX_CATALOG_BYTES, OWNER_LIBRARY_MAX_RECORDS,
};
#[cfg(feature = "cloud-auth")]
mod cloud_account;
#[cfg(feature = "cloud-auth")]
mod cloud_library;
#[cfg(all(feature = "cloud-auth", feature = "http", feature = "plugin-install"))]
mod cloud_publisher;
#[cfg(feature = "cloud-auth")]
pub(crate) mod cloud_sync;
#[cfg(feature = "cloud-auth")]
mod cloud_workspace;
mod commands;
mod competitions;
mod delivery;
mod effects;
pub use effects::{EffectHistory, EffectKind, EffectReceipt, EffectState};
mod streams;
#[cfg(feature = "persistence")]
pub(crate) use commands::CoordinationWorker;
pub use commands::{CommandCaller, WorkflowMutation};
#[cfg(test)]
pub(crate) use competitions::bound_ranking_presentation;
#[cfg(feature = "plugin-install")]
mod cloud_catalog;
mod globals;
mod helpers;
mod live;
mod media;
#[cfg(feature = "plugin-install")]
mod plugin_packages;
mod plugin_settings;
mod plugins;
mod points;
pub use points::{valid_currency, PointsAdjustment};
mod processors;
mod rankings;
mod system;
mod widgets;
mod workflows;

pub use crate::input_access::InputAccessResult;
pub use analytics::GiftDebugResult;
pub use automation::{
    AutomationEnablementMiss, AutomationKind, AutomationMutation, AutomationRecords,
    AutomationSetEnablementResult, ScriptAnalysisResult,
};
#[cfg(feature = "cloud-auth")]
pub use cloud_account::CloudLoginUrl;
#[cfg(feature = "plugin-install")]
pub use cloud_catalog::{
    CloudCatalogAdvisory, CloudCatalogPage, CloudCatalogPaginationCapabilities, CloudCatalogQuery,
    CloudCatalogSummariesPage, CloudCatalogSummariesQuery, CloudCatalogSummary,
    CloudCatalogVersion, CloudCatalogVersionsPage, CloudCatalogVersionsQuery,
};
#[cfg(feature = "cloud-auth")]
pub use cloud_library::{
    CloudLibraryDocument, CloudLibraryDocumentMeta, CloudLibraryKind, CloudLibraryPage,
    CloudLibraryQuery, CloudLibraryRead, CloudLibraryScope, CloudLibraryStatus,
};
#[cfg(feature = "cloud-auth")]
pub use cloud_sync::{
    SyncNowResult, SyncRecoveryChoice, SyncRecoveryOutcome, SyncRecoveryReview, SyncRecoveryValue,
};
#[cfg(feature = "cloud-auth")]
pub use cloud_workspace::{WorkspaceV2Discovery, WorkspaceV2DiscoveryState};
#[cfg(any(test, feature = "native-tiktok"))]
pub(crate) use helpers::check_session_cookie_len;
pub(crate) use helpers::{clean_plugin_id, clean_record_id, fresh_record_id};
pub use live::{LiveChannel, LiveChannelsResult, LiveStatus};
#[cfg(feature = "plugin-install")]
pub use plugin_packages::{
    AvailableRelease, PluginVersions, PolicyPatch, RegistryAddInput, RegistryPackageView,
    RegistryRootKey, RegistryRootPreview,
};
pub use plugin_settings::{PluginConnectionResult, PluginProvisionResult, PluginSettingsResult};
pub use plugins::{PluginActionOutcome, PluginInstallResult};
pub use processors::ProcessorOutcomeDto;
pub use rankings::{ContributionOutcome, ExternalContribution};
pub use system::{DoctorCheck, DoctorReport};
pub use widgets::{WidgetsCopyResult, WidgetsState, WidgetsStatusResult, GATEWAY_PLUGIN_ID};

/// Machine-readable operation failure. The control API maps this 1:1 onto
/// the JSON-RPC error envelope (`code` + `message`).
#[derive(Debug, Clone, PartialEq)]
pub struct OperationError {
    pub code: &'static str,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl OperationError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found",
            message: message.into(),
            data: None,
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_params",
            message: message.into(),
            data: None,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "unavailable",
            message: message.into(),
            data: None,
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "conflict",
            message: message.into(),
            data: None,
        }
    }

    /// Authenticated caller acts outside its own plugin scope (cross-plugin
    /// action invocation or option-source read). The owner check resolves
    /// through the action catalog, never through name prefixes.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            code: "forbidden",
            message: message.into(),
            data: None,
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            code: "timeout",
            message: message.into(),
            data: None,
        }
    }

    /// Desktop-only capability (native dialogs, audio output) requested on a
    /// headless host. Callers must not emulate the capability.
    pub fn capability_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "capability_unavailable",
            message: message.into(),
            data: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal",
            message: message.into(),
            data: None,
        }
    }

    #[cfg(feature = "plugin-install")]
    pub fn install(error: tiktools_plugin_installer::InstallError) -> Self {
        Self {
            code: "install_failed",
            message: error.message.clone(),
            data: serde_json::to_value(&error).ok(),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for OperationError {}

impl From<tiktools_plugin_loader::PluginLoaderError> for OperationError {
    fn from(error: tiktools_plugin_loader::PluginLoaderError) -> Self {
        match &error {
            tiktools_plugin_loader::PluginLoaderError::NotFound(_) => {
                Self::not_found(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::RuntimeUnavailable(_)
                if cfg!(target_os = "android") =>
            {
                Self::capability_unavailable(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::RuntimeUnavailable(_)
            | tiktools_plugin_loader::PluginLoaderError::Overloaded(_) => {
                Self::unavailable(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::Timeout(_) => {
                Self::timeout(error.to_string())
            }
            tiktools_plugin_loader::PluginLoaderError::Manifest(_)
            | tiktools_plugin_loader::PluginLoaderError::InvalidDirectory(_)
            | tiktools_plugin_loader::PluginLoaderError::Runtime(_)
            | tiktools_plugin_loader::PluginLoaderError::LockPoisoned(_) => {
                Self::internal(error.to_string())
            }
        }
    }
}

pub use crate::services::token_import::{TokenImportCandidate, TokenImportResult};
