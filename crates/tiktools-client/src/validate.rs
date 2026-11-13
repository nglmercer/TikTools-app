//! Params validation for dry-run and agent flows.
//!
//! Deserializes caller JSON into the registry's own typed params,
//! mirroring host validation exactly without executing anything.

use super::ClientError;
use serde::Deserialize;
use tiktools_control_api::modules::analytics::AnalyticsSummaryParams;
use tiktools_control_api::modules::app::{AppStateGetParams, AppStateSetParams};
use tiktools_control_api::modules::automation::{
    AutomationAdoptParams, AutomationCreateParams, AutomationFireParams, AutomationGetParams,
    AutomationListParams, AutomationManifestParams, AutomationScriptParams,
    AutomationSetEnablementParams, AutomationTestParams, AutomationUpdateParams,
};
use tiktools_control_api::modules::automation_adoption::{
    OwnerAdoptionEmpty, OwnerAdoptionReview, OwnerAdoptionSubmit,
};
use tiktools_control_api::modules::automation_approval::{
    ApprovalActivateParams, ApprovalRevokeParams, OwnerApprovalEmpty, OwnerApprovalReviewParams,
    OwnerApprovalReviewUnknown, OwnerApprovalSubmit,
};
use tiktools_control_api::modules::automation_definition::{
    OwnerDefinitionEmpty, OwnerDefinitionReviewUnknown, OwnerDefinitionSubmit,
};
use tiktools_control_api::modules::automation_effects::{EffectGetParams, EffectHistoryParams};
use tiktools_control_api::modules::automation_library::{
    OwnerLibraryEmpty, OwnerLibraryReviewUnknown, OwnerLibrarySubmit,
};
#[cfg(feature = "cloud-auth")]
use tiktools_control_api::modules::bundles::LibraryItemsQuery;
#[cfg(feature = "cloud-auth")]
use tiktools_control_api::modules::bundles::{
    AssetIdParams, AssetSetLabelParams, AssetUploadParams, CatalogForkParams, CloudPageQuery,
    OverlayCreateParams, OverlayIdParams, OverlayReviseParams, OverlayRevisionParams,
    OverlayRevisionsQuery, PackageCreateParams, PackageIdParams,
};
#[cfg(feature = "cloud-auth")]
use tiktools_control_api::modules::cloud::CloudLoginOperationParams;
use tiktools_control_api::modules::competitions::{
    CompetitionActionId, CompetitionId, CompetitionResultsRequest, CompetitionSummaryParams,
    Definition as CompetitionDefinition, ResolveCompetitionAction, SelectWinner,
};
use tiktools_control_api::modules::creators::{CreatorGetParams, CreatorsRecentParams};
use tiktools_control_api::modules::gifts::GiftDebugParams;
use tiktools_control_api::modules::globals::{
    GlobalsDeleteParams, GlobalsGetParams, GlobalsSetParams, RuntimeVariablesReplace,
};
#[cfg(feature = "cloud-auth")]
use tiktools_control_api::modules::integrations::{IntegrationConnectParams, IntegrationIdParams};
use tiktools_control_api::modules::live::{
    LiveChannelsParams, LiveConnectParams, LiveMonitorStartParams, LivePickParams,
};
use tiktools_control_api::modules::media::{MediaPickParams, MediaPlayParams, MediaValidateParams};
use tiktools_control_api::modules::packages::{
    JobParams, PackageInspectParams, PackageInstallParams, PackagePluginParams,
    PackagePolicyParams, PackageRollbackParams, PackageUpdateParams,
};
use tiktools_control_api::modules::plugins::{
    PluginActionParams, PluginIdParams, PluginInstallParams, PluginInstallSetParams,
    PluginOptionsParams, PluginProvisionParams, PluginUninstallParams,
};
use tiktools_control_api::modules::points::{
    PartialPointsConfig, PointsAdjustParams, PointsLeaderboardParams, PointsResetParams,
    PointsViewerParams,
};
use tiktools_control_api::modules::processors::ProcessorTestParams;
#[cfg(feature = "cloud-auth")]
use tiktools_control_api::modules::publisher::{
    PublisherCreateParams, PublisherEnrollParams, PublisherIdParams, PublisherReleaseParams,
    PublisherSubmitParams, PublisherUploadParams,
};
use tiktools_control_api::modules::rankings::{
    Definition, ExternalContribution, RankingId, RankingUpdate,
};
use tiktools_control_api::modules::registry::{
    CloudCatalogItemQuery, CloudCatalogItemsQuery, CloudCatalogQuery, CloudCatalogSummariesQuery,
    CloudCatalogVersionsQuery, RegistryDefaults, RegistryPackageParams, RegistryRefreshParams,
    SearchQuery,
};
use tiktools_control_api::modules::rpc::SchemaParams;
use tiktools_control_api::modules::settings::{PluginSettingsGet, PluginSettingsSet};
#[cfg(feature = "cloud-auth")]
use tiktools_control_api::modules::sync::{
    CloudLibraryQuery, CloudLibraryRead, SyncResolveParams, SyncUncertainResolveParams,
    SyncUncertainReviewParams,
};
use tiktools_control_api::modules::system::SaveFileParams;
#[cfg(feature = "cloud-auth")]
use tiktools_control_api::modules::teams::{
    CommandRunParams, TeamCreateParams, TeamDecisionParams, TeamDeviceParams, TeamEnrollParams,
    TeamGrantRevokeParams, TeamGrantUpdateParams, TeamHostParams, TeamInvitationCreateParams,
    TeamInvitationParams, TeamRedeemParams, TeamSessionParams, TeamUpdateParams,
};
use tiktools_control_api::modules::updates::{CheckParams as UpdateCheckParams, NoUpdateParams};
use tiktools_control_api::modules::widgets::{WidgetsCopyCloudOverlayParams, WidgetsCopyParams};
use tiktools_control_api::modules::workflows::{WorkflowIdParams, WorkflowSaveParams};
use tiktools_control_api::modules::Empty;

