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
            return;
        }
        let Some(updater) = self.core.app_updater() else {
            return;
        };
        let status = updater.status();
        if !status.install_supported
            || status.state != tiktools_core::app_update::Phase::ReadyToInstall
        {
            return;
        }
        let Some(version) = status.available_version.as_deref() else {
            return;
        };
        let Some(location) = updater.installation_location() else {
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
        let dialog = rfd::MessageDialog::new()
            .set_title(title)
            .set_description(description)
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNo);
        let dialog = if let Some(window) = self.window.as_ref() {
            dialog.set_parent(window)
        } else {
            dialog
        };
        if dialog.show() != rfd::MessageDialogResult::Yes {
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
