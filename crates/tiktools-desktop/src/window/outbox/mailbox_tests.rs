use super::*;

#[test]
fn high_rate_snapshots_have_one_wake_and_one_latest_snapshot() {
    let mut mailbox = ForwardMailbox::default();
    for n in 0..10_000 {
        let wake = mailbox.push_value(&serde_json::json!({"type":"room-stats", "viewers":n}));
        assert_eq!(wake, n == 0);
    }
    let (batch, failed, wake) = mailbox.drain();
    assert!(!failed);
    assert!(!wake);
    assert_eq!(batch.len(), 1);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&batch[0]).unwrap()["viewers"],
        9999
    );
}

#[test]
fn value_path_classifies_without_reparsing() {
    let mut mailbox = ForwardMailbox::default();
    // Critical via the value path lands in the reliable lane.
    assert!(mailbox.push_value(&serde_json::json!({"type": "rpc-response", "id": 1})));
    // Feed via the value path is droppable; snapshots coalesce.
    let feed = serde_json::json!({"method": "event", "params": {"topic": "live.ui-event"}});
    for _ in 0..10 {
        mailbox.push_value(&feed);
    }
    let stats = serde_json::json!({"method": "event", "params": {"topic": "room.stats", "n": 1}});
    mailbox.push_value(&stats);
    let stats2 = serde_json::json!({"method": "event", "params": {"topic": "room.stats", "n": 2}});
    mailbox.push_value(&stats2);
    let (batch, failed, wake) = mailbox.drain();
    assert!(!failed);
    assert!(!wake);
    // 1 reliable + 10 feed + 1 coalesced snapshot (n=2 replaced n=1).
    assert_eq!(batch.len(), 12);
    assert!(batch[0].contains("rpc-response"));
    assert!(
        batch
            .iter()
            .filter(|line| line.contains("room.stats"))
            .count()
            == 1
    );
    assert!(batch.last().is_some_and(|line| line.contains("\"n\":2")));
}

#[test]
fn forced_teardown_while_saturated_releases_everything() {
    let mut mailbox = ForwardMailbox::default();
    // Saturate the reliable lane past the safety policy while feed
    // keeps arriving, as during teardown under load.
    for index in 0..MAX_RELIABLE_MESSAGES + 64 {
        mailbox.push_value(&serde_json::json!({"type": "rpc-response", "id": index}));
    }
    for _ in 0..MAX_PENDING_WEBVIEW_MESSAGES + 64 {
        mailbox.push_value(
            &serde_json::json!({"method": "event", "params": {"topic": "live.ui-event"}}),
        );
    }
    // Forced teardown: the page is gone mid-burst.
    let (batch, failed, wake) = mailbox.drain();
    assert!(failed, "saturation must surface as transport failure");
    assert!(
        batch.is_empty(),
        "failed transport clears instead of delivering"
    );
    assert!(!wake);
    // Recovery bounds the fresh page: functional, empty, and quiet.
    mailbox.recover();
    assert!(mailbox.push_value(&serde_json::json!({"type": "rpc-response", "id": "after"})));
    let (batch, failed, wake) = mailbox.drain();
    assert!(!failed);
    assert!(!wake);
    assert_eq!(batch.len(), 1);
}

#[test]
fn reliable_proxy_backlog_fails_loudly_and_recovers() {
    let mut mailbox = ForwardMailbox::default();
    for _ in 0..MAX_RELIABLE_MESSAGES + 1 {
        mailbox.push_value(&serde_json::json!({}));
    }
    let (batch, failed, wake) = mailbox.drain();
    assert!(failed);
    assert!(batch.is_empty());
    assert!(!wake);
    assert!(mailbox.outbox.transport_failed());
    mailbox.recover();
    assert!(mailbox.push_value(&serde_json::json!({})));
    assert_eq!(mailbox.drain().0.len(), 1);
}
