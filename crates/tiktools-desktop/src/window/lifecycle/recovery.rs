//! Document generation, readiness and bounded WebView recovery. Stale handshakes cannot
//! restore delivery to a replaced document.

use super::super::{
    reload::{failure_reasons, RecoveryDecision},
    DesktopApp, StartupState, FRONTEND_STARTUP_TIMEOUT,
};
use crate::platform;
use std::{sync::atomic::Ordering, time::Instant};
use winit::event_loop::ActiveEventLoop;

impl DesktopApp {
    /// User-initiated reload on the UI thread: halt delivery to the current
    /// document immediately, then reload. The resulting Started event
    /// advances the generation and restarts the bounded watchdog, so a
    /// manual refresh flows through the same lifecycle as a programmatic
    /// one. Never touches `AppCore` or user data.
    pub(super) fn reload_webview(&mut self) {
        if self.shutting_down {
            return;
        }
        let Some(webview) = self.webview.as_ref() else {
            return;
        };
        self.page_ready = false;
        self.pending_batch = None;
        self.emitter.clear_batch();
        if let Err(error) = webview.reload() {
            tracing::debug!(%error, "manual WebView reload failed");
        }
    }

    pub(super) fn sync_page_generation(&mut self, generation: u64) {
        if generation == self.ui_generation {
            return;
        }
        let replacing_page = self.ui_generation != 0;
        self.ui_generation = generation;
        // Navigation Started: advance the generation (done by the caller),
        // invalidate pending batches, and stop delivery to the previous
        // document before a single new message can cross documents. The
        // stale backlog is dropped without replay: recovery never retries
        // non-idempotent RPCs, the fresh page resynchronizes through its
        // mount reads instead.
        self.page_ready = false;
        self.pending_batch = None;
        self.emitter.clear_batch();
        // (Re)start the bounded reload watchdog for this document.
        let now = Instant::now();
        self.reload.on_navigation_started(now);
        tracing::info!(
            generation,
            replacing_page,
            lifecycle = ?self.reload.state(),
            "WebView navigation Started: batches invalidated, delivery halted, watchdog started"
        );
        if !self.pending_host_messages.is_empty() {
            // Initial bootstrap snapshots precede the first page's full
            // authoritative fetch; only replacing an existing page creates
            // a discontinuity for a client that was already consuming.
            if replacing_page {
                self.core.record_event_gap();
            }
            self.pending_host_messages.clear_for_reload();
        }
        // A navigation during startup restarts the startup budget as well:
        // the fresh document gets the full handshake window instead of
        // inheriting the previous document's deadline.
        if self.startup_state == StartupState::WebViewLoading {
            self.startup_deadline = Some(now + FRONTEND_STARTUP_TIMEOUT);
        }
    }

    pub(super) fn frontend_ready(&mut self) {
        // Frontend-ready: validate the current generation before marking
        // Ready. A stale handshake from the previous document can never
        // restore delivery or clear a newer document's watchdog.
        let generation = self.emitter.generation.load(Ordering::Acquire);
        if !self
            .reload
            .on_frontend_ready(self.ui_generation, generation)
        {
            tracing::warn!(
                ui_generation = self.ui_generation,
                emitter_generation = generation,
                "stale frontend-ready ignored: generation mismatch"
            );
            return;
        }
        tracing::info!(
            generation = self.ui_generation,
            lifecycle = ?self.reload.state(),
            "WebView frontend-ready: page Ready, IPC delivery restored"
        );
        // A reload/recreate handshake recovers a failed transport only
        // once the fresh page is actually running: stale responses can
        // never mis-resolve the new page's calls, and the mount reads
        // resync authoritative state. Recovery runs before the startup
        // gate below so a mid-startup reload still completes startup.
        // Authoritative resync: the rebooted frontend refetches through
        // its mount reads; the gap marker makes any discontinuity
        // explicit instead of replaying the dropped backlog.
        if self.reload_pending && !self.shutting_down {
            self.reload_pending = false;
            self.pending_host_messages.recover_transport();
            self.domain_mailbox
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .recover();
            self.webview_failed.store(false, Ordering::SeqCst);
            self.core.set_webview_error(None);
            tracing::info!(
                "WebView transport recovered after reload; authoritative resync via mount reads"
            );
        }
        if !self.shutting_down {
            self.page_ready = true;
        }
        if self.shutting_down || self.startup_state == StartupState::Ready {
            return;
        }
        if self.startup_state != StartupState::WebViewLoading {
            tracing::debug!(state = ?self.startup_state, "ignoring frontend-ready outside WebView startup");
            return;
        }
        tracing::debug!("frontend-ready IPC received, main window shown");
        self.startup_state = StartupState::Ready;
        self.startup_deadline = None;
        // Plugin polling is owned by host startup (see `DesktopApp::new`),
        // never by WebView readiness: no lifecycle call here.
        self.set_window_visible(true);
        if let Some(window) = self.window.as_ref() {
            window.focus_window();
        }
    }

