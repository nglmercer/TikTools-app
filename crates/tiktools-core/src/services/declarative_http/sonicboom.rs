//! SonicBoom discovery separates network access, authorization and model loading.
//! All requests keep the host's existing destination and credential policy.
use super::execution::PluginConnectionCheck;
use serde_json::json;
use tiktools_plugin_api::PluginManifest;

impl crate::AppCore {
    pub(super) async fn check_sonicboom_connection(
        &self,
        manifest: &PluginManifest,
    ) -> PluginConnectionCheck {
        let started = crate::helpers::now_millis();
        let mut result = PluginConnectionCheck {
            ok: false,
            latency_ms: 0,
            error: None,
            details: None,
            reachable: false,
            gameplay_ready: None,
            server_info: None,
        };
        let checked = async {
            let settings = self.capabilities.load_plugin_settings_raw(manifest)
                .map_err(|error| error.to_string())?;
            let scope = json!({"settings": settings});
            let info = self.fetch_declarative_json(manifest, "GET", "/api/info", Some(5000), &scope).await?;
            result.reachable = true;
            let required = info["auth"]["required"].as_bool()
                .ok_or("Incompatible SonicBoom server: authentication discovery is unavailable.")?;
            let setup = info["setup_complete"].as_bool()
                .ok_or("Incompatible SonicBoom server: setup discovery is unavailable.")?;
            let status = match info["model"]["status"].as_str() {
                Some(value @ ("idle" | "downloading" | "loading" | "ready" | "failed")) => value,
                _ => return Err("Incompatible SonicBoom server: model discovery is unavailable.".to_owned()),
            };
            let mut server = json!({
                "authRequired": required, "authorized": false, "setupComplete": setup,
                "modelStatus": status, "playback": info["features"]["playback"].as_bool().unwrap_or(false)
            });
            result.server_info = Some(server.clone());
            if !setup {
                return Err("Complete the initial setup in SonicBoom, then connect again.".to_owned());
            }
            // /ready is public and depends on model loading. This protected,
            // model-independent endpoint actually validates the saved credential.
            match self.fetch_declarative_json(manifest, "GET", "/api/tts/plugin/providers", Some(5000), &scope).await {
                Ok(_) => {
                    server["authorized"] = json!(true);
                    result.server_info = Some(server);
                    result.ok = true;
                    Ok(())
                }
                Err(error) if error.starts_with("HTTP 401") || error.starts_with("HTTP 403") => {
                    Err(if required {
                        "Sign in to SonicBoom to authorize this connection."
                    } else {
                        "SonicBoom rejected local access. Connect from the same computer using http://127.0.0.1:17842, or check its access mode."
                    }.to_owned())
                }
                Err(error) => Err(error),
            }
        }.await;
        result.latency_ms = crate::helpers::now_millis().saturating_sub(started);
        if let Err(error) = checked {
            result.error = Some(error);
        }
        result
    }
}
