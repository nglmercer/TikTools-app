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

pub mod assets;
pub mod broker;
#[cfg(test)]
mod smoke;

use std::collections::{HashMap, HashSet};
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

impl PluginWindowState {
    /// `CloseRequested` transition: `Open -> Closing` returns true (the
    /// caller must post the deferred teardown command); `Closing` returns
    /// false (a teardown is already queued).
    fn begin_close(&mut self) -> bool {
        if *self == PluginWindowState::Closing {
            return false;
        }
        *self = PluginWindowState::Closing;
        true
    }
}

/// One ordered teardown step. [`teardown_plan`] pins the destruction
/// sequence as data so tests assert it without a display server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeardownStep {
    DropWebView,
    SyncDisplay,
    DropWindow,
    ForgetWindow,
}

/// Destruction order for one plugin window: the child WebView dies first,
/// then (on Linux) the GDK round-trip flushes its container destroy while
/// the parent destroy is still buffered, and only then is the parent
/// window handle destroyed. When the server already destroyed the parent
/// (`Destroyed` without `CloseRequested`), the dead handle is forgotten
/// instead of destroyed twice.
fn teardown_plan(externally_destroyed: bool) -> [TeardownStep; 3] {
    if externally_destroyed {
        [
            TeardownStep::DropWebView,
            TeardownStep::SyncDisplay,
            TeardownStep::ForgetWindow,
        ]
    } else {
        [
            TeardownStep::DropWebView,
            TeardownStep::SyncDisplay,
            TeardownStep::DropWindow,
        ]
    }
}

struct PluginUiWindow {
    /// `None` only after [`PluginUiWindow::teardown`] has taken a handle:
    /// map-resident entries always hold both live handles.
    webview: Option<WebView>,
    window: Option<Window>,
    topics: HashSet<String>,
    state: PluginWindowState,
}

impl Drop for PluginUiWindow {
    fn drop(&mut self) {
        // `Drop::drop` runs BEFORE field destructors (fields then drop in
        // declaration order), so every handle is taken explicitly in
        // `teardown` instead of relying on field order. Covers every
        // removal path: close, deferred CloseRequested, retain-based
        // teardown, and map drop.
        self.teardown();
    }
}

impl PluginUiWindow {
    fn handle(&self) -> &Window {
        self.window
            .as_ref()
            .expect("map-resident plugin window holds its handle")
    }

    fn view(&self) -> &WebView {
        self.webview
            .as_ref()
            .expect("map-resident plugin window holds its WebView")
    }

    /// Explicit ordered teardown: child WebView, then display sync, then
    /// parent window. Safe to run twice (second run finds `None`s).
    fn teardown(&mut self) {
        for step in teardown_plan(false) {
            self.execute(step);
        }
    }

    /// Drops a window the server already destroyed (no CloseRequested, e.g.
    /// an external kill): the dead parent handle is forgotten instead of
    /// queueing a second destroy for a dead id (which would file the same
    /// stale BadWindow the sync exists to prevent). `self` then drops with
    /// both handles taken, so the normal `Drop` is a no-op: no double
    /// destroy, and no leak on the normal close path.
    fn disown_destroyed(mut self) {
        for step in teardown_plan(true) {
            self.execute(step);
        }
    }

    fn execute(&mut self, step: TeardownStep) {
        match step {
            TeardownStep::DropWebView => {
                if let Some(webview) = self.webview.take() {
                    drop(webview);
                }
            }
            // Must run AFTER the WebView drop and BEFORE the parent
            // window drop: it forces the container destroy ahead of the
            // parent destroy (see `platform::sync_gtk_display`).
            TeardownStep::SyncDisplay => crate::platform::sync_gtk_display(),
            TeardownStep::DropWindow => {
                if let Some(window) = self.window.take() {
                    drop(window);
                }
            }
            TeardownStep::ForgetWindow => {
                std::mem::forget(self.window.take());
            }
        }
    }
}

#[derive(Default)]
pub struct PluginUiWindows {
    windows: HashMap<(String, String), PluginUiWindow>,
}

