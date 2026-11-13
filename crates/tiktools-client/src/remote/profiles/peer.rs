use super::*;
use tiktools_peer::{Config, EndpointAddr, PeerEndpoint, TransportAddr};
pub(super) type EndpointCache = std::collections::BTreeMap<PathBuf, (Config, PeerEndpoint)>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerProfile {
    pub key_path: PathBuf,
    pub local_endpoint_id: String,
    pub remote_address: EndpointAddr,
    pub relay_urls: Vec<String>,
    pub cloud_host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_device_id: Option<String>,
}

impl PeerProfile {
    pub(crate) fn validate(&self) -> Result<(), ClientError> {
        let key: tiktools_peer::EndpointId = self
            .local_endpoint_id
            .parse()
            .map_err(|_| fail("profile_invalid", "invalid local peer identity"))?;
        if !self.key_path.is_absolute()
            || key.to_string() != self.local_endpoint_id
            || !valid_id(&self.cloud_host_id)
            || self.host_device_id.as_ref().is_some_and(|id| !valid_id(id))
            || self.remote_address.addrs.len() > 12
            || self.relay_urls.len() > 4
            || self
                .remote_address
                .addrs
                .iter()
                .any(|addr| !matches!(addr, TransportAddr::Ip(_) | TransportAddr::Relay(_)))
        {
            return Err(fail("profile_invalid", "invalid peer profile"));
        }
        let mut approved_relays = std::collections::BTreeSet::new();
        for raw in &self.relay_urls {
            let url = url::Url::parse(raw).map_err(|_| protocol())?;
            if raw.len() > 2048
                || url.scheme() != "https"
                || url.host().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
            {
                return Err(fail("profile_invalid", "invalid peer relay"));
            }
            approved_relays.insert(url.origin().ascii_serialization());
        }
        let mut ip_count = 0;
        let mut relay_count = 0;
        for address in &self.remote_address.addrs {
            match address {
                TransportAddr::Ip(socket) => {
                    ip_count += 1;
                    if socket.port() == 0
                        || socket.ip().is_unspecified()
                        || socket.ip().is_multicast()
                    {
                        return Err(fail("profile_invalid", "invalid remote peer address"));
                    }
                }
                TransportAddr::Relay(relay) => {
                    relay_count += 1;
                    let url = url::Url::parse(relay.as_str()).map_err(|_| protocol())?;
                    if url.path() != "/"
                        || url.query().is_some()
                        || url.fragment().is_some()
                        || !url.username().is_empty()
                        || url.password().is_some()
                        || !approved_relays.contains(&url.origin().ascii_serialization())
                    {
                        return Err(fail("profile_invalid", "remote peer relay is not approved"));
                    }
                }
                _ => return Err(fail("profile_invalid", "unsupported remote peer address")),
            }
        }
        if ip_count > 8 || relay_count > 4 {
            return Err(fail("profile_invalid", "too many remote peer addresses"));
        }
        Ok(())
    }
}

impl ProfileManager {
    /// Save only metadata for an already admitted P2P session. Its private
    /// device key stays in the owner-only native key file, never in this JSON.
    pub async fn install_peer(
        &self,
        name: &str,
        cloud_origin: &str,
        peer: PeerProfile,
        remote: &RemoteClient,
        kind: ClientKind,
    ) -> Result<RemoteProfile, ClientError> {
        peer.validate()?;
        super::super::http::origin(cloud_origin)?;
        let lease = self.lease(name, true).await?;
        if lease.profile.is_some() {
            return Err(fail("profile_exists", "remote profile already exists"));
        }
        let hello = remote.hello();
        if remote.client_kind() != kind
            || kind == ClientKind::Browser
            || remote.peer_endpoint_id() != Some(peer.remote_address.id)
            || peer.host_device_id.is_some() && !remote.uses_rtc()
        {
            return Err(fail(
                "profile_invalid",
                "an admitted native peer connection is required",
            ));
        }
        let mut profile = RemoteProfile {
            version: if peer.host_device_id.is_some() { 3 } else { 2 },
            name: name.into(),
            profile_id: Uuid::new_v4().to_string(),
            origin: cloud_origin.into(),
            host_id: hello.host_id.clone(),
            principal_id: hello.principal_id.clone(),
            session_id: hello.session_id.clone(),
            client_kind: kind,
            trust: TlsTrust::System,
            credential_key: String::new(),
            refresh_attempt: None,
            peer: Some(peer),
        };
        profile.credential_key = credential_key(&profile);
        profile.validate()?;
        let saved = profile.clone();
        tokio::task::spawn_blocking(move || lease.write(&saved))
            .await
            .map_err(|_| profile_error())??;
        Ok(profile)
    }

    #[cfg(not(feature = "cloud-peer"))]
    pub(crate) async fn peer_endpoint(
        &self,
        key_path: &std::path::Path,
        config: Config,
    ) -> Result<PeerEndpoint, ClientError> {
        // Missing approved keys are not silently replaced by a new identity.
        if !key_path.is_file() {
            return Err(fail(
                "peer_identity_missing",
                "the approved peer key is unavailable",
            ));
        }
        let path = key_path.to_path_buf();
        let key = tokio::task::spawn_blocking(move || tiktools_peer::load_key(&path))
            .await
            .map_err(|_| profile_error())?
            .map_err(|_| fail("peer_identity_missing", "the approved peer key is invalid"))?;
        let mut cache = self.peer_endpoints.lock().await;
        let endpoint = if let Some((existing, endpoint)) = cache.get(key_path) {
            if existing != &config {
                return Err(fail(
                    "peer_configuration_changed",
                    "restart the client or forget affected peer profiles before changing relay configuration",
                ));
            }
            if endpoint.id() != key.public() {
                return Err(fail(
                    "peer_identity_changed",
                    "peer key changed while the endpoint was active",
                ));
            }
            endpoint.clone()
        } else {
            if cache.len() >= 32 {
                return Err(fail("overloaded", "peer endpoint cache is full"));
            }
            let endpoint = PeerEndpoint::bind(key, &config)
                .await
                .map_err(|_| network())?;
            cache.insert(key_path.to_path_buf(), (config, endpoint.clone()));
            endpoint
        };
        drop(cache);
        Ok(endpoint)
    }

