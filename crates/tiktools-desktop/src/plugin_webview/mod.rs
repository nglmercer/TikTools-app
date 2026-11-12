//! Isolated per-plugin WebViews and their restricted broker.
//!
//! A webview-mode plugin page opens in its own native window, served from
//! the plugin's built UI directory over `tiktools-plugin://` with a strict
//! CSP (see [`assets`]). An initialization script installs the narrow
//! `window.tiktools` surface on top of Wry's `window.ipc` transport before
//! page code runs; the transport itself is left untouched (on WebKitGTK it
//! is a non-configurable property installed before init scripts, so it
//! cannot be deleted). The security boundary is the Rust handler behind
//! that transport: this window's `window.ipc` reaches only the restricted
//! [`broker::PluginUiBroker`], never the main window's full IPC router.
//! All broker calls (see [`broker`]) are ownership-scoped to the window's
//! bound plugin id.
//!
//! The manager owns every plugin window; the main-window lifecycle routes
//! window events here and issues open/close/respond commands. Teardown is
//! explicit and ordered (child WebView, then display sync, then parent
//! window); see [`PluginUiWindow::teardown`].

mod broker_delivery;
mod events;
mod lifecycle;
mod opening;
mod output;
mod recovery;

#[cfg(test)]
use broker_delivery::apply_topics;
#[cfg(test)]
use lifecycle::{close_owner_matches, teardown_plan};
#[cfg(test)]
use output::{is_teardown_topic, parse_host_notification};

pub mod assets;
pub mod broker;
#[cfg(test)]
mod smoke;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::Value;
use tiktools_control_api::ControlApi;
use tiktools_core::AppCore;
use tokio::runtime::Handle;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};
use wry::dpi::{PhysicalPosition as WryPhysicalPosition, PhysicalSize as WryPhysicalSize};
use wry::{Rect, WebView, WebViewBuilder};

use crate::event::{DesktopCommand, DesktopEvent};
use crate::platform;
use crate::window::reload::{
    failure_reasons, PluginBridgeState, PluginPageSupervisor, RecoveryDecision, WebviewLifecycle,
};
use assets::{allows_navigation, plugin_page_url, PluginAssetServer, PLUGIN_ASSET_SCHEME};
use broker::PluginUiBroker;

/// Mirrors the core identifier rule (`helpers::is_identifier`): plugin and
/// page ids in open requests must be strict identifiers before any lookup.
pub fn is_valid_ui_id(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some('a'..='z' | 'A'..='Z' | '_'))
        && value.len() <= 128
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

static NEXT_WINDOW_OWNER: AtomicU64 = AtomicU64::new(1);
static NEXT_PAGE_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Lifecycle of one native plugin window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PluginWindowState {
    /// Live: events, resizes, and broker responses flow normally.
    Open,
    /// `CloseRequested` was received: a deferred `ClosePluginUi` command
    /// will tear the entry down outside the window-event handler. Resizes
    /// and broker responses are ignored meanwhile; a racing `Destroyed`
    /// takes the disown path instead. A second close is never queued.
    Closing,
}

/// One ordered teardown step. [`lifecycle::teardown_plan`] pins the destruction
/// sequence as data so tests assert it without a display server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeardownStep {
    DropWebView,
    SyncDisplay,
    DropWindow,
    ForgetWindow,
}

struct PluginUiWindow {
    /// `None` only after [`PluginUiWindow::teardown`] has taken a handle:
    /// map-resident entries always hold both live handles.
    webview: Option<WebView>,
    window: Option<Window>,
    topics: HashSet<String>,
    state: PluginWindowState,
    ipc_shutdown: tokio::sync::watch::Sender<bool>,
    ipc_slots: Arc<tokio::sync::Semaphore>,
    output: crate::window::outbox::WebviewOutbox,
    generation: Arc<AtomicU64>,
    output_generation: u64,
    owner: u64,
    /// Explicit lifecycle for this pop-out: Loading until the first broker
    /// round-trip proves the init script installed `window.tiktools`.
    lifecycle: PluginPageSupervisor,
}

/// Verifies the initialization script is available before a plugin window
/// is built. The bridge check is post-navigation (first broker round-trip),
/// but a missing or truncated script must fail fast with a diagnostic
/// instead of opening a blank page that only times out later.
pub fn init_script_available() -> bool {
    !INIT_SCRIPT.is_empty()
        && INIT_SCRIPT.contains("window.tiktools")
        && INIT_SCRIPT.contains("__tiktools_broker_push__")
        && INIT_SCRIPT.contains("apiVersion: 1")
        && !INIT_SCRIPT.contains("delete window.ipc")
}

#[derive(Default)]
pub struct PluginUiWindows {
    windows: HashMap<(String, String), PluginUiWindow>,
    flush_cursor: usize,
    evaluate_calls: u64,
    evaluate_bytes: u64,
    evaluate_errors: u64,
    /// Auto-recreate budget per plugin page: at most one automatic reopen
    /// after a timed-out window is closed. Manual reopens reset the count
    /// by going through `open` (which clears the entry on success).
    recreate_budget: HashMap<(String, String), u32>,
    pending_recreates: Vec<(String, String)>,
}

/// Runs before any page script: installs the narrow `window.tiktools`
/// surface on top of Wry's `window.ipc` transport. The transport itself
/// is deliberately left untouched: on WebKitGTK it is installed (via
/// `Object.defineProperty`, non-writable and non-configurable) *before*
/// initialization scripts run, so deleting or overwriting it throws in
/// strict mode and aborts the whole script — leaving `window.tiktools`
/// uninstalled. The security boundary is not the absence of the
/// transport but the Rust handler behind it: this window's `window.ipc`
/// reaches only [`broker::PluginUiBroker`], never the main router.
const INIT_SCRIPT: &str = include_str!("init.js");

#[cfg(test)]
mod tests;
