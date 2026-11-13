//! WebRTC admission into the shared RPC/receipt/permission engine.
#[cfg(feature = "cloud-peer")]
mod cloud;
use crate::{
    auth::Authentication,
    socket::{self, wire::Wire},
    state::State,
    Error,
};
#[cfg(feature = "cloud-peer")]
pub(crate) use cloud::supervise;
#[cfg(feature = "cloud-peer")]
pub use cloud::CloudRtcHost;
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tiktools_collaboration::{ClientKind, CloudPeerSource, PeerGatewayStatus};
use tiktools_control_api::ControlApi;
use tiktools_rtc::{Channel, Frame};
use tokio_util::sync::CancellationToken;

pub struct RtcGateway {
    state: Arc<State>,
    source: CloudPeerSource,
    device: String,
    endpoint: String,
    revision: String,
}
impl RtcGateway {
    /// Native supervisor entry point. Source/device come from the enrolled
    /// account, not from a remote request. Creates no TLS listener or AppCore.
    pub async fn start(
        api: Arc<ControlApi>,
        source: CloudPeerSource,
        device: String,
    ) -> Result<Self, Error> {
        if uuid::Uuid::parse_str(&device).map_or(true, |id| id.to_string() != device) {
            return Err(Error::Configuration);
        }
        let host = api.clone();
        let (service, config, revision, endpoint) = tokio::task::spawn_blocking(move || {
            let service = host.core().collaboration()?;
            let (config, revision) = service.configuration_snapshot()?;
            if !config.enabled || config.peer.is_none() {
                return Err(Error::Configuration);
            }
            let path = host
                .core()
                .db
                .paths()
                .data
                .join("collaboration-peer/identity.key");
            // Cloud registration creates a key only after proving that this
            // device has no registered endpoint. Admission never replaces it.
            let key = tiktools_peer::load_key(&path)?;
            Ok::<_, Error>((service, config, revision, key.public().to_string()))
        })
        .await
        .map_err(|_| Error::Task)??;
        let state = State::new_peer(api, service.clone(), config, CancellationToken::new())?;
        service.update_peer_gateway_status(|status| {
            *status = PeerGatewayStatus::default();
            status.gateway.available = true;
            status.gateway.running = true;
            status.gateway.generation = state.generation.clone();
            status.gateway.configuration_revision = Some(revision.clone());
            status.endpoint_id = Some(endpoint.clone());
        })?;
        Ok(Self {
            state,
            source,
            device,
            endpoint,
            revision,
        })
    }
    pub fn endpoint_id(&self) -> &str {
        &self.endpoint
    }
    pub fn stop(&self) {
        self.state.stopping.cancel();
        let _ = self.state.update_status(|status| {
            if status.generation == self.state.generation {
                status.running = false;
            }
        });
    }
    pub async fn serve(&self, channel: impl Into<Channel>) -> Result<(), Error> {
        let channel = channel.into();
        tokio::select! {
            biased;
            _=self.state.stopping.cancelled()=>Ok(()),
            result=self.serve_inner(channel)=>result,
        }
    }
    async fn serve_inner(&self, channel: Channel) -> Result<(), Error> {
        let local = channel.local_context();
        let remote = channel.remote_context();
        if channel.local_endpoint_id() != self.endpoint
            || local.host_id.to_string() != self.source.host_id
            || local.host_device_id.to_string() != self.device
            || local.from_device_id != local.host_device_id
            || remote.from_device_id == remote.host_device_id
        {
            return Err(Error::Configuration);
        }
        let endpoint = channel.remote_endpoint_id().to_owned();
        self.state
            .rate(&format!("source:peer:{endpoint}"))
            .map_err(|_| Error::Configuration)?;
        let source = self.source.clone();
        let peer = endpoint.clone();
        let device = remote.from_device_id.to_string();
        let revision = remote.grant_revision;
        let service = self.state.service.clone();
        let authority = tokio::task::spawn_blocking(move || {
            service.authenticate_rtc_peer(&peer, &source, &device, revision)
        })
        .await
        .map_err(|_| Error::Task)??;
        let identity = authority.snapshot()?;
        let _permit = self
            .state
            .admit(&identity.principal_id, &identity.session_id)
            .map_err(|_| Error::Configuration)?;
        let mut framed = channel.into_transport();
        let request = tokio::time::timeout(Duration::from_secs(10), framed.recv())
            .await
            .map_err(|_| Error::Task)??;
        let Frame::Text(raw) = request else {
            return Err(Error::Configuration);
        };
        if raw.len() > 4096 {
            return Err(Error::Configuration);
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Request {
            #[serde(rename = "type")]
            kind: String,
            client_kind: ClientKind,
        }
        let request: Request = serde_json::from_str(&raw).map_err(|_| Error::Configuration)?;
        if request.kind != "session" || request.client_kind != identity.client_kind {
            return Err(Error::Configuration);
        }
        let response = serde_json::json!({"hostId":self.state.service.host_id(),"principalId":identity.principal_id,"sessionId":identity.session_id,"clientKind":identity.client_kind,"accessExpiresAt":identity.access_expires_at});
        tokio::time::timeout(
            Duration::from_secs(5),
            framed.send(Frame::Text(response.to_string())),
        )
        .await
        .map_err(|_| Error::Task)??;
        socket::run(
            Wire::Rtc(framed),
            self.state.clone(),
            Authentication {
                authority,
                token: String::new(),
                browser: false,
                peer_id: Some(endpoint),
            },
        )
        .await;
        Ok(())
    }
}
impl Drop for RtcGateway {
    fn drop(&mut self) {
        self.stop();
    }
}
