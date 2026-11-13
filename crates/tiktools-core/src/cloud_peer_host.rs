//! Trusted native Cloud coordination. Never registered as a remote RPC method.
mod policy;
mod rtc;
use crate::{
    cloud_account::CloudAccount,
    cloud_teams::{self, CloudTeams, PeerEndpointSigner, PeerPublication},
    control::OperationError,
    AppCore,
};
use std::sync::Arc;
use tiktools_collaboration::{
    CloudPeerGrant, CloudPeerObservation, CloudPeerSource, CollaborationService,
};

#[derive(Clone)]
pub struct CloudPeerHostBridge {
    account: Arc<CloudAccount>,
    teams: Arc<CloudTeams>,
    service: Arc<CollaborationService>,
    host: String,
    device: String,
}
fn local(error: tiktools_collaboration::Error) -> OperationError {
    OperationError {
        code: error.code(),
        message: error.to_string(),
        data: None,
    }
}
/// Unsupported grants stay unapproved without blocking other valid devices.
#[derive(Default)]
pub struct CloudPeerSyncReport {
    pub unsupported_grants: usize,
}
impl AppCore {
    /// Native supervision hook; does not create collaboration storage.
    pub fn retain_cloud_peer_context(&self) -> Result<(), OperationError> {
        // Manually hosted native transports have no Cloud coordinator. They
        // must not acquire account lifecycle behavior just by enabling P2P.
        let Some((host_id, _)) = self.teams.host_binding()? else {
            return Ok(());
        };
        let service = self
            .collaboration
            .lock()
            .map_err(|_| OperationError::internal("collaboration lock unavailable"))?
            .clone();
        let Some(service) = service else {
            return Ok(());
        };
        self.cloud
            .auth()
            .with_sync_identity_permit(|identity| {
                let source = identity.map(|(origin, account_id)| CloudPeerSource {
                    origin,
                    account_id,
                    host_id,
                });
                service
                    .retain_cloud_peer_source(source.as_ref())
                    .map_err(local)
            })
            .map_err(|_| OperationError::unavailable("Cloud account is changing"))?
    }
    pub fn cloud_peer_host_bridge(&self) -> Result<Option<CloudPeerHostBridge>, OperationError> {
        let Some((host, device)) = self.teams.host_binding()? else {
            return Ok(None);
        };
        Ok(Some(CloudPeerHostBridge {
            account: self.cloud.clone(),
            teams: self.teams.clone(),
            service: self.collaboration().map_err(local)?,
            host,
            device,
        }))
    }
}
impl AppCore {
    pub(crate) async fn start_cloud_peer_host(
        self: &Arc<Self>,
    ) -> Result<crate::cloud_remote_host::RemoteHostStatus, OperationError> {
        let bridge = self
            .cloud_peer_host_bridge()?
            .ok_or_else(|| OperationError::invalid("enroll this PC as a team host first"))?;
        let captured = self
            .cloud
            .auth()
            .session()
            .await
            .ok_or_else(|| OperationError::unavailable("sign in to Cloud first"))?;
        let transport = self.teams.peer_config(self.cloud.auth()).await?;
        let snapshot = self
            .teams
            .peer_authorizations(self.cloud.auth(), &bridge.host)
            .await?;
        if snapshot.host_status != "active" {
            return Err(OperationError::forbidden("Cloud team is disabled"));
        }
        bridge
            .commit(captured, move |bridge| {
                let (mut configuration, revision) =
                    bridge.service.configuration_snapshot().map_err(local)?;
                configuration.enabled = true;
                configuration.peer = Some(tiktools_collaboration::PeerConfiguration {
                    relay_urls: transport.relay_urls,
                });
                bridge
                    .service
                    .configure_checked(configuration, Some(&revision))
                    .map_err(local)?;
                Ok(())
            })
            .await?;
        self.remote_host.stop();
        Ok(self
            .peer_host_status()
            .unwrap_or(crate::cloud_remote_host::RemoteHostStatus {
                state: "connecting".into(),
                host_id: Some(bridge.host),
                last_error: None,
            }))
    }
    pub(crate) fn peer_host_status(&self) -> Option<crate::cloud_remote_host::RemoteHostStatus> {
        if !self
            .collaboration_peer_available
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return None;
        }
        let service = self.collaboration.lock().ok()?.as_ref()?.clone();
        let configuration = service.configuration().ok()?;
        configuration.peer.as_ref()?;
        let status = service.peer_gateway_status().ok()?;
        Some(crate::cloud_remote_host::RemoteHostStatus {
            state: if !configuration.enabled {
                "stopped"
            } else if status.gateway.running {
                "ready"
            } else {
                "connecting"
            }
            .into(),
            host_id: self
                .teams
                .host_binding()
                .ok()
                .flatten()
                .map(|(host, _)| host),
            last_error: status.gateway.last_error,
        })
    }
    pub(crate) fn stop_cloud_peer_host(&self) -> Result<(), OperationError> {
        if self.peer_host_status().is_none() {
            return Ok(());
        }
        let service = self.collaboration().map_err(local)?;
        let (mut configuration, revision) = service.configuration_snapshot().map_err(local)?;
        configuration.peer = None;
        configuration.enabled &= configuration.advertised_origin.is_some();
        service
            .configure_checked(configuration, Some(&revision))
            .map_err(local)?;
        service.retain_cloud_peer_source(None).map_err(local)
    }
}

