
use super::*;
use std::sync::{Arc, Mutex};

#[test]
fn paused_ui_concurrent_feed_producers_are_bounded_and_account_loss() {
    let mailbox = Arc::new(Mutex::new(ForwardMailbox::default()));
    let workers: Vec<_> = (0..4)
        .map(|worker| {
            let mailbox = mailbox.clone();
            std::thread::spawn(move || {
                for id in 0..25_000 {
                    let mut queue = mailbox.lock().unwrap();
                    queue.push_value(
                        &serde_json::json!({"type":"live-event", "worker":worker, "id":id}),
                    );
                    assert!(queue.outbox.lossy.len() <= MAX_PENDING_WEBVIEW_MESSAGES);
                    assert!(queue.outbox.lossy_bytes <= MAX_LOSSY_BYTES);
                }
            })
        })
        .collect();
    for id in 0..100 {
        mailbox
            .lock()
            .unwrap()
            .push_value(&serde_json::json!({"type":"rpc-response", "id":id}));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    let mut mailbox = mailbox.lock().unwrap();
    assert_eq!(mailbox.outbox.reliable.len(), 100);
    assert_eq!(
        mailbox.take_lossy_drops(),
        100_000 - MAX_PENDING_WEBVIEW_MESSAGES as u64
    );
    assert_eq!(mailbox.take_lossy_drops(), 0);
    assert!(!mailbox.outbox.transport_failed());
    let (batch, failed, _) = mailbox.drain();
    assert!(!failed);
    for (id, body) in batch.iter().take(100).enumerate() {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(body).unwrap()["id"],
            id
        );
    }
}

#[test]
fn outbound_encoding_stops_at_item_budget_and_shutdown_releases_backlog() {
    let huge = serde_json::json!({"blob": "x".repeat(MAX_OUTBOUND_BYTES + 1)});
    assert!(encode_bounded(&huge).is_err());
    let mut mailbox = ForwardMailbox::default();
    mailbox.push_value(&huge);
    assert!(mailbox.drain().1);
    mailbox.shutdown();
    assert!(!mailbox.push("{}".to_owned()));
    assert!(mailbox.drain().0.is_empty());
}

#[test]
fn large_owner_rpc_drains_alone_with_escaped_literal_without_widening_event_budget() {
    let value = serde_json::json!({"type":"rpc-response","response":{"id":"library","result":{"source":"\"".repeat(3 * 1024 * 1024)}}});
    assert!(encode_bounded(&value).is_err());
    let encoded = encode_owner_rpc_bounded(&value).unwrap();
    assert!(encoded.len() > MAX_OUTBOUND_BYTES && encoded.len() < MAX_OWNER_RPC_BYTES);
    let mut mailbox = ForwardMailbox::default();
    mailbox.push(encoded);
    let (batch, failed, _) = mailbox.drain();
    assert!(!failed);
    assert_eq!(batch.len(), 1);
    assert!(batch[0].javascript_literal.as_ref().unwrap().len() > MAX_BATCH_BYTES);
    let literal: String =
        serde_json::from_str(batch[0].javascript_literal.as_ref().unwrap()).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&literal).unwrap(),
        value
    );
    assert!(mailbox.drain().0.is_empty());

    let mut events = WebviewOutbox::new();
    events.push_value(
        &serde_json::json!({"type":"live-event","source":"x".repeat(MAX_OUTBOUND_BYTES + 1)}),
    );
    assert!(events.transport_failed());
}

#[test]
fn lossy_bytes_are_bounded_even_below_count_limit() {
    let mut outbox = WebviewOutbox::new();
    let body = serde_json::json!({"type":"live-event", "blob":"x".repeat(512 * 1024)});
    for _ in 0..100 {
        outbox.push_value(&body);
    }
    assert!(outbox.lossy.len() < MAX_PENDING_WEBVIEW_MESSAGES);
    assert!(outbox.lossy_bytes <= MAX_LOSSY_BYTES);
    assert_eq!(outbox.lossy_dropped + outbox.lossy.len() as u64, 100);
    while !outbox.is_empty() {
        let batch = outbox.take_batch();
        assert!(batch.iter().map(String::len).sum::<usize>() <= MAX_BATCH_BYTES);
    }
    assert_eq!(outbox.lossy_bytes, 0);
}
