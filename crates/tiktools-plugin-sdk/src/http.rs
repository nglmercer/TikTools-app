//! Verified TLS for native hosts and provider tooling. No Android JVM required.

pub fn client_builder() -> Result<reqwest::ClientBuilder, String> {
    let builder = reqwest::Client::builder();
    #[cfg(target_os = "android")]
    {
        // rustls-platform-verifier requires an initialized Java runtime on
        // Android. Termux is a native process: use its maintained CA bundle.
        let path = std::env::var_os("SSL_CERT_FILE").map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("PREFIX").map(|prefix|
                std::path::PathBuf::from(prefix).join("etc/tls/cert.pem")))
            .ok_or("TLS CA bundle unavailable; set SSL_CERT_FILE or PREFIX (pkg install ca-certificates)")?;
        let bytes = std::fs::read(&path).map_err(|_| {
            "Cannot read TLS CA bundle; check SSL_CERT_FILE or install ca-certificates".to_owned()
        })?;
        let certs = reqwest::Certificate::from_pem_bundle(&bytes)
            .map_err(|_| "TLS CA bundle contains invalid PEM certificates".to_owned())?;
        if certs.is_empty() {
            return Err("TLS CA bundle contains no certificates".to_owned());
        }
        Ok(builder.tls_certs_only(certs))
    }
    #[cfg(not(target_os = "android"))]
    Ok(builder)
}

#[cfg(all(test, target_os = "android"))]
mod tests {
    #[test]
    fn native_client_builds_without_a_java_runtime() {
        super::client_builder()
            .expect("Termux CA bundle")
            .build()
            .expect("verified native TLS client");
    }
}