/// Validates `params` for `method` without executing it.
/// Unknown methods fail with `method_not_found`; shape mismatches
/// fail with `invalid_params`, exactly like the host.
pub fn validate_params(method: &str, params: &serde_json::Value) -> Result<(), ClientError> {
    let invalid = |error: serde_json::Error| ClientError::new("invalid_params", error.to_string());
    match method {
        "collaboration.hello" => {
            validate::<tiktools_control_api::modules::collaboration::HelloParams>(params)
        }
        "collaboration.me" => validate::<Empty>(params),
        "collaboration.subscribe" => {
            validate::<tiktools_control_api::modules::collaboration::SubscribeParams>(params)
        }
        "collaboration.snapshot" => {
            validate::<tiktools_control_api::modules::collaboration::SnapshotParams>(params)
        }
        "collaboration.resume" => {
            validate::<tiktools_control_api::modules::collaboration::ResumeParams>(params)
        }
        "collaboration.commands.get" => {
            validate::<tiktools_control_api::modules::collaboration::CommandGet>(params)
        }
        "collaboration.status" => validate::<Empty>(params),
        "collaboration.invites.list"
        | "collaboration.requests.list"
        | "collaboration.sessions.list"
        | "collaboration.principals.list" => {
            validate::<tiktools_control_api::modules::collaboration::ListQuery>(params)
        }
        "collaboration.configure" => {
            validate::<tiktools_control_api::modules::collaboration::ConfigureInput>(params)
        }
        "collaboration.invites.create" => {
            validate::<tiktools_control_api::modules::collaboration::InviteCreate>(params)
        }
        "collaboration.invites.revoke"
        | "collaboration.requests.approve"
        | "collaboration.requests.reject"
        | "collaboration.sessions.revoke"
        | "collaboration.principals.get"
        | "collaboration.principals.revoke" => {
            validate::<tiktools_control_api::modules::collaboration::IdParams>(params)
        }
        "collaboration.grants.set" => {
            validate::<tiktools_control_api::modules::collaboration::GrantSet>(params)
        }
        "collaboration.audit.list" => {
            validate::<tiktools_control_api::modules::collaboration::AuditQuery>(params)
        }
        "plugins.ui.assets" => PluginIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "ui.feed.backfill" => {
            tiktools_control_api::modules::ui::FeedBackfillParams::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "ui.feed.creator" => {
            tiktools_control_api::modules::ui::CreatorFeedParams::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "ui.feed.browser" => {
            tiktools_control_api::modules::ui::BrowserFeedParams::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "analytics.summary" => AnalyticsSummaryParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "app.state.get" => AppStateGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "app.state.set" => AppStateSetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "app.update.check" => UpdateCheckParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "app.update.status" | "app.update.download" | "app.update.install" => {
            NoUpdateParams::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "automation.adopt" => AutomationAdoptParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.adoption.get" | "automation.adoption.resolve" => {
            validate::<OwnerAdoptionEmpty>(params)
        }
        "automation.adoption.submit" => validate::<OwnerAdoptionSubmit>(params),
        "automation.adoption.review_unknown" => validate::<OwnerAdoptionReview>(params),
        "automation.approval.activate" => validate::<ApprovalActivateParams>(params),
        "automation.approval.revoke" => validate::<ApprovalRevokeParams>(params),
        "automation.approval.review" => validate::<OwnerApprovalReviewParams>(params),
        "automation.approval.get" | "automation.approval.resolve" => {
            validate::<OwnerApprovalEmpty>(params)
        }
        "automation.approval.submit" => validate::<OwnerApprovalSubmit>(params),
        "automation.approval.review_unknown" => validate::<OwnerApprovalReviewUnknown>(params),
        "automation.definition.get" | "automation.definition.resolve" => {
            validate::<OwnerDefinitionEmpty>(params)
        }
        "automation.definition.submit" => validate::<OwnerDefinitionSubmit>(params),
        "automation.definition.review_unknown" => validate::<OwnerDefinitionReviewUnknown>(params),
        "automation.library.snapshot" | "automation.library.get" | "automation.library.resolve" => {
            validate::<OwnerLibraryEmpty>(params)
        }
        "automation.library.submit" => validate::<OwnerLibrarySubmit>(params),
        "automation.library.review_unknown" => validate::<OwnerLibraryReviewUnknown>(params),
        "automation.effects.get" => serde_json::from_value::<EffectGetParams>(params.clone())
            .map_err(invalid)?
            .validate()
            .map_err(|error| ClientError::new(error.code, error.message)),
        "automation.effects.history" => {
            serde_json::from_value::<EffectHistoryParams>(params.clone())
                .map_err(invalid)?
                .validate()
                .map_err(|error| ClientError::new(error.code, error.message))
        }
        "automation.authority.get" => AutomationGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.review" => AutomationGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.source.get" => AutomationGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.manifest.get" | "automation.delegation.preflight" => {
            validate::<AutomationManifestParams>(params)
        }
        "automation.list" => AutomationListParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.get" => AutomationGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.create" => AutomationCreateParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.update" => AutomationUpdateParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.delete" => AutomationGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.enable" => AutomationGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.disable" => AutomationGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.set_enablement" => AutomationSetEnablementParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.context" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "automation.test" => AutomationTestParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.fire" => AutomationFireParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.nodes.list" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "automation.script.analyze" => AutomationScriptParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.runs" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "automation.snapshot" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "competitions.create" | "competitions.update" => CompetitionDefinition::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "competitions.action" | "competitions.markActionUncertain" => {
            CompetitionActionId::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "competitions.resolveAction" => ResolveCompetitionAction::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "competitions.selectWinner" => SelectWinner::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "competitions.results" => CompetitionResultsRequest::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "competitions.summary" => CompetitionSummaryParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "competitions.list" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "competitions.get"
        | "competitions.delete"
        | "competitions.state"
        | "competitions.finish"
        | "competitions.candidates"
        | "competitions.start"
        | "competitions.pause"
        | "competitions.resume"
        | "competitions.cancel" => CompetitionId::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "creators.get" => CreatorGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "creators.recent" => CreatorsRecentParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "creators.history.clear" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "gifts.list" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "gifts.debug" => GiftDebugParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "globals.list" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "globals.typed.list" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "globals.typed.replace" => RuntimeVariablesReplace::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "globals.get" => GlobalsGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "globals.set" => GlobalsSetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "globals.delete" => GlobalsDeleteParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "live.platforms" | "live.connections" | "live.platform.analytics" => {
            Empty::deserialize(params).map(|_| ()).map_err(invalid)
        }
        "live.platform.connect" => {
            tiktools_control_api::modules::live::PlatformConnectParams::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "live.platform.disconnect" => {
            tiktools_control_api::modules::live::PlatformDisconnectParams::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "live.platform.credentials.store" => {
            tiktools_control_api::modules::live::PlatformCredentialParams::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "live.platform.credentials.delete" => {
            tiktools_control_api::modules::live::PlatformCredentialDelete::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "live.connect" => LiveConnectParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "live.pick" => LivePickParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "live.disconnect" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "live.status" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "live.channels" => LiveChannelsParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "live.monitor.start" => LiveMonitorStartParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "live.monitor.stop" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "live.monitor.status" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "media.validate" => MediaValidateParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "media.play" => MediaPlayParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "media.pick" => MediaPickParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.list" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "plugins.get" => PluginIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.install" => PluginInstallParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.uninstall" => PluginUninstallParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.enable" => PluginIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.disable" => PluginIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.start" => PluginIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.stop" => PluginIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.health" => PluginIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.diagnostics" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "plugins.options" => PluginOptionsParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.action.execute" => PluginActionParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.token.import" => {
            tiktools_control_api::modules::plugins::PluginTokenImportParams::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "plugins.token.provision" => PluginProvisionParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.install.set" => PluginInstallSetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "rankings.create" => Definition::deserialize(params).map(|_| ()).map_err(invalid),
        "rankings.contribute" => ExternalContribution::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "rankings.update" => RankingUpdate::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "rankings.metrics" | "rankings.list" => {
            Empty::deserialize(params).map(|_| ()).map_err(invalid)
        }
        "rankings.get" | "rankings.snapshot" | "rankings.delete" | "rankings.reset"
        | "rankings.pause" | "rankings.resume" => {
            RankingId::deserialize(params).map(|_| ()).map_err(invalid)
        }
        "points.config.get" | "points.info" => {
            Empty::deserialize(params).map(|_| ()).map_err(invalid)
        }
        "points.config.set" => PartialPointsConfig::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "points.view" => {
            tiktools_control_api::modules::points::PointsViewParams::deserialize(params)
                .map(|_| ())
                .map_err(invalid)
        }
        "points.viewer.get" => PointsViewerParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "points.adjust" => PointsAdjustParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "points.leaderboard" => PointsLeaderboardParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "points.reset" => PointsResetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "processors.list" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "processors.status" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "processors.test" => ProcessorTestParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "rpc.discover" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "rpc.schema" => SchemaParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.settings.get" => PluginSettingsGet::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.settings.set" => PluginSettingsSet::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.settings.reset" => PluginSettingsGet::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "system.info" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "system.health" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "system.snapshot" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "system.doctor" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "system.shutdown" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "system.ping" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "system.requestInputAccess" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "system.saveFile" => SaveFileParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "widgets.status" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "widgets.copyObsUrl" => WidgetsCopyParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "widgets.copyCloudOverlayUrl" => WidgetsCopyCloudOverlayParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "workflows.list" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "workflows.get" => WorkflowIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "workflows.save" => WorkflowSaveParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "workflows.delete" => WorkflowIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "workflows.enable" => WorkflowIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "workflows.disable" => WorkflowIdParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.packages.inspect" => validate::<PackageInspectParams>(params),
        "plugins.packages.install" => validate::<PackageInstallParams>(params),
        "plugins.packages.update" => validate::<PackageUpdateParams>(params),
        "plugins.packages.rollback" => validate::<PackageRollbackParams>(params),
        "plugins.packages.versions" => validate::<PackagePluginParams>(params),
        "plugins.packages.policy" => validate::<PackagePolicyParams>(params),
        "plugins.jobs.get" | "plugins.jobs.cancel" => validate::<JobParams>(params),
        "plugins.packages.receipts"
        | "plugins.updates.check"
        | "plugins.updates.list"
        | "plugins.jobs.list"
        | "registry.list"
        | "registry.cloudCatalogPaginationCapabilities"
        | "registry.defaults.get" => validate::<Empty>(params),
        "registry.refresh" => validate::<RegistryRefreshParams>(params),
        "registry.search" => validate::<SearchQuery>(params),
        "registry.cloudCatalog" => validate::<CloudCatalogQuery>(params),
        "registry.cloudCatalogSummaries" => validate::<CloudCatalogSummariesQuery>(params),
        "catalog.items" => validate::<CloudCatalogItemsQuery>(params),
        "catalog.item" => validate::<CloudCatalogItemQuery>(params),
        "registry.cloudCatalogVersions" => validate::<CloudCatalogVersionsQuery>(params),
        "registry.package" => validate::<RegistryPackageParams>(params),
        "registry.defaults.set" => validate::<RegistryDefaults>(params),
        #[cfg(feature = "cloud-auth")]
        "cloud.status" | "cloud.login.reserve" | "cloud.restore" | "cloud.logout" => {
            validate::<Empty>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "cloud.login.begin"
        | "cloud.login.open"
        | "cloud.login.complete"
        | "cloud.login.cancel" => validate::<CloudLoginOperationParams>(params),
        #[cfg(feature = "cloud-auth")]
        "sync.status"
        | "sync.enable"
        | "sync.disable"
        | "sync.now"
        | "sync.uncertain.list"
        | "sync.v2.discover"
        | "sync.v2.selected"
        | "sync.v2.preview"
        | "sync.v2.now" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "sync.resolve" => validate::<SyncResolveParams>(params),
        #[cfg(feature = "cloud-auth")]
        "sync.v2.migrate"
        | "sync.v2.rebuild"
        | "sync.v2.inventory.review"
        | "sync.v2.changes.list"
        | "sync.v2.conflict.list" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "sync.v2.variables.review" | "sync.v2.variables.fetch" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "sync.v2.variables.apply" | "sync.v2.variables.send" | "sync.v2.variables.withdraw" => {
            validate::<tiktools_control_api::modules::sync::VariableSyncApply>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "sync.v2.inventory.repair" => {
            validate::<tiktools_control_api::modules::sync::WorkspaceInventoryRepairParams>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "sync.v2.section" => {
            validate::<tiktools_control_api::modules::sync::SyncSectionParams>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "sync.v2.conflict.review" => {
            validate::<tiktools_control_api::modules::sync::SyncV2ConflictReviewParams>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "sync.v2.conflict.resolve" => {
            validate::<tiktools_control_api::modules::sync::SyncV2ConflictResolveParams>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "sync.v2.stageVariables" => {
            validate::<tiktools_control_api::modules::sync::SyncV2StageVariablesParams>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "sync.v2.stagePluginSettings" => {
            validate::<tiktools_control_api::modules::sync::SyncV2StagePluginSettingsParams>(params)
        }
        // Preferences are a JSON object; `null` must not validate.
        #[cfg(feature = "cloud-auth")]
        "sync.v2.stagePreferences" => {
            validate::<serde_json::Map<String, serde_json::Value>>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "sync.uncertain.review" => validate::<SyncUncertainReviewParams>(params),
        #[cfg(feature = "cloud-auth")]
        "cloudLibrary.list" => validate::<CloudLibraryQuery>(params),
        #[cfg(feature = "cloud-auth")]
        "cloudLibrary.read" => validate::<CloudLibraryRead>(params),
        #[cfg(feature = "cloud-auth")]
        "sync.uncertain.resolve" => validate::<SyncUncertainResolveParams>(params),
        #[cfg(feature = "cloud-auth")]
        "catalog.mine" => validate::<LibraryItemsQuery>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.assets.upload" => validate::<AssetUploadParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.overlays.create" => validate::<OverlayCreateParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.overlays.read" => validate::<OverlayIdParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.overlays.revise" => validate::<OverlayReviseParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.packages.create" => validate::<PackageCreateParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.packages.read" => validate::<PackageIdParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.packages.preflight" => validate::<PackageIdParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.packages.delete" => validate::<PackageIdParams>(params),
        #[cfg(feature = "cloud-auth")]
        "catalog.fork" => validate::<CatalogForkParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.assets.list" | "bundles.overlays.list" => validate::<CloudPageQuery>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.assets.setLabel" => validate::<AssetSetLabelParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.assets.delete" => validate::<AssetIdParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.overlays.revisions" => validate::<OverlayRevisionsQuery>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.overlays.revision" => validate::<OverlayRevisionParams>(params),
        #[cfg(feature = "cloud-auth")]
        "bundles.overlays.delete" => validate::<OverlayIdParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.invitations.redeem" => validate::<TeamRedeemParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.commands.run" => validate::<CommandRunParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.host.status" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.host.start" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.host.stop" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.hosts.list" | "teams.accessible.list" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.people.search" => {
            validate::<tiktools_control_api::modules::teams::TeamPeopleSearchParams>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "teams.hosts.create" => validate::<TeamCreateParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.hosts.get" => validate::<TeamHostParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.hosts.update" => validate::<TeamUpdateParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.devices.list" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.devices.enroll" => validate::<TeamEnrollParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.devices.revoke" => validate::<TeamDeviceParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.invitations.list" => validate::<TeamHostParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.invitations.create" => validate::<TeamInvitationCreateParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.invitations.revoke" => validate::<TeamInvitationParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.requests.list" => validate::<TeamHostParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.requests.decide" => validate::<TeamDecisionParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.grants.list" => validate::<TeamHostParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.grants.update" => validate::<TeamGrantUpdateParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.grants.revoke" => validate::<TeamGrantRevokeParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.sessions.list" => validate::<TeamHostParams>(params),
        #[cfg(feature = "cloud-auth")]
        "teams.sessions.revoke" => validate::<TeamSessionParams>(params),
        #[cfg(feature = "cloud-auth")]
        "integrations.connections" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "integrations.connect" => validate::<IntegrationConnectParams>(params),
        #[cfg(feature = "cloud-auth")]
        "integrations.events" | "integrations.disconnect" => {
            validate::<IntegrationIdParams>(params)
        }
        #[cfg(feature = "cloud-auth")]
        "publisher.list" => validate::<Empty>(params),
        #[cfg(feature = "cloud-auth")]
        "publisher.create" => validate::<PublisherCreateParams>(params),
        #[cfg(feature = "cloud-auth")]
        "publisher.delete" => validate::<PublisherIdParams>(params),
        #[cfg(feature = "cloud-auth")]
        "publisher.keys" => validate::<PublisherIdParams>(params),
        #[cfg(feature = "cloud-auth")]
        "publisher.keys.enroll" => validate::<PublisherEnrollParams>(params),
        #[cfg(feature = "cloud-auth")]
        "publisher.uploads.send" => validate::<PublisherUploadParams>(params),
        #[cfg(feature = "cloud-auth")]
        "publisher.releases.submit" => validate::<PublisherSubmitParams>(params),
        #[cfg(feature = "cloud-auth")]
        "publisher.releases.get" | "publisher.publication.request" => {
            validate::<PublisherReleaseParams>(params)
        }
        _ => Err(ClientError::new(
            "method_not_found",
            format!("unknown method `{method}`"),
        )),
    }
}

fn validate<T: serde::de::DeserializeOwned>(params: &serde_json::Value) -> Result<(), ClientError> {
    T::deserialize(params)
        .map(|_| ())
        .map_err(|error| ClientError::new("invalid_params", error.to_string()))
}
