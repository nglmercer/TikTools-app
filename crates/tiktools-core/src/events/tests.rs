use super::*;

#[test]
fn presentation_events_reach_ui_without_entering_executable_observer_rings() {
    let bus = EventBus::new(4);
    let mut ui = bus.subscribe_projection();
    let mut plugin = bus.subscribe_plugin_domain();
    bus.publish_presentation_domain(DomainEvent::PointsChanged {
        unique_id: "restricted-viewer".into(),
        delta: 2.0,
        total_points: 3.0,
        level: 1,
    });
    assert!(matches!(
        ui.try_recv(),
        Ok(DomainEvent::PointsChanged { .. })
    ));
    assert!(matches!(
        plugin.inner.try_recv_published(),
        Err(DomainTryRecvError::Empty)
    ));
    bus.publish_domain(DomainEvent::PointsChanged {
        unique_id: "owner-viewer".into(),
        delta: 1.0,
        total_points: 4.0,
        level: 1,
    });
    let received = plugin.inner.try_recv_published().unwrap();
    assert!(received.observer_allowed);
    assert!(
        matches!(received.event.as_ref(), DomainEvent::PointsChanged { unique_id, .. } if unique_id == "owner-viewer")
    );
}

#[test]
fn restricted_bursts_cannot_generate_plugin_gaps_or_evict_owner_notifications() {
    let bus = EventBus::new(2);
    let mut ui = bus.subscribe_projection();
    let mut plugin = bus.subscribe_plugin_domain();
    assert!(bus.has_domain_subscribers());
    bus.publish_domain(DomainEvent::PointsReset { unique_id: None });
    for _ in 0..32 {
        bus.publish_presentation_domain(DomainEvent::AutomationRunCompleted {
            run: serde_json::json!({"observerAllowed":true,"authority":{"kind":"owner"}}),
        });
    }
    assert!(matches!(
        ui.try_recv(),
        Err(DomainTryRecvError::ReliableLagged(_))
    ));
    let received = plugin.inner.try_recv_published().unwrap();
    assert!(matches!(
        received.event.as_ref(),
        DomainEvent::PointsReset { .. }
    ));
    assert!(matches!(
        plugin.inner.try_recv_published(),
        Err(DomainTryRecvError::Empty)
    ));
}

#[test]
fn bounded_ranking_event_fits_plugin_frame_with_maximum_escaped_metadata() {
    use tiktools_aggregation::{Entry, EntryMetadata, Snapshot};
    let escaped = "\u{0001}".repeat(512);
    let image = "\u{0001}".repeat(2048);
    let entries = (0..100)
        .map(|index| Entry {
            key: format!("{index:03}{}", &escaped[..509]),
            rank: index + 1,
            score: 1,
            label: escaped.clone(),
            metadata: EntryMetadata {
                label: None,
                user_id: Some(escaped.clone()),
                unique_id: Some(escaped.clone()),
                nickname: Some(escaped.clone()),
                avatar_url: Some(image.clone()),
                gift_id: Some(escaped.clone()),
                icon_url: Some(image.clone()),
            },
        })
        .collect();
    let snapshot = Snapshot {
        ranking_id: "r".into(),
        schema_version: 1,
        creator_id: "c".into(),
        room_id: "room".into(),
        window: Default::default(),
        revision: 1,
        instance_id: None,
        integrity_status: "unknown".into(),
        rejected: 0,
        dropped: 0,
        limited: 0,
        cooldown_rejected: 0,
        paused: false,
        entries,
        total_score: 100,
        total_participants: 100,
    };
    let snapshot = crate::control::bound_ranking_presentation(snapshot);
    assert_eq!(snapshot.total_score, 100);
    assert_eq!(snapshot.total_participants, 100);
    let envelope = DomainEvent::RankingUpdated { snapshot }
        .to_envelope()
        .unwrap();
    // Include a conservative JSON-RPC wrapper around the event payload.
    let frame = serde_json::to_vec(
        &serde_json::json!({"jsonrpc":"2.0","method":"event","params":envelope}),
    )
    .unwrap();
    assert!(frame.len() < tiktools_plugin_api::MAX_FRAME_BYTES);
    assert!(frame.len() < 1024 * 1024);
}

