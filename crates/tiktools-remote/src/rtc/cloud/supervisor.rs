use super::*;

pub(crate) async fn supervise(api: Arc<ControlApi>) {
    api.core().set_collaboration_peer_available(true);
    let mut current: Option<CloudRtcHost> = None;
    let mut clock = tokio::time::interval(Duration::from_millis(500));
    let mut retry = tokio::time::Instant::now();
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
                return None;
            }
            host.core()
                .collaboration()
                .ok()?
                .configuration_snapshot()
                .ok()
        })
        .await
        .ok()
        .flatten();
        let Some((config, revision)) = snapshot else {
            continue;
        };
        if current.as_ref().is_some_and(|host| {
            host.is_finished()
                || host.gateway.revision != revision
                || !config.enabled
                || config.peer.is_none()
        }) {
            if let Some(host) = current.take() {
                host.shutdown().await;
            }
        }
        if current.is_some()
            || !config.enabled
            || config.peer.is_none()
            || tokio::time::Instant::now() < retry
        {
            continue;
        }
        let boot = CloudRtcHost::start(api.clone());
        tokio::pin!(boot);
        let result = loop {
            tokio::select! {
                result=&mut boot=>break Some(result),
                _=clock.tick()=>{
                    if api.core().is_shutdown(){break None;}
                    let host=api.clone();let expected=revision.clone();
                    let valid=tokio::task::spawn_blocking(move||host.core().collaboration().ok().and_then(|s|s.configuration_snapshot().ok()).is_some_and(|(config,current)|config.enabled&&config.peer.is_some()&&current==expected)).await.unwrap_or(false);
                    if !valid {break None;}
                },
            }
        };
        match result {
            Some(Ok(host)) => current = Some(host),
            Some(Err(_)) => {
                retry = tokio::time::Instant::now() + Duration::from_secs(2);
                if let Ok(service) = api.core().collaboration() {
                    let _ = service.update_peer_gateway_status(|status| {
                        status.gateway.available = true;
                        status.gateway.running = false;
                        status.gateway.last_error =
                            Some("Cloud collaboration is unavailable".into());
                    });
                }
            }
            None => {}
        }
    }
    if let Some(host) = current {
        host.shutdown().await;
    }
    api.core().set_collaboration_peer_available(false);
}
