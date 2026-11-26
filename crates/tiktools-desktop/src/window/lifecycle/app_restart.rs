//! One-shot native restart confirmation. The IPC handler admits exactly one
//! prompt per process; shutdown runs through the same graceful path as Quit,
//! and the relaunch is performed by `app::run` after the instance lock is free.

use super::super::DesktopApp;
use crate::event::{DesktopCommand, DesktopEvent};
use crate::native_messages;
use std::sync::atomic::Ordering;

impl DesktopApp {
    /// Consumed once by `app::run` after the event loop returns. The string is
    /// the locale used for the relaunch-failure dialog.
    pub(crate) fn take_restart_request(&mut self) -> Option<String> {
        self.restart_requested.take()
    }

    pub(super) fn request_app_restart(&mut self, generation: u64, locale: String) {
        if self.shutting_down || generation != self.emitter.generation.load(Ordering::Acquire) {
            // The page that asked is gone; let the live page try again.
            self.emitter
                .restart_prompt_pending
                .store(false, Ordering::Release);
            return;
        }
        // The guard stays set after the dialog, whatever the answer. A second
        // plugin install must not trigger a second prompt in the same session.
        let dialog = rfd::MessageDialog::new()
            .set_title(native_messages::restart_title(&locale))
            .set_description(native_messages::restart_description(&locale))
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNo);
        // Native dialog parenting needs a raw window handle; the GTK shell
        // window does not expose one through the shell abstraction, so on
        // Linux the dialog is simply unparented (still modal by default).
        #[cfg(not(target_os = "linux"))]
        let dialog = if let Some(window) = self.window.as_ref() {
            dialog.set_parent(window.winit_window())
        } else {
            dialog
        };
        if dialog.show() != rfd::MessageDialogResult::Yes {
            return;
        }
        self.restart_requested = Some(locale);
        let _ = self.proxy.send_event(DesktopEvent::Command(
            DesktopCommand::BeginAppRestartShutdown,
        ));
    }
}
