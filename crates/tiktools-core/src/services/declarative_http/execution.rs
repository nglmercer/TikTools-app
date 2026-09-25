//! Declarative action execution, fetching, and connection probes.

use super::{
    build_declarative_request, declarative_auth_label, declarative_request_line,
    redact_endpoint_secrets, with_auth_hint, DeclarativeBuild,
};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use tiktools_plugin_api::PluginManifest;

impl crate::AppCore {
    /// Executes a v3 action descriptor's `http` block through the automation
    /// HTTP engine. Test runs describe the request without sending it.
    pub(crate) async fn execute_declarative_action(
        self: &Arc<Self>,
        plugin: &tiktools_plugin_loader::DiscoveredPlugin,
        action_http: &Value,
        action: &Value,
        event: &Value,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        let manifest = &plugin.manifest;
        let block = action_http
            .as_object()
            .ok_or_else(|| format!("Plugin `{}` action has no HTTP block.", manifest.id))?;
        let http = manifest
            .http
            .as_ref()
            .ok_or_else(|| format!("Plugin `{}` declares no HTTP integration.", manifest.id))?;
        let settings = self
            .capabilities
            .load_plugin_settings_raw(manifest)
            .map_err(|error| error.to_string())?;
        let config = action
            .get("config")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let scope = json!({"event": event, "settings": settings, "config": config});
        let method = block
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("POST");
        let path = block
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Plugin `{}` action has no HTTP path.", manifest.id))?;
        let endpoint = build_declarative_request(&DeclarativeBuild {
            manifest,
            http,
            method,
            path_template: path,
            extra_headers: block.get("headers").and_then(Value::as_object),
            body_template: block.get("body").and_then(Value::as_str),
            timeout_override_ms: block.get("timeoutMs").and_then(Value::as_u64),
            scope: &scope,
            broker: &self.capabilities,
        })?;
        if test {
            let host = url::Url::parse(&endpoint.url)
                .ok()
                .and_then(|url| url.host_str().map(ToOwned::to_owned))
                .unwrap_or_default();
            return Ok(redact_endpoint_secrets(
                &format!("would {} request to {host}", endpoint.method),
                &endpoint.secrets,
            ));
        }
        let mut fetch = Map::new();
        fetch.insert("url".to_owned(), Value::String(endpoint.url.clone()));
        fetch.insert("method".to_owned(), Value::String(endpoint.method.clone()));
        fetch.insert(
            "headers".to_owned(),
            Value::Object(
                endpoint
                    .headers
                    .iter()
                    .cloned()
                    .map(|(name, value)| (name, Value::String(value)))
                    .collect(),
            ),
        );
        if let Some(body) = endpoint.body.as_deref() {
            fetch.insert("body".to_owned(), Value::String(body.to_owned()));
        }
        fetch.insert(
            "timeoutMs".to_owned(),
            Value::Number(endpoint.timeout_ms.into()),
        );
        fetch.insert(
            "allowPrivateNetwork".to_owned(),
            Value::Bool(endpoint.allow_private_network),
        );
        if let Some(emit) = block.get("emitResponseAs") {
            fetch.insert("emitResponseAs".to_owned(), emit.clone());
        }
        let host = url::Url::parse(&endpoint.url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
            .unwrap_or_default();
        let allowed = vec![host];
        let globals = self.automation_globals();
        // One request line per call so plugin/TTS logs show exactly what was
        // sent: method, path, and whether credentials were attached. The
        // token value itself never appears here.
        let (auth_type, token_setting) = declarative_auth_label(http);
        if logs.len() < 40 {
            logs.push(redact_endpoint_secrets(
                &declarative_request_line(&endpoint, &auth_type, &token_setting),
                &endpoint.secrets,
            ));
        }
        let summary = self
            .execute_http_action(&fetch, event, &globals, logs, Some(&allowed), false)
            .await
            .map_err(|error| {
                with_auth_hint(
                    &redact_endpoint_secrets(&error, &endpoint.secrets),
                    &manifest.id,
                    &auth_type,
                    &token_setting,
                    !endpoint.secrets.is_empty(),
                )
            })?;
        for line in logs.iter_mut() {
            *line = redact_endpoint_secrets(line, &endpoint.secrets);
        }
        Ok(redact_endpoint_secrets(&summary, &endpoint.secrets))
    }

    /// Fetches one declarative JSON document (option source or health probe)
    /// through the hardened sender. Only 2xx responses yield a body.
    pub(crate) async fn fetch_declarative_json(
        &self,
        manifest: &PluginManifest,
        method: &str,
        path: &str,
        timeout_override_ms: Option<u64>,
        scope: &Value,
    ) -> Result<Value, String> {
        let http = manifest
            .http
            .as_ref()
            .ok_or_else(|| format!("Plugin `{}` declares no HTTP integration.", manifest.id))?;
        let endpoint = build_declarative_request(&DeclarativeBuild {
            manifest,
            http,
            method,
            path_template: path,
            extra_headers: None,
            body_template: None,
            timeout_override_ms,
            scope,
            broker: &self.capabilities,
        });
        let endpoint = endpoint?;
        let url = url::Url::parse(&endpoint.url)
            .map_err(|_| format!("Plugin `{}` rendered an invalid request URL.", manifest.id))?;
        let host = url
            .host_str()
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| format!("Plugin `{}` has a server URL with no host.", manifest.id))?;
        let request = crate::automation_runtime::HardenedHttpRequest {
            method: endpoint.method.clone(),
            url: endpoint.url.clone(),
            headers: endpoint.headers.clone(),
            body: endpoint.body.clone(),
            timeout_ms: endpoint.timeout_ms,
        };
        #[cfg(feature = "http")]
        {
            let allowed = vec![host.clone()];
            if let Err(error) = crate::helpers::validate_http_url(
                &url,
                &host,
                Some(&allowed),
                endpoint.allow_private_network,
            )
            .await
            {
                return Err(redact_endpoint_secrets(&error, &endpoint.secrets));
            }
        }
        let sent = self
            .send_hardened_http_request(&request, &host)
            .await
            .map_err(|error| redact_endpoint_secrets(&error, &endpoint.secrets))?;
        if !(200..300).contains(&sent.status) {
            return Err(redact_endpoint_secrets(
                &format!("HTTP {} from {host}", sent.status),
                &endpoint.secrets,
            ));
        }
        Ok(sent.body)
    }
}

