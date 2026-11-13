use super::*;
use iroh::{endpoint::presets, tls::CaTlsConfig, RelayMode};
use std::time::Duration;

async fn endpoints(relay_only: bool) -> (PeerEndpoint, PeerEndpoint, impl AsyncFnOnce()) {
    let (relays, _, relay) = iroh::test_utils::run_relay_server().await.unwrap();
    let make = |relays| {
        let builder = iroh::Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Custom(relays))
            // Only this local self-signed relay fixture bypasses CA verification.
            // Endpoint-to-endpoint identity authentication remains enabled.
            .ca_tls_config(CaTlsConfig::insecure_skip_verify())
            .alpns(vec![ALPN.to_vec()]);
        if relay_only {
            builder.clear_ip_transports()
        } else {
            builder
        }
    };
    let host = PeerEndpoint {
        endpoint: make(relays.clone()).bind().await.unwrap(),
    };
    let client = PeerEndpoint {
        endpoint: make(relays).bind().await.unwrap(),
    };
    tokio::join!(host.endpoint.online(), client.endpoint.online());
    (host, client, async move || relay.shutdown().await.unwrap())
}

#[tokio::test]
async fn relay_carries_authenticated_frames_when_ip_transport_is_unavailable() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (host, client, shutdown) = endpoints(true).await;
        let (accepted, connected) = tokio::join!(host.accept(), client.connect(host.address()));
        let accepted = accepted.unwrap();
        let connected = connected.unwrap();
        let mut sending = connected.open().await.unwrap();
        sending.send(Frame::Text("via relay".into())).await.unwrap();
        let mut receiving = accepted.accept().await.unwrap();
        assert!(matches!(receiving.recv().await.unwrap(), Frame::Text(s) if s == "via relay"));
        assert_eq!(connected.route(), Route::Relay);
        assert_eq!(accepted.remote_id(), client.id());
        host.close().await;
        client.close().await;
        shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn established_direct_channel_survives_loss_of_the_relay_service() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (host, client, shutdown) = endpoints(false).await;
        let (accepted, connected) = tokio::join!(host.accept(), client.connect(host.address()));
        let accepted = accepted.unwrap();
        let connected = connected.unwrap();
        let mut sending = connected.open().await.unwrap();
        sending
            .send(Frame::Text("before outage".into()))
            .await
            .unwrap();
        let mut receiving = accepted.accept().await.unwrap();
        receiving.recv().await.unwrap();
        while connected.route() != Route::Direct || accepted.route() != Route::Direct {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        shutdown().await;
        sending
            .send(Frame::Text("after outage".into()))
            .await
            .unwrap();
        assert!(matches!(receiving.recv().await.unwrap(), Frame::Text(s) if s == "after outage"));
        receiving
            .send(Frame::Text("still connected".into()))
            .await
            .unwrap();
        assert!(matches!(sending.recv().await.unwrap(), Frame::Text(s) if s == "still connected"));
        host.close().await;
        client.close().await;
    })
    .await
    .unwrap();
}
