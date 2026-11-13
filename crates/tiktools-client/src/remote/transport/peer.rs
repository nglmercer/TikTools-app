use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PeerSession {
    host_id: String,
    principal_id: String,
    session_id: String,
    client_kind: ClientKind,
    #[serde(rename = "accessExpiresAt")]
    _access_expires_at: i64,
}

impl RemoteClient {
    /// Connect using a directory-verified endpoint key. Identity is authenticated
    /// by Iroh; the host selects an already approved local principal/session.
    /// No Cloud request or HTTPS certificate is needed on this data path.
    pub async fn connect_peer(
        endpoint: tiktools_peer::PeerEndpoint,
        address: tiktools_peer::EndpointAddr,
        kind: ClientKind,
        expected_host_id: Option<&str>,
    ) -> Result<Self, ClientError> {
        if kind == ClientKind::Browser {
            return Err(fail("invalid_params", "native client kind is required"));
        }
        let connection = endpoint.connect(address).await.map_err(|_| network())?;
        let mut framed = connection.open().await.map_err(|_| network())?;
        let session: PeerSession = tokio::time::timeout(Duration::from_secs(10), async {
            framed
                .send(tiktools_peer::Frame::Text(
                    json!({"type":"session","clientKind":kind}).to_string(),
                ))
                .await
                .map_err(|_| network())?;
            let response = framed.recv().await.map_err(|_| network())?;
            let tiktools_peer::Frame::Text(raw) = response else {
                return Err(protocol());
            };
            if raw.len() > 4096 {
                return Err(protocol());
            }
            let value: PeerSession = serde_json::from_str(&raw)
                .map_err(|_| fail("unauthorized", "peer session was not admitted"))?;
            Ok(value)
        })
        .await
        .map_err(|_| network())??;
        if !valid_id(&session.host_id)
            || !valid_id(&session.principal_id)
            || !valid_id(&session.session_id)
            || session.client_kind != kind
            || expected_host_id.is_some_and(|host| host != session.host_id)
        {
            connection.close();
            return Err(fail(
                "host_changed",
                "peer session identity does not match the approved profile",
            ));
        }
        let credentials = SessionIdentity {
            host_id: session.host_id,
            principal_id: session.principal_id,
            session_id: session.session_id,
            access_expires_at: None,
            redactor: None,
        };
        let socket = Socket::Peer(socket::PeerSocket {
            framed,
            connection,
            _endpoint: endpoint,
        });
        Self::negotiate(socket, credentials, kind).await
    }
}

#[cfg(test)]
mod tests;

impl RemoteClient {
    /// The channel can only be constructed by signed SDP/ICE negotiation.
    /// The admitted RPC identity still comes from the host's local grant.
    pub async fn connect_rtc(
        channel: impl Into<tiktools_rtc::Channel>,
        kind: ClientKind,
        expected_host_id: Option<&str>,
    ) -> Result<Self, ClientError> {
        let channel = channel.into();
        if kind == ClientKind::Browser
            || channel.remote_context().from_device_id != channel.remote_context().host_device_id
            || channel.local_context().from_device_id == channel.local_context().host_device_id
        {
            return Err(fail(
                "invalid_params",
                "native controller channel is required",
            ));
        }
        let remote = channel
            .remote_endpoint_id()
            .parse()
            .map_err(|_| protocol())?;
        let mut framed = channel.into_transport();
        let session: PeerSession = tokio::time::timeout(Duration::from_secs(10), async {
            framed
                .send(tiktools_rtc::Frame::Text(
                    json!({"type":"session","clientKind":kind}).to_string(),
                ))
                .await
                .map_err(|_| network())?;
            let tiktools_rtc::Frame::Text(raw) = framed.recv().await.map_err(|_| network())? else {
                return Err(protocol());
            };
            if raw.len() > 4096 {
                return Err(protocol());
            }
            serde_json::from_str(&raw)
                .map_err(|_| fail("unauthorized", "WebRTC session was not admitted"))
        })
        .await
        .map_err(|_| network())??;
        if !valid_id(&session.host_id)
            || !valid_id(&session.principal_id)
            || !valid_id(&session.session_id)
            || session.client_kind != kind
            || expected_host_id.is_some_and(|host| host != session.host_id)
        {
            return Err(fail(
                "host_changed",
                "WebRTC identity does not match the approved profile",
            ));
        }
        let credentials = SessionIdentity {
            host_id: session.host_id,
            principal_id: session.principal_id,
            session_id: session.session_id,
            access_expires_at: None,
            redactor: None,
        };
        Self::negotiate(Socket::Rtc { framed, remote }, credentials, kind).await
    }
}
