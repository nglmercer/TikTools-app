use super::*;
fn publish_creator(bus: &EventBus, creator: &str, text: &str) {
    bus.publish_live_event_from_creator(serde_json::json!({"creatorId":creator,"kind":"chat","author":"viewer1","text":text,"roomId":"private-room","msgId":"private-msg"}), Some(creator));
}

#[test]
fn creator_feed_keeps_independent_sequences_and_projects_only_allowed_content() {
    let bus = EventBus::new(2);
    bus.publish_domain(DomainEvent::LiveUiEvent { event: serde_json::json!({"creatorId":"alice","kind":"chat","author":"spoof","text":"untrusted"}) });
    assert!(bus
        .creator_feed("alice", None, 0, 64, 2048)
        .unwrap()
        .events
        .is_empty());
    publish_creator(&bus, "alice", "alice one");
    publish_creator(&bus, "bob", "bob secret");
    publish_creator(&bus, "bob", "bob another");
    publish_creator(&bus, "alice", "alice two");
    let alice = bus.creator_feed("alice", None, 0, 64, 2048).unwrap();
    let bob = bus.creator_feed("bob", None, 0, 64, 2048).unwrap();
    assert_eq!(
        alice
            .events
            .iter()
            .map(|e| e.cursor.as_str())
            .collect::<Vec<_>>(),
        ["1", "2"]
    );
    assert_eq!(
        bob.events
            .iter()
            .map(|e| e.cursor.as_str())
            .collect::<Vec<_>>(),
        ["1", "2"]
    );
    assert_ne!(alice.stream_id, bob.stream_id);
    let encoded = serde_json::to_string(&alice).unwrap();
    assert!(
        !encoded.contains("bob")
            && !encoded.contains("private-room")
            && !encoded.contains("private-msg")
            && !encoded.contains("feedCursor")
    );
    publish_creator(&bus, "bob", "bob still private");
    let tail = bus
        .creator_feed("alice", alice.stream_id.as_deref(), 2, 64, 2048)
        .unwrap();
    assert_eq!(tail.latest_cursor, "2");
    assert!(tail.events.is_empty() && !tail.gap && !tail.truncated);
    let page = bus
        .creator_feed("alice", alice.stream_id.as_deref(), 0, 1, 2048)
        .unwrap();
    assert_eq!(page.next_cursor, "1");
    assert!(page.truncated);
    let next = bus
        .creator_feed("alice", alice.stream_id.as_deref(), 1, 1, 2048)
        .unwrap();
    assert_eq!(next.events[0].text, "alice two");
    assert!(!next.truncated);
}

#[test]
fn creator_feed_reports_eviction_and_changed_stream_without_leaking_other_history() {
    let bus = EventBus::new(2);
    publish_creator(&bus, "alice", "old");
    let previous = bus.creator_feed("alice", None, 0, 64, 2048).unwrap();
    for n in 0..512 {
        publish_creator(&bus, "bob", &format!("secret {n}"));
    }
    let retained = bus
        .creator_feed("alice", previous.stream_id.as_deref(), 0, 64, 2048)
        .unwrap();
    assert!(!retained.gap && !retained.truncated);
    assert_eq!(retained.events[0].text, "old");
    for n in 0..64 {
        publish_creator(&bus, "alice", &format!("own {n}"));
    }
    let expired = bus
        .creator_feed("alice", previous.stream_id.as_deref(), 0, 64, 2048)
        .unwrap();
    assert!(expired.gap);
    assert_eq!(expired.oldest_cursor, "2");
    for n in 0..65 {
        publish_creator(&bus, &format!("creator-{n}"), "other");
    }
    publish_creator(&bus, "alice", "new");
    let recovered = bus
        .creator_feed("alice", previous.stream_id.as_deref(), 1, 64, 2048)
        .unwrap();
    assert!(recovered.gap);
    assert_ne!(recovered.stream_id, previous.stream_id);
    assert_eq!(recovered.next_cursor, "1");
    assert_eq!(recovered.events.len(), 1);
    assert_eq!(recovered.events[0].text, "new");
}

#[cfg(feature = "native-tiktok")]
#[test]
fn creator_feed_loss_waits_for_trusted_generation_and_remains_scoped() {
    let bus = EventBus::new(2);
    // Data can be shed before the worker processes the Connected item.
    bus.record_live_ingress_gap(10);
    publish_creator(&bus, "bob", "private");
    let bob = bus.creator_feed("bob", None, 0, 64, 2048).unwrap();
    assert!(!bob.gap);
    assert!(!bus.creator_feed("alice", None, 0, 64, 2048).unwrap().gap);
    bus.bind_live_generation(11, "bob");
    assert!(!bus.creator_feed("bob", None, 0, 64, 2048).unwrap().gap);
    bus.bind_live_generation(10, "alice");
    let alice = bus.creator_feed("alice", None, 0, 64, 2048).unwrap();
    assert!(alice.gap);
    assert!(alice.stream_id.is_some());
    assert!(alice.events.is_empty());
    publish_creator(&bus, "alice", "after loss");
    let tail = bus
        .creator_feed("alice", alice.stream_id.as_deref(), 0, 64, 2048)
        .unwrap();
    assert!(tail.gap);
    assert_eq!(tail.events[0].text, "after loss");
    let next = bus
        .creator_feed("alice", alice.stream_id.as_deref(), 1, 64, 2048)
        .unwrap();
    assert!(
        next.gap,
        "loss remains visible to late and multiple readers"
    );
    let bob_after = bus
        .creator_feed("bob", bob.stream_id.as_deref(), 0, 64, 2048)
        .unwrap();
    assert!(!bob_after.gap);
    assert_eq!(bob_after.events[0].text, "private");
}

