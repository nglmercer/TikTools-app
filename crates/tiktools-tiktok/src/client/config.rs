//! Platform preset, TLS provider selection and device runtime defaults.
use crate::NativeTikTokConfig;
use std::{path::PathBuf, time::Duration};

impl Default for NativeTikTokConfig {
    fn default() -> Self {
        Self {
            bundle_path: std::env::var_os("TIKTOOLS_TIKTOK_SIGNING_BUNDLE").map(PathBuf::from),
            bundle_cache_path: std::env::var_os("TIKTOOLS_TIKTOK_BUNDLE_CACHE").map(PathBuf::from),
            bundle_url: std::env::var("TIKTOOLS_TIKTOK_BUNDLE_URL").unwrap_or_else(|_| {
                "https://sf16-website-login.neutral.ttwstatic.com/obj/tiktok_web_login_static/webmssdk/1.0.0.388/webmssdk.js".to_owned()
            }),
            reconnect: ttl_live_ws::ReconnectPolicy {
                max_attempts: 5,
                initial_backoff: Duration::from_secs(2),
                max_backoff: Duration::from_secs(30),
            },
        }
    }
}

pub(in super::super) fn ensure_tls_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        // reqwest and tokio-tungstenite can enable different rustls providers
        // through Cargo feature unification. Selecting ring here keeps the
        // library safe when it is embedded without the desktop binary.
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

pub(in super::super) fn default_preset() -> ttl_sign_core::Preset {
    #[cfg(target_os = "windows")]
    let device = ttl_sign_core::DevicePreset::chrome_windows();
    #[cfg(target_os = "macos")]
    let device = ttl_sign_core::DevicePreset::chrome_macos();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let device = ttl_sign_core::DevicePreset::chrome_linux();
    ttl_sign_core::Preset::new(
        device,
        ttl_sign_core::LocationPreset::us_east(),
        ttl_sign_core::ScreenPreset::FHD,
    )
}
