//! Main WebView construction and physical bounds, shared by initial startup and recovery.
//! Navigation policy and custom asset protocols remain identical.

use super::super::DesktopApp;
use super::bridge::MainIpcBridge;
use crate::platform;
use std::sync::atomic::Ordering;
use winit::{dpi::PhysicalSize, window::Window};
use wry::{
    dpi::{PhysicalPosition as WryPhysicalPosition, PhysicalSize as WryPhysicalSize},
    PageLoadEvent, Rect, WebViewBuilder,
};

const UPDATE_RELEASES_URL: &str = "https://github.com/nglmercer/TikTools-app/releases";

fn is_update_releases_url(url: &str) -> bool {
    url == UPDATE_RELEASES_URL
}

impl DesktopApp {
    /// Builds the main frontend WebView as a child of `window` on the UI
    /// thread. The frontend URL, navigation policy, IPC bridge and asset
    /// protocols are identical for the first load and every recreate, so a
    /// recovered WebView never escapes the allowed origin and never touches
    /// `AppCore` or user data (only the WebView handle is rebuilt).
    pub(super) fn build_main_webview(&mut self, window: &Window) -> Result<wry::WebView, String> {
        let bridge = MainIpcBridge::new(self);
        let page_emitter = self.emitter.clone();
        let navigation_frontend = self.frontend.clone();
        #[cfg(all(test, feature = "live-benchmark"))]
        let base_builder = match self.benchmark_context.as_mut() {
            // Isolate benchmark cookies/localStorage/cache from the user's app.
            Some(context) => WebViewBuilder::new_with_web_context(context),
            None => WebViewBuilder::new(),
        };
        #[cfg(not(all(test, feature = "live-benchmark")))]
        let base_builder = WebViewBuilder::new();
        let mut builder = base_builder
            .with_devtools(cfg!(debug_assertions) || cfg!(feature = "devtools"))
            .with_focused(false)
            .with_autoplay(true)
            // Page-focused shortcuts (F5/Ctrl+R reload, F12/Ctrl+Shift+I
            // inspector): on Linux the child WebView holds keyboard focus,
            // so the Winit window never sees these keys. The script reloads
            // in-page and asks the host for the inspector.
            .with_initialization_script(crate::webview::MAIN_INIT_SCRIPT)
            .with_initialization_script(if cfg!(feature = "remote-client") { "window.__tiktools_remote_client_available__ = true;" } else { "window.__tiktools_remote_client_available__ = false;" })
            .with_navigation_handler(move |url| {
                if is_update_releases_url(&url) {
                    if let Err(error) = open::that_detached(UPDATE_RELEASES_URL) {
                        tracing::warn!(%error, "could not open the application release page");
                    }
                    return false;
                }
                // Plugin asset URLs load inline frames in the plugin tabs;
                // installation is enforced per request by the asset server.
                let allowed = navigation_frontend.allows_navigation(&url)
                    || crate::plugin_webview::assets::allows_plugin_navigation(&url);
                if !allowed {
                    tracing::warn!(url = %url, "blocked WebView navigation outside the application frontend");
                }
                allowed
            })
            .with_new_window_req_handler(move |url, _features| {
                tracing::debug!(url = %url, "blocked WebView new-window request");
                wry::NewWindowResponse::Deny
            })
            // Anchor downloads (the rule-profile export, plugin asset
            // dumps) surface through WebKit's download signal, which
            // wry only wires up when a handler is registered. Without
            // this, `download`-attribute clicks in the WebView fail
            // silently: the blob URL is never written anywhere.
            .with_download_started_handler(move |url, destination| {
                tracing::debug!(
                    url = %url,
                    destination = %destination.display(),
                    "WebView download started"
                );
                true
            })
            .with_on_page_load_handler(move |event, url| {
                match event {
                    PageLoadEvent::Started => {
                        page_emitter.advance_generation();
                        tracing::info!(url = %url, generation = page_emitter.generation.load(Ordering::Acquire), "WebView navigation Started");
                    }
                    PageLoadEvent::Finished => {
                        tracing::info!(url = %url, "WebView navigation Finished; waiting for frontend-ready");
                    }
                }
            })
            .with_ipc_handler(move |request| bridge.handle(request.body()));

        // Wry's Linux/X11 child-window path converts logical default bounds
        // using the X11 screen millimeter dimensions. Some XWayland/KDE
        // sessions report those dimensions as zero, which produces an invalid
        // scale factor before the WebView is even attached. The window resize
        // event already gives us physical pixels, so keep this boundary
        // physical and avoid that conversion entirely.
        let initial_size = window.inner_size();
        builder = builder.with_bounds(Rect {
            position: WryPhysicalPosition::new(0, 0).into(),
            size: WryPhysicalSize::new(initial_size.width.max(1), initial_size.height.max(1))
                .into(),
        });

        if let Some(assets) = self.frontend.asset_server() {
            builder = builder.with_custom_protocol("tiktools".to_owned(), move |_id, request| {
                assets.respond(request)
            });
        }
        // Plugin UI assets for the inline tab frames, in dev and packaged
        // builds alike (the main document URL is irrelevant: the scheme is
        // registered on the WebView itself).
        let plugin_assets =
            crate::plugin_webview::assets::SharedPluginAssetServer::new(self.core.clone());
        builder = builder.with_custom_protocol(
            crate::plugin_webview::assets::PLUGIN_ASSET_SCHEME.to_owned(),
            move |_id, request| plugin_assets.respond(request),
        );
        builder = builder.with_url(self.frontend.url().as_str());
        tracing::debug!(
            url = %self.frontend.url(),
            "creating TikTools frontend WebView"
        );
        platform::build_webview(builder, window)
            .map_err(|error| format!("could not create Wry WebView: {error}"))
    }

    pub(super) fn resize_webview(&self, size: PhysicalSize<u32>) {
        let Some(webview) = self.webview.as_ref() else {
            return;
        };
        let bounds = Rect {
            position: WryPhysicalPosition::new(0, 0).into(),
            size: WryPhysicalSize::new(size.width.max(1), size.height.max(1)).into(),
        };
        if let Err(error) = webview.set_bounds(bounds) {
            tracing::debug!(%error, "could not resize WebView");
        }
    }
}

#[cfg(test)]
mod update_release_link_tests {
    use super::*;

    #[test]
    fn only_the_fixed_release_page_opens_externally() {
        assert!(is_update_releases_url(UPDATE_RELEASES_URL));
        for url in [
            "http://github.com/nglmercer/TikTools-app/releases",
            "https://github.com/nglmercer/TikTools-app/releases?next=evil",
            "https://github.com.evil.example/nglmercer/TikTools-app/releases",
            "https://github.com/nglmercer/TikTools-app/releases/other",
        ] {
            assert!(!is_update_releases_url(url));
        }
    }
}
