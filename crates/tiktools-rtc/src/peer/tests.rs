use super::*;
use crate::{Frame, MAX_FRAME_BYTES};
use std::time::Duration;

async fn connected() -> (Peer, Peer, Framed, Framed) {
    let config = || Config {
        bind_addresses: vec!["127.0.0.1:0".into()],
        ..Default::default()
    };
    let mut left = Peer::new(config()).await.unwrap();
    let mut right = Peer::new(config()).await.unwrap();
    let offer = left.offer().await.unwrap();
    let answer = right.apply(offer).await.unwrap().unwrap();
    left.apply(answer).await.unwrap();
    let (mut a, mut b) = (None, None);
    tokio::time::timeout(Duration::from_secs(20), async {
        while a.is_none() || b.is_none() {
            tokio::select! {
                event=left.next_event()=>match event.unwrap() {
                    Event::Signal(signal)=>{right.apply(signal).await.unwrap();},
                    Event::Connected(stream)=>{assert!(a.is_none());a=Some(stream);},
                },
                event=right.next_event()=>match event.unwrap() {
                    Event::Signal(signal)=>{left.apply(signal).await.unwrap();},
                    Event::Connected(stream)=>{assert!(b.is_none());b=Some(stream);},
                },
            }
        }
    })
    .await
    .expect("native ICE/DTLS/SCTP connection");
    (left, right, a.unwrap(), b.unwrap())
}

#[tokio::test]
async fn direct_native_channel_preserves_large_frames_and_close() {
    let (left, right, mut sender, mut receiver) = connected().await;
    tokio::time::timeout(Duration::from_secs(3), async {
        while left.route() != Some(crate::Route::Direct)
            || right.route() != Some(crate::Route::Direct)
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("selected ICE transport must report the direct route");
    let payload = "x".repeat(MAX_FRAME_BYTES);
    let (sent, received) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(sender.send(Frame::Text(payload.clone())), receiver.recv())
    })
    .await
    .unwrap();
    sent.unwrap();
    assert!(matches!(received.unwrap(),Frame::Text(text) if text==payload));
    sender.send(Frame::Ping).await.unwrap();
    assert!(matches!(receiver.recv().await.unwrap(), Frame::Ping));
    assert!(matches!(
        sender
            .send(Frame::Text("x".repeat(MAX_FRAME_BYTES + 1)))
            .await,
        Err(Error::Frame)
    ));
    sender
        .send(Frame::Text("still usable".into()))
        .await
        .unwrap();
    assert!(matches!(receiver.recv().await.unwrap(),Frame::Text(s) if s=="still usable"));
    left.close().await;
    assert!(
        tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .unwrap()
            .is_err()
    );
    right.close().await;
}

#[tokio::test]
async fn negotiation_rejects_replayed_descriptions() {
    let mut peer = Peer::new(Config {
        bind_addresses: vec!["127.0.0.1:0".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    assert!(matches!(
        peer.apply(Signal::EndOfCandidates).await,
        Err(Error::Negotiation)
    ));
    peer.offer().await.unwrap();
    assert!(matches!(peer.offer().await, Err(Error::Negotiation)));
    peer.close().await;
}

#[tokio::test]
async fn cancelled_receive_keeps_channel_usable() {
    let (left, right, mut sender, mut receiver) = connected().await;
    assert!(
        tokio::time::timeout(Duration::from_millis(10), receiver.recv())
            .await
            .is_err()
    );
    sender
        .send(Frame::Text("after cancellation".into()))
        .await
        .unwrap();
    assert!(matches!(receiver.recv().await.unwrap(),Frame::Text(s) if s=="after cancellation"));
    drop(sender);
    assert!(
        tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .unwrap()
            .is_err()
    );
    left.close().await;
    right.close().await;
}

#[tokio::test]
async fn configured_stun_server_produces_a_server_reflexive_candidate() {
    use stun::{
        fingerprint::FINGERPRINT,
        message::{Message, BINDING_REQUEST, BINDING_SUCCESS},
        xoraddr::XorMappedAddress,
    };
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = socket.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut buffer = [0; 2048];
        loop {
            let (length, source) = socket.recv_from(&mut buffer).await.unwrap();
            let mut request = Message::new();
            request.unmarshal_binary(&buffer[..length]).unwrap();
            assert_eq!(request.typ, BINDING_REQUEST);
            let mut response = Message::new();
            response
                .build(&[
                    Box::new(BINDING_SUCCESS),
                    Box::new(request.transaction_id),
                    Box::new(XorMappedAddress {
                        ip: "203.0.113.7".parse().unwrap(),
                        port: 55001,
                    }),
                    Box::new(FINGERPRINT),
                ])
                .unwrap();
            socket.send_to(&response.raw, source).await.unwrap();
        }
    });
    let mut peer = Peer::new(Config {
        bind_addresses: vec!["127.0.0.1:0".into()],
        ice_servers: vec![IceServer {
            urls: vec![format!("stun:{address}")],
            username: String::new(),
            credential: String::new(),
        }],
    })
    .await
    .unwrap();
    peer.offer().await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match peer.next_signal().await.unwrap() {
                Signal::Candidate { candidate, .. } if candidate.contains(" typ srflx") => {
                    assert!(candidate.contains("203.0.113.7 55001"));
                    break;
                }
                Signal::EndOfCandidates => {
                    panic!("configured STUN discovery must yield the mapped address")
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    peer.close().await;
    server.abort();
    let _ = server.await;
}
