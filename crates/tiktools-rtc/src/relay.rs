//! End-to-end TLS for an opaque application relay. The outer Cloud WSS only
//! transports records; device signatures bind each temporary TLS trust root
//! to the exact approved negotiation. No OS trust store or user certificate
//! installation is involved. This module alone does not enable relay routing.
use crate::{signaling::Context, Error};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rcgen::{BasicConstraints, CertificateParams, CertifiedIssuer, IsCa, KeyPair, KeyUsagePurpose};
use ring::signature::{Ed25519KeyPair, KeyPair as _, UnparsedPublicKey, ED25519};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName},
    ClientConfig, RootCertStore, ServerConfig,
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use tokio_rustls::{TlsAcceptor, TlsConnector, TlsStream};

const DOMAIN: &[u8] = b"tiktools.relay.tls.v1\0";
const ALPN: &[u8] = b"tiktools/relay/1";
const NAME: &str = "relay.tiktools.invalid";
const DEADLINE: Duration = Duration::from_secs(60);
const MAX_CERTIFICATE: usize = 4096;
mod framing;
pub use framing::RelayFramed;
pub mod io;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Announcement {
    pub context: Context,
    pub certificate: String,
    pub signature: String,
}

fn signed_bytes(context: &Context, certificate: &str) -> Result<Vec<u8>, Error> {
    let mut value = serde_json::to_value(context).map_err(|_| Error::Negotiation)?;
    value["certificate"] = serde_json::json!(certificate);
    // Only scalar fields: the backend can use its existing canonical JSON.
    let sorted: std::collections::BTreeMap<_, _> = value
        .as_object()
        .ok_or(Error::Negotiation)?
        .iter()
        .collect();
    let mut bytes = DOMAIN.to_vec();
    serde_json::to_writer(&mut bytes, &sorted).map_err(|_| Error::Negotiation)?;
    Ok(bytes)
}

/// Ephemeral secrets remain in native memory and are consumed by one handshake.
pub struct Identity {
    context: Context,
    endpoint: String,
    announcement: Announcement,
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
    started: Instant,
}

/// Construction requires the pinned device key, never a key supplied by WSS.
pub struct VerifiedPeer {
    context: Context,
    endpoint: String,
    certificate: CertificateDer<'static>,
}

/// Both TLS peers proved possession of keys certified by device-signed roots.
/// Business permission selection still belongs to the host's local authority.
pub struct SecureChannel<S> {
    stream: TlsStream<S>,
    local: Context,
    remote: Context,
    local_endpoint: String,
    remote_endpoint: String,
}
impl<S> SecureChannel<S> {
    pub fn local_context(&self) -> &Context {
        &self.local
    }
    pub fn remote_context(&self) -> &Context {
        &self.remote
    }
    pub fn local_endpoint_id(&self) -> &str {
        &self.local_endpoint
    }
    pub fn remote_endpoint_id(&self) -> &str {
        &self.remote_endpoint
    }
    pub fn into_stream(self) -> TlsStream<S> {
        self.stream
    }
}
impl<S: AsyncRead + AsyncWrite + Unpin> SecureChannel<S> {
    pub fn into_framed(self) -> RelayFramed<S> {
        RelayFramed::new(self.stream)
    }
}

impl Identity {
    pub fn new(context: Context, device_secret: &[u8; 32]) -> Result<Self, Error> {
        if context.grant_revision <= 0
            || [
                context.host_id,
                context.session_id,
                context.host_device_id,
                context.from_device_id,
                context.negotiation_id,
            ]
            .iter()
            .any(uuid::Uuid::is_nil)
        {
            return Err(Error::Configuration);
        }
        let started = Instant::now();
        let mut params = CertificateParams::new(vec![]).map_err(|_| Error::Configuration)?;
        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let issuer = CertifiedIssuer::self_signed(
            params,
            KeyPair::generate().map_err(|_| Error::Configuration)?,
        )
        .map_err(|_| Error::Configuration)?;
        let certificate = URL_SAFE_NO_PAD.encode(issuer.der().as_ref());
        let leaf_key = KeyPair::generate().map_err(|_| Error::Configuration)?;
        let mut leaf_params =
            CertificateParams::new(vec![NAME.into()]).map_err(|_| Error::Configuration)?;
        leaf_params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        leaf_params.extended_key_usages = vec![
            rcgen::ExtendedKeyUsagePurpose::ServerAuth,
            rcgen::ExtendedKeyUsagePurpose::ClientAuth,
        ];
        let leaf = leaf_params
            .signed_by(&leaf_key, &issuer)
            .map_err(|_| Error::Configuration)?;
        let signer =
            Ed25519KeyPair::from_seed_unchecked(device_secret).map_err(|_| Error::Configuration)?;
        let signature =
            URL_SAFE_NO_PAD.encode(signer.sign(&signed_bytes(&context, &certificate)?).as_ref());
        Ok(Self {
            announcement: Announcement {
                context: context.clone(),
                certificate,
                signature,
            },
            context,
            endpoint: endpoint(signer.public_key().as_ref()),
            chain: vec![leaf.der().clone()],
            key: PrivatePkcs8KeyDer::from(leaf_key.serialize_der()).into(),
            started,
        })
    }

