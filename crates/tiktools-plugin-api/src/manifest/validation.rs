use serde_json::Value;
use thiserror::Error;

use super::types::MAX_PLUGIN_ACTION_TIMEOUT_MS;

pub(crate) const PLUGIN_SCHEMA_VERSION: u32 = 3;
/// Oldest schema the host still accepts. Schema v2 manifests parse exactly
/// as before: declarative keys are ignored unless the manifest declares v3.
pub(crate) const MIN_PLUGIN_SCHEMA_VERSION: u32 = 2;
pub(crate) fn is_supported_schema(version: u32) -> bool {
    (MIN_PLUGIN_SCHEMA_VERSION..=PLUGIN_SCHEMA_VERSION).contains(&version)
}
pub(crate) const MAX_MANIFEST_BYTES: usize = 256 * 1024;
pub(crate) const MAX_LIST_ENTRIES: usize = 128;
pub(crate) const MAX_DESCRIPTOR_BYTES: usize = 64 * 1024;
/// Long-form card copy rendered as markdown-lite in Details. Four times the
/// short description: room for a paragraph plus a short list, still bounded.
pub(crate) const MAX_LONG_DESCRIPTION_LEN: usize = 16 * 1024;
/// Declarative HTTP timeouts stay inside the automation executor's clamp.
pub(crate) const MIN_HTTP_TIMEOUT_MS: u64 = 100;
pub(crate) const MAX_HTTP_TIMEOUT_MS: u64 = 120_000;
pub(crate) const MAX_HTTP_HEADERS: usize = 32;
pub(crate) const MAX_BASE_URL_LEN: usize = 512;
pub(crate) const MAX_HTTP_PATH_LEN: usize = 1_024;
pub(crate) const MAX_OPTION_SOURCES: usize = 32;
pub(crate) const MAX_TEMPLATE_NODES: usize = 16;
pub(crate) const MAX_PAGE_SECTIONS: usize = 16;
#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("plugin manifest is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("plugin manifest must be a JSON object")]
    NotAnObject,
    #[error("plugin manifest has unsupported schema version {0}")]
    UnsupportedSchema(u32),
    #[error("plugin manifest field `{0}` is missing or invalid")]
    MissingField(&'static str),
    #[error("plugin manifest field `{0}` is invalid")]
    InvalidField(&'static str),
    #[error("plugin manifest entry must stay inside its package")]
    UnsafeEntry,
    #[error("plugin manifest is larger than {MAX_MANIFEST_BYTES} bytes")]
    TooLarge,
}
/// Prefixes owned by the host. Plugins declare their own event types under
/// any other dotted name (for example hotkey.pressed or timer.tick).
const RESERVED_EVENT_PREFIXES: [&str; 3] = ["tiktok.", "points.", "plugin."];

pub(crate) const MAX_EVENT_TYPE_LEN: usize = 64;
const MAX_EVENT_FIELDS: usize = 64;
const MAX_EVENT_OPTIONS: usize = 128;

/// Validate manifest fields that affect host-side plugin action execution.
/// Descriptor payloads remain JSON so plugin-defined fields stay extensible.
pub fn validate_action_type(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    if let Some(timeout) = object.get("timeoutMs") {
        let timeout = timeout
            .as_u64()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if timeout == 0 || timeout > MAX_PLUGIN_ACTION_TIMEOUT_MS {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    Ok(())
}

/// Validate the declarative `http` block of a v3 action descriptor, plus its
/// per-field `optionSources`. Only called for schema v3 manifests so v2
/// descriptors keep flowing to process plugins untouched.
pub fn validate_declarative_action(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    if let Some(http) = object.get("http") {
        validate_action_http(http)?;
    }
    if let Some(sources) = object.get("optionSources") {
        let sources = sources
            .as_object()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if sources.len() > MAX_OPTION_SOURCES {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
        for (field, source) in sources {
            if field.trim().is_empty() || field.len() > 128 {
                return Err(ManifestError::InvalidField("actionTypes"));
            }
            validate_option_source(source)?;
        }
    }
    Ok(())
}

fn validate_action_http(http: &Value) -> Result<(), ManifestError> {
    let object = http
        .as_object()
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    if let Some(method) = object.get("method") {
        let method = method
            .as_str()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if !matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD") {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    validate_relative_path(path).map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    if let Some(headers) = object.get("headers") {
        validate_header_map(headers).map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    }
    if let Some(body) = object.get("body") {
        let body = body
            .as_str()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if body.len() > MAX_DESCRIPTOR_BYTES {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    if let Some(timeout) = object.get("timeoutMs") {
        validate_http_timeout(timeout).map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    }
    if let Some(emit) = object.get("emitResponseAs") {
        let emit = emit
            .as_str()
            .ok_or(ManifestError::InvalidField("actionTypes"))?;
        if emit.trim().is_empty() || emit.len() > 128 {
            return Err(ManifestError::InvalidField("actionTypes"));
        }
    }
    Ok(())
}

fn validate_option_source(source: &Value) -> Result<(), ManifestError> {
    let object = source
        .as_object()
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("actionTypes"))?;
    validate_relative_path(path).map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    for key in ["itemsPath", "valuePath", "labelPath"] {
        if let Some(value) = object.get(key) {
            let value = value
                .as_str()
                .ok_or(ManifestError::InvalidField("actionTypes"))?;
            if value.trim().is_empty() || value.len() > 256 {
                return Err(ManifestError::InvalidField("actionTypes"));
            }
        }
    }
    if let Some(timeout) = object.get("timeoutMs") {
        validate_http_timeout(timeout).map_err(|_| ManifestError::InvalidField("actionTypes"))?;
    }
    Ok(())
}

/// Validate the top-level declarative `http` integration block (v3 only).
/// The block is parsed strictly: a malformed base URL or auth shape fails
/// discovery instead of silently degrading host-side fetches.
pub fn validate_http_config(http: &Value) -> Result<(), ManifestError> {
    let object = http
        .as_object()
        .ok_or(ManifestError::InvalidField("http"))?;
    let base_url = object
        .get("baseUrl")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("http"))?;
    validate_base_url(base_url)?;
    if let Some(timeout) = object.get("timeoutMs") {
        validate_http_timeout(timeout).map_err(|_| ManifestError::InvalidField("http"))?;
    }
    if let Some(allow) = object.get("allowPrivateNetwork") {
        if !allow.is_boolean() {
            return Err(ManifestError::InvalidField("http"));
        }
    }
    if let Some(headers) = object.get("headers") {
        validate_header_map(headers).map_err(|_| ManifestError::InvalidField("http"))?;
    }
    if let Some(auth) = object.get("auth") {
        validate_http_auth(auth)?;
    }
    if let Some(health) = object.get("health") {
        let health = health
            .as_object()
            .ok_or(ManifestError::InvalidField("http"))?;
        let path = health
            .get("path")
            .and_then(Value::as_str)
            .ok_or(ManifestError::InvalidField("http"))?;
        validate_relative_path(path).map_err(|_| ManifestError::InvalidField("http"))?;
        if let Some(timeout) = health.get("timeoutMs") {
            validate_http_timeout(timeout).map_err(|_| ManifestError::InvalidField("http"))?;
        }
    }
    if let Some(provisioning) = object.get("tokenProvisioning") {
        validate_token_provisioning(provisioning)?;
    }
    Ok(())
}

/// Validate the optional `tokenProvisioning` descriptor: a named strategy
/// the host can execute to mint an API token from operator credentials.
/// Shape errors fail discovery; unknown strategy names pass validation but
/// the host offers no provisioning for them, so newer manifests keep
/// loading on older hosts with the button simply absent.
fn validate_token_provisioning(provisioning: &Value) -> Result<(), ManifestError> {
    let object = provisioning
        .as_object()
        .ok_or(ManifestError::InvalidField("http"))?;
    let strategy = object
        .get("strategy")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("http"))?;
    if strategy.trim().is_empty() || strategy.len() > 64 {
        return Err(ManifestError::InvalidField("http"));
    }
    Ok(())
}

fn validate_http_auth(auth: &Value) -> Result<(), ManifestError> {
    let object = auth
        .as_object()
        .ok_or(ManifestError::InvalidField("http"))?;
    let auth_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("http"))?;
    match auth_type {
        "none" => Ok(()),
        "bearer" => {
            require_token_setting(object)?;
            if let Some(scheme) = object.get("scheme") {
                let scheme = scheme.as_str().ok_or(ManifestError::InvalidField("http"))?;
                if scheme.trim().is_empty() || scheme.len() > 32 || !is_header_token(scheme) {
                    return Err(ManifestError::InvalidField("http"));
                }
            }
            Ok(())
        }
        "header" => {
            require_token_setting(object)?;
            let name = object
                .get("header")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("http"))?;
            if !is_header_token(name) || name.eq_ignore_ascii_case("authorization") {
                return Err(ManifestError::InvalidField("http"));
            }
            Ok(())
        }
        "query" => {
            require_token_setting(object)?;
            let param = object
                .get("param")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("http"))?;
            if param.trim().is_empty()
                || param.len() > 64
                || !param
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            {
                return Err(ManifestError::InvalidField("http"));
            }
            Ok(())
        }
        _ => Err(ManifestError::InvalidField("http")),
    }
}

