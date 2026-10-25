//! Public read-only LIVE normalization probe. Never prints event payloads.
use tiktools_tiktok::{
    events::CanonicalLiveEvent, ClientEvent, ConnectRequest, NativeTikTokClient, NativeTikTokConfig,
};
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let client = NativeTikTokClient::new(NativeTikTokConfig::default());
    let mut events = client.subscribe();
    let request = ConnectRequest {
        unique_id: args
            .get(1)
            .expect("usage: live_decode_probe <creator> [room-id]")
            .clone(),
        room_id: args.get(2).cloned(),
        session_cookie: String::new(),
    };
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.connect_native(request),
    )
    .await
    {
        Ok(Ok(_)) => println!("connected"),
        _ => {
            println!("public connection unavailable");
            return;
        }
    }
    let result = tokio::time::timeout(std::time::Duration::from_secs(45), async {
        loop {
            match events.recv().await {
                Ok(ClientEvent::Event(event)) => match event.base {
                    CanonicalLiveEvent::Gift(_) => {
                        println!("canonical gift received");
                        break;
                    }
                    CanonicalLiveEvent::Unknown { method, .. }
                        if method == "WebcastGiftMessage" =>
                    {
                        println!("gift still unknown")
                    }
                    _ => {}
                },
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                _ => {}
            }
        }
    })
    .await;
    if result.is_err() {
        println!("no gift observed within probe window");
    }
    let _ = client.disconnect_native().await;
}
