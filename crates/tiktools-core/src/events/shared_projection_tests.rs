use super::*;
#[tokio::test]
async fn shared_subscribers_see_the_same_payload_without_deep_cloning() {
    let bus = EventBus::new(16);
    let mut first = bus.subscribe_domain();
    let mut second = bus.subscribe_domain();
    bus.publish_domain(DomainEvent::AutomationRunsChanged {
        runs: vec![serde_json::json!({"id": "large-run", "text": "x".repeat(10_000)})],
    });
    let left = first.recv_shared().await.unwrap();
    let right = second.recv_shared().await.unwrap();
    assert!(Arc::ptr_eq(&left, &right));
    assert_eq!(left.to_envelope().unwrap(), right.to_envelope().unwrap());
}
#[tokio::test]
async fn feed_sequence_distinguishes_skipped_ui_events_from_normalized_duplicates() {
    let bus = EventBus::new(1);
    let mut subscriber = bus.subscribe_domain();
    for i in 0..10 {
        bus.publish_domain(DomainEvent::LiveEvent {
            event_type: "chat".to_owned(),
            event: serde_json::json!({"n": i}),
        });
        bus.publish_domain(DomainEvent::LiveUiEvent {
            event: serde_json::json!({"n": i}),
        });
    }
    assert!(matches!(
        subscriber.recv_shared().await,
        Err(DomainRecvError::LossyLagged(_))
    ));
    let mut last = 0;
    let mut missing = 0;
    for _ in 0..4 {
        if matches!(
            subscriber.recv_shared().await.unwrap().as_ref(),
            DomainEvent::LiveUiEvent { .. }
        ) {
            let next = subscriber.ui_sequence();
            missing += next - last - 1;
            last = next;
        }
    }
    assert_eq!(last, 10);
    assert_eq!(missing, 8); // Eight actual UI messages, not 16 domain duplicates.
}