fn require_token_setting(object: &serde_json::Map<String, Value>) -> Result<(), ManifestError> {
    let setting = object
        .get("tokenSetting")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("http"))?;
    if setting.trim().is_empty() || setting.len() > 128 {
        return Err(ManifestError::InvalidField("http"));
    }
    Ok(())
}

/// Base URLs are either literal http(s) URLs or `{{ settings.* }}` templates
/// (for example a user-overridable server URL). Only the skeleton is checked
/// here; the host validates the rendered URL strictly before every fetch, so
/// a setting value can never smuggle a new scheme past execution policy.
fn validate_base_url(base_url: &str) -> Result<(), ManifestError> {
    if base_url.is_empty() || base_url.len() > MAX_BASE_URL_LEN {
        return Err(ManifestError::InvalidField("http"));
    }
    if base_url.chars().any(|char| char.is_control()) {
        return Err(ManifestError::InvalidField("http"));
    }
    if base_url.contains("{{") {
        // Template spans may carry whitespace (`{{ settings.serverUrl }}`);
        // the literal remainder must not.
        let mut rest = base_url;
        let mut spans = 0;
        while let Some(start) = rest.find("{{") {
            let head = &rest[..start];
            if head.chars().any(char::is_whitespace) {
                return Err(ManifestError::InvalidField("http"));
            }
            let tail = &rest[start + 2..];
            let Some(end) = tail.find("}}") else {
                return Err(ManifestError::InvalidField("http"));
            };
            spans += 1;
            rest = &tail[end + 2..];
        }
        if spans == 0 || rest.chars().any(char::is_whitespace) {
            return Err(ManifestError::InvalidField("http"));
        }
        return Ok(());
    }
    if base_url.chars().any(char::is_whitespace) {
        return Err(ManifestError::InvalidField("http"));
    }
    let rest = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .ok_or(ManifestError::InvalidField("http"))?;
    if rest.is_empty() {
        return Err(ManifestError::InvalidField("http"));
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() || path.len() > MAX_HTTP_PATH_LEN {
        return Err("path length");
    }
    if !path.starts_with('/') {
        return Err("path must be base-relative");
    }
    if path.chars().any(|character| character.is_control()) {
        return Err("path holds control characters");
    }
    // Template spans may carry whitespace (`{{ config.voice }}`); the
    // literal remainder must not, and it must not carry a scheme.
    let mut rest = path;
    while let Some(start) = rest.find("{{") {
        let head = &rest[..start];
        if head.chars().any(char::is_whitespace) || head.contains("://") {
            return Err("path must stay base-relative");
        }
        let tail = &rest[start + 2..];
        let Some(end) = tail.find("}}") else {
            return Err("path holds an unclosed template");
        };
        rest = &tail[end + 2..];
    }
    if rest.chars().any(char::is_whitespace) || rest.contains("://") {
        return Err("path must stay base-relative");
    }
    Ok(())
}

fn validate_header_map(headers: &Value) -> Result<(), &'static str> {
    let headers = headers.as_object().ok_or("headers must be an object")?;
    if headers.len() > MAX_HTTP_HEADERS {
        return Err("too many headers");
    }
    for (name, value) in headers {
        if !is_header_token(name) {
            return Err("invalid header name");
        }
        let value = value.as_str().ok_or("header values must be strings")?;
        if value.len() > 4_096 || value.chars().any(|char| matches!(char, '\r' | '\n')) {
            return Err("invalid header value");
        }
    }
    Ok(())
}

