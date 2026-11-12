//! Host-only OBS URL construction and clipboard writes.
use super::*;

impl AppCore {
    /// Builds the OBS Browser Source URL and copies it to the OS clipboard.
    /// Only `{ok: true}` (or a typed, secret-free error) returns: the token
    /// never crosses into the caller. Copying needs a stored credential but
    /// deliberately not a running gateway — the URL stays valid, so OBS can
    /// hold it while the gateway (re)starts.
    pub fn widgets_copy_obs_url(&self, widget: &str) -> Result<WidgetsCopyResult, OperationError> {
        self.widgets_copy_obs_url_with_ranking(widget, None, None)
    }

    pub fn widgets_copy_obs_url_with_ranking(
        &self,
        widget: &str,
        ranking_id: Option<&str>,
        visualization: Option<&str>,
    ) -> Result<WidgetsCopyResult, OperationError> {
        self.widgets_copy_obs_url_with_display(widget, ranking_id, visualization, None, None, None)
    }

    pub fn widgets_copy_obs_url_with_display(
        &self,
        widget: &str,
        ranking_id: Option<&str>,
        visualization: Option<&str>,
        show_labels: Option<bool>,
        show_percentages: Option<bool>,
        show_legend: Option<bool>,
    ) -> Result<WidgetsCopyResult, OperationError> {
        self.widgets_copy_obs_url_with_competition(
            widget,
            ranking_id,
            visualization,
            show_labels,
            show_percentages,
            show_legend,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn widgets_copy_obs_url_with_competition(
        &self,
        widget: &str,
        ranking_id: Option<&str>,
        visualization: Option<&str>,
        show_labels: Option<bool>,
        show_percentages: Option<bool>,
        show_legend: Option<bool>,
        competition_id: Option<&str>,
    ) -> Result<WidgetsCopyResult, OperationError> {
        if let Some(competition_id) = competition_id {
            if widget != "ranking"
                || visualization != Some("roulette")
                || self.competition_get(competition_id)?.ranking_id
                    != ranking_id.unwrap_or_default()
            {
                return Err(OperationError::invalid(
                    "competition must match the roulette ranking",
                ));
            }
        }
        if widget == "ranking" {
            let id = ranking_id
                .filter(|id| !id.is_empty())
                .ok_or_else(|| OperationError::invalid("rankingId is required"))?;
            self.ranking_get(id)?;
            if visualization.is_some_and(|value| {
                !["ranking", "bar", "pie", "donut", "poll", "roulette"].contains(&value)
            }) {
                return Err(OperationError::invalid("invalid ranking visualization"));
            }
        } else if ranking_id.is_some()
            || visualization.is_some()
            || show_labels.is_some()
            || show_percentages.is_some()
            || show_legend.is_some()
        {
            return Err(OperationError::invalid(
                "ranking options require ranking widget",
            ));
        }
        if !VALID_WIDGETS.contains(&widget) {
            return Err(OperationError::invalid(
                "widget must be one of: follow, gift, chat, share, subscribe, likes, ranking",
            ));
        }
        let plugin = self.require_discovered(GATEWAY_PLUGIN_ID)?;
        let values = self
            .capabilities
            .load_plugin_settings_raw(&plugin.manifest)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let port = self.gateway_port(&values);
        let bind = self.gateway_bind(&values);
        let Some(credential) = self.gateway_widget_credential(&values) else {
            return Err(OperationError::unavailable(
                "no widget credential is stored; start the Event Gateway once, then retry",
            ));
        };
        let host = match bind {
            std::net::IpAddr::V4(_) => GATEWAY_LOOPBACK_V4.to_owned(),
            std::net::IpAddr::V6(addr) => format!("[{addr}]"),
        };
        let query = if widget == "ranking" {
            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            serializer.append_pair("rankingId", ranking_id.expect("validated ranking"));
            if let Some(visualization) = visualization {
                serializer.append_pair("visualization", visualization);
            }
            if let Some(competition_id) = competition_id {
                serializer.append_pair("competitionId", competition_id);
            }
            for (name, value) in [
                ("showLabels", show_labels),
                ("showPercentages", show_percentages),
                ("showLegend", show_legend),
            ] {
                if let Some(value) = value {
                    serializer.append_pair(name, if value { "true" } else { "false" });
                }
            }
            format!("?{}", serializer.finish())
        } else {
            String::new()
        };
        let mut url = format!(
            "http://{host}:{port}/widgets/{widget}/{query}#token={}",
            percent_encode_fragment(&credential)
        );
        let key = format!("widgets.design.{widget}");
        let state = self.app_state_get(Some(std::slice::from_ref(&key)))?;
        if let Some(design) = state.get(&key).and_then(|raw| portable_design(raw)) {
            url.push_str("&design=");
            url.push_str(&percent_encode_fragment(&design));
        }
        copy_text_to_clipboard(&url)?;
        tracing::debug!(widget, "widgets OBS URL copied to clipboard");
        Ok(WidgetsCopyResult { ok: true })
    }

    /// Copies the OBS URL of a cloud overlay. The document travels in the fragment as
    /// base64url JSON; the browser source never needs a second request to render it.
    pub fn widgets_copy_cloud_overlay_url(
        &self,
        overlay: &str,
    ) -> Result<WidgetsCopyResult, OperationError> {
        if overlay.is_empty()
            || overlay.len() > 6000
            || !overlay
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(OperationError::invalid(
                "the overlay does not fit an OBS URL; remove elements and try again",
            ));
        }
        let plugin = self.require_discovered(GATEWAY_PLUGIN_ID)?;
        let values = self
            .capabilities
            .load_plugin_settings_raw(&plugin.manifest)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let port = self.gateway_port(&values);
        let bind = self.gateway_bind(&values);
        let Some(credential) = self.gateway_widget_credential(&values) else {
            return Err(OperationError::unavailable(
                "no widget credential is stored; start the Event Gateway once, then retry",
            ));
        };
        let host = match bind {
            std::net::IpAddr::V4(_) => GATEWAY_LOOPBACK_V4.to_owned(),
            std::net::IpAddr::V6(addr) => format!("[{addr}]"),
        };
        let url = format!(
            "http://{host}:{port}/widgets/cloud-overlay/#token={}&overlay={overlay}",
            percent_encode_fragment(&credential)
        );
        copy_text_to_clipboard(&url)?;
        tracing::debug!("cloud overlay OBS URL copied to clipboard");
        Ok(WidgetsCopyResult { ok: true })
    }
}

/// Fragment-encodes a credential the same way browsers do for `#token=…`
/// (unreserved marks stay literal, everything else becomes `%XX`).
pub(super) fn percent_encode_fragment(value: &str) -> String {
    const UNRESERVED: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if UNRESERVED.contains(byte) {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 15) as usize] as char);
        }
    }
    encoded
}

#[cfg(not(target_os = "android"))]
fn copy_text_to_clipboard(text: &str) -> Result<(), OperationError> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text))
        .map_err(|_| OperationError::unavailable("could not access the OS clipboard"))
}

#[cfg(target_os = "android")]
fn copy_text_to_clipboard(_text: &str) -> Result<(), OperationError> {
    Err(OperationError::capability_unavailable(
        "OS clipboard has no Android backend",
    ))
}
