//! The RPC adapter accepts only channels produced by this signed negotiation.
use crate::{
    signaling::{Context, ForwardedSignal, Signer, Verifier},
    Config, Error, Event as PeerEvent, Framed, Peer,
};
use serde_json::Value;
use uuid::Uuid;

pub struct Negotiation {
    peer: Option<Peer>,
    signer: Signer,
    verifier: Verifier,
    local: Context,
    remote: Context,
    remote_key: [u8; 32],
}
pub enum Event {
    Signal(Value),
    Connected(Box<AuthenticatedChannel>),
}
/// Private construction proves that the pinned remote device signed the SDP
/// whose DTLS fingerprint the established DataChannel actually authenticated.
/// This identifies a transport peer; the host must still select its local grant.
pub struct AuthenticatedChannel {
    peer: Peer,
    framed: Framed,
    local: Context,
    remote: Context,
    local_endpoint: String,
    remote_endpoint: String,
}
impl AuthenticatedChannel {
    pub fn local_context(&self) -> &Context {
        &self.local
    }
    pub fn remote_context(&self) -> &Context {
        &self.remote
    }
    pub fn local_endpoint_id(&self) -> &str {
        &self.local_endpoint
    }
    pub fn remote_endpoint_id(&self) -> &str {
        &self.remote_endpoint
    }
    pub fn into_transport(self) -> (Peer, Framed) {
        (self.peer, self.framed)
    }
}
impl Negotiation {
    pub async fn new(
        config: Config,
        local: Context,
        local_secret: &[u8; 32],
        remote_device: Uuid,
        remote_key: [u8; 32],
    ) -> Result<Self, Error> {
        if local.from_device_id == remote_device
            || (local.from_device_id == local.host_device_id)
                == (remote_device == local.host_device_id)
            || local.grant_revision <= 0
        {
            return Err(Error::Configuration);
        }
        let mut remote = local.clone();
        remote.from_device_id = remote_device;
        let signer = Signer::new(local.clone(), local_secret)?;
        let verifier = Verifier::new(remote.clone(), remote_key);
        Ok(Self {
            peer: Some(Peer::new(config).await?),
            signer,
            verifier,
            local,
            remote,
            remote_key,
        })
    }
    pub async fn offer(&mut self) -> Result<Value, Error> {
        if self.local.from_device_id == self.local.host_device_id {
            return Err(Error::Negotiation);
        }
        let signal = self
            .peer
            .as_mut()
            .ok_or(Error::Negotiation)?
            .offer()
            .await?;
        self.signer.frame(signal)
    }
    pub async fn apply(&mut self, frame: ForwardedSignal) -> Result<Option<Value>, Error> {
        let result = async {
            let signal = self.verifier.verify(frame)?;
            // Gathering may finish just after the channel was handed to RPC.
            // The established owner continues ICE checks independently; stale
            // negotiation notifications must not tear down that live channel.
            if self.peer.is_none() {
                return Ok(None);
            }
            let reply = self
                .peer
                .as_mut()
                .ok_or(Error::Negotiation)?
                .apply(signal)
                .await?;
            reply.map(|signal| self.signer.frame(signal)).transpose()
        }
        .await;
        if result.is_err() {
            if let Some(peer) = &self.peer {
                peer.close().await;
            }
        }
        result
    }
    pub async fn next(&mut self) -> Result<Event, Error> {
        let event = self
            .peer
            .as_mut()
            .ok_or(Error::Negotiation)?
            .next_event()
            .await?;
        match event {
            PeerEvent::Signal(signal) => self.signer.frame(signal).map(Event::Signal),
            PeerEvent::Connected(framed) => {
                let peer = self.peer.take().ok_or(Error::Negotiation)?;
                Ok(Event::Connected(Box::new(AuthenticatedChannel {
                    peer,
                    framed,
                    local: self.local.clone(),
                    remote: self.remote.clone(),
                    local_endpoint: self.signer.endpoint_id(),
                    remote_endpoint: self
                        .remote_key
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect(),
                })))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::KeyPair;
    use serde_json::json;
    #[tokio::test]
    async fn changed_sdp_cannot_produce_an_authenticated_rpc_channel() {
        let controller_secret = [9; 32];
        let host_secret = [8; 32];
        let key = |seed: &[u8; 32]| -> [u8; 32] {
            ring::signature::Ed25519KeyPair::from_seed_unchecked(seed)
                .unwrap()
                .public_key()
                .as_ref()
                .try_into()
                .unwrap()
        };
        let context = Context {
            host_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            host_device_id: Uuid::new_v4(),
            from_device_id: Uuid::new_v4(),
            negotiation_id: Uuid::new_v4(),
            grant_revision: 1,
        };
        let mut host_context = context.clone();
        host_context.from_device_id = context.host_device_id;
        let config = || Config {
            bind_addresses: vec!["127.0.0.1:0".into()],
            ..Default::default()
        };
        let mut controller = Negotiation::new(
            config(),
            context.clone(),
            &controller_secret,
            context.host_device_id,
            key(&host_secret),
        )
        .await
        .unwrap();
        let mut host = Negotiation::new(
            config(),
            host_context,
            &host_secret,
            context.from_device_id,
            key(&controller_secret),
        )
        .await
        .unwrap();
        let request = controller.offer().await.unwrap();
        let mut forwarded = serde_json::to_value(&context).unwrap();
        for (name, value) in request.as_object().unwrap() {
            forwarded[name] = value.clone();
        }
        forwarded["fromEndpointId"] = json!(controller.signer.endpoint_id());
        forwarded["expiresAt"] = json!(9999999999999_i64);
        forwarded["signal"]["sdp"] = json!("v=0\r\na=fingerprint:sha-256 00\r\n");
        assert!(host
            .apply(serde_json::from_value(forwarded).unwrap())
            .await
            .is_err());
        assert!(host.next().await.is_err());
    }
}
