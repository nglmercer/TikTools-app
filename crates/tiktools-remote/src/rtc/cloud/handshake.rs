use super::*;
use serde_json::Value;
use tiktools_rtc::{
    negotiation::{Event, Negotiation},
    signaling::{Context, ForwardedSignal},
};

pub(super) struct Attempt {
    pub gateway: Arc<RtcGateway>,
    pub bridge: CloudPeerHostBridge,
    pub key: Arc<SecretKey>,
    pub ice: tiktools_rtc::Config,
    pub offer: ForwardedSignal,
    pub incoming: mpsc::Receiver<ForwardedSignal>,
    pub outgoing: mpsc::Sender<Value>,
    pub connected: mpsc::Sender<Channel>,
}
impl Attempt {
    pub async fn run(self) -> Result<(), Error> {
        let Self {
            gateway,
            bridge,
            key,
            ice,
            offer,
            mut incoming,
            outgoing,
            connected,
        } = self;
        let remote = offer.from_device_id;
        let endpoint = bridge
            .rtc_peer_key(
                gateway.source.clone(),
                remote.to_string(),
                offer.grant_revision,
            )
            .await
            .map_err(|_| Error::Configuration)?;
        let context = Context {
            host_id: offer.host_id,
            session_id: offer.session_id,
            host_device_id: offer.host_device_id,
            from_device_id: offer.host_device_id,
            negotiation_id: offer.negotiation_id,
            grant_revision: offer.grant_revision,
        };
        let mut negotiation =
            Negotiation::new(ice, context, &key.to_bytes(), remote, endpoint).await?;
        let answer = negotiation
            .apply(offer)
            .await?
            .ok_or(Error::Configuration)?;
        outgoing.send(answer).await.map_err(|_| Error::Task)?;
        loop {
            tokio::select! {
                frame=incoming.recv()=>{
                    let frame=frame.ok_or(Error::Task)?;
                    if let Some(reply)=negotiation.apply(frame).await? {outgoing.send(reply).await.map_err(|_|Error::Task)?;}
                },
                event=negotiation.next()=>match event? {
                    Event::Signal(frame)=>outgoing.send(frame).await.map_err(|_|Error::Task)?,
                    Event::Connected(channel)=>{connected.send((*channel).into()).await.map_err(|_|Error::Task)?;return Ok(());},
                },
            }
        }
    }
}
