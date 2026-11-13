//! Iroh admission into the existing scoped RPC session engine.
#[cfg(feature = "cloud-peer")]
mod cloud;
use crate::{auth::Authentication, socket, state::State, Error};
use serde::Deserialize;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tiktools_collaboration::{ClientKind, PeerGatewayStatus};
use tiktools_control_api::ControlApi;
use tiktools_peer::{Frame, PeerConnection, PeerEndpoint};
use tokio::{
    sync::Semaphore,
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;

pub struct PeerGateway {
    endpoint: PeerEndpoint,
    state: Arc<State>,
    task: Option<JoinHandle<()>>,
    revision: String,
}

impl PeerGateway {
    pub fn address(&self) -> tiktools_peer::EndpointAddr {
        self.endpoint.address()
    }
    pub fn configuration_revision(&self) -> &str {
        &self.revision
    }
    pub fn is_finished(&self) -> bool {
        self.task.as_ref().is_none_or(JoinHandle::is_finished)
    }
    pub fn refresh_status(&self) -> Result<(), Error> {
        let address = self.endpoint.address();
        self.state.service.update_peer_gateway_status(|status| {
            if status.gateway.generation != self.state.generation {
                return;
            }
            status.addresses = address
                .addrs
                .iter()
                .filter_map(|addr| match addr {
                    tiktools_peer::TransportAddr::Ip(ip) => Some(ip.to_string()),
                    _ => None,
                })
                .collect();
            status.relay_urls = address
                .addrs
                .iter()
                .filter_map(|addr| match addr {
                    tiktools_peer::TransportAddr::Relay(url) => Some(url.to_string()),
                    _ => None,
                })
                .collect();
        })?;
        Ok(())
    }
    pub async fn shutdown(mut self) {
        self.state.stopping.cancel();
        self.endpoint.close().await;
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}
impl Drop for PeerGateway {
    fn drop(&mut self) {
        self.state.stopping.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

pub async fn start_peer(api: Arc<ControlApi>) -> Result<PeerGateway, Error> {
    api.core().set_collaboration_peer_available(true);
    let host = api.clone();
    let (service, configuration, revision, key) = tokio::task::spawn_blocking(move || {
        let service = host.core().collaboration()?;
        let (configuration, revision) = service.configuration_snapshot()?;
        if !configuration.enabled || configuration.peer.is_none() {
            return Err(Error::Configuration);
        }
        let path = host
            .core()
            .db
            .paths()
            .data
            .join("collaboration-peer/identity.key");
        let key = tiktools_peer::load_or_create_key(&path)?;
        Ok::<_, Error>((service, configuration, revision, key))
    })
    .await
    .map_err(|_| Error::Task)??;
    let config = tiktools_peer::Config {
        relays: configuration
            .peer
            .as_ref()
            .ok_or(Error::Configuration)?
            .relay_urls
            .clone(),
    };
    let endpoint = PeerEndpoint::bind(key, &config).await?;
    let state = State::new_peer(
        api,
        service.clone(),
        configuration,
        CancellationToken::new(),
    )?;
    service.update_peer_gateway_status(|status| {
        *status = PeerGatewayStatus::default();
        status.gateway.available = true;
        status.gateway.running = true;
        status.gateway.generation = state.generation.clone();
        status.gateway.configuration_revision = Some(revision.clone());
        status.endpoint_id = Some(endpoint.id().to_string());
        status.addresses = endpoint
            .address()
            .addrs
            .iter()
            .filter_map(|addr| match addr {
                tiktools_peer::TransportAddr::Ip(ip) => Some(ip.to_string()),
                _ => None,
            })
            .collect();
        status.relay_urls = endpoint
            .address()
            .addrs
            .iter()
            .filter_map(|addr| match addr {
                tiktools_peer::TransportAddr::Relay(url) => Some(url.to_string()),
                _ => None,
            })
            .collect();
    })?;
    let listener = endpoint.clone();
    let sessions = state.clone();
    let task = tokio::spawn(async move {
        let handshakes = Arc::new(Semaphore::new(
            sessions.configuration.limits.connections as usize * 2,
        ));
        let mut tasks = JoinSet::new();
        #[cfg(feature = "cloud-peer")]
        {
            let endpoint = listener.clone();
            let state = sessions.clone();
            tasks.spawn(async move {
                cloud::run(state, endpoint).await;
            });
        }
        loop {
            tokio::select! {
                _ = sessions.stopping.cancelled() => break,
                _ = tasks.join_next(), if !tasks.is_empty() => {},
                incoming = listener.incoming() => {
                    let Some(incoming) = incoming else { break; };
                    let Ok(permit) = handshakes.clone().try_acquire_owned() else { drop(incoming); continue; };
                    let state = sessions.clone();
                    tasks.spawn(async move {
                        let _handshake = permit;
                        if let Ok(connection) = PeerEndpoint::finish_accept(incoming).await {
                            let _ = session(connection.clone(), state).await;
                            connection.close();
                        }
                    });
                }
            }
        }
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        let _ = sessions.update_status(|status| {
            if status.generation == sessions.generation {
                status.running = false;
            }
        });
    });
    Ok(PeerGateway {
        endpoint,
        state,
        task: Some(task),
        revision,
    })
}

/// The normal Desktop/headless supervisor owns this task. An idle local-only
/// process never creates peer keys or opens collaboration storage for it.
#[cfg(not(feature = "cloud-peer"))]
pub(crate) async fn supervise_peer(api: Arc<ControlApi>) {
    api.core().set_collaboration_peer_available(true);
    let mut gateway: Option<PeerGateway> = None;
    let mut clock = tokio::time::interval(Duration::from_millis(500));
    loop {
        clock.tick().await;
        if api.core().is_shutdown() {
            break;
        }
        let host = api.clone();
        let snapshot = tokio::task::spawn_blocking(move || {
            if !host
                .core()
                .db
                .paths()
                .data
                .join("collaboration/collaboration.db")
                .exists()
            {
                return Ok::<_, Error>(None);
            }
            let service = host.core().collaboration()?;
            service.update_peer_gateway_status(|status| status.gateway.available = true)?;
            Ok(Some(service.configuration_snapshot()?))
        })
        .await;
        let Ok(Ok(snapshot)) = snapshot else {
            continue;
        };
        if let Some(current) = &gateway {
            let _ = current.refresh_status();
        }
        if gateway.as_ref().is_some_and(|current| {
            current.is_finished()
                || snapshot
                    .as_ref()
                    .is_none_or(|(_, revision)| revision != current.configuration_revision())
        }) {
            if let Some(current) = gateway.take() {
                current.shutdown().await;
            }
        }
        if snapshot
            .is_some_and(|(configuration, _)| configuration.enabled && configuration.peer.is_some())
            && gateway.is_none()
        {
            match start_peer(api.clone()).await {
                Ok(started) => gateway = Some(started),
                Err(_) => {
                    if let Ok(service) = api.core().collaboration() {
                        let _ = service.update_peer_gateway_status(|status| {
                            status.gateway.available = true;
                            status.gateway.running = false;
                            status.gateway.last_error =
                                Some("Unable to start peer transport".into());
                        });
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    if let Some(current) = gateway {
        current.shutdown().await;
    }
    api.core().set_collaboration_peer_available(false);
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionRequest {
    #[serde(rename = "type")]
    request_type: String,
    client_kind: ClientKind,
}

async fn session(connection: PeerConnection, state: Arc<State>) -> Result<(), Error> {
    let peer = connection.remote_id().to_string();
    if state.rate(&format!("source:peer:{peer}")).is_err() {
        return Err(Error::Configuration);
    }
    let service = state.service.clone();
    let identity_key = peer.clone();
    let authority = tokio::task::spawn_blocking(move || service.authenticate_peer(&identity_key))
        .await
        .map_err(|_| Error::Task)??;
    let identity = authority.snapshot()?;
    let _permit = state
        .admit(&identity.principal_id, &identity.session_id)
        .map_err(|_| Error::Configuration)?;
    let mut framed = tokio::time::timeout(Duration::from_secs(10), connection.accept())
        .await
        .map_err(|_| Error::Task)??;
    let request = tokio::time::timeout(Duration::from_secs(10), framed.recv())
        .await
        .map_err(|_| Error::Task)??;
    let Frame::Text(raw) = request else {
        return Err(Error::Configuration);
    };
    if raw.len() > 4096 {
        return Err(Error::Configuration);
    }
    let request: SessionRequest = serde_json::from_str(&raw).map_err(|_| Error::Configuration)?;
    if request.request_type != "session" || request.client_kind != identity.client_kind {
        return Err(Error::Configuration);
    }
    let response = json!({"hostId":state.service.host_id(),"principalId":identity.principal_id,"sessionId":identity.session_id,"clientKind":identity.client_kind,"accessExpiresAt":identity.access_expires_at});
    tokio::time::timeout(
        Duration::from_secs(5),
        framed.send(Frame::Text(response.to_string())),
    )
    .await
    .map_err(|_| Error::Task)??;
    socket::run(
        socket::wire::Wire::Peer(framed),
        state,
        Authentication {
            authority,
            token: String::new(),
            browser: false,
            peer_id: Some(peer),
        },
    )
    .await;
    Ok(())
}
