//! One-click API token provisioning for declarative plugins.
//!
//! Some servers (SonicBoom) require a Bearer [REDACTED] token that can only be
//! minted through an admin login. Instead of making the operator copy that
//! token by hand, a manifest may declare a `tokenProvisioning` strategy and
//! the host executes it: admin login with operator-supplied credentials,
//! token creation, and storage into the manifest's `tokenSetting`.
//!
//! Security boundaries, all deliberate:
//! - The admin password is used once for the login POST and never persisted,
//!   logged, or echoed. Only the minted API token is stored, through the
//!   same secret-aware settings path as a manual paste.
//! - The minted value must be strict 64-char hex before it is stored, so a
//!   confused or drifted server page can never plant garbage HTML.
//! - Remote destinations keep the declarative policy: loopback is trusted,
//!   anything else needs the `network.bind` permission (plus the private-LAN
//!   opt-in), because the admin password travels in these requests.
//! - Responses are capped and redirects are never followed (the shared
//!   client disables them); the login 302 is read, not chased.

use std::sync::Arc;

use serde_json::Value;
use tiktools_plugin_api::PluginManifest;

#[cfg(feature = "http")]
use super::declarative_http::{render_scoped_template, NETWORK_BIND_PERMISSION};
#[cfg(feature = "http")]
use crate::helpers::{is_loopback_host, is_private_host};
#[cfg(feature = "http")]
use serde_json::json;

/// Strategy implemented below: form-based admin login (`id`/`pw`), CSRF
/// token read from the admin page, token creation, strict hex validation.
const ADMIN_FORM_STRATEGY: &str = "admin-form";
#[cfg(feature = "http")]
const PROVISION_TIMEOUT_MS: u64 = 10_000;
#[cfg(feature = "http")]
const MAX_PROVISION_BODY_BYTES: usize = 512 * 1024;
#[cfg(feature = "http")]
const MAX_RENDERED_URL_LEN: usize = 8_192;

