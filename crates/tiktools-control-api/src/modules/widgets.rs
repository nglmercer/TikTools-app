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
    #[serde(default)]
    pub ranking_id: Option<String>,
    #[serde(default)]
    pub visualization: Option<String>,
    #[serde(default)]
    pub competition_id: Option<String>,
    #[serde(default)]
    pub show_labels: Option<bool>,
    #[serde(default)]
    pub show_percentages: Option<bool>,
    #[serde(default)]
    pub show_legend: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WidgetsCopyCloudOverlayParams {
    /// base64url JSON of the cloud overlay document, at most 6000 characters.
    pub overlay: String,
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
    router.register_typed_full::<WidgetsCopyCloudOverlayParams, WidgetsCopyResult, _, _>(
        "widgets.copyCloudOverlayUrl",
        "Copies the OBS Browser Source URL of a cloud overlay, with its widget credential, to the OS clipboard",
        true,
        false,
        true,
        |core: Arc<AppCore>, params: WidgetsCopyCloudOverlayParams| async move {
            blocking_task("cloud overlay clipboard", move || core.widgets_copy_cloud_overlay_url(&params.overlay))
                .await?
                .map_err(ApiError::from)
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
                core.widgets_copy_obs_url_with_competition(&params.widget, params.ranking_id.as_deref(), params.visualization.as_deref(), params.show_labels, params.show_percentages, params.show_legend, params.competition_id.as_deref())
            })
            .await?
            .map_err(ApiError::from)
        },
    );
}

#[cfg(test)]
mod tests {
    use super::WidgetsCopyParams;
    use serde_json::json;

    #[test]
    fn reveal_widget_parameters_are_typed_and_backward_compatible() {
        let legacy: WidgetsCopyParams = serde_json::from_value(json!({"widget":"likes"})).unwrap();
        assert!(legacy.competition_id.is_none());
        let params: WidgetsCopyParams = serde_json::from_value(json!({"widget":"ranking", "rankingId":"rank", "competitionId":"poll", "visualization":"roulette", "showLabels":false, "showPercentages":true, "showLegend":false})).unwrap();
        assert_eq!(params.competition_id.as_deref(), Some("poll"));
        assert_eq!(params.show_labels, Some(false));
        assert_eq!(
            serde_json::to_value(params).unwrap()["showPercentages"],
            true
        );
        assert!(serde_json::from_value::<WidgetsCopyParams>(
            json!({"widget":"ranking", "showLegend":"false"})
        )
        .is_err());
    }
}
