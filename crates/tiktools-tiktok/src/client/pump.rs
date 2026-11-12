//! Owned live event loop with cancellation, generation checks and bounded close.
use crate::{events, lifecycle, ClientEvent, ErrorPhase, GiftInfo, ManagedConnection};
use std::time::Duration;

pub(in super::super) async fn run_connection(
    mut connection: ManagedConnection,
    stop: tokio_util::sync::CancellationToken,
    event_sender: lifecycle::EventSender,
    state: lifecycle::LifecycleState,
    connection_generation: u64,
    gifts: Vec<GiftInfo>,
) {
    tracing::debug!(connection_generation, "TikTok live event task started");
    let mut gifts = gifts
        .into_iter()
        .map(|gift| (gift.id.clone(), gift))
        .collect::<std::collections::HashMap<_, _>>();
    loop {
        tokio::select! {
            biased;
            _ = stop.cancelled() => {
                if tokio::time::timeout(Duration::from_secs(3), connection.close()).await.is_err() {
                    event_sender.send(connection_generation, ClientEvent::Error { phase: ErrorPhase::Live, message: "transport close deadline exceeded; socket released".into() });
                    tracing::warn!(connection_generation, "transport close deadline exceeded");
                }
                tracing::debug!(connection_generation, "TikTok live event task stopped");
                return;
            }
            message = connection.next_message() => match message {
                Some(Ok(message)) => {
                    tracing::debug!(
                        log_id = message.log_id,
                        bytes = message.payload.len(),
                        "TikTok live frame received"
                    );
                    match events::decode_batch(&message.payload, &mut gifts) {
                    Ok(batch) => {
                        tracing::debug!(events = batch.len(), "TikTok live frame decoded");
                        for event in batch {
                            if state.snapshot().generation != connection_generation {
                                let _ = connection.close().await;
                                tracing::debug!(connection_generation, "discarding stale TikTok live connection");
                                return;
                            }
                            tracing::debug!(method = event.method(), "TikTok live event decoded");
                            event_sender.send(connection_generation, ClientEvent::Event(event));
                        }
                    }
                    Err(error) => {
                        event_sender.send(connection_generation, ClientEvent::Error {
                            phase: ErrorPhase::Live,
                            message: format!("could not decode TikTok event batch: {error}"),
                        });
                    }
                    }
                }
                Some(Err(error)) => {
                    event_sender.send(connection_generation, ClientEvent::Error {
                        phase: ErrorPhase::Live,
                        message: error.to_string(),
                    });
                    event_sender.send(connection_generation, ClientEvent::Disconnected { reason: error.to_string() });
                    tracing::warn!(%error, connection_generation, "TikTok live event task ended after a transport error");
                    return;
                }
                None => {
                    event_sender.send(connection_generation, ClientEvent::Disconnected { reason: "TikTok closed the live stream".to_owned() });
                    tracing::info!(connection_generation, "TikTok live event task ended because the stream closed");
                    return;
                }
            }
        }
    }
}
