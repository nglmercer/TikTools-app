//! Secret-safe request descriptions and error hints.

use super::DeclarativeEndpoint;
use crate::services::capabilities::SECRET_SETTING_PLACEHOLDER;
use serde_json::Value;

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
pub(crate) fn declarative_auth_label(http: &Value) -> (String, String) {
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
pub(crate) fn declarative_request_line(
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
pub(crate) fn with_auth_hint(
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
