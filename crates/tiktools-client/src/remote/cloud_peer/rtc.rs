//! Cloud discovery/signaling only. RPC traffic and its receipts stay native.
use super::*;
use tiktools_rtc::{
    cloud_socket::{Connection, Event as CloudEvent, Role},
    negotiation::{Event, Negotiation},
    signaling::Context,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
mod connection;

impl MaintenanceContext {
    pub(crate) async fn dial_rtc(
        &self,
        host: &str,
        kind: ClientKind,
        expected: Option<&PeerProfile>,
        native_host: Option<&str>,
        origin: Option<&str>,
    ) -> Result<(RemoteClient, PeerProfile), ClientError> {
        let captured = self
            .auth
            .session()
            .await
            .ok_or_else(|| fail("sign_in_required", "sign in to Cloud first"))?;
        let operation = tokio::time::timeout(
            Duration::from_secs(60),
            self.dial_inner(host, kind, expected, native_host, origin),
        );
        tokio::pin!(operation);
        let mut fence = tokio::time::interval(Duration::from_millis(500));
        loop {
            tokio::select! {
                biased;
                _=fence.tick()=>{if !self.auth.is_current_session(&captured){return Err(fail("account_changed","Cloud account changed during connection"));}},
                result=&mut operation=>return result.map_err(|_|network())?,
            }
        }
    }
    async fn dial_inner(
        &self,
        host: &str,
        kind: ClientKind,
        expected: Option<&PeerProfile>,
        native_host: Option<&str>,
        origin: Option<&str>,
    ) -> Result<(RemoteClient, PeerProfile), ClientError> {
        if kind == ClientKind::Browser || !super::super::valid_id(host) {
            return Err(protocol());
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or_else(|| fail("sign_in_required", "sign in to Cloud first"))?;
        let current_origin = session.instance_origin();
        let account = session.account_id().to_owned();
        if origin.is_some_and(|origin| origin != current_origin) {
            return Err(fail("account_changed", "Cloud origin changed"));
        }
        let (teams, data) = scoped(&self.data, &current_origin, &account);
        let device = teams.controller_device().map_err(cloud)?;
        let discovery = teams.rtc_config(&self.auth).await.map_err(cloud)?;
        let path = data.join("identity.key");
        if expected.is_some_and(|peer| peer.key_path != path || peer.cloud_host_id != host) {
            return Err(fail(
                "account_changed",
                "profile does not belong to this Cloud account",
            ));
        }
        let previous = match teams.own_peer_endpoint(&self.auth, &device).await {
            Ok(record) => Some(record),
            Err(error) if error.code == "not_found" => None,
            Err(error) => return Err(cloud(error)),
        };
        if expected.is_some() && previous.is_none() {
            return Err(fail(
                "peer_identity_changed",
                "registered peer identity is unavailable",
            ));
        }
        let approved = previous.is_some();
        let key_path = path.clone();
        let key = Arc::new(
            tokio::task::spawn_blocking(move || {
                if approved {
                    tiktools_peer::load_key(&key_path)
                } else {
                    tiktools_peer::load_or_create_key(&key_path)
                }
            })
            .await
            .map_err(|_| protocol())?
            .map_err(|_| fail("peer_identity_missing", "approved peer key is unavailable"))?,
        );
        let local = key.public().to_string();
        if previous
            .as_ref()
            .is_some_and(|record| record.endpoint_id != local)
            || expected.is_some_and(|peer| peer.local_endpoint_id != local)
        {
            return Err(fail("peer_identity_changed", "registered peer key changed"));
        }
        if let Some(peer) = expected {
            self.binding(peer, &current_origin, &account, &local)?;
        }
        let publication = PeerPublication {
            device_id: device.clone(),
            endpoint_id: local.clone(),
            addresses: vec![],
            relay_urls: vec![],
            expected_revision: previous.map_or(0, |record| record.revision),
        };
        teams
            .publish_peer(&self.auth, &publication, &|bytes| {
                key.sign(bytes).to_bytes()
            })
            .await
            .map_err(cloud)?;
        let directory = teams
            .peer_directory(&self.auth, host)
            .await
            .map_err(cloud)?;
        if !directory
            .items
            .iter()
            .any(|item| item.device_id == device && item.endpoint_id == local)
        {
            return Err(fail(
                "approval_pending",
                "wait for the owner and host to approve this device",
            ));
        }
        let target = directory
            .items
            .iter()
            .find(|item| item.kind == "host")
            .ok_or_else(|| fail("unavailable", "the native host is not available"))?;
        let remote_key: tiktools_peer::EndpointId =
            target.endpoint_id.parse().map_err(|_| protocol())?;
        if expected.is_some_and(|peer| {
            peer.remote_address.id != remote_key
                || peer
                    .host_device_id
                    .as_ref()
                    .is_some_and(|id| id != &target.device_id)
        }) {
            return Err(fail(
                "host_changed",
                "Cloud host device or key changed; review the pairing",
            ));
        }
        let ticket = teams
            .issue_ticket_as(&self.auth, "controller", host, &device)
            .await
            .map_err(cloud)?;
        let _permit = self
            .auth
            .try_session_emission_permit(&session)
            .map_err(|_| fail("account_changed", "Cloud account changed"))?;
        let cloud_host: Uuid = host.parse().map_err(|_| protocol())?;
        let socket = Connection::connect(&current_origin, &ticket, Role::Controller, cloud_host)
            .await
            .map_err(|_| network())?;
        let context = Context {
            host_id: cloud_host,
            session_id: socket.ready().session_id.ok_or_else(protocol)?,
            host_device_id: target.device_id.parse().map_err(|_| protocol())?,
            from_device_id: device.parse().map_err(|_| protocol())?,
            negotiation_id: Uuid::new_v4(),
            grant_revision: socket.ready().grant_revision.ok_or_else(protocol)?,
        };
        let channel = connection::connect(
            socket,
            context,
            &key.to_bytes(),
            *remote_key.as_bytes(),
            tiktools_rtc::Config {
                ice_servers: discovery
                    .stun_urls
                    .into_iter()
                    .map(|url| tiktools_rtc::IceServer {
                        urls: vec![url],
                        username: String::new(),
                        credential: String::new(),
                    })
                    .collect(),
                ..Default::default()
            },
        )
        .await?;
        let remote = RemoteClient::connect_rtc(channel, kind, native_host).await?;
        let metadata = PeerProfile {
            key_path: path,
            local_endpoint_id: local,
            remote_address: EndpointAddr::new(remote_key),
            relay_urls: vec![],
            cloud_host_id: host.into(),
            host_device_id: Some(target.device_id.clone()),
        };
        self.clone().maintain_rtc(
            metadata.clone(),
            key,
            current_origin,
            account,
            remote.cancellation(),
        );
        Ok((remote, metadata))
    }
    fn maintain_rtc(
        self,
        profile: PeerProfile,
        key: Arc<tiktools_peer::SecretKey>,
        origin: String,
        account: String,
        stop: CancellationToken,
    ) {
        tokio::spawn(async move {
            let identity = self.clone();
            let expected_origin = origin.clone();
            let expected_account = account.clone();
            maintenance::maintain(
                stop.clone(),
                || {
                    identity
                        .auth
                        .with_sync_identity_permit(|current| {
                            current.is_some_and(|(origin, account)| {
                                origin == expected_origin && account == expected_account
                            })
                        })
                        .unwrap_or(false)
                },
                || async {
                    let session = self.auth.session().await.ok_or_else(network)?;
                    if session.instance_origin() != origin || session.account_id() != account {
                        return Err(fail("account_changed", "Cloud account changed"));
                    }
                    let _permit = self
                        .auth
                        .try_session_emission_permit(&session)
                        .map_err(|_| network())?;
                    let (teams, device) =
                        self.binding(&profile, &origin, &account, &key.public().to_string())?;
                    let previous = teams
                        .own_peer_endpoint(&self.auth, &device)
                        .await
                        .map_err(cloud)?;
                    if previous.endpoint_id != profile.local_endpoint_id {
                        stop.cancel();
                        return Err(fail("peer_identity_changed", "registered key changed"));
                    }
                    let publication = PeerPublication {
                        device_id: device,
                        endpoint_id: profile.local_endpoint_id.clone(),
                        addresses: vec![],
                        relay_urls: vec![],
                        expected_revision: previous.revision,
                    };
                    teams
                        .publish_peer(&self.auth, &publication, &|bytes| {
                            key.sign(bytes).to_bytes()
                        })
                        .await
                        .map_err(cloud)?;
                    Ok(())
                },
            )
            .await;
        });
    }
}
