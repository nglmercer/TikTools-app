//! Params validation for dry-run and agent flows.
//!
//! Deserializes caller JSON into the registry's own typed params,
//! mirroring host validation exactly without executing anything.

use super::ClientError;
use serde::Deserialize;
use tiktools_control_api::modules::analytics::AnalyticsSummaryParams;
use tiktools_control_api::modules::app::{AppStateGetParams, AppStateSetParams};
use tiktools_control_api::modules::automation::{
    AutomationCreateParams, AutomationGetParams, AutomationListParams, AutomationScriptParams,
    AutomationTestParams, AutomationUpdateParams,
};
use tiktools_control_api::modules::creators::{CreatorGetParams, CreatorsRecentParams};
use tiktools_control_api::modules::gifts::GiftDebugParams;
use tiktools_control_api::modules::live::{LiveConnectParams, LivePickParams};
use tiktools_control_api::modules::media::{MediaPickParams, MediaPlayParams, MediaValidateParams};
use tiktools_control_api::modules::plugins::{
    PluginActionParams, PluginIdParams, PluginInstallParams, PluginInstallSetParams,
    PluginOptionsParams, PluginProvisionParams,
};
use tiktools_control_api::modules::points::{
    PartialPointsConfig, PointsAdjustParams, PointsLeaderboardParams, PointsResetParams,
    PointsViewerParams,
};
use tiktools_control_api::modules::processors::ProcessorTestParams;
use tiktools_control_api::modules::rpc::SchemaParams;
use tiktools_control_api::modules::settings::{PluginSettingsGet, PluginSettingsSet};
use tiktools_control_api::modules::workflows::{WorkflowIdParams, WorkflowSaveParams};
use tiktools_control_api::modules::Empty;

/// Validates `params` for `method` without executing it.
/// Unknown methods fail with `method_not_found`; shape mismatches
/// fail with `invalid_params`, exactly like the host.
pub fn validate_params(method: &str, params: &serde_json::Value) -> Result<(), ClientError> {
    let invalid = |error: serde_json::Error| ClientError::new("invalid_params", error.to_string());
    match method {
        "analytics.summary" => AnalyticsSummaryParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "app.state.get" => AppStateGetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "app.state.set" => AppStateSetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
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
        "automation.context" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "automation.test" => AutomationTestParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.nodes.list" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "automation.script.analyze" => AutomationScriptParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "automation.runs" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "automation.snapshot" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
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
        "live.connect" => LiveConnectParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "live.pick" => LivePickParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "live.disconnect" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "live.status" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
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
        "plugins.uninstall" => PluginIdParams::deserialize(params)
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
        "plugins.token.provision" => PluginProvisionParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "plugins.install.set" => PluginInstallSetParams::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
        "points.config.get" => Empty::deserialize(params).map(|_| ()).map_err(invalid),
        "points.config.set" => PartialPointsConfig::deserialize(params)
            .map(|_| ())
            .map_err(invalid),
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
        _ => Err(ClientError::new(
            "method_not_found",
            format!("unknown method `{method}`"),
        )),
    }
}