#[cfg(feature = "native-tiktok")]
#[test]
fn creator_feed_ambiguous_or_unbounded_generation_state_fails_closed() {
    let bus = EventBus::new(2);
    bus.bind_live_generation(1, "alice");
    bus.bind_live_generation(1, "bob");
    assert!(bus.creator_feed("alice", None, 0, 64, 2048).is_err());
    assert!(bus.creator_feed("bob", None, 0, 64, 2048).is_err());
    let bus = EventBus::new(2);
    for generation in 0..128 {
        bus.record_live_ingress_gap(generation);
    }
    assert!(bus.creator_feed("alice", None, 0, 64, 2048).is_ok());
    bus.record_live_ingress_gap(128);
    assert!(bus.creator_feed("alice", None, 0, 64, 2048).is_err());
}

#[cfg(feature = "native-tiktok")]
#[test]
fn unverified_generation_never_binds_pending_or_future_loss() {
    let bus = EventBus::new(2);
    bus.record_live_ingress_gap(7);
    bus.reject_unverified_live_generation(7);
    bus.bind_live_generation(7, "alice");
    bus.record_live_ingress_gap(7);
    assert!(!bus.is_verified_live_generation(7, "alice"));
    let alice = bus.creator_feed("alice", None, 0, 64, 2048).unwrap();
    assert!(!alice.gap);
    assert!(alice.events.is_empty());
    bus.bind_live_generation(8, "bob");
    assert!(bus.is_verified_live_generation(8, "bob"));
    assert!(!bus.creator_feed("bob", None, 0, 64, 2048).unwrap().gap);
}

#[test]
fn creator_feed_entropy_failure_is_explicit_and_does_not_disable_local_history() {
    let bus = EventBus::new(2);
    {
        let mut history = bus.feed_history.lock().unwrap();
        assert!(history.creator_stream("alice", 1, |_| false).is_none());
    }
    publish_creator(&bus, "alice", "still in local history");
    assert!(bus.creator_feed("alice", None, 0, 64, 2048).is_err());
    assert_eq!(bus.feed_backfill(0, 64).events.len(), 1);
    #[cfg(feature = "native-tiktok")]
    {
        // Entropy failure only disables the feed, not an independently
        // verified native creator or Owner automation context.
        bus.bind_live_generation(3, "alice");
        assert!(bus.is_verified_live_generation(3, "alice"));
        bus.bind_live_generation(3, "bob");
        assert!(!bus.is_verified_live_generation(3, "alice"));
        assert!(!bus.is_verified_live_generation(3, "bob"));
    }
}

#[test]
fn bounded_recovery_is_ordered_and_reports_eviction_and_restart() {
    let bus = EventBus::new(2);
    for n in 0..520 {
        bus.publish_domain(DomainEvent::LiveUiEvent {
            event: serde_json::json!({"text":format!("hello 👋 {n}")}),
        });
    }
    let page = bus.feed_backfill(1, 2);
    assert_eq!(page.depth, 512);
    assert_eq!(page.oldest_cursor, 9);
    assert!(page.gap && page.truncated);
    assert_eq!(page.events[0].cursor, 9);
    assert_eq!(page.events[0].event["text"], "hello 👋 8");
    let tail = bus.feed_backfill(519, 512);
    assert_eq!(tail.events.len(), 1);
    assert!(!tail.truncated && !tail.gap);
    assert!(EventBus::new(1).feed_backfill(520, 512).gap);
}
#[test]
fn journal_cursor_survives_typed_ipc_reserialization() {
    let bus = EventBus::new(2);
    let mut receiver = bus.subscribe_domain();
    bus.publish_domain(DomainEvent::LiveUiEvent {
        event: serde_json::json!({"kind": "gift", "msgId": "0"}),
    });
    let event = receiver.try_recv().unwrap();
    let wire = serde_json::to_value(&event).unwrap();
    let decoded: DomainEvent = serde_json::from_value(wire).unwrap();
    let wire = serde_json::to_value(decoded).unwrap();
    assert_eq!(wire["data"]["event"]["feedCursor"], 1);
    assert_eq!(bus.feed_backfill(0, 1).events[0].event["feedCursor"], 1);
}
#[test]
fn recovery_does_not_republish_or_execute_events() {
    let bus = EventBus::new(2);
    bus.publish_domain(DomainEvent::LiveUiEvent {
        event: serde_json::json!({"text":"hello"}),
    });
    let receiver = bus.subscribe_domain();
    let before = receiver.ui_sequence();
    for _ in 0..5 {
        assert_eq!(bus.feed_backfill(0, 0).events.len(), 1);
    }
    assert_eq!(receiver.ui_sequence(), before);
}
