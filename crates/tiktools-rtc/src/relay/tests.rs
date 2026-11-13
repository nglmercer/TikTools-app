use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

fn contexts() -> (Context, Context) {
    let controller = Context {
        host_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        host_device_id: Uuid::new_v4(),
        from_device_id: Uuid::new_v4(),
        negotiation_id: Uuid::new_v4(),
        grant_revision: 7,
    };
    let mut host = controller.clone();
    host.from_device_id = controller.host_device_id;
    (controller, host)
}
fn key(secret: &[u8; 32]) -> [u8; 32] {
    Ed25519KeyPair::from_seed_unchecked(secret)
        .unwrap()
        .public_key()
        .as_ref()
        .try_into()
        .unwrap()
}

#[tokio::test]
async fn device_bound_mutual_tls_hides_rpc_from_the_opaque_relay() {
    let (controller_ctx, host_ctx) = contexts();
    let controller = Identity::new(controller_ctx.clone(), &[1; 32]).unwrap();
    let host = Identity::new(host_ctx.clone(), &[2; 32]).unwrap();
    let verified_host = controller
        .verify(&host_ctx, key(&[2; 32]), host.announcement())
        .unwrap();
    let verified_controller = host
        .verify(&controller_ctx, key(&[1; 32]), controller.announcement())
        .unwrap();
    let (client_io, mut relay_client) = tokio::io::duplex(4096);
    let (mut relay_host, server_io) = tokio::io::duplex(4096);
    let relay = tokio::spawn(async move {
        let mut captured = vec![];
        let mut from_client = [0; 1024];
        let mut from_host = [0; 1024];
        loop {
            tokio::select! {
                n = relay_client.read(&mut from_client) => {
                    let n = n.unwrap();
                    if n == 0 { break; }
                    captured.extend_from_slice(&from_client[..n]);
                    if relay_host.write_all(&from_client[..n]).await.is_err() { break; }
                }
                n = relay_host.read(&mut from_host) => {
                    let n = n.unwrap();
                    if n == 0 { break; }
                    captured.extend_from_slice(&from_host[..n]);
                    if relay_client.write_all(&from_host[..n]).await.is_err() { break; }
                }
            }
        }
        captured
    });
    let (controller, host) = tokio::join!(
        controller.connect(verified_host, client_io),
        host.accept(verified_controller, server_io)
    );
    let controller = controller.unwrap();
    let host = host.unwrap();
    assert!(controller.local_context() == &controller_ctx);
    assert!(controller.remote_context() == &host_ctx);
    assert_eq!(controller.local_endpoint_id(), endpoint(&key(&[1; 32])));
    assert_eq!(controller.remote_endpoint_id(), endpoint(&key(&[2; 32])));
    assert_eq!(host.local_endpoint_id(), controller.remote_endpoint_id());
    assert_eq!(host.remote_endpoint_id(), controller.local_endpoint_id());
    let mut controller = controller.into_stream();
    let mut host = host.into_stream();
    assert_eq!(
        controller.get_ref().1.protocol_version(),
        Some(rustls::ProtocolVersion::TLSv1_3)
    );
    assert!(!controller
        .get_ref()
        .1
        .peer_certificates()
        .unwrap()
        .is_empty());
    assert!(!host.get_ref().1.peer_certificates().unwrap().is_empty());
    let request = br#"{"operation":"points.adjust","secret":"relay-must-not-read-this"}"#;
    controller.write_all(request).await.unwrap();
    controller.flush().await.unwrap();
    let mut received = vec![0; request.len()];
    host.read_exact(&mut received).await.unwrap();
    assert_eq!(received, request);
    host.write_all(b"receipt:accepted").await.unwrap();
    host.flush().await.unwrap();
    let mut reply = [0; 16];
    controller.read_exact(&mut reply).await.unwrap();
    assert_eq!(&reply, b"receipt:accepted");
    drop(controller);
    drop(host);
    let captured = relay.await.unwrap();
    assert!(!captured.windows(request.len()).any(|part| part == request));
    assert!(!captured.windows(16).any(|part| part == b"receipt:accepted"));
}

#[tokio::test]
async fn relay_identity_rejects_modified_certificates_contexts_and_device_keys() {
    let (controller_ctx, host_ctx) = contexts();
    let controller = Identity::new(controller_ctx, &[1; 32]).unwrap();
    let host = Identity::new(host_ctx.clone(), &[2; 32]).unwrap();
    let announcement = host.announcement();
    for field in [
        "hostId",
        "sessionId",
        "hostDeviceId",
        "fromDeviceId",
        "negotiationId",
        "grantRevision",
    ] {
        let mut altered = serde_json::to_value(&announcement).unwrap();
        altered["context"][field] = if field == "grantRevision" {
            serde_json::json!(8)
        } else {
            serde_json::json!(Uuid::new_v4())
        };
        assert!(controller
            .verify(
                &host_ctx,
                key(&[2; 32]),
                serde_json::from_value(altered).unwrap()
            )
            .is_err());
    }
    let mut altered = announcement.clone();
    altered.certificate = Identity::new(host_ctx.clone(), &[2; 32])
        .unwrap()
        .announcement()
        .certificate;
    assert!(controller
        .verify(&host_ctx, key(&[2; 32]), altered)
        .is_err());
    assert!(controller
        .verify(&host_ctx, key(&[3; 32]), announcement.clone())
        .is_err());
    let mut altered = announcement;
    altered.signature.push('a');
    assert!(controller
        .verify(&host_ctx, key(&[2; 32]), altered)
        .is_err());
}