/// One representative value per variant plus its stable topic and a
/// data-shape assertion. Adding a variant without extending this table
/// fails the count assertion in the conversion test below.
type VariantCase = (DomainEvent, &'static str, fn(&Value));

fn all_variant_cases() -> Vec<VariantCase> {
    vec![
        (
            DomainEvent::PluginInstalled {
                plugin_id: "demo".to_owned(),
            },
            "plugin.installed",
            |data| assert_eq!(data["pluginId"], "demo"),
        ),
        (
            DomainEvent::PluginUninstalled {
                plugin_id: "demo".to_owned(),
            },
            "plugin.uninstalled",
            |data| assert_eq!(data["pluginId"], "demo"),
        ),
        (
            DomainEvent::PluginStarted {
                plugin_id: "demo".to_owned(),
            },
            "plugin.started",
            |data| assert_eq!(data["pluginId"], "demo"),
        ),
        (
            DomainEvent::PluginStopped {
                plugin_id: "demo".to_owned(),
            },
            "plugin.stopped",
            |data| assert_eq!(data["pluginId"], "demo"),
        ),
        (
            DomainEvent::PluginProgress {
                plugin_id: "demo".to_owned(),
                state: "running".to_owned(),
                progress: Some(0.5),
                message: "half".to_owned(),
            },
            "plugin.progress",
            |data| {
                assert_eq!(data["pluginId"], "demo");
                assert_eq!(data["progress"], 0.5);
            },
        ),
        (
            DomainEvent::PluginSettingsChanged {
                plugin_id: "demo".to_owned(),
            },
            "plugin.settings-changed",
            |data| assert_eq!(data["pluginId"], "demo"),
        ),
        (
            DomainEvent::PluginUpdated {
                plugin_id: "demo".to_owned(),
                from_version: Some("1.0.0".to_owned()),
                to_version: "1.1.0".to_owned(),
            },
            "plugin.updated",
            |data| {
                assert_eq!(data["pluginId"], "demo");
                assert_eq!(data["toVersion"], "1.1.0");
            },
        ),
        (
            DomainEvent::PluginJob {
                job: serde_json::json!({"id": "job-1", "status": "running"}),
            },
            "plugin.job",
            |data| assert_eq!(data["job"]["id"], "job-1"),
        ),
        (
            DomainEvent::PluginUpdates {
                report: serde_json::json!({"updates": []}),
            },
            "plugin.updates",
            |data| assert!(data["report"]["updates"].is_array()),
        ),
        (
            DomainEvent::PluginEvent {
                plugin_id: "demo".to_owned(),
                event_type: "hotkey.pressed".to_owned(),
                event: serde_json::json!({"key": "k"}),
            },
            "plugin.event",
            |data| {
                assert_eq!(data["eventType"], "hotkey.pressed");
                assert_eq!(data["event"]["key"], "k");
            },
        ),
        (
            DomainEvent::PluginStatus {
                plugin_id: "demo".to_owned(),
                status: serde_json::json!({"state": "active"}),
            },
            "plugin.status",
            |data| assert_eq!(data["status"]["state"], "active"),
        ),
        (
            DomainEvent::AutomationRunCompleted {
                run: serde_json::json!({"id": "r1"}),
            },
            "automation.run.completed",
            |data| assert_eq!(data["run"]["id"], "r1"),
        ),
        (
            DomainEvent::AutomationRunsChanged {
                runs: vec![serde_json::json!({"id": "r1"})],
            },
            "automation.runs.changed",
            |data| assert_eq!(data["runs"][0]["id"], "r1"),
        ),
        (
            DomainEvent::LiveConnected {
                unique_id: Some("viewer".to_owned()),
                room_id: None,
            },
            "live.connected",
            |data| assert_eq!(data["uniqueId"], "viewer"),
        ),
        (DomainEvent::LiveDisconnected, "live.disconnected", |data| {
            assert!(data.is_null(), "unit variants carry null data")
        }),
        (
            DomainEvent::LiveEvent {
                event_type: "chat".to_owned(),
                event: serde_json::json!({}),
            },
            "live.event",
            |data| assert_eq!(data["eventType"], "chat"),
        ),
        (
            DomainEvent::LiveUiEvent {
                event: serde_json::json!({"kind": "chat"}),
            },
            "live.ui-event",
            |data| assert_eq!(data["event"]["kind"], "chat"),
        ),
        (
            DomainEvent::RoomStats {
                viewers: 7,
                total_users: 9,
                top_viewers: vec![],
            },
            "room.stats",
            |data| {
                assert_eq!(data["viewers"], 7);
                assert_eq!(data["totalUsers"], 9);
            },
        ),
        (
            DomainEvent::GiftsCatalog {
                gifts: vec![serde_json::json!({"id": 1})],
            },
            "gifts.catalog",
            |data| assert_eq!(data["gifts"][0]["id"], 1),
        ),
        (
            DomainEvent::LiveReconnecting {
                attempt: 2,
                delay_ms: 500,
            },
            "live.reconnecting",
            |data| {
                assert_eq!(data["attempt"], 2);
                assert_eq!(data["delayMs"], 500);
            },
        ),
        (
            DomainEvent::LiveError {
                phase: "connect".to_owned(),
                message: "boom".to_owned(),
            },
            "live.error",
            |data| {
                assert_eq!(data["phase"], "connect");
                assert_eq!(data["message"], "boom");
            },
        ),
        (
            DomainEvent::PointsReset { unique_id: None },
            "points.reset",
            |data| assert!(data["uniqueId"].is_null()),
        ),
        (
            DomainEvent::PointsChanged {
                unique_id: "viewer".to_owned(),
                delta: 2.0,
                total_points: 4.0,
                level: 1,
            },
            "points.changed",
            |data| {
                assert_eq!(data["uniqueId"], "viewer");
                assert_eq!(data["level"], 1);
            },
        ),
        (
            DomainEvent::WorkflowChanged {
                kind: "graph".to_owned(),
                id: "w1".to_owned(),
                change: "saved".to_owned(),
            },
            "workflow.changed",
            |data| {
                assert_eq!(data["id"], "w1");
                assert_eq!(data["change"], "saved");
            },
        ),
        (
            DomainEvent::CreatorChanged { unique_id: None },
            "creator.changed",
            |data| assert!(data.get("uniqueId").is_some()),
        ),
        (
            DomainEvent::AnalyticsUpdated {
                creator_unique_id: "creator".to_owned(),
            },
            "analytics.updated",
            |data| assert_eq!(data["creatorUniqueId"], "creator"),
        ),
        (
            DomainEvent::CompetitionResult {
                result: tiktools_competitions::ResultRecord {
                    schema_version: 1,
                    competition_id: "poll".into(),
                    generation: 1,
                    ranking_id: "rank".into(),
                    ranking_revision: 2,
                    finalized_at: 3,
                    selection: tiktools_competitions::Selection {
                        mode: tiktools_competitions::SelectionMode::WeightedRandom,
                        tie_policy: tiktools_competitions::TiePolicy::Stable,
                        eligible: vec![tiktools_competitions::Candidate {
                            key: "a".into(),
                            weight: 3,
                        }],
                        winner: Some("a".into()),
                    },
                }
                .reveal_snapshot()
                .unwrap(),
            },
            "competition.result",
            |data| {
                assert_eq!(data["result"]["rankingId"], "rank");
                assert_eq!(data["result"]["selection"]["winner"], "a");
            },
        ),
        (DomainEvent::Shutdown, "shutdown", |data| {
            assert!(data.is_null(), "unit variants carry null data")
        }),
    ]
}

#[test]
fn every_variant_converts_to_a_stable_envelope() {
    let cases = all_variant_cases();
    assert_eq!(
        cases.len(),
        28,
        "a DomainEvent variant is missing test coverage"
    );
    for (event, topic, check_data) in &cases {
        assert_eq!(event.topic(), *topic);
        let envelope = event
            .to_envelope()
            .unwrap_or_else(|error| panic!("{topic} must convert: {error}"));
        assert_eq!(envelope.topic, *topic);
        check_data(&envelope.data);
        // The envelope preserves the exact historical wire shape: the
        // same data every transport already extracted from the event.
        let serialized = serde_json::to_value(event).expect("variant must serialize");
        match serialized.get("data") {
            Some(expected) => assert_eq!(&envelope.data, expected),
            None => assert!(envelope.data.is_null()),
        }
        // The envelope itself round-trips through the plugin boundary.
        let round_trip: tiktools_plugin_api::DomainEventEnvelope =
            serde_json::from_value(serde_json::to_value(&envelope).expect("envelope serializes"))
                .expect("envelope deserializes");
        assert_eq!(round_trip, envelope);
    }
}

#[test]
fn topic_matches_serialized_tag_for_every_variant() {
    for (event, topic, _) in all_variant_cases() {
        let serialized = serde_json::to_value(&event).expect("variant must serialize");
        assert_eq!(serialized.get("topic").and_then(Value::as_str), Some(topic));
    }
}