    pub fn announcement(&self) -> Announcement {
        self.announcement.clone()
    }

    pub fn verify(
        &self,
        expected: &Context,
        pinned_key: [u8; 32],
        announcement: Announcement,
    ) -> Result<VerifiedPeer, Error> {
        if self.started.elapsed() >= DEADLINE
            || &announcement.context != expected
            || !self.matches(expected)
            || announcement.signature.len() != 86
            || announcement.certificate.len() > MAX_CERTIFICATE * 4 / 3 + 4
        {
            return Err(Error::Negotiation);
        }
        let signature = URL_SAFE_NO_PAD
            .decode(&announcement.signature)
            .map_err(|_| Error::Negotiation)?;
        UnparsedPublicKey::new(&ED25519, pinned_key)
            .verify(
                &signed_bytes(expected, &announcement.certificate)?,
                &signature,
            )
            .map_err(|_| Error::Negotiation)?;
        let certificate = URL_SAFE_NO_PAD
            .decode(announcement.certificate)
            .map_err(|_| Error::Negotiation)?;
        if certificate.is_empty() || certificate.len() > MAX_CERTIFICATE {
            return Err(Error::Negotiation);
        }
        let certificate = CertificateDer::from(certificate);
        let mut roots = RootCertStore::empty();
        roots
            .add(certificate.clone())
            .map_err(|_| Error::Negotiation)?;
        Ok(VerifiedPeer {
            context: expected.clone(),
            endpoint: endpoint(&pinned_key),
            certificate,
        })
    }

    fn matches(&self, remote: &Context) -> bool {
        let mut expected = self.context.clone();
        expected.from_device_id = remote.from_device_id;
        expected == *remote
            && remote.from_device_id != self.context.from_device_id
            && ((remote.from_device_id == remote.host_device_id)
                != (self.context.from_device_id == self.context.host_device_id))
    }

    pub async fn connect<S>(self, peer: VerifiedPeer, stream: S) -> Result<SecureChannel<S>, Error>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        self.handshake(peer, stream, false).await
    }

    pub async fn accept<S>(self, peer: VerifiedPeer, stream: S) -> Result<SecureChannel<S>, Error>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        self.handshake(peer, stream, true).await
    }

    async fn handshake<S>(
        self,
        peer: VerifiedPeer,
        stream: S,
        server: bool,
    ) -> Result<SecureChannel<S>, Error>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        if !self.matches(&peer.context)
            || server != (self.context.from_device_id == self.context.host_device_id)
        {
            return Err(Error::Negotiation);
        }
        let remaining = DEADLINE
            .checked_sub(self.started.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or(Error::Negotiation)?;
        let mut roots = RootCertStore::empty();
        roots
            .add(peer.certificate)
            .map_err(|_| Error::Negotiation)?;
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut result = tokio::time::timeout(remaining.min(Duration::from_secs(15)), async {
            if server {
                let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
                    Arc::new(roots),
                    provider.clone(),
                )
                .build()
                .map_err(|_| Error::Configuration)?;
                let mut config = ServerConfig::builder_with_provider(provider)
                    .with_protocol_versions(&[&rustls::version::TLS13])
                    .map_err(|_| Error::Configuration)?
                    .with_client_cert_verifier(verifier)
                    .with_single_cert(self.chain, self.key)
                    .map_err(|_| Error::Configuration)?;
                config.alpn_protocols = vec![ALPN.to_vec()];
                config.send_tls13_tickets = 0;
                TlsAcceptor::from(Arc::new(config))
                    .accept(stream)
                    .await
                    .map(TlsStream::Server)
                    .map_err(|_| Error::Transport)
            } else {
                let mut config = ClientConfig::builder_with_provider(provider)
                    .with_protocol_versions(&[&rustls::version::TLS13])
                    .map_err(|_| Error::Configuration)?
                    .with_root_certificates(roots)
                    .with_client_auth_cert(self.chain, self.key)
                    .map_err(|_| Error::Configuration)?;
                config.alpn_protocols = vec![ALPN.to_vec()];
                config.resumption = rustls::client::Resumption::disabled();
                TlsConnector::from(Arc::new(config))
                    .connect(
                        ServerName::try_from(NAME).map_err(|_| Error::Configuration)?,
                        stream,
                    )
                    .await
                    .map(TlsStream::Client)
                    .map_err(|_| Error::Transport)
            }
        })
        .await
        .map_err(|_| Error::Transport)??;
        if result.get_ref().1.alpn_protocol() != Some(ALPN) {
            return Err(Error::Negotiation);
        }
        match &mut result {
            TlsStream::Client(stream) => stream.get_mut().1.set_buffer_limit(Some(256 * 1024)),
            TlsStream::Server(stream) => stream.get_mut().1.set_buffer_limit(Some(256 * 1024)),
        }
        Ok(SecureChannel {
            stream: result,
            local: self.context,
            remote: peer.context,
            local_endpoint: self.endpoint,
            remote_endpoint: peer.endpoint,
        })
    }
}

fn endpoint(key: &[u8]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
