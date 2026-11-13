//! Metadata refresh is independent of the live RPC channel and its heartbeat.
use super::*;
use tiktools_core::cloud_teams::PeerPublication;

pub(super) async fn run(state: Arc<State>, endpoint: PeerEndpoint) {
    let mut clock = tokio::time::interval(Duration::from_secs(20));
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut idle_fence = tokio::time::interval(Duration::from_millis(500));
    loop {
        tokio::select! {
            _=state.stopping.cancelled()=>return,
            _=clock.tick()=>{},
            _=idle_fence.tick()=>{
                retain(&state).await;
                continue;
            }
        }
        let host = state.api.clone();
        let bridge =
            tokio::task::spawn_blocking(move || host.core().cloud_peer_host_bridge()).await;
        let Ok(Ok(Some(bridge))) = bridge else {
            continue;
        };
        let context = bridge.clone();
        let _ = tokio::task::spawn_blocking(move || context.retain_context()).await;
        let address = endpoint.address();
        let publication = PeerPublication {
            device_id: bridge.device_id().into(),
            endpoint_id: address.id.to_string(),
            addresses: address
                .addrs
                .iter()
                .filter_map(|addr| match addr {
                    tiktools_peer::TransportAddr::Ip(ip) => Some(ip.to_string()),
                    _ => None,
                })
                .take(8)
                .collect(),
            relay_urls: address
                .addrs
                .iter()
                .filter_map(|addr| match addr {
                    tiktools_peer::TransportAddr::Relay(url) => Some(url.to_string()),
                    _ => None,
                })
                .take(4)
                .collect(),
            expected_revision: 0,
        };
        let sign = |message: &[u8]| endpoint.sign(message);
        let sync = bridge.synchronize(publication, &sign);
        tokio::pin!(sync);
        let mut fence = tokio::time::interval(Duration::from_millis(500));
        let result = loop {
            tokio::select! {
                _=state.stopping.cancelled()=>return,
                result=&mut sync=>break result,
                _=fence.tick()=>{
                    let context=bridge.clone();
                    let _=tokio::task::spawn_blocking(move||context.retain_context()).await;
                }
            }
        };
        let _ = state.service.update_peer_gateway_status(|status| {
            if status.gateway.generation == state.generation {
                status.gateway.last_error = match result {
                    Ok(report) if report.unsupported_grants > 0 => {
                        Some("Some Cloud native policies are unsupported".into())
                    }
                    Ok(_) => None,
                    Err(error) => Some(format!("Cloud peer coordination: {}", error.code)),
                };
            }
        });
    }
}

async fn retain(state: &Arc<State>) {
    let host = state.api.clone();
    let _ = tokio::task::spawn_blocking(move || host.core().retain_cloud_peer_context()).await;
}
