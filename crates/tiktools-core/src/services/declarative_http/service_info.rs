//! Generic `service-info` health strategy for declarative HTTP plugins whose
//! server separates network access, authorization and model loading.
//!
//! The manifest declares it as:
//!
//! ```json
//! "health": { "strategy": "service-info", "path": "/api/info", "authProbePath": "/api/protected" }
//! ```
//!
//! `path` answers a public discovery document
//! (`{auth: {required}, setup_complete, model: {status}, features: {playback}}`);
//! `authProbePath` is any protected, model-independent endpoint that proves
//! the saved credential works. The legacy strategy name `sonicboom` remains an
//! alias so already-installed manifests keep working.
//!
//! All requests keep the host's existing destination and credential policy.
use super::execution::PluginConnectionCheck;
use serde_json::{json, Map, Value};
use tiktools_plugin_api::PluginManifest;

/// Strategy names handled here. `sonicboom` is the deprecated alias.
pub(super) fn is_service_info_strategy(strategy: Option<&str>) -> bool {
    matches!(strategy, Some("service-info" | "sonicboom"))
}

/// The protected probe path. Only the legacy alias falls back to the path
/// its original manifests relied on before `authProbePath` existed.
fn auth_probe_path(health: &Map<String, Value>) -> Option<&str> {
    match health.get("authProbePath").and_then(Value::as_str) {
        Some(path) => Some(path),
        None if health.get("strategy").and_then(Value::as_str) == Some("sonicboom") => {
            Some("/api/tts/plugin/providers")
        }
        None => None,
    }
}

/// True when the plugin's health probe uses the guided `service-info`
/// strategy, so the UI can offer a one-click connect and sign-in flow.
pub(crate) fn uses_guided_connection(manifest: &PluginManifest) -> bool {
    is_service_info_strategy(
        manifest
            .http
            .as_ref()
            .and_then(|http| http.pointer("/health/strategy"))
            .and_then(Value::as_str),
    )
}

impl crate::AppCore {
    pub(super) async fn check_service_info_connection(
        &self,
        manifest: &PluginManifest,
        health: &Map<String, Value>,
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
        let name = manifest.name.as_str();
        let info_path = health
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let timeout = health
            .get("timeoutMs")
            .and_then(Value::as_u64)
            .or(Some(5000));
        let checked = async {
            let settings = self.capabilities.load_plugin_settings_raw(manifest)
                .map_err(|error| error.to_string())?;
            let scope = json!({"settings": settings});
            let info = self.fetch_declarative_json(manifest, "GET", info_path, timeout, &scope).await?;
            result.reachable = true;
            let incompatible = |what: &str| format!("Incompatible {name} server: {what} discovery is unavailable.");
            let required = info["auth"]["required"].as_bool()
                .ok_or_else(|| incompatible("authentication"))?;
            let setup = info["setup_complete"].as_bool()
                .ok_or_else(|| incompatible("setup"))?;
            let status = match info["model"]["status"].as_str() {
                Some(value @ ("idle" | "downloading" | "loading" | "ready" | "failed")) => value,
                _ => return Err(incompatible("model")),
            };
            let mut server = json!({
                "authRequired": required, "authorized": false, "setupComplete": setup,
                "modelStatus": status, "playback": info["features"]["playback"].as_bool().unwrap_or(false)
            });
            result.server_info = Some(server.clone());
            if !setup {
                return Err(format!("Complete the initial setup in {name}, then connect again."));
            }
            let Some(probe) = auth_probe_path(health) else {
                // Nothing protected to verify: reachability is the answer.
                server["authorized"] = json!(!required);
                result.server_info = Some(server);
                result.ok = !required;
                return if required {
                    Err(format!("{name} requires sign-in, but the plugin declares no authProbePath."))
                } else {
                    Ok(())
                };
            };
            match self.fetch_declarative_json(manifest, "GET", probe, timeout, &scope).await {
                Ok(_) => {
                    server["authorized"] = json!(true);
                    result.server_info = Some(server);
                    result.ok = true;
                    Ok(())
                }
                Err(error) if error.starts_with("HTTP 401") || error.starts_with("HTTP 403") => {
                    Err(if required {
                        format!("Sign in to {name} to authorize this connection.")
                    } else {
                        format!("{name} rejected local access. Connect from the same computer, or check its access mode.")
                    })
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

#[cfg(test)]
mod tests {
    use super::{auth_probe_path, is_service_info_strategy};
    use serde_json::json;

    #[test]
    fn generic_strategy_and_legacy_alias_are_recognized() {
        assert!(is_service_info_strategy(Some("service-info")));
        assert!(is_service_info_strategy(Some("sonicboom")));
        assert!(!is_service_info_strategy(Some("other")));
        assert!(!is_service_info_strategy(None));
    }

    #[test]
    fn probe_path_comes_from_the_manifest() {
        let generic = json!({"strategy": "service-info", "path": "/api/info", "authProbePath": "/api/private"});
        assert_eq!(
            auth_probe_path(generic.as_object().unwrap()),
            Some("/api/private")
        );
        let undeclared = json!({"strategy": "service-info", "path": "/api/info"});
        assert_eq!(auth_probe_path(undeclared.as_object().unwrap()), None);
        let legacy = json!({"strategy": "sonicboom", "path": "/api/info"});
        assert_eq!(
            auth_probe_path(legacy.as_object().unwrap()),
            Some("/api/tts/plugin/providers")
        );
    }
}
