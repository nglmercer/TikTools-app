//! Typed ranking operations.
use super::TikToolsClient;
use tiktools_control_api::{
    modules::{
        rankings::{
            ContributionOutcome, Definition, ExternalContribution, MetricList, RankingId,
            RankingList, RankingUpdate, Snapshot,
        },
        Empty, OkResult,
    },
    ClientError,
};
pub(crate) const METHODS: &[&str] = &[
    "rankings.metrics",
    "rankings.create",
    "rankings.update",
    "rankings.list",
    "rankings.get",
    "rankings.snapshot",
    "rankings.delete",
    "rankings.reset",
    "rankings.pause",
    "rankings.resume",
    "rankings.contribute",
];
impl TikToolsClient {
    pub async fn rankings_metrics(&self) -> Result<MetricList, ClientError> {
        self.call("rankings.metrics", Empty::default()).await
    }
    pub async fn rankings_create(&self, definition: Definition) -> Result<OkResult, ClientError> {
        self.call("rankings.create", definition).await
    }
    pub async fn rankings_update(&self, update: RankingUpdate) -> Result<OkResult, ClientError> {
        self.call("rankings.update", update).await
    }
    pub async fn rankings_list(&self) -> Result<RankingList, ClientError> {
        self.call("rankings.list", Empty::default()).await
    }
    pub async fn rankings_get(&self, id: RankingId) -> Result<Definition, ClientError> {
        self.call("rankings.get", id).await
    }
    pub async fn rankings_snapshot(&self, id: RankingId) -> Result<Snapshot, ClientError> {
        self.call("rankings.snapshot", id).await
    }
    pub async fn rankings_delete(&self, id: RankingId) -> Result<OkResult, ClientError> {
        self.call("rankings.delete", id).await
    }
    pub async fn rankings_reset(&self, id: RankingId) -> Result<OkResult, ClientError> {
        self.call("rankings.reset", id).await
    }
    pub async fn rankings_pause(&self, id: RankingId) -> Result<OkResult, ClientError> {
        self.call("rankings.pause", id).await
    }
    pub async fn rankings_resume(&self, id: RankingId) -> Result<OkResult, ClientError> {
        self.call("rankings.resume", id).await
    }
    pub async fn rankings_contribute(
        &self,
        contribution: ExternalContribution,
    ) -> Result<ContributionOutcome, ClientError> {
        self.call("rankings.contribute", contribution).await
    }
}
