use super::TikToolsClient;
use tiktools_control_api::modules::cloud::{
    CloudBackupIdParams, CloudBackupNameParams, CloudBackupRestoreParams,
    CloudBackupStoredRestoreParams, CloudLoginOperationParams, CloudLoginReservationResult,
    CloudLoginUrlResult, CloudStatusResult,
};
use tiktools_control_api::modules::{Empty, OkResult};
use tiktools_control_api::{ClientError, StoredBackupView};

pub(crate) const METHODS: &[&str] = &[
    CLOUD_PLAN_GET,
    CLOUD_BACKUP_CREATE,
    CLOUD_BACKUP_PREVIEW,
    CLOUD_BACKUP_RESTORE,
    CLOUD_BACKUP_LIST,
    CLOUD_BACKUP_CREATE_NAMED,
    CLOUD_BACKUP_GET,
    CLOUD_BACKUP_DELETE,
    CLOUD_BACKUP_RESTORE_STORED,
    CLOUD_STATUS,
    CLOUD_LOGIN_RESERVE,
    CLOUD_LOGIN_BEGIN,
    CLOUD_LOGIN_OPEN,
    CLOUD_LOGIN_COMPLETE,
    CLOUD_LOGIN_CANCEL,
    CLOUD_RESTORE,
    CLOUD_LOGOUT,
    CLOUD_LOGIN_CODE_SEND,
    CLOUD_LOGIN_CODE_RESEND,
    CLOUD_LOGIN_CODE_SUBMIT,
];

const CLOUD_PLAN_GET: &str = "cloud.plan.get";
const CLOUD_BACKUP_CREATE: &str = "cloud.backup.create";
const CLOUD_BACKUP_PREVIEW: &str = "cloud.backup.preview";
const CLOUD_BACKUP_RESTORE: &str = "cloud.backup.restore";
const CLOUD_BACKUP_LIST: &str = "cloud.backup.list";
const CLOUD_BACKUP_CREATE_NAMED: &str = "cloud.backup.createNamed";
const CLOUD_BACKUP_GET: &str = "cloud.backup.get";
const CLOUD_BACKUP_DELETE: &str = "cloud.backup.delete";
const CLOUD_BACKUP_RESTORE_STORED: &str = "cloud.backup.restoreStored";
const CLOUD_STATUS: &str = "cloud.status";
const CLOUD_LOGIN_RESERVE: &str = "cloud.login.reserve";
const CLOUD_LOGIN_BEGIN: &str = "cloud.login.begin";
const CLOUD_LOGIN_OPEN: &str = "cloud.login.open";
const CLOUD_LOGIN_COMPLETE: &str = "cloud.login.complete";
const CLOUD_LOGIN_CANCEL: &str = "cloud.login.cancel";
const CLOUD_RESTORE: &str = "cloud.restore";
const CLOUD_LOGOUT: &str = "cloud.logout";
const CLOUD_LOGIN_CODE_SEND: &str = "cloud.login.code.send";
const CLOUD_LOGIN_CODE_RESEND: &str = "cloud.login.code.resend";
const CLOUD_LOGIN_CODE_SUBMIT: &str = "cloud.login.code.submit";

impl TikToolsClient {
    pub async fn cloud_backup_list(&self) -> Result<Vec<StoredBackupView>, ClientError> {
        self.call(CLOUD_BACKUP_LIST, Empty::default()).await
    }

    pub async fn cloud_backup_create_named(
        &self,
        params: CloudBackupNameParams,
    ) -> Result<StoredBackupView, ClientError> {
        self.call(CLOUD_BACKUP_CREATE_NAMED, params).await
    }

    pub async fn cloud_backup_get(&self, id: String) -> Result<serde_json::Value, ClientError> {
        self.call(CLOUD_BACKUP_GET, CloudBackupIdParams { id })
            .await
    }

    pub async fn cloud_backup_delete(&self, id: String) -> Result<serde_json::Value, ClientError> {
        self.call(CLOUD_BACKUP_DELETE, CloudBackupIdParams { id })
            .await
    }

    pub async fn cloud_backup_restore_stored(
        &self,
        params: CloudBackupStoredRestoreParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(CLOUD_BACKUP_RESTORE_STORED, params).await
    }

    pub async fn cloud_backup_create(&self) -> Result<serde_json::Value, ClientError> {
        self.call(CLOUD_BACKUP_CREATE, Empty::default()).await
    }

    pub async fn cloud_backup_preview(
        &self,
        params: CloudBackupRestoreParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(CLOUD_BACKUP_PREVIEW, params).await
    }

    pub async fn cloud_backup_restore(
        &self,
        params: CloudBackupRestoreParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(CLOUD_BACKUP_RESTORE, params).await
    }

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
