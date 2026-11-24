//! The one process-wide rustls crypto provider for TikTools binaries.
//!
//! Libraries never choose a provider. Each application calls
//! [`ensure_default_crypto_provider`] once at startup; the provider it installs
//! is the one enabled by that application's `aws-lc-rs` or `ring` feature.

/// Installs the provider compiled into this build, unless one is already
/// installed. When both features are enabled, `ring` is installed.
///
/// Does nothing when neither provider feature is enabled, so libraries can call
/// it defensively without choosing a provider themselves.
pub fn ensure_default_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        #[cfg(feature = "ring")]
        let _ = rustls::crypto::ring::default_provider().install_default();
        #[cfg(all(feature = "aws-lc-rs", not(feature = "ring")))]
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }
}
