use super::*;
use secrecy::ExposeSecret;
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tiktools_collaboration::{hash_secret, Collect, Credentials, Redeem};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub protocol_min: u32,
    pub protocol_max: u32,
    pub host_id: String,
    pub name: String,
    pub certificate_fingerprint: Option<String>,
}
#[derive(Clone)]
pub struct RemoteEndpoint {
    pub(super) origin: String,
    pub(super) tls: Arc<rustls::ClientConfig>,
    pub(super) http: reqwest::Client,
    pub(super) trust: TlsTrust,
}
impl std::fmt::Debug for RemoteEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteEndpoint")
            .field("origin", &self.origin)
            .field("trust", &self.trust)
            .finish()
    }
}
pub(super) fn origin(input: &str) -> Result<url::Url, ClientError> {
    let url = url::Url::parse(input)
        .map_err(|_| fail("invalid_params", "an HTTPS origin is required"))?;
    if url.scheme() != "https"
        || url.host().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(fail(
            "invalid_params",
            "use an HTTPS origin without credentials, paths, query or fragment",
        ));
    }
    Ok(url)
}
impl RemoteEndpoint {
    pub async fn new(url: &str, trust: TlsTrust) -> Result<Self, ClientError> {
        let origin = origin(url)?.origin().ascii_serialization();
        let selection = trust.clone();
        let tls = tokio::task::spawn_blocking(move || selection.config())
            .await
            .map_err(|_| network())??;
        let http = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(10))
            .tls_backend_preconfigured((*tls).clone())
            .build()
            .map_err(|_| fail("trust", "TLS client configuration is unavailable"))?;
        Ok(Self {
            origin,
            tls,
            http,
            trust,
        })
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn trust(&self) -> &TlsTrust {
        &self.trust
    }
    pub async fn info(&self) -> Result<HostInfo, ClientError> {
        let (status, value) = self
            .json(self.http.get(format!("{}/remote/v1/info", self.origin)))
            .await?;
        if status != reqwest::StatusCode::OK {
            return Err(http_error(&value));
        }
        let info: HostInfo = serde_json::from_value(value).map_err(|_| protocol())?;
        if !valid_id(&info.host_id)
            || info.protocol_min > 1
            || info.protocol_max < 1
            || info.name.len() > 128
        {
            return Err(fail(
                "protocol_incompatible",
                "remote protocol version 1 is required",
            ));
        }
        Ok(info)
    }
    pub(super) async fn verify_host(
        &self,
        credentials: &NativeCredentials,
    ) -> Result<(), ClientError> {
        if credentials.origin != self.origin {
            return Err(fail(
                "host_changed",
                "credentials belong to a different endpoint",
            ));
        }
        if self.info().await?.host_id != credentials.host_id {
            return Err(fail(
                "host_changed",
                "remote host identity changed; pair explicitly again",
            ));
        }
        Ok(())
    }
    pub async fn begin_pairing(
        &self,
        invite_id: &str,
        invite_secret: &SecretString,
        label: &str,
        kind: ClientKind,
    ) -> Result<PendingPairing, ClientError> {
        if !valid_id(invite_id)
            || invite_secret.expose_secret().len() != 64
            || label.trim().is_empty()
            || label.len() > 128
            || kind == ClientKind::Browser
        {
            return Err(fail("invalid_params", "invalid native pairing request"));
        }
        let info = self.info().await?;
        let exchange = secret()?;
        let input = Redeem {
            invite_id: invite_id.into(),
            secret: invite_secret.expose_secret().into(),
            client_label: label.into(),
            client_kind: kind,
            exchange_secret_hash: hash_secret(exchange.expose_secret()),
        };
        let (status, value) = self
            .json(
                self.http
                    .post(format!("{}/remote/v1/pair/redeem", self.origin))
                    .header("x-tiktools-client", "native")
                    .json(&input),
            )
            .await
            .map_err(|_| {
                fail(
                    "pairing_uncertain",
                    "pairing request was not confirmed; request a new invitation",
                )
            })?;
        if status != reqwest::StatusCode::OK {
            return Err(http_error(&value));
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Pending {
            request_id: String,
            expires_at: i64,
        }
        let p: Pending = serde_json::from_value(value).map_err(|_| protocol())?;
        if !valid_id(&p.request_id) || p.expires_at <= now() {
            return Err(protocol());
        }
        Ok(PendingPairing {
            endpoint: self.clone(),
            host_id: info.host_id,
            request_id: p.request_id,
            expires_at: p.expires_at,
            exchange,
            terminal: false,
        })
    }
    /// One-use rotation for explicit in-memory sessions. Persistent callers
    /// should use ProfileManager, which durably fences ambiguous rotations.
    pub async fn rotate_credentials(
        &self,
        credentials: &NativeCredentials,
    ) -> Result<NativeCredentials, ClientError> {
        self.verify_host(credentials).await?;
        let request = self
            .http
            .post(format!("{}/remote/v1/session/refresh", self.origin))
            .header("x-tiktools-client", "native")
            .json(&serde_json::json!({"refreshToken":credentials.refresh.expose_secret()}));
        let (status, value) = self.json(request).await?;
        if status != reqwest::StatusCode::OK {
            return Err(http_error(&value));
        }
        let wire: Credentials = serde_json::from_value(value).map_err(|_| protocol())?;
        if wire.principal_id != credentials.principal_id
            || wire.session_id != credentials.session_id
        {
            return Err(protocol());
        }
        NativeCredentials::issued(&self.origin, &credentials.host_id, wire)
    }
    pub async fn logout(&self, credentials: &NativeCredentials) -> Result<(), ClientError> {
        self.verify_host(credentials).await?;
        let (status, value) = self
            .json(
                self.http
                    .post(format!("{}/remote/v1/session/logout", self.origin))
                    .header(
                        reqwest::header::AUTHORIZATION,
                        bearer(credentials.access.expose_secret())?,
                    ),
            )
            .await?;
        if status != reqwest::StatusCode::OK {
            return Err(http_error(&value));
        }
        Ok(())
    }
    pub(super) async fn json(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<(reqwest::StatusCode, serde_json::Value), ClientError> {
        let operation = async {
            let mut response = request.send().await.map_err(|_| network())?;
            let status = response.status();
            if status.is_redirection() {
                return Err(fail("trust", "remote redirects are not followed"));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| network())? {
                if bytes.len().saturating_add(chunk.len()) > 16 * 1024 {
                    return Err(protocol());
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok((
                status,
                serde_json::from_slice(&bytes).map_err(|_| protocol())?,
            ))
        };
        tokio::time::timeout(Duration::from_secs(10), operation)
            .await
            .map_err(|_| network())?
    }
}
pub(super) fn bearer(secret: &str) -> Result<reqwest::header::HeaderValue, ClientError> {
    let mut h = reqwest::header::HeaderValue::from_str(&format!("Bearer {secret}"))
        .map_err(|_| protocol())?;
    h.set_sensitive(true);
    Ok(h)
}
fn http_error(value: &serde_json::Value) -> ClientError {
    let code = value
        .pointer("/error/code")
        .and_then(serde_json::Value::as_str)
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 64
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
        .unwrap_or("protocol");
    fail(code, "remote host rejected the session request")
}
pub enum PairingPoll {
    Pending,
    Approved(NativeCredentials),
}
pub struct PendingPairing {
    endpoint: RemoteEndpoint,
    host_id: String,
    pub request_id: String,
    pub expires_at: i64,
    exchange: SecretString,
    terminal: bool,
}
impl PendingPairing {
    pub async fn poll(&mut self) -> Result<PairingPoll, ClientError> {
        if self.terminal {
            return Err(fail(
                "pairing_uncertain",
                "pairing collection already ended; request a new invitation",
            ));
        }
        if self.expires_at <= now() {
            self.terminal = true;
            return Err(fail("pairing_expired", "pairing expired"));
        }
        let input = Collect {
            request_id: self.request_id.clone(),
            exchange_secret: self.exchange.expose_secret().into(),
        };
        let response = self
            .endpoint
            .json(
                self.endpoint
                    .http
                    .post(format!("{}/remote/v1/pair/status", self.endpoint.origin))
                    .header("x-tiktools-client", "native")
                    .json(&input),
            )
            .await;
        match response {
            Ok((reqwest::StatusCode::ACCEPTED, _)) => Ok(PairingPoll::Pending),
            Ok((status, value)) => {
                self.terminal = true;
                if status != reqwest::StatusCode::OK {
                    return Err(http_error(&value));
                }
                let wire: Credentials = serde_json::from_value(value).map_err(|_| protocol())?;
                Ok(PairingPoll::Approved(NativeCredentials::issued(
                    &self.endpoint.origin,
                    &self.host_id,
                    wire,
                )?))
            }
            Err(_) => {
                self.terminal = true;
                Err(fail(
                    "pairing_uncertain",
                    "credential collection was not confirmed; request a new invitation",
                ))
            }
        }
    }
}
