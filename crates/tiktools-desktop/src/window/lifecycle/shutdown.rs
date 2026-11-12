//! Graceful host shutdown and UI resource teardown. The WebView container drops before its
//! parent window and update handoff requires a safe core shutdown.

use super::super::{DesktopApp, StartupState};
use crate::{
    event::{CoalescedSignals, DesktopCommand, DesktopEvent},
    platform,
};
use std::sync::{atomic::Ordering, Arc};
use winit::event_loop::ActiveEventLoop;

impl DesktopApp {
    pub(super) fn shutdown(&mut self, _event_loop: &ActiveEventLoop) {
        if self.shutting_down {
            return;
        }
        tracing::debug!("shutdown started");
        self.shutting_down = true;
        self.emitter.ipc_slots.close();
        self.emitter.generation.fetch_add(1, Ordering::AcqRel);
        self.domain_mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .shutdown();
        self.startup_state = StartupState::ShuttingDown;
        self.startup_deadline = None;
        let core = Arc::clone(&self.core);
        let proxy = self.proxy.clone();
        self.runtime.spawn(async move {
            core.shutdown().await;
            let _ = proxy.send_coalesced(DesktopEvent::Command(DesktopCommand::ShutdownComplete));
        });
    }

    pub(super) fn finalize_shutdown(&mut self, event_loop: &ActiveEventLoop) {
        self.startup_state = StartupState::ShuttingDown;
        self.tray.take();
        if let Some(webview) = self.webview.take() {
            platform::prepare_webview_teardown(&webview);
            drop(webview);
        }
        // Container destroy ahead of the parent destroy (see
        // `platform::sync_gtk_display`); the parent handle drops normally.
        platform::sync_gtk_display();
        self.window.take();
        self.update_ready_for_handoff =
            self.pending_update.is_some() && self.core.update_shutdown_safe();
        tracing::debug!("shutdown complete, exiting");
        event_loop.exit();
    }
}

impl Drop for DesktopApp {
    fn drop(&mut self) {
        self.emitter.ipc_slots.close();
        self.domain_mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .shutdown();
        if let Some(webview) = self.webview.take() {
            platform::prepare_webview_teardown(&webview);
            drop(webview);
            platform::sync_gtk_display();
        }
        self.window.take();
    }
}
