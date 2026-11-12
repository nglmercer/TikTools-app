use super::*;

impl PluginUiWindows {
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
                    window_id: Some(window_id),
                    owner: Some(entry.owner),
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
            WindowEvent::KeyboardInput { event, .. } => {
                // Parent-focused shortcuts (the page-focused case is
                // covered by the init script). Closing windows ignore
                // keys: their WebView is about to be torn down.
                use winit::event::ElementState;
                use winit::keyboard::{KeyCode, PhysicalKey};
                if event.state == ElementState::Pressed && !event.repeat {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::F12) => {
                            self.open_devtools_for_window(window_id);
                        }
                        PhysicalKey::Code(KeyCode::F5) => {
                            if let Some(window) = self
                                .windows
                                .get_mut(&key)
                                .filter(|entry| entry.state == PluginWindowState::Open)
                            {
                                window.output.discard_pending();
                                if let Err(error) = window.view().reload() {
                                    tracing::debug!(%error, "plugin WebView manual reload failed");
                                }
                            }
                        }
                        _ => {}
                    }
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
            WindowEvent::Focused(gained) => {
                tracing::debug!(plugin = %key.0, page = %key.1, gained, "plugin window focus changed");
                // Closing windows ignore focus: their WebView is about to
                // be torn down by the deferred close.
                if platform::window_focus_action(*gained)
                    == platform::WindowFocusAction::FocusWebview
                {
                    if let Some(window) = self
                        .windows
                        .get(&key)
                        .filter(|entry| entry.state == PluginWindowState::Open)
                    {
                        if let Err(error) = window.view().focus() {
                            tracing::debug!(%error, "could not focus plugin WebView");
                        }
                    }
                }
            }
            _ => {}
        }
        true
    }
}
