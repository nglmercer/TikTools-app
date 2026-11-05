use super::TikToolsClient;
use tiktools_control_api::modules::sync::{SyncNowResult, SyncResolveParams, SyncStatusResult};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    SYNC_STATUS,
    SYNC_ENABLE,
    SYNC_DISABLE,
    SYNC_NOW,
    SYNC_RESOLVE,
];

const SYNC_STATUS: &str = "sync.status";
const SYNC_ENABLE: &str = "sync.enable";
const SYNC_DISABLE: &str = "sync.disable";
const SYNC_NOW: &str = "sync.now";
const SYNC_RESOLVE: &str = "sync.resolve";

impl TikToolsClient {
    pub async fn sync_status(&self) -> Result<SyncStatusResult, ClientError> {
        self.call(SYNC_STATUS, Empty::default()).await
    }

    pub async fn sync_enable(&self) -> Result<SyncStatusResult, ClientError> {
        self.call(SYNC_ENABLE, Empty::default()).await
    }

    pub async fn sync_disable(&self) -> Result<SyncStatusResult, ClientError> {
        self.call(SYNC_DISABLE, Empty::default()).await
    }

    pub async fn sync_now(&self) -> Result<SyncNowResult, ClientError> {
        self.call(SYNC_NOW, Empty::default()).await
    }

    pub async fn sync_resolve(
        &self,
        params: SyncResolveParams,
    ) -> Result<SyncStatusResult, ClientError> {
        self.call(SYNC_RESOLVE, params).await
    }
}
