//! Cloud account exposed through the shared control plane.
//!
//! Everything here is nonsecret: the account id, the origin, the login
//! phase, and error text. Tokens never leave native code.

use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::{
    cloud_account::AccountStatus,
    control::{CloudLoginUrl, OperationError},
    AppCore,
};

use super::{Empty, OkResult};
use crate::ApiError;
use crate::ControlRouter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudStatusResult {
    #[serde(flatten)]
    pub status: AccountStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudLoginUrlResult {
    pub browser_url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudLoginOperationParams {
    #[serde(default)]
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudLoginReservationResult {
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudLoginCodeParams {
    #[serde(default)]
    pub operation_id: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudLoginCodeSubmitParams {
    #[serde(default)]
    pub operation_id: Option<String>,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudBackupRestoreParams {
    pub manifest: serde_json::Value,
    #[serde(default = "default_backup_mode")]
    pub mode: String,
    #[serde(default)]
    pub dry_run: bool,
}

fn default_backup_mode() -> String {
    "replace".to_owned()
}

fn map_error(error: OperationError) -> ApiError {
    ApiError::from(error)
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, tiktools_core::control::AccountPlanView, _, _>(
        "cloud.plan.get",
        "Shows the Cloud plan, its ceilings and the current storage usage",
        false,
        |core: Arc<AppCore>, _params: Empty| async move { core.cloud_plan().await.map_err(map_error) },
    );
    router.register_typed::<Empty, serde_json::Value, _, _>(
        "cloud.backup.create",
        "Creates a portable, versioned backup of all Cloud-owned data",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.cloud_backup().await.map_err(map_error)
        },
    );
    router.register_typed::<CloudBackupRestoreParams, serde_json::Value, _, _>(
        "cloud.backup.preview",
        "Summarizes what a backup restore would import without changing anything",
        false,
        |core: Arc<AppCore>, params: CloudBackupRestoreParams| async move {
            core.cloud_backup_preview(params.manifest)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed_full::<CloudBackupRestoreParams, serde_json::Value, _, _>(
        "cloud.backup.restore",
        "Restores a backup manifest on this device",
        true,
        true,
        true,
        |core: Arc<AppCore>, params: CloudBackupRestoreParams| async move {
            core.cloud_backup_restore(params.manifest)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<Empty, CloudStatusResult, _, _>(
        "cloud.status",
        "Shows the Cloud account state without exposing any credentials",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.cloud_status()
                .await
                .map(|status| CloudStatusResult { status })
                .map_err(map_error)
        },
    );
    router.register_typed::<Empty, CloudLoginReservationResult, _, _>(
        "cloud.login.reserve",
        "Reserves a correlated login operation without starting a callback",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.cloud_reserve_login()
                .await
                .map(|operation_id| CloudLoginReservationResult { operation_id })
                .map_err(map_error)
        },
    );
    router.register_typed::<CloudLoginCodeParams, tiktools_cloud_auth::LoginCodeStatus, _, _>(
        "cloud.login.code.send",
        "Sends a short-lived login code to connected devices (OAuth if no account)",
        true,
        |core: Arc<AppCore>, params: CloudLoginCodeParams| async move {
            core.cloud_start_login_code(params.operation_id.as_deref(), &params.email)
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<CloudLoginOperationParams, tiktools_cloud_auth::LoginCodeStatus, _, _>(
        "cloud.login.code.resend",
        "Relays the login code to connected devices again",
        true,
        |core: Arc<AppCore>, params: CloudLoginOperationParams| async move {
            core.cloud_relay_login_code(params.operation_id.as_deref())
                .await
                .map_err(map_error)
        },
    );
    router.register_typed::<CloudLoginCodeSubmitParams, CloudStatusResult, _, _>(
        "cloud.login.code.submit",
        "Redeems a relayed login code without exposing any credential",
        true,
        |core: Arc<AppCore>, params: CloudLoginCodeSubmitParams| async move {
            core.cloud_redeem_login_code(params.operation_id.as_deref(), &params.code)
                .await
                .map(|status| CloudStatusResult { status })
                .map_err(map_error)
        },
    );
    router.register_typed::<CloudLoginOperationParams, CloudLoginUrlResult, _, _>(
        "cloud.login.begin",
        "Starts the system-browser login and returns the URL to open",
        true,
        |core: Arc<AppCore>, params: CloudLoginOperationParams| async move {
            core.cloud_begin_login_for(params.operation_id.as_deref())
                .await
                .map(|url: CloudLoginUrl| CloudLoginUrlResult {
                    browser_url: url.browser_url,
                })
                .map_err(map_error)
        },
    );
    router.register_typed::<CloudLoginOperationParams, OkResult, _, _>(
        "cloud.login.open",
        "Opens the pending login URL in the system browser",
        true,
        |core: Arc<AppCore>, params: CloudLoginOperationParams| async move {
            core.cloud_open_browser_for(params.operation_id.as_deref())
                .await
                .map(|()| OkResult::ok())
                .map_err(map_error)
        },
    );
    router.register_typed::<CloudLoginOperationParams, CloudStatusResult, _, _>(
        "cloud.login.complete",
        "Waits for the browser callback, redeems it, and signs in",
        true,
        |core: Arc<AppCore>, params: CloudLoginOperationParams| async move {
            core.cloud_complete_login_for(params.operation_id.as_deref())
                .await
                .map(|status| CloudStatusResult { status })
                .map_err(map_error)
        },
    );
    router.register_typed::<CloudLoginOperationParams, CloudStatusResult, _, _>(
        "cloud.login.cancel",
        "Aborts an in-progress login and drops its secrets",
        true,
        |core: Arc<AppCore>, params: CloudLoginOperationParams| async move {
            core.cloud_cancel_login_for(params.operation_id.as_deref())
                .await
                .map(|status| CloudStatusResult { status })
                .map_err(map_error)
        },
    );
    router.register_typed::<Empty, CloudStatusResult, _, _>(
        "cloud.restore",
        "Reconnects from the OS credential store after a restart",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.cloud_restore()
                .await
                .map(|status| CloudStatusResult { status })
                .map_err(map_error)
        },
    );
    router.register_typed::<Empty, CloudStatusResult, _, _>(
        "cloud.logout",
        "Revokes the server session and wipes local credentials",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.cloud_logout()
                .await
                .map(|status| CloudStatusResult { status })
                .map_err(map_error)
        },
    );
}
