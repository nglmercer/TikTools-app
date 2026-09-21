//! Rendered endpoint representation and request authorization policy.

use super::{render_scoped_template, NETWORK_BIND_PERMISSION};
use crate::helpers::value_to_string;
use crate::helpers::{is_loopback_host, is_private_host};
use crate::services::capabilities::CapabilityBroker;
use serde_json::{Map, Value};
use tiktools_plugin_api::PluginManifest;

pub(crate) const DEFAULT_DECLARATIVE_TIMEOUT_MS: u64 = 10_000;

const MAX_RENDERED_URL_LEN: usize = 8_192;

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
