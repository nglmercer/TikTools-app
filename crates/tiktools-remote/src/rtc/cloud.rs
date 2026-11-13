//! Native Cloud lifecycle. Losing signaling never tears down established RPC.
mod handshake;
mod relay;
mod signaling;
mod supervisor;
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
pub(crate) use supervisor::supervise;
use tiktools_core::{cloud_peer_host::CloudPeerHostBridge, cloud_teams::PeerPublication};
use tiktools_peer::SecretKey;
use tiktools_rtc::Channel;
use tokio::{
    sync::{mpsc, Notify},
    task::{JoinHandle, JoinSet},
};

pub struct CloudRtcHost {
    gateway: Arc<RtcGateway>,
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
    signaling_ready: Arc<AtomicBool>,
}
impl CloudRtcHost {
    pub async fn start(api: Arc<ControlApi>) -> Result<Self, Error> {
        Self::start_with_config(api, tiktools_rtc::Config::default()).await
    }
    pub async fn start_with_config(
        api: Arc<ControlApi>,
        ice: tiktools_rtc::Config,
    ) -> Result<Self, Error> {
        tokio::time::timeout(Duration::from_secs(60), Self::prepare(api, ice))
            .await
            .map_err(|_| Error::Task)?
    }
    async fn prepare(api: Arc<ControlApi>, mut ice: tiktools_rtc::Config) -> Result<Self, Error> {
        if api.core().is_shutdown() {
            return Err(Error::Configuration);
        }
        let core = api.clone();
        let bridge = tokio::task::spawn_blocking(move || core.core().cloud_peer_host_bridge())
            .await
            .map_err(|_| Error::Task)?
            .map_err(|_| Error::Configuration)?
            .ok_or(Error::Configuration)?;
        let (source, previous) = bridge
            .rtc_registration()
            .await
            .map_err(|_| Error::Configuration)?;
        let operator_ice = ice.ice_servers.is_empty();
        let discovery = bridge
            .rtc_ice_config(&source)
            .await
            .map_err(|_| Error::Configuration)?;
        if ice.ice_servers.is_empty() {
            ice.ice_servers = discovery
                .stun_urls
                .into_iter()
                .map(|url| tiktools_rtc::IceServer {
                    urls: vec![url],
                    username: String::new(),
                    credential: String::new(),
                })
                .collect();
        }
        let path = api
            .core()
            .db
            .paths()
            .data
            .join("collaboration-peer/identity.key");
        let registered = previous.is_some();
        let key = tokio::task::spawn_blocking(move || {
            if registered {
                tiktools_peer::load_key(&path)
            } else {
                tiktools_peer::load_or_create_key(&path)
            }
        })
        .await
        .map_err(|_| Error::Task)??;
        if previous.is_some_and(|record| record.endpoint_id != key.public().to_string()) {
            return Err(Error::Configuration);
        }
        let key = Arc::new(key);
        let publication = publication(&bridge, &key);
        let sign = |message: &[u8]| key.sign(message).to_bytes();
        bridge
            .synchronize(publication, &sign)
            .await
            .map_err(|_| Error::Configuration)?;
        let gateway = Arc::new(RtcGateway::start(api, source, bridge.device_id().into()).await?);
        let stop = gateway.state.stopping.clone();
        let signaling_ready = Arc::new(AtomicBool::new(false));
        let runtime = Runtime {
            gateway: gateway.clone(),
            bridge,
            key,
            ice,
            operator_ice,
            wake: Arc::new(Notify::new()),
            signaling_ready: signaling_ready.clone(),
        };
        let task = tokio::spawn(runtime.run());
        Ok(Self {
            gateway,
            stop,
            task: Some(task),
            signaling_ready,
        })
    }
    pub fn signaling_ready(&self) -> bool {
        self.signaling_ready.load(Ordering::Acquire)
            && !self.stop.is_cancelled()
            && !self.is_finished()
    }
    pub fn is_finished(&self) -> bool {
        self.task.as_ref().is_none_or(JoinHandle::is_finished)
    }
    pub async fn shutdown(mut self) {
        self.stop.cancel();
        self.gateway.stop();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}
impl Drop for CloudRtcHost {
    fn drop(&mut self) {
        self.stop.cancel();
        self.gateway.stop();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
struct Runtime {
    gateway: Arc<RtcGateway>,
    bridge: CloudPeerHostBridge,
    key: Arc<SecretKey>,
    ice: tiktools_rtc::Config,
    operator_ice: bool,
    wake: Arc<Notify>,
    signaling_ready: Arc<AtomicBool>,
}
fn publication(bridge: &CloudPeerHostBridge, key: &SecretKey) -> PeerPublication {
    PeerPublication {
        device_id: bridge.device_id().into(),
        endpoint_id: key.public().to_string(),
        addresses: vec![],
        relay_urls: vec![],
        expected_revision: 0,
    }
}
impl Runtime {
    async fn run(self) {
        let (connected, mut channels) = mpsc::channel::<Channel>(16);
        let mut calls = JoinSet::new();
        let signaling = signaling::run(&self, connected);
        let maintenance = self.maintain();
        tokio::pin!(signaling, maintenance);
        loop {
            tokio::select! {
                biased;
                _=self.gateway.state.stopping.cancelled()=>break,
                _=&mut maintenance=>break,
                _=&mut signaling=>break,
                channel=channels.recv()=>match channel {
                    Some(channel)=>{if calls.len()>=self.gateway.state.configuration.limits.connections as usize {drop(channel);continue;} let gateway=self.gateway.clone();calls.spawn(async move{let _=gateway.serve(channel).await;});},
                    None=>break,
                },
                _=calls.join_next(),if !calls.is_empty()=>{},
            }
        }
        self.gateway.stop();
        calls.abort_all();
        while calls.join_next().await.is_some() {}
    }
    async fn maintain(&self) {
        let mut sync = tokio::time::interval(Duration::from_secs(20));
        sync.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut fence = tokio::time::interval(Duration::from_millis(500));
        let mut pending = JoinSet::new();
        loop {
            tokio::select! {
                _=self.gateway.state.stopping.cancelled()=>break,
                _=fence.tick()=>{
                    let bridge=self.bridge.clone();let source=self.gateway.source.clone();let api=self.gateway.state.api.clone();let revision=self.gateway.revision.clone();let generation=self.gateway.state.generation.clone();
                    let valid=tokio::task::spawn_blocking(move|| {
                        let matches=bridge.rtc_source_matches(&source);
                        if matches {let _=bridge.retain_context();} else {let _=api.core().retain_cloud_peer_context();}
                        matches && !api.core().is_shutdown() && api.core().collaboration().ok().and_then(|s|s.configuration_snapshot().ok()).is_some_and(|(config,current)|config.enabled && config.peer.is_some() && revision==current && api.core().collaboration().ok().and_then(|s|s.peer_gateway_status().ok()).is_some_and(|s|s.gateway.generation==generation))
                    }).await.unwrap_or(false);
                    if !valid {self.gateway.stop();break;}
                },
                _=sync.tick()=>self.refresh(&mut pending),
                _=self.wake.notified()=>self.refresh(&mut pending),
                _=pending.join_next(),if !pending.is_empty()=>{},
            }
        }
        pending.abort_all();
        while pending.join_next().await.is_some() {}
    }
    fn refresh(&self, pending: &mut JoinSet<()>) {
        if !pending.is_empty() {
            return;
        }
        let bridge = self.bridge.clone();
        let key = self.key.clone();
        let gateway = self.gateway.clone();
        pending.spawn(async move {
            let sign = |message: &[u8]| key.sign(message).to_bytes();
            let result = tokio::time::timeout(
                Duration::from_secs(30),
                bridge.synchronize(publication(&bridge, &key), &sign),
            )
            .await;
            let error = match result {
                Ok(Ok(report)) if report.unsupported_grants > 0 => {
                    Some("Some Cloud native policies are unsupported".into())
                }
                Ok(Ok(_)) => None,
                Ok(Err(error)) => Some(format!("Cloud coordination: {}", error.code)),
                Err(_) => Some("Cloud coordination timed out".into()),
            };
            let _ = gateway.state.service.update_peer_gateway_status(|status| {
                if status.gateway.generation == gateway.state.generation {
                    status.gateway.last_error = error;
                }
            });
        });
    }
}
