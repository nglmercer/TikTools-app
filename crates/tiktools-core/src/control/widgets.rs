//! Host-side OBS widget operations: status probing and clipboard copy.
//!
//! The WebView must never see raw gateway secrets: plugin settings arrive
//! redacted (`••••••••`), so the Widgets tab cannot build OBS URLs itself.
//! These operations run entirely on the host — raw settings are loaded
//! internally, the widget credential is embedded into the URL, and only a
//! redacted state (or an OS clipboard write) crosses back to the UI. Tokens
//! never appear in logs, errors, or RPC results.

mod design;
mod obs;
mod probe;
mod settings;
mod status;

use design::portable_design;
#[cfg(test)]
use obs::percent_encode_fragment;
#[cfg(test)]
use probe::parse_http_status;
use probe::probe_gateway;

use super::OperationError;
use crate::services::SECRET_SETTING_PLACEHOLDER;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

pub const GATEWAY_PLUGIN_ID: &str = "tiktools.event-gateway";
pub const GATEWAY_DEFAULT_PORT: u16 = 17_452;

/// OBS widget names served by the gateway (`/widgets/<kind>/`). Keep in sync
/// with the gateway route allowlist and `GATEWAY_WIDGET_KINDS`.
const VALID_WIDGETS: &[&str] = &[
    "follow",
    "gift",
    "chat",
    "share",
    "subscribe",
    "likes",
    "ranking",
];

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

const GATEWAY_LOOPBACK_V4: &str = "127.0.0.1";

#[cfg(test)]
mod tests;