/// Value-returning health-probe outcome shared by the WebView emit path and
/// the headless control API.
#[derive(Debug, Clone)]
pub(crate) struct PluginConnectionCheck {
    pub(crate) ok: bool,
    pub(crate) latency_ms: u64,
    pub(crate) error: Option<String>,
}

impl crate::AppCore {
    /// Checks a plugin's declared health endpoint. Only installed, enabled,
    /// available schema v3 plugins with an `http.health` block can be
    /// probed; anything else yields an explanatory error, never a fetch.
    pub(crate) async fn check_plugin_connection(&self, id: &str) -> PluginConnectionCheck {
        let started = crate::helpers::now_millis();
        let failed = |error: String| PluginConnectionCheck {
            ok: false,
            latency_ms: crate::helpers::now_millis().saturating_sub(started),
            error: Some(error),
        };
        let plugin = self.plugins.get(id);
        let Some(plugin) = plugin else {
            return failed(format!("Plugin `{id}` is not installed."));
        };
        if !self.plugin_ready(&plugin.manifest.id) {
            return failed(format!(
                "Plugin `{id}` is not installed, enabled, or available."
            ));
        }
        let health = plugin
            .manifest
            .http
            .as_ref()
            .and_then(|http| http.get("health"))
            .and_then(Value::as_object);
        let Some(health) = health else {
            return failed(format!("Plugin `{id}` declares no health endpoint."));
        };
        let path = health
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let settings = match self.capabilities.load_plugin_settings_raw(&plugin.manifest) {
            Ok(settings) => settings,
            Err(fetch) => return failed(fetch.to_string()),
        };
        let scope = json!({"settings": settings});
        if let Err(fetch) = self
            .fetch_declarative_json(
                &plugin.manifest,
                "GET",
                path,
                health.get("timeoutMs").and_then(Value::as_u64),
                &scope,
            )
            .await
        {
            return failed(fetch);
        }
        PluginConnectionCheck {
            ok: true,
            latency_ms: crate::helpers::now_millis().saturating_sub(started),
            error: None,
        }
    }

    /// Probes a plugin's declared health endpoint and emits the result.
    /// Legacy adapter over the `plugins.health` control operation.
    pub(crate) async fn probe_plugin_connection(self: &Arc<Self>, id: &str) {
        match self.plugin_connection_check(id).await {
            Ok(check) => self.emit_connection_latency(
                &check.plugin_id,
                check.ok,
                check.latency_ms,
                check.error,
            ),
            Err(error) => {
                self.emit_connection_latency(id, false, 0, Some(error.message().to_owned()));
            }
        }
    }

    fn emit_connection_latency(&self, id: &str, ok: bool, latency_ms: u64, error: Option<String>) {
        self.emit(crate::ipc::messages::HostMessage::PluginConnectionResult {
            id: id.to_owned(),
            ok,
            latency_ms,
            error,
        });
    }
}
