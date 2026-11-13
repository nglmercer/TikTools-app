//! Observe the selected ICE pair, rather than inferring a route from readiness.
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc, Weak,
};
use tokio_util::sync::CancellationToken;
use webrtc::peer_connection::{PeerConnection, RTCIceCandidateType};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Direct,
    Relay,
}
pub(crate) fn track(
    connection: Weak<dyn PeerConnection>,
    route: Arc<AtomicU8>,
    stop: CancellationToken,
) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            tokio::select! {biased;_=stop.cancelled()=>return,_=tick.tick()=>{}}
            let Some(connection) = connection.upgrade() else {
                return;
            };
            let selected=tokio::select! {
                biased;
                _=stop.cancelled()=>return,
                pair=tokio::time::timeout(std::time::Duration::from_secs(1),async {
                    let sctp=connection.sctp().await?;
                    sctp.transport().ice_transport().get_selected_candidate_pair().await.ok().flatten()
                })=>pair.ok().flatten(),
            }.map(|pair|if [pair.local().typ,pair.remote().typ].contains(&RTCIceCandidateType::Relay){2}else{1}).unwrap_or(0);
            route.store(selected, Ordering::Release);
        }
    });
}
