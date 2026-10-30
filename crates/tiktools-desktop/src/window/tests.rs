//! Window outbox, inbound routing, and forwarder tests.

use super::domain_events::forward_domain_events;
use super::ipc::{
    classify_inbound_message, is_devtools_command_value, is_frontend_ready_fast,
    is_open_devtools_fast, is_probably_control_rpc, webview_request_too_large, InboundRoute,
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
        Critical
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
    assert_eq!(outbox.lossy.len(), 4, "unexpected lossy len");
    assert_eq!(outbox.reliable.len(), 5, "unexpected reliable len");
    assert!(outbox.lossy[0].body.contains(r#""viewers":2"#));
    assert!(outbox.lossy[1].body.contains(r#""creatorUniqueId":"b""#));
    assert!(outbox.reliable[0].contains(r#""state":"loading""#));
    assert!(outbox.reliable[1].contains(r#""state":"ready""#));
    assert!(outbox.reliable[2].contains(r#""pluginId":"p2""#));
}

#[test]
fn ranking_snapshots_coalesce_per_ranking_without_losing_other_rankings() {
    let mut outbox = WebviewOutbox::new();
    outbox.push(domain_event(
        "ranking.updated",
        r#"{"snapshot":{"rankingId":"a","revision":1}}"#,
    ));
    outbox.push(domain_event(
        "ranking.updated",
        r#"{"snapshot":{"rankingId":"b","revision":1}}"#,
    ));
    outbox.push(domain_event(
        "ranking.updated",
        r#"{"snapshot":{"rankingId":"a","revision":2}}"#,
    ));
    assert!(outbox.reliable.is_empty());
    assert_eq!(outbox.lossy.len(), 2);
    assert!(outbox
        .lossy
        .iter()
        .any(|item| item.body.contains(r#""rankingId":"a","revision":2"#)));
    assert!(outbox
        .lossy
        .iter()
        .any(|item| item.body.contains(r#""rankingId":"b","revision":1"#)));
}

#[test]
fn ranking_coalescing_compares_revisions_only_within_an_instance() {
    let mut outbox = WebviewOutbox::new();
    outbox.push(domain_event("ranking.updated", r#"{"snapshot":{"rankingId":"a","instanceId":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":9}}"#));
    outbox.push(domain_event("ranking.updated", r#"{"snapshot":{"rankingId":"a","instanceId":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":3}}"#));
    assert!(outbox.lossy[0].contains(r#""revision":9"#));
    outbox.push(domain_event("ranking.updated", r#"{"snapshot":{"rankingId":"a","instanceId":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","revision":1}}"#));
    assert_eq!(outbox.lossy.len(), 1);
    assert!(outbox.lossy[0].contains(r#""revision":1"#));
}

#[test]
fn committed_reveals_coalesce_per_competition_but_lifecycle_remains_reliable() {
    let mut outbox = WebviewOutbox::new();
    outbox.push(domain_event(
        "competition.result",
        r#"{"result":{"competitionId":"a","generation":1}}"#,
    ));
    outbox.push(domain_event(
        "competition.result",
        r#"{"result":{"competitionId":"b","generation":1}}"#,
    ));
    outbox.push(domain_event(
        "competition.result",
        r#"{"result":{"competitionId":"a","generation":2}}"#,
    ));
    outbox.push(domain_event(
        "competition.result",
        r#"{"result":{"competitionId":"a","generation":1}}"#,
    ));
    outbox.push(domain_event(
        "competition.finished",
        r#"{"competitionId":"a"}"#,
    ));
    assert_eq!(outbox.lossy.len(), 2);
    assert_eq!(outbox.reliable.len(), 1);
    assert!(outbox
        .lossy
        .iter()
        .any(|item| item.body.contains(r#""competitionId":"a","generation":2"#)));
    assert!(outbox
        .lossy
        .iter()
        .any(|item| item.body.contains(r#""competitionId":"b","generation":1"#)));
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
        tokio::spawn(forward_domain_events(
            receiver,
            core,
            test_projection(),
            move |notification, _generation| {
                forwarded
                    .lock()
                    .expect("forwarded lock poisoned")
                    .push(notification.to_string());
            },
        ))
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
        tokio::spawn(forward_domain_events(
            receiver,
            core,
            test_projection(),
            move |notification, _generation| {
                forwarded
                    .lock()
                    .expect("forwarded lock poisoned")
                    .push(notification.to_string());
            },
        ))
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
    // exactly once without retaining an item beyond the hard count bound.
    outbox.push(r#"{"type":"rpc-response","response":{"id":"trip"}}"#.to_owned());
    assert!(outbox.transport_failed());
    assert!(outbox.take_transport_failure());
    assert!(!outbox.take_transport_failure());
    assert_eq!(outbox.reliable.len(), MAX_RELIABLE_MESSAGES);
    assert!(outbox.reliable_bytes() > 0);
    // While failed, new reliable messages are counted, never queued:
    // the queue cannot grow without bound for a dead page.
    outbox.push(r#"{"type":"rpc-response","response":{"id":"refused"}}"#.to_owned());
    assert_eq!(outbox.reliable.len(), MAX_RELIABLE_MESSAGES);
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
        "x".repeat(1024 * 1024 - 128)
    );
    for _ in 0..17 {
        outbox.push(big.clone());
    }
    assert!(outbox.transport_failed());
    assert!(outbox.reliable_bytes() <= MAX_RELIABLE_BYTES);
    assert!(!outbox.reliable.is_empty());
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

#[test]
fn devtools_shortcut_messages_are_recognized_without_parsing() {
    // Exact devtools shortcut (plus padding) matches for routing.
    assert!(is_open_devtools_fast(r#"{"type":"open-devtools"}"#));
    assert!(is_open_devtools_fast("  {\"type\":\"open-devtools\"}\n"));
    assert!(!is_open_devtools_fast(r#"{"type":"frontend-ready"}"#));
    assert!(!is_open_devtools_fast(r#"{"type":"disconnect"}"#));
    assert!(!is_open_devtools_fast(r#"{"method":"system.ping"}"#));
    // The length gate rejects large payloads without parsing.
    let large = format!(r#"{{"type":"open-devtools","pad":"{}"}}"#, "y".repeat(1024));
    assert!(!is_open_devtools_fast(&large));
    // Value-level recognition is exact on the `type` field only.
    assert!(is_devtools_command_value(
        &serde_json::json!({"type": "open-devtools"})
    ));
    assert!(!is_devtools_command_value(
        &serde_json::json!({"type": "open-devtools", "method": "system.ping"})
    ));
    assert!(!is_devtools_command_value(
        &serde_json::json!({"type": "frontend-ready"})
    ));
    // The shortcut still classifies as legacy (never control-plane), so
    // the Tokio dispatcher routes it without touching the ControlApi.
    assert!(matches!(
        classify_inbound_message(r#"{"type":"open-devtools"}"#),
        InboundRoute::Legacy(_)
    ));
}

#[test]
fn batch_acknowledgments_are_small_and_strictly_correlated() {
    use super::ipc::parse_batch_ack;
    assert_eq!(parse_batch_ack("tiktools-batch-ack:7:42"), Some((7, 42)));
    for malformed in [
        "tiktools-batch-ack:7:42:3",
        "tiktools-batch-ack:x:1",
        "tiktools-batch-ack:1:",
        "{\"type\":\"host-batch-ack\"}",
    ] {
        assert_eq!(parse_batch_ack(malformed), None);
    }
    assert_eq!(
        parse_batch_ack(&format!("tiktools-batch-ack:1:{}", "9".repeat(128))),
        None
    );
}

#[tokio::test]
async fn main_page_filters_normalized_live_events_without_changing_bus() {
    let (core, home) = forwarder_test_core("projection");
    let bus = tiktools_core::events::EventBus::new(16);
    let receiver = bus.subscribe_domain();
    let mut independent = bus.subscribe_domain();
    bus.publish_domain(tiktools_core::events::DomainEvent::LiveEvent {
        event_type: "chat".to_owned(),
        event: serde_json::json!({"text": "hello"}),
    });
    bus.publish_domain(tiktools_core::events::DomainEvent::LiveUiEvent {
        event: serde_json::json!({"kind": "chat"}),
    });
    let forwarded = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = Arc::clone(&forwarded);
    let task = tokio::spawn(forward_domain_events(
        receiver,
        core,
        test_projection(),
        move |notification, _generation| {
            captured.lock().unwrap().push(notification.clone());
        },
    ));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while forwarded.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    // The independent subscriber still observes both source contracts.
    let first = independent.recv().await.unwrap();
    assert!(matches!(
        first,
        tiktools_core::events::DomainEvent::LiveEvent { .. }
    ));
    bus.publish_domain(tiktools_core::events::DomainEvent::Shutdown);
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    let messages = forwarded.lock().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["params"]["topic"], "live.ui-event");
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn hidden_feed_events_are_journaled_and_backfillable_by_cursor() {
    let (core, home) = forwarder_test_core("feed-journal");
    let bus = tiktools_core::events::EventBus::new(16);
    let receiver = bus.subscribe_domain();
    let projection = Arc::new(super::projection::MainPageProjection::default());
    // Feed hidden behind a visible page: events are suspended, never sent,
    // but must still be retained for a bounded cursor backfill.
    projection.update(0, 1, false, false, false, true);
    let task = tokio::spawn(forward_domain_events(
        receiver,
        core.clone(),
        projection.clone(),
        move |_notification, _generation| {},
    ));
    for index in 1..=5 {
        bus.publish_domain(tiktools_core::events::DomainEvent::LiveUiEvent {
            event: serde_json::json!({"kind": "chat", "text": index.to_string()}),
        });
    }
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while projection
            .feed_suspended
            .load(std::sync::atomic::Ordering::Relaxed)
            < 5
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    bus.publish_domain(tiktools_core::events::DomainEvent::Shutdown);
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    let backfill = projection.feed_backfill(0, super::projection::FEED_JOURNAL_CAPACITY);
    let events = backfill["events"].as_array().unwrap();
    assert_eq!(events.len(), 5);
    assert_eq!(events[0]["event"]["text"], "1");
    assert_eq!(events[4]["event"]["text"], "5");
    assert_eq!(backfill["gap"], false);
    assert_eq!(backfill["truncated"], false);
    let third = events[2]["cursor"].as_u64().unwrap();
    let tail = projection.feed_backfill(third, super::projection::FEED_JOURNAL_CAPACITY);
    assert_eq!(tail["events"].as_array().unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn visible_feed_forward_carries_the_retained_bus_cursor() {
    let (core, home) = forwarder_test_core("feed-cursor");
    let bus = tiktools_core::events::EventBus::new(16);
    let receiver = bus.subscribe_domain();
    let forwarded = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = Arc::clone(&forwarded);
    let task = tokio::spawn(forward_domain_events(
        receiver,
        core.clone(),
        test_projection(),
        move |notification, _generation| {
            captured.lock().unwrap().push(notification.clone());
        },
    ));
    bus.publish_domain(tiktools_core::events::DomainEvent::LiveUiEvent {
        event: serde_json::json!({"kind": "chat", "text": "a"}),
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while forwarded.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    bus.publish_domain(tiktools_core::events::DomainEvent::Shutdown);
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    let messages = forwarded.lock().unwrap();
    let feed = messages
        .iter()
        .find(|message| message["params"]["topic"] == "live.ui-event")
        .expect("feed event must be forwarded");
    assert_eq!(feed["params"]["data"]["cursor"], 1);
    assert_eq!(feed["params"]["data"]["sequence"], 1);
    assert_eq!(feed["params"]["data"]["event"]["text"], "a");
    let _ = std::fs::remove_dir_all(&home);
}

fn test_projection() -> Arc<super::projection::MainPageProjection> {
    let projection = Arc::new(super::projection::MainPageProjection::default());
    projection.update(0, 1, true, false, false, true);
    projection
}

#[tokio::test]
async fn points_burst_projects_exact_top_k_and_preserves_individual_domain_updates() {
    let (core, home) = forwarder_test_core("points-projection");
    let projection = Arc::new(super::projection::MainPageProjection::default());
    projection.update(0, 1, true, false, true, true);
    let receiver = core.events.subscribe_domain();
    let mut independent = core.events.subscribe_domain();
    let raw = tokio::spawn(async move {
        let mut awards = 0;
        loop {
            match independent.recv().await {
                Ok(tiktools_core::events::DomainEvent::PointsChanged { .. }) => awards += 1,
                Ok(tiktools_core::events::DomainEvent::Shutdown) => return awards,
                Ok(_) | Err(tiktools_core::events::DomainRecvError::LossyLagged(_)) => {}
                Err(error) => panic!("individual subscriber gap: {error:?}"),
            }
        }
    });
    let forwarded = Arc::new(std::sync::Mutex::new(Vec::new()));
    let task = {
        let forwarded = forwarded.clone();
        tokio::spawn(forward_domain_events(
            receiver,
            core.clone(),
            projection.clone(),
            move |notification, _| {
                forwarded.lock().unwrap().push(notification.clone());
            },
        ))
    };
    for index in 0..10_000 {
        core.points_adjust("alice", 1.0).unwrap();
        if index % 32 == 0 {
            tokio::task::yield_now().await;
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    core.events
        .publish_domain(tiktools_core::events::DomainEvent::Shutdown);
    task.await.unwrap();
    assert_eq!(raw.await.unwrap(), 10_000);
    {
        let messages = forwarded.lock().unwrap();
        assert!(!messages
            .iter()
            .any(|m| m["params"]["topic"] == "points.changed"));
        let snapshots: Vec<_> = messages
            .iter()
            .filter(|m| m["params"]["topic"] == "ui.points.snapshot")
            .collect();
        assert!(!snapshots.is_empty());
        assert!(
            snapshots.len() < 100,
            "one snapshot per timer, not per award"
        );
        assert_eq!(
            snapshots.last().unwrap()["params"]["data"]["viewers"][0]["points"],
            10_000.0
        );
        assert_eq!(
            projection
                .points_coalesced
                .load(std::sync::atomic::Ordering::Relaxed),
            10_000
        );
    }
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(home);
}

#[tokio::test]
async fn ten_thousand_actions_keep_exact_history_without_per_action_ui_lists() {
    let (core, home) = forwarder_test_core("runs-projection");
    let projection = Arc::new(super::projection::MainPageProjection::default());
    projection.update(0, 1, false, true, false, true);
    let receiver = core.events.subscribe_domain();
    let mut independent = core.events.subscribe_domain();
    let raw = tokio::spawn(async move {
        let mut completions = 0;
        loop {
            match independent.recv_shared().await {
                Ok(event) => match event.as_ref() {
                    tiktools_core::events::DomainEvent::AutomationRunCompleted { .. } => {
                        completions += 1
                    }
                    tiktools_core::events::DomainEvent::Shutdown => return completions,
                    _ => {}
                },
                Err(tiktools_core::events::DomainRecvError::LossyLagged(_)) => {}
                Err(error) => panic!("requested action stream gap: {error:?}"),
            }
        }
    });
    let forwarded = Arc::new(std::sync::Mutex::new(Vec::new()));
    let task = {
        let captured = forwarded.clone();
        tokio::spawn(forward_domain_events(
            receiver,
            core.clone(),
            projection.clone(),
            move |message, _| {
                captured.lock().unwrap().push(message.clone());
            },
        ))
    };
    let action = serde_json::json!({"id": "bench", "name": "Benchmark", "typeId": "core.log", "config": {"message": "complete"}});
    let mut last = serde_json::Value::Null;
    for index in 0..10_000 {
        last = core.test_automation_action(&action, None).await;
        assert_eq!(last["status"], "ok");
        if index % 16 == 0 {
            tokio::task::yield_now().await;
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    core.events
        .publish_domain(tiktools_core::events::DomainEvent::Shutdown);
    task.await.unwrap();
    assert_eq!(raw.await.unwrap(), 10_000);
    {
        let messages = forwarded.lock().unwrap();
        assert!(!messages
            .iter()
            .any(|m| m["params"]["topic"] == "automation.runs.changed"));
        let snapshots: Vec<_> = messages
            .iter()
            .filter(|m| m["params"]["topic"] == "ui.automation.runs")
            .collect();
        assert!(!snapshots.is_empty());
        assert!(snapshots.len() < 1000);
        let runs = &snapshots.last().unwrap()["params"]["data"]["runs"];
        assert_eq!(runs.as_array().unwrap().len(), 60);
        assert_eq!(runs[0]["id"], last["id"]);
        assert_eq!(runs, &serde_json::json!(core.automation_runs()));
    }
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn delayed_page_requests_cannot_execute_in_a_new_generation() {
    use super::ipc::page_generation_matches;
    assert!(page_generation_matches(
        &serde_json::json!({"method":"points.adjust"}),
        2
    ));
    assert!(page_generation_matches(
        &serde_json::json!({"pageGeneration":2}),
        2
    ));
    assert!(!page_generation_matches(
        &serde_json::json!({"pageGeneration":1}),
        2
    ));
    assert!(!page_generation_matches(
        &serde_json::json!({"pageGeneration":"2"}),
        2
    ));
}