impl PluginUiWindows {
    /// Opens (or focuses, when already open) the isolated window for one
    /// plugin page. Returns whether a new window was created.
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        &mut self,
        event_loop: &ActiveEventLoop,
        core: &Arc<AppCore>,
        control: &Arc<ControlApi>,
        runtime: &Handle,
        proxy: &EventLoopProxy<DesktopEvent>,
        plugin_id: &str,
        page_id: &str,
    ) -> Result<bool, String> {
        if !is_valid_ui_id(plugin_id) || !is_valid_ui_id(page_id) {
            return Err("invalid plugin or page id".to_owned());
        }
        let key = (plugin_id.to_owned(), page_id.to_owned());
        if let Some(existing) = self.windows.get(&key) {
            if existing.state == PluginWindowState::Closing {
                // A deferred teardown is still queued: finish it now so the
                // fresh window never shares a map entry with the dying one.
                // The queued `ClosePluginUi` then finds nothing and is a
                // harmless no-op.
                self.windows.remove(&key);
            } else {
                existing.handle().focus_window();
                tracing::debug!(
                    plugin = plugin_id,
                    page = page_id,
                    "plugin window already open, focused"
                );
                return Ok(false);
            }
        }
        let target = core
            .plugin_ui_target(plugin_id, page_id)
            .map_err(|error| error.to_string())?;
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title(format!("TikTools — {}", target.name))
                    .with_inner_size(winit::dpi::LogicalSize::new(960_u32, 720_u32))
                    .with_min_inner_size(winit::dpi::LogicalSize::new(640_u32, 480_u32))
                    .with_resizable(true)
                    .with_visible(true),
            )
            .map_err(|error| format!("could not create plugin window: {error}"))?;

        let assets = PluginAssetServer::new(target.id.clone(), Arc::new(target.asset_root.clone()));
        let navigation_plugin = target.id.clone();
        let ipc_plugin = target.id.clone();
        let ipc_page = target.page_id.clone();
        let ipc_proxy = proxy.clone();
        let ipc_runtime = runtime.clone();
        let ipc_control = control.clone();
        let mut builder = WebViewBuilder::new()
            .with_devtools(cfg!(debug_assertions) || cfg!(feature = "devtools"))
            .with_autoplay(true)
            .with_initialization_script(INIT_SCRIPT)
            .with_navigation_handler(move |url| {
                let allowed = allows_navigation(&navigation_plugin, &url);
                if !allowed {
                    tracing::warn!(url = %url, plugin = %navigation_plugin, "blocked plugin WebView navigation");
                }
                allowed
            })
            .with_new_window_req_handler(move |url, _features| {
                tracing::debug!(url = %url, "blocked plugin WebView new-window request");
                wry::NewWindowResponse::Deny
            })
            .with_ipc_handler(move |request| {
                // Size-gated before queueing (same bound the broker
                // enforces): the payload moves to Tokio for handling and
                // the response returns through `PluginUiRespond`.
                let raw = request.body().clone();
                let broker = PluginUiBroker::new(ipc_plugin.clone(), ipc_control.clone());
                let proxy = ipc_proxy.clone();
                let plugin_id = ipc_plugin.clone();
                let page_id = ipc_page.clone();
                ipc_runtime.spawn(async move {
                    let (response, effect) = broker.handle(&raw).await;
                    // Rejections are host-side diagnostics (a page that
                    // only sees `broker error` cannot tell a bad method
                    // from a dead window): log them with owner context.
                    // Responses are small host-generated envelopes.
                    if let Ok(value) = serde_json::from_str::<Value>(&response) {
                        if value.get("ok") == Some(&Value::Bool(false)) {
                            tracing::debug!(
                                plugin = %plugin_id,
                                page = %page_id,
                                error = %value
                                    .get("error")
                                    .and_then(|error| error.as_str())
                                    .unwrap_or("broker error"),
                                "plugin broker rejected native request"
                            );
                        }
                    }
                    let _ = proxy.send_event(DesktopEvent::Command(
                        DesktopCommand::PluginUiRespond {
                            plugin_id: plugin_id.clone(),
                            page_id: page_id.clone(),
                            response,
                        },
                    ));
                    if let Some(effect) = effect {
                        let _ = proxy.send_event(DesktopEvent::Command(
                            DesktopCommand::PluginUiSubscribe {
                                plugin_id,
                                page_id,
                                effect: effect.into(),
                            },
                        ));
                    }
                });
            });
        // Physical bounds from the start (see the main window): some
        // XWayland/KDE sessions report zero screen dimensions, which breaks
        // the logical-default conversion before the WebView attaches.
        let initial_size = window.inner_size();
        builder = builder.with_bounds(Rect {
            position: WryPhysicalPosition::new(0, 0).into(),
            size: WryPhysicalSize::new(initial_size.width.max(1), initial_size.height.max(1))
                .into(),
        });
        builder = builder
            .with_custom_protocol(PLUGIN_ASSET_SCHEME.to_owned(), move |_id, request| {
                assets.respond(request)
            });
        let url = plugin_page_url(&target.id, &target.entry_file, &target.page_id);
        builder = builder.with_url(url.as_str());
        let webview = platform::build_webview(builder, &window)
            .map_err(|error| format!("could not create plugin WebView: {error}"))?;
        window.focus_window();
        self.windows.insert(
            key,
            PluginUiWindow {
                webview: Some(webview),
                window: Some(window),
                topics: HashSet::new(),
                state: PluginWindowState::Open,
            },
        );
        tracing::debug!(
            plugin = plugin_id,
            page = page_id,
            transport = "native",
            "plugin window opened"
        );
        Ok(true)
    }

    /// Tears one window down through the ordered teardown (`Drop`). Runs
    /// outside window-event dispatch (deferred `CloseRequested`, explicit
    /// close requests, teardown topics). Closing a missing window is a
    /// harmless no-op, so a queued close can never double-destroy.
    pub fn close(&mut self, plugin_id: &str, page_id: &str) {
        if self
            .windows
            .remove(&(plugin_id.to_owned(), page_id.to_owned()))
            .is_some()
        {
            tracing::debug!(
                plugin = plugin_id,
                page = page_id,
                "closed plugin UI window"
            );
        } else {
            tracing::debug!(
                plugin = plugin_id,
                page = page_id,
                "plugin window close ignored (not open)"
            );
        }
    }

    /// Closes every window owned by one plugin (lifecycle: the plugin was
    /// stopped, disabled, or uninstalled).
    pub fn close_plugin(&mut self, plugin_id: &str) {
        let before = self.windows.len();
        self.windows.retain(|(owner, _), _| owner != plugin_id);
        let closed = before - self.windows.len();
        if closed > 0 {
            tracing::debug!(
                plugin = plugin_id,
                closed,
                "plugin torn down, windows closed"
            );
        }
    }

    pub fn apply_subscription(
        &mut self,
        plugin_id: &str,
        page_id: &str,
        effect: crate::event::PluginUiSubscription,
    ) {
        let Some(window) = self
            .windows
            .get_mut(&(plugin_id.to_owned(), page_id.to_owned()))
        else {
            return;
        };
        match effect {
            crate::event::PluginUiSubscription::Subscribe(topics) => {
                window.topics.extend(topics);
            }
            crate::event::PluginUiSubscription::Unsubscribe(topics) => {
                for topic in topics {
                    window.topics.remove(&topic);
                }
            }
        }
    }

    /// Delivers one broker response to the calling window. Late
    /// responses for closed (or closing) windows are ignored safely: the
    /// entry is gone or skipped, so nothing ever evaluates script on a
    /// dead WebView.
    pub fn respond(&self, plugin_id: &str, page_id: &str, response: &str) {
        let Some(window) = self
            .windows
            .get(&(plugin_id.to_owned(), page_id.to_owned()))
            .filter(|entry| entry.state == PluginWindowState::Open)
        else {
            return;
        };
        // The payload is host-generated JSON embedded as an object
        // literal; the init script drops anything off-protocol.
        let script = format!(
            "if (typeof window.__tiktools_broker_push__ === 'function') {{ window.__tiktools_broker_push__({response}); }}"
        );
        if let Err(error) = window.view().evaluate_script(&script) {
            tracing::debug!(%error, plugin = plugin_id, "could not deliver broker response");
        }
    }

    /// Pushes one plugin-scoped domain event to subscribed windows.
    pub fn deliver_event(&self, plugin_id: &str, topic: &str, data: &Value) {
        let envelope = serde_json::json!({
            "apiVersion": broker::BROKER_API_VERSION,
            "event": topic,
            "data": data,
        })
        .to_string();
        for ((owner, _), window) in &self.windows {
            if owner != plugin_id
                || window.state != PluginWindowState::Open
                || !window.topics.contains(topic)
            {
                continue;
            }
            let script = format!(
                "if (typeof window.__tiktools_broker_push__ === 'function') {{ window.__tiktools_broker_push__({envelope}); }}"
            );
            if let Err(error) = window.view().evaluate_script(&script) {
                tracing::debug!(%error, plugin = plugin_id, "could not push plugin event");
            }
        }
    }

    /// Fans host JSON-RPC `event` notifications out to subscribed plugin
    /// windows, and closes windows whose plugin went away. Called with the
    /// same already-queued main-window payload (host-generated, bounded);
    /// unparseable input is ignored — the main window path owns delivery.
    pub fn forward_host_message(&mut self, message: &str) {
        let Some((plugin_id, topic, data)) = parse_host_notification(message) else {
            return;
        };
        if is_teardown_topic(&topic) {
            self.close_plugin(&plugin_id);
        } else {
            self.deliver_event(&plugin_id, &topic, &data);
        }
    }

    /// Routes a window event to plugin windows. Returns true when a plugin
    /// window consumed it.
    ///
    /// `CloseRequested` never destroys handles re-entrantly: the entry is
    /// marked `Closing`, hidden for immediate feedback, and torn down by a
    /// deferred `ClosePluginUi` command outside window-event dispatch.
    pub fn on_window_event(
        &mut self,
        proxy: &EventLoopProxy<DesktopEvent>,
        window_id: WindowId,
        event: &WindowEvent,
    ) -> bool {
        let key = self
            .windows
            .iter()
            .find(|(_, window)| {
                window
                    .window
                    .as_ref()
                    .is_some_and(|handle| handle.id() == window_id)
            })
            .map(|(key, _)| key.clone());
        let Some(key) = key else {
            return false;
        };
        match event {
            WindowEvent::CloseRequested => {
                let Some(entry) = self.windows.get_mut(&key) else {
                    return true;
                };
                if !entry.state.begin_close() {
                    // Duplicate request while a teardown is already queued.
                    return true;
                }
                entry.handle().set_visible(false);
                tracing::debug!(
                    plugin = %key.0,
                    page = %key.1,
                    "plugin UI window close requested; teardown deferred"
                );
                let _ = proxy.send_event(DesktopEvent::Command(DesktopCommand::ClosePluginUi {
                    plugin_id: key.0.clone(),
                    page_id: key.1.clone(),
                }));
            }
            WindowEvent::Destroyed => {
                // The server destroyed this window without a close
                // request: disown the dead handle instead of dropping it.
                // A queued `ClosePluginUi` for the same entry then finds
                // nothing and is a harmless no-op: no double destroy.
                tracing::debug!(
                    plugin = %key.0,
                    page = %key.1,
                    "plugin window destroyed by server"
                );
                if let Some(entry) = self.windows.remove(&key) {
                    entry.disown_destroyed();
                }
            }
            WindowEvent::Resized(size) => {
                // Closing windows ignore resizes: their WebView is about
                // to be torn down by the deferred close.
                if let Some(window) = self
                    .windows
                    .get(&key)
                    .filter(|entry| entry.state == PluginWindowState::Open)
                {
                    let bounds = Rect {
                        position: WryPhysicalPosition::new(0, 0).into(),
                        size: WryPhysicalSize::new(size.width.max(1), size.height.max(1)).into(),
                    };
                    if let Err(error) = window.view().set_bounds(bounds) {
                        tracing::debug!(%error, "could not resize plugin WebView");
                    }
                }
            }
            _ => {}
        }
        true
    }
}