#[tokio::test]
async fn signed_root_does_not_allow_a_relay_to_substitute_its_tls_identity() {
    let (controller_ctx, host_ctx) = contexts();
    let controller = Identity::new(controller_ctx.clone(), &[1; 32]).unwrap();
    let honest_host = Identity::new(host_ctx.clone(), &[2; 32]).unwrap();
    // Even another certificate signed by the same device key is not the
    // certificate the controller verified for this handshake.
    let substituted_host = Identity::new(host_ctx, &[2; 32]).unwrap();
    let expected_host = controller
        .verify(
            &honest_host.context,
            key(&[2; 32]),
            honest_host.announcement(),
        )
        .unwrap();
    let expected_controller = substituted_host
        .verify(&controller_ctx, key(&[1; 32]), controller.announcement())
        .unwrap();
    let (client_io, server_io) = tokio::io::duplex(4096);
    let (controller, host) = tokio::join!(
        controller.connect(expected_host, client_io),
        substituted_host.accept(expected_controller, server_io)
    );
    assert!(controller.is_err());
    assert!(host.is_err());
}

#[tokio::test(start_paused = true)]
async fn abandoned_relay_handshakes_expire_without_wall_clock_authority() {
    let (controller_ctx, host_ctx) = contexts();
    let controller = Identity::new(controller_ctx, &[1; 32]).unwrap();
    let host = Identity::new(host_ctx.clone(), &[2; 32]).unwrap();
    let announcement = host.announcement();
    let peer = controller
        .verify(&host_ctx, key(&[2; 32]), announcement.clone())
        .unwrap();
    tokio::time::advance(DEADLINE).await;
    assert!(controller
        .verify(&host_ctx, key(&[2; 32]), announcement)
        .is_err());
    let (stream, _unresponsive_relay) = tokio::io::duplex(4096);
    assert!(controller.connect(peer, stream).await.is_err());
}

#[tokio::test(start_paused = true)]
async fn a_silent_relay_cannot_hold_the_tls_handshake_open() {
    let (controller_ctx, host_ctx) = contexts();
    let controller = Identity::new(controller_ctx, &[1; 32]).unwrap();
    let host = Identity::new(host_ctx.clone(), &[2; 32]).unwrap();
    let peer = controller
        .verify(&host_ctx, key(&[2; 32]), host.announcement())
        .unwrap();
    let (stream, _unresponsive_relay) = tokio::io::duplex(4096);
    assert!(controller.connect(peer, stream).await.is_err());
}

async fn channels() -> (
    SecureChannel<tokio::io::DuplexStream>,
    SecureChannel<tokio::io::DuplexStream>,
) {
    let (controller_ctx, host_ctx) = contexts();
    let controller = Identity::new(controller_ctx.clone(), &[1; 32]).unwrap();
    let host = Identity::new(host_ctx.clone(), &[2; 32]).unwrap();
    let verified_host = controller
        .verify(&host_ctx, key(&[2; 32]), host.announcement())
        .unwrap();
    let verified_controller = host
        .verify(&controller_ctx, key(&[1; 32]), controller.announcement())
        .unwrap();
    let (client_io, server_io) = tokio::io::duplex(4096);
    let (controller, host) = tokio::join!(
        controller.connect(verified_host, client_io),
        host.accept(verified_controller, server_io)
    );
    (controller.unwrap(), host.unwrap())
}

#[tokio::test]
async fn relay_frames_carry_maximum_rpc_and_control_frames_without_an_unbounded_queue() {
    use crate::{Frame, MAX_FRAME_BYTES};
    let (controller, host) = channels().await;
    let mut controller = controller.into_framed();
    let mut host = host.into_framed();
    let payload = "x".repeat(MAX_FRAME_BYTES);
    let (sent, received) = tokio::join!(controller.send(Frame::Text(payload.clone())), host.recv());
    sent.unwrap();
    assert!(matches!(received.unwrap(), Frame::Text(s) if s == payload));
    assert!(controller
        .send(Frame::Text("x".repeat(MAX_FRAME_BYTES + 1)))
        .await
        .is_err());
    // Rejecting an unsent oversized frame need not poison an otherwise healthy stream.
    controller.send(Frame::Ping).await.unwrap();
    assert!(matches!(host.recv().await.unwrap(), Frame::Ping));
    host.send(Frame::Pong).await.unwrap();
    assert!(matches!(controller.recv().await.unwrap(), Frame::Pong));
    controller.send(Frame::Text(String::new())).await.unwrap();
    assert!(matches!(host.recv().await.unwrap(), Frame::Text(s) if s.is_empty()));
    controller.send(Frame::Close).await.unwrap();
    assert!(matches!(host.recv().await.unwrap(), Frame::Close));
    assert!(host.recv().await.is_err());
    assert!(controller.send(Frame::Ping).await.is_err());
}

