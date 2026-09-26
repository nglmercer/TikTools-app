//! Typed `automation.*` methods.

use super::TikToolsClient;
use serde_json::Value;
use tiktools_control_api::modules::automation::{
    AutomationContextParams, AutomationContextResult, AutomationCreateParams,
    AutomationDeleteResult, AutomationGetParams, AutomationListParams, AutomationListResult,
    AutomationNodesResult, AutomationRunsResult, AutomationScriptParams, AutomationTestParams,
    AutomationUpdateParams, ScriptAnalysisResult,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[
    AUTOMATION_LIST,
    AUTOMATION_GET,
    AUTOMATION_CREATE,
    AUTOMATION_UPDATE,
    AUTOMATION_DELETE,
    AUTOMATION_ENABLE,
    AUTOMATION_DISABLE,
    AUTOMATION_CONTEXT,
    AUTOMATION_TEST,
    AUTOMATION_NODES_LIST,
    AUTOMATION_SCRIPT_ANALYZE,
    AUTOMATION_RUNS,
    AUTOMATION_SNAPSHOT,
];

const AUTOMATION_LIST: &str = "automation.list";
const AUTOMATION_GET: &str = "automation.get";
const AUTOMATION_CREATE: &str = "automation.create";
const AUTOMATION_UPDATE: &str = "automation.update";
const AUTOMATION_DELETE: &str = "automation.delete";
const AUTOMATION_ENABLE: &str = "automation.enable";
const AUTOMATION_DISABLE: &str = "automation.disable";
const AUTOMATION_CONTEXT: &str = "automation.context";
const AUTOMATION_TEST: &str = "automation.test";
const AUTOMATION_NODES_LIST: &str = "automation.nodes.list";
const AUTOMATION_SCRIPT_ANALYZE: &str = "automation.script.analyze";
const AUTOMATION_RUNS: &str = "automation.runs";
const AUTOMATION_SNAPSHOT: &str = "automation.snapshot";

impl TikToolsClient {
    /// Behavior events and actions (kind: event, action, all).
    /// RPC method `automation.list`.
    pub async fn automation_list(
        &self,
        params: AutomationListParams,
    ) -> Result<AutomationListResult, ClientError> {
        self.call(AUTOMATION_LIST, params).await
    }
    /// One behavior event or action by id.
    /// RPC method `automation.get`.
    pub async fn automation_get(&self, params: AutomationGetParams) -> Result<Value, ClientError> {
        self.call(AUTOMATION_GET, params).await
    }
    /// Creates a behavior event or action (id generated when omitted).
    /// RPC method `automation.create`.
    pub async fn automation_create(
        &self,
        params: AutomationCreateParams,
    ) -> Result<Value, ClientError> {
        self.call(AUTOMATION_CREATE, params).await
    }
    /// Replaces a behavior event or action (must already exist).
    /// RPC method `automation.update`.
    pub async fn automation_update(
        &self,
        params: AutomationUpdateParams,
    ) -> Result<Value, ClientError> {
        self.call(AUTOMATION_UPDATE, params).await
    }
    /// Deletes a behavior event or action.
    /// RPC method `automation.delete`.
    pub async fn automation_delete(
        &self,
        params: AutomationGetParams,
    ) -> Result<AutomationDeleteResult, ClientError> {
        self.call(AUTOMATION_DELETE, params).await
    }
    /// Enables a behavior event or action.
    /// RPC method `automation.enable`.
    pub async fn automation_enable(
        &self,
        params: AutomationGetParams,
    ) -> Result<Value, ClientError> {
        self.call(AUTOMATION_ENABLE, params).await
    }
    /// Disables a behavior event or action.
    /// RPC method `automation.disable`.
    pub async fn automation_disable(
        &self,
        params: AutomationGetParams,
    ) -> Result<Value, ClientError> {
        self.call(AUTOMATION_DISABLE, params).await
    }
    /// Last automation event observed, for context panels and previews.
    /// RPC method `automation.context`.
    pub async fn automation_context(&self) -> Result<AutomationContextResult, ClientError> {
        self.automation_context_for(None).await
    }
    /// Last envelope of one event type (`tiktok.gift`, ...), for emulating
    /// that trigger against real data. `None` returns the global last event.
    /// RPC method `automation.context`.
    pub async fn automation_context_for(
        &self,
        event_type: Option<String>,
    ) -> Result<AutomationContextResult, ClientError> {
        self.call(AUTOMATION_CONTEXT, AutomationContextParams { event_type })
            .await
    }
    /// Dry-runs a saved (id) or inline (record) event or action.
    /// RPC method `automation.test`.
    pub async fn automation_test(
        &self,
        params: AutomationTestParams,
    ) -> Result<Value, ClientError> {
        self.call(AUTOMATION_TEST, params).await
    }
    /// Built-in node catalog for the workflow graph editor.
    /// RPC method `automation.nodes.list`.
    pub async fn automation_nodes_list(&self) -> Result<AutomationNodesResult, ClientError> {
        self.call(AUTOMATION_NODES_LIST, Empty::default()).await
    }
    /// Validates an automation script and returns diagnostics.
    /// RPC method `automation.script.analyze`.
    pub async fn automation_script_analyze(
        &self,
        params: AutomationScriptParams,
    ) -> Result<ScriptAnalysisResult, ClientError> {
        self.call(AUTOMATION_SCRIPT_ANALYZE, params).await
    }
    /// Recent behavior execution runs.
    /// RPC method `automation.runs`.
    pub async fn automation_runs(&self) -> Result<AutomationRunsResult, ClientError> {
        self.call(AUTOMATION_RUNS, Empty::default()).await
    }
    /// Merged behavior snapshot: records plus the live runtime catalog.
    /// RPC method `automation.snapshot`.
    pub async fn automation_snapshot(&self) -> Result<Value, ClientError> {
        self.call(AUTOMATION_SNAPSHOT, Empty::default()).await
    }
}
