//! Declarative HTTP integrations for schema v3 plugin manifests.
//!
//! A manifest `http` block declares one base URL plus shared auth for the
//! plugin's HTTP actions, option sources, and health probe. The host renders
//! `{{ event.* }}`, `{{ settings.* }}`, and `{{ config.* }}` templates,
//! enforces the loopback trust boundary, and sends through the same hardened
//! client as automation HTTP actions. Secrets stay host-side: settings
//! resolve from raw storage (never the redacted WebView payload), and tokens
//! are redacted from every surfaced string.

use std::sync::Arc;

use serde_json::{json, Map, Value};
use tiktools_plugin_api::PluginManifest;

use super::capabilities::{CapabilityBroker, SECRET_SETTING_PLACEHOLDER};
use crate::helpers::{is_loopback_host, is_private_host, value_to_string};

/// Permission a manifest must declare before a declarative fetch may leave
/// loopback. Loopback servers work without it.
pub const NETWORK_BIND_PERMISSION: &str = "network.bind";
/// Capability a manifest must declare to run declarative HTTP actions.
pub const HTTP_REQUEST_CAPABILITY: &str = "http.request";

pub(crate) const DEFAULT_DECLARATIVE_TIMEOUT_MS: u64 = 10_000;
const MAX_RENDERED_URL_LEN: usize = 8_192;

/// Renders `{{ dotted.path }}` templates against a scope object such as
/// `{"event": …, "settings": …, "config": …}`. Unknown paths render empty,
/// matching the automation template behavior.
pub(crate) fn render_scoped_template(source: &str, scope: &Value) -> String {
    let mut rendered = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        rendered.push_str(&rest[..start]);
        let expression = &rest[start + 2..];
        let Some(end) = expression.find("}}") else {
            rendered.push_str(&rest[start..]);
            return rendered;
        };
        let path = expression[..end].trim();
        if let Some(value) = read_scope_path(scope, path) {
            rendered.push_str(&value_to_string(value));
        }
        rest = &expression[end + 2..];
    }
    rendered.push_str(rest);
    rendered
}

fn read_scope_path<'a>(scope: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = scope;
    for part in path.split('.') {
        if part.is_empty() {
            continue;
        }
        current = current.get(part)?;
    }
    Some(current)
}

/// One fully rendered, policy-approved declarative request.
pub(crate) struct DeclarativeEndpoint {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_ms: u64,
    /// Raw secrets embedded in the request (tokens), for redaction from any
    /// surfaced summary, log line, or error. Never emitted or logged.
    pub secrets: Vec<String>,
    /// Private-network allowance for the hardened sender, derived from the
    /// loopback trust decision plus the manifest opt-in.
    pub allow_private_network: bool,
}

// Secrets and the possibly token-bearing URL never appear in debug output;
// only the secret count is visible for diagnostics.
impl std::fmt::Debug for DeclarativeEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeclarativeEndpoint")
            .field("method", &self.method)
            .field("timeout_ms", &self.timeout_ms)
            .field("secrets", &self.secrets.len())
            .field("allow_private_network", &self.allow_private_network)
            .finish_non_exhaustive()
    }
}

pub(crate) struct DeclarativeBuild<'a> {
    pub manifest: &'a PluginManifest,
    pub http: &'a Value,
    pub method: &'a str,
    pub path_template: &'a str,
    pub extra_headers: Option<&'a Map<String, Value>>,
    pub body_template: Option<&'a str>,
    pub timeout_override_ms: Option<u64>,
    pub scope: &'a Value,
    pub broker: &'a CapabilityBroker,
}