fn is_header_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn validate_http_timeout(timeout: &Value) -> Result<(), &'static str> {
    let timeout = timeout.as_u64().ok_or("timeout must be a number")?;
    if !(MIN_HTTP_TIMEOUT_MS..=MAX_HTTP_TIMEOUT_MS).contains(&timeout) {
        return Err("timeout out of range");
    }
    Ok(())
}

/// Validate one templates entry from a v3 manifest. Like event types, shape
/// errors are reported by the host catalog merge, which skips the entry.
pub fn validate_plugin_template(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("templates"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("templates"))?;
    if !is_valid_plugin_id(id) {
        return Err(ManifestError::InvalidField("templates"));
    }
    validate_localized(object.get("title"))
        .map_err(|_| ManifestError::InvalidField("templates"))?;
    if let Some(description) = object.get("description") {
        validate_localized(Some(description))
            .map_err(|_| ManifestError::InvalidField("templates"))?;
    }
    if let Some(icon) = object.get("icon") {
        let icon = icon
            .as_str()
            .ok_or(ManifestError::InvalidField("templates"))?;
        if icon.trim().is_empty() || icon.len() > 64 || !is_header_token(icon) {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    let event_type = object
        .get("eventType")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("templates"))?;
    // Templates target host triggers (tiktok.chat), so only the dotted
    // syntax is checked here; host namespaces stay reserved for plugins that
    // declare their own event types.
    if !is_valid_trigger_name(event_type) {
        return Err(ManifestError::InvalidField("templates"));
    }
    let required = object
        .get("requiredNodeTypes")
        .and_then(Value::as_array)
        .ok_or(ManifestError::InvalidField("templates"))?;
    if required.is_empty() || required.len() > MAX_TEMPLATE_NODES {
        return Err(ManifestError::InvalidField("templates"));
    }
    for node_type in required {
        let node_type = node_type
            .as_str()
            .ok_or(ManifestError::InvalidField("templates"))?;
        if !is_valid_trigger_name(node_type) {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    if let Some(category) = object.get("category") {
        let category = category
            .as_str()
            .ok_or(ManifestError::InvalidField("templates"))?;
        if category.trim().is_empty() || category.len() > 64 {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    if let Some(params) = object.get("params") {
        if !params.is_object() {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    if let Some(ui_hints) = object.get("uiHints") {
        if !ui_hints.is_object() {
            return Err(ManifestError::InvalidField("templates"));
        }
    }
    let workflow = object
        .get("workflow")
        .and_then(Value::as_object)
        .ok_or(ManifestError::InvalidField("templates"))?;
    let nodes = workflow
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or(ManifestError::InvalidField("templates"))?;
    if nodes.is_empty() || nodes.len() > MAX_TEMPLATE_NODES {
        return Err(ManifestError::InvalidField("templates"));
    }
    for node in nodes {
        let node = node
            .as_object()
            .ok_or(ManifestError::InvalidField("templates"))?;
        let node_type = node
            .get("type")
            .and_then(Value::as_str)
            .ok_or(ManifestError::InvalidField("templates"))?;
        if !is_valid_trigger_name(node_type) {
            return Err(ManifestError::InvalidField("templates"));
        }
        if let Some(config) = node.get("config") {
            if !config.is_object() {
                return Err(ManifestError::InvalidField("templates"));
            }
        }
    }
    Ok(())
}

/// Validate one pages entry from a v3 manifest. Section kinds form a fixed,
/// host-rendered widget set: unknown kinds are rejected so a manifest can
/// never smuggle arbitrary markup or script into the WebView.
pub fn validate_plugin_page(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("pages"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("pages"))?;
    if !is_valid_plugin_id(id) {
        return Err(ManifestError::InvalidField("pages"));
    }
    validate_localized(object.get("title")).map_err(|_| ManifestError::InvalidField("pages"))?;
    if let Some(icon) = object.get("icon") {
        let icon = icon.as_str().ok_or(ManifestError::InvalidField("pages"))?;
        if icon.trim().is_empty() || icon.len() > 64 || !is_header_token(icon) {
            return Err(ManifestError::InvalidField("pages"));
        }
    }
    let sections = object
        .get("sections")
        .and_then(Value::as_array)
        .ok_or(ManifestError::InvalidField("pages"))?;
    if sections.is_empty() || sections.len() > MAX_PAGE_SECTIONS {
        return Err(ManifestError::InvalidField("pages"));
    }
    for section in sections {
        validate_page_section(section)?;
    }
    Ok(())
}

fn validate_page_section(section: &Value) -> Result<(), ManifestError> {
    let object = section
        .as_object()
        .ok_or(ManifestError::InvalidField("pages"))?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("pages"))?;
    if let Some(title) = object.get("title") {
        validate_localized(Some(title)).map_err(|_| ManifestError::InvalidField("pages"))?;
    }
    match kind {
        "text" => {
            let text = object
                .get("text")
                .ok_or(ManifestError::InvalidField("pages"))?;
            validate_localized(Some(text)).map_err(|_| ManifestError::InvalidField("pages"))?;
            Ok(())
        }
        "form" => {
            if object
                .get("schema")
                .is_some_and(|schema| !schema.is_object())
            {
                return Err(ManifestError::InvalidField("pages"));
            }
            if object
                .get("uiHints")
                .is_some_and(|hints| !hints.is_object())
            {
                return Err(ManifestError::InvalidField("pages"));
            }
            Ok(())
        }
        "connection" => Ok(()),
        "list" => {
            let source = object
                .get("optionsFrom")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("pages"))?;
            if source.trim().is_empty() || source.len() > 256 {
                return Err(ManifestError::InvalidField("pages"));
            }
            Ok(())
        }
        "tts" => {
            let action_type = object
                .get("actionType")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("pages"))?;
            if action_type.trim().is_empty() || action_type.len() > 128 {
                return Err(ManifestError::InvalidField("pages"));
            }
            let voices_from = object
                .get("voicesFrom")
                .and_then(Value::as_str)
                .ok_or(ManifestError::InvalidField("pages"))?;
            if voices_from.trim().is_empty() || voices_from.len() > 256 {
                return Err(ManifestError::InvalidField("pages"));
            }
            // Server-side audio outputs are optional: older manifests omit
            // the selector, and the host hides it when the source is absent.
            if let Some(outputs_from) = object.get("outputsFrom") {
                let outputs_from = outputs_from
                    .as_str()
                    .ok_or(ManifestError::InvalidField("pages"))?;
                if outputs_from.trim().is_empty() || outputs_from.len() > 256 {
                    return Err(ManifestError::InvalidField("pages"));
                }
            }
            Ok(())
        }
        _ => Err(ManifestError::InvalidField("pages")),
    }
}

fn validate_localized(value: Option<&Value>) -> Result<(), &'static str> {
    let object = value
        .and_then(Value::as_object)
        .ok_or("localized text must be an object")?;
    let default = object
        .get("default")
        .and_then(Value::as_str)
        .ok_or("localized text needs a default")?;
    if default.trim().is_empty() || default.len() > 1_024 {
        return Err("localized default out of range");
    }
    if let Some(key) = object.get("i18key") {
        let key = key.as_str().ok_or("i18key must be a string")?;
        if key.trim().is_empty() || key.len() > 256 {
            return Err("i18key out of range");
        }
    }
    Ok(())
}

fn is_valid_trigger_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=MAX_EVENT_TYPE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

/// Validate one eventTypes entry from a plugin manifest. Shape errors are
/// reported by the host catalog merge, which skips the entry with a warning.
pub fn validate_event_type(entry: &Value) -> Result<(), ManifestError> {
    let object = entry
        .as_object()
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let event_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    if !is_valid_event_type(event_type) {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    let title = object
        .get("title")
        .and_then(Value::as_object)
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let default = title
        .get("default")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if default.trim().is_empty() || default.len() > 120 {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    if let Some(fields) = object.get("fields") {
        let fields = fields
            .as_array()
            .ok_or(ManifestError::InvalidField("eventTypes"))?;
        if fields.len() > MAX_EVENT_FIELDS {
            return Err(ManifestError::InvalidField("eventTypes"));
        }
        for field in fields {
            validate_event_field(field)?;
        }
    }
    if object
        .get("sample")
        .is_some_and(|sample| !sample.is_object())
    {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    Ok(())
}

fn validate_event_field(field: &Value) -> Result<(), ManifestError> {
    let object = field
        .as_object()
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if path.trim().is_empty() || path.len() > 200 || path.chars().any(char::is_whitespace) {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    if let Some(kind) = object.get("kind") {
        let kind = kind.as_str().unwrap_or_default();
        if !matches!(kind, "text" | "number" | "boolean") {
            return Err(ManifestError::InvalidField("eventTypes"));
        }
    }
    if let Some(options) = object.get("options") {
        let options = options
            .as_array()
            .ok_or(ManifestError::InvalidField("eventTypes"))?;
        if options.len() > MAX_EVENT_OPTIONS {
            return Err(ManifestError::InvalidField("eventTypes"));
        }
        for option in options {
            validate_event_option(option)?;
        }
    }
    Ok(())
}

fn validate_event_option(option: &Value) -> Result<(), ManifestError> {
    let object = option
        .as_object()
        .ok_or(ManifestError::InvalidField("eventTypes"))?;
    let value = object
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    // Empty values are legitimate ("none" options); only bound the length.
    if value.len() > 64 {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    if object.get("label").is_some_and(|label| !label.is_object()) {
        return Err(ManifestError::InvalidField("eventTypes"));
    }
    Ok(())
}
/// Event type names are dotted lowercase: hotkey.pressed, timer.tick.
/// Host namespaces stay reserved so a plugin can never shadow built-in
/// triggers or the internal plugin.emit channel.
pub fn is_valid_event_type(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=MAX_EVENT_TYPE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
        && !RESERVED_EVENT_PREFIXES
            .iter()
            .any(|prefix| value.starts_with(prefix))
}

/// Domain event subscriptions may observe host namespaces, unlike plugin
/// `eventTypes` which are restricted to plugin-owned trigger namespaces.
/// Keep the grammar deliberately small so wildcard routing is deterministic.
pub fn is_valid_event_subscription(value: &str) -> bool {
    if value == "*" {
        return true;
    }
    let topic = value.strip_suffix(".*").unwrap_or(value);
    let bytes = topic.as_bytes();
    (2..=MAX_EVENT_TYPE_LEN).contains(&bytes.len())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
        && (!value.contains('*') || value.ends_with(".*"))
}

pub fn is_valid_plugin_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=128).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[1..].iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

/// Validate a package-relative path before joining it with a plugin root.
pub fn is_safe_relative_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\0') || value.starts_with('/') || value.starts_with('\\')
    {
        return false;
    }
    let normalized = value.replace('\\', "/");
    if normalized.starts_with('/') || normalized.contains(":/") {
        return false;
    }
    let parts: Vec<&str> = normalized.split('/').collect();
    !parts.is_empty()
        && normalized != "."
        && parts.iter().all(|part| !part.is_empty() && *part != "..")
}

pub fn current_platform() -> String {
    match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        platform => platform,
    }
    .to_owned()
}

pub fn current_target() -> String {
    let platform = current_platform();
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        architecture => architecture,
    };
    let abi = match std::env::consts::OS {
        "windows" => "msvc",
        "linux" => "gnu",
        "macos" => "darwin",
        other => other,
    };
    format!("{platform}-{arch}-{abi}")
}
