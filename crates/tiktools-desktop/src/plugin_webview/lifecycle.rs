use super::*;

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

impl PluginWindowState {
    /// `CloseRequested` transition: `Open -> Closing` returns true (the
    /// caller must post the deferred teardown command); `Closing` returns
    /// false (a teardown is already queued).
    pub(super) fn begin_close(&mut self) -> bool {
        if *self == PluginWindowState::Closing {
            return false;
        }
        *self = PluginWindowState::Closing;
        true
    }
}

impl PluginUiWindow {
    pub(super) fn sync_generation(&mut self) {
        let generation = self.generation.load(Ordering::Acquire);
        if generation != self.output_generation {
            self.output_generation = generation;
            // Navigation Started: reset subscriptions for the previous page
            // before the fresh document can inherit them, drop its stale
            // backlog without replay, and restart the bounded bridge
            // watchdog. Recovery never replays broker calls.
            self.output.discard_pending();
            self.topics.clear();
            let from = self.lifecycle.state();
            self.lifecycle
                .on_navigation_started(std::time::Instant::now());
            tracing::info!(
                generation,
                from = ?from,
                to = ?self.lifecycle.state(),
                "plugin WebView navigation Started: subscriptions reset, bridge watchdog started"
            );
        }
    }

    /// Post-navigation bridge readiness check: the first broker round-trip
    /// on this generation proves the init script installed
    /// `window.tiktools`. Until then delivery stays halted.
    pub(super) fn mark_bridge_ready(&mut self, plugin_id: &str, page_id: &str) {
        if self.lifecycle.bridge() != PluginBridgeState::Ready {
            self.lifecycle.on_bridge_message();
            tracing::info!(
                plugin = plugin_id,
                page = page_id,
                lifecycle = ?self.lifecycle.state(),
                "plugin WebView bridge Ready: init script verified, delivery restored"
            );
        }
    }

    pub(super) fn handle(&self) -> &Window {
        self.window
            .as_ref()
            .expect("map-resident plugin window holds its handle")
    }

    pub(super) fn view(&self) -> &WebView {
        self.webview
            .as_ref()
            .expect("map-resident plugin window holds its WebView")
    }

    /// Explicit ordered teardown: child WebView, then display sync, then
    /// parent window. Safe to run twice (second run finds `None`s).
    pub(super) fn teardown(&mut self) {
        let _ = self.ipc_shutdown.send(true);
        self.ipc_slots.close();
        self.output.discard_pending();
        self.topics.clear();
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
    pub(super) fn disown_destroyed(mut self) {
        let _ = self.ipc_shutdown.send(true);
        self.ipc_slots.close();
        self.output.discard_pending();
        self.topics.clear();
        for step in teardown_plan(true) {
            self.execute(step);
        }
    }

    pub(super) fn execute(&mut self, step: TeardownStep) {
        match step {
            TeardownStep::DropWebView => {
                if let Some(webview) = self.webview.take() {
                    crate::platform::prepare_webview_teardown(&webview);
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
                crate::platform::record_external_destroy();
                std::mem::forget(self.window.take());
            }
        }
    }
}

impl PluginUiWindows {
    /// Tears one window down through the ordered teardown (`Drop`). Runs
    /// outside window-event dispatch (deferred `CloseRequested`, explicit
    /// close requests, teardown topics). Closing a missing window is a
    /// harmless no-op, so a queued close can never double-destroy.
    pub(crate) fn close_if_current(
        &mut self,
        plugin_id: &str,
        page_id: &str,
        window_id: Option<WindowId>,
        owner: Option<u64>,
    ) {
        if window_id.is_some() || owner.is_some() {
            let Some(entry) = self
                .windows
                .get(&(plugin_id.to_owned(), page_id.to_owned()))
            else {
                return;
            };
            if !close_owner_matches(window_id, owner, entry.handle().id(), entry.owner) {
                return;
            }
        }
        self.close(plugin_id, page_id);
    }

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
}

pub(super) fn close_owner_matches(
    window_id: Option<WindowId>,
    owner: Option<u64>,
    current_id: WindowId,
    current_owner: u64,
) -> bool {
    window_id.is_none_or(|id| id == current_id)
        && owner.is_none_or(|generation| generation == current_owner)
}

/// Destruction order for one plugin window: the child WebView dies first,
/// then (on Linux) the GDK round-trip flushes its container destroy while
/// the parent destroy is still buffered, and only then is the parent
/// window handle destroyed. When the server already destroyed the parent
/// (`Destroyed` without `CloseRequested`), the dead handle is forgotten
/// instead of destroyed twice.
pub(super) fn teardown_plan(externally_destroyed: bool) -> [TeardownStep; 3] {
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
