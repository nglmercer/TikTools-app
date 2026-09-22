//! Host-side OBS widget operations: status probing and clipboard copy.
//!
//! The WebView must never see raw gateway secrets: plugin settings arrive
//! redacted (`••••••••`), so the Widgets tab cannot build OBS URLs itself.
//! These operations run entirely on the host — raw settings are loaded
//! internally, the widget credential is embedded into the URL, and only a
//! redacted state (or an OS clipboard write) crosses back to the UI. Tokens
//! never appear in logs, errors, or RPC results.

use super::OperationError;
use crate::services::SECRET_SETTING_PLACEHOLDER;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

pub const GATEWAY_PLUGIN_ID: &str = "tiktools.event-gateway";
pub const GATEWAY_DEFAULT_PORT: u16 = 17_452;

/// OBS widget names served by the gateway (`/widgets/<kind>/`). Keep in sync
/// with the gateway route allowlist and `GATEWAY_WIDGET_KINDS`.
const VALID_WIDGETS: &[&str] = &["follow", "gift", "chat", "share", "subscribe"];

/// Loopback probe budget per connection phase. The gateway answers from
/// memory on loopback, so anything slower means it is not there.
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WidgetsState {
    /// The Event Gateway plugin is not installed/discovered.
    Missing,
    /// Installed but disabled (or unavailable).
    Disabled,
    /// Enabled but its process is not running.
    Stopped,
    /// Process running, loopback port not accepting yet (mid-startup).
    Starting,
    /// Running (or ready to run) but no usable widget credential is stored.
    CredentialUnavailable,
    /// Gateway answers but `/widgets/*` is not served (assets missing).
    AssetsMissing,
    /// Reachable socket but no valid gateway HTTP answers.
    Unreachable,
    /// Credential present, health OK, every widget bundle served.
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WidgetsStatusResult {
    pub state: WidgetsState,
    pub port: u16,
    /// Human-safe detail; never contains secrets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WidgetsCopyResult {
    pub ok: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeOutcome {
    /// Nothing listening (yet): refused, timed out, or DNS-less failure.
    NotListening,
    /// Connected, but no valid gateway HTTP answer.
    BadGateway,
    /// Health OK, but at least one widget bundle 404s.
    AssetsMissing,
    /// Health OK and both widget bundles served.
    Ready,
}

impl AppCore {
    // ------------------------------------------------------------------
    // Widgets.
    // ------------------------------------------------------------------

    fn gateway_activation(&self) -> (bool, bool) {
        // Same defaults as the readiness hot path: plugins without a
        // persisted row count as installed and enabled.
        recover_rwlock_read(&self.plugin_state.activation, "plugin activation")
            .get(GATEWAY_PLUGIN_ID)
            .map(|state| (state.installed, state.enabled))
            .unwrap_or((true, true))
    }

    fn gateway_port(&self, values: &Value) -> u16 {
        values
            .get("port")
            .and_then(Value::as_u64)
            .and_then(|port| u16::try_from(port).ok())
            .filter(|port| *port != 0)
            .unwrap_or(GATEWAY_DEFAULT_PORT)
    }

    fn gateway_bind(&self, values: &Value) -> std::net::IpAddr {
        values
            .get("bind")
            .and_then(Value::as_str)
            .and_then(|bind| bind.parse::<std::net::IpAddr>().ok())
            .filter(|bind| bind.is_loopback())
            .unwrap_or_else(|| {
                GATEWAY_LOOPBACK_V4
                    .parse()
                    .expect("loopback literal is valid")
            })
    }

    /// Raw (unredacted) widget credential, host-internal only. The
    /// redaction placeholder is never a valid credential, even if it
    /// somehow ends up stored.
    fn gateway_widget_credential(&self, values: &Value) -> Option<String> {
        let token = values
            .get("widgetToken")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|token| !token.is_empty())?;
        if token == SECRET_SETTING_PLACEHOLDER || token.len() > 4096 {
            return None;
        }
        Some(token.to_owned())
    }

    /// Pure state resolution over already-gathered inputs, so every
    /// Widgets tab state is unit-testable without processes or sockets.
    fn resolve_widgets_state(
        discovered: bool,
        installed: bool,
        enabled: bool,
        available: bool,
        running: bool,
        credential_present: bool,
        probe: Option<ProbeOutcome>,
    ) -> WidgetsState {
        if !discovered || !installed {
            return WidgetsState::Missing;
        }
        if !enabled || !available {
            return WidgetsState::Disabled;
        }
        if !running {
            return WidgetsState::Stopped;
        }
        if !credential_present {
            // The gateway generates credentials on its first start; a
            // running gateway without one needs a restart (or its settings
            // were just reset).
            return WidgetsState::CredentialUnavailable;
        }
        match probe {
            None | Some(ProbeOutcome::NotListening) => WidgetsState::Starting,
            Some(ProbeOutcome::BadGateway) => WidgetsState::Unreachable,
            Some(ProbeOutcome::AssetsMissing) => WidgetsState::AssetsMissing,
            Some(ProbeOutcome::Ready) => WidgetsState::Ready,
        }
    }

    pub async fn widgets_status(&self) -> Result<WidgetsStatusResult, OperationError> {
        let plugin = self.plugins.get(GATEWAY_PLUGIN_ID);
        let Some(plugin) = plugin else {
            return Ok(WidgetsStatusResult {
                state: WidgetsState::Missing,
                port: GATEWAY_DEFAULT_PORT,
                error: Some("the Event Gateway plugin is not installed".to_owned()),
            });
        };
        let values = self
            .capabilities
            .load_plugin_settings_raw(&plugin.manifest)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let port = self.gateway_port(&values);
        let bind = self.gateway_bind(&values);
        let (installed, enabled) = self.gateway_activation();
        let running = self.plugins.is_running(GATEWAY_PLUGIN_ID);
        let credential_present = self.gateway_widget_credential(&values).is_some();
        let probe = if installed && enabled && plugin.available && running && credential_present {
            Some(probe_gateway(bind, port).await)
        } else {
            None
        };
        let state = Self::resolve_widgets_state(
            true,
            installed,
            enabled,
            plugin.available,
            running,
            credential_present,
            probe,
        );
        let error = match state {
            WidgetsState::Ready => None,
            WidgetsState::Missing => Some("the Event Gateway plugin is not installed".to_owned()),
            WidgetsState::Disabled => Some("the Event Gateway plugin is disabled".to_owned()),
            WidgetsState::Stopped => Some("the Event Gateway is stopped".to_owned()),
            WidgetsState::Starting => {
                Some("the Event Gateway is starting; retry in a moment".to_owned())
            }
            WidgetsState::CredentialUnavailable => {
                Some("no widget credential is stored; restart the Event Gateway once".to_owned())
            }
            WidgetsState::AssetsMissing => {
                Some("the gateway does not serve widget assets".to_owned())
            }
            WidgetsState::Unreachable => {
                Some("the Event Gateway is not answering on its loopback port".to_owned())
            }
        };
        Ok(WidgetsStatusResult { state, port, error })
    }

    /// Builds the OBS Browser Source URL and copies it to the OS clipboard.
    /// Only `{ok: true}` (or a typed, secret-free error) returns: the token
    /// never crosses into the caller. Copying needs a stored credential but
    /// deliberately not a running gateway — the URL stays valid, so OBS can
    /// hold it while the gateway (re)starts.
    pub fn widgets_copy_obs_url(&self, widget: &str) -> Result<WidgetsCopyResult, OperationError> {
        if !VALID_WIDGETS.contains(&widget) {
            return Err(OperationError::invalid(
                "widget must be one of: follow, gift, chat, share, subscribe",
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
        let mut url = format!(
            "http://{host}:{port}/widgets/{widget}/#token={}",
            percent_encode_fragment(&credential)
        );
        let key = format!("widgets.design.{widget}");
        let state = self.app_state_get(Some(std::slice::from_ref(&key)))?;
        if let Some(design) = state.get(&key).and_then(|raw| portable_design(raw)) {
            url.push_str("&design=");
            url.push_str(&percent_encode_fragment(&design));
        }
        copy_text_to_clipboard(&url)
            .map_err(|_| OperationError::unavailable("could not access the OS clipboard"))?;
        tracing::debug!(widget, "widgets OBS URL copied to clipboard");
        Ok(WidgetsCopyResult { ok: true })
    }
}

const GATEWAY_LOOPBACK_V4: &str = "127.0.0.1";

/// Copy only known style tokens, never arbitrary application state or CSS.
/// Mirrors the WebView `normalizeDesign` bounds so OBS renders exactly what
/// the builder preview shows.
fn portable_design(raw: &str) -> Option<String> {
    const TEXT_FIELDS: [&str; 7] = [
        "title",
        "streakTitle",
        "name",
        "handle",
        "message",
        "count",
        "diamonds",
    ];
    let source: serde_json::Value = serde_json::from_str(raw).ok()?;
    let mut design = serde_json::Map::new();
    if let Some(fields) = source.get("text").and_then(|value| value.as_object()) {
        let mut text = serde_json::Map::new();
        for key in TEXT_FIELDS {
            if let Some(value) = fields.get(key).and_then(|value| value.as_str()) {
                text.insert(
                    key.to_owned(),
                    serde_json::Value::from(value.chars().take(300).collect::<String>()),
                );
            }
        }
        if !text.is_empty() {
            design.insert("text".to_owned(), serde_json::Value::Object(text));
        }
    }
    for key in ["hiddenText", "textOrder"] {
        if let Some(fields) = source.get(key).and_then(|value| value.as_array()) {
            let mut kept: Vec<serde_json::Value> = Vec::new();
            for field in fields {
                let Some(name) = field.as_str() else {
                    continue;
                };
                if !TEXT_FIELDS.contains(&name)
                    || kept.iter().any(|kept| kept.as_str() == Some(name))
                {
                    continue;
                }
                kept.push(serde_json::Value::from(name));
                if kept.len() >= TEXT_FIELDS.len() {
                    break;
                }
            }
            if !kept.is_empty() {
                design.insert(key.to_owned(), serde_json::Value::Array(kept));
            }
        }
    }
    if let Some(layers) = source.get("layers").and_then(|value| value.as_array()) {
        let mut kept = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for item in layers {
            let Some(layer) = item.as_object() else {
                continue;
            };
            let Some(id) = layer.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            let valid_id = !id.is_empty()
                && id.len() <= 80
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'));
            if !valid_id || ids.contains(id) {
                continue;
            }
            let Some(kind) = layer.get("kind").and_then(|value| value.as_str()) else {
                continue;
            };
            if !matches!(kind, "text" | "avatar" | "art") {
                continue;
            }
            ids.insert(id.to_owned());
            let mut entry = serde_json::Map::new();
            entry.insert("id".to_owned(), serde_json::Value::from(id));
            entry.insert("kind".to_owned(), serde_json::Value::from(kind));
            let name = layer
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or(id);
            entry.insert(
                "name".to_owned(),
                serde_json::Value::from(name.chars().take(80).collect::<String>()),
            );
            if kind == "text" {
                let text = layer
                    .get("text")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                entry.insert(
                    "text".to_owned(),
                    serde_json::Value::from(text.chars().take(300).collect::<String>()),
                );
                if let Some(field) = layer.get("field").and_then(|value| value.as_str()) {
                    if TEXT_FIELDS.contains(&field) {
                        entry.insert("field".to_owned(), serde_json::Value::from(field));
                    }
                }
                if let Some(color) = portable_color(layer.get("color")) {
                    entry.insert("color".to_owned(), serde_json::Value::from(color));
                }
                if let Some(size) = portable_number(layer.get("fontSize"), 8.0, 96.0) {
                    entry.insert("fontSize".to_owned(), serde_json::Value::from(size));
                }
                if let Some(weight) = portable_number(layer.get("fontWeight"), 100.0, 900.0) {
                    entry.insert(
                        "fontWeight".to_owned(),
                        serde_json::Value::from((weight / 100.0).round() * 100.0),
                    );
                }
            } else {
                if let Some(size) = portable_number(layer.get("size"), 16.0, 160.0) {
                    entry.insert("size".to_owned(), serde_json::Value::from(size));
                }
                if let Some(placement) = layer.get("placement").and_then(|value| value.as_str()) {
                    if matches!(placement, "left" | "right" | "top" | "bottom") {
                        entry.insert("placement".to_owned(), serde_json::Value::from(placement));
                    }
                }
            }
            kept.push(serde_json::Value::Object(entry));
        }
        design.insert("layers".to_owned(), serde_json::Value::Array(kept));
    }
    for key in ["background", "textColor", "accent", "borderColor"] {
        if let Some(color) = portable_color(source.get(key)) {
            design.insert(key.to_owned(), serde_json::Value::from(color));
        }
    }
    for (key, min, max) in [
        ("radius", 0.0, 48.0),
        ("borderWidth", 0.0, 8.0),
        ("opacity", 0.0, 100.0),
        ("padding", 0.0, 64.0),
        ("gap", 0.0, 48.0),
        ("width", 240.0, 720.0),
    ] {
        if let Some(number) = portable_number(source.get(key), min, max) {
            design.insert(key.to_owned(), serde_json::Value::from(number));
        }
    }
    if let Some(shadow) = source.get("shadow").and_then(|value| value.as_bool()) {
        design.insert("shadow".to_owned(), serde_json::Value::from(shadow));
    }
    if let Some(align) = source.get("align").and_then(|value| value.as_str()) {
        if matches!(align, "left" | "center" | "right") {
            design.insert("align".to_owned(), serde_json::Value::from(align));
        }
    }
    if let Some(avatar) = source.get("avatar").and_then(|value| value.as_object()) {
        let mut kept = serde_json::Map::new();
        if let Some(visible) = avatar.get("visible").and_then(|value| value.as_bool()) {
            kept.insert("visible".to_owned(), serde_json::Value::from(visible));
        }
        for (key, min, max) in [
            ("size", 16.0, 160.0),
            ("radius", 0.0, 80.0),
            ("borderWidth", 0.0, 8.0),
        ] {
            if let Some(number) = portable_number(avatar.get(key), min, max) {
                kept.insert(key.to_owned(), serde_json::Value::from(number));
            }
        }
        if let Some(color) = portable_color(avatar.get("borderColor")) {
            kept.insert("borderColor".to_owned(), serde_json::Value::from(color));
        }
        if !kept.is_empty() {
            design.insert("avatar".to_owned(), serde_json::Value::Object(kept));
        }
    }
    if let Some(badge) = source.get("badge").and_then(|value| value.as_object()) {
        let mut kept = serde_json::Map::new();
        if let Some(visible) = badge.get("visible").and_then(|value| value.as_bool()) {
            kept.insert("visible".to_owned(), serde_json::Value::from(visible));
        }
        if let Some(color) = portable_color(badge.get("color")) {
            kept.insert("color".to_owned(), serde_json::Value::from(color));
        }
        for (key, min, max) in [("fontSize", 8.0, 32.0), ("letterSpacing", 0.0, 8.0)] {
            if let Some(number) = portable_number(badge.get(key), min, max) {
                kept.insert(key.to_owned(), serde_json::Value::from(number));
            }
        }
        if let Some(weight) = badge.get("fontWeight").and_then(|value| value.as_f64()) {
            let rounded = (weight / 100.0).round() * 100.0;
            kept.insert(
                "fontWeight".to_owned(),
                serde_json::Value::from(rounded.clamp(400.0, 900.0)),
            );
        }
        if !kept.is_empty() {
            design.insert("badge".to_owned(), serde_json::Value::Object(kept));
        }
    }
    (!design.is_empty()).then(|| serde_json::Value::Object(design).to_string())
}

fn portable_color(value: Option<&serde_json::Value>) -> Option<&str> {
    let color = value.and_then(|value| value.as_str())?;
    (matches!(color.len(), 7 | 9)
        && color.starts_with('#')
        && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit))
    .then_some(color)
}

fn portable_number(value: Option<&serde_json::Value>, min: f64, max: f64) -> Option<f64> {
    value.and_then(|value| value.as_f64()).map(|number| {
        if number.is_finite() {
            number.clamp(min, max)
        } else {
            min
        }
    })
}

/// Fragment-encodes a credential the same way browsers do for `#token=…`
/// (unreserved marks stay literal, everything else becomes `%XX`).
fn percent_encode_fragment(value: &str) -> String {
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

fn copy_text_to_clipboard(text: &str) -> Result<(), arboard::Error> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)
}

async fn probe_gateway(bind: std::net::IpAddr, port: u16) -> ProbeOutcome {
    match http_get_status(bind, port, "/health").await {
        Fetch::Unreachable => return ProbeOutcome::NotListening,
        Fetch::Status(Some(200)) => {}
        // Connected, but no valid gateway answer.
        Fetch::Status(_) => return ProbeOutcome::BadGateway,
    }
    for kind in VALID_WIDGETS {
        let path = format!("/widgets/{kind}/");
        if !matches!(
            http_get_status(bind, port, &path).await,
            Fetch::Status(Some(200))
        ) {
            return ProbeOutcome::AssetsMissing;
        }
    }
    ProbeOutcome::Ready
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fetch {
    /// Nothing listening: refused, timed out, or otherwise unreachable.
    Unreachable,
    /// Connected; the status line parsed (`None` when malformed).
    Status(Option<u16>),
}

async fn http_get_status(bind: std::net::IpAddr, port: u16, path: &str) -> Fetch {
    let stream =
        match tokio::time::timeout(PROBE_TIMEOUT, tokio::net::TcpStream::connect((bind, port)))
            .await
        {
            Ok(Ok(stream)) => stream,
            _ => return Fetch::Unreachable,
        };
    let mut stream = stream;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let request = format!("GET {path} HTTP/1.0\r\nHost: {bind}\r\nConnection: close\r\n\r\n");
    if tokio::time::timeout(PROBE_TIMEOUT, stream.write_all(request.as_bytes()))
        .await
        .ok()
        .is_none_or(|result: Result<(), std::io::Error>| result.is_err())
    {
        return Fetch::Status(None);
    }
    let mut body = Vec::with_capacity(1024);
    // Responses are tiny (`/health`, widget heads); cap the read so a
    // rogue listener cannot hold the probe open with a large body.
    let mut limited = stream.take(64 * 1024);
    if tokio::time::timeout(PROBE_TIMEOUT, limited.read_to_end(&mut body))
        .await
        .ok()
        .is_none_or(|result: Result<usize, std::io::Error>| result.is_err())
    {
        return Fetch::Status(None);
    }
    Fetch::Status(parse_http_status(&body))
}

/// Parses `HTTP/1.x <status>` from a raw response head. Anything
/// malformed yields `None` instead of guessing.
fn parse_http_status(bytes: &[u8]) -> Option<u16> {
    let head = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .and_then(|line| std::str::from_utf8(line).ok())?;
    let mut parts = head.split_whitespace();
    if !parts
        .next()
        .is_some_and(|version| version.starts_with("HTTP/"))
    {
        return None;
    }
    parts.next()?.parse::<u16>().ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn copied_design_preserves_text_templates_and_hidden_lines() {
        let result = super::portable_design(
            r#"{"text":{"title":"","message":"Thanks {{name}}!","unknown":"drop"}}"#,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"text":{"title":"","message":"Thanks {{name}}!"}})
        );
    }
    #[test]
    fn copied_design_only_contains_portable_style_tokens() {
        let result = super::portable_design(
            r##"{"background":"#112233aa","accent":"url(evil)","radius":99,"token":"secret"}"##,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"background": "#112233aa", "radius": 48.0})
        );
        assert!(super::portable_design("broken").is_none());
        assert!(super::portable_design("{}").is_none());
    }
    #[test]
    fn copied_design_covers_layout_avatar_badge_and_hidden_text() {
        let result = super::portable_design(
            r##"{"hiddenText":["title","name","title","evil"],"borderColor":"#2a2a33","borderWidth":99,"shadow":false,"opacity":80,"padding":24,"gap":12,"align":"center","width":100,"avatar":{"visible":false,"size":64,"radius":12,"borderColor":"#ffffff","borderWidth":3,"evil":1},"badge":{"color":"#00dce8","fontSize":14,"fontWeight":750,"letterSpacing":2},"align2":"drop"}"##,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "hiddenText": ["title", "name"],
                "borderColor": "#2a2a33",
                "borderWidth": 8.0,
                "shadow": false,
                "opacity": 80.0,
                "padding": 24.0,
                "gap": 12.0,
                "align": "center",
                "width": 240.0,
                "avatar": {"visible": false, "size": 64.0, "radius": 12.0, "borderColor": "#ffffff", "borderWidth": 3.0},
                "badge": {"color": "#00dce8", "fontSize": 14.0, "fontWeight": 800.0, "letterSpacing": 2.0},
            })
        );
        assert!(super::portable_design(r#"{"hiddenText":[]}"#).is_none());
        assert!(super::portable_design(r#"{"hiddenText":"title"}"#).is_none());
        assert!(super::portable_design(r#"{"align":"diagonal"}"#).is_none());
    }
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use tiktools_plugin_loader::{PluginManager, PluginRoot, PluginSource};

    struct NullEmitter;

    impl crate::HostEmitter for NullEmitter {
        fn emit(&self, _message: crate::HostMessage) {}
    }

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "tiktools-widgets-test-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ))
    }

    fn gateway_manifest() -> String {
        r#"{"schemaVersion": 3, "id": "tiktools.event-gateway", "name": "Event Gateway", "version": "1.0.0", "runtime": "declarative", "capabilities": [], "permissions": []}"#.to_owned()
    }

    /// AppCore with a discovered (but never running) gateway plugin and a
    /// temp-rooted capability broker. The caller writes settings files
    /// under `<root>/tiktools.event-gateway/settings.json`.
    fn core_with_gateway() -> (AppCore, std::path::PathBuf) {
        let root = temp_root("core");
        let package = root.join("packages").join("gateway");
        std::fs::create_dir_all(&package).expect("test plugin dir");
        std::fs::write(package.join("plugin.json"), gateway_manifest()).expect("manifest");
        let manager = PluginManager::new(vec![PluginRoot {
            path: root.join("packages"),
            source: PluginSource::Development,
        }]);
        manager.scan().expect("test scan");
        assert!(manager.get(GATEWAY_PLUGIN_ID).is_some());
        let mut core = AppCore::new(Arc::new(NullEmitter));
        core.plugins = Arc::new(manager);
        core.capabilities = Arc::new(crate::services::CapabilityBroker::new(root.join("data")));
        (core, root)
    }

    fn write_settings(root: &std::path::Path, values: &Value) {
        let dir = root.join("data").join(GATEWAY_PLUGIN_ID);
        std::fs::create_dir_all(&dir).expect("settings dir");
        std::fs::write(
            dir.join("settings.json"),
            serde_json::to_vec(values).unwrap(),
        )
        .expect("settings file");
    }

    #[test]
    fn state_resolution_covers_every_widgets_tab_state() {
        use WidgetsState::*;
        // (installed, enabled, available, running, credential, probe) -> state.
        assert_eq!(
            AppCore::resolve_widgets_state(false, true, true, true, true, true, None),
            Missing
        );
        // Present in the catalog but install-flagged off reads as missing.
        assert_eq!(
            AppCore::resolve_widgets_state(true, false, true, true, false, false, None),
            Missing
        );
        assert_eq!(
            AppCore::resolve_widgets_state(true, true, false, true, false, false, None),
            Disabled
        );
        assert_eq!(
            AppCore::resolve_widgets_state(true, true, true, false, false, false, None),
            Disabled
        );
        assert_eq!(
            AppCore::resolve_widgets_state(true, true, true, true, false, false, None),
            Stopped
        );
        // A running gateway without a stored credential cannot mint URLs.
        assert_eq!(
            AppCore::resolve_widgets_state(true, true, true, true, true, false, None),
            CredentialUnavailable
        );
        assert_eq!(
            AppCore::resolve_widgets_state(
                true,
                true,
                true,
                true,
                true,
                true,
                Some(ProbeOutcome::NotListening)
            ),
            Starting
        );
        assert_eq!(
            AppCore::resolve_widgets_state(
                true,
                true,
                true,
                true,
                true,
                true,
                Some(ProbeOutcome::BadGateway)
            ),
            Unreachable
        );
        assert_eq!(
            AppCore::resolve_widgets_state(
                true,
                true,
                true,
                true,
                true,
                true,
                Some(ProbeOutcome::AssetsMissing)
            ),
            AssetsMissing
        );
        assert_eq!(
            AppCore::resolve_widgets_state(
                true,
                true,
                true,
                true,
                true,
                true,
                Some(ProbeOutcome::Ready)
            ),
            Ready
        );
    }

    #[test]
    fn widget_credential_rejects_placeholders_and_blanks() {
        let (core, root) = core_with_gateway();
        for (values, expected) in [
            (serde_json::json!({}), None),
            (serde_json::json!({"widgetToken": ""}), None),
            (serde_json::json!({"widgetToken": "   "}), None),
            // The redaction placeholder is never a valid credential, even
            // if it somehow ends up stored.
            (
                serde_json::json!({"widgetToken": SECRET_SETTING_PLACEHOLDER}),
                None,
            ),
            (serde_json::json!({"widgetToken": "x".repeat(4097)}), None),
            (
                serde_json::json!({"widgetToken": "  ttw_widget  "}),
                Some("ttw_widget".to_owned()),
            ),
        ] {
            assert_eq!(core.gateway_widget_credential(&values), expected);
        }
        assert_eq!(
            core.gateway_port(&serde_json::json!({})),
            GATEWAY_DEFAULT_PORT
        );
        assert_eq!(
            core.gateway_port(&serde_json::json!({"port": 19999})),
            19999
        );
        assert_eq!(
            core.gateway_port(&serde_json::json!({"port": 0})),
            GATEWAY_DEFAULT_PORT
        );
        assert_eq!(
            core.gateway_port(&serde_json::json!({"port": "bad"})),
            GATEWAY_DEFAULT_PORT
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn copy_rejects_bad_widgets_and_missing_credentials_before_clipboard() {
        let (core, root) = core_with_gateway();
        // Unknown widget kinds fail without touching settings or clipboard.
        assert!(core.widgets_copy_obs_url("overlay").is_err());
        assert!(core.widgets_copy_obs_url("").is_err());
        // No stored credential: unavailable, never a placeholder URL.
        write_settings(&root, &serde_json::json!({"port": 17452}));
        let error = core.widgets_copy_obs_url("follow").unwrap_err();
        assert!(error.to_string().contains("credential"));
        assert!(!error.to_string().contains(SECRET_SETTING_PLACEHOLDER));
        write_settings(
            &root,
            &serde_json::json!({"widgetToken": SECRET_SETTING_PLACEHOLDER}),
        );
        assert!(core.widgets_copy_obs_url("gift").is_err());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn copy_with_stored_credential_needs_a_display() {
        let (core, root) = core_with_gateway();
        write_settings(
            &root,
            &serde_json::json!({"widgetToken": "ttw_test_credential"}),
        );
        let result = core.widgets_copy_obs_url("follow");
        if std::env::var_os("TIKTOOLS_TEST_CLIPBOARD").is_none() {
            // Headless CI has no clipboard: the op must fail gracefully
            // (typed error, no panic, no secret in the message), and must
            // not write to any real clipboard. Set
            // TIKTOOLS_TEST_CLIPBOARD=1 to assert the round-trip instead.
            if std::env::var_os("DISPLAY").is_none()
                && std::env::var_os("WAYLAND_DISPLAY").is_none()
            {
                let error = result.unwrap_err();
                assert!(error.to_string().contains("clipboard"));
                assert!(!error.to_string().contains("ttw_test_credential"));
            }
        } else {
            result.expect("clipboard round-trip");
            let mut clipboard = arboard::Clipboard::new().expect("clipboard");
            let pasted = clipboard.get_text().expect("pasted text");
            assert!(pasted.contains("/widgets/follow/#token="));
        }
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn status_reports_lifecycle_states_without_a_process() {
        let (core, root) = core_with_gateway();
        // Discovered, default activation, never running: stopped, with the
        // configured (or default) port echoed for the UI.
        write_settings(&root, &serde_json::json!({"port": 19999}));
        let status = block_on_current_thread(core.widgets_status()).unwrap();
        assert_eq!(status.state, WidgetsState::Stopped);
        assert_eq!(status.port, 19999);
        // Disabled plugin: disabled, no probe attempted.
        core.set_plugin_activation(GATEWAY_PLUGIN_ID, true, false);
        let status = block_on_current_thread(core.widgets_status()).unwrap();
        assert_eq!(status.state, WidgetsState::Disabled);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn status_reports_missing_for_unknown_plugins() {
        let core = AppCore::new(Arc::new(NullEmitter));
        let status = block_on_current_thread(core.widgets_status()).unwrap();
        assert_eq!(status.state, WidgetsState::Missing);
        assert_eq!(status.port, GATEWAY_DEFAULT_PORT);
    }

    #[test]
    fn fragment_encoding_matches_browser_unreserved_set() {
        assert_eq!(percent_encode_fragment("ttw_abc123"), "ttw_abc123");
        assert_eq!(percent_encode_fragment("a b+c"), "a%20b%2Bc");
        assert_eq!(percent_encode_fragment("/#?"), "%2F%23%3F");
        assert_eq!(percent_encode_fragment(""), "");
    }

    #[test]
    fn status_line_parsing_rejects_garbage() {
        assert_eq!(parse_http_status(b"HTTP/1.1 200 OK\r\n"), Some(200));
        assert_eq!(parse_http_status(b"HTTP/1.0 404 Not Found\n"), Some(404));
        assert_eq!(parse_http_status(b""), None);
        assert_eq!(parse_http_status(b"not http at all"), None);
        assert_eq!(parse_http_status(b"HTTP/1.1 nope\r\n"), None);
        assert_eq!(parse_http_status(b"\xff\xfe binary"), None);
    }

    /// Minimal single-threaded block_on so status/probe tests need no
    /// runtime fixture. The futures only touch loopback TCP.
    fn block_on_current_thread<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
            .block_on(future)
    }

    /// Serves canned `(path-prefix, status)` answers on loopback until
    /// `hits` connections complete. Returns the bound port.
    fn serve_canned(responses: Vec<(&'static str, u16)>, hits: usize) -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("loopback listener");
        let port = listener.local_addr().expect("port").port();
        std::thread::spawn(move || {
            listener.set_nonblocking(false).expect("blocking listener");
            for _ in 0..hits {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                use std::io::{Read, Write};
                let mut request = vec![0u8; 4096];
                let Ok(read) = stream.read(&mut request) else {
                    continue;
                };
                let head = String::from_utf8_lossy(&request[..read]);
                let path = head.split_whitespace().nth(1).unwrap_or("/");
                let status = responses
                    .iter()
                    .find(|(prefix, _)| path.starts_with(prefix))
                    .map(|(_, status)| *status)
                    .unwrap_or(404);
                let reason = if status == 200 { "OK" } else { "Not Found" };
                let body = if status == 200 { "ok" } else { "missing" };
                let _ = write!(
                    stream,
                    "HTTP/1.0 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        port
    }

    #[test]
    fn probe_classifies_listeners_and_refusals() {
        // Nothing listening: refused fast (TEST-NET port stays closed).
        let outcome = block_on_current_thread(probe_gateway("127.0.0.1".parse().unwrap(), 9));
        assert_eq!(outcome, ProbeOutcome::NotListening);
        // Healthy gateway shape: /health plus every bundle answers 200.
        let port = serve_canned(vec![("/health", 200), ("/widgets/", 200)], 6);
        let outcome = block_on_current_thread(probe_gateway("127.0.0.1".parse().unwrap(), port));
        assert_eq!(outcome, ProbeOutcome::Ready);
        // Health OK but bundles missing: assets missing, not unreachable.
        let port = serve_canned(vec![("/health", 200)], 2);
        let outcome = block_on_current_thread(probe_gateway("127.0.0.1".parse().unwrap(), port));
        assert_eq!(outcome, ProbeOutcome::AssetsMissing);
        // Wrong answers on a live socket: bad gateway, not starting.
        let port = serve_canned(vec![("/health", 500)], 1);
        let outcome = block_on_current_thread(probe_gateway("127.0.0.1".parse().unwrap(), port));
        assert_eq!(outcome, ProbeOutcome::BadGateway);
    }
}
