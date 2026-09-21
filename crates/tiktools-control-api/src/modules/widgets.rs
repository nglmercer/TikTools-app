use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use tiktools_core::control::{WidgetsCopyResult, WidgetsState, WidgetsStatusResult};
use tiktools_core::AppCore;

use crate::{error::ApiError, modules::blocking_task, router::ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WidgetsCopyParams {
    pub widget: String,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<crate::modules::Empty, WidgetsStatusResult, _, _>(
        "widgets.status",
        "OBS widgets readiness: gateway state, loopback port, and a secret-free hint",
        false,
        |core: Arc<AppCore>, _params: crate::modules::Empty| async move {
            core.widgets_status().await.map_err(ApiError::from)
        },
    );
    router.register_typed_full::<WidgetsCopyParams, WidgetsCopyResult, _, _>(
        "widgets.copyObsUrl",
        "Copies one OBS Browser Source URL (with its widget credential) to the OS clipboard; the URL is never returned",
        true,
        false,
        true,
        |core: Arc<AppCore>, params: WidgetsCopyParams| async move {
            // The clipboard round-trips with the OS session; keep it off
            // the Tokio workers like other blocking host calls.
            blocking_task("widgets clipboard", move || {
                core.widgets_copy_obs_url(&params.widget)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
}
