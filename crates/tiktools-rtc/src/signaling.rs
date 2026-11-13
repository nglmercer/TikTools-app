//! Signatures bind SDP (including DTLS fingerprints) and ICE to the approved
//! device, Cloud session and exact grant revision. Never trust a key supplied
//! by an incoming signaling frame: the caller provides the pinned public key.
use crate::{Error, Signal};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use serde_json::{json, Value};
use uuid::Uuid;

const DOMAIN: &[u8] = b"tiktools.webrtc.signal.v1\0";
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Context {
    pub host_id: Uuid,
    pub session_id: Uuid,
    pub host_device_id: Uuid,
    pub from_device_id: Uuid,
    pub negotiation_id: Uuid,
    pub grant_revision: i64,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForwardedSignal {
    pub r#type: String,
    pub host_id: Uuid,
    pub session_id: Uuid,
    pub host_device_id: Uuid,
    pub from_device_id: Uuid,
    pub negotiation_id: Uuid,
    pub grant_revision: i64,
    pub sequence: u32,
    pub signal: Signal,
    pub from_endpoint_id: String,
    pub signature: String,
    pub expires_at: i64,
}
impl ForwardedSignal {
    fn context(&self) -> Context {
        Context {
            host_id: self.host_id,
            session_id: self.session_id,
            host_device_id: self.host_device_id,
            from_device_id: self.from_device_id,
            negotiation_id: self.negotiation_id,
            grant_revision: self.grant_revision,
        }
    }
}
fn signed_bytes(context: &Context, sequence: u32, signal: &Signal) -> Result<Vec<u8>, Error> {
    let mut value = serde_json::to_value(context).map_err(|_| Error::Negotiation)?;
    value["sequence"] = json!(sequence);
    value["signal"] = serde_json::to_value(signal).map_err(|_| Error::Negotiation)?;
    fn sorted(value: Value) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .map(|(k, v)| (k, sorted(v)))
                    .collect::<std::collections::BTreeMap<_, _>>()
                    .into_iter()
                    .collect(),
            ),
            Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
            value => value,
        }
    }
    let mut bytes = DOMAIN.to_vec();
    serde_json::to_writer(&mut bytes, &sorted(value)).map_err(|_| Error::Negotiation)?;
    Ok(bytes)
}
fn endpoint(key: &[u8]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn valid_position(context: &Context, sequence: u32, signal: &Signal) -> bool {
    if context.grant_revision <= 0 || sequence > 64 || !signal.valid() {
        return false;
    }
    match signal {
        Signal::Offer { .. } => sequence == 0 && context.from_device_id != context.host_device_id,
        Signal::Answer { .. } => sequence == 0 && context.from_device_id == context.host_device_id,
        _ => sequence > 0,
    }
}
/// Each negotiation side has an independent, strictly ordered sequence.
pub struct Signer {
    context: Context,
    key: Ed25519KeyPair,
    next: u32,
    started: std::time::Instant,
}
impl Signer {
    pub fn new(context: Context, secret: &[u8; 32]) -> Result<Self, Error> {
        let key = Ed25519KeyPair::from_seed_unchecked(secret).map_err(|_| Error::Configuration)?;
        Ok(Self {
            context,
            key,
            next: 0,
            started: std::time::Instant::now(),
        })
    }
    pub fn endpoint_id(&self) -> String {
        endpoint(self.key.public_key().as_ref())
    }
    pub fn frame(&mut self, signal: Signal) -> Result<Value, Error> {
        if self.started.elapsed() >= std::time::Duration::from_secs(60)
            || !valid_position(&self.context, self.next, &signal)
        {
            return Err(Error::Negotiation);
        }
        let bytes = signed_bytes(&self.context, self.next, &signal)?;
        let frame = json!({"type":"rtc.signal","negotiationId":self.context.negotiation_id,"sessionId":self.context.session_id,"hostDeviceId":self.context.host_device_id,"sequence":self.next,"signal":signal,"signature":URL_SAFE_NO_PAD.encode(self.key.sign(&bytes).as_ref())});
        self.next += 1;
        Ok(frame)
    }
}
/// The expected context and key come from the approved Cloud directory/local
/// grant. Expiry is bounded by the owning negotiation's monotonic deadline;
/// the forwarded `expiresAt` is metadata, not part of the device signature.
pub struct Verifier {
    context: Context,
    key: [u8; 32],
    next: u32,
    started: std::time::Instant,
}
impl Verifier {
    pub fn new(context: Context, pinned_key: [u8; 32]) -> Self {
        Self {
            context,
            key: pinned_key,
            next: 0,
            started: std::time::Instant::now(),
        }
    }
    pub fn verify(&mut self, frame: ForwardedSignal) -> Result<Signal, Error> {
        if self.started.elapsed() >= std::time::Duration::from_secs(60)
            || frame.r#type != "rtc.signal"
            || frame.context() != self.context
            || frame.sequence != self.next
            || frame.from_endpoint_id != endpoint(&self.key)
            || !valid_position(&self.context, frame.sequence, &frame.signal)
            || frame.signature.len() != 86
        {
            return Err(Error::Negotiation);
        }
        let signature = URL_SAFE_NO_PAD
            .decode(&frame.signature)
            .map_err(|_| Error::Negotiation)?;
        let bytes = signed_bytes(&self.context, frame.sequence, &frame.signal)?;
        UnparsedPublicKey::new(&ED25519, self.key)
            .verify(&bytes, &signature)
            .map_err(|_| Error::Negotiation)?;
        self.next += 1;
        Ok(frame.signal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> Context {
        Context {
            host_id: Uuid::from_u128(1),
            session_id: Uuid::from_u128(2),
            host_device_id: Uuid::from_u128(3),
            from_device_id: Uuid::from_u128(4),
            negotiation_id: Uuid::from_u128(5),
            grant_revision: 7,
        }
    }
    fn offer() -> Signal {
        Signal::Offer {
            sdp: "v=0\r\na=fingerprint:sha-256 00\r\n".into(),
        }
    }
    fn forwarded(context: &Context, frame: Value, endpoint: &str) -> ForwardedSignal {
        let mut value = serde_json::to_value(context).unwrap();
        for (key, item) in frame.as_object().unwrap() {
            value[key] = item.clone();
        }
        value["fromEndpointId"] = json!(endpoint);
        value["expiresAt"] = json!(9999999999999_i64);
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn signatures_pin_device_session_revision_and_sequence() {
        let mut signer = Signer::new(context(), &[9; 32]).unwrap();
        let key: [u8; 32] = signer.key.public_key().as_ref().try_into().unwrap();
        let endpoint = signer.endpoint_id();
        let frame = signer.frame(offer()).unwrap();
        let mut verifier = Verifier::new(context(), key);
        let mut forged = frame.clone();
        forged["signal"]["sdp"] = json!("v=0\r\na=fingerprint:sha-256 99\r\n");
        assert!(verifier
            .verify(forwarded(&context(), forged, &endpoint))
            .is_err());
        let mut other_context = context();
        other_context.grant_revision += 1;
        assert!(verifier
            .verify(forwarded(&other_context, frame.clone(), &endpoint))
            .is_err());
        assert!(Verifier::new(context(), [0; 32])
            .verify(forwarded(&context(), frame.clone(), &endpoint))
            .is_err());
        assert!(verifier
            .verify(forwarded(&context(), frame.clone(), &endpoint))
            .is_ok());
        assert!(verifier
            .verify(forwarded(&context(), frame, &endpoint))
            .is_err());
        let candidate = Signal::Candidate {
            candidate: "candidate:1 1 udp 1 127.0.0.1 1234 typ host".into(),
            sdp_mid: None,
            sdp_mline_index: None,
            username_fragment: None,
        };
        let frame = signer.frame(candidate).unwrap();
        assert!(frame["signal"]["sdpMid"].is_null());
        assert!(verifier
            .verify(forwarded(&context(), frame, &endpoint))
            .is_ok());
    }
    #[test]
    fn negotiation_expiry_uses_elapsed_time() {
        let mut signer = Signer::new(context(), &[9; 32]).unwrap();
        let key = signer.key.public_key().as_ref().try_into().unwrap();
        let frame = signer.frame(offer()).unwrap();
        let endpoint = signer.endpoint_id();
        signer.started -= std::time::Duration::from_secs(61);
        assert!(signer.frame(Signal::EndOfCandidates).is_err());
        let mut verifier = Verifier::new(context(), key);
        verifier.started -= std::time::Duration::from_secs(61);
        assert!(verifier
            .verify(forwarded(&context(), frame, &endpoint))
            .is_err());
    }
    #[test]
    fn canonical_bytes_match_backend_contract() {
        let bytes = signed_bytes(&context(), 1, &Signal::EndOfCandidates).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            concat!(
                "tiktools.webrtc.signal.v1\0",
                r#"{"fromDeviceId":"00000000-0000-0000-0000-000000000004","grantRevision":7,"hostDeviceId":"00000000-0000-0000-0000-000000000003","hostId":"00000000-0000-0000-0000-000000000001","negotiationId":"00000000-0000-0000-0000-000000000005","sequence":1,"sessionId":"00000000-0000-0000-0000-000000000002","signal":{"type":"endOfCandidates"}}"#
            )
        );
    }
    #[test]
    fn signer_enforces_role_and_bounded_negotiation() {
        let mut signer = Signer::new(context(), &[9; 32]).unwrap();
        assert!(signer.frame(Signal::EndOfCandidates).is_err());
        signer.frame(offer()).unwrap();
        assert!(signer.frame(offer()).is_err());
        for _ in 1..=64 {
            signer.frame(Signal::EndOfCandidates).unwrap();
        }
        assert!(signer.frame(Signal::EndOfCandidates).is_err());
        let mut host = context();
        host.from_device_id = host.host_device_id;
        assert!(Signer::new(host, &[9; 32]).unwrap().frame(offer()).is_err());
    }
}
