//! Native window events, keyboard shortcuts and visibility. Plugin windows are considered
//! before the main-window identity filter.

use super::super::{DesktopApp, StartupState};
use crate::platform;
use std::sync::atomic::Ordering;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::WindowId};

impl DesktopApp {
    pub(super) fn on_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        use winit::event::ElementState;
        use winit::keyboard::{KeyCode, PhysicalKey};
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }
        // Parent-focused shortcuts. When the page itself holds keyboard
        // focus (the common Linux case: the child WebView owns focus), the
        // Winit window never sees the key and the init script handles it
        // instead (F5/Ctrl+R in-page reload, F12/Ctrl+Shift+I inspector
        // request). F12 is the conventional inspector shortcut. WebView2
        // also handles it natively when devtools are enabled; this covers
        // the other backends and guarantees the shortcut exists.
        match event.physical_key {
            PhysicalKey::Code(KeyCode::F12) => self.open_devtools(),
            PhysicalKey::Code(KeyCode::F5) => self.reload_webview(),
            _ => {}
        }
    }

    #[cfg(any(debug_assertions, feature = "devtools"))]
    pub(super) fn open_devtools(&self) {
        if let Some(webview) = self.webview.as_ref() {
            webview.open_devtools();
        }
    }

    /// Release builds without the `devtools` feature have no inspector API;
    /// the shortcut and tray item stay compiled but are intentional no-ops.
    #[cfg(not(any(debug_assertions, feature = "devtools")))]
    pub(super) fn open_devtools(&self) {}

    pub(super) fn set_window_visible(&self, visible: bool) {
        self.emitter.projection.set_host_visible(visible);
        self.emitter.send_value_generation(
            &serde_json::json!({"jsonrpc": "2.0", "method": "event", "params": {
                "topic": "ui.host.visibility", "data": {"visible": visible},
            }}),
            self.emitter.generation.load(Ordering::Acquire),
        );
        if let Some(webview) = self.webview.as_ref() {
            if let Err(error) = webview.set_visible(visible) {
                tracing::debug!(%error, visible, "could not change WebView visibility");
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.set_visible(visible);
        }
    }

    pub(super) fn restore_window(&mut self) {
        if self.shutting_down {
            return;
        }
        if self.startup_state != StartupState::Ready {
            self.pending_activation = true;
            return;
        }
        self.pending_activation = false;
        if let Some(window) = self.window.as_ref() {
            window.set_minimized(false);
        }
        self.set_window_visible(true);
        if let Some(window) = self.window.as_ref() {
            window.focus_window();
        }
    }

    pub(super) fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .plugin_ui
            .on_window_event(&self.proxy, window_id, &event)
        {
            return;
        }
        if self
            .window
            .as_ref()
            .is_some_and(|window| window.id() != window_id)
        {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.tray.is_some() {
                    tracing::debug!("window close requested; hiding TikTools in the tray");
                    self.set_window_visible(false);
                } else {
                    tracing::debug!("window close requested without a tray; shutting down");
                    self.shutdown(event_loop);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.on_keyboard_input(event),
            WindowEvent::Resized(size) => self.resize_webview(size),
            WindowEvent::Focused(gained) => {
                tracing::debug!(gained, "desktop window focus changed");
                if platform::window_focus_action(gained)
                    == platform::WindowFocusAction::FocusWebview
                {
                    if let Some(webview) = self.webview.as_ref() {
                        if let Err(error) = webview.focus() {
                            tracing::debug!(%error, "could not focus WebView");
                        }
                    }
                }
            }
            WindowEvent::Destroyed => {
                tracing::debug!(tray = self.tray.is_some(), "window was destroyed");
                if let Some(webview) = self.webview.take() {
                    platform::prepare_webview_teardown(&webview);
                    drop(webview);
                }
                platform::sync_gtk_display();
                // The server already destroyed this window: dropping the
                // handle would queue a second destroy for a dead id (stale
                // BadWindow filed where Winit's IME path panics on it).
                // Leak the handle; the tray path below recreates on demand.
                self.emitter.advance_generation();
                if let Some(window) = self.window.take() {
                    platform::record_external_destroy();
                    std::mem::forget(window);
                }
                if self.tray.is_none() {
                    self.shutdown(event_loop);
                }
            }
            _ => {}
        }
    }
}
