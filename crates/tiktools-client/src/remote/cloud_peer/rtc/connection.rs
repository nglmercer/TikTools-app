use super::*;
use tiktools_rtc::{
    relay::{io, Identity},
    Channel,
};
use tokio::sync::mpsc;

pub(super) async fn connect(
    mut socket: Connection,
    mut context: Context,
    secret: &[u8; 32],
    remote_key: [u8; 32],
    config: tiktools_rtc::Config,
) -> Result<Channel, ClientError> {
    if let Some(channel) = direct(&mut socket, &context, secret, remote_key, config).await? {
        socket.close().await;
        return Ok(channel);
    }
    // A fallback is a new transport negotiation, never a replay of an RPC.
    context.negotiation_id = Uuid::new_v4();
    relay(socket, context, secret, remote_key).await
}

async fn direct(
    socket: &mut Connection,
    context: &Context,
    secret: &[u8; 32],
    remote_key: [u8; 32],
    config: tiktools_rtc::Config,
) -> Result<Option<Channel>, ClientError> {
    let Ok(mut negotiation) = Negotiation::new(
        config,
        context.clone(),
        secret,
        context.host_device_id,
        remote_key,
    )
    .await
    else {
        return Ok(None);
    };
    let Ok(offer) = negotiation.offer().await else {
        return Ok(None);
    };
    socket.send(&offer).await.map_err(|_| network())?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        tokio::select! {
            _=tokio::time::sleep_until(deadline)=>return Ok(None),
            event=negotiation.next()=>match event {
                Ok(Event::Signal(frame))=>socket.send(&frame).await.map_err(|_|network())?,
                Ok(Event::Connected(channel))=>return Ok(Some((*channel).into())),
                Err(_)=>return Ok(None),
            },
            event=socket.next()=>match event.map_err(|_|network())? {
                CloudEvent::Signal(frame)=>{negotiation.apply(*frame).await.map_err(|_|fail("unauthorized","signed host negotiation was rejected"))?;},
                CloudEvent::Accepted{..}|CloudEvent::GrantsChanged=>{},
                CloudEvent::Rejected{code,..} if code=="directUnavailable"=>return Ok(None),
                CloudEvent::Rejected{code,..}=>return Err(rejected(&code)),
                _=>return Err(protocol()),
            },
        }
    }
}

async fn relay(
    mut socket: Connection,
    context: Context,
    secret: &[u8; 32],
    remote_key: [u8; 32],
) -> Result<Channel, ClientError> {
    let identity = Identity::new(context.clone(), secret).map_err(|_| protocol())?;
    socket
        .send(&serde_json::json!({"type":"relay.open","announcement":identity.announcement()}))
        .await
        .map_err(|_| network())?;
    let mut remote = context.clone();
    remote.from_device_id = remote.host_device_id;
    let peer = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match socket.next().await.map_err(|_| network())? {
                CloudEvent::RelayOpen(frame) => {
                    let pinned: String = remote_key.iter().map(|b| format!("{b:02x}")).collect();
                    if frame.from_endpoint_id != pinned {
                        return Err(fail("unauthorized", "relay device key changed"));
                    }
                    return identity
                        .verify(&remote, remote_key, frame.announcement)
                        .map_err(|_| fail("unauthorized", "signed relay identity was rejected"));
                }
                CloudEvent::Signal(_)
                | CloudEvent::Accepted { .. }
                | CloudEvent::RelayAccepted { .. }
                | CloudEvent::GrantsChanged => {}
                CloudEvent::Rejected {
                    tunnel_id: Some(id),
                    code,
                    ..
                } if id == context.negotiation_id => return Err(rejected(&code)),
                CloudEvent::Rejected {
                    negotiation_id: Some(_),
                    ..
                } => {}
                _ => return Err(network()),
            }
        }
    })
    .await
    .map_err(|_| network())??;
    let (outgoing, mut packets) = mpsc::channel(64);
    let (stream, mut records) = io::stream(context, remote, outgoing).map_err(|_| protocol())?;
    tokio::spawn(async move {
        let mut fence = tokio::time::interval(Duration::from_millis(100));
        loop {
            let result: Result<(), tiktools_rtc::Error> = tokio::select! {
                _=fence.tick()=>{if records.is_closed(){break;} Ok(())},
                frame=packets.recv()=>match frame { Some(frame)=>socket.send(&frame).await, None=>break },
                event=socket.next()=>match event {
                    Ok(CloudEvent::RelayRecord(record))=>records.apply(record),
                    Ok(CloudEvent::RelayAccepted{..}|CloudEvent::Accepted{..}|CloudEvent::Signal(_)|CloudEvent::GrantsChanged)=>Ok(()),
                    Ok(CloudEvent::Rejected{negotiation_id:Some(_),tunnel_id:None,..})=>Ok(()),
                    _=>break,
                },
            };
            if result.is_err() {
                break;
            }
        }
        records.cancel();
        socket.close().await;
    });
    identity
        .connect(peer, stream)
        .await
        .map(Into::into)
        .map_err(|_| network())
}

fn rejected(code: &str) -> ClientError {
    match code {
        "rateLimited" => fail("rate_limited", "Cloud transport rate limit exceeded"),
        "busy" | "peerOffline" | "peerBusy" | "temporaryFailure" => {
            fail("unavailable", "native host is unavailable")
        }
        _ => fail("unauthorized", "Cloud did not admit this transport"),
    }
}

#[cfg(test)]
mod tests;