impl CloudPeerHostBridge {
    pub fn device_id(&self) -> &str {
        &self.device
    }
    /// A synchronous account fence, called on the blocking pool even when
    /// Cloud is unreachable. Remembered offline identity does not renew leases.
    pub fn retain_context(&self) -> Result<(), OperationError> {
        self.account
            .auth()
            .with_sync_identity_permit(|identity| {
                let source = identity.map(|(origin, account_id)| CloudPeerSource {
                    origin,
                    account_id,
                    host_id: self.host.clone(),
                });
                self.service
                    .retain_cloud_peer_source(source.as_ref())
                    .map_err(local)
            })
            .map_err(|_| OperationError::unavailable("Cloud account is changing"))?
    }
    pub async fn synchronize(
        &self,
        mut publication: PeerPublication,
        sign: &PeerEndpointSigner<'_>,
    ) -> Result<CloudPeerSyncReport, OperationError> {
        if publication.device_id != self.device {
            return Err(OperationError::invalid("peer host device changed"));
        }
        let auth = self.account.auth();
        let captured = auth.session().await.ok_or_else(|| {
            OperationError::unavailable("sign in to Cloud to refresh peer authorization")
        })?;
        let source = CloudPeerSource {
            origin: captured.instance_origin(),
            account_id: captured.account_id().into(),
            host_id: self.host.clone(),
        };
        let mut snapshot = self.teams.peer_authorizations(auth, &self.host).await?;
        let initial = snapshot.clone();
        let initial_source = source.clone();
        self.commit(captured.clone(), move |bridge| {
            bridge.reconcile(&initial_source, &initial)
        })
        .await?;
        if snapshot.host_status != "active" {
            return Err(OperationError::forbidden("Cloud team is disabled"));
        }
        // Recover the CAS revision after restart or a lost publication reply.
        publication.expected_revision = match self.teams.own_peer_endpoint(auth, &self.device).await
        {
            Ok(record) if record.endpoint_id == publication.endpoint_id => record.revision,
            Ok(_) => {
                return Err(OperationError::conflict(
                    "peer identity changed; reconcile the enrolled device",
                ))
            }
            Err(error) if error.code == "not_found" => 0,
            Err(error) => return Err(error),
        };
        self.teams.publish_peer(auth, &publication, sign).await?;
        // Confirm only revisions whose entire native policy can be understood.
        // Never approve an unsupported policy merely because it has a role name.
        for grant in snapshot
            .items
            .iter()
            .filter(|g| g.status == "pendingHost" && g.device_status == "active")
            .take(20)
        {
            if policy::grant(grant).is_err() {
                continue;
            }
            self.teams
                .confirm_peer_grant(auth, &self.host, &self.device, &grant.id, grant.revision)
                .await?;
        }
        snapshot = self.teams.peer_authorizations(auth, &self.host).await?;
        self.commit(captured, move |bridge| {
            bridge.reconcile(&source, &snapshot)?;
            if snapshot.host_status != "active" {
                return Ok(CloudPeerSyncReport::default());
            }
            let mut unsupported = 0;
            for item in &snapshot.items {
                if !matches!(item.status.as_str(), "active" | "pendingHost")
                    || item.device_status != "active"
                {
                    continue;
                }
                let (grant, kind) = match policy::grant(item) {
                    Ok(policy) => policy,
                    Err(_) => {
                        unsupported += 1;
                        continue;
                    }
                };
                if item.status != "active" || item.confirmed_revision != Some(item.revision) {
                    continue;
                }
                let Some(endpoint) = &item.endpoint_id else {
                    continue;
                };
                let input = CloudPeerGrant {
                    origin: source.origin.clone(),
                    account_id: source.account_id.clone(),
                    host_id: bridge.host.clone(),
                    device_id: item.device_id.clone(),
                    grant_id: item.id.clone(),
                    cloud_revision: item.revision,
                    endpoint_id: endpoint.clone(),
                    label: item.device_label.clone(),
                    client_kind: kind,
                    grant,
                };
                // Local Owner edits/revocation take precedence over Cloud renewal.
                match bridge.service.apply_cloud_peer_grant(input) {
                    Ok(_) => {}
                    Err(
                        tiktools_collaboration::Error::Unauthorized
                        | tiktools_collaboration::Error::RevisionConflict,
                    ) => {}
                    Err(error) => return Err(local(error)),
                }
            }
            Ok(CloudPeerSyncReport {
                unsupported_grants: unsupported,
            })
        })
        .await
    }
    fn reconcile(
        &self,
        source: &CloudPeerSource,
        snapshot: &cloud_teams::PeerAuthorizationSnapshot,
    ) -> Result<(), OperationError> {
        let observations = snapshot
            .items
            .iter()
            .map(|item| CloudPeerObservation {
                device_id: item.device_id.clone(),
                grant_id: item.id.clone(),
                revision: item.revision,
                approved: snapshot.host_status == "active"
                    && item.device_status == "active"
                    && item.status == "active"
                    && item.confirmed_revision == Some(item.revision)
                    && policy::grant(item).is_ok(),
                revoked: item.status == "revoked" || item.device_status == "revoked",
            })
            .collect::<Vec<_>>();
        self.service
            .reconcile_cloud_peers(source, &observations)
            .map_err(local)
    }
    async fn commit<R: Send + 'static>(
        &self,
        captured: Arc<cloud_client::Session>,
        operation: impl FnOnce(&Self) -> Result<R, OperationError> + Send + 'static,
    ) -> Result<R, OperationError> {
        let bridge = self.clone();
        tokio::task::spawn_blocking(move || {
            bridge
                .account
                .auth()
                .with_current_session_permit(&captured, || {
                    if bridge.teams.host_binding()?.as_ref()
                        != Some(&(bridge.host.clone(), bridge.device.clone()))
                    {
                        return Err(OperationError::conflict("Cloud host binding changed"));
                    }
                    operation(&bridge)
                })
                .map_err(|_| {
                    OperationError::unavailable("Cloud account changed during peer synchronization")
                })?
        })
        .await
        .map_err(|_| OperationError::internal("peer authorization task failed"))?
    }
}
