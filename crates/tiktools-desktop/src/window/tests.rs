//! Window outbox, inbound routing, and forwarder tests.

use super::domain_events::forward_domain_events;
use super::ipc::{
    classify_inbound_message, is_frontend_ready_fast, is_probably_control_rpc,
    webview_request_too_large, InboundRoute,
};
use super::outbox::{
    classify_webview_message, WebviewOutbox, MAX_BATCH_PER_TICK, MAX_PENDING_WEBVIEW_MESSAGES,
    MAX_RELIABLE_BYTES, MAX_RELIABLE_MESSAGES,
};
use std::sync::Arc;
use tiktools_core::AppCore;

fn domain_event(topic: &str, data: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","method":"event","params":{{"topic":"{topic}","data":{data}}}}}"#)
}

#[test]
fn classifier_assigns_plan_classes() {
    use super::outbox::WebviewMessageClass::{Coalescable, Critical, Droppable};
    // Critical: RPC responses, transitions, errors, shutdown, unknowns.
    assert_eq!(
        classify_webview_message(r#"{"type":"rpc-response","response":{}}"#),
        Critical
    );
    assert_eq!(
        classify_webview_message(r#"{"type":"error","message":"x"}"#),
        Critical
    );
    assert_eq!(
        classify_webview_message(&domain_event("live.connected", "{}")),
        Critical
    );
    assert_eq!(
        classify_webview_message(&domain_event("live.disconnected", "{}")),
        Critical
    );
    assert_eq!(
        classify_webview_message(&domain_event("plugin.started", "{}")),
        Critical
    );
    assert_eq!(
        classify_webview_message(&domain_event("points.changed", "{}")),
        Critical
    );
    assert_eq!(
        classify_webview_message(&domain_event("shutdown", "{}")),
        Critical
    );
    // The gap/resync signal is authoritative: it travels the reliable
    // lane, never shed under saturation.
    assert_eq!(
        classify_webview_message(
            r#"{"jsonrpc":"2.0","method":"event.gap","params":{"lost":12,"resync":true}}"#
        ),
        Critical
    );
    assert_eq!(
        classify_webview_message(&domain_event("future.unknown", "{}")),
        Critical
    );
    assert_eq!(classify_webview_message("not json {{{"), Critical);
    // Coalescable: legacy and domain snapshots.
    assert_eq!(
        classify_webview_message(r#"{"type":"room-stats"}"#),
        Coalescable
    );
    assert_eq!(
        classify_webview_message(r#"{"type":"leaderboard"}"#),
        Coalescable
    );
    assert_eq!(
        classify_webview_message(&domain_event("room.stats", "{}")),
        Coalescable
    );
    assert_eq!(
        classify_webview_message(&domain_event("analytics.updated", "{}")),
        Coalescable
    );
    assert_eq!(
        classify_webview_message(&domain_event("gifts.catalog", "{}")),
        Coalescable
    );
    assert_eq!(
        classify_webview_message(&domain_event("plugin.progress", "{}")),
        Coalescable
    );
    // Droppable: high-rate feed plus compat duplicates of domain twins.
    assert_eq!(
        classify_webview_message(&domain_event("live.ui-event", "{}")),
        Droppable
    );
    assert_eq!(
        classify_webview_message(&domain_event("live.event", "{}")),
        Droppable
    );
    assert_eq!(
        classify_webview_message(r#"{"type":"live-event"}"#),
        Droppable
    );
    assert_eq!(
        classify_webview_message(r#"{"type":"points-awarded"}"#),
        Droppable
    );
}

#[test]
fn coalesces_disposable_snapshots() {
    let mut outbox = WebviewOutbox::new();
    outbox.push(r#"{"type":"room-stats","viewers":1,"totalUsers":1,"topViewers":[]}"#.to_owned());
    outbox.push(r#"{"type":"room-stats","viewers":2,"totalUsers":2,"topViewers":[]}"#.to_owned());
    outbox.push(r#"{"type":"live-event","event":{"kind":"chat"}}"#.to_owned());
    assert_eq!(outbox.lossy.len(), 2);
    assert!(outbox.lossy[0].body.contains("\"viewers\":2"));
    assert!(outbox.reliable.is_empty());
}

#[test]
fn domain_snapshots_coalesce_but_feed_and_lifecycle_do_not() {
    let mut outbox = WebviewOutbox::new();
    outbox.push(domain_event("room.stats", r#"{"viewers":1}"#));
    outbox.push(domain_event("room.stats", r#"{"viewers":2}"#));
    outbox.push(domain_event(
        "analytics.updated",
        r#"{"creatorUniqueId":"a"}"#,
    ));
    outbox.push(domain_event(
        "analytics.updated",
        r#"{"creatorUniqueId":"b"}"#,
    ));
    // Same plugin collapses; a different plugin is a separate stream.
    outbox.push(domain_event(
        "plugin.progress",
        r#"{"pluginId":"p1","state":"loading"}"#,
    ));
    outbox.push(domain_event(
        "plugin.progress",
        r#"{"pluginId":"p1","state":"ready"}"#,
    ));
    outbox.push(domain_event(
        "plugin.progress",
        r#"{"pluginId":"p2","state":"ready"}"#,
    ));
    // Feed is never coalesced.
    outbox.push(domain_event("live.ui-event", r#"{"event":{"n":1}}"#));
    outbox.push(domain_event("live.ui-event", r#"{"event":{"n":2}}"#));
    // Lifecycle is reliable: never coalesced, never shed.
    outbox.push(domain_event("plugin.started", r#"{"pluginId":"p1"}"#));
    outbox.push(domain_event("plugin.started", r#"{"pluginId":"p1"}"#));
    assert_eq!(outbox.lossy.len(), 6, "unexpected lossy len");
    assert_eq!(outbox.reliable.len(), 2, "unexpected reliable len");
    assert!(outbox.lossy[0].body.contains(r#""viewers":2"#));
    assert!(outbox.lossy[1].body.contains(r#""creatorUniqueId":"b""#));
    assert!(outbox.lossy[2].body.contains(r#""state":"ready""#));
    assert!(outbox.lossy[3].body.contains(r#""pluginId":"p2""#));
}

#[test]
fn lossy_lane_never_exceeds_bound() {
    let mut outbox = WebviewOutbox::new();
    for index in 0..(MAX_PENDING_WEBVIEW_MESSAGES + 50) {
        outbox.push(format!(r#"{{"type":"live-event","n":{index}}}"#));
    }
    assert_eq!(outbox.lossy.len(), MAX_PENDING_WEBVIEW_MESSAGES);
    // Oldest-first shedding: n:0..50 are gone, n:50 survives.
    assert!(outbox.lossy[0].body.contains(r#""n":50"#));
    assert!(outbox.reliable.is_empty());
}

#[test]
fn rpc_response_survives_saturation() {
    let mut outbox = WebviewOutbox::new();
    for index in 0..MAX_PENDING_WEBVIEW_MESSAGES {
        outbox.push(domain_event(
            "live.ui-event",
            &format!(r#"{{"event":{{"n":{index}}}}}"#),
        ));
    }
    assert_eq!(outbox.lossy.len(), MAX_PENDING_WEBVIEW_MESSAGES);
    outbox.push(r#"{"type":"rpc-response","response":{"id":7,"result":{}}}"#.to_owned());
    // The response lands in the reliable lane, untouched by the full
    // lossy lane, and heads the very next batch.
    assert_eq!(outbox.reliable.len(), 1);
    assert!(outbox.reliable[0].contains(r#""id":7"#));
    let batch = outbox.take_batch();
    assert!(batch[0].contains(r#""id":7"#));
}

#[test]
fn reliable_lane_never_drops_under_massive_burst() {
    let mut outbox = WebviewOutbox::new();
    // 1500 critical messages blow past the old 1024 hard cap that used
    // to shed them; every one must be retained and drain in order.
    for index in 0..1500 {
        outbox.push(format!(
            r#"{{"type":"rpc-response","response":{{"id":{index}}}}}"#
        ));
    }
    assert_eq!(outbox.reliable.len(), 1500);
    assert!(outbox.reliable[0].contains(r#""id":0"#));
    assert!(outbox.reliable[1499].contains(r#""id":1499"#));
    let mut drained = 0;
    while !outbox.is_empty() {
        let batch = outbox.take_batch();
        assert!(!batch.is_empty());
        assert!(batch.len() <= MAX_BATCH_PER_TICK);
        drained += batch.len();
    }
    assert_eq!(drained, 1500);
}

#[test]
fn critical_transitions_survive_lossy_flood() {
    let mut outbox = WebviewOutbox::new();
    for index in 0..(MAX_PENDING_WEBVIEW_MESSAGES + 100) {
        outbox.push(domain_event(
            "live.ui-event",
            &format!(r#"{{"event":{{"n":{index}}}}}"#),
        ));
    }
    for topic in [
        "live.disconnected",
        "live.error",
        "plugin.stopped",
        "points.changed",
        "shutdown",
    ] {
        outbox.push(domain_event(topic, "{}"));
    }
    assert_eq!(outbox.reliable.len(), 5);
    assert_eq!(outbox.lossy.len(), MAX_PENDING_WEBVIEW_MESSAGES);
    let batch = outbox.take_batch();
    for topic in [
        "live.disconnected",
        "live.error",
        "plugin.stopped",
        "points.changed",
        "shutdown",
    ] {
        assert!(
            batch.iter().any(|body| body.contains(topic)),
            "lost {topic} under lossy flood"
        );
    }
}

#[test]
fn live_feed_keeps_fifo_order() {
    let mut outbox = WebviewOutbox::new();
    for index in 0..50 {
        outbox.push(domain_event(
            "live.ui-event",
            &format!(r#"{{"event":{{"n":{index}}}}}"#),
        ));
    }
    for (position, queued) in outbox.lossy.iter().enumerate() {
        assert!(
            queued.body.contains(&format!(r#"{{"n":{position}}}"#)),
            "feed reordered at {position}: {}",
            queued.body
        );
    }
}

#[test]
fn oversized_bursts_drain_in_capped_batches() {
    let mut outbox = WebviewOutbox::new();
    for index in 0..300 {
        outbox.push(domain_event(
            "live.ui-event",
            &format!(r#"{{"event":{{"n":{index}}}}}"#),
        ));
    }
    // Below the lossy cap nothing is shed; 300 messages need three
    // capped batches, each preserving order, until the outbox is empty.
    let first = outbox.take_batch();
    let second = outbox.take_batch();
    let third = outbox.take_batch();
    assert_eq!((first.len(), second.len(), third.len()), (128, 128, 44));
    assert!(first[0].contains(r#"{"n":0}"#));
    assert!(second[0].contains(r#"{"n":128}"#));
    assert!(third[0].contains(r#"{"n":256}"#));
    assert!(outbox.is_empty());
    assert!(outbox.take_batch().is_empty());
}

#[test]
fn oversized_webview_requests_are_rejected_at_the_boundary() {
    let limit = tiktools_control_api::MAX_REQUEST_BYTES;
    assert!(!webview_request_too_large(&"x".repeat(limit)));
    assert!(webview_request_too_large(&"x".repeat(limit + 1)));
    assert!(is_probably_control_rpc(r#"{"method":"system.ping"}"#));
    assert!(!is_probably_control_rpc(r#"{"type":"disconnect"}"#));
}

fn forwarder_test_core(tag: &str) -> (Arc<AppCore>, std::path::PathBuf) {
    struct Emitter;
    impl tiktools_core::HostEmitter for Emitter {
        fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
    }
    let home = std::env::temp_dir().join(format!(
        "tiktools-webview-gap-{tag}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    std::env::set_var("TIKTOOLS_HOME", &home);
    (Arc::new(AppCore::new(Arc::new(Emitter))), home)
}

#[tokio::test]
async fn lagged_burst_emits_gap_and_forwarder_survives() {
    let (core, home) = forwarder_test_core("reliable");
    let bus = tiktools_core::events::EventBus::new(1);
    let receiver = bus.subscribe_domain();
    // Lag the receiver deterministically: blast a >512-event burst
    // through the capacity-1 reliable lane before the forwarder reads.
    for _ in 0..600 {
        bus.publish_domain(tiktools_core::events::DomainEvent::LiveDisconnected);
    }
    let forwarded = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let task = {
        let forwarded = std::sync::Arc::clone(&forwarded);
        let core = Arc::clone(&core);
        tokio::spawn(forward_domain_events(receiver, core, move |notification| {
            forwarded
                .lock()
                .expect("forwarded lock poisoned")
                .push(notification);
        }))
    };
    // Only terminate once the post-burst message is through, so the
    // Shutdown cannot collapse into the lagged burst itself.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let delivered = forwarded.lock().expect("forwarded lock poisoned").len();
            if delivered >= 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("forwarder must deliver gap plus backlog after lag");
    bus.publish_domain(tiktools_core::events::DomainEvent::Shutdown);
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("forwarder must terminate after Shutdown")
        .expect("forwarder panicked");
    let forwarded = forwarded.lock().expect("forwarded lock poisoned");
    // Reliable lag is explicit, never silent: a gap notification heads
    // the retained backlog, later events still arrive, and Shutdown
    // still terminates the forwarder.
    assert_eq!(
        forwarded.len(),
        3,
        "unexpected forwarded batch: {forwarded:?}"
    );
    let gap: serde_json::Value = serde_json::from_str(&forwarded[0]).expect("gap is JSON");
    assert_eq!(gap["method"], serde_json::json!("event.gap"));
    assert_eq!(gap["params"]["resync"], serde_json::json!(true));
    assert!(
        gap["params"]["lost"].as_u64().unwrap_or_default() > 0,
        "gap must count the lost events: {gap}"
    );
    assert!(forwarded[1].contains("live.disconnected"), "{forwarded:?}");
    assert!(forwarded[2].contains("shutdown"), "{forwarded:?}");
    assert_eq!(core.event_gap_stats().0, 1);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn lossy_burst_forwards_without_gap_signal() {
    let (core, home) = forwarder_test_core("lossy");
    let bus = tiktools_core::events::EventBus::new(1);
    let receiver = bus.subscribe_domain();
    for index in 0..600 {
        bus.publish_domain(tiktools_core::events::DomainEvent::LiveUiEvent {
            event: serde_json::json!({"n": index}),
        });
    }
    let forwarded = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let task = {
        let forwarded = std::sync::Arc::clone(&forwarded);
        let core = Arc::clone(&core);
        tokio::spawn(forward_domain_events(receiver, core, move |notification| {
            forwarded
                .lock()
                .expect("forwarded lock poisoned")
                .push(notification);
        }))
    };
    // Wait for the retained lossy backlog, then terminate cleanly.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let delivered = forwarded.lock().expect("forwarded lock poisoned").len();
            if delivered >= 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("forwarder must deliver the retained backlog");
    bus.publish_domain(tiktools_core::events::DomainEvent::Shutdown);
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("forwarder must terminate after Shutdown")
        .expect("forwarder panicked");
    let forwarded = forwarded.lock().expect("forwarded lock poisoned");
    assert!(
        !forwarded.iter().any(|line| line.contains("event.gap")),
        "a lossy flood must never cause a gap signal: {forwarded:?}"
    );
    assert!(
        forwarded
            .last()
            .is_some_and(|line| line.contains("shutdown")),
        "shutdown still terminates after a lossy flood: {forwarded:?}"
    );
    assert_eq!(core.event_gap_stats().0, 0);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn reliable_lane_trips_safety_policy_by_count() {
    let mut outbox = WebviewOutbox::new();
    for index in 0..MAX_RELIABLE_MESSAGES {
        outbox.push(format!(
            r#"{{"type":"rpc-response","response":{{"id":{index}}}}}"#
        ));
    }
    assert!(!outbox.transport_failed());
    assert!(!outbox.take_transport_failure());
    // The 4097th message trips the policy: the failure edge fires
    // exactly once and every queued message is still retained.
    outbox.push(r#"{"type":"rpc-response","response":{"id":"trip"}}"#.to_owned());
    assert!(outbox.transport_failed());
    assert!(outbox.take_transport_failure());
    assert!(!outbox.take_transport_failure());
    assert_eq!(outbox.reliable.len(), MAX_RELIABLE_MESSAGES + 1);
    assert!(outbox.reliable_bytes() > 0);
    // While failed, new reliable messages are counted, never queued:
    // the queue cannot grow without bound for a dead page.
    outbox.push(r#"{"type":"rpc-response","response":{"id":"refused"}}"#.to_owned());
    assert_eq!(outbox.reliable.len(), MAX_RELIABLE_MESSAGES + 1);
    assert_eq!(outbox.dropped_while_failed, 1);
    // Recovery rearms the transport: new messages queue again.
    outbox.clear_for_reload();
    assert!(outbox.is_empty());
    assert_eq!(outbox.reliable_bytes(), 0);
    outbox.recover_transport();
    assert!(!outbox.transport_failed());
    outbox.push(r#"{"type":"rpc-response","response":{"id":"recovered"}}"#.to_owned());
    assert_eq!(outbox.reliable.len(), 1);
}

#[test]
fn reliable_lane_trips_safety_policy_by_bytes() {
    let mut outbox = WebviewOutbox::new();
    // 17 one-megabyte critical messages exceed the 16 MiB policy with
    // far fewer than 4096 messages.
    let big = format!(
        r#"{{"type":"rpc-response","response":{{"blob":"{}"}}}}"#,
        "x".repeat(1024 * 1024)
    );
    for _ in 0..17 {
        outbox.push(big.clone());
    }
    assert!(outbox.transport_failed());
    assert!(outbox.reliable_bytes() > MAX_RELIABLE_BYTES);
    assert!(outbox.take_transport_failure());
}

#[test]
fn take_batch_accounts_reliable_bytes() {
    let mut outbox = WebviewOutbox::new();
    outbox.push(r#"{"type":"rpc-response","response":{"id":1}}"#.to_owned());
    outbox.push(r#"{"type":"rpc-response","response":{"id":2}}"#.to_owned());
    let held = outbox.reliable_bytes();
    assert!(held > 0);
    let batch = outbox.take_batch();
    assert_eq!(batch.len(), 2);
    assert_eq!(outbox.reliable_bytes(), 0);
    assert!(outbox.is_empty());
}

#[test]
fn inbound_routing_parses_once_and_correlates_errors() {
    // Valid shapes route by value, never by re-scanning text.
    assert!(matches!(
        classify_inbound_message(r#"{"jsonrpc":"2.0","id":1,"method":"system.ping"}"#),
        InboundRoute::Control(_)
    ));
    assert!(matches!(
        classify_inbound_message(r#"{"type":"disconnect"}"#),
        InboundRoute::Legacy(_)
    ));
    // The exact handshake and its whitespace variants both complete
    // startup without touching either router.
    assert_eq!(
        classify_inbound_message(r#"{"type":"frontend-ready"}"#),
        InboundRoute::FrontendReady
    );
    assert_eq!(
        classify_inbound_message("  { \"type\" : \"frontend-ready\" }  "),
        InboundRoute::FrontendReady
    );
    // Malformed control-shaped input answers with a correlated RPC
    // error instead of falling into the legacy router.
    assert!(matches!(
        classify_inbound_message(r#"{"id":7,"method":"system.ping","params":{broken"#),
        InboundRoute::MalformedControl(_)
    ));
    assert_eq!(
        tiktools_control_api::RpcId::extract_from_prefix(
            r#"{"id":7,"method":"system.ping","params":{broken"#
        ),
        tiktools_control_api::RpcId::Number(7)
    );
    // Malformed legacy-shaped input never enters the ControlApi.
    assert_eq!(
        classify_inbound_message(r#"{"type":"disconnect",oops"#),
        InboundRoute::MalformedLegacy
    );
    assert_eq!(
        classify_inbound_message("not json at all"),
        InboundRoute::MalformedLegacy
    );
}

#[test]
fn frontend_ready_fast_path_rejects_large_json_without_parsing() {
    // Exact handshake (plus padding) matches on the Winit thread.
    assert!(is_frontend_ready_fast(r#"{"type":"frontend-ready"}"#));
    assert!(is_frontend_ready_fast("  {\"type\":\"frontend-ready\"}\n"));
    assert!(!is_frontend_ready_fast(r#"{"type":"disconnect"}"#));
    assert!(!is_frontend_ready_fast(r#"{"method":"system.ping"}"#));
    // The length gate alone rejects large payloads: a 1 MiB valid
    // JSON document never reaches a parser on the UI thread.
    let large = format!(
        r#"{{"method":"system.ping","params":{{"blob":"{}"}}}}"#,
        "y".repeat(1024 * 1024)
    );
    assert!(!is_frontend_ready_fast(&large));
    // Even a large payload smuggling the handshake string is rejected
    // here and classified off-thread instead.
    let smuggled = format!(
        r#"{{"note":"frontend-ready","blob":"{}"}}"#,
        "z".repeat(1024)
    );
    assert!(!is_frontend_ready_fast(&smuggled));
    assert!(matches!(
        classify_inbound_message(&smuggled),
        InboundRoute::Legacy(_)
    ));
}
