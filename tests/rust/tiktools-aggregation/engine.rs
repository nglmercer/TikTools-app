use tiktools_aggregation::engine::Engine;
use tiktools_aggregation::model::{Contribution, Definition, SourceRule};

#[test]
fn daily_window_resets_at_local_midnight() {
    use tiktools_aggregation::model::Window;
    const DAY: u64 = 86_400_000;
    let mut engine = Engine::default();
    engine
        .create(Definition {
            schema_version: 1,
            id: "daily".into(),
            creator_id: "c".into(),
            room_id: "r".into(),
            group_by: Default::default(),
            // UTC-5: local midnight is 05:00 UTC.
            window: Window::Daily {
                utc_offset_minutes: -300,
            },
            user_limits: None,
            capacity: 3,
            limit: 3,
            sources: vec![SourceRule {
                source: "like".into(),
                metric: "count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        })
        .unwrap();
    let event = Contribution {
        creator_id: "c",
        room_id: "r",
        source: "like",
        user: Some("u"),
        gift: None,
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("count", 3)],
    };
    let base = 10 * DAY + 3_600_000; // 01:00 UTC = 20:00 local, previous day.
    engine.apply_at(&event, base);
    engine.apply_at(&event, base + 3_000_000); // 01:50 UTC, same local day.
    assert_eq!(engine.snapshot("daily").unwrap().total_score, 6);
    engine.expire_at(base + 4 * 3_600_000 + 1); // 05:00 UTC: local midnight.
    assert_eq!(engine.snapshot("daily").unwrap().total_score, 0);
    engine.apply_at(&event, base + 5 * 3_600_000);
    assert_eq!(engine.snapshot("daily").unwrap().total_score, 3);
}
#[test]
fn checkpoint_preserves_rolling_expiry_and_user_limits() {
    use tiktools_aggregation::model::{UserLimits, Window};
    let definition = Definition {
        schema_version: 1,
        id: "rolling".into(),
        creator_id: "c".into(),
        room_id: "r".into(),
        group_by: Default::default(),
        window: Window::Rolling {
            duration_ms: 100,
            max_events: 4,
        },
        user_limits: Some(UserLimits {
            max_contributions: Some(2),
            cooldown_ms: Some(20),
            max_users: 3,
        }),
        capacity: 3,
        limit: 3,
        sources: vec![SourceRule {
            source: "like".into(),
            metric: "count".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
    };
    let mut source = Engine::default();
    source.create(definition.clone()).unwrap();
    let event = Contribution {
        creator_id: "c",
        room_id: "r",
        source: "like",
        user: Some("u"),
        gift: None,
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("count", 3)],
    };
    source.apply_at(&event, 100);
    let checkpoint = source.session_checkpoint("rolling").unwrap();
    let mut restored = Engine::default();
    restored.create(definition).unwrap();
    restored.restore_session_checkpoint(&checkpoint).unwrap();
    restored.apply_at(&event, 110);
    assert_eq!(restored.snapshot("rolling").unwrap().cooldown_rejected, 1);
    restored.apply_at(&event, 120);
    assert_eq!(restored.snapshot("rolling").unwrap().total_score, 6);
    restored.expire_at(200);
    assert_eq!(restored.snapshot("rolling").unwrap().total_score, 3);
    restored.apply_at(&event, 220);
    assert_eq!(restored.snapshot("rolling").unwrap().limited, 1);
    assert_eq!(restored.snapshot("rolling").unwrap().total_score, 0);
}
#[test]
fn session_checkpoint_restores_full_ranking_and_rejects_corruption_atomically() {
    let definition = Definition {
        schema_version: 1,
        id: "rank".into(),
        creator_id: "c".into(),
        room_id: "r".into(),
        group_by: Default::default(),
        window: Default::default(),
        user_limits: None,
        capacity: 3,
        limit: 1,
        sources: vec![SourceRule {
            source: "like".into(),
            metric: "count".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
    };
    let mut source = Engine::default();
    source.create(definition.clone()).unwrap();
    for user in ["a", "b"] {
        assert!(source
            .apply(&Contribution {
                creator_id: "c",
                room_id: "r",
                source: "like",
                user: Some(user),
                gift: None,
                option: None,
                team: None,
                custom_key: None,
                metrics: &[("count", 3)]
            })
            .is_empty());
    }
    let checkpoint = source.session_checkpoint("rank").unwrap();
    assert_eq!(checkpoint.snapshot.entries.len(), 2);
    let (presentation, limit) = source.snapshot_unranked_capped("rank", 1).unwrap();
    assert_eq!(presentation.rank(limit).entries.len(), 1);
    assert_eq!(source.selection_snapshot("rank").unwrap().entries.len(), 2);
    let mut target = Engine::default();
    target.create(definition.clone()).unwrap();
    let fresh_identity = target.snapshot("rank").unwrap().instance_id.unwrap();
    assert_ne!(
        Some(&fresh_identity),
        checkpoint.snapshot.instance_id.as_ref()
    );
    let mut corrupt = checkpoint.clone();
    corrupt.snapshot.total_score += 1;
    assert_eq!(
        target.restore_session_checkpoint(&corrupt),
        Err(tiktools_aggregation::Error::IncompatibleDefinition)
    );
    assert_eq!(target.snapshot("rank").unwrap().total_score, 0);
    let mut legacy = checkpoint.clone();
    legacy.snapshot.instance_id = None;
    let mut legacy_target = Engine::default();
    legacy_target.create(definition).unwrap();
    let legacy_identity = legacy_target.snapshot("rank").unwrap().instance_id;
    legacy_target.restore_session_checkpoint(&legacy).unwrap();
    assert_eq!(
        legacy_target.snapshot("rank").unwrap().instance_id,
        legacy_identity
    );
    target.restore_session_checkpoint(&checkpoint).unwrap();
    assert_eq!(
        target.selection_snapshot("rank"),
        source.selection_snapshot("rank")
    );
    target.set_paused("rank", true).unwrap().unwrap();
    target.mark_recovery_gap("rank").unwrap();
    assert_eq!(
        target.snapshot("rank").unwrap().integrity_status,
        "degraded"
    );
    assert_eq!(
        target.restore_session_checkpoint(&checkpoint),
        Err(tiktools_aggregation::Error::IncompatibleDefinition)
    );
}
#[test]
fn presentation_metadata_follows_accepted_scores_and_expires() {
    use tiktools_aggregation::{model::Window, EntryMetadata};
    let mut engine = Engine::default();
    engine
        .create(Definition {
            schema_version: 1,
            id: "likes".into(),
            creator_id: "c".into(),
            room_id: "r".into(),
            group_by: Default::default(),
            window: Window::Rolling {
                duration_ms: 100,
                max_events: 2,
            },
            user_limits: None,
            capacity: 2,
            limit: 2,
            sources: vec![SourceRule {
                source: "like".into(),
                metric: "count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        })
        .unwrap();
    let mut event = Contribution {
        creator_id: "c",
        room_id: "r",
        source: "like",
        user: Some("42"),
        gift: None,
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("count", 2)],
    };
    let metadata = EntryMetadata {
        label: Some("Alice".into()),
        user_id: Some("42".into()),
        nickname: Some("Alice".into()),
        avatar_url: Some("x".repeat(2049)),
        ..Default::default()
    };
    assert!(engine
        .apply_at_with_metadata(&event, 10, Some(&metadata), None)
        .is_empty());
    let entry = &engine.snapshot("likes").unwrap().entries[0];
    assert_eq!(
        (
            entry.key.as_str(),
            entry.label.as_str(),
            entry.metadata.user_id.as_deref()
        ),
        ("42", "Alice", Some("42"))
    );
    assert!(entry.metadata.avatar_url.is_none());
    event.metrics = &[("count", 0)];
    let other = EntryMetadata {
        label: Some("Ignored".into()),
        ..Default::default()
    };
    engine.apply_at_with_metadata(&event, 11, Some(&other), None);
    assert_eq!(engine.snapshot("likes").unwrap().entries[0].label, "Alice");
    engine.expire_at(110);
    assert!(engine.snapshot("likes").unwrap().entries.is_empty());
    event.metrics = &[("count", 1)];
    engine.apply_at(&event, 111);
    assert_eq!(engine.snapshot("likes").unwrap().entries[0].label, "42");
}
#[test]
fn durable_assignment_rebuild_and_switch_match_incremental_projection() {
    use tiktools_aggregation::model::GroupBy;
    use tiktools_aggregation::Error;
    let mut engine = Engine::default();
    let definition = Definition {
        schema_version: 1,
        id: "poll".into(),
        creator_id: "c".into(),
        room_id: "r".into(),
        group_by: GroupBy::Option,
        window: Default::default(),
        user_limits: None,
        sources: vec![SourceRule {
            source: "competition.vote".into(),
            metric: "event.count".into(),
            weight: 3,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
        limit: 10,
        capacity: 2,
    };
    engine.create(definition).unwrap();
    engine.apply_assignment("poll", None, ("a", 1)).unwrap();
    engine
        .apply_assignment("poll", Some(("a", 1)), ("b", 2))
        .unwrap();
    let before = engine.snapshot("poll").unwrap();
    assert_eq!(
        (before.entries[0].key.as_str(), before.total_score),
        ("b", 6)
    );
    assert_eq!(
        engine.restore_assignments("poll", &[("a".into(), u64::MAX)]),
        Err(Error::Overflow)
    );
    assert_eq!(engine.snapshot("poll").unwrap(), before);
    engine
        .restore_assignments("poll", &[("b".into(), 2)])
        .unwrap();
    let restored = engine.snapshot("poll").unwrap();
    assert_eq!(restored.entries, before.entries);
    assert_eq!(restored.total_score, before.total_score);
    assert!(restored.revision > before.revision);
    assert_eq!(
        engine.restore_assignments("poll", &[("a".into(), 1), ("a".into(), 2)]),
        Err(Error::InvalidKey)
    );
    assert_eq!(engine.snapshot("poll").unwrap(), restored);
    engine
        .restore_assignments_with_scores(
            "poll",
            &[("b".into(), 2)],
            &[("a".into(), 5), ("b".into(), 4)],
        )
        .unwrap();
    let mixed = engine.snapshot("poll").unwrap();
    assert_eq!(mixed.total_score, 15);
    assert_eq!(
        (mixed.entries[0].key.as_str(), mixed.entries[0].score),
        ("b", 10)
    );
    assert_eq!(
        engine.restore_assignments_with_scores(
            "poll",
            &[("b".into(), 2)],
            &[("a".into(), u64::MAX)]
        ),
        Err(Error::Overflow)
    );
    assert_eq!(engine.snapshot("poll").unwrap(), mixed);
    assert_eq!(
        engine.restore_assignments_with_scores("poll", &[], &[("a".into(), 1), ("a".into(), 2)]),
        Err(Error::InvalidKey)
    );
    assert_eq!(engine.snapshot("poll").unwrap(), mixed);
}
#[test]
fn user_limits_are_bounded_and_count_only_accepted_scored_packets() {
    use tiktools_aggregation::model::{GroupBy, UserLimits};
    use tiktools_aggregation::Error;
    let mut engine = Engine::default();
    engine
        .create(Definition {
            schema_version: 1,
            id: "limits".into(),
            creator_id: "c".into(),
            room_id: "r".into(),
            group_by: GroupBy::Gift,
            window: Default::default(),
            user_limits: Some(UserLimits {
                max_contributions: Some(2),
                cooldown_ms: Some(100),
                max_users: 1,
            }),
            sources: vec![SourceRule {
                source: "gift".into(),
                metric: "count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
            limit: 10,
            capacity: 10,
        })
        .unwrap();
    let mut event = Contribution {
        creator_id: "c",
        room_id: "r",
        source: "gift",
        user: Some("u"),
        gift: Some("a"),
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("count", 3)],
    };
    assert!(engine.apply_at(&event, 1000).is_empty());
    event.gift = Some("b");
    assert!(engine.apply_at(&event, 1099).is_empty());
    event.metrics = &[("count", 0)];
    assert!(engine.apply_at(&event, 1099).is_empty());
    event.metrics = &[("count", 3)];
    assert!(engine.apply_at(&event, 1100).is_empty());
    assert!(engine.apply_at(&event, 1200).is_empty());
    let snapshot = engine.snapshot("limits").unwrap();
    assert_eq!(
        (
            snapshot.total_score,
            snapshot.limited,
            snapshot.cooldown_rejected
        ),
        (6, 1, 1)
    );
    assert_eq!(
        (snapshot.rejected, snapshot.integrity_status.as_str()),
        (0, "unknown")
    );
    event.user = Some("other");
    assert_eq!(
        engine.apply_at(&event, 1300),
        vec![("limits".into(), Error::Capacity)]
    );
    event.user = None;
    assert_eq!(
        engine.apply_at(&event, 1301),
        vec![("limits".into(), Error::InvalidKey)]
    );
    engine.reset("limits").unwrap().unwrap();
    event.user = Some("other");
    assert!(engine.apply_at(&event, 1302).is_empty());
    let reset = engine.snapshot("limits").unwrap();
    assert_eq!(
        (
            reset.total_score,
            reset.limited,
            reset.cooldown_rejected,
            reset.rejected
        ),
        (3, 0, 0, 0)
    );
}
#[test]
fn gift_multiplier_is_integer_scoped_and_rejects_ambiguous_or_overflowing_input() {
    use std::collections::BTreeMap;
    use tiktools_aggregation::Error;
    let mut engine = Engine::default();
    let rule = SourceRule {
        source: "tiktok.gift".into(),
        metric: "gift.diamonds".into(),
        weight: 2,
        enabled: true,
        gift_multipliers: BTreeMap::from([("rose".into(), 3), ("disabled".into(), 0)]),
    };
    let definition = Definition {
        schema_version: 1,
        id: "gift".into(),
        creator_id: "c".into(),
        room_id: "r".into(),
        group_by: Default::default(),
        window: Default::default(),
        user_limits: None,
        capacity: 10,
        limit: 10,
        sources: vec![rule],
    };
    engine.create(definition.clone()).unwrap();
    let mut event = Contribution {
        creator_id: "c",
        room_id: "r",
        source: "tiktok.gift",
        user: Some("u"),
        gift: Some("rose"),
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("gift.diamonds", 5)],
    };
    assert!(engine.apply_at(&event, 1).is_empty());
    assert_eq!(engine.snapshot("gift").unwrap().total_score, 30);
    event.gift = Some("disabled");
    event.metrics = &[("gift.diamonds", u64::MAX)];
    assert!(engine.apply_at(&event, 2).is_empty());
    event.metrics = &[("gift.diamonds", 5)];
    event.gift = Some("unknown");
    assert!(engine.apply_at(&event, 3).is_empty());
    assert_eq!(engine.snapshot("gift").unwrap().total_score, 40);
    event.gift = None;
    assert_eq!(
        engine.apply_at(&event, 4),
        vec![("gift".into(), Error::InvalidKey)]
    );
    event.metrics = &[("gift.diamonds", 0)];
    assert!(engine.apply_at(&event, 5).is_empty());
    event.gift = Some("rose");
    event.metrics = &[("gift.diamonds", u64::MAX)];
    assert_eq!(
        engine.apply_at(&event, 6),
        vec![("gift".into(), Error::Overflow)]
    );
    assert_eq!(engine.snapshot("gift").unwrap().total_score, 40);
    let mut invalid = definition;
    invalid.id = "invalid".into();
    invalid.sources[0].weight = 1_000_000_001;
    assert!(engine.create(invalid).is_err());
}
#[test]
fn rolling_window_expires_at_boundary_and_rejects_excess() {
    use tiktools_aggregation::model::Window;
    use tiktools_aggregation::Error;
    let mut engine = Engine::default();
    engine
        .create(Definition {
            schema_version: 1,
            id: "r".into(),
            creator_id: "c".into(),
            room_id: "room".into(),
            group_by: Default::default(),
            window: Window::Rolling {
                duration_ms: 100,
                max_events: 2,
            },
            user_limits: None,
            limit: 10,
            capacity: 4,
            sources: vec![SourceRule {
                source: "like".into(),
                metric: "count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        })
        .unwrap();
    let event = Contribution {
        creator_id: "c",
        room_id: "room",
        source: "like",
        user: Some("u"),
        gift: None,
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("count", 5)],
    };
    assert!(engine.apply_at(&event, 1000).is_empty());
    assert!(engine.apply_at(&event, 1010).is_empty());
    assert_eq!(
        engine.apply_at(&event, 1020),
        vec![("r".into(), Error::WindowCapacity)]
    );
    assert_eq!(engine.snapshot("r").unwrap().total_score, 10);
    engine.expire_at(1099);
    assert_eq!(engine.snapshot("r").unwrap().total_score, 10);
    engine.expire_at(1100);
    assert_eq!(engine.snapshot("r").unwrap().total_score, 5);
    // Regressing clocks never bring expired contributions back.
    engine.expire_at(900);
    assert_eq!(engine.snapshot("r").unwrap().total_score, 5);
    engine.expire_at(1110);
    let snapshot = engine.snapshot("r").unwrap();
    assert_eq!(snapshot.total_score, 0);
    assert_eq!(snapshot.total_participants, 0);
    assert_eq!(snapshot.integrity_status, "degraded");
}
#[test]
fn definition_update_requires_pause_and_explicit_reset() {
    use tiktools_aggregation::engine::DefinitionError;
    let mut engine = Engine::default();
    let mut definition = Definition {
        schema_version: 1,
        id: "r".into(),
        creator_id: "c".into(),
        room_id: "room".into(),
        group_by: Default::default(),
        window: Default::default(),
        user_limits: None,
        limit: 2,
        capacity: 4,
        sources: vec![SourceRule {
            source: "tiktok.like".into(),
            metric: "like.count".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
    };
    engine.create(definition.clone()).unwrap();
    assert_eq!(
        engine.update(definition.clone(), false),
        Err(DefinitionError::NotPaused)
    );
    let event = Contribution {
        creator_id: "c",
        room_id: "room",
        source: "tiktok.like",
        user: Some("u"),
        gift: None,
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("like.count", 5)],
    };
    engine.apply(&event);
    engine.set_paused("r", true).unwrap().unwrap();
    definition.limit = 1;
    engine.update(definition.clone(), false).unwrap();
    assert_eq!(engine.snapshot("r").unwrap().total_score, 5);
    definition.sources[0].weight = 2;
    assert_eq!(
        engine.update(definition.clone(), false),
        Err(DefinitionError::ResetRequired)
    );
    engine.update(definition, true).unwrap();
    assert_eq!(engine.snapshot("r").unwrap().total_score, 0);
    engine.set_paused("r", false).unwrap().unwrap();
    engine.apply(&event);
    assert_eq!(engine.snapshot("r").unwrap().total_score, 10);
}
#[test]
fn ingress_loss_degrades_running_rankings_and_reset_recovers_local_status() {
    let mut engine = Engine::default();
    engine
        .create(Definition {
            schema_version: 1,
            id: "r".into(),
            creator_id: "c".into(),
            room_id: "room".into(),
            group_by: Default::default(),
            window: Default::default(),
            user_limits: None,
            limit: 1,
            capacity: 4,
            sources: vec![SourceRule {
                source: "tiktok.like".into(),
                metric: "like.count".into(),
                weight: 1,
                enabled: true,
                gift_multipliers: Default::default(),
            }],
        })
        .unwrap();
    engine.mark_ingress_loss(3);
    let degraded = engine.snapshot("r").unwrap();
    assert_eq!(degraded.dropped, 3);
    assert_eq!(degraded.integrity_status, "degraded");
    assert_eq!(degraded.revision, 1);
    engine.reset("r").unwrap().unwrap();
    let reset = engine.snapshot("r").unwrap();
    assert_eq!(reset.dropped, 0);
    assert_eq!(reset.integrity_status, "unknown");
    assert!(reset.revision > degraded.revision);
}
#[test]
fn packet_overflow_is_atomic_and_marks_integrity() {
    use tiktools_aggregation::model::GroupBy;
    let mut engine = Engine::default();
    engine
        .create(Definition {
            schema_version: 1,
            id: "gift".into(),
            creator_id: "c".into(),
            room_id: "r".into(),
            group_by: GroupBy::Gift,
            window: Default::default(),
            user_limits: None,
            limit: 10,
            capacity: 10,
            sources: vec![
                SourceRule {
                    source: "gift".into(),
                    metric: "count".into(),
                    weight: 1,
                    enabled: true,
                    gift_multipliers: Default::default(),
                },
                SourceRule {
                    source: "gift".into(),
                    metric: "diamonds".into(),
                    weight: 2,
                    enabled: true,
                    gift_multipliers: Default::default(),
                },
            ],
        })
        .unwrap();
    let event = Contribution {
        creator_id: "c",
        room_id: "r",
        source: "gift",
        user: Some("u"),
        gift: Some("9"),
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("count", 1), ("diamonds", u64::MAX)],
    };
    assert_eq!(engine.apply(&event).len(), 1);
    let snapshot = engine.snapshot("gift").unwrap();
    assert_eq!(snapshot.ranking_id, "gift");
    assert_eq!(snapshot.creator_id, "c");
    assert_eq!(snapshot.total_score, 0);
    assert_eq!(snapshot.rejected, 1);
    assert_eq!(snapshot.integrity_status, "degraded");
}
#[test]
fn sources_scopes_and_pause_are_independent() {
    let mut engine = Engine::default();
    engine
        .create(Definition {
            schema_version: 1,
            id: "rank".into(),
            creator_id: "creator".into(),
            room_id: "room".into(),
            group_by: Default::default(),
            window: Default::default(),
            user_limits: None,
            limit: 10,
            capacity: 4,
            sources: vec![
                SourceRule {
                    source: "tiktok.like".into(),
                    metric: "like.count".into(),
                    weight: 2,
                    enabled: true,
                    gift_multipliers: Default::default(),
                },
                SourceRule {
                    source: "tiktok.gift".into(),
                    metric: "gift.diamonds".into(),
                    weight: 3,
                    enabled: false,
                    gift_multipliers: Default::default(),
                },
            ],
        })
        .unwrap();
    let mut c = Contribution {
        creator_id: "creator",
        room_id: "room",
        source: "tiktok.like",
        user: Some("user"),
        gift: None,
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("like.count", 5)],
    };
    assert!(engine.apply(&c).is_empty());
    c.room_id = "other";
    engine.apply(&c);
    c.room_id = "room";
    c.source = "tiktok.gift";
    c.metrics = &[("gift.diamonds", 5)];
    engine.apply(&c);
    engine.set_paused("rank", true).unwrap().unwrap();
    c.source = "tiktok.like";
    c.metrics = &[("like.count", 5)];
    engine.apply(&c);
    assert_eq!(engine.snapshot("rank").unwrap().total_score, 10);
    assert!(engine.remove("rank"));
    assert!(engine.snapshot("rank").is_none());
}
