use super::*;
use std::collections::BTreeMap;
use tiktools_rtc::{
    cloud_socket::{Connection, Event, Role},
    signaling::ForwardedSignal,
    Signal,
};
use tokio::sync::Semaphore;
use uuid::Uuid;
struct Route {
    session: Uuid,
    device: Uuid,
    revision: i64,
    tx: mpsc::Sender<ForwardedSignal>,
    stop: CancellationToken,
}
impl Drop for Route {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

pub(super) async fn run(runtime: &Runtime, connected: mpsc::Sender<Channel>) {
    let slots = Arc::new(Semaphore::new(
        (runtime.gateway.state.configuration.limits.connections as usize)
            .saturating_mul(2)
            .min(128),
    ));
    loop {
        let attempt = async {
            // An API restart may change operator discovery services. Refresh
            // them before accepting new negotiations; established RPC stays up.
            let mut ice = runtime.ice.clone();
            if runtime.operator_ice {
                let config = runtime
                    .bridge
                    .rtc_ice_config(&runtime.gateway.source)
                    .await
                    .map_err(|_| Error::Configuration)?;
                ice.ice_servers = config
                    .stun_urls
                    .into_iter()
                    .map(|url| tiktools_rtc::IceServer {
                        urls: vec![url],
                        username: String::new(),
                        credential: String::new(),
                    })
                    .collect();
            }
            let ticket = runtime
                .bridge
                .rtc_ticket(&runtime.gateway.source)
                .await
                .map_err(|_| Error::Configuration)?;
            let host = runtime
                .gateway
                .source
                .host_id
                .parse()
                .map_err(|_| Error::Configuration)?;
            let mut socket =
                Connection::connect(&runtime.gateway.source.origin, &ticket, Role::Host, host)
                    .await?;
            runtime.signaling_ready.store(true, Ordering::Release);
            socket_loop(runtime, &mut socket, &connected, &slots, &ice).await
        };
        tokio::select! {biased;_=runtime.gateway.state.stopping.cancelled()=>return,_=attempt=>{}}
        runtime.signaling_ready.store(false, Ordering::Release);
        tokio::select! {biased;_=runtime.gateway.state.stopping.cancelled()=>return,_=tokio::time::sleep(Duration::from_secs(2))=>{}}
    }
}
async fn socket_loop(
    runtime: &Runtime,
    socket: &mut Connection,
    connected: &mpsc::Sender<Channel>,
    slots: &Arc<Semaphore>,
    ice: &tiktools_rtc::Config,
) -> Result<(), Error> {
    let (outbound, mut outgoing) = mpsc::channel(64);
    let mut routes = BTreeMap::<Uuid, Route>::new();
    let mut relays = BTreeMap::<Uuid, tiktools_rtc::relay::io::Records>::new();
    let mut tasks = JoinSet::new();
    loop {
        routes.retain(|_, route| !route.tx.is_closed());
        relays.retain(|_, route| !route.is_closed());
        tokio::select! {
            biased;
            _=runtime.gateway.state.stopping.cancelled()=>return Ok(()),
            result=tasks.join_next(),if !tasks.is_empty()=>{if let Some(Ok(id))=result {routes.remove(&id);}},
            frame=outgoing.recv()=>{socket.send(&frame.ok_or(Error::Task)?).await?;},
            event=socket.next()=>match event? {
                Event::UnsupportedCommand{command_id}=>{
                    socket.send(&serde_json::json!({"type":"result","commandId":command_id,
                        "status":"failed","code":"unsupportedOperation","result":null})).await?;
                },
                Event::GrantsChanged=>runtime.wake.notify_one(),
                Event::Accepted{..}|Event::RelayAccepted{..}=>{},
                Event::Rejected{tunnel_id:Some(id),..}=>{relays.remove(&id);},
                Event::Rejected{negotiation_id:Some(id),..}=>{routes.remove(&id);},
                Event::Rejected{negotiation_id:None,..}=>return Err(Error::Configuration),
                Event::RelayRecord(record)=>{
                    let id=record.position().0;
                    if let Some(route)=relays.get_mut(&id) {let _=route.apply(record);}
                },
                Event::RelayOpen(frame)=>{
                    let frame=*frame;
                    let remote=frame.announcement.context.clone();
                    let id=remote.negotiation_id;
                    if remote.host_id.to_string()!=runtime.gateway.source.host_id || remote.host_device_id.to_string()!=runtime.gateway.device
                        || remote.from_device_id==remote.host_device_id || relays.contains_key(&id)
                        || relays.len()+routes.len()>=128 {continue;}
                    // The controller abandoned direct ICE for this same Cloud
                    // session. Release its pending negotiation and UDP work.
                    routes.retain(|_, route| route.session != remote.session_id);
                    let Ok(permit)=slots.clone().try_acquire_owned() else {continue;};
                    let mut local=remote.clone();local.from_device_id=local.host_device_id;
                    let (stream,records)=tiktools_rtc::relay::io::stream(local,remote,outbound.clone())?;
                    relays.insert(id,records);
                    let task=relay::Attempt {gateway:runtime.gateway.clone(),bridge:runtime.bridge.clone(),key:runtime.key.clone(),
                        offer:frame,stream,outgoing:outbound.clone(),connected:connected.clone()};
                    let stop=runtime.gateway.state.stopping.clone();
                    tasks.spawn(async move {
                        let _permit=permit;
                        tokio::select! {biased;_=stop.cancelled()=>{},_=tokio::time::timeout(Duration::from_secs(60),task.run())=>{}}
                        id
                    });
                },
                Event::Signal(frame)=>{
                    let frame=*frame;
                    let id=frame.negotiation_id;
                    if frame.host_id.to_string()!=runtime.gateway.source.host_id || frame.host_device_id.to_string()!=runtime.gateway.device || frame.from_device_id==frame.host_device_id {continue;}
                    if matches!(frame.signal,Signal::Offer{..}) && frame.sequence==0 {
                        if routes.contains_key(&id) {continue;}
                        let Ok(permit)=slots.clone().try_acquire_owned() else {continue;};
                        let (tx,rx)=mpsc::channel(64);
                        let stop=runtime.gateway.state.stopping.child_token();
                        routes.insert(id,Route{session:frame.session_id,device:frame.from_device_id,revision:frame.grant_revision,tx,stop:stop.clone()});
                        let bridge=runtime.bridge.clone();let gateway=runtime.gateway.clone();let key=runtime.key.clone();let ice=ice.clone();let outbound=outbound.clone();let connected=connected.clone();
                        tasks.spawn(async move {
                            let _permit=permit;
                            let task=handshake::Attempt {gateway,bridge,key,ice,offer:frame,incoming:rx,outgoing:outbound,connected}.run();
                            tokio::select! {biased;_=stop.cancelled()=>{},_=tokio::time::timeout(Duration::from_secs(60),task)=>{}}
                            id
                        });
                    } else if let Some(route)=routes.get(&id) {
                        if frame.session_id==route.session && frame.from_device_id==route.device && frame.grant_revision==route.revision && route.tx.try_send(frame).is_err() {routes.remove(&id);}
                    }
                },
            },
        }
    }
}