/// Topics that tear down a plugin's windows: the runtime went away
/// (stop covers disable; uninstall covers removal), so no window may
/// outlive it.
fn is_teardown_topic(topic: &str) -> bool {
    matches!(topic, "plugin.uninstalled" | "plugin.stopped")
}

/// Extracts `(plugin_id, topic, data)` from a host `event` notification.
/// Only `plugin.*` topics with a string `pluginId` route to plugin
/// windows; everything else (including gap markers) is ignored.
fn parse_host_notification(message: &str) -> Option<(String, String, Value)> {
    if !message.contains("\"method\":\"event\"") && !message.contains("\"method\": \"event\"") {
        return None;
    }
    let value: Value = serde_json::from_str(message).ok()?;
    let params = value.get("params")?;
    let topic = params.get("topic")?.as_str()?;
    if !topic.starts_with("plugin.") {
        return None;
    }
    let data = params.get("data")?.clone();
    let plugin_id = data.get("pluginId")?.as_str()?.to_owned();
    Some((plugin_id, topic.to_owned(), data))
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
const INIT_SCRIPT: &str = r#"(function () {
  'use strict';
  if (window.tiktools !== undefined) return;
  var ipc = (window.ipc && typeof window.ipc.postMessage === 'function') ? window.ipc : null;
  function unavailable() { return Promise.reject(new Error('plugin host is unavailable')); }
  if (!ipc) {
    window.__tiktools_transport__ = 'unavailable';
    window.tiktools = {
      settings: { get: unavailable, set: unavailable },
      actions: { execute: unavailable },
      options: { get: unavailable },
      events: { subscribe: function () { return function () {}; } },
      host: { locale: unavailable, theme: unavailable }
    };
    return;
  }
  window.__tiktools_transport__ = 'native';
  var sequence = 0;
  var pending = new Map();
  var listeners = new Map();
  function fire(topic, data) {
    var watchers = listeners.get(topic);
    if (!watchers) return;
    watchers.slice().forEach(function (listener) {
      try { listener(topic, data); } catch (listenerError) { /* listener bugs must not break the bridge */ }
    });
  }
  window.__tiktools_broker_push__ = function (envelope) {
    if (!envelope || envelope.apiVersion !== 1) return;
    if (typeof envelope.event === 'string') {
      fire(envelope.event, envelope.data);
      return;
    }
    if (typeof envelope.id !== 'string') return;
    var entry = pending.get(envelope.id);
    if (!entry) return;
    pending.delete(envelope.id);
    if (envelope.ok === true) entry.resolve(envelope.result);
    else entry.reject(new Error(typeof envelope.error === 'string' ? envelope.error : 'broker error'));
  };
  function call(method, params) {
    sequence += 1;
    var id = 'plg-' + sequence;
    return new Promise(function (resolve, reject) {
      pending.set(id, { resolve: resolve, reject: reject });
      ipc.postMessage(JSON.stringify({ apiVersion: 1, id: id, method: method, params: params || {} }));
    });
  }
  window.tiktools = {
    settings: {
      get: function () { return call('settings.get', {}); },
      set: function (values) { return call('settings.set', { values: values || {} }); }
    },
    actions: {
      execute: function (action, config) { return call('actions.execute', { action: action, config: config || {} }); }
    },
    options: {
      get: function (source, refresh) { return call('options.get', { source: source, refresh: refresh === true }); }
    },
    events: {
      subscribe: function (topics, listener) {
        var names = Array.isArray(topics) ? topics : [topics];
        names.forEach(function (topic) {
          if (!listeners.has(topic)) listeners.set(topic, []);
          listeners.get(topic).push(listener);
        });
        call('events.subscribe', { topics: names }).catch(function () { /* host logs the rejection */ });
        return function unsubscribe() {
          names.forEach(function (topic) {
            var watchers = listeners.get(topic);
            if (!watchers) return;
            var index = watchers.indexOf(listener);
            if (index >= 0) watchers.splice(index, 1);
            if (watchers.length === 0) {
              listeners.delete(topic);
              call('events.unsubscribe', { topics: [topic] }).catch(function () {});
            }
          });
        };
      }
    },
    host: {
      locale: function () { return call('host.locale', {}); },
      theme: function () { return call('host.theme', {}); }
    }
  };
})();"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_ids_follow_the_identifier_rule() {
        assert!(is_valid_ui_id("sonicboom.server"));
        assert!(is_valid_ui_id("plugin-a_b.c"));
        assert!(!is_valid_ui_id(""));
        assert!(!is_valid_ui_id("9lives"));
        assert!(!is_valid_ui_id("../escape"));
        assert!(!is_valid_ui_id("has space"));
        assert!(!is_valid_ui_id(&"x".repeat(129)));
    }

    #[test]
    fn host_notifications_route_by_plugin_topic() {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": {
                "topic": "plugin.event",
                "data": { "pluginId": "owner", "eventType": "speech.state", "event": {} },
            },
        })
        .to_string();
        let (plugin_id, topic, data) = parse_host_notification(&message).expect("routes");
        assert_eq!(plugin_id, "owner");
        assert_eq!(topic, "plugin.event");
        assert_eq!(data["eventType"], "speech.state");

        // Non-plugin topics never reach plugin windows.
        let other = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": { "topic": "live.event", "data": { "pluginId": "owner" } },
        })
        .to_string();
        assert!(parse_host_notification(&other).is_none());
        // RPC responses and malformed input are ignored.
        assert!(parse_host_notification(r#"{"type":"rpc-response"}"#).is_none());
        assert!(parse_host_notification("not json").is_none());
    }

    #[test]
    fn teardown_topics_close_windows_without_affecting_delivery() {
        assert!(is_teardown_topic("plugin.uninstalled"));
        assert!(is_teardown_topic("plugin.stopped"));
        assert!(!is_teardown_topic("plugin.started"));
        assert!(!is_teardown_topic("plugin.event"));
        assert!(!is_teardown_topic("plugin.settings-changed"));
        // Forwarding without windows is a no-op, never a panic.
        let mut windows = PluginUiWindows::default();
        windows.forward_host_message(
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "event",
                "params": {
                    "topic": "plugin.uninstalled",
                    "data": {"pluginId": "gone.plugin"},
                },
            })
            .to_string(),
        );
    }

    #[test]
    fn init_script_installs_the_narrow_surface_only() {
        // The transport is read but never deleted or overwritten: on
        // WebKitGTK `window.ipc` is a non-configurable property installed
        // before init scripts, so touching it throws in strict mode and
        // aborts the whole script (leaving `window.tiktools` missing).
        assert!(!INIT_SCRIPT.contains("delete window.ipc"));
        assert!(!INIT_SCRIPT.contains("window.ipc = "));
        assert!(!INIT_SCRIPT.contains("window.ipc= "));
        assert!(INIT_SCRIPT.contains("window.ipc && typeof window.ipc.postMessage"));
        assert!(INIT_SCRIPT.contains("window.tiktools = {"));
        assert!(INIT_SCRIPT.contains("apiVersion: 1"));
        assert!(INIT_SCRIPT.contains("__tiktools_broker_push__"));
        // Transport diagnostics for native debugging (see also the Rust
        // `transport = "native"` open log and broker rejection logs).
        assert!(INIT_SCRIPT.contains("__tiktools_transport__"));
        assert!(INIT_SCRIPT.contains("'native'"));
        assert!(INIT_SCRIPT.contains("'unavailable'"));
    }

    #[test]
    fn teardown_plan_destroys_the_child_before_the_parent() {
        // Pure pin of the destruction order: the child WebView dies
        // first, the display sync flushes its destroy, and only then is
        // the parent window handle destroyed. This orders the Rust calls;
        // it cannot prove the X11/GTK behavior, which the native smoke
        // test (`smoke::native_open_close_cycles`) exercises for real.
        assert_eq!(
            teardown_plan(false),
            [
                TeardownStep::DropWebView,
                TeardownStep::SyncDisplay,
                TeardownStep::DropWindow,
            ]
        );
        // A server-destroyed parent is forgotten, never destroyed twice —
        // but the WebView still drops first with the same sync.
        assert_eq!(
            teardown_plan(true),
            [
                TeardownStep::DropWebView,
                TeardownStep::SyncDisplay,
                TeardownStep::ForgetWindow,
            ]
        );
    }

    #[test]
    fn close_requests_queue_exactly_one_teardown() {
        let mut state = PluginWindowState::Open;
        assert!(state.begin_close());
        assert_eq!(state, PluginWindowState::Closing);
        // A duplicate request while closing queues nothing.
        assert!(!state.begin_close());
        assert_eq!(state, PluginWindowState::Closing);
    }

    #[test]
    fn manager_close_paths_are_harmless_without_windows() {
        let mut windows = PluginUiWindows::default();
        // Double closes, missing windows, and late broker traffic never
        // panic, recreate entries, or evaluate script anywhere.
        windows.close("ghost.plugin", "main");
        windows.close("ghost.plugin", "main");
        windows.close_plugin("ghost.plugin");
        windows.close_plugin("ghost.plugin");
        windows.respond("ghost.plugin", "main", r#"{"ok":true}"#);
        windows.apply_subscription(
            "ghost.plugin",
            "main",
            crate::event::PluginUiSubscription::Subscribe(vec!["plugin.event".to_owned()]),
        );
        windows.deliver_event("ghost.plugin", "plugin.event", &Value::Null);
        windows.forward_host_message(
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "event",
                "params": {
                    "topic": "plugin.stopped",
                    "data": {"pluginId": "ghost.plugin"},
                },
            })
            .to_string(),
        );
        assert!(windows.windows.is_empty());
    }
}
