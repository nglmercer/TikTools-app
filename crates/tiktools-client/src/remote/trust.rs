use super::{fail, network, ClientError};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
    DigitallySignedStruct, SignatureScheme,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum TlsTrust {
    System,
    CertificateSha256 { sha256: String },
}
pub fn normalize_fingerprint(value: &str) -> Result<String, ClientError> {
    let value = value.trim().to_ascii_lowercase();
    let value = value.strip_prefix("sha256:").unwrap_or(&value);
    let value: String = value.chars().filter(|c| *c != ':').collect();
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(fail(
            "invalid_params",
            "a SHA-256 certificate fingerprint is required",
        ));
    }
    Ok(value)
}
pub(super) fn fingerprint(cert: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, cert)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
impl TlsTrust {
    pub fn validate(&self) -> Result<(), ClientError> {
        if let Self::CertificateSha256 { sha256 } = self {
            if normalize_fingerprint(sha256)? != *sha256 {
                return Err(fail(
                    "invalid_params",
                    "stored TLS fingerprint must be canonical",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn config(&self) -> Result<Arc<rustls::ClientConfig>, ClientError> {
        self.validate()?;
        let builder = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| fail("trust", "TLS configuration is unavailable"))?;
        let config = match self {
            Self::System => {
                let result = rustls_native_certs::load_native_certs();
                let mut roots = rustls::RootCertStore::empty();
                for cert in result.certs {
                    let _ = roots.add(cert);
                }
                if roots.is_empty() {
                    return Err(fail("trust", "system certificate roots are unavailable"));
                }
                builder.with_root_certificates(roots).with_no_client_auth()
            }
            Self::CertificateSha256 { sha256 } => builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(PinnedVerifier {
                    expected: sha256.clone(),
                }))
                .with_no_client_auth(),
        };
        Ok(Arc::new(config))
    }
}
#[derive(Debug)]
struct PinnedVerifier {
    expected: String,
}
fn algs() -> rustls::crypto::WebPkiSupportedAlgorithms {
    rustls::crypto::ring::default_provider().signature_verification_algorithms
}
impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        leaf: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        name: &ServerName<'_>,
        _: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if leaf.len() > 64 * 1024 || fingerprint(leaf.as_ref()) != self.expected {
            return Err(rustls::Error::General(
                "approved certificate fingerprint changed".into(),
            ));
        }
        let (_, cert) = x509_parser::parse_x509_certificate(leaf.as_ref()).map_err(|_| {
            rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding)
        })?;
        if now.as_secs() > i64::MAX as u64
            || !cert.validity().is_valid_at(
                x509_parser::time::ASN1Time::from_timestamp(now.as_secs() as i64)
                    .map_err(|_| rustls::Error::General("certificate time is invalid".into()))?,
            )
        {
            return Err(rustls::Error::General(
                "approved certificate is outside its validity period".into(),
            ));
        }
        let parsed = rustls::server::ParsedCertificate::try_from(leaf)?;
        if cert
            .key_usage()
            .map_err(|_| rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding))?
            .is_some_and(|usage| !usage.value.digital_signature())
            || cert
                .extended_key_usage()
                .map_err(|_| {
                    rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding)
                })?
                .is_some_and(|usage| !usage.value.server_auth && !usage.value.any)
        {
            return Err(rustls::Error::General(
                "approved certificate cannot authenticate a TLS server".into(),
            ));
        }
        rustls::client::verify_server_name(&parsed, name)?;
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        sig: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, sig, &algs())
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        sig: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, sig, &algs())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        algs().supported_schemes()
    }
}
#[derive(Debug)]
struct RejectingProbe {
    seen: Arc<Mutex<Option<String>>>,
}
impl ServerCertVerifier for RejectingProbe {
    fn verify_server_cert(
        &self,
        leaf: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if leaf.len() <= 64 * 1024 {
            if let Ok(mut seen) = self.seen.lock() {
                *seen = Some(fingerprint(leaf.as_ref()));
            }
        }
        // Discovery aborts the handshake. It neither authenticates a host nor
        // permits an HTTP request, invitation or credential to be sent.
        Err(rustls::Error::General(
            "unverified certificate probe stopped".into(),
        ))
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        s: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(m, c, s, &algs())
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        s: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, s, &algs())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        algs().supported_schemes()
    }
}
pub async fn probe_fingerprint(origin: &str) -> Result<String, ClientError> {
    let url = super::http::origin(origin)?;
    let seen = Arc::new(Mutex::new(None));
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|_| network())?
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(RejectingProbe { seen: seen.clone() }))
    .with_no_client_auth();
    let host = url
        .host_str()
        .ok_or_else(network)?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned();
    let name = ServerName::try_from(host.clone()).map_err(|_| network())?;
    let operation = async {
        let tcp = tokio::net::TcpStream::connect((
            host.as_str(),
            url.port_or_known_default().unwrap_or(443),
        ))
        .await?;
        let _ = tokio_rustls::TlsConnector::from(Arc::new(config))
            .connect(name, tcp)
            .await;
        Ok::<_, std::io::Error>(())
    };
    tokio::time::timeout(Duration::from_secs(10), operation)
        .await
        .map_err(|_| network())?
        .map_err(|_| network())?;
    let result = seen
        .lock()
        .map_err(|_| network())?
        .clone()
        .ok_or_else(network);
    result
}
