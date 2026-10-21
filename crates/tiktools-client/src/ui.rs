//! Shared read-only UI recovery operations.
use super::TikToolsClient;
use tiktools_control_api::{
    modules::ui::{FeedBackfill, FeedBackfillParams},
    ClientError,
};
pub(crate) const METHODS: &[&str] = &[FEED_BACKFILL];
const FEED_BACKFILL: &str = "ui.feed.backfill";
impl TikToolsClient {
    pub async fn ui_feed_backfill(
        &self,
        params: FeedBackfillParams,
    ) -> Result<FeedBackfill, ClientError> {
        self.call(FEED_BACKFILL, params).await
    }
}
