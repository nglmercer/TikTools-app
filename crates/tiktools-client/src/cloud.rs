use super::TikToolsClient;
use tiktools_control_api::modules::cloud::{
    CloudLoginOperationParams, CloudLoginReservationResult, CloudLoginUrlResult, CloudStatusResult,
};
use tiktools_control_api::modules::{Empty, OkResult};
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    CLOUD_STATUS,
    CLOUD_LOGIN_RESERVE,
    CLOUD_LOGIN_BEGIN,
    CLOUD_LOGIN_OPEN,
    CLOUD_LOGIN_COMPLETE,
    CLOUD_LOGIN_CANCEL,
    CLOUD_RESTORE,
    CLOUD_LOGOUT,
];

const CLOUD_STATUS: &str = "cloud.status";
const CLOUD_LOGIN_RESERVE: &str = "cloud.login.reserve";
const CLOUD_LOGIN_BEGIN: &str = "cloud.login.begin";
const CLOUD_LOGIN_OPEN: &str = "cloud.login.open";
const CLOUD_LOGIN_COMPLETE: &str = "cloud.login.complete";
const CLOUD_LOGIN_CANCEL: &str = "cloud.login.cancel";
const CLOUD_RESTORE: &str = "cloud.restore";
const CLOUD_LOGOUT: &str = "cloud.logout";

impl TikToolsClient {
    pub async fn cloud_status(&self) -> Result<CloudStatusResult, ClientError> {
        self.call(CLOUD_STATUS, Empty::default()).await
    }

    pub async fn cloud_login_begin(&self) -> Result<CloudLoginUrlResult, ClientError> {
        self.call(CLOUD_LOGIN_BEGIN, Empty::default()).await
    }

    pub async fn cloud_login_reserve(&self) -> Result<CloudLoginReservationResult, ClientError> {
        self.call(CLOUD_LOGIN_RESERVE, Empty::default()).await
    }

    pub async fn cloud_login_begin_for(
        &self,
        operation_id: String,
    ) -> Result<CloudLoginUrlResult, ClientError> {
        self.call(
            CLOUD_LOGIN_BEGIN,
            CloudLoginOperationParams {
                operation_id: Some(operation_id),
            },
        )
        .await
    }

    pub async fn cloud_login_open(&self) -> Result<OkResult, ClientError> {
        self.call(CLOUD_LOGIN_OPEN, Empty::default()).await
    }

    pub async fn cloud_login_open_for(
        &self,
        operation_id: String,
    ) -> Result<OkResult, ClientError> {
        self.call(
            CLOUD_LOGIN_OPEN,
            CloudLoginOperationParams {
                operation_id: Some(operation_id),
            },
        )
        .await
    }

    pub async fn cloud_login_complete(&self) -> Result<CloudStatusResult, ClientError> {
        self.call(CLOUD_LOGIN_COMPLETE, Empty::default()).await
    }

    pub async fn cloud_login_complete_for(
        &self,
        operation_id: String,
    ) -> Result<CloudStatusResult, ClientError> {
        self.call(
            CLOUD_LOGIN_COMPLETE,
            CloudLoginOperationParams {
                operation_id: Some(operation_id),
            },
        )
        .await
    }

    pub async fn cloud_login_cancel(&self) -> Result<CloudStatusResult, ClientError> {
        self.call(CLOUD_LOGIN_CANCEL, Empty::default()).await
    }

    pub async fn cloud_login_cancel_for(
        &self,
        operation_id: String,
    ) -> Result<CloudStatusResult, ClientError> {
        self.call(
            CLOUD_LOGIN_CANCEL,
            CloudLoginOperationParams {
                operation_id: Some(operation_id),
            },
        )
        .await
    }

    pub async fn cloud_restore(&self) -> Result<CloudStatusResult, ClientError> {
        self.call(CLOUD_RESTORE, Empty::default()).await
    }

    pub async fn cloud_logout(&self) -> Result<CloudStatusResult, ClientError> {
        self.call(CLOUD_LOGOUT, Empty::default()).await
    }
}
