use super::*;
use tiktools_rtc::{
    cloud_socket::ForwardedAnnouncement,
    relay::{io, Identity},
};

pub(super) struct Attempt {
    pub gateway: Arc<RtcGateway>,
    pub bridge: CloudPeerHostBridge,
    pub key: Arc<SecretKey>,
    pub offer: ForwardedAnnouncement,
    pub stream: io::Stream,
    pub outgoing: mpsc::Sender<serde_json::Value>,
    pub connected: mpsc::Sender<Channel>,
}
impl Attempt {
    pub async fn run(self) -> Result<(), Error> {
        let remote = self.offer.announcement.context.clone();
        let endpoint = self
            .bridge
            .rtc_peer_key(
                self.gateway.source.clone(),
                remote.from_device_id.to_string(),
                remote.grant_revision,
            )
            .await
            .map_err(|_| Error::Configuration)?;
        let registered: String = endpoint.iter().map(|b| format!("{b:02x}")).collect();
        if registered != self.offer.from_endpoint_id {
            return Err(Error::Configuration);
        }
        let mut local = remote.clone();
        local.from_device_id = local.host_device_id;
        let identity = Identity::new(local, &self.key.to_bytes())?;
        let peer = identity.verify(&remote, endpoint, self.offer.announcement)?;
        self.outgoing
            .send(serde_json::json!({"type":"relay.open","announcement":identity.announcement()}))
            .await
            .map_err(|_| Error::Task)?;
        let channel = identity.accept(peer, self.stream).await?;
        self.connected
            .send(channel.into())
            .await
            .map_err(|_| Error::Task)
    }
}
