//! Headless Cloud account operations. Everything crossing this boundary
//! is nonsecret ([`AccountStatus`]); tokens stay in native code.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::OperationError;
use crate::{
    cloud_account::{AuthError, CloudLoginBegin},
    AppCore,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudLoginUrl {
    pub browser_url: String,
}

impl From<CloudLoginBegin> for CloudLoginUrl {
    fn from(begin: CloudLoginBegin) -> Self {
        Self {
            browser_url: begin.browser_url,
        }
    }
}

fn auth_error(error: AuthError) -> OperationError {
    let reason = error.code();
    let message = error.to_string();
    let data = serde_json::json!({ "reason": reason });
    let mut operation = match &error {
        AuthError::Unconfigured | AuthError::NoSavedSession | AuthError::StorageUnavailable => {
            OperationError::unavailable(message)
        }
        AuthError::AlreadyInProgress
        | AuthError::NoLoginInProgress
        | AuthError::AlreadySignedIn => OperationError::conflict(message),
        AuthError::Cancelled | AuthError::Expired | AuthError::Failed(_) => {
            OperationError::internal(message)
        }
    };
    operation.data = Some(data);
    operation
}

impl AppCore {
    fn cloud(&self) -> &crate::cloud_account::CloudAccount {
        &self.cloud
    }

    pub async fn cloud_status(&self) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        Ok(self.cloud().auth().status().await)
    }

    pub async fn cloud_begin_login(&self) -> Result<CloudLoginUrl, OperationError> {
        self.cloud_begin_login_for(None).await
    }

    pub async fn cloud_reserve_login(&self) -> Result<String, OperationError> {
        self.cloud()
            .auth()
            .reserve_login()
            .await
            .map_err(auth_error)
    }

    pub async fn cloud_begin_login_for(
        &self,
        operation: Option<&str>,
    ) -> Result<CloudLoginUrl, OperationError> {
        self.cloud()
            .auth()
            .begin_login_for(operation)
            .await
            .map(|begin| CloudLoginUrl {
                browser_url: begin.browser_url,
            })
            .map_err(auth_error)
    }

    pub async fn cloud_open_browser(&self) -> Result<(), OperationError> {
        self.cloud_open_browser_for(None).await
    }

    pub async fn cloud_open_browser_for(
        &self,
        operation: Option<&str>,
    ) -> Result<(), OperationError> {
        self.cloud()
            .auth()
            .open_browser_for(operation)
            .await
            .map_err(auth_error)
    }

    pub async fn cloud_complete_login(
        &self,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud_complete_login_for(None).await
    }

    pub async fn cloud_complete_login_for(
        &self,
        operation: Option<&str>,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud()
            .auth()
            .complete_login_for(operation)
            .await
            .map_err(auth_error)
    }

    pub async fn cloud_cancel_login(
        &self,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud_cancel_login_for(None).await
    }

    pub async fn cloud_cancel_login_for(
        &self,
        operation: Option<&str>,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud()
            .auth()
            .cancel_login_for(operation)
            .await
            .map_err(auth_error)
    }

    pub async fn cloud_restore(
        &self,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud().auth().restore().await.map_err(auth_error)
    }

    pub async fn cloud_logout(&self) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud().auth().logout().await.map_err(auth_error)
    }
}