/// Renders and authorizes one declarative request without sending it.
///
/// Policy, in order: the plugin must be schema v3; the rendered base URL
/// must be http(s) with a host; the rendered path must stay base-relative
/// and keep the base host; loopback destinations are trusted (no permission,
/// token optional); anything beyond loopback requires the `network.bind`
/// permission plus declared, present authentication; private LAN hosts
/// additionally require the manifest `allowPrivateNetwork` opt-in.
pub(crate) fn build_declarative_request(
    build: &DeclarativeBuild<'_>,
) -> Result<DeclarativeEndpoint, String> {
    let manifest = build.manifest;
    if manifest.schema_version < 3 {
        return Err(format!(
            "Plugin `{}` is not a declarative (schema v3) plugin.",
            manifest.id
        ));
    }
    let http = build
        .http
        .as_object()
        .ok_or_else(|| format!("Plugin `{}` declares no HTTP integration.", manifest.id))?;
    let base_template = http
        .get("baseUrl")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let rendered_base = render_scoped_template(base_template, build.scope);
    let rendered_base = rendered_base.trim();
    if rendered_base.is_empty() || rendered_base.len() > MAX_RENDERED_URL_LEN {
        return Err(format!(
            "Plugin `{}` has an invalid server URL.",
            manifest.id
        ));
    }
    let base = url::Url::parse(rendered_base)
        .map_err(|_| format!("Plugin `{}` has an invalid server URL.", manifest.id))?;
    if !matches!(base.scheme(), "http" | "https") {
        return Err(format!(
            "Plugin `{}` must use an http(s) server URL.",
            manifest.id
        ));
    }
    if !base.username().is_empty() || base.password().is_some() {
        return Err(format!(
            "Plugin `{}` must not embed credentials in its server URL.",
            manifest.id
        ));
    }
    let host = base
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| format!("Plugin `{}` has a server URL with no host.", manifest.id))?;
    let loopback = is_loopback_host(&host);
    if !loopback {
        build
            .broker
            .require_permission(manifest, NETWORK_BIND_PERMISSION)
            .map_err(|error| error.to_string())?;
        if is_private_host(&host)
            && !http
                .get("allowPrivateNetwork")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            return Err(format!(
                "Plugin `{manifest_id}` reaches a private host; its manifest must opt in with allowPrivateNetwork.",
                manifest_id = manifest.id
            ));
        }
    }

    let rendered_path = render_scoped_template(build.path_template, build.scope);
    let rendered_path = rendered_path.trim();
    if !rendered_path.starts_with('/') {
        return Err(format!(
            "Plugin `{}` must keep declarative paths base-relative.",
            manifest.id
        ));
    }
    if rendered_path
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(format!(
            "Plugin `{}` rendered an invalid request path.",
            manifest.id
        ));
    }
    let mut url = url::Url::parse(&format!(
        "{base}{path}",
        base = rendered_base.trim_end_matches('/'),
        path = rendered_path
    ))
    .map_err(|_| format!("Plugin `{}` rendered an invalid request URL.", manifest.id))?;
    if url.host_str().map(str::to_ascii_lowercase).as_deref() != Some(host.as_str()) {
        return Err(format!(
            "Plugin `{}` must not change the destination host per request.",
            manifest.id
        ));
    }

    let auth = http.get("auth").and_then(Value::as_object);
    let auth_type = auth
        .and_then(|auth| auth.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let mut secrets = Vec::new();
    let mut auth_header: Option<(String, String)> = None;
    match auth_type {
        "none" => {
            if !loopback {
                return Err(format!(
                    "Plugin `{}` reaches beyond loopback without authentication.",
                    manifest.id
                ));
            }
        }
        "bearer" | "header" | "query" => {
            let token_setting = auth
                .and_then(|auth| auth.get("tokenSetting"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let token = build
                .scope
                .get("settings")
                .and_then(|settings| settings.get(token_setting))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned();
            if token.is_empty() {
                if !loopback {
                    return Err(format!(
                        "Plugin `{}` needs its `{token_setting}` setting to reach {host}.",
                        manifest.id
                    ));
                }
            } else {
                secrets.push(token.clone());
                match auth_type {
                    "bearer" => {
                        let scheme = auth
                            .and_then(|auth| auth.get("scheme"))
                            .and_then(Value::as_str)
                            .unwrap_or("Bearer");
                        auth_header =
                            Some(("authorization".to_owned(), format!("{scheme} {token}")));
                    }
                    "header" => {
                        let name = auth
                            .and_then(|auth| auth.get("header"))
                            .and_then(Value::as_str)
                            .unwrap_or("X-Api-Token");
                        auth_header = Some((name.to_owned(), token));
                    }
                    _ => {
                        let param = auth
                            .and_then(|auth| auth.get("param"))
                            .and_then(Value::as_str)
                            .unwrap_or("token");
                        url.query_pairs_mut().append_pair(param, &token);
                    }
                }
            }
        }
        _ => {
            return Err(format!(
                "Plugin `{}` declares an unknown auth type.",
                manifest.id
            ));
        }
    }

    let mut headers = Vec::new();
    for block in [
        http.get("headers").and_then(Value::as_object),
        build.extra_headers,
    ]
    .into_iter()
    .flatten()
    {
        for (name, value) in block {
            headers.push((
                name.clone(),
                render_scoped_template(&value_to_string(value), build.scope),
            ));
        }
    }
    if let Some(header) = auth_header {
        headers.push(header);
    }
    if headers.len() > 64 {
        return Err(format!(
            "Plugin `{}` declares too many headers.",
            manifest.id
        ));
    }

    let timeout_ms = build
        .timeout_override_ms
        .or_else(|| http.get("timeoutMs").and_then(Value::as_u64))
        .unwrap_or(DEFAULT_DECLARATIVE_TIMEOUT_MS)
        .clamp(100, 120_000);
    Ok(DeclarativeEndpoint {
        method: build.method.to_ascii_uppercase(),
        url: url.to_string(),
        headers,
        body: build
            .body_template
            .map(|template| render_scoped_template(template, build.scope)),
        timeout_ms,
        secrets,
        allow_private_network: loopback
            || http
                .get("allowPrivateNetwork")
                .and_then(Value::as_bool)
                .unwrap_or(false),
    })
}

/// Replaces embedded tokens in surfaced text (summaries, logs, errors) with
/// the settings placeholder. Very short values are skipped: they cannot be
/// real tokens, and blanking them would corrupt unrelated prose.
pub(crate) fn redact_endpoint_secrets(text: &str, secrets: &[String]) -> String {
    let mut redacted = text.to_owned();
    for secret in secrets {
        if secret.len() >= 4 {
            redacted = redacted.replace(secret, SECRET_SETTING_PLACEHOLDER);
        }
    }
    redacted
}

/// Reads the manifest-declared auth shape as `(auth_type, token_setting)`.
/// Unknown shapes degrade to `("none", "")` so diagnostics stay total.
fn declarative_auth_label(http: &Value) -> (String, String) {
    let auth = http.get("auth").and_then(Value::as_object);
    let auth_type = auth
        .and_then(|auth| auth.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_owned();
    let token_setting = auth
        .and_then(|auth| auth.get("tokenSetting"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    (auth_type, token_setting)
}

/// One debug line per declarative call: method, path, and whether a
/// credential was attached. Callers must redact the result: query-string
/// auth embeds the token in the URL. The secret value itself is never
/// named here on purpose — only its presence.
fn declarative_request_line(
    endpoint: &DeclarativeEndpoint,
    auth_type: &str,
    token_setting: &str,
) -> String {
    let path = url::Url::parse(&endpoint.url)
        .map(|url| match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_owned(),
        })
        .unwrap_or_else(|_| "(unparsable url)".to_owned());
    let auth_note = if endpoint.secrets.is_empty() {
        if token_setting.is_empty() {
            "auth: none attached".to_owned()
        } else {
            format!("auth: none attached (`{token_setting}` is empty)")
        }
    } else {
        format!("auth: {auth_type} attached")
    };
    format!("{} {path} ({auth_note})", endpoint.method)
}

/// Turns a bare transport failure into an actionable credential hint.
///
/// The shared HTTP engine reports only `HTTP {status} ...`; the declarative
/// layer knows which plugin, endpoint, and token setting were involved, so
/// 401/403 responses name them instead of leaving the operator guessing.
/// Other statuses pass through untouched.
fn with_auth_hint(
    error: &str,
    manifest_id: &str,
    auth_type: &str,
    token_setting: &str,
    token_attached: bool,
) -> String {
    let status = if error.starts_with("HTTP 401") {
        Some(401)
    } else if error.starts_with("HTTP 403") {
        Some(403)
    } else {
        None
    };
    let Some(status) = status else {
        return error.to_owned();
    };
    let setting_note = if token_setting.is_empty() {
        "The plugin declares no token setting, so no credential can be attached.".to_owned()
    } else if token_attached {
        format!(
            "A {auth_type} credential from the `{token_setting}` setting was attached, so the stored value is wrong, expired, or revoked. Paste a fresh API token issued by this server into `{token_setting}` (Connection page)."
        )
    } else {
        format!(
            "No credential was attached because the `{token_setting}` setting is empty. This server requires authentication: set `{token_setting}` in the plugin connection settings."
        )
    };
    if status == 401 {
        format!(
            "{error} The server rejected the credentials for plugin `{manifest_id}`. {setting_note}"
        )
    } else {
        format!("{error} The server forbade the request for plugin `{manifest_id}`. {setting_note}")
    }
}

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
            .execute_http_action(&fetch, event, logs, Some(&allowed), false)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(http: Value) -> PluginManifest {
        PluginManifest::from_json_str(
            &serde_json::json!({
                "schemaVersion": 3,
                "id": "demo.http",
                "name": "Demo",
                "version": "1.0.0",
                "runtime": "declarative",
                "permissions": ["network.bind"],
                "http": http
            })
            .to_string(),
        )
        .unwrap()
    }

    fn broker() -> CapabilityBroker {
        CapabilityBroker::new(std::env::temp_dir())
    }

    #[test]
    fn auth_failures_name_the_plugin_setting_and_attachment() {
        // A token was attached but rejected: point at the stored value.
        let hinted = with_auth_hint(
            "HTTP 401 request failed",
            "sonicboom.server",
            "bearer",
            "apiToken",
            true,
        );
        assert!(hinted.contains("sonicboom.server"), "{hinted}");
        assert!(hinted.contains("`apiToken`"), "{hinted}");
        assert!(hinted.contains("wrong, expired, or revoked"), "{hinted}");
        // Nothing attached: point at the empty setting.
        let hinted = with_auth_hint(
            "HTTP 401 request failed",
            "sonicboom.server",
            "bearer",
            "apiToken",
            false,
        );
        assert!(hinted.contains("`apiToken` setting is empty"), "{hinted}");
        // 403 gets the same treatment with forbidden wording.
        let hinted = with_auth_hint(
            "HTTP 403 request failed",
            "demo.http",
            "header",
            "apiKey",
            true,
        );
        assert!(hinted.contains("forbade"), "{hinted}");
        assert!(hinted.contains("`apiKey`"), "{hinted}");
        // Other statuses pass through untouched.
        assert_eq!(
            with_auth_hint(
                "HTTP 500 server error",
                "demo.http",
                "bearer",
                "apiToken",
                true
            ),
            "HTTP 500 server error"
        );
        assert_eq!(
            with_auth_hint(
                "connection refused",
                "demo.http",
                "bearer",
                "apiToken",
                false
            ),
            "connection refused"
        );
    }

    #[test]
    fn request_line_reports_path_and_auth_presence_without_secrets() {
        let endpoint = DeclarativeEndpoint {
            method: "POST".to_owned(),
            url: "http://localhost:17842/api/tts/play?voice=M1".to_owned(),
            headers: Vec::new(),
            body: None,
            timeout_ms: 10_000,
            secrets: vec!["super-secret-token".to_owned()],
            allow_private_network: true,
        };
        let line = redact_endpoint_secrets(
            &declarative_request_line(&endpoint, "bearer", "apiToken"),
            &endpoint.secrets,
        );
        assert!(line.contains("POST /api/tts/play?voice=M1"), "{line}");
        assert!(line.contains("auth: bearer attached"), "{line}");
        assert!(!line.contains("super-secret-token"), "{line}");

        let bare = DeclarativeEndpoint {
            method: "POST".to_owned(),
            url: "http://localhost:17842/api/tts/play?voice=M1".to_owned(),
            headers: Vec::new(),
            body: None,
            timeout_ms: 10_000,
            secrets: Vec::new(),
            allow_private_network: true,
        };
        let line = declarative_request_line(&bare, "bearer", "apiToken");
        assert!(
            line.contains("auth: none attached (`apiToken` is empty)"),
            "{line}"
        );

        // Query-string auth embeds the token in the URL: redaction blanks it.
        let query = DeclarativeEndpoint {
            method: "GET".to_owned(),
            url: "http://localhost:17842/v1/voices?token=super-secret-token".to_owned(),
            headers: Vec::new(),
            body: None,
            timeout_ms: 10_000,
            secrets: vec!["super-secret-token".to_owned()],
            allow_private_network: true,
        };
        let line = redact_endpoint_secrets(
            &declarative_request_line(&query, "query", "apiToken"),
            &query.secrets,
        );
        assert!(!line.contains("super-secret-token"), "{line}");
        assert!(line.contains(SECRET_SETTING_PLACEHOLDER), "{line}");
    }

    #[test]
    fn renders_event_settings_and_config_scopes() {
        let scope = json!({
            "event": {"data": {"comment": "hello"}},
            "settings": {"serverUrl": "http://localhost:17842"},
            "config": {"voice": "M1"}
        });
        assert_eq!(
            render_scoped_template(
                "{{ settings.serverUrl }}/play?voice={{ config.voice }}&text={{ event.data.comment }}",
                &scope
            ),
            "http://localhost:17842/play?voice=M1&text=hello"
        );
        assert_eq!(render_scoped_template("a{{ missing }}b", &scope), "ab");
        assert_eq!(
            render_scoped_template("a{{ unclosed", &scope),
            "a{{ unclosed"
        );
    }

    #[test]
    fn loopback_is_trusted_without_permission_or_token() {
        let manifest = manifest(json!({
            "baseUrl": "{{ settings.serverUrl }}",
            "auth": {"type": "bearer", "tokenSetting": "apiToken"}
        }));
        let broker = broker();
        let scope = json!({"settings": {"serverUrl": "http://localhost:17842"}});
        let endpoint = build_declarative_request(&DeclarativeBuild {
            manifest: &manifest,
            http: manifest.http.as_ref().unwrap(),
            method: "POST",
            path_template: "/api/tts/play?voice={{ config.voice }}",
            extra_headers: None,
            body_template: Some("{{ event.data.comment }}"),
            timeout_override_ms: None,
            scope: &scope,
            broker: &broker,
        })
        .unwrap();
        assert_eq!(endpoint.url, "http://localhost:17842/api/tts/play?voice=");
        assert!(endpoint.allow_private_network);
        assert!(endpoint.secrets.is_empty());
        assert!(endpoint.headers.is_empty());
    }

    #[test]
    fn remote_hosts_require_permission_and_token() {
        let manifest = manifest(json!({
            "baseUrl": "https://tts.example.com",
            "auth": {"type": "bearer", "tokenSetting": "apiToken"}
        }));
        let broker = broker();
        // Missing token is rejected even with the permission declared.
        let scope = json!({"settings": {}});
        assert!(build_declarative_request(&DeclarativeBuild {
            manifest: &manifest,
            http: manifest.http.as_ref().unwrap(),
            method: "GET",
            path_template: "/v1/voices",
            extra_headers: None,
            body_template: None,
            timeout_override_ms: None,
            scope: &scope,
            broker: &broker,
        })
        .is_err());
        // A present token authorizes with a Bearer header.
        let scope = json!({"settings": {"apiToken": "tok-123"}});
        let endpoint = build_declarative_request(&DeclarativeBuild {
            manifest: &manifest,
            http: manifest.http.as_ref().unwrap(),
            method: "GET",
            path_template: "/v1/voices",
            extra_headers: None,
            body_template: None,
            timeout_override_ms: None,
            scope: &scope,
            broker: &broker,
        })
        .unwrap();
        assert_eq!(endpoint.url, "https://tts.example.com/v1/voices");
        assert_eq!(
            endpoint.headers,
            vec![("authorization".to_owned(), "Bearer tok-123".to_owned())]
        );
        assert_eq!(endpoint.secrets, vec!["tok-123".to_owned()]);
        assert_eq!(
            redact_endpoint_secrets(
                "https://tts.example.com/?token=tok-123 failed",
                &endpoint.secrets
            ),
            "https://tts.example.com/?token=•••••••• failed"
        );
    }

    #[test]
    fn remote_hosts_without_permission_are_rejected() {
        let manifest = PluginManifest::from_json_str(
            &serde_json::json!({
                "schemaVersion": 3,
                "id": "demo.noperm",
                "name": "Demo",
                "version": "1.0.0",
                "runtime": "declarative",
                "http": {
                    "baseUrl": "https://tts.example.com",
                    "auth": {"type": "bearer", "tokenSetting": "apiToken"}
                }
            })
            .to_string(),
        )
        .unwrap();
        let broker = broker();
        let scope = json!({"settings": {"apiToken": "tok-123"}});
        let error = build_declarative_request(&DeclarativeBuild {
            manifest: &manifest,
            http: manifest.http.as_ref().unwrap(),
            method: "GET",
            path_template: "/v1/voices",
            extra_headers: None,
            body_template: None,
            timeout_override_ms: None,
            scope: &scope,
            broker: &broker,
        })
        .unwrap_err();
        assert!(error.contains("network.bind"), "unexpected error: {error}");
    }

    #[test]
    fn lan_hosts_require_the_manifest_opt_in() {
        let manifest = manifest(json!({"baseUrl": "http://192.168.1.10:3000"}));
        let broker = broker();
        let scope = json!({"settings": {}});
        let error = build_declarative_request(&DeclarativeBuild {
            manifest: &manifest,
            http: manifest.http.as_ref().unwrap(),
            method: "GET",
            path_template: "/ready",
            extra_headers: None,
            body_template: None,
            timeout_override_ms: None,
            scope: &scope,
            broker: &broker,
        })
        .unwrap_err();
        assert!(
            error.contains("allowPrivateNetwork"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rendered_paths_cannot_change_the_host() {
        let manifest = manifest(json!({"baseUrl": "http://localhost:17842"}));
        let broker = broker();
        let scope = json!({"config": {"next": "https://evil.example/"}});
        assert!(build_declarative_request(&DeclarativeBuild {
            manifest: &manifest,
            http: manifest.http.as_ref().unwrap(),
            method: "GET",
            path_template: "{{ config.next }}",
            extra_headers: None,
            body_template: None,
            timeout_override_ms: None,
            scope: &scope,
            broker: &broker,
        })
        .is_err());
    }

    /// Minimal canned HTTP/1.1 stub on loopback. Serves one response per
    /// connection based on the request path; records nothing sensitive.
    #[cfg(feature = "http")]
    async fn stub_server(
        routes: std::collections::HashMap<String, (u16, String)>,
    ) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let mut buffer = vec![0u8; 8_192];
                let Ok(read) = socket.read(&mut buffer).await else {
                    continue;
                };
                let request = String::from_utf8_lossy(&buffer[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let path = path.split('?').next().unwrap_or(path);
                let (status, body) = routes
                    .get(path)
                    .cloned()
                    .unwrap_or_else(|| (404, r#"{"error":"not found"}"#.to_owned()));
                let reason = if status == 200 { "OK" } else { "Not Found" };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });
        (address, task)
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    async fn fetches_option_documents_from_loopback() {
        use std::sync::Mutex;
        struct Emitter {
            messages: Mutex<Vec<crate::ipc::messages::HostMessage>>,
        }
        impl crate::HostEmitter for Emitter {
            fn emit(&self, message: crate::ipc::messages::HostMessage) {
                self.messages.lock().unwrap().push(message);
            }
        }
        let (address, server) = stub_server(
            [
                ("/ready".to_owned(), (200, r#"{"ok":true}"#.to_owned())),
                (
                    "/v1/voices".to_owned(),
                    (
                        200,
                        r#"{"voices":[{"id":"M1","name":"Marcus"},{"id":"F2","name":"Freya"}]}"#
                            .to_owned(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        )
        .await;
        let core = Arc::new(crate::AppCore::new(Arc::new(Emitter {
            messages: Mutex::new(Vec::new()),
        })));
        let manifest = manifest(json!({
            "baseUrl": format!("http://{address}"),
            "auth": {"type": "bearer", "tokenSetting": "apiToken"}
        }));
        let scope = json!({"settings": {"apiToken": "tok-123"}});
        let body = core
            .fetch_declarative_json(&manifest, "GET", "/ready", None, &scope)
            .await
            .unwrap();
        assert_eq!(body.get("ok"), Some(&Value::Bool(true)));
        let body = core
            .fetch_declarative_json(&manifest, "GET", "/v1/voices", None, &scope)
            .await
            .unwrap();
        let options =
            super::super::option_sources::map_option_items(&body, None, None, None).unwrap();
        assert_eq!(
            options,
            vec![
                json!({"value": "M1", "label": "Marcus"}),
                json!({"value": "F2", "label": "Freya"}),
            ]
        );
        server.abort();
    }

    #[test]
    fn builds_output_switch_post_with_bearer_auth() {
        let manifest = manifest(json!({
            "baseUrl": "{{ settings.serverUrl }}",
            "auth": {"type": "bearer", "tokenSetting": "apiToken"}
        }));
        let broker = broker();
        let scope = json!({
            "settings": {"serverUrl": "http://localhost:17842", "apiToken": "tok-123"},
            "config": {"device": "CABLE Input (VB-Audio Virtual Cable)"}
        });
        let mut headers = Map::new();
        headers.insert(
            "Content-Type".to_owned(),
            Value::String("application/json".to_owned()),
        );
        let endpoint = build_declarative_request(&DeclarativeBuild {
            manifest: &manifest,
            http: manifest.http.as_ref().unwrap(),
            method: "POST",
            path_template: "/api/audio/output",
            extra_headers: Some(&headers),
            body_template: Some("{\"device\":\"{{ config.device }}\"}"),
            timeout_override_ms: Some(10_000),
            scope: &scope,
            broker: &broker,
        })
        .unwrap();
        assert_eq!(endpoint.method, "POST");
        assert_eq!(endpoint.url, "http://localhost:17842/api/audio/output");
        assert_eq!(
            endpoint.body.as_deref(),
            Some("{\"device\":\"CABLE Input (VB-Audio Virtual Cable)\"}")
        );
        assert_eq!(
            endpoint.headers,
            vec![
                ("Content-Type".to_owned(), "application/json".to_owned()),
                ("authorization".to_owned(), "Bearer tok-123".to_owned()),
            ]
        );
        assert_eq!(endpoint.secrets, vec!["tok-123".to_owned()]);
        assert_eq!(endpoint.timeout_ms, 10_000);
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    async fn maps_device_list_and_reports_selection() {
        struct Emitter;
        impl crate::HostEmitter for Emitter {
            fn emit(&self, _message: crate::ipc::messages::HostMessage) {}
        }
        let (address, server) = stub_server(
            [(
                "/api/audio/devices".to_owned(),
                (
                    200,
                    r#"{"devices":[{"id":"default","name":"System Default","is_default":true,"is_selected":false},{"id":"CABLE Input (VB-Audio Virtual Cable)","name":"CABLE Input (VB-Audio Virtual Cable)","is_default":false,"is_selected":true}],"selected":"CABLE Input (VB-Audio Virtual Cable)"}"#
                        .to_owned(),
                ),
            )]
            .into_iter()
            .collect(),
        )
        .await;
        let core = Arc::new(crate::AppCore::new(Arc::new(Emitter)));
        let manifest = manifest(json!({
            "baseUrl": format!("http://{address}"),
            "auth": {"type": "bearer", "tokenSetting": "apiToken"}
        }));
        let scope = json!({"settings": {"apiToken": "tok-123"}});
        let body = core
            .fetch_declarative_json(&manifest, "GET", "/api/audio/devices", None, &scope)
            .await
            .unwrap();
        let options = super::super::option_sources::map_option_items(
            &body,
            Some("devices"),
            Some("id"),
            Some("name"),
        )
        .unwrap();
        assert_eq!(
            options,
            vec![
                json!({"value": "default", "label": "System Default"}),
                json!({"value": "CABLE Input (VB-Audio Virtual Cable)", "label": "CABLE Input (VB-Audio Virtual Cable)"}),
            ]
        );
        assert_eq!(
            super::super::option_sources::selected_option_value(&body, Some("devices"), Some("id"))
                .as_deref(),
            Some("CABLE Input (VB-Audio Virtual Cable)")
        );
        server.abort();
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    async fn missing_option_endpoint_reports_its_status() {
        struct Emitter;
        impl crate::HostEmitter for Emitter {
            fn emit(&self, _message: crate::ipc::messages::HostMessage) {}
        }
        // No routes: every path answers 404 like an older server without the
        // audio API. The UI matches this prefix to tell "unsupported" apart
        // from "unreachable", so the format is pinned here.
        let (address, server) = stub_server(std::collections::HashMap::new()).await;
        let core = Arc::new(crate::AppCore::new(Arc::new(Emitter)));
        let manifest = manifest(json!({
            "baseUrl": format!("http://{address}"),
            "auth": {"type": "bearer", "tokenSetting": "apiToken"}
        }));
        let scope = json!({"settings": {"apiToken": "tok-123"}});
        let error = core
            .fetch_declarative_json(&manifest, "GET", "/api/audio/devices", None, &scope)
            .await
            .unwrap_err();
        assert!(error.starts_with("HTTP 404"), "unexpected error: {error}");
        server.abort();
    }

    #[tokio::test]
    async fn probe_reports_unknown_plugins_without_fetching() {
        use std::sync::Mutex;
        struct Emitter {
            messages: Mutex<Vec<crate::ipc::messages::HostMessage>>,
        }
        impl crate::HostEmitter for Emitter {
            fn emit(&self, message: crate::ipc::messages::HostMessage) {
                self.messages.lock().unwrap().push(message);
            }
        }
        let emitter = Arc::new(Emitter {
            messages: Mutex::new(Vec::new()),
        });
        let core = Arc::new(crate::AppCore::new(emitter.clone()));
        core.probe_plugin_connection("definitely.not.installed")
            .await;
        let messages = emitter.messages.lock().unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            crate::ipc::messages::HostMessage::PluginConnectionResult { id, ok, error, .. } => {
                assert_eq!(id, "definitely.not.installed");
                assert!(!ok);
                assert!(error
                    .as_deref()
                    .unwrap_or_default()
                    .contains("not installed"));
            }
            other => panic!("unexpected probe message: {other:?}"),
        }
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    async fn fetch_errors_never_carry_tokens() {
        struct Emitter;
        impl crate::HostEmitter for Emitter {
            fn emit(&self, _message: crate::ipc::messages::HostMessage) {}
        }
        // A closed loopback port fails fast without touching the network.
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = closed.local_addr().unwrap();
        drop(closed);
        let core = Arc::new(crate::AppCore::new(Arc::new(Emitter)));
        let manifest = manifest(json!({
            "baseUrl": format!("http://{address}"),
            "auth": {"type": "query", "tokenSetting": "apiToken", "param": "token"}
        }));
        let scope = json!({"settings": {"apiToken": "tok-secret-123"}});
        let error = core
            .fetch_declarative_json(&manifest, "GET", "/v1/voices", None, &scope)
            .await
            .unwrap_err();
        assert!(
            !error.contains("tok-secret-123"),
            "token leaked into error: {error}"
        );
        assert!(
            error.contains(SECRET_SETTING_PLACEHOLDER),
            "token not redacted in error: {error}"
        );
    }
}
