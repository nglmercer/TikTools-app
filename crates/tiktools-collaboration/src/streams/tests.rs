use super::*;
use crate::{
    Authenticated, Capability, ClientKind, CommandReceipt, CommandResource, CommandState,
    CoordinationEvent, Error, Grant, RemoteLimits, ResourceKind,
};
use serde_json::json;
use std::time::Duration;
fn identity() -> Authenticated {
    Authenticated {
        principal_id: "moderator".into(),
        session_id: "device-a".into(),
        grant: Grant {
            capabilities: [Capability::WorkflowsRead, Capability::CommandsRead].into(),
            ..Grant::default()
        },
        grant_revision: "1".into(),
        client_kind: ClientKind::Desktop,
        access_expires_at: i64::MAX,
    }
}
fn version(id: &str, n: u64) -> ResourceVersion {
    ResourceVersion {
        kind: ResourceKind::Workflow,
        scope: "host".into(),
        id: id.into(),
        revision: n.to_string(),
    }
}
fn changed(id: &str, n: u64) -> CoordinationEvent {
    CoordinationEvent::Invalidated {
        event_id: format!("{id}-{n}"),
        resource: version(id, n),
        principal_id: "owner".into(),
        command_id: Some("private-other-command".into()),
        change: "saved".into(),
        committed_at: 123,
    }
}
fn subscribe(hub: &StreamHub, identity: &Authenticated, connection: &str) -> SubscribeResult {
    hub.subscribe(
        identity,
        connection,
        "instance-a",
        SubscribeParams {
            domains: [StreamDomain::Workflows, StreamDomain::Commands].into(),
            feed: false,
            points_limit: None,
        },
        &RemoteLimits::default(),
    )
    .unwrap()
}
fn ready(hub: &StreamHub, identity: &Authenticated, connection: &str) -> SubscribeResult {
    let sub = subscribe(hub, identity, connection);
    hub.capture(identity, connection, &sub.subscription_id, Vec::new(), 1024)
        .unwrap();
    sub
}
fn resume(sub: &SubscribeResult, sequence: &str) -> ResumeParams {
    ResumeParams {
        subscription_id: sub.subscription_id.clone(),
        host_instance_id: sub.host_instance_id.clone(),
        last_applied_sequence: sequence.into(),
    }
}

#[test]
fn projected_points_baselines_echo_the_bound_and_still_invalidate_outside_the_board() {
    let mut user = identity();
    user.grant.capabilities.insert(Capability::PointsRead);
    for limit in [0, 1, 20] {
        let hub = StreamHub::default();
        let sub = hub
            .subscribe(
                &user,
                "conn",
                "instance-a",
                SubscribeParams {
                    domains: [StreamDomain::Points].into(),
                    feed: false,
                    points_limit: Some(limit),
                },
                &RemoteLimits::default(),
            )
            .unwrap();
        assert_eq!(sub.points_limit, Some(limit));
        let baseline = hub
            .capture(&user, "conn", &sub.subscription_id, Vec::new(), 1024)
            .unwrap();
        assert_eq!(baseline.points_limit, Some(limit));
        assert!(baseline.points.is_empty());
        let mut event = changed("not-on-board", 2);
        if let CoordinationEvent::Invalidated { resource, .. } = &mut event {
            resource.kind = ResourceKind::Viewer;
        }
        hub.publish(&event).unwrap();
        hub.detach("conn");
        assert_eq!(
            hub.resume(
                &user,
                "new",
                "instance-a",
                resume(&sub, "0"),
                &RemoteLimits::default()
            )
            .unwrap()
            .through_sequence,
            "1"
        );
        assert_eq!(
            hub.next(&user, "new").unwrap().unwrap().resource.id,
            "not-on-board"
        );
    }
    for (domains, limit) in [
        ([StreamDomain::Points].into(), 1001),
        ([StreamDomain::Workflows].into(), 0),
    ] {
        assert!(StreamHub::default()
            .subscribe(
                &user,
                "c",
                "i",
                SubscribeParams {
                    domains,
                    feed: false,
                    points_limit: Some(limit)
                },
                &RemoteLimits::default()
            )
            .is_err());
    }
    let hub = StreamHub::default();
    let sub = hub
        .subscribe(
            &user,
            "c",
            "i",
            SubscribeParams {
                domains: [StreamDomain::Points].into(),
                feed: false,
                points_limit: Some(0),
            },
            &RemoteLimits::default(),
        )
        .unwrap();
    let viewer = ResourceVersion {
        kind: ResourceKind::Viewer,
        ..version("alice", 1)
    };
    assert!(matches!(
        hub.capture(
            &user,
            "c",
            &sub.subscription_id,
            vec![(json!({"uniqueId":"alice"}), viewer)],
            1024
        ),
        Err(Error::TooLarge)
    ));
}

