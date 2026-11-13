use super::*;
use tiktools_collaboration::RemoteLimits;
use uuid::Uuid;

#[test]
fn hello_retry_requires_correlated_explicit_rate_limit_and_is_bounded() {
    let refused = |id: &str, retry: u64| {
        json!({"jsonrpc":"2.0","id":id,
        "error":{"code":"rate_limited","message":"limited","data":{"retryAfterMs":retry}}})
    };
    assert!(
        matches!(classify_hello(refused("hello-0", 1), "hello-0", 0),
        Ok(HelloReply::Retry(wait)) if wait == Duration::from_millis(1000))
    );
    assert!(
        matches!(classify_hello(refused("hello-1", 999_999), "hello-1", 1),
        Ok(HelloReply::Retry(wait)) if wait == Duration::from_millis(2000))
    );
    assert_eq!(
        classify_hello(refused("hello-0", 1000), "hello-1", 1)
            .err()
            .unwrap()
            .code,
        "protocol"
    );
    assert_eq!(
        classify_hello(refused("hello-4", 1000), "hello-4", 4)
            .err()
            .unwrap()
            .code,
        "rate_limited"
    );
    assert_eq!(
        classify_hello(
            json!({"jsonrpc":"2.0","id":"hello-0",
        "error":{"code":"protocol_incompatible","message":"wrong version"}}),
            "hello-0",
            0
        )
        .err()
        .unwrap()
        .code,
        "protocol_incompatible"
    );
    assert_eq!(
        classify_hello(
            json!({"jsonrpc":"2.0","id":"hello-0","result":{},
        "error":{"code":"rate_limited"}}),
            "hello-0",
            0
        )
        .err()
        .unwrap()
        .code,
        "protocol"
    );
}
async fn pair() -> (RemoteClient, WebSocketStream<TcpStream>, NativeCredentials) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (tcp, accepted) = tokio::join!(
        TcpStream::connect(listener.local_addr().unwrap()),
        listener.accept()
    );
    let socket = WebSocketStream::from_raw_socket(
        MaybeTlsStream::Plain(tcp.unwrap()),
        tokio_tungstenite::tungstenite::protocol::Role::Client,
        None,
    )
    .await;
    let server = WebSocketStream::from_raw_socket(
        accepted.unwrap().0,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    )
    .await;
    let creds = NativeCredentials {
        origin: "https://example.com".into(),
        host_id: Uuid::new_v4().to_string(),
        principal_id: Uuid::new_v4().to_string(),
        session_id: Uuid::new_v4().to_string(),
        access: SecretString::from("a".repeat(64)),
        refresh: SecretString::from("b".repeat(64)),
        access_expires_at: now() + 900_000,
        expires_at: now() + 86_400_000,
    };
    let hello = HelloResult {
        protocol_version: 1,
        host_id: creds.host_id.clone(),
        host_instance_id: Uuid::new_v4().to_string(),
        connection_id: Uuid::new_v4().to_string(),
        principal_id: creds.principal_id.clone(),
        session_id: creds.session_id.clone(),
        grant_revision: "1".into(),
        capabilities: Default::default(),
        resource_restrictions: Default::default(),
        server_time: now(),
        limits: RemoteLimits::default(),
        features: Default::default(),
        subscription_domains: Vec::new(),
    };
    (
        RemoteClient::from_negotiated(socket, hello, &creds, ClientKind::Agent),
        server,
        creds,
    )
}
async fn request(server: &mut WebSocketStream<TcpStream>) -> Value {
    loop {
        match server.next().await.unwrap().unwrap() {
            Message::Text(raw) => return serde_json::from_str(&raw).unwrap(),
            Message::Ping(bytes) => server.send(Message::Pong(bytes)).await.unwrap(),
            _ => {}
        }
    }
}
#[tokio::test]
async fn losing_the_ack_fails_once_without_retrying_the_command() {
    let (client, mut server, _) = pair().await;
    let host = tokio::spawn(async move {
        let value = request(&mut server).await;
        assert_eq!(value["method"], "workflows.save");
        assert_eq!(value["meta"]["commandId"], "once");
        drop(server);
        1
    });
    let meta = RequestMetadata {
        command_id: Some("once".into()),
        host_instance_id: Some(client.hello().host_instance_id.clone()),
        expected_revision: Some("0".into()),
    };
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        client.call_value_with_meta(
            "workflows.save",
            json!({"graph":{"id":"welcome","name":"Once"}}),
            Some(meta),
        ),
    )
    .await
    .unwrap()
    .err()
    .unwrap();
    assert_eq!(error.code, "transport");
    assert_eq!(host.await.unwrap(), 1);
    assert!(client.is_closed());
}
#[tokio::test]
async fn malformed_correlated_responses_drain_pending_calls_immediately() {
    let (client, mut server, _) = pair().await;
    let host = tokio::spawn(async move {
        let value = request(&mut server).await;
        server
            .send(Message::Text(
                json!({"jsonrpc":"2.0","id":value["id"],"result":{},"error":{"code":"forbidden"}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
    });
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        client.call_value_with_meta("live.status", json!({}), None),
    )
    .await
    .unwrap()
    .err()
    .unwrap();
    assert_eq!(error.code, "protocol");
    assert!(client.is_closed());
    assert!(client.requires_resync());
    host.await.unwrap();
}
#[tokio::test]
async fn error_reflections_are_redacted_in_keys_and_values_while_version_data_survives() {
    let (client, mut server, creds) = pair().await;
    let reflected = creds.clone();
    let host = tokio::spawn(async move {
        let value = request(&mut server).await;
        let mut map = serde_json::Map::new();
        map.insert(
            reflected.refresh.expose_secret().into(),
            json!(reflected.access.expose_secret()),
        );
        let response = json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":"revision_conflict","message":reflected.access.expose_secret(),"data":{"reflected":map,"resourceVersions":[{"kind":"workflow","scope":"host","id":"welcome","revision":"2"}]}}});
        server
            .send(Message::Text(response.to_string().into()))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    });
    let error = client
        .call_value_with_meta("workflows.save", json!({}), None)
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, "revision_conflict");
    assert_eq!(
        error.data.as_ref().unwrap()["resourceVersions"][0]["revision"],
        "2"
    );
    let text = format!("{error:?}");
    assert!(!text.contains(creds.access.expose_secret()));
    assert!(!text.contains(creds.refresh.expose_secret()));
    client.close();
    host.await.unwrap();
}
#[tokio::test]
async fn silent_peers_close_on_the_heartbeat_deadline_and_admission_remains_bounded() {
    let (client, _server, _) = pair().await;
    tokio::time::pause();
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(31)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert!(client.is_closed());
    tokio::time::resume();
}
