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

fn map_error(error: OperationError) -> ApiError {
    ApiError::from(error)
}

pub fn register(router: &mut ControlRouter) {
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