#[test]
fn points_only_streams_scope_before_sequences_and_replay_without_workflow_permission() {
    let hub = StreamHub::default();
    let mut user = identity();
    user.grant.capabilities = [Capability::PointsRead, Capability::CommandsRead].into();
    user.grant
        .resources
        .insert(ResourceKind::Viewer, ["alice".into()].into());
    let params = SubscribeParams {
        domains: [StreamDomain::Points, StreamDomain::Commands].into(),
        feed: false,
        points_limit: None,
    };
    let sub = hub
        .subscribe(
            &user,
            "conn",
            "instance-a",
            params.clone(),
            &RemoteLimits::default(),
        )
        .unwrap();
    let v = ResourceVersion {
        kind: ResourceKind::Viewer,
        ..version("alice", 1)
    };
    let baseline = hub
        .capture(
            &user,
            "conn",
            &sub.subscription_id,
            vec![(json!({"uniqueId":"alice","points":2}), v.clone())],
            1024,
        )
        .unwrap();
    assert!(baseline.workflows.is_empty());
    assert_eq!(baseline.points[0]["points"], 2);
    hub.publish(&changed("alice", 2)).unwrap(); // Same ID in another domain.
    for id in ["hidden", "alice"] {
        let mut event = changed(id, 2);
        if let CoordinationEvent::Invalidated {
            resource, event_id, ..
        } = &mut event
        {
            resource.kind = ResourceKind::Viewer;
            *event_id = format!("viewer-{id}-2");
        }
        hub.publish(&event).unwrap();
    }
    let first = hub.next(&user, "conn").unwrap().unwrap();
    assert_eq!(first.sequence, "1");
    assert_eq!(first.topic, "points.changed");
    assert_eq!(first.resource.id, "alice");
    assert!(first.command_id.is_none());
    hub.detach("conn");
    hub.resume(
        &user,
        "reconnected",
        "instance-a",
        resume(&sub, "0"),
        &RemoteLimits::default(),
    )
    .unwrap();
    assert_eq!(hub.next(&user, "reconnected").unwrap().unwrap(), first);
    hub.sent(&user, "reconnected", &first).unwrap();
    assert!(hub.next(&user, "reconnected").unwrap().is_none());
    // Creator restrictions cannot turn global host points into a creator view.
    user.grant
        .resources
        .insert(ResourceKind::Live, ["creator".into()].into());
    assert!(matches!(
        hub.subscribe(
            &user,
            "other",
            "instance-a",
            params,
            &RemoteLimits::default()
        ),
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        hub.next(&user, "reconnected"),
        Err(Error::ResyncRequired)
    ));
}

#[test]
fn mixed_snapshots_preserve_same_id_in_distinct_resource_kinds() {
    let hub = StreamHub::default();
    let mut user = identity();
    user.grant.capabilities.insert(Capability::PointsRead);
    let sub = hub
        .subscribe(
            &user,
            "conn",
            "instance-a",
            SubscribeParams {
                domains: [StreamDomain::Workflows, StreamDomain::Points].into(),
                feed: false,
                points_limit: None,
            },
            &RemoteLimits::default(),
        )
        .unwrap();
    let page = hub
        .capture(
            &user,
            "conn",
            &sub.subscription_id,
            vec![
                (json!({"id":"same","graph":{}}), version("same", 3)),
                (
                    json!({"uniqueId":"same","points":5}),
                    ResourceVersion {
                        kind: ResourceKind::Viewer,
                        ..version("same", 9)
                    },
                ),
            ],
            1024,
        )
        .unwrap();
    assert_eq!(page.workflows.len(), 1);
    assert_eq!(page.points.len(), 1);
    assert_eq!(page.resource_versions.len(), 2);
    assert_ne!(
        page.resource_versions[0].kind,
        page.resource_versions[1].kind
    );
}

