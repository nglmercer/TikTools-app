//! Shared read-only UI recovery operations.
use super::TikToolsClient;
use tiktools_control_api::{
    modules::ui::{BrowserFeedParams, CreatorFeedParams, FeedBackfill, FeedBackfillParams},
    ClientError,
};
pub(crate) const METHODS: &[&str] = &[FEED_BACKFILL, "ui.feed.browser", "ui.feed.creator"];
const FEED_BACKFILL: &str = "ui.feed.backfill";
impl TikToolsClient {
    pub async fn ui_feed_backfill(
        &self,
        params: FeedBackfillParams,
    ) -> Result<FeedBackfill, ClientError> {
        self.call(FEED_BACKFILL, params).await
    }
}

impl TikToolsClient {
    pub async fn ui_feed_browser(
        &self,
        params: BrowserFeedParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call("ui.feed.browser", params).await
    }

    pub async fn ui_feed_creator(
        &self,
        params: CreatorFeedParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call("ui.feed.creator", params).await
    }
}
