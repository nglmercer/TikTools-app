//! Read-only LIVE probe. Logs counts and lifecycle progress, never participant payloads.
use std::{collections::BTreeMap, time::Duration};
use tiktools_tiktok::{
    events::CanonicalLiveEvent, ClientEvent, ConnectRequest, NativeTikTokClient, NativeTikTokConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let client = NativeTikTokClient::new(NativeTikTokConfig::default());
    let mut events = client.subscribe();
    let session_cookie = match std::env::var_os("TIKTOOLS_TIKTOK_SESSION_FILE") {
        Some(path) => std::fs::read_to_string(path)?.trim().to_owned(),
        None => String::new(),
    };
    let connection = async {
        match args.get(1) {
            Some(creator) => {
                client
                    .connect_native(ConnectRequest {
                        unique_id: creator.clone(),
                        room_id: args.get(2).cloned(),
                        session_cookie,
                    })
                    .await
            }
            None => client
                .pick_generation(session_cookie)
                .await
                .map(|(_, info)| info),
        }
    };
    match tokio::time::timeout(Duration::from_secs(150), connection).await {
        Ok(Ok(_)) => println!("connected"),
        result => {
            client.disconnect_native().await?;
            return Err(match result {
                Ok(Err(error)) => error.to_string(),
                _ => "connection deadline exceeded".into(),
            }
            .into());
        }
    }
    let seconds = std::env::var("TIKTOOLS_LIVE_PROBE_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(45)
        .clamp(1, 300);
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut lagged = 0;
    let mut failures = 0;
    let _ = tokio::time::timeout(Duration::from_secs(seconds), async {
        loop {
            match events.recv().await {
                Ok(ClientEvent::Event(event)) => {
                    let kind = match event.base {
                        CanonicalLiveEvent::Gift(_) => "gift",
                        CanonicalLiveEvent::Chat(_) => "chat",
                        CanonicalLiveEvent::EmoteChat(_) => "emote_chat",
                        CanonicalLiveEvent::Unknown { .. } => "unknown",
                        _ => "other",
                    };
                    *counts.entry(kind).or_default() += 1;
                }
                Ok(ClientEvent::Reconnecting { attempt, delay_ms }) => {
                    println!("retry attempt={attempt} delay_ms={delay_ms}")
                }
                Ok(ClientEvent::Error { .. }) => failures += 1,
                Ok(ClientEvent::Disconnected { .. })
                | Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => lagged += count,
                _ => {}
            }
        }
    })
    .await;
    client.disconnect_native().await?;
    println!(
        "counts={} lagged={lagged} errors={failures}",
        serde_json::to_string(&counts)?
    );
    if counts.values().sum::<usize>() == 0 || lagged > 0 || failures > 0 {
        return Err("probe did not establish clean event delivery".into());
    }
    Ok(())
}
