//! Desktop-only updater initialization and access to its owned service.
use super::*;

impl AppCore {
    /// Enable the desktop updater only from the native desktop host. The key is
    /// fixed at build time; control clients cannot choose a key or endpoint.
    #[cfg(feature = "app-update")]
    pub fn enable_desktop_app_updater(&self) {
        let events = self.events.clone();
        // Feed publication is a separate release gate: embedding a key alone
        // must not activate the proposed URLs before they are live.
        let key = if option_env!("TIKTOOLS_APP_UPDATE_FEED_ENABLED") == Some("1") {
            option_env!("TIKTOOLS_APP_UPDATE_PUBLIC_KEY")
        } else {
            None
        };
        let service = tiktools_app_update::AppUpdater::new_desktop(
            env!("CARGO_PKG_VERSION"),
            key,
            option_env!("TIKTOOLS_APP_UPDATE_INSTALL_ENABLED") == Some("1"),
            option_env!("TIKTOOLS_APP_UPDATE_NSIS_ENABLED") == Some("1"),
            move |state| {
                if let Ok(state) = serde_json::to_value(state) {
                    events.publish_domain(crate::events::DomainEvent::AppUpdateStateChanged {
                        state,
                    });
                }
            },
        );
        *self.app_updater.write().unwrap_or_else(|e| e.into_inner()) = Some(service);
    }

    #[cfg(feature = "app-update")]
    pub fn app_updater(&self) -> Option<Arc<tiktools_app_update::AppUpdater>> {
        self.app_updater
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
