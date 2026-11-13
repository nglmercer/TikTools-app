use super::*;
use tiktools_collaboration::RemoteLimits;
use tiktools_peer::{Config, Frame, PeerEndpoint, SecretKey};

#[tokio::test]
async fn peer_admission_and_live_calls_do_not_depend_on_matching_calendar_clocks() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let server = PeerEndpoint::bind(SecretKey::generate(), &Config::default())
            .await
            .unwrap();
        let client = PeerEndpoint::bind(SecretKey::generate(), &Config::default())
            .await
            .unwrap();
        let address = server.address();
        let host_id = uuid::Uuid::new_v4().to_string();
        let expected_host = host_id.clone();
        let principal_id = uuid::Uuid::new_v4().to_string();
        let session_id = uuid::Uuid::new_v4().to_string();
        let host = tokio::spawn(async move {
            let connection = server.accept().await.unwrap();
            let mut wire = connection.accept().await.unwrap();
            assert!(matches!(wire.recv().await.unwrap(), Frame::Text(_)));
            wire.send(Frame::Text(
                json!({
                    "hostId":host_id,"principalId":principal_id,"sessionId":session_id,
                    "clientKind":"agent","accessExpiresAt":0
                })
                .to_string(),
            ))
            .await
            .unwrap();
            let Frame::Text(request) = wire.recv().await.unwrap() else {
                panic!("hello required")
            };
            let request: Value = serde_json::from_str(&request).unwrap();
            assert_eq!(request["method"], "collaboration.hello");
            let hello = HelloResult {
                protocol_version: 1,
                host_id,
                principal_id,
                session_id,
                host_instance_id: uuid::Uuid::new_v4().to_string(),
                connection_id: uuid::Uuid::new_v4().to_string(),
                grant_revision: "1".into(),
                capabilities: Default::default(),
                resource_restrictions: Default::default(),
                server_time: 0,
                limits: RemoteLimits::default(),
                features: Default::default(),
                subscription_domains: vec![],
            };
            wire.send(Frame::Text(
                json!({"jsonrpc":"2.0","id":request["id"],"result":hello}).to_string(),
            ))
            .await
            .unwrap();
            loop {
                match wire.recv().await {
                    Ok(Frame::Ping) => wire.send(Frame::Pong).await.unwrap(),
                    Ok(Frame::Text(raw)) => {
                        let request: Value = serde_json::from_str(&raw).unwrap();
                        wire.send(Frame::Text(
                            json!({"jsonrpc":"2.0","id":request["id"],"result":{"ok":true}})
                                .to_string(),
                        ))
                        .await
                        .unwrap();
                    }
                    Ok(Frame::Close) | Err(_) => break,
                    _ => {}
                }
            }
        });
        let remote = tokio::time::timeout(
            Duration::from_secs(5),
            // As in ProfileManager, keep the shared endpoint alive while the
            // individual session's close frame drains through QUIC.
            RemoteClient::connect_peer(
                client.clone(),
                address,
                ClientKind::Agent,
                Some(&expected_host),
            ),
        )
        .await
        .expect("peer handshake must complete")
        .unwrap();
        // Cross several periodic expiration checks after receiving a host wall
        // timestamp long before this controller's clock. The host, not a stale
        // copied timestamp, remains responsible for current lease admission.
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(!remote.is_closed());
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            remote.call_value_with_meta("live.status", json!({}), None),
        )
        .await
        .expect("RPC response must arrive")
        .unwrap();
        assert_eq!(result.result["ok"], true);
        remote.close();
        tokio::time::timeout(Duration::from_secs(3), host)
            .await
            .expect("host must observe close")
            .unwrap();
    })
    .await
    .unwrap();
}