#[test]
fn filtering_precedes_contiguous_sequences_and_command_outcomes_are_principal_bound() {
    let hub = StreamHub::default();
    let mut user = identity();
    user.grant
        .resources
        .insert(ResourceKind::Workflow, ["welcome".into()].into());
    ready(&hub, &user, "conn");
    hub.publish(&changed("hidden", 1)).unwrap();
    hub.publish(&changed("welcome", 1)).unwrap();
    hub.publish(&changed("welcome", 1)).unwrap();
    let first = hub.next(&user, "conn").unwrap().unwrap();
    assert_eq!(first.sequence, "1");
    assert!(first.command_id.is_none());
    assert_eq!(first.actor.principal_id, "owner");
    hub.sent(&user, "conn", &first).unwrap();
    let mut command = CoordinationEvent::CommandChanged {
        event_id: "command-1".into(),
        resource: version("welcome", 1),
        principal_id: "owner".into(),
        command: CommandReceipt {
            command_id: "result".into(),
            state: CommandState::Succeeded,
            resource: Some(CommandResource {
                kind: ResourceKind::Workflow,
                scope: "host".into(),
                id: "welcome".into(),
            }),
            committed_at: Some(123),
        },
        committed_at: 123,
    };
    hub.publish(&command).unwrap();
    assert!(hub.next(&user, "conn").unwrap().is_none());
    if let CoordinationEvent::CommandChanged { principal_id, .. } = &mut command {
        *principal_id = user.principal_id.clone();
    }
    hub.publish(&command).unwrap();
    let own = hub.next(&user, "conn").unwrap().unwrap();
    assert_eq!(own.sequence, "2");
    assert_eq!(own.topic, "command.changed");
    assert_eq!(own.command_id.as_deref(), Some("result"));
}
#[test]
fn resume_replays_original_ids_and_sequences_after_send_ack_or_credential_rotation() {
    let hub = StreamHub::default();
    let user = identity();
    let sub = ready(&hub, &user, "old");
    hub.publish(&changed("welcome", 1)).unwrap();
    let original = hub.next(&user, "old").unwrap().unwrap();
    hub.sent(&user, "old", &original).unwrap();
    hub.detach("old");
    hub.publish(&changed("welcome", 2)).unwrap();
    let mut rotated = user.clone();
    rotated.access_expires_at -= 1;
    let resumed = hub
        .resume(
            &rotated,
            "new",
            "instance-a",
            resume(&sub, "0"),
            &RemoteLimits::default(),
        )
        .unwrap();
    assert!(resumed.replaying);
    assert_eq!(resumed.through_sequence, "2");
    assert_eq!(hub.next(&rotated, "new").unwrap().unwrap(), original);
    hub.sent(&rotated, "new", &original).unwrap();
    assert_eq!(hub.next(&rotated, "new").unwrap().unwrap().sequence, "2");
    assert!(hub.next(&user, "old").unwrap().is_none());
}
#[test]
fn new_grants_sessions_instances_and_active_connections_cannot_resume_old_state() {
    let hub = StreamHub::default();
    let user = identity();
    let sub = ready(&hub, &user, "old");
    assert!(matches!(
        hub.resume(
            &user,
            "steal",
            "instance-a",
            resume(&sub, "0"),
            &RemoteLimits::default()
        ),
        Err(Error::ResyncRequired)
    ));
    hub.detach("old");
    for field in ["principal", "session", "grant", "instance"] {
        let mut changed = user.clone();
        let mut instance = "instance-a";
        match field {
            "principal" => changed.principal_id = "someone-else".into(),
            "session" => changed.session_id = "device-b".into(),
            "grant" => changed.grant_revision = "2".into(),
            _ => instance = "instance-b",
        }
        assert!(matches!(
            hub.resume(
                &changed,
                "new",
                instance,
                resume(&sub, "0"),
                &RemoteLimits::default()
            ),
            Err(Error::ResyncRequired)
        ));
    }
    hub.resume(
        &user,
        "new",
        "instance-a",
        resume(&sub, "0"),
        &RemoteLimits::default(),
    )
    .unwrap();
    hub.publish(&changed("welcome", 1)).unwrap();
    let mut revoked = user.clone();
    revoked.grant_revision = "2".into();
    assert!(matches!(
        hub.next(&revoked, "new"),
        Err(Error::ResyncRequired)
    ));
    assert!(matches!(hub.next(&user, "new"), Err(Error::ResyncRequired)));
    assert_eq!(
        hub.state.lock().unwrap().streams[&sub.subscription_id].bytes,
        0
    );
}
#[test]
fn immutable_pages_hold_the_watermark_and_wait_for_complete_baseline() {
    let hub = StreamHub::default();
    let user = identity();
    let sub = subscribe(&hub, &user, "conn");
    hub.publish(&changed("a", 1)).unwrap();
    let records = (0..4)
        .map(|i| {
            let id = format!("wf-{i}");
            (json!({"id":id,"body":"x".repeat(600)}), version(&id, 1))
        })
        .collect();
    let first = hub
        .capture(&user, "conn", &sub.subscription_id, records, 1024)
        .unwrap();
    assert_eq!(first.through_sequence, "1");
    assert!(first.next_page_token.is_some());
    hub.publish(&changed("wf-3", 2)).unwrap();
    assert!(hub.next(&user, "conn").unwrap().is_none());
    let mut page = first.clone();
    let mut count = page.workflows.len();
    while let Some(token) = page.next_page_token {
        page = hub
            .page(&user, "conn", &sub.subscription_id, &token)
            .unwrap();
        assert_eq!(page.snapshot_id, first.snapshot_id);
        assert_eq!(page.through_sequence, "1");
        assert!(page.resource_versions.iter().all(|v| v.revision == "1"));
        count += page.workflows.len();
    }
    assert_eq!(count, 4);
    assert_eq!(hub.next(&user, "conn").unwrap().unwrap().sequence, "2");
    assert!(matches!(
        hub.page(&user, "other", &sub.subscription_id, "bogus"),
        Err(Error::ResyncRequired)
    ));
}

