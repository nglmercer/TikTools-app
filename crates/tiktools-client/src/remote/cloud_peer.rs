//! Cloud-linked native controller. Owns metadata and keys, never AppCore/LIVE.
use super::profiles::{PeerProfile, ProfileManager, RemoteProfile};
use super::{fail, network, protocol, ClientKind, RemoteClient};
use crate::ClientError;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tiktools_cloud_auth::CloudAuth;
use tiktools_core::{
    cloud_teams::{
        self, CloudTeams, PeerPublication, TeamDevice, TeamEnrollParams, TeamRedeemParams,
        TeamRedeemResult,
    },
    control::OperationError,
};
use tiktools_peer::EndpointAddr;
mod maintenance;
mod rtc;
pub(crate) use maintenance::MaintenanceContext;

pub struct CloudPeerController {
    auth: Arc<CloudAuth>,
    data: PathBuf,
    profiles: ProfileManager,
    kind: ClientKind,
    lifecycle: tokio::sync::Mutex<()>,
}
fn cloud(error: OperationError) -> ClientError {
    ClientError::new(error.code, error.message)
}
impl CloudPeerController {
    pub fn from_environment(
        data: PathBuf,
        profiles: ProfileManager,
        kind: ClientKind,
    ) -> Result<Self, ClientError> {
        let auth = CloudAuth::from_environment(
            data.clone(),
            Arc::new(tiktools_cloud_auth::OsTokenStore::new()),
        );
        Self::new(Arc::new(auth), data, profiles, kind)
    }
    pub async fn controller_device_id(&self) -> Result<String, ClientError> {
        let session = self
            .auth
            .session()
            .await
            .ok_or_else(|| fail("sign_in_required", "sign in to Cloud first"))?;
        let (teams, _) = self.scoped(&session.instance_origin(), session.account_id());
        teams.controller_device().map_err(cloud)
    }
    pub fn new(
        auth: Arc<CloudAuth>,
        data: PathBuf,
        profiles: ProfileManager,
        kind: ClientKind,
    ) -> Result<Self, ClientError> {
        if !data.is_absolute() || kind == ClientKind::Browser {
            return Err(fail(
                "invalid_params",
                "native peer controller needs an absolute data directory",
            ));
        }
        profiles.set_cloud_context(MaintenanceContext::new(auth.clone(), data.clone()));
        Ok(Self {
            auth,
            data,
            profiles,
            kind,
            lifecycle: tokio::sync::Mutex::new(()),
        })
    }
    /// Cloud login/restore stays native; callers expose only AccountStatus and
    /// browser login URLs through their own CLI or WebView management boundary.
    pub fn account(&self) -> &CloudAuth {
        &self.auth
    }
    fn scoped(&self, origin: &str, account: &str) -> (CloudTeams, PathBuf) {
        scoped(&self.data, origin, account)
    }
}
fn scoped(data: &Path, origin: &str, account: &str) -> (CloudTeams, PathBuf) {
    let digest = ring::digest::digest(
        &ring::digest::SHA256,
        format!("peer-controller-v1\0{origin}\0{account}").as_bytes(),
    );
    let key: String = digest.as_ref().iter().map(|b| format!("{b:02x}")).collect();
    let data = data.join("cloud-peer").join(key);
    (CloudTeams::from_data_directory(&data), data)
}
impl CloudPeerController {
    pub async fn teams(
        &self,
    ) -> Result<Vec<tiktools_core::cloud_teams::PeerMembership>, ClientError> {
        let session = self
            .auth
            .session()
            .await
            .ok_or_else(|| fail("sign_in_required", "sign in to Cloud first"))?;
        let _permit = self
            .auth
            .try_session_emission_permit(&session)
            .map_err(|_| fail("account_changed", "Cloud account changed"))?;
        let (teams, _) = self.scoped(&session.instance_origin(), session.account_id());
        let device = teams.controller_device().map_err(cloud)?;
        let memberships = tokio::time::timeout(
            Duration::from_secs(20),
            teams.peer_memberships(&self.auth, &device),
        )
        .await
        .map_err(|_| network())?
        .map_err(cloud)?;
        let kind = match self.kind {
            ClientKind::Desktop => "desktop",
            ClientKind::Agent => "agent",
            ClientKind::Browser => return Err(protocol()),
        };
        Ok(memberships
            .items
            .into_iter()
            .filter(|item| item.client_kind == kind)
            .collect())
    }
    pub async fn enroll(&self, label: String) -> Result<TeamDevice, ClientError> {
        let _lifecycle = self.lifecycle.lock().await;
        let session = self
            .auth
            .session()
            .await
            .ok_or_else(|| fail("sign_in_required", "sign in to Cloud first"))?;
        let _permit = self
            .auth
            .try_session_emission_permit(&session)
            .map_err(|_| fail("account_changed", "Cloud account changed"))?;
        let (teams, _) = self.scoped(&session.instance_origin(), session.account_id());
        tokio::time::timeout(
            Duration::from_secs(40),
            teams.enroll(
                &self.auth,
                TeamEnrollParams {
                    kind: "controller".into(),
                    label,
                    host_id: None,
                },
            ),
        )
        .await
        .map_err(|_| network())?
        .map_err(cloud)
    }
    /// The invitation becomes a request. It does not grant native authority.
    pub async fn redeem(&self, token: String) -> Result<TeamRedeemResult, ClientError> {
        let _lifecycle = self.lifecycle.lock().await;
        let session = self
            .auth
            .session()
            .await
            .ok_or_else(|| fail("sign_in_required", "sign in to Cloud first"))?;
        let _permit = self
            .auth
            .try_session_emission_permit(&session)
            .map_err(|_| fail("account_changed", "Cloud account changed"))?;
        let (teams, _) = self.scoped(&session.instance_origin(), session.account_id());
        let device = teams.controller_device().map_err(cloud)?;
        tokio::time::timeout(
            Duration::from_secs(20),
            cloud_teams::redeem_invitation(&self.auth, &device, TeamRedeemParams { token }),
        )
        .await
        .map_err(|_| network())?
        .map_err(cloud)
    }
    /// Registers the endpoint, resolves only the approved host, authenticates
    /// its key and saves the reusable profile after local RPC admission.
    pub async fn connect(&self, name: &str, host: &str) -> Result<RemoteProfile, ClientError> {
        let _lifecycle = self.lifecycle.lock().await;
        if !super::valid_id(host) {
            return Err(fail("invalid_params", "invalid Cloud host ID"));
        }
        self.profiles.ensure_new(name).await?;
        let session = self
            .auth
            .session()
            .await
            .ok_or_else(|| fail("sign_in_required", "sign in to Cloud first"))?;
        let _permit = self
            .auth
            .try_session_emission_permit(&session)
            .map_err(|_| fail("account_changed", "Cloud account changed"))?;
        let origin = session.instance_origin();
        let (teams, data) = self.scoped(&origin, session.account_id());
        tokio::time::timeout(
            Duration::from_secs(90),
            self.connect_inner(name, host, &origin, &teams, &data),
        )
        .await
        .map_err(|_| network())?
    }
    async fn connect_inner(
        &self,
        name: &str,
        host: &str,
        origin: &str,
        _teams: &CloudTeams,
        _data: &Path,
    ) -> Result<RemoteProfile, ClientError> {
        let context = MaintenanceContext::new(self.auth.clone(), self.data.clone());
        let (remote, metadata) = context
            .dial_rtc(host, self.kind, None, None, Some(origin))
            .await?;
        let saved = self
            .profiles
            .install_peer(name, origin, metadata, &remote, self.kind)
            .await;
        remote.close();
        saved
    }
}