#[tokio::test]
async fn cancelling_a_relay_receive_preserves_partial_plaintext() {
    use crate::Frame;
    let (controller, host) = channels().await;
    let mut controller = controller.into_stream();
    let mut host = host.into_framed();
    controller
        .write_all(&[0, 0, 0, 0, 6, b'a', b'b'])
        .await
        .unwrap();
    controller.flush().await.unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(5), host.recv())
        .await
        .is_err());
    controller.write_all(b"cdef").await.unwrap();
    controller.flush().await.unwrap();
    assert!(matches!(host.recv().await.unwrap(), Frame::Text(s) if s == "abcdef"));
}

#[tokio::test]
async fn cancelling_a_partial_relay_send_prevents_replaying_an_uncertain_command() {
    use crate::{Frame, MAX_FRAME_BYTES};
    let (controller, _host) = channels().await;
    let mut controller = controller.into_framed();
    assert!(tokio::time::timeout(
        Duration::from_millis(5),
        controller.send(Frame::Text("x".repeat(MAX_FRAME_BYTES)))
    )
    .await
    .is_err());
    assert!(controller.send(Frame::Text("retry".into())).await.is_err());
}

#[tokio::test]
async fn malformed_or_oversized_relay_frames_poison_the_receiver() {
    use crate::{Frame, MAX_FRAME_BYTES};
    for header in [vec![4, 0, 0, 0, 0], vec![1, 0, 0, 0, 1], {
        let mut header = vec![0];
        header.extend_from_slice(&((MAX_FRAME_BYTES + 1) as u32).to_be_bytes());
        header
    }] {
        let (controller, host) = channels().await;
        let mut controller = controller.into_stream();
        let mut host = host.into_framed();
        controller.write_all(&header).await.unwrap();
        controller.flush().await.unwrap();
        assert!(host.recv().await.is_err());
        assert!(host.recv().await.is_err());
        assert!(host.send(Frame::Ping).await.is_err());
    }
}

#[tokio::test]
async fn mutual_tls_rpc_survives_chunking_through_the_native_record_bridge() {
    use crate::{Frame, MAX_FRAME_BYTES};
    let (controller_ctx, host_ctx) = contexts();
    let controller = Identity::new(controller_ctx.clone(), &[1; 32]).unwrap();
    let host = Identity::new(host_ctx.clone(), &[2; 32]).unwrap();
    let verified_host = controller
        .verify(&host_ctx, key(&[2; 32]), host.announcement())
        .unwrap();
    let verified_controller = host
        .verify(&controller_ctx, key(&[1; 32]), controller.announcement())
        .unwrap();
    let (out_a, mut packets_a) = tokio::sync::mpsc::channel::<serde_json::Value>(64);
    let (out_b, mut packets_b) = tokio::sync::mpsc::channel::<serde_json::Value>(64);
    let (stream_a, mut incoming_a) =
        super::io::stream(controller_ctx.clone(), host_ctx.clone(), out_a).unwrap();
    let (stream_b, mut incoming_b) =
        super::io::stream(host_ctx.clone(), controller_ctx.clone(), out_b).unwrap();
    let mut forwarding = tokio::task::JoinSet::new();
    forwarding.spawn(async move {
        while let Some(mut frame) = packets_a.recv().await {
            frame["fromDeviceId"] = serde_json::json!(controller_ctx.from_device_id);
            incoming_b.apply(serde_json::from_value(frame).unwrap())?;
        }
        Ok::<_, Error>(())
    });
    forwarding.spawn(async move {
        while let Some(mut frame) = packets_b.recv().await {
            frame["fromDeviceId"] = serde_json::json!(host_ctx.from_device_id);
            incoming_a.apply(serde_json::from_value(frame).unwrap())?;
        }
        Ok::<_, Error>(())
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        let (controller, host) = tokio::join!(
            controller.connect(verified_host, stream_a),
            host.accept(verified_controller, stream_b)
        );
        let mut controller = controller.unwrap().into_framed();
        let mut host = host.unwrap().into_framed();
        let payload = "x".repeat(MAX_FRAME_BYTES);
        let (sent, received) =
            tokio::join!(controller.send(Frame::Text(payload.clone())), host.recv());
        sent.unwrap();
        assert!(matches!(received.unwrap(), Frame::Text(s) if s == payload));
        host.send(Frame::Text("receipt".into())).await.unwrap();
        assert!(matches!(controller.recv().await.unwrap(), Frame::Text(s) if s == "receipt"));
    })
    .await
    .unwrap();
    forwarding.abort_all();
    while forwarding.join_next().await.is_some() {}
}