#[test]
fn impossible_snapshot_rate_budget_fails_before_returning_a_page() {
    let hub = StreamHub::default();
    let user = identity();
    let limits = RemoteLimits {
        requests_per_second: 1,
        request_burst: 1,
        ..RemoteLimits::default()
    };
    let sub = hub
        .subscribe(
            &user,
            "conn",
            "instance-a",
            SubscribeParams {
                domains: [StreamDomain::Workflows].into(),
                feed: false,
                points_limit: None,
            },
            &limits,
        )
        .unwrap();
    let records = |count| {
        (0..count)
            .map(|i| {
                let id = format!("wf-{i:02}");
                (json!({"id":id,"body":"x".repeat(600)}), version(&id, 1))
            })
            .collect()
    };
    // One record per page. The first page is in the capture response; the
    // other 31 are the optimistic ceiling for a 30s TTL and a full bucket.
    let within = hub
        .capture(&user, "conn", &sub.subscription_id, records(32), 1024)
        .unwrap();
    assert!(within.next_page_token.is_some());
    assert!(matches!(
        hub.capture(&user, "conn", &sub.subscription_id, records(33), 1024),
        Err(Error::SnapshotRateBudget)
    ));
    // A failed replacement discards the previous snapshot and keeps the
    // baseline incomplete, rather than returning a partial projection.
    assert!(matches!(
        hub.page(
            &user,
            "conn",
            &sub.subscription_id,
            within.next_page_token.as_deref().unwrap()
        ),
        Err(Error::ResyncRequired)
    ));
    let state = hub.state.lock().unwrap();
    let stream = &state.streams[&sub.subscription_id];
    assert!(stream.snapshot.is_none());
    assert!(!stream.baseline_complete);
}

