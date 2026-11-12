//! Window startup, shutdown, visibility, and event-loop integration.

mod app_update;
mod bridge;
mod main_webview;
mod recovery;
mod shutdown;
mod startup;
mod transport;
mod window_events;

use super::reload::failure_reasons;
use super::{DesktopApp, StartupState};
use crate::{
    event::{CoalescedSignals, DesktopCommand, DesktopEvent},
    platform,
};
use std::{sync::atomic::Ordering, time::Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
    window::WindowId,
};

impl ApplicationHandler<DesktopEvent> for DesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        if self.window.is_some() {
            return;
        }
        if let Err(error) = self.create_window(event_loop) {
            tracing::error!(%error, "Rust desktop host could not start");
            crate::show_startup_failure(&error, &self.log_path);
            self.startup_state = StartupState::Failed;
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle_window_event(event_loop, window_id, event);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: DesktopEvent) {
        match crate::event::consume_signal(event) {
            DesktopEvent::Command(DesktopCommand::AdmittedCommand {
                command,
                admission: _admission,
            }) => {
                self.user_event(event_loop, DesktopEvent::Command(*command));
            }
            // Wake-only chaining for multi-batch bursts; the flush runs in
            // `about_to_wait`, so this arm intentionally does nothing.
            DesktopEvent::Command(DesktopCommand::FlushWebviewBatch) => {}
            DesktopEvent::Command(DesktopCommand::HostBatchAcknowledged) => {
                self.acknowledge_host_batch();
            }
            #[cfg(all(test, feature = "live-benchmark"))]
            DesktopEvent::Command(DesktopCommand::BenchmarkSample { .. }) => {}
            DesktopEvent::Command(DesktopCommand::IpcFailed(message)) => {
                // A second owner at startup means a stale host is still
                // running: fail fast instead of mixing runtimes. After
                // startup the degraded `system.health` plus retry loop
                // keeps CLI/agent loss visible without killing the GUI.
                if self.startup_state != StartupState::Ready && !self.shutting_down {
                    self.fail_startup(
                        event_loop,
                        format!(
                            "The control IPC endpoint is already owned ({message}). Close the other TikTools host and restart."
                        ),
                    );
                } else {
                    tracing::error!(message, "control IPC unavailable; CLI/agents degraded");
                }
            }
            DesktopEvent::Command(DesktopCommand::FrontendReady { generation: _ }) => {
                let generation = self.emitter.consume_ready();
                if generation == self.emitter.generation.load(Ordering::Acquire) {
                    self.sync_page_generation(generation);
                    self.frontend_ready();
                }
            }
            DesktopEvent::Command(DesktopCommand::ShowWindow) => {
                if self.window.is_none() {
                    if let Err(error) = self.create_window(event_loop) {
                        tracing::error!(%error, "could not recreate TikTools window from tray");
                        return;
                    }
                    // A failed transport recreates with a fresh page: drop
                    // the stale backlog now; the mount handshake recovers
                    // the latch (see `frontend_ready`).
                    if self.pending_host_messages.transport_failed() {
                        self.pending_host_messages.clear_for_reload();
                        self.reload_pending = true;
                    }
                }
                self.restore_window();
            }
            DesktopEvent::Command(DesktopCommand::HideWindow) => {
                self.set_window_visible(false);
            }
            DesktopEvent::Command(DesktopCommand::OpenDevtools) => self.open_devtools(),
            DesktopEvent::Command(DesktopCommand::OpenPluginDevtools { window_id }) => {
                self.plugin_ui.open_devtools_for_window(window_id)
            }
            DesktopEvent::Command(DesktopCommand::Quit) => self.shutdown(event_loop),
            DesktopEvent::Command(DesktopCommand::RequestAppUpdateInstall {
                generation,
                spanish,
            }) => {
                self.request_app_update_install(generation, spanish);
            }
            DesktopEvent::Command(DesktopCommand::BeginAppUpdateShutdown) => {
                if self.pending_update.is_some() {
                    self.shutdown(event_loop);
                }
            }
            DesktopEvent::Command(DesktopCommand::ShutdownComplete) => {
                self.finalize_shutdown(event_loop)
            }
            DesktopEvent::Command(DesktopCommand::OpenPluginUi { plugin_id, page_id }) => {
                if self.shutting_down {
                    return;
                }
                if let Err(error) = self.plugin_ui.open(
                    event_loop,
                    &self.core,
                    &self.control,
                    &self.runtime,
                    &self.proxy,
                    &plugin_id,
                    &page_id,
                ) {
                    tracing::warn!(%error, plugin = %plugin_id, page = %page_id, "could not open plugin UI");
                }
            }
            DesktopEvent::Command(DesktopCommand::ClosePluginUi {
                plugin_id,
                page_id,
                window_id,
                owner,
            }) => {
                self.plugin_ui
                    .close_if_current(&plugin_id, &page_id, window_id, owner);
            }
            DesktopEvent::Command(DesktopCommand::PluginUiRespond {
                window_id,
                plugin_id,
                page_id,
                response,
                effect,
                generation,
                admission: _admission,
            }) => {
                self.plugin_ui.respond(
                    &plugin_id, &page_id, response, window_id, generation, effect,
                );
            }
            DesktopEvent::Command(DesktopCommand::PluginUiSubscribe {
                window_id,
                plugin_id,
                page_id,
                effect,
                generation,
                admission: _admission,
            }) => {
                self.plugin_ui
                    .apply_subscription(&plugin_id, &page_id, effect, window_id, generation);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        platform::pump();
        let generation = self.emitter.generation.load(Ordering::Acquire);
        self.sync_page_generation(generation);
        // Bounded reload watchdog: missing `frontend-ready` records a
        // reason and attempts one controlled reload, then one recreate,
        // then parks in Failed. Never restarts AppCore, never replays RPCs.
        self.poll_reload_watchdog();
        // Plugin pop-outs share the same bound: timed-out bridge checks
        // reload once, then close/recreate, then park without looping.
        self.plugin_ui.poll_recovery();
        // Automatic recreates run on the UI thread through the same `open`
        // path (same origin policy, same broker, same AppCore): at most one
        // per failure sequence, so a permanently broken page parks instead
        // of reopening forever.
        self.recreate_pending_plugin_webviews(event_loop);
        if self
            .pending_batch
            .as_ref()
            .is_some_and(|batch| Instant::now() >= batch.deadline)
        {
            self.core.record_event_gap();
            self.fail_webview_transport(failure_reasons::BATCH_TIMEOUT);
        }
        self.poll_desktop_diagnostics();
        self.drain_domain_mailbox();
        // One batch per UI tick: all host messages queued since the last
        // tick share a single `evaluate_script`.
        self.flush_webview_batch();
        let plugin_pending = self.plugin_ui.flush_batches();
        // A batch is capped, so a burst longer than one batch must chain
        // exactly one wake per remaining batch instead of stalling under
        // ControlFlow::Wait with no further events. The wake event itself
        // is a no-op; the next turn's flush does the work. No busy loop:
        // one wake is scheduled per turn that still has backlog.
        if (self.page_ready
            && self.pending_batch.is_none()
            && !self.pending_host_messages.is_empty())
            || plugin_pending
        {
            let _ = self
                .proxy
                .send_coalesced(DesktopEvent::Command(DesktopCommand::FlushWebviewBatch));
        }
        if self.startup_state == StartupState::WebViewLoading {
            if let Some(deadline) = self.startup_deadline {
                if Instant::now() >= deadline {
                    tracing::error!(
                        state = ?self.startup_state,
                        "frontend startup timeout"
                    );
                    self.fail_startup(
                        event_loop,
                        "The packaged web application did not become ready within 10 seconds.",
                    );
                    return;
                }
                // On Linux this wakes at the next GTK pump interval (capped by
                // the deadline) instead of sleeping through the whole startup
                // timeout; other platforms wait for the deadline directly.
                platform::prepare_for_startup_wait(event_loop, deadline);
                return;
            }
        }
        if let Some(batch) = self.pending_batch.as_ref() {
            platform::prepare_for_startup_wait(event_loop, batch.deadline);
        } else {
            platform::prepare_for_wait(event_loop);
        }
    }
}
