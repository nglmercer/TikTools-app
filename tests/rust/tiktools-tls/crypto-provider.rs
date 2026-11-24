#![cfg(any(feature = "aws-lc-rs", feature = "ring"))]

#[test]
fn installs_one_provider_and_is_idempotent() {
    tiktools_tls::ensure_default_crypto_provider();
    let first = rustls::crypto::CryptoProvider::get_default()
        .expect("a provider should be installed")
        .clone();
    tiktools_tls::ensure_default_crypto_provider();
    let second = rustls::crypto::CryptoProvider::get_default()
        .expect("a provider should stay installed")
        .clone();
    assert!(std::sync::Arc::ptr_eq(&first, &second));
}