#[test]
fn resumed_stream_uses_new_snapshot_rate_limits() {
    let hub = StreamHub::default();
    let user = identity();
    let sub = ready(&hub, &user, "old");
    hub.detach("old");
    let low = RemoteLimits {
        requests_per_second: 1,
        request_burst: 1,
        ..RemoteLimits::default()
    };
    hub.resume(&user, "new", "instance-a", resume(&sub, "0"), &low)
        .unwrap();
    let records = (0..33)
        .map(|i| {
            let id = format!("wf-{i:02}");
            (json!({"id":id,"body":"x".repeat(600)}), version(&id, 1))
        })
        .collect();
    assert!(matches!(
        hub.capture(&user, "new", &sub.subscription_id, records, 1024),
        Err(Error::SnapshotRateBudget)
    ));
}
#[test]
fn unsent_count_and_byte_overflow_break_the_stream_without_blocking_publication() {
    for by_bytes in [false, true] {
        let hub = StreamHub::default();
        let user = identity();
        let sub = ready(&hub, &user, "conn");
        for n in 1..=if by_bytes {
            20
        } else {
            STREAM_EVENTS as u64 + 1
        } {
            let mut event = changed("welcome", n);
            if by_bytes {
                if let CoordinationEvent::Invalidated { change, .. } = &mut event {
                    *change = "x".repeat(256 * 1024);
                }
            }
            hub.publish(&event).unwrap();
        }
        assert!(matches!(
            hub.next(&user, "conn"),
            Err(Error::ResyncRequired)
        ));
        let state = hub.state.lock().unwrap();
        let stream = &state.streams[&sub.subscription_id];
        assert_eq!(stream.bytes, 0);
        assert!(stream.snapshot.is_none());
        assert!(stream.events.is_empty());
    }
}

