//! Typed native discovery and dual-key registration through authenticated Cloud.
use super::*;

/// Native signing callback; the private transport key never enters Cloud JSON.
pub type PeerEndpointSigner<'a> = dyn Fn(&[u8]) -> [u8; 64] + Sync + 'a;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerTransportConfig {
    pub protocol_version: u32,
    pub alpn: String,
    pub relay_urls: Vec<String>,
    pub max_offline_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerEndpointRecord {
    pub device_id: String,
    pub endpoint_id: String,
    pub addresses: Vec<String>,
    pub relay_urls: Vec<String>,
    pub revision: i64,
    pub kind: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerDirectory {
    pub host_id: String,
    pub items: Vec<PeerEndpointRecord>,
    pub relay_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerPublished {
    pub device_id: String,
    pub endpoint_id: String,
    pub revision: i64,
    pub ttl_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerPublication {
    pub device_id: String,
    pub endpoint_id: String,
    pub addresses: Vec<String>,
    pub relay_urls: Vec<String>,
    pub expected_revision: i64,
}

impl PeerEndpointRecord {
    pub fn validate(&self) -> Result<(), OperationError> {
        valid_uuid(&self.device_id)?;
        one_of(&self.kind, &["host", "controller"], "peer device kind")?;
        validate_endpoint(&self.endpoint_id, &self.addresses, &self.relay_urls)?;
        if self.revision <= 0 || self.expires_at.is_empty() || self.expires_at.len() > 64 {
            return Err(OperationError::internal("Cloud sent invalid peer metadata"));
        }
        Ok(())
    }
}

fn validate_endpoint(
    endpoint: &str,
    addresses: &[String],
    relays: &[String],
) -> Result<(), OperationError> {
    if endpoint.len() != 64
        || !endpoint
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || addresses.len() > 8
        || relays.len() > 4
    {
        return Err(OperationError::invalid("invalid peer endpoint"));
    }
    for address in addresses {
        let socket: std::net::SocketAddr = address
            .parse()
            .map_err(|_| OperationError::invalid("invalid peer address"))?;
        if socket.port() == 0 || socket.ip().is_unspecified() || socket.ip().is_multicast() {
            return Err(OperationError::invalid("invalid peer address"));
        }
    }
    validate_relays(relays)
}

fn validate_relays(relays: &[String]) -> Result<(), OperationError> {
    if relays.len() > 4 {
        return Err(OperationError::invalid("too many peer relays"));
    }
    for raw in relays {
        let url =
            url::Url::parse(raw).map_err(|_| OperationError::invalid("invalid peer relay"))?;
        if raw.len() > 2048
            || url.scheme() != "https"
            || url.host().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
        {
            return Err(OperationError::invalid(
                "peer relay must be an HTTPS origin",
            ));
        }
    }
    Ok(())
}

impl CloudTeams {
    pub async fn peer_config(
        &self,
        auth: &CloudAuth,
    ) -> Result<PeerTransportConfig, OperationError> {
        let (origin, token) = credentials(auth).await?;
        let config: PeerTransportConfig = decode(
            send(
                Method::GET,
                &origin,
                &token,
                "/api/v3/remote/peer-config",
                None,
            )
            .await?,
        )?;
        if config.protocol_version != 1
            || config.alpn != "tiktools/collaboration/1"
            || config.max_offline_seconds != 28800
        {
            return Err(OperationError::unavailable(
                "Cloud peer protocol is not supported",
            ));
        }
        validate_relays(&config.relay_urls)?;
        Ok(config)
    }

    pub async fn peer_directory(
        &self,
        auth: &CloudAuth,
        host: &str,
    ) -> Result<PeerDirectory, OperationError> {
        valid_uuid(host)?;
        let (origin, token) = credentials(auth).await?;
        let directory: PeerDirectory = decode(
            send(
                Method::GET,
                &origin,
                &token,
                &format!("/api/v3/remote/hosts/{host}/peers"),
                None,
            )
            .await?,
        )?;
        if directory.host_id != host || directory.items.len() > 64 {
            return Err(OperationError::internal(
                "Cloud sent invalid peer discovery",
            ));
        }
        validate_relays(&directory.relay_urls)?;
        for item in &directory.items {
            item.validate()?;
        }
        Ok(directory)
    }

    pub async fn own_peer_endpoint(
        &self,
        auth: &CloudAuth,
        device: &str,
    ) -> Result<PeerEndpointRecord, OperationError> {
        valid_uuid(device)?;
        let (origin, token) = credentials(auth).await?;
        let record: PeerEndpointRecord = decode(
            send(
                Method::GET,
                &origin,
                &token,
                &format!("/api/v3/remote/peers/{device}"),
                None,
            )
            .await?,
        )?;
        record.validate()?;
        if record.device_id != device {
            return Err(OperationError::internal("Cloud changed the peer device"));
        }
        Ok(record)
    }

    pub async fn publish_peer(
        &self,
        auth: &CloudAuth,
        publication: &PeerPublication,
        sign_endpoint: &PeerEndpointSigner<'_>,
    ) -> Result<PeerPublished, OperationError> {
        valid_uuid(&publication.device_id)?;
        validate_endpoint(
            &publication.endpoint_id,
            &publication.addresses,
            &publication.relay_urls,
        )?;
        let next_revision = publication
            .expected_revision
            .checked_add(1)
            .ok_or_else(|| OperationError::invalid("peer revision exhausted"))?;
        if publication.expected_revision < 0 {
            return Err(OperationError::invalid("invalid peer revision"));
        }
        let body = serde_json::to_value(publication)
            .map_err(|_| OperationError::invalid("invalid peer metadata"))?;
        let result: PeerPublished = decode(
            self.signed_peer_operation(
                auth,
                "peer",
                "/api/v3/remote/peers",
                body,
                Some(sign_endpoint),
            )
            .await?,
        )?;
        if result.device_id != publication.device_id
            || result.endpoint_id != publication.endpoint_id
            || result.revision != next_revision
            || result.ttl_seconds != 300
        {
            return Err(OperationError::internal(
                "Cloud sent invalid peer publication result",
            ));
        }
        Ok(result)
    }

    pub async fn confirm_peer_grant(
        &self,
        auth: &CloudAuth,
        host: &str,
        device: &str,
        grant: &str,
        revision: i64,
    ) -> Result<(), OperationError> {
        for id in [host, device, grant] {
            valid_uuid(id)?;
        }
        if revision <= 0 {
            return Err(OperationError::invalid("invalid grant revision"));
        }
        let result = self
            .signed_peer_operation(
                auth,
                "peer-grant",
                "/api/v3/remote/peer-grants/confirm",
                json!({"hostId":host,"deviceId":device,"grantId":grant,"revision":revision}),
                None,
            )
            .await?;
        if result["grantId"] != grant
            || result["revision"] != revision
            || result["status"] != "active"
        {
            return Err(OperationError::internal(
                "Cloud sent invalid grant confirmation",
            ));
        }
        Ok(())
    }

    async fn signed_peer_operation(
        &self,
        auth: &CloudAuth,
        purpose: &str,
        path: &str,
        binding: Value,
        sign_endpoint: Option<&PeerEndpointSigner<'_>>,
    ) -> Result<Value, OperationError> {
        let key = self.enrolled_device_key()?;
        let (origin, token) = credentials(auth).await?;
        let challenge = send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/challenges",
            Some(json!({"purpose":purpose,"binding":binding})),
        )
        .await?;
        let id = str_field(&challenge, "id")?;
        let nonce = URL_SAFE_NO_PAD
            .decode(str_field(&challenge, "nonce")?)
            .map_err(|_| OperationError::internal("Cloud sent an invalid peer challenge"))?;
        let digest = str_field(&challenge, "bindingSha256")?;
        if nonce.len() != 32
            || digest != binding_digest(&binding)
            || challenge["purpose"] != purpose
        {
            return Err(OperationError::internal(
                "Cloud changed the peer challenge binding",
            ));
        }
        let message = proof_message(purpose, &nonce, &digest);
        let mut request = json!({"challengeId":id,"binding":binding,"signature":URL_SAFE_NO_PAD.encode(key.sign(&message).as_ref())});
        if let Some(sign) = sign_endpoint {
            request["endpointSignature"] = json!(URL_SAFE_NO_PAD.encode(sign(&message)));
        }
        send(Method::POST, &origin, &token, path, Some(request)).await
    }
}
