//! Native update confirmation and safe handoff. One prompt guard spans every branch; the
//! install plan remains owned by DesktopApp until shutdown completes.

use super::super::DesktopApp;
use crate::event::{DesktopCommand, DesktopEvent};
use std::sync::{atomic::Ordering, Arc};

impl DesktopApp {
    pub(crate) fn take_update_plan(
        &mut self,
    ) -> Result<Option<(tiktools_core::app_update::InstallPlan, bool)>, bool> {
        let plan = self.pending_update.take();
        match plan {
            Some((_, spanish)) if !self.update_ready_for_handoff => Err(spanish),
            _ => Ok(plan),
        }
    }

    pub(super) fn request_app_update_install(&mut self, generation: u64, spanish: bool) {
        struct ResetPrompt(Arc<crate::event::EventLoopEmitter>);
        impl Drop for ResetPrompt {
            fn drop(&mut self) {
                self.0.update_prompt_pending.store(false, Ordering::Release);
            }
        }
        let _reset = ResetPrompt(self.emitter.clone());
        if self.shutting_down || generation != self.emitter.generation.load(Ordering::Acquire) {
            tracing::warn!(
                shutting_down = self.shutting_down,
                generation,
                current = self.emitter.generation.load(Ordering::Acquire),
                "update install request dropped: stale generation or shutdown in progress"
            );
            return;
        }
        let Some(updater) = self.core.app_updater() else {
            tracing::warn!("update install request dropped: updater unavailable");
            return;
        };
        let status = updater.status();
        if !status.install_supported
            || status.state != tiktools_core::app_update::Phase::ReadyToInstall
        {
            tracing::warn!(
                install_supported = status.install_supported,
                state = ?status.state,
                "update install request dropped: updater not ready to install"
            );
            return;
        }
        let Some(version) = status.available_version.as_deref() else {
            tracing::warn!("update install request dropped: no available version in status");
            return;
        };
        let Some(location) = updater.installation_location() else {
            tracing::warn!("update install request dropped: installation location unknown");
            return;
        };
        let location = location
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_control() { '�' } else { c })
            .collect::<String>();
        let (title, description) = if spanish {
            ("Instalar actualización de TikTools", format!("¿Instalar TikTools {version} y reiniciar? Las sesiones LIVE y automatizaciones activas se detendrán.\n\nUbicación: {location}"))
        } else {
            ("Install TikTools update", format!("Install TikTools {version} and restart? Active LIVE sessions and automations will stop.\n\nLocation: {location}"))
        };
        // Linux gets a GTK-native dialog transient for the shell window:
        // the rfd xdg-portal backend cannot be parented through the shell
        // abstraction, and an unparented portal dialog surfaces behind the
        // main window on compositors with focus-stealing prevention — the
        // confirmation then reads exactly like a dead install button.
        #[cfg(target_os = "linux")]
        let confirmed = {
            use gtk::prelude::*;
            let dialog = gtk::MessageDialog::new(
                self.window.as_ref().and_then(|shell| shell.gtk_window()),
                gtk::DialogFlags::MODAL,
                gtk::MessageType::Warning,
                gtk::ButtonsType::YesNo,
                &title,
            );
            dialog.set_secondary_text(Some(&description));
            let confirmed = dialog.run() == gtk::ResponseType::Yes;
            unsafe { dialog.destroy() };
            confirmed
        };
        #[cfg(not(target_os = "linux"))]
        let confirmed = {
            let dialog = rfd::MessageDialog::new()
                .set_title(title.clone())
                .set_description(description.clone())
                .set_level(rfd::MessageLevel::Warning)
                .set_buttons(rfd::MessageButtons::YesNo);
            let dialog = if let Some(window) = self.window.as_ref() {
                dialog.set_parent(window.winit_window())
            } else {
                dialog
            };
            dialog.show() == rfd::MessageDialogResult::Yes
        };
        if !confirmed {
            return;
        }
        match updater.prepare_install() {
            Ok(plan) => {
                self.pending_update = Some((plan, spanish));
                // Allow the Installing event to reach the WebView before
                // closing its outbound mailbox and beginning shutdown.
                let proxy = self.proxy.clone();
                self.runtime.spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    let _ = proxy.send_event(DesktopEvent::Command(
                        DesktopCommand::BeginAppUpdateShutdown,
                    ));
                });
            }
            Err(error) => {
                tracing::warn!(%error, "could not prepare application update");
                let message = if spanish {
                    "No se pudo preparar la actualización. Comprueba el estado e inténtalo de nuevo."
                } else {
                    "The update could not be prepared. Check its status and try again."
                };
                let _ = rfd::MessageDialog::new()
                    .set_title(if spanish {
                        "Actualización de TikTools"
                    } else {
                        "TikTools update"
                    })
                    .set_description(message)
                    .set_level(rfd::MessageLevel::Error)
                    .set_buttons(rfd::MessageButtons::Ok)
                    .show();
            }
        }
    }
}