/// True when the host can offer one-click provisioning for this plugin: a
/// supported strategy plus a token setting to store the minted value into.
pub(crate) fn supports_token_provisioning(manifest: &PluginManifest) -> bool {
    let http = manifest.http.as_ref();
    let strategy = http
        .and_then(|http| http.get("tokenProvisioning"))
        .and_then(|provisioning| provisioning.get("strategy"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if strategy != ADMIN_FORM_STRATEGY {
        return false;
    }
    http.and_then(|http| http.get("auth"))
        .and_then(|auth| auth.get("tokenSetting"))
        .and_then(Value::as_str)
        .is_some_and(|setting| !setting.trim().is_empty())
}

/// Extracts `<input ... name="{name}" value="...">` from an admin page.
/// Bounded string search on the exact markers the server renders; anything
/// else is a version drift the caller must report, never guess through.
#[cfg_attr(not(feature = "http"), allow(dead_code))]
fn extract_hidden_input(html: &str, name: &str) -> Option<String> {
    let marker = format!(r#"name="{name}" value=""#);
    let start = html.find(&marker)? + marker.len();
    let rest = html.get(start..)?;
    let end = rest.find('"')?;
    if end == 0 || end > 256 {
        return None;
    }
    Some(rest[..end].to_owned())
}

/// Extracts the one-time raw token from a token-creation response:
/// `<div class="new-token">…<code>RAW</code>`. The value must be strict
/// 64-char hex (the server mints 256-bit hex tokens); anything else fails
/// closed so markup can never become a stored credential.
#[cfg_attr(not(feature = "http"), allow(dead_code))]
fn extract_new_token(html: &str) -> Option<String> {
    let section = html.find(r#"<div class="new-token">"#)?;
    let code = html[section..].find("<code>")? + section + "<code>".len();
    let rest = html.get(code..)?;
    let end = rest.find("</code>")?;
    let raw = rest.get(..end)?.trim();
    if is_hex64(raw) {
        Some(raw.to_owned())
    } else {
        None
    }
}

#[cfg_attr(not(feature = "http"), allow(dead_code))]
fn is_hex64(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg_attr(not(feature = "http"), allow(dead_code))]
fn form_body(pairs: &[(&str, &str)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

#[cfg(feature = "http")]
struct ProvisionResponse {
    status: u16,
    cookies: Vec<String>,
    body: String,
}

impl crate::AppCore {
    #[cfg(feature = "http")]
    async fn provision_send(
        &self,
        method: &str,
        url: &str,
        headers: Vec<(String, String)>,
        body: Option<String>,
    ) -> Result<ProvisionResponse, String> {
        let client = self.http_client.as_ref().ok_or_else(|| {
            self.http_client_error.clone().unwrap_or_else(|| {
                "HTTP provisioning is disabled because its hardened client is unavailable."
                    .to_owned()
            })
        })?;
        let method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| format!("HTTP method is invalid: {method}"))?;
        let parsed =
            reqwest::Url::parse(url).map_err(|_| "Provisioning URL is invalid.".to_owned())?;
        let mut outgoing = client.request(method, parsed);
        for (key, value) in &headers {
            outgoing = outgoing.header(key, value);
        }
        if let Some(body) = body {
            outgoing = outgoing.body(body);
        }
        let response = outgoing
            .timeout(std::time::Duration::from_millis(PROVISION_TIMEOUT_MS))
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    format!("Provisioning timed out after {PROVISION_TIMEOUT_MS} ms.")
                } else {
                    format!("Provisioning request failed: {error}")
                }
            })?;
        // Redirects are never followed: the shared client disables them, and
        // the login 302 is consumed below as a success signal instead.
        let status = response.status().as_u16();
        let cookies = response
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter_map(|value| value.split(';').next())
            .map(str::trim)
            .filter(|pair| pair.contains('='))
            .map(str::to_owned)
            .collect();
        if response
            .content_length()
            .is_some_and(|length| length > MAX_PROVISION_BODY_BYTES as u64)
        {
            return Err("Provisioning response exceeds the size limit.".to_owned());
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("could not read provisioning response: {error}"))?;
        if bytes.len() > MAX_PROVISION_BODY_BYTES {
            return Err("Provisioning response exceeds the size limit.".to_owned());
        }
        Ok(ProvisionResponse {
            status,
            cookies,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        })
    }

    /// Mints an API token through the plugin's declared provisioning flow
    /// and stores it into the manifest's token setting.
    ///
    /// This is the value-returning core shared by the legacy WebView IPC
    /// path and the `plugins.token.provision` control operation. The admin
    /// password is used for exactly one login POST and never persisted,
    /// logged, echoed, or included in the error string.
    #[cfg(feature = "http")]
    pub(crate) async fn provision_plugin_token_inner(
        &self,
        id: &str,
        username: &str,
        password: &str,
    ) -> Result<(), String> {
        let Some(plugin) = self.plugins.get(id) else {
            return Err(format!("Plugin `{id}` is not installed."));
        };
        if !self.plugin_ready(&plugin.manifest.id) {
            return Err(format!(
                "Plugin `{id}` is not installed, enabled, or available."
            ));
        }
        let manifest = &plugin.manifest;
        if !supports_token_provisioning(manifest) {
            return Err(format!(
                "Plugin `{id}` does not declare a supported token provisioning flow."
            ));
        }
        let token_setting = manifest
            .http
            .as_ref()
            .and_then(|http| http.get("auth"))
            .and_then(|auth| auth.get("tokenSetting"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let settings = match self.capabilities.load_plugin_settings_raw(manifest) {
            Ok(settings) => settings,
            Err(error) => return Err(error.to_string()),
        };
        let base_template = manifest
            .http
            .as_ref()
            .and_then(|http| http.get("baseUrl"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let scope = json!({"settings": settings});
        let base = render_scoped_template(base_template, &scope);
        let base = base.trim().trim_end_matches('/');
        if base.is_empty() || base.len() > MAX_RENDERED_URL_LEN {
            return Err(format!("Plugin `{id}` has an invalid server URL."));
        }
        let parsed = match url::Url::parse(base) {
            Ok(parsed) => parsed,
            Err(_) => return Err(format!("Plugin `{id}` has an invalid server URL.")),
        };
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(format!("Plugin `{id}` must use an http(s) server URL."));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(format!(
                "Plugin `{id}` must not embed credentials in its server URL."
            ));
        }
        let host = match parsed.host_str() {
            Some(host) => host.to_ascii_lowercase(),
            None => return Err(format!("Plugin `{id}` has a server URL with no host.")),
        };
        // Credential-bearing requests keep the declarative trust policy:
        // loopback is trusted; anything else needs the network permission
        // (plus the private-LAN opt-in for LAN hosts).
        if !is_loopback_host(&host) {
            if let Err(error) = self
                .capabilities
                .require_permission(manifest, NETWORK_BIND_PERMISSION)
            {
                return Err(error.to_string());
            }
            let allow_private = manifest
                .http
                .as_ref()
                .and_then(|http| http.get("allowPrivateNetwork"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if is_private_host(&host) && !allow_private {
                return Err(format!(
                    "Plugin `{id}` reaches a private host; its manifest must opt in with allowPrivateNetwork."
                ));
            }
        }

        // Step 1: admin login. Success is a redirect carrying a session
        // cookie; failures stay on distinctive statuses with no cookie.
        let login = match self
            .provision_send(
                "POST",
                &format!("{base}/admin/login"),
                vec![(
                    "content-type".to_owned(),
                    "application/x-www-form-urlencoded".to_owned(),
                )],
                Some(form_body(&[("id", username), ("pw", password)])),
            )
            .await
        {
            Ok(login) => login,
            Err(error) => return Err(error),
        };
        if login.status == 401 {
            return Err("Admin login rejected: invalid ID or password.".to_owned());
        }
        if login.status == 429 {
            return Err(
                "Too many login attempts; the server locked this address temporarily. Try again later."
                    .to_owned(),
            );
        }
        if !(300..400).contains(&login.status) || login.cookies.is_empty() {
            return Err(format!(
                "Admin login failed with HTTP {}. Is this a SonicBoom server?",
                login.status
            ));
        }
        let cookie = login.cookies.join("; ");

        // Step 2: read the admin page for this session's CSRF token. A
        // non-200 here means the session did not authenticate.
        let admin = match self
            .provision_send(
                "GET",
                &format!("{base}/admin"),
                vec![("cookie".to_owned(), cookie.clone())],
                None,
            )
            .await
        {
            Ok(admin) => admin,
            Err(error) => return Err(error),
        };
        if admin.status != 200 {
            return Err(format!(
                "Admin session was not established (HTTP {}).",
                admin.status
            ));
        }
        let Some(csrf) =
            extract_hidden_input(&admin.body, "csrf_token").filter(|value| is_hex64(value))
        else {
            return Err(
                "Could not read the admin form (unexpected page). Is this a SonicBoom server?"
                    .to_owned(),
            );
        };

        // Step 3: mint the token (blank expiry = never expires).
        let created = match self
            .provision_send(
                "POST",
                &format!("{base}/admin/tokens"),
                vec![
                    (
                        "content-type".to_owned(),
                        "application/x-www-form-urlencoded".to_owned(),
                    ),
                    ("cookie".to_owned(), cookie.clone()),
                ],
                Some(form_body(&[("csrf_token", &csrf), ("expires_at", "")])),
            )
            .await
        {
            Ok(created) => created,
            Err(error) => return Err(error),
        };
        if created.status != 200 {
            return Err(format!(
                "Token creation failed with HTTP {}.",
                created.status
            ));
        }
        let Some(raw_token) = extract_new_token(&created.body) else {
            return Err("Token creation succeeded but the new value could not be read (server version mismatch?).".to_owned());
        };

        // Best-effort logout so the admin session does not linger.
        let _ = self
            .provision_send(
                "POST",
                &format!("{base}/admin/logout"),
                vec![
                    (
                        "content-type".to_owned(),
                        "application/x-www-form-urlencoded".to_owned(),
                    ),
                    ("cookie".to_owned(), cookie),
                ],
                Some(form_body(&[("csrf_token", &csrf)])),
            )
            .await;

        // Store the minted token through the secret-aware settings path and
        // refresh the settings echo so the UI updates without a restart.
        let stored = self
            .capabilities
            .load_plugin_settings_raw(manifest)
            .ok()
            .and_then(|values| values.as_object().cloned())
            .unwrap_or_default();
        let mut merged: std::collections::BTreeMap<String, Value> = stored.into_iter().collect();
        merged.insert(token_setting, Value::String(raw_token));
        self.save_plugin_settings(id, merged);
        Ok(())
    }

    #[cfg(not(feature = "http"))]
    pub(crate) async fn provision_plugin_token_inner(
        &self,
        _id: &str,
        _username: &str,
        _password: &str,
    ) -> Result<(), String> {
        Err("Token provisioning requires the host HTTP capability.".to_owned())
    }

    /// Legacy WebView adapter: same authoritative provisioning core, with
    /// UI-shaped `plugin-provision-result` reporting around it.
    pub(crate) async fn provision_plugin_token(
        self: &Arc<Self>,
        id: String,
        username: String,
        password: String,
    ) {
        match self
            .provision_plugin_token_inner(&id, &username, &password)
            .await
        {
            Ok(()) => self.emit(crate::ipc::messages::HostMessage::PluginProvisionResult {
                id,
                ok: true,
                error: None,
            }),
            Err(message) => self.emit(crate::ipc::messages::HostMessage::PluginProvisionResult {
                id,
                ok: false,
                error: Some(message),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provisioning_needs_a_supported_strategy_and_a_token_setting() {
        let supported = PluginManifest::from_json_str(
            r#"{"schemaVersion":3,"id":"demo.prov","name":"Demo","version":"1.0.0","runtime":"declarative","http":{"baseUrl":"http://localhost:17842","auth":{"type":"bearer","tokenSetting":"apiToken"},"tokenProvisioning":{"strategy":"admin-form"}}}"#,
        )
        .unwrap();
        assert!(supports_token_provisioning(&supported));
        let unknown = PluginManifest::from_json_str(
            r#"{"schemaVersion":3,"id":"demo.prov","name":"Demo","version":"1.0.0","runtime":"declarative","http":{"baseUrl":"http://localhost:17842","auth":{"type":"bearer","tokenSetting":"apiToken"},"tokenProvisioning":{"strategy":"future-v2"}}}"#,
        )
        .unwrap();
        assert!(!supports_token_provisioning(&unknown));
        let missing = PluginManifest::from_json_str(
            r#"{"schemaVersion":3,"id":"demo.prov","name":"Demo","version":"1.0.0","runtime":"declarative","http":{"baseUrl":"http://localhost:17842","auth":{"type":"none"},"tokenProvisioning":{"strategy":"admin-form"}}}"#,
        )
        .unwrap();
        assert!(!supports_token_provisioning(&missing));
    }

    #[test]
    fn csrf_extraction_is_strict_about_markers() {
        let csrf = "ab".repeat(32);
        let html = format!(
            r#"<form method="post" action="/admin/tokens"><input type="hidden" name="csrf_token" value="{csrf}">"#
        );
        assert_eq!(
            extract_hidden_input(&html, "csrf_token").as_deref(),
            Some(csrf.as_str())
        );
        assert_eq!(
            extract_hidden_input("<p>no form here</p>", "csrf_token"),
            None
        );
        assert_eq!(
            extract_hidden_input(r#"<input name="csrf_token" value="">"#, "csrf_token"),
            None
        );
        assert!(!is_hex64("xyz"));
        assert!(is_hex64(&csrf));
        assert!(!is_hex64(&csrf[..63]));
    }

    #[test]
    fn new_token_extraction_rejects_anything_but_strict_hex() {
        let raw = "cd".repeat(32);
        let html = format!(
            r#"<div class="new-token"><h2>New Token Created</h2><p><code>{raw}</code></p></div>"#
        );
        assert_eq!(extract_new_token(&html).as_deref(), Some(raw.as_str()));
        // Outside the banner, even valid hex is ignored.
        assert_eq!(
            extract_new_token(&format!("<p><code>{raw}</code></p>")),
            None
        );
        // Non-hex inside the banner fails closed.
        assert_eq!(
            extract_new_token(r#"<div class="new-token"><p><code>not-a-token!!</code></p></div>"#),
            None
        );
        assert_eq!(extract_new_token("<p>empty</p>"), None);
    }

    #[test]
    fn form_bodies_are_url_encoded() {
        assert_eq!(
            form_body(&[("id", "admin"), ("pw", "p@ss & word")]),
            "id=admin&pw=p%40ss+%26+word"
        );
    }
}
