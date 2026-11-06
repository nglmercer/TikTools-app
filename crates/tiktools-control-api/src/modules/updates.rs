//! Application updater RPC. This is separate from plugin package management.
use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::AppCore;

use crate::{error::ApiError, router::ControlRouter};

#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoUpdateParams {}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
}

#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckParams {
    #[serde(default)]
    pub channel: UpdateChannel,
}

#[cfg(feature = "app-update")]
fn updater(core: &AppCore) -> Result<Arc<tiktools_core::app_update::AppUpdater>, ApiError> {
    core.app_updater().ok_or_else(|| {
        ApiError::capability_unavailable("desktop application updater is unavailable in this host")
    })
}

#[cfg(feature = "app-update")]
fn error(error: tiktools_core::app_update::ServiceError) -> ApiError {
    use tiktools_core::app_update::ServiceError;
    match error {
        ServiceError::Unavailable => ApiError::capability_unavailable(error.to_string()),
        ServiceError::InvalidState => ApiError::new("invalid_update_state", error.to_string()),
    }
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<NoUpdateParams, Value, _, _>(
        "app.update.status",
        "Application updater status",
        false,
        |core: Arc<AppCore>, _: NoUpdateParams| async move {
            #[cfg(feature = "app-update")]
            {
                let updater = updater(&core)?;
                let mut value = serde_json::to_value(updater.status())
                    .map_err(|_| ApiError::internal("could not serialize updater status"))?;
                value["checkSupported"] = Value::Bool(updater.configured());
                Ok(value)
            }
            #[cfg(not(feature = "app-update"))]
            {
                let _ = core;
                Err(ApiError::capability_unavailable(
                    "desktop application updater is unavailable in this host",
                ))
            }
        },
    );
    router.register_typed::<CheckParams, Value, _, _>(
        "app.update.check",
        "Check signed application update metadata",
        false,
        |core: Arc<AppCore>, params: CheckParams| async move {
            #[cfg(feature = "app-update")]
            {
                use tiktools_core::app_update::Channel;
                let channel = match params.channel {
                    UpdateChannel::Stable => Channel::Stable,
                    UpdateChannel::Beta => Channel::Beta,
                };
                let status = updater(&core)?.check(channel).map_err(error)?;
                serde_json::to_value(status)
                    .map_err(|_| ApiError::internal("could not serialize updater status"))
            }
            #[cfg(not(feature = "app-update"))]
            {
                let _ = (core, params);
                Err(ApiError::capability_unavailable(
                    "desktop application updater is unavailable in this host",
                ))
            }
        },
    );
    router.register_typed::<NoUpdateParams, Value, _, _>(
        "app.update.download",
        "Download and verify the selected application update",
        true,
        |core: Arc<AppCore>, _: NoUpdateParams| async move {
            #[cfg(feature = "app-update")]
            {
                let status = updater(&core)?.download().map_err(error)?;
                serde_json::to_value(status)
                    .map_err(|_| ApiError::internal("could not serialize updater status"))
            }
            #[cfg(not(feature = "app-update"))]
            {
                let _ = core;
                Err(ApiError::capability_unavailable(
                    "desktop application updater is unavailable in this host",
                ))
            }
        },
    );
    router.register_typed::<NoUpdateParams, Value, _, _>(
        "app.update.install",
        "Install requires trusted native desktop confirmation",
        true,
        |_core: Arc<AppCore>, _: NoUpdateParams| async move {
            Err::<Value, ApiError>(ApiError::capability_unavailable(
                "application installation requires native confirmation and is not available yet",
            ))
        },
    );
}
