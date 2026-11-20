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
    if rustls::crypto::CryptoProvider::get_default().is_some() {
        return;
    }
    #[cfg(feature = "ring")]
    let _ = rustls::crypto::ring::default_provider().install_default();
    #[cfg(all(feature = "aws-lc-rs", not(feature = "ring")))]
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

#[cfg(all(test, any(feature = "aws-lc-rs", feature = "ring")))]
mod tests {
    use super::ensure_default_crypto_provider;

    #[test]
    fn installs_one_provider_and_is_idempotent() {
        ensure_default_crypto_provider();
        let first = rustls::crypto::CryptoProvider::get_default()
            .expect("a provider should be installed")
            .clone();
        ensure_default_crypto_provider();
        let second = rustls::crypto::CryptoProvider::get_default()
            .expect("a provider should stay installed")
            .clone();
        assert!(std::sync::Arc::ptr_eq(&first, &second));
    }
}
