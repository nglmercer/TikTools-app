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
        // Fail fast under bursts: no unbounded gameplay queue. Opt-in actions
        // share a conservative process-wide budget below REPOapi's 20 rps.
        static IN_FLIGHT: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
        static RATE: std::sync::Mutex<std::collections::VecDeque<std::time::Instant>> =
            std::sync::Mutex::new(std::collections::VecDeque::new());
        let guarded = action_http
            .get("idempotency")
            .and_then(|v| v.get("enabled"))
            .and_then(Value::as_bool)
            == Some(true);
        let _permit = if guarded && !test {
            let permit = IN_FLIGHT
                .try_acquire()
                .map_err(|_| "Gameplay busy: concurrent request limit reached")?;
            let mut rate = RATE.lock().map_err(|_| "Request budget unavailable")?;
            while rate
                .front()
                .is_some_and(|started| started.elapsed() >= std::time::Duration::from_secs(1))
            {
                rate.pop_front();
            }
            if rate.len() >= 16 {
                return Err("Gameplay rate limit reached; command was not sent".to_owned());
            }
            rate.push_back(std::time::Instant::now());
            Some(permit)
        } else {
            None
        };
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
        let mut config = action
            .get("config")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        apply_settings_defaults(&mut config, &settings, block.get("defaultsFromSettings"));
        let scope = json!({"event": event, "settings": settings, "config": config});
        let method = block
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("POST");
        let path = block
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Plugin `{}` action has no HTTP path.", manifest.id))?;
        let mut endpoint = build_declarative_request(&DeclarativeBuild {
            manifest,
            http,
            method,
            path_template: path,
            extra_headers: block.get("headers").and_then(Value::as_object),
            body_template: block.get("body").and_then(Value::as_str),
            timeout_override_ms: block.get("timeoutMs").and_then(Value::as_u64),
            timeout_by_field: block.get("timeoutByField").and_then(Value::as_object),
            scope: &scope,
            broker: &self.capabilities,
        })?;
        if let Some(template) = block.get("bodyJson") {
            let mut template = template.clone();
            if let Some(spec) = block.get("bodyVariants") {
                let field = spec["field"].as_str().ok_or("Missing variant field")?;
                let selected = config
                    .get(field)
                    .filter(|v| v.as_str() != Some(""))
                    .or_else(|| settings.get(field))
                    .and_then(Value::as_str)
                    .ok_or("Select a request variant")?;
                let variant = spec["values"]
                    .get(selected)
                    .and_then(Value::as_object)
                    .ok_or("Invalid request variant")?;
                template
                    .as_object_mut()
                    .ok_or("bodyJson must be an object")?
                    .extend(variant.clone());
            }
            endpoint.body = Some(super::template::render_json(&template, &scope)?.to_string());
        }
        if block
            .get("idempotency")
            .and_then(|v| v.get("enabled"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            let header = block
                .get("idempotency")
                .and_then(|v| v.get("header"))
                .and_then(Value::as_str)
                .unwrap_or("Idempotency-Key");
            let key = execution_key()?;
            endpoint
                .headers
                .retain(|(name, _)| !name.eq_ignore_ascii_case(header));
            endpoint.headers.push((header.to_owned(), key));
        }
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
            fetch.insert(
                "body".to_owned(),
                if block.contains_key("bodyJson") {
                    serde_json::from_str(body).map_err(|_| "Invalid structured body")?
                } else {
                    Value::String(body.to_owned())
                },
            );
        }
        fetch.insert(
            "timeoutMs".to_owned(),
            Value::Number(endpoint.timeout_ms.into()),
        );
        // Plugin actions may legitimately exceed the generic automation-node
        // clamp; the executor honours this internal ceiling instead of its
        // 120s default. It is set only by the declarative path, never by user
        // workflow config.
        fetch.insert(
            "timeoutCeilingMs".to_owned(),
            Value::Number(tiktools_plugin_api::manifest::MAX_DECLARATIVE_HTTP_TIMEOUT_MS.into()),
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
            timeout_by_field: None,
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
    pub(crate) details: Option<String>,
    pub(crate) reachable: bool,
    pub(crate) gameplay_ready: Option<bool>,
    pub(crate) server_info: Option<Value>,
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
            details: None,
            reachable: false,
            gameplay_ready: None,
            server_info: None,
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
        if health.get("strategy").and_then(Value::as_str) == Some("sonicboom") {
            return self.check_sonicboom_connection(&plugin.manifest).await;
        }
        let path = health
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let settings = match self.capabilities.load_plugin_settings_raw(&plugin.manifest) {
            Ok(settings) => settings,
            Err(fetch) => return failed(fetch.to_string()),
        };
        let scope = json!({"settings": settings});
        let body = match self
            .fetch_declarative_json(
                &plugin.manifest,
                "GET",
                path,
                health.get("timeoutMs").and_then(Value::as_u64),
                &scope,
            )
            .await
        {
            Ok(body) => body,
            Err(error) => return failed(error),
        };
        let details = health.get("readinessField").and_then(Value::as_str).map(|field| {
            let version = body["apiVersion"].as_str().filter(|v| v.len() <= 32 && v.bytes().all(|b| b.is_ascii_digit() || b == b'.')).unwrap_or("unknown");
            format!("API version: {version}; game running: {}; in level: {}; host: {}; gameplay ready: {}",
                body["gameRunning"].as_bool().unwrap_or(false), body["inLevel"].as_bool().unwrap_or(false), body["isHost"].as_bool().unwrap_or(false), body[field].as_bool().unwrap_or(false))
        });
        if let Some(field) = health.get("readinessField").and_then(Value::as_str) {
            if body.get(field).and_then(Value::as_bool) != Some(true) {
                let mut result = failed("API reachable; gameplay unavailable.".to_owned());
                result.details = details;
                result.reachable = true;
                result.gameplay_ready = Some(false);
                return result;
            }
        }
        PluginConnectionCheck {
            ok: true,
            latency_ms: crate::helpers::now_millis().saturating_sub(started),
            error: None,
            details,
            server_info: None,
            reachable: true,
            gameplay_ready: health
                .get("readinessField")
                .and_then(Value::as_str)
                .map(|field| body.get(field).and_then(Value::as_bool).unwrap_or(false)),
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

/// Created once when preparing an execution, outside the transport sender.
fn execution_key() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "Cannot generate execution key")?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod execution_tests {
    #[test]
    fn distinct_executions_have_distinct_keys() {
        let keys: std::collections::HashSet<_> =
            (0..1024).map(|_| super::execution_key().unwrap()).collect();
        assert_eq!(keys.len(), 1024);
        assert!(keys.iter().all(|k| k.len() == 32));
    }
}

/// Optional manifest defaults use the same host-owned settings snapshot as
/// endpoint authentication. Explicit saved automation configuration wins.
fn apply_settings_defaults(
    config: &mut Map<String, Value>,
    settings: &Value,
    defaults: Option<&Value>,
) {
    let Some(defaults) = defaults.and_then(Value::as_object) else {
        return;
    };
    for (field, pointers) in defaults.iter().take(32) {
        if config
            .get(field)
            .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
        {
            continue;
        }
        if let Some(value) = pointers
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|pointer| settings.pointer(pointer))
            .find(|value| !value.is_null() && value.as_str() != Some(""))
        {
            config.insert(field.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod settings_defaults_tests {
    use super::*;
    #[test]
    fn defaults_from_settings_align_provider_voice_language_and_preserve_explicit_actions() {
        let settings = json!({"tts":{"providerId":"qwen3-gguf","defaultVoice":"qwen3-gguf:owned","language":"es"},"defaultVoice":"M1"});
        let defaults = json!({"provider":["/tts/providerId"],"voice":["/tts/defaultVoice","/defaultVoice"],"language":["/tts/language","/defaultLanguage"]});
        let mut config = Map::new();
        apply_settings_defaults(&mut config, &settings, Some(&defaults));
        assert_eq!(config["provider"], "qwen3-gguf");
        assert_eq!(config["voice"], "qwen3-gguf:owned");
        assert_eq!(config["language"], "es");
        config.insert("provider".into(), json!("pocket-tts"));
        config.insert("voice".into(), json!("pocket-tts:explicit"));
        apply_settings_defaults(&mut config, &settings, Some(&defaults));
        assert_eq!(config["provider"], "pocket-tts");
        assert_eq!(config["voice"], "pocket-tts:explicit");
        let mut legacy = Map::new();
        apply_settings_defaults(&mut legacy, &json!({"defaultVoice":"M1"}), Some(&defaults));
        assert_eq!(legacy["voice"], "M1");
        assert!(!legacy.contains_key("provider"));
    }
}

#[cfg(all(test, feature = "http"))]
mod sonicboom_action_http_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn sonicboom_actions_inherit_selected_provider_over_real_http() {
        struct Emitter;
        impl crate::HostEmitter for Emitter {
            fn emit(&self, _: crate::ipc::messages::HostMessage) {}
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for provider in ["supertonic", "pocket-tts", "qwen3-gguf"] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = [0; 8192];
                let n = socket.read(&mut bytes).await.unwrap();
                let request = String::from_utf8_lossy(&bytes[..n]);
                assert!(
                    request.starts_with(&format!("POST /api/tts/play?provider={provider}&voice=")),
                    "provider settings must reach HTTP routing"
                );
                assert!(request.lines().next().unwrap().contains("lang=es"));
                assert!(request
                    .to_lowercase()
                    .contains("authorization: bearer test-owned-token"));
                socket.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}").await.unwrap();
            }
        });
        let document: Value =
            serde_json::from_str(include_str!("../../../../../plugins/sonicboom/plugin.json"))
                .unwrap();
        let manifest = PluginManifest::from_json_str(&document.to_string()).unwrap();
        let root =
            std::env::temp_dir().join(format!("sonicboom-action-http-{}", fastrand::u64(..)));
        let mut core = crate::AppCore::new(Arc::new(Emitter));
        core.capabilities = Arc::new(crate::services::capabilities::CapabilityBroker::new(
            root.clone(),
        ));
        let core = Arc::new(core);
        let plugin = tiktools_plugin_loader::DiscoveredPlugin {
            manifest,
            directory: root.clone(),
            source: tiktools_plugin_loader::PluginSource::Development,
            available: true,
            reason: None,
            running: true,
        };
        for provider in ["supertonic", "pocket-tts", "qwen3-gguf"] {
            let settings = json!({"serverUrl":format!("http://{address}"),"apiToken":"test-owned-token","tts":{"providerId":provider,"defaultVoice":format!("{provider}:owned"),"language":"es"}});
            let settings = settings
                .as_object()
                .unwrap()
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            core.capabilities
                .save_plugin_settings(&plugin.manifest, &settings)
                .unwrap();
            let mut logs = Vec::new();
            let result = core
                .execute_declarative_action(
                    &plugin,
                    &document["actionTypes"][0]["http"],
                    &json!({"config":{"text":"test"}}),
                    &json!({}),
                    &mut logs,
                    false,
                )
                .await;
            assert!(result.is_ok(), "{result:?}");
            assert!(logs.iter().all(|log| !log.contains("test-owned-token")));
        }
        tokio::time::timeout(std::time::Duration::from_secs(10), server)
            .await
            .unwrap()
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