    /// Recreates the main WebView on the UI thread after a failed
    /// navigation recovery. Only the WebView handle is rebuilt with the
    /// same URL, navigation policy and bridges: `AppCore`, user data and
    /// the parent window are preserved.
    pub(super) fn recreate_main_webview(&mut self) -> Result<(), String> {
        let window = self
            .window
            .take()
            .ok_or_else(|| "no window to recreate the WebView in".to_owned())?;
        if let Some(old) = self.webview.take() {
            platform::prepare_webview_teardown(&old);
            drop(old);
        }
        platform::sync_gtk_display();
        let result = self.build_main_webview(&window);
        self.window = Some(window);
        match result {
            Ok(webview) => {
                self.webview = Some(webview);
                self.page_ready = false;
                self.pending_batch = None;
                self.emitter.clear_batch();
                self.pending_host_messages.clear_for_reload();
                self.reload_pending = true;
                tracing::info!("WebView recreated on the UI thread; waiting for frontend-ready");
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Polls the bounded reload watchdog on the UI thread. Runs only after
    /// startup completed: the first load keeps the startup dialog path.
    /// Recovery is bounded (one navigation reload, one WebView recreate,
    /// then `Failed`): no infinite reload loops, no unbounded memory, no
    /// replay of non-idempotent RPCs, no `AppCore` restart.
    pub(super) fn poll_reload_watchdog(&mut self) {
        if self.shutting_down || self.startup_state != StartupState::Ready {
            return;
        }
        // Batch-ack timeouts already fail the transport explicitly below;
        // the watchdog here covers the missing `frontend-ready` case.
        let now = Instant::now();
        match self.reload.check_timeout(now) {
            RecoveryDecision::Wait => {}
            RecoveryDecision::ReloadOnce => {
                tracing::warn!(
                    reason = failure_reasons::RELOAD_TIMEOUT,
                    "WebView reload timeout: attempting one controlled navigation recovery"
                );
                self.core.set_webview_error(Some(format!(
                    "WebView reload timed out ({}); attempting navigation recovery",
                    failure_reasons::RELOAD_TIMEOUT
                )));
                self.page_ready = false;
                self.pending_batch = None;
                self.emitter.clear_batch();
                self.pending_host_messages.clear_for_reload();
                match self.webview.as_ref() {
                    Some(webview) => match webview.reload() {
                        Ok(()) => {
                            self.reload_pending = true;
                            tracing::info!("WebView navigation recovery reload issued");
                        }
                        Err(error) => {
                            tracing::error!(%error, "controlled navigation recovery reload failed");
                        }
                    },
                    None => {
                        tracing::error!("WebView reload timeout with no WebView to reload");
                    }
                }
            }
            RecoveryDecision::Recreate => {
                tracing::warn!(
                    reason = failure_reasons::RELOAD_TIMEOUT,
                    "WebView reload timeout persists: recreating the WebView on the UI thread"
                );
                self.core.set_webview_error(Some(format!(
                    "WebView reload timed out ({}); recreating the WebView",
                    failure_reasons::RELOAD_TIMEOUT
                )));
                if let Err(error) = self.recreate_main_webview() {
                    tracing::error!(%error, "WebView recreate failed; transport stays failed");
                    self.reload.record_failure(failure_reasons::RELOAD_TIMEOUT);
                }
            }
            RecoveryDecision::GiveUp => {
                tracing::error!(
                    reason = ?self.reload.failure_reason(),
                    lifecycle = ?self.reload.state(),
                    "WebView reload recovery exhausted; parking in Failed without further retries"
                );
                self.page_ready = false;
                self.pending_batch = None;
                self.emitter.clear_batch();
                self.core.set_webview_error(Some(format!(
                    "WebView reload failed ({:?}); reload the window to retry",
                    self.reload.failure_reason().unwrap_or("unknown")
                )));
                self.webview_failed.store(true, Ordering::SeqCst);
            }
        }
    }

    pub(super) fn recreate_pending_plugin_webviews(&mut self, event_loop: &ActiveEventLoop) {
        for (plugin_id, page_id) in self.plugin_ui.drain_pending_recreates() {
            if self.shutting_down {
                continue;
            }
            tracing::info!(plugin = %plugin_id, page = %page_id, "recreating plugin WebView on the UI thread");
            if let Err(error) = self.plugin_ui.open(
                event_loop,
                &self.core,
                &self.control,
                &self.runtime,
                &self.proxy,
                &plugin_id,
                &page_id,
            ) {
                tracing::error!(%error, plugin = %plugin_id, page = %page_id, "plugin WebView recreate failed");
            }
        }
    }
}