    #[cfg(not(feature = "cloud-peer"))]
    pub(super) async fn open_peer_profile(
        &self,
        profile: &RemoteProfile,
    ) -> Result<RemoteClient, ClientError> {
        let peer = profile.peer.as_ref().ok_or_else(protocol)?;
        let endpoint = self
            .peer_endpoint(
                &peer.key_path,
                Config {
                    relays: peer.relay_urls.clone(),
                },
            )
            .await?;
        if endpoint.id().to_string() != peer.local_endpoint_id {
            return Err(fail(
                "peer_identity_changed",
                "the peer device key changed; pair again",
            ));
        }
        #[cfg(feature = "cloud-peer")]
        let cloud = self.cloud_context().await;
        // Restore an expired relay lease before dialing. An unavailable Cloud
        // service must not prevent a cached, locally authorized direct session.
        #[cfg(feature = "cloud-peer")]
        cloud.refresh_bounded(profile, &endpoint).await;
        let mut remote = RemoteClient::connect_peer(
            endpoint.clone(),
            peer.remote_address.clone(),
            profile.client_kind,
            Some(&profile.host_id),
        )
        .await?;
        if remote.hello().principal_id != profile.principal_id
            || remote.hello().session_id != profile.session_id
        {
            remote.close();
            return Err(fail(
                "host_changed",
                "peer session no longer matches the approved profile",
            ));
        }
        remote.profile_binding = Some((self.clone(), profile.name.clone()));
        #[cfg(feature = "cloud-peer")]
        cloud.maintain(profile.clone(), endpoint, remote.cancellation());
        Ok(remote)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> PeerProfile {
        let home = std::env::temp_dir().join("tiktools-peer-validation");
        let key = tiktools_peer::SecretKey::generate().public();
        PeerProfile {
            key_path: home.join("identity.key"),
            local_endpoint_id: key.to_string(),
            remote_address: EndpointAddr::new(key),
            relay_urls: vec!["https://relay.example".into()],
            cloud_host_id: Uuid::new_v4().to_string(),
            host_device_id: None,
        }
    }
    #[test]
    fn saved_peer_addresses_are_bounded_and_use_only_approved_relays() {
        let mut saved = profile();
        saved.remote_address.addrs.insert(TransportAddr::Relay(
            "https://relay.example/".parse().unwrap(),
        ));
        saved
            .remote_address
            .addrs
            .insert(TransportAddr::Ip("127.0.0.1:12345".parse().unwrap()));
        assert!(saved.validate().is_ok());
        saved.remote_address.addrs.insert(TransportAddr::Relay(
            "https://foreign.example".parse().unwrap(),
        ));
        assert!(saved.validate().is_err());
        for address in ["0.0.0.0:12345", "127.0.0.1:0", "224.0.0.1:12345"] {
            let mut saved = profile();
            saved
                .remote_address
                .addrs
                .insert(TransportAddr::Ip(address.parse().unwrap()));
            assert!(saved.validate().is_err());
        }
        let mut saved = profile();
        for port in 1..=9 {
            saved
                .remote_address
                .addrs
                .insert(TransportAddr::Ip(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    port,
                ))));
        }
        assert!(saved.validate().is_err());
    }
}

#[cfg(test)]
mod rtc_validation_tests {
    use super::*;
    #[test]
    fn rtc_profiles_require_device_pins_and_exclude_cached_network_routes() {
        let key = tiktools_peer::SecretKey::generate().public();
        let mut profile = RemoteProfile {
            version: 3,
            name: "rtc".into(),
            profile_id: Uuid::new_v4().to_string(),
            origin: "https://cloud.example".into(),
            host_id: Uuid::new_v4().to_string(),
            principal_id: Uuid::new_v4().to_string(),
            session_id: Uuid::new_v4().to_string(),
            client_kind: ClientKind::Desktop,
            trust: TlsTrust::System,
            credential_key: String::new(),
            refresh_attempt: None,
            peer: Some(PeerProfile {
                key_path: std::env::temp_dir().join("identity.key"),
                local_endpoint_id: key.to_string(),
                remote_address: EndpointAddr::new(key),
                relay_urls: vec![],
                cloud_host_id: Uuid::new_v4().to_string(),
                host_device_id: Some(Uuid::new_v4().to_string()),
            }),
        };
        profile.credential_key = credential_key(&profile);
        assert!(profile.validate().is_ok());
        let saved = serde_json::to_value(&profile).unwrap();
        profile.version = 2;
        assert!(profile.validate().is_err());
        profile.version = 3;
        profile
            .peer
            .as_mut()
            .unwrap()
            .remote_address
            .addrs
            .insert(TransportAddr::Ip("127.0.0.1:1234".parse().unwrap()));
        assert!(profile.validate().is_err());
        let mut legacy = saved;
        legacy["version"] = serde_json::json!(2);
        legacy["peer"]
            .as_object_mut()
            .unwrap()
            .remove("hostDeviceId");
        assert!(serde_json::from_value::<RemoteProfile>(legacy)
            .unwrap()
            .validate()
            .is_ok());
    }
}
