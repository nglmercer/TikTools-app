use super::*;
use futures_util::{SinkExt, StreamExt};
use ring::signature::KeyPair as _;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

fn pinned() -> [u8; 32] {
    ring::signature::Ed25519KeyPair::from_seed_unchecked(&[2; 32])
        .unwrap()
        .public_key()
        .as_ref()
        .try_into()
        .unwrap()
}
async fn backend(
    forge_answer: bool,
) -> (Connection, Context, tokio::task::JoinHandle<Vec<String>>) {
    let context = Context {
        host_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        host_device_id: Uuid::new_v4(),
        from_device_id: Uuid::new_v4(),
        negotiation_id: Uuid::new_v4(),
        grant_revision: 1,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let c = context.clone();
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
        socket.next().await.unwrap().unwrap();
        socket
            .send(Message::Text(
                json!({"type":"ready","role":"controller","hostId":c.host_id,
            "sessionId":c.session_id,"grantRevision":1})
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
        let mut observed = vec![];
        while let Some(Ok(message)) = socket.next().await {
            let Message::Text(text) = message else {
                break;
            };
            let value: Value = serde_json::from_str(&text).unwrap();
            let kind = value["type"].as_str().unwrap();
            observed.push(kind.to_owned());
            let reply = match kind {
                "ping" => json!({"type":"pong"}),
                "rtc.signal" if forge_answer => {
                    let endpoint: String = pinned().iter().map(|b| format!("{b:02x}")).collect();
                    json!({"type":"rtc.signal","hostId":c.host_id,"sessionId":c.session_id,
                        "hostDeviceId":c.host_device_id,"fromDeviceId":c.host_device_id,
                        "negotiationId":c.negotiation_id,"grantRevision":1,"sequence":0,
                        "signal":{"type":"answer","sdp":"v=0\r\na=fingerprint:sha-256 00:11\r\n"},
                        "fromEndpointId":endpoint,"expiresAt":9999999999999_i64,
                        "signature":"A".repeat(86)})
                }
                "rtc.signal" => {
                    json!({"type":"rtc.accepted","negotiationId":c.negotiation_id,"sequence":value["sequence"]})
                }
                _ => break,
            };
            if socket
                .send(Message::Text(reply.to_string().into()))
                .await
                .is_err()
            {
                break;
            }
        }
        observed
    });
    let socket = Connection::connect(&origin, &"t".repeat(32), Role::Controller, context.host_id)
        .await
        .unwrap();
    (socket, context, server)
}
fn config() -> tiktools_rtc::Config {
    tiktools_rtc::Config {
        bind_addresses: vec!["127.0.0.1:0".into()],
        ..Default::default()
    }
}

#[tokio::test]
async fn an_unreachable_webrtc_peer_releases_the_attempt_for_relay_after_eight_seconds() {
    let (mut socket, context, server) = backend(false).await;
    let started = tokio::time::Instant::now();
    let result = direct(&mut socket, &context, &[1; 32], pinned(), config())
        .await
        .unwrap();
    assert!(result.is_none());
    assert!(started.elapsed() >= Duration::from_secs(8));
    socket.close().await;
    assert!(server
        .await
        .unwrap()
        .iter()
        .any(|kind| kind == "rtc.signal"));
}

#[tokio::test]
async fn invalid_signed_sdp_is_rejected_without_attempting_a_relay() {
    let (socket, context, server) = backend(true).await;
    let result = connect(socket, context, &[1; 32], pinned(), config()).await;
    let Err(error) = result else {
        panic!("a forged host must not be admitted");
    };
    assert_eq!(error.code, "unauthorized");
    let observed = server.await.unwrap();
    assert!(!observed.iter().any(|kind| kind == "relay.open"));
}
