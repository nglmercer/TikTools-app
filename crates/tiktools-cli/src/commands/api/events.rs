//! Bounded NDJSON event streaming and client-side topic filters.
use super::*;

/// Streams NDJSON events until `--max-events`, `--timeout`, the host going
/// away, or Ctrl-C. Every line is `{topic, data}`; local or host gaps
/// surface as `event.gap` markers so consumers resync explicitly.
pub(super) async fn stream_events(
    client: &TikToolsClient,
    topics: &[String],
    timeout_secs: Option<u64>,
    max_events: Option<u64>,
) -> Result<(), ClientError> {
    #[cfg(feature = "remote-client")]
    if let TikToolsClient::Remote(remote) = client {
        return crate::remote::events(remote.clone(), topics, timeout_secs, max_events).await;
    }
    eprintln!("tiktools: listening for events (Ctrl-C to stop)");
    let mut events = client.subscribe();
    let deadline = timeout_secs.map(|secs| tokio::time::Instant::now() + Duration::from_secs(secs));
    let mut printed: u64 = 0;
    loop {
        if max_events.is_some_and(|max| printed >= max) {
            break;
        }
        let event = match deadline {
            Some(deadline) => match tokio::time::timeout_at(deadline, events.recv()).await {
                Ok(outcome) => outcome,
                Err(_) => break,
            },
            None => events.recv().await,
        };
        match event {
            Ok(ControlEvent::Reliable(event)) => {
                if !topic_allowed(topics, &event.topic) {
                    continue;
                }
                println!("{}", serde_json::to_string(&event).unwrap_or_default());
                printed += 1;
            }
            Ok(ControlEvent::Domain(event)) => {
                if !topic_allowed(topics, event.topic()) {
                    continue;
                }
                let line = serde_json::to_value(&event).unwrap_or(Value::Null);
                println!("{}", serde_json::to_string(&line).unwrap_or_default());
                printed += 1;
            }
            Ok(ControlEvent::Gap { lost }) => {
                println!(
                    "{}",
                    json!({"topic": "event.gap", "data": {"lost": lost, "resync": true}})
                );
                printed += 1;
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(lost)) => {
                println!(
                    "{}",
                    json!({"topic": "event.gap", "data": {"lost": lost, "resync": true}})
                );
                printed += 1;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                eprintln!("tiktools: event stream closed by the host");
                break;
            }
        }
    }
    Ok(())
}

/// Client-side topic filter: exact topics, the `*` match-all wildcard,
/// and `prefix.*` namespace wildcards covering the bare `prefix` plus
/// every `prefix.`-scoped child.
pub(super) fn topic_allowed(filters: &[String], topic: &str) -> bool {
    filters.is_empty()
        || filters.iter().any(|filter| {
            filter == "*"
                || filter == topic
                || filter.strip_suffix(".*").is_some_and(|prefix| {
                    !prefix.is_empty()
                        && (topic == prefix
                            || topic
                                .strip_prefix(prefix)
                                .is_some_and(|rest| rest.starts_with('.')))
                })
        })
}
