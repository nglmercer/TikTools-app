//! Typed `workflows.*` methods.

use super::TikToolsClient;
use serde_json::Value;
use tiktools_control_api::modules::workflows::{
    WorkflowDeleteResult, WorkflowIdParams, WorkflowListResult, WorkflowSaveParams,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[
    WORKFLOWS_LIST,
    WORKFLOWS_GET,
    WORKFLOWS_SAVE,
    WORKFLOWS_DELETE,
    WORKFLOWS_ENABLE,
    WORKFLOWS_DISABLE,
];

const WORKFLOWS_LIST: &str = "workflows.list";
const WORKFLOWS_GET: &str = "workflows.get";
const WORKFLOWS_SAVE: &str = "workflows.save";
const WORKFLOWS_DELETE: &str = "workflows.delete";
const WORKFLOWS_ENABLE: &str = "workflows.enable";
const WORKFLOWS_DISABLE: &str = "workflows.disable";

impl TikToolsClient {
    /// Graph workflows for the node-canvas editor.
    /// RPC method `workflows.list`.
    pub async fn workflows_list(&self) -> Result<WorkflowListResult, ClientError> {
        self.call(WORKFLOWS_LIST, Empty::default()).await
    }
    /// One graph workflow by id.
    /// RPC method `workflows.get`.
    pub async fn workflows_get(&self, params: WorkflowIdParams) -> Result<Value, ClientError> {
        self.call(WORKFLOWS_GET, params).await
    }
    /// Creates or replaces a graph workflow (graph must carry id and name).
    /// RPC method `workflows.save`.
    pub async fn workflows_save(&self, params: WorkflowSaveParams) -> Result<Value, ClientError> {
        self.call(WORKFLOWS_SAVE, params).await
    }
    /// Deletes a graph workflow.
    /// RPC method `workflows.delete`.
    pub async fn workflows_delete(
        &self,
        params: WorkflowIdParams,
    ) -> Result<WorkflowDeleteResult, ClientError> {
        self.call(WORKFLOWS_DELETE, params).await
    }
    /// Enables a graph workflow.
    /// RPC method `workflows.enable`.
    pub async fn workflows_enable(&self, params: WorkflowIdParams) -> Result<Value, ClientError> {
        self.call(WORKFLOWS_ENABLE, params).await
    }
    /// Disables a graph workflow.
    /// RPC method `workflows.disable`.
    pub async fn workflows_disable(&self, params: WorkflowIdParams) -> Result<Value, ClientError> {
        self.call(WORKFLOWS_DISABLE, params).await
    }
}
