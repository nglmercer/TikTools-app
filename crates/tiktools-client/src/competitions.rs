//! Typed competition definition operations.
use super::TikToolsClient;
use tiktools_control_api::{
    modules::{
        competitions::{
            CompetitionActionId, CompetitionActionResult, CompetitionId, CompetitionList,
            CompetitionResults, CompetitionResultsRequest, CompetitionSummaryPage,
            CompetitionSummaryParams, Definition, LifecycleState, ManualCandidates,
            ResolveCompetitionAction, ResultRecord, SelectWinner,
        },
        Empty, OkResult,
    },
    ClientError,
};
pub(crate) const METHODS: &[&str] = &[
    "competitions.create",
    "competitions.state",
    "competitions.results",
    "competitions.action",
    "competitions.markActionUncertain",
    "competitions.resolveAction",
    "competitions.start",
    "competitions.finish",
    "competitions.candidates",
    "competitions.selectWinner",
    "competitions.pause",
    "competitions.resume",
    "competitions.cancel",
    "competitions.update",
    "competitions.delete",
    "competitions.list",
    "competitions.get",
    "competitions.summary",
];
impl TikToolsClient {
    pub async fn competitions_summary(
        &self,
        params: CompetitionSummaryParams,
    ) -> Result<CompetitionSummaryPage, ClientError> {
        self.call("competitions.summary", params).await
    }
    pub async fn competitions_create(
        &self,
        definition: Definition,
    ) -> Result<OkResult, ClientError> {
        self.call("competitions.create", definition).await
    }
    pub async fn competitions_update(
        &self,
        definition: Definition,
    ) -> Result<OkResult, ClientError> {
        self.call("competitions.update", definition).await
    }
    pub async fn competitions_delete(&self, id: CompetitionId) -> Result<OkResult, ClientError> {
        self.call("competitions.delete", id).await
    }
    pub async fn competitions_action(
        &self,
        params: CompetitionActionId,
    ) -> Result<CompetitionActionResult, ClientError> {
        self.call("competitions.action", params).await
    }
    pub async fn competitions_mark_action_uncertain(
        &self,
        params: CompetitionActionId,
    ) -> Result<OkResult, ClientError> {
        self.call("competitions.markActionUncertain", params).await
    }
    pub async fn competitions_resolve_action(
        &self,
        params: ResolveCompetitionAction,
    ) -> Result<OkResult, ClientError> {
        self.call("competitions.resolveAction", params).await
    }
    pub async fn competitions_results(
        &self,
        request: CompetitionResultsRequest,
    ) -> Result<CompetitionResults, ClientError> {
        self.call("competitions.results", request).await
    }
    pub async fn competitions_state(
        &self,
        id: CompetitionId,
    ) -> Result<LifecycleState, ClientError> {
        self.call("competitions.state", id).await
    }
    pub async fn competitions_candidates(
        &self,
        id: CompetitionId,
    ) -> Result<ManualCandidates, ClientError> {
        self.call("competitions.candidates", id).await
    }
    pub async fn competitions_select_winner(
        &self,
        params: SelectWinner,
    ) -> Result<ResultRecord, ClientError> {
        self.call("competitions.selectWinner", params).await
    }
    pub async fn competitions_finish(
        &self,
        id: CompetitionId,
    ) -> Result<ResultRecord, ClientError> {
        self.call("competitions.finish", id).await
    }
    pub async fn competitions_start(
        &self,
        id: CompetitionId,
    ) -> Result<LifecycleState, ClientError> {
        self.call("competitions.start", id).await
    }
    pub async fn competitions_pause(
        &self,
        id: CompetitionId,
    ) -> Result<LifecycleState, ClientError> {
        self.call("competitions.pause", id).await
    }
    pub async fn competitions_resume(
        &self,
        id: CompetitionId,
    ) -> Result<LifecycleState, ClientError> {
        self.call("competitions.resume", id).await
    }
    pub async fn competitions_cancel(
        &self,
        id: CompetitionId,
    ) -> Result<LifecycleState, ClientError> {
        self.call("competitions.cancel", id).await
    }
    pub async fn competitions_list(&self) -> Result<CompetitionList, ClientError> {
        self.call("competitions.list", Empty::default()).await
    }
    pub async fn competitions_get(&self, id: CompetitionId) -> Result<Definition, ClientError> {
        self.call("competitions.get", id).await
    }
}