#[test]
fn stalled_stream_overflow_does_not_block_another_principal_or_its_recovery() {
    let hub = StreamHub::default();
    let stalled = identity();
    let mut active = identity();
    active.principal_id = "active-moderator".into();
    active.session_id = "device-b".into();
    let stalled_sub = ready(&hub, &stalled, "stalled");
    let active_sub = ready(&hub, &active, "active");

    // The stalled client never acknowledges a send. The active client
    // consumes the same committed invalidations without gaps or leaked IDs.
    for n in 1..=STREAM_EVENTS as u64 + 2 {
        hub.publish(&changed("welcome", n)).unwrap();
        let event = hub.next(&active, "active").unwrap().unwrap();
        assert_eq!(event.sequence, n.to_string());
        assert_eq!(event.resource.revision, n.to_string());
        assert!(event.command_id.is_none());
        hub.sent(&active, "active", &event).unwrap();
    }
    assert!(matches!(
        hub.next(&stalled, "stalled"),
        Err(Error::ResyncRequired)
    ));
    {
        let state = hub.state.lock().unwrap();
        let slow = &state.streams[&stalled_sub.subscription_id];
        let fast = &state.streams[&active_sub.subscription_id];
        assert!(slow.broken);
        assert_eq!(slow.bytes, 0);
        assert!(slow.events.is_empty());
        assert!(!fast.broken);
        assert!(fast.bytes <= STREAM_BYTES);
        assert!(fast.events.len() <= STREAM_EVENTS);
    }

    // A fresh snapshot restores only the stalled client; the active stream
    // keeps its sequence and does not need to reconnect.
    let replacement = ready(&hub, &stalled, "stalled");
    assert_ne!(replacement.subscription_id, stalled_sub.subscription_id);
    let next_revision = STREAM_EVENTS as u64 + 3;
    hub.publish(&changed("welcome", next_revision)).unwrap();
    let fast = hub.next(&active, "active").unwrap().unwrap();
    assert_eq!(fast.sequence, next_revision.to_string());
    assert_eq!(fast.subscription_id, active_sub.subscription_id);
    hub.sent(&active, "active", &fast).unwrap();
    let recovered = hub.next(&stalled, "stalled").unwrap().unwrap();
    assert_eq!(recovered.sequence, "1");
    assert_eq!(recovered.resource.revision, next_revision.to_string());
    assert!(recovered.command_id.is_none());
}
#[test]
fn delivered_history_rolls_forward_but_old_reconnect_cursors_require_a_snapshot() {
    let hub = StreamHub::default();
    let user = identity();
    let sub = ready(&hub, &user, "conn");
    for n in 1..=STREAM_EVENTS as u64 + 3 {
        hub.publish(&changed("welcome", n)).unwrap();
        let event = hub.next(&user, "conn").unwrap().unwrap();
        hub.sent(&user, "conn", &event).unwrap();
    }
    hub.detach("conn");
    assert!(matches!(
        hub.resume(
            &user,
            "new",
            "instance-a",
            resume(&sub, "0"),
            &RemoteLimits::default()
        ),
        Err(Error::ResyncRequired)
    ));
    hub.resume(
        &user,
        "new",
        "instance-a",
        resume(&sub, "4097"),
        &RemoteLimits::default(),
    )
    .unwrap();
    assert_eq!(hub.next(&user, "new").unwrap().unwrap().sequence, "4098");
}
#[test]
fn snapshot_expiration_and_detached_retention_are_enforced_without_new_business_events() {
    let hub = StreamHub::default();
    let user = identity();
    let sub = ready(&hub, &user, "conn");
    {
        let mut state = hub.state.lock().unwrap();
        state
            .streams
            .get_mut(&sub.subscription_id)
            .unwrap()
            .snapshot
            .as_mut()
            .unwrap()
            .created -= Duration::from_secs(31);
    }
    hub.prune();
    assert!(hub.state.lock().unwrap().streams[&sub.subscription_id]
        .snapshot
        .is_none());
    hub.detach("conn");
    {
        hub.state
            .lock()
            .unwrap()
            .streams
            .get_mut(&sub.subscription_id)
            .unwrap()
            .detached_at = Some(std::time::Instant::now() - Duration::from_secs(61));
    }
    hub.prune();
    assert!(hub.state.lock().unwrap().streams.is_empty());
}
#[test]
fn age_expiration_of_unsent_events_requires_resync() {
    let hub = StreamHub::default();
    let user = identity();
    ready(&hub, &user, "conn");
    hub.publish(&changed("welcome", 1)).unwrap();
    {
        for s in hub.state.lock().unwrap().streams.values_mut() {
            s.events.front_mut().unwrap().at -= Duration::from_secs(61);
        }
    }
    assert!(matches!(
        hub.next(&user, "conn"),
        Err(Error::ResyncRequired)
    ));
}
#[test]
fn total_stream_slots_and_active_principal_limits_are_bounded() {
    let hub = StreamHub::default();
    let user = identity();
    let limits = RemoteLimits {
        connections: 1,
        connections_per_principal: 1,
        ..RemoteLimits::default()
    };
    let params = SubscribeParams {
        domains: [StreamDomain::Workflows].into(),
        feed: false,
        points_limit: None,
    };
    let old = hub
        .subscribe(&user, "old", "instance-a", params.clone(), &limits)
        .unwrap();
    assert!(matches!(
        hub.subscribe(&user, "extra", "instance-a", params.clone(), &limits),
        Err(Error::Overloaded)
    ));
    hub.capture(&user, "old", &old.subscription_id, Vec::new(), 1024)
        .unwrap();
    hub.detach("old");
    for n in 0..30 {
        let conn = format!("conn-{n}");
        let sub = hub
            .subscribe(&user, &conn, "instance-a", params.clone(), &limits)
            .unwrap();
        hub.capture(&user, &conn, &sub.subscription_id, Vec::new(), 1024)
            .unwrap();
        hub.detach(&conn);
        assert!(hub.state.lock().unwrap().streams.len() <= 2);
    }
    assert!(matches!(
        hub.resume(&user, "new", "instance-a", resume(&old, "0"), &limits),
        Err(Error::ResyncRequired)
    ));
    let hub = StreamHub::default();
    let mut last = None;
    for n in 0..6 {
        let connection = format!("prior-{n}");
        last = Some(ready(&hub, &user, &connection));
        hub.detach(&connection);
    }
    assert_eq!(hub.state.lock().unwrap().streams.len(), 6);
    hub.resume(
        &user,
        "new",
        "instance-a",
        resume(&last.unwrap(), "0"),
        &limits,
    )
    .unwrap();
    assert!(hub.state.lock().unwrap().streams.len() <= 2);
}
#[test]
fn replacement_invalidates_old_ids_tokens_and_incomplete_disconnect_cannot_resume() {
    let hub = StreamHub::default();
    let user = identity();
    let old = subscribe(&hub, &user, "conn");
    let current = subscribe(&hub, &user, "conn");
    assert_ne!(old.subscription_id, current.subscription_id);
    assert!(matches!(
        hub.validate(&user, "conn", &old.subscription_id),
        Err(Error::ResyncRequired)
    ));
    hub.detach("conn");
    assert!(matches!(
        hub.resume(
            &user,
            "new",
            "instance-a",
            resume(&current, "0"),
            &RemoteLimits::default()
        ),
        Err(Error::ResyncRequired)
    ));
}
#[test]
fn unsupported_scopes_missing_capabilities_and_noncanonical_cursors_fail_closed() {
    let hub = StreamHub::default();
    let mut user = identity();
    for params in [
        SubscribeParams {
            domains: [StreamDomain::Live].into(),
            feed: false,
            points_limit: None,
        },
        SubscribeParams {
            domains: [StreamDomain::Workflows].into(),
            feed: true,
            points_limit: None,
        },
    ] {
        assert!(matches!(
            hub.subscribe(
                &user,
                "conn",
                "instance-a",
                params,
                &RemoteLimits::default()
            ),
            Err(Error::Unsupported)
        ));
    }
    user.grant.capabilities.clear();
    assert!(matches!(
        hub.subscribe(
            &user,
            "conn",
            "instance-a",
            SubscribeParams {
                domains: [StreamDomain::Workflows].into(),
                feed: false,
                points_limit: None,
            },
            &RemoteLimits::default()
        ),
        Err(Error::Forbidden)
    ));
    for value in ["", "01", "+1", "-1", "9223372036854775808"] {
        assert!(stream_sequence(value).is_err());
    }
    assert_eq!(
        stream_sequence("9223372036854775807").unwrap(),
        i64::MAX as u64
    );
}
#[test]
fn snapshot_limits_and_resource_checks_precede_retaining_a_partial_baseline() {
    let hub = StreamHub::default();
    let mut user = identity();
    user.grant
        .resources
        .insert(ResourceKind::Workflow, ["welcome".into()].into());
    let sub = subscribe(&hub, &user, "conn");
    assert!(matches!(
        hub.capture(
            &user,
            "conn",
            &sub.subscription_id,
            vec![(
                json!({"id":"welcome","body":"x".repeat(2000)}),
                version("welcome", 1)
            )],
            1024
        ),
        Err(Error::TooLarge)
    ));
    assert!(matches!(
        hub.capture(
            &user,
            "conn",
            &sub.subscription_id,
            vec![(json!({"id":"hidden"}), version("hidden", 1))],
            1024
        ),
        Err(Error::Forbidden)
    ));
    assert!(hub.state.lock().unwrap().streams[&sub.subscription_id]
        .snapshot
        .is_none());
    // Individually valid records still cannot exceed the total retained
    // budget after accounting for page envelopes and opaque token indexes.
    let record = (
        json!({"id":"welcome","body":"x".repeat(700)}),
        version("welcome", 1),
    );
    assert!(matches!(
        hub.capture(
            &user,
            "conn",
            &sub.subscription_id,
            vec![record; 5000],
            1024
        ),
        Err(Error::TooLarge)
    ));
    assert!(hub.state.lock().unwrap().streams[&sub.subscription_id]
        .snapshot
        .is_none());
}
