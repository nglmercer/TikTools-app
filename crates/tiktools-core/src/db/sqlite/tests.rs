use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::*;
use crate::paths::AppPaths;
use serde_json::json;

fn paths() -> AppPaths {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("tiktools-rust-sqlite-{suffix}"));
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin-plugins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    }
}

#[test]
fn initializes_compatible_schemas_and_round_trips_records() {
    let paths = paths();
    paths.ensure_directories().unwrap();
    let database = DatabaseManager::new(paths.clone());
    let config = database.load_points_config().unwrap();
    assert_eq!(config.currency_name, "Points");

    let graph = json!({
        "schemaVersion": 1,
        "id": "workflow-1",
        "name": "Demo",
        "enabled": false,
        "nodes": [],
        "edges": []
    });
    let saved = database.save_workflow(&graph).unwrap();
    assert_eq!(saved["id"], "workflow-1");
    assert_eq!(database.load_workflows().unwrap().len(), 1);

    let action = json!({"id":"action-1","name":"Action","enabled":true,"type":"code"});
    database.save_behavior("behavior_actions", &action).unwrap();
    let snapshot = database.load_behavior_snapshot().unwrap();
    assert_eq!(snapshot["actions"][0]["id"], "action-1");

    let _ = fs::remove_dir_all(paths.root);
}

#[test]
fn ranking_definitions_survive_reopen_and_delete() {
    use tiktools_aggregation::model::{Definition, SourceRule};
    let paths = paths();
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    let definition = Definition {
        schema_version: 1,
        id: "likes".into(),
        creator_id: "creator".into(),
        room_id: "room".into(),
        group_by: Default::default(),
        window: Default::default(),
        user_limits: None,
        capacity: 10,
        limit: 10,
        sources: vec![SourceRule {
            source: "tiktok.like".into(),
            metric: "like.count".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
    };
    db.insert_aggregation_definition(&definition).unwrap();
    let mut engine = tiktools_aggregation::engine::Engine::default();
    engine.create(definition.clone()).unwrap();
    let checkpoint = engine.session_checkpoint("likes").unwrap();
    db.claim_aggregation_checkpoints("owner").unwrap();
    db.save_aggregation_checkpoint(&checkpoint, "owner")
        .unwrap();
    let reopened = DatabaseManager::new(paths.clone());
    assert_eq!(
        reopened.load_aggregation_definitions().unwrap(),
        vec![definition.clone()]
    );
    // Loading does not consume: a crash before the next write keeps it.
    for _ in 0..2 {
        assert!(reopened
            .load_aggregation_checkpoint("likes", "owner")
            .unwrap()
            .is_some());
    }
    reopened.clear_aggregation_checkpoint("likes").unwrap();
    assert!(reopened
        .load_aggregation_checkpoint("likes", "owner")
        .unwrap()
        .is_none());
    reopened
        .save_aggregation_checkpoint(&checkpoint, "owner")
        .unwrap();
    reopened.claim_aggregation_checkpoints("new-owner").unwrap();
    assert!(db
        .save_aggregation_checkpoint(&checkpoint, "owner")
        .is_err());
    let mut updated = definition;
    updated.limit = 5;
    assert!(reopened.update_aggregation_definition(&updated).unwrap());
    assert!(reopened
        .load_aggregation_checkpoint("likes", "new-owner")
        .unwrap()
        .is_none());
    assert!(reopened
        .save_aggregation_checkpoint(&checkpoint, "new-owner")
        .is_err());
    assert_eq!(
        reopened.load_aggregation_definitions().unwrap(),
        vec![updated]
    );
    assert!(reopened.delete_aggregation_definition("likes").unwrap());
    assert!(reopened.load_aggregation_definitions().unwrap().is_empty());
    let _ = fs::remove_dir_all(paths.root);
}

#[test]
fn runtime_checkpoint_survives_unclean_exit() {
    use tiktools_aggregation::engine::Engine;
    use tiktools_aggregation::model::{Contribution, Definition, SourceRule};
    let paths = paths();
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    let definition = Definition {
        schema_version: 1,
        id: "gifts".into(),
        creator_id: "creator".into(),
        room_id: "room".into(),
        group_by: Default::default(),
        window: Default::default(),
        user_limits: None,
        capacity: 10,
        limit: 10,
        sources: vec![SourceRule {
            source: "tiktok.gift".into(),
            metric: "gift.diamonds".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
    };
    db.insert_aggregation_definition(&definition).unwrap();
    db.claim_aggregation_checkpoints("crashed").unwrap();
    let mut engine = Engine::default();
    engine.create(definition.clone()).unwrap();
    let event = Contribution {
        creator_id: "creator",
        room_id: "room",
        source: "tiktok.gift",
        user: Some("user"),
        gift: None,
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("gift.diamonds", 30)],
    };
    assert!(engine.apply_at(&event, 1).is_empty());
    // Runtime write, then the process dies without finishing the run.
    db.save_aggregation_checkpoint(&engine.session_checkpoint("gifts").unwrap(), "crashed")
        .unwrap();
    let reopened = DatabaseManager::new(paths.clone());
    assert!(!reopened.claim_aggregation_checkpoints("next").unwrap());
    let checkpoint = reopened
        .load_aggregation_checkpoint("gifts", "next")
        .unwrap()
        .unwrap();
    let mut restored = Engine::default();
    restored.create(definition).unwrap();
    restored.restore_session_checkpoint(&checkpoint).unwrap();
    restored.mark_recovery_gap("gifts").unwrap();
    let snapshot = restored.snapshot("gifts").unwrap();
    assert_eq!(snapshot.total_score, 30);
    assert_eq!(snapshot.integrity_status, "degraded");
    let _ = fs::remove_dir_all(paths.root);
}

#[test]
fn rolling_checkpoint_survives_sqlite_and_preserves_limits() {
    use tiktools_aggregation::engine::Engine;
    use tiktools_aggregation::model::{Contribution, Definition, SourceRule, UserLimits, Window};
    let paths = paths();
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    let definition = Definition {
        schema_version: 1,
        id: "rolling".into(),
        creator_id: "creator".into(),
        room_id: "room".into(),
        group_by: Default::default(),
        window: Window::Rolling {
            duration_ms: 100,
            max_events: 10,
        },
        user_limits: Some(UserLimits {
            max_contributions: Some(1),
            cooldown_ms: None,
            max_users: 10,
        }),
        capacity: 10,
        limit: 10,
        sources: vec![SourceRule {
            source: "tiktok.like".into(),
            metric: "like.count".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
    };
    db.insert_aggregation_definition(&definition).unwrap();
    db.claim_aggregation_checkpoints("owner").unwrap();
    let mut engine = Engine::default();
    engine.create(definition.clone()).unwrap();
    let event = Contribution {
        creator_id: "creator",
        room_id: "room",
        source: "tiktok.like",
        user: Some("user"),
        gift: None,
        option: None,
        team: None,
        custom_key: None,
        metrics: &[("like.count", 7)],
    };
    assert!(engine.apply_at(&event, 100).is_empty());
    db.save_aggregation_checkpoint(&engine.session_checkpoint("rolling").unwrap(), "owner")
        .unwrap();
    db.finish_aggregation_checkpoints("owner").unwrap();
    assert!(db.claim_aggregation_checkpoints("next").unwrap());
    let checkpoint = db
        .load_aggregation_checkpoint("rolling", "next")
        .unwrap()
        .unwrap();
    let mut restored = Engine::default();
    restored.create(definition).unwrap();
    restored.restore_session_checkpoint(&checkpoint).unwrap();
    restored.expire_at(200);
    assert_eq!(restored.snapshot("rolling").unwrap().total_score, 0);
    assert!(restored.apply_at(&event, 201).is_empty());
    assert_eq!(restored.snapshot("rolling").unwrap().limited, 1);
    let _ = fs::remove_dir_all(paths.root);
}

#[test]
fn competition_result_is_immutable_across_retry_and_reopen() {
    use tiktools_competitions::{
        Candidate, ResultRecord, Selection, SelectionMode, TiePolicy, WinnerAction,
    };
    let paths = paths();
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    use tiktools_competitions::{
        CompetitionKind, Definition, Lifecycle, OptionDefinition, Transition,
    };
    let action = WinnerAction {
        action_id: "winner-action".into(),
        inputs: json!({"amount": 20}),
    };
    db.insert_competition_definition(&Definition {
        schema_version: 1,
        id: "poll".into(),
        name: "Poll".into(),
        kind: CompetitionKind::Poll,
        ranking_id: "rank".into(),
        options: vec![
            OptionDefinition {
                id: "a".into(),
                label: "A".into(),
                commands: vec![],
                gift_ids: vec![],
                image_url: None,
                on_win: Some(action.clone()),
            },
            OptionDefinition {
                id: "b".into(),
                label: "B".into(),
                commands: vec![],
                gift_ids: vec![],
                image_url: None,
                on_win: None,
            },
        ],
        selection_mode: SelectionMode::WeightedRandom,
        tie_policy: TiePolicy::Stable,
        duration_ms: Some(10),
        vote_limit: 1,
        allow_switch: false,
        minimum_participants: 1,
        target_score: None,
        on_user_win: None,
    })
    .unwrap();
    db.transition_competition("poll", Transition::Start, 100)
        .unwrap();
    let result = ResultRecord {
        schema_version: 1,
        competition_id: "poll".into(),
        generation: 1,
        ranking_id: "rank".into(),
        ranking_revision: 7,
        finalized_at: 123,
        selection: Selection {
            mode: SelectionMode::WeightedRandom,
            tie_policy: TiePolicy::Stable,
            eligible: vec![
                Candidate {
                    key: "a".into(),
                    weight: 2,
                },
                Candidate {
                    key: "b".into(),
                    weight: 1,
                },
            ],
            winner: Some("a".into()),
        },
    };
    assert!(db.store_competition_result(&result, Some(&action)).is_err());
    assert!(db.latest_competition_results().unwrap().is_empty());
    db.transition_competition("poll", Transition::Expire, 110)
        .unwrap();
    let mut wrong = result.clone();
    wrong.generation = 2;
    assert!(db.store_competition_result(&wrong, Some(&action)).is_err());
    wrong = result.clone();
    wrong.ranking_id = "other".into();
    assert!(db.store_competition_result(&wrong, Some(&action)).is_err());
    assert!(db.store_competition_result(&result, None).is_err());
    assert_eq!(
        db.load_competition_state("poll")
            .unwrap()
            .unwrap()
            .lifecycle,
        Lifecycle::Finalizing
    );
    // A failed outbox write must also roll back the result and lifecycle.
    let connection = db.open(&db.automation_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_outbox BEFORE INSERT ON competition_action_outbox BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
    drop(connection);
    assert!(db.store_competition_result(&result, Some(&action)).is_err());
    assert!(db.load_competition_result("poll", 1).unwrap().is_none());
    assert_eq!(
        db.load_competition_state("poll")
            .unwrap()
            .unwrap()
            .lifecycle,
        Lifecycle::Finalizing
    );
    db.open(&db.automation_path())
        .unwrap()
        .execute_batch("DROP TRIGGER reject_outbox")
        .unwrap();
    assert_eq!(
        db.store_competition_result(&result, Some(&action)).unwrap(),
        result
    );
    assert_eq!(
        db.latest_competition_results().unwrap(),
        vec![result.clone()]
    );
    assert_eq!(
        db.load_competition_state("poll")
            .unwrap()
            .unwrap()
            .lifecycle,
        Lifecycle::Finished
    );
    let mut retry = result.clone();
    retry.selection.winner = Some("b".into());
    assert_eq!(db.store_competition_result(&retry, None).unwrap(), result);
    let connection = db.open(&db.automation_path()).unwrap();
    let status: String = connection.query_row("SELECT status FROM competition_action_outbox WHERE competition_id = 'poll' AND generation = 1", [], |row| row.get(0)).unwrap();
    assert_eq!(status, "pending");
    let mut corrupt = serde_json::to_value(&result).unwrap();
    corrupt["selection"]["winner"] = json!("missing");
    connection.execute("UPDATE competition_results SET result_json=?1 WHERE competition_id='poll' AND generation=1", [corrupt.to_string()]).unwrap();
    assert!(db.load_competition_result("poll", 1).is_err());
    assert!(db.list_competition_results("poll", None, 1).is_err());
    assert!(db.latest_competition_results().is_err());
    assert!(db.store_competition_result(&result, Some(&action)).is_err());
    corrupt = serde_json::to_value(&result).unwrap();
    corrupt["competitionId"] = json!("other");
    connection.execute("UPDATE competition_results SET result_json=?1 WHERE competition_id='poll' AND generation=1", [corrupt.to_string()]).unwrap();
    assert!(db.load_competition_result("poll", 1).is_err());
    connection.execute("UPDATE competition_results SET result_json=?1 WHERE competition_id='poll' AND generation=1", [serde_json::to_string(&result).unwrap()]).unwrap();
    connection.execute("UPDATE competition_action_outbox SET action_json='invalid' WHERE competition_id='poll' AND generation=1", []).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_invalid_outbox BEFORE INSERT ON competition_action_diagnostics BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    assert!(db.claim_competition_action("poll", 1, 124).is_err());
    assert_eq!(
        db.competition_action("poll", 1).unwrap().unwrap().status,
        tiktools_competitions::ActionStatus::Pending
    );
    connection
        .execute_batch("DROP TRIGGER reject_invalid_outbox")
        .unwrap();
    assert!(
        matches!(db.claim_competition_action_detailed("poll", 1, 124).unwrap(), ActionClaimOutcome::Invalid { action_id } if action_id == "winner-action")
    );
    let invalid = db.competition_action("poll", 1).unwrap().unwrap();
    assert_eq!(invalid.status, tiktools_competitions::ActionStatus::Failed);
    assert_eq!(invalid.reason.as_deref(), Some("invalid_outbox_record"));
    connection.execute("UPDATE competition_action_outbox SET action_json=?1,status='pending' WHERE competition_id='poll' AND generation=1", [serde_json::to_string(&action).unwrap()]).unwrap();
    connection.execute("DELETE FROM competition_action_diagnostics WHERE competition_id='poll' AND generation=1", []).unwrap();
    drop(connection);
    let claim = db
        .claim_competition_action("poll", 1, 124)
        .unwrap()
        .unwrap();
    assert_eq!(claim.action.action_id, "winner-action");
    let connection = db.open(&db.automation_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_action_diagnostic BEFORE INSERT ON competition_action_diagnostics BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    assert!(db
        .record_competition_action_outcome(
            "poll",
            1,
            tiktools_competitions::ActionStatus::Uncertain,
            Some("execution_failed"),
            124
        )
        .is_err());
    assert_eq!(
        db.competition_action("poll", 1).unwrap().unwrap().status,
        tiktools_competitions::ActionStatus::Running
    );
    connection
        .execute_batch("DROP TRIGGER reject_action_diagnostic")
        .unwrap();
    drop(connection);
    assert!(db
        .record_competition_action_outcome(
            "poll",
            1,
            tiktools_competitions::ActionStatus::Uncertain,
            Some("not_allowlisted"),
            124
        )
        .is_err());
    assert!(db
        .claim_competition_action("poll", 1, 125)
        .unwrap()
        .is_none());
    assert!(db
        .mark_competition_action_uncertain("poll", 1, 126)
        .unwrap());
    assert_eq!(
        db.competition_action("poll", 1)
            .unwrap()
            .unwrap()
            .reason
            .as_deref(),
        Some("operator_marked_uncertain")
    );
    assert!(!db
        .mark_competition_action_uncertain("poll", 1, 126)
        .unwrap());
    assert_eq!(
        db.mark_interrupted_competition_actions_uncertain(126)
            .unwrap(),
        0
    );
    assert!(db
        .claim_competition_action("poll", 1, 127)
        .unwrap()
        .is_none());
    assert_eq!(
        db.competition_action("poll", 1).unwrap().unwrap().status,
        tiktools_competitions::ActionStatus::Uncertain
    );
    assert!(db
        .resolve_uncertain_competition_action(
            "poll",
            1,
            tiktools_competitions::ActionResolution::Completed,
            128
        )
        .unwrap());
    assert!(!db
        .resolve_uncertain_competition_action(
            "poll",
            1,
            tiktools_competitions::ActionResolution::Failed,
            129
        )
        .unwrap());
    assert_eq!(
        db.competition_action("poll", 1).unwrap().unwrap().status,
        tiktools_competitions::ActionStatus::Completed
    );
    assert!(db
        .claim_competition_action("poll", 1, 130)
        .unwrap()
        .is_none());
    assert!(!db
        .complete_competition_action("poll", 1, true, 128)
        .unwrap());
    let reopened = DatabaseManager::new(paths.clone());
    assert_eq!(
        reopened.list_competition_results("poll", None, 2).unwrap(),
        vec![result.clone()]
    );
    assert!(reopened
        .list_competition_results("poll", Some(1), 2)
        .unwrap()
        .is_empty());
    assert!(reopened.list_competition_results("poll", None, 22).is_err());
    assert_eq!(
        reopened.load_competition_result("poll", 1).unwrap(),
        Some(result)
    );
    let _ = fs::remove_dir_all(paths.root);
}

#[test]
fn competition_definition_survives_reopen() {
    use tiktools_competitions::{
        CompetitionKind, Definition, OptionDefinition, SelectionMode, TiePolicy,
    };
    let paths = paths();
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    let definition = Definition {
        schema_version: 1,
        id: "poll".into(),
        name: "Poll".into(),
        kind: CompetitionKind::Poll,
        ranking_id: "rank".into(),
        options: vec![OptionDefinition {
            id: "a".into(),
            label: "A".into(),
            commands: vec!["!a".into()],
            gift_ids: vec![],
            image_url: None,
            on_win: None,
        }],
        selection_mode: SelectionMode::HighestScore,
        tie_policy: TiePolicy::Stable,
        duration_ms: Some(1000),
        vote_limit: 1,
        allow_switch: false,
        minimum_participants: 1,
        target_score: None,
        on_user_win: None,
    };
    db.insert_aggregation_definition(&tiktools_aggregation::model::Definition {
        schema_version: 1,
        id: "rank".into(),
        creator_id: "creator".into(),
        room_id: "room".into(),
        group_by: Default::default(),
        window: Default::default(),
        user_limits: None,
        sources: vec![tiktools_aggregation::model::SourceRule {
            source: "tiktok.chat".into(),
            metric: "event.count".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
        limit: 10,
        capacity: 100,
    })
    .unwrap();
    db.insert_competition_definition(&definition).unwrap();
    let reopened = DatabaseManager::new(paths.clone());
    assert_eq!(
        reopened.load_competition_definitions().unwrap(),
        vec![definition.clone()]
    );
    let service = crate::services::competitions::CompetitionService::restore(&reopened);
    assert!(!service.load_failed);
    assert_eq!(
        crate::recover_mutex(&service.command_index, "test").matching("creator", "room", "!A")[0]
            .competition_id,
        "poll"
    );
    assert!(crate::recover_mutex(&service.command_index, "test")
        .matching("creator", "other", "!A")
        .is_empty());
    assert_eq!(
        crate::recover_mutex(&service.definitions, "test").get("poll"),
        Some(&definition)
    );
    let mut updated = definition;
    updated.name = "Updated".into();
    assert!(reopened.update_competition_definition(&updated).unwrap());
    assert_eq!(
        reopened.load_competition_definitions().unwrap(),
        vec![updated]
    );
    assert!(reopened.delete_competition_definition("poll").unwrap());
    let _ = fs::remove_dir_all(paths.root);
}

#[test]
fn competition_deadline_expiry_is_durable_and_idempotent() {
    use tiktools_competitions::{
        CompetitionKind, Definition, Lifecycle, OptionDefinition, SelectionMode, TiePolicy,
        Transition,
    };
    let paths = paths();
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    let definition = Definition {
        schema_version: 1,
        id: "timer".into(),
        name: "Timer".into(),
        kind: CompetitionKind::Poll,
        ranking_id: "rank".into(),
        options: vec![OptionDefinition {
            id: "a".into(),
            label: "A".into(),
            commands: vec![],
            gift_ids: vec![],
            image_url: None,
            on_win: None,
        }],
        selection_mode: SelectionMode::HighestScore,
        tie_policy: TiePolicy::Stable,
        duration_ms: Some(50),
        vote_limit: 1,
        allow_switch: false,
        minimum_participants: 1,
        target_score: None,
        on_user_win: None,
    };
    db.insert_competition_definition(&definition).unwrap();
    db.transition_competition("timer", Transition::Start, 100)
        .unwrap();
    assert_eq!(db.expire_due_competitions(149).unwrap(), 0);
    assert_eq!(db.expire_due_competitions(150).unwrap(), 1);
    assert_eq!(db.expire_due_competitions(151).unwrap(), 0);
    let reopened = DatabaseManager::new(paths.clone());
    assert_eq!(
        reopened
            .load_competition_state("timer")
            .unwrap()
            .unwrap()
            .lifecycle,
        Lifecycle::Finalizing
    );
    let _ = fs::remove_dir_all(paths.root);
}

#[test]
fn competition_command_votes_are_transactional_scoped_and_replay_safe() {
    use tiktools_competitions::{
        CommandParticipation, CompetitionKind, Definition, OptionDefinition,
        ParticipationOutcome as Outcome, SelectionMode, TiePolicy, Transition,
    };
    let paths = paths();
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    let option = |id: &str| OptionDefinition {
        id: id.into(),
        label: id.into(),
        commands: vec![format!("!{id}")],
        gift_ids: vec![],
        image_url: None,
        on_win: None,
    };
    let definition = Definition {
        schema_version: 1,
        id: "poll".into(),
        name: "Poll".into(),
        kind: CompetitionKind::WeightedPoll,
        ranking_id: "rank".into(),
        options: vec![option("a"), option("b")],
        selection_mode: SelectionMode::HighestScore,
        tie_policy: TiePolicy::Stable,
        duration_ms: Some(100),
        vote_limit: 2,
        allow_switch: true,
        minimum_participants: 1,
        target_score: None,
        on_user_win: None,
    };
    db.insert_aggregation_definition(&tiktools_aggregation::model::Definition {
        schema_version: 1,
        id: "rank".into(),
        creator_id: "creator".into(),
        room_id: "room".into(),
        group_by: Default::default(),
        window: Default::default(),
        user_limits: None,
        sources: vec![tiktools_aggregation::model::SourceRule {
            source: "tiktok.chat".into(),
            metric: "event.count".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
        limit: 10,
        capacity: 100,
    })
    .unwrap();
    let input = |participant_id, event_identity, command, received_at| CommandParticipation {
        creator_id: "creator",
        room_id: "room",
        participant_id,
        event_identity,
        command,
        received_at,
    };
    db.insert_competition_definition(&definition).unwrap();
    assert_eq!(
        db.route_competition_command_with_previous(
            "poll",
            &input("u", "draft-command", "!a", 99),
            &mut None,
            Some("host-a")
        )
        .unwrap(),
        Some(Outcome::Closed)
    );
    assert_eq!(
        db.route_competition_score(
            "poll",
            &super::competitions::ScoredInput {
                creator_id: "creator",
                room_id: "room",
                participant_id: "u",
                event_identity: "draft-like",
                source: "tiktok.like",
                score: 1,
                received_at: 99
            },
            Some("host-a")
        )
        .unwrap(),
        Some(super::competitions::ScoredOutcome::Closed)
    );
    db.transition_competition_owned("poll", Transition::Start, 100, Some("host-a"))
        .unwrap();
    // An open generation fails closed on recovery. Only an explicit clean
    // checkpoint can be consumed once; a degraded checkpoint never clears.
    assert!(!db.begin_competition_recovery("poll", 1, "host-a").unwrap());
    db.save_competition_integrity(&[("poll".into(), 1, true)], "host-a")
        .unwrap();
    assert!(db.begin_competition_recovery("poll", 1, "host-a").unwrap());
    assert!(!db.begin_competition_recovery("poll", 1, "host-a").unwrap());
    db.save_competition_integrity(&[("poll".into(), 1, false)], "host-a")
        .unwrap();
    assert!(!db.begin_competition_recovery("poll", 1, "host-a").unwrap());
    // A delayed shutdown from an older host cannot certify a newer run.
    assert!(!db.begin_competition_recovery("poll", 1, "host-b").unwrap());
    assert!(db
        .route_competition_command_with_previous(
            "poll",
            &input("u", "stale", "!a", 101),
            &mut None,
            Some("host-a")
        )
        .is_err());
    assert!(db
        .transition_competition_owned("poll", Transition::Pause, 101, Some("host-a"))
        .is_err());
    assert_eq!(
        db.expire_due_competitions_owned(201, Some("host-a"))
            .unwrap(),
        0
    );
    assert!(db
        .save_competition_integrity(&[("poll".into(), 1, true)], "host-a")
        .is_err());
    db.save_competition_integrity(&[("poll".into(), 1, false)], "host-b")
        .unwrap();
    assert!(!db.begin_competition_recovery("poll", 1, "host-b").unwrap());
    // A failed contribution write must roll back both participant and counts.
    let connection = db.open(&db.automation_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_vote BEFORE INSERT ON competition_contributions BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    assert!(db
        .route_competition_command("poll", &input("u", "failed-event", "!a", 101))
        .is_err());
    let rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM competition_participants", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(rows, 0);
    let rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM competition_participation_counts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(rows, 0);
    connection
        .execute_batch("DROP TRIGGER reject_vote")
        .unwrap();
    drop(connection);
    let wrong_room = CommandParticipation {
        room_id: "other",
        ..input("u", "scope-event", "!a", 101)
    };
    assert_eq!(
        db.route_competition_command("poll", &wrong_room).unwrap(),
        None
    );
    let wrong_creator = CommandParticipation {
        creator_id: "other",
        ..input("u", "scope-event", "!a", 101)
    };
    assert_eq!(
        db.route_competition_command("poll", &wrong_creator)
            .unwrap(),
        None
    );
    let route = |user, identity, command, now| {
        db.route_competition_command("poll", &input(user, identity, command, now))
            .unwrap()
            .unwrap()
    };
    assert_eq!(
        route("u", "event-1", "!A", 101),
        Outcome::Accepted {
            option_id: "a".into(),
            votes: 1,
            switched: false
        }
    );
    assert_eq!(
        route("u", "event-1", "!a", 102),
        Outcome::Duplicate {
            option_id: "a".into()
        }
    );
    assert_eq!(
        route("other", "event-1", "!a", 102),
        Outcome::IdentityConflict
    );
    assert_eq!(
        route("u", "event-2", "!a extra", 103),
        Outcome::UnknownCommand
    );
    assert_eq!(
        route("u", "event-2", "!b", 104),
        Outcome::Accepted {
            option_id: "b".into(),
            votes: 2,
            switched: true
        }
    );
    assert_eq!(route("u", "event-3", "!b", 105), Outcome::VoteLimit);
    assert_eq!(
        route("u", "event-6", "!a", 106),
        Outcome::Accepted {
            option_id: "a".into(),
            votes: 2,
            switched: true
        }
    );
    assert_eq!(
        route("u", "event-6", "!a", 107),
        Outcome::Duplicate {
            option_id: "a".into()
        }
    );
    // This rejected event would otherwise switch the participant back to b.
    assert_eq!(
        route("u", "event-3", "!b", 108),
        Outcome::Duplicate {
            option_id: "b".into()
        }
    );
    db.transition_competition("poll", Transition::Pause, 110)
        .unwrap();
    assert_eq!(route("v", "event-4", "!a", 111), Outcome::Closed);
    // A failed ignored-event write cannot advance the bounded ledger count.
    let connection = db.open(&db.automation_path()).unwrap();
    let before: i64 = connection.query_row("SELECT contributions FROM competition_participation_counts WHERE competition_id='poll' AND generation=1", [], |row| row.get(0)).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_ignored BEFORE INSERT ON competition_contributions BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    assert!(db
        .route_competition_command("poll", &input("v", "failed-ignored", "!a", 112))
        .is_err());
    let after: i64 = connection.query_row("SELECT contributions FROM competition_participation_counts WHERE competition_id='poll' AND generation=1", [], |row| row.get(0)).unwrap();
    assert_eq!(before, after);
    connection
        .execute_batch("DROP TRIGGER reject_ignored")
        .unwrap();
    drop(connection);
    // Ignored identities survive reopening before the competition resumes.
    let during_pause = DatabaseManager::new(paths.clone());
    assert_eq!(
        during_pause
            .route_competition_command("poll", &input("v", "event-4", "!a", 112))
            .unwrap(),
        Some(Outcome::Duplicate {
            option_id: "a".into()
        })
    );
    db.transition_competition("poll", Transition::Resume, 120)
        .unwrap();
    assert_eq!(
        route("v", "event-4", "!a", 121),
        Outcome::Duplicate {
            option_id: "a".into()
        }
    );
    assert_eq!(
        route("v", "event-4-new", "!a", 122),
        Outcome::Accepted {
            option_id: "a".into(),
            votes: 1,
            switched: false
        }
    );
    assert_eq!(route("w", "event-5", "!a", 210), Outcome::Closed);
    let reopened = DatabaseManager::new(paths.clone());
    assert_eq!(
        reopened
            .route_competition_command("poll", &input("u", "event-1", "!a", 220))
            .unwrap(),
        Some(Outcome::Duplicate {
            option_id: "a".into()
        })
    );
    assert_eq!(
        reopened.competition_vote_totals("poll", 1).unwrap(),
        vec![("a".to_owned(), 3)]
    );
    let connection = reopened.open(&reopened.automation_path()).unwrap();
    let counts: (i64, i64) = connection.query_row("SELECT participants, contributions FROM competition_participation_counts WHERE competition_id='poll' AND generation=1", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    assert_eq!(counts, (2, 7));
    drop(connection);
    db.transition_competition("poll", Transition::Cancel, 130)
        .unwrap();
    assert!(db.delete_competition_definition("poll").unwrap());
    let mut recreated = definition;
    recreated.vote_limit = 1;
    db.insert_competition_definition(&recreated).unwrap();
    db.transition_competition("poll", Transition::Start, 250)
        .unwrap();
    assert!(matches!(
        db.route_competition_command("poll", &input("u", "one", "!a", 251))
            .unwrap(),
        Some(Outcome::Accepted { votes: 1, .. })
    ));
    assert_eq!(
        db.route_competition_command("poll", &input("u", "switch", "!b", 252))
            .unwrap(),
        Some(Outcome::Accepted {
            option_id: "b".into(),
            votes: 1,
            switched: true
        })
    );
    db.transition_competition("poll", Transition::Cancel, 253)
        .unwrap();
    assert!(db.delete_competition_definition("poll").unwrap());
    recreated.allow_switch = false;
    db.insert_competition_definition(&recreated).unwrap();
    db.transition_competition("poll", Transition::Start, 300)
        .unwrap();
    assert_eq!(
        db.route_competition_command("poll", &input("u", "event-1", "!a", 301))
            .unwrap(),
        Some(Outcome::Accepted {
            option_id: "a".into(),
            votes: 1,
            switched: false
        })
    );
    assert_eq!(
        db.route_competition_command("poll", &input("u", "new-event", "!b", 302))
            .unwrap(),
        Some(Outcome::SwitchForbidden)
    );
    assert_eq!(
        db.route_competition_command("poll", &input("u", "new-event", "!b", 303))
            .unwrap(),
        Some(Outcome::Duplicate {
            option_id: "b".into()
        })
    );
    // A scored event before selection is remembered but never credited,
    // including after the same participant subsequently selects an option.
    use super::competitions::{ScoredInput, ScoredOutcome};
    let scored = |participant_id, event_identity, source, score, received_at| ScoredInput {
        creator_id: "creator",
        room_id: "room",
        participant_id,
        event_identity,
        source,
        score,
        received_at,
    };
    assert_eq!(
        db.route_competition_score("poll", &scored("v", "like-1", "tiktok.like", 7, 304), None)
            .unwrap(),
        Some(ScoredOutcome::Unassigned)
    );
    assert!(matches!(
        db.route_competition_command("poll", &input("v", "vote-v", "!a", 305))
            .unwrap(),
        Some(Outcome::Accepted { .. })
    ));
    assert_eq!(
        db.route_competition_score("poll", &scored("v", "like-1", "tiktok.like", 7, 306), None)
            .unwrap(),
        Some(ScoredOutcome::Duplicate)
    );
    assert_eq!(
        db.route_competition_score("poll", &scored("v", "like-2", "tiktok.like", 9, 307), None)
            .unwrap(),
        Some(ScoredOutcome::Accepted {
            option_id: "a".into()
        })
    );
    assert_eq!(
        db.route_competition_score("poll", &scored("v", "like-2", "tiktok.like", 8, 308), None)
            .unwrap(),
        Some(ScoredOutcome::IdentityConflict)
    );
    db.transition_competition("poll", Transition::Pause, 309)
        .unwrap();
    assert_eq!(
        db.route_competition_score(
            "poll",
            &scored("v", "share-1", "tiktok.share", 4, 310),
            None
        )
        .unwrap(),
        Some(ScoredOutcome::Closed)
    );
    db.transition_competition("poll", Transition::Resume, 311)
        .unwrap();
    let reopened = DatabaseManager::new(paths.clone());
    assert_eq!(
        reopened
            .route_competition_score(
                "poll",
                &scored("v", "share-1", "tiktok.share", 4, 312),
                None
            )
            .unwrap(),
        Some(ScoredOutcome::Duplicate)
    );
    assert_eq!(
        reopened.competition_scored_totals("poll", 1).unwrap(),
        vec![("a".into(), 9)]
    );
    assert!(reopened
        .route_competition_score(
            "poll",
            &scored("v", "wrong-owner", "tiktok.like", 1, 313),
            Some("stale-host")
        )
        .is_err());
    let connection = db.open(&db.automation_path()).unwrap();
    let before: i64 = connection.query_row("SELECT contributions FROM competition_participation_counts WHERE competition_id='poll' AND generation=1", [], |row| row.get(0)).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_scored BEFORE INSERT ON competition_scored_contributions BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    assert!(db
        .route_competition_score(
            "poll",
            &scored("v", "failed-score", "tiktok.like", 1, 313),
            None
        )
        .is_err());
    let after: i64 = connection.query_row("SELECT contributions FROM competition_participation_counts WHERE competition_id='poll' AND generation=1", [], |row| row.get(0)).unwrap();
    assert_eq!(before, after);
    connection
        .execute_batch("DROP TRIGGER reject_scored")
        .unwrap();
    drop(connection);
    assert!(matches!(
        db.route_competition_score(
            "poll",
            &scored("v", "max-score", "tiktok.like", u64::MAX, 314),
            None
        )
        .unwrap(),
        Some(ScoredOutcome::Accepted { .. })
    ));
    assert!(db.competition_scored_totals("poll", 1).is_err());
    let _ = fs::remove_dir_all(paths.root);
}

#[test]
fn gift_combo_high_water_is_durable_scoped_and_transactional() {
    use super::gift_accounting::{advance_combo, GiftDelta};
    let paths = paths();
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    let mut connection = db.open(&db.automation_path()).unwrap();
    let tx = connection.transaction().unwrap();
    assert_eq!(
        advance_combo(&tx, "poll", 1, "u", "gift", "group", 2, Some(3)).unwrap(),
        GiftDelta {
            quantity: 2,
            diamonds: Some(6)
        }
    );
    tx.commit().unwrap();
    drop(connection);
    let reopened = DatabaseManager::new(paths.clone());
    let mut connection = reopened.open(&reopened.automation_path()).unwrap();
    let tx = connection.transaction().unwrap();
    assert_eq!(
        advance_combo(&tx, "poll", 1, "u", "gift", "group", 2, Some(3)).unwrap(),
        GiftDelta {
            quantity: 0,
            diamonds: Some(0)
        }
    );
    assert_eq!(
        advance_combo(&tx, "poll", 1, "u", "gift", "group", 4, None).unwrap(),
        GiftDelta {
            quantity: 2,
            diamonds: None
        }
    );
    assert_eq!(
        advance_combo(&tx, "poll", 1, "u", "gift", "group", 4, Some(3)).unwrap(),
        GiftDelta {
            quantity: 0,
            diamonds: Some(6)
        }
    );
    assert_eq!(
        advance_combo(&tx, "poll", 2, "u", "gift", "group", 1, Some(3)).unwrap(),
        GiftDelta {
            quantity: 1,
            diamonds: Some(3)
        }
    );
    tx.rollback().unwrap();
    let tx = connection.transaction().unwrap();
    assert_eq!(
        advance_combo(&tx, "poll", 1, "u", "gift", "group", 3, Some(3)).unwrap(),
        GiftDelta {
            quantity: 1,
            diamonds: Some(3)
        }
    );
    assert!(advance_combo(&tx, "poll", 1, "u", "gift", "other", u64::MAX, Some(2)).is_err());
    tx.rollback().unwrap();
    drop(connection);
    let _ = fs::remove_dir_all(paths.root);
}

#[test]
fn competition_gift_admission_commits_identity_combo_and_score_together() {
    use super::competitions::{GiftInput, GiftOutcome};
    use tiktools_competitions::{
        CompetitionKind, Definition, OptionDefinition, SelectionMode, TiePolicy, Transition,
    };
    let paths = paths();
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    db.insert_aggregation_definition(&tiktools_aggregation::model::Definition {
        schema_version: 1,
        id: "rank".into(),
        creator_id: "creator".into(),
        room_id: "room".into(),
        group_by: Default::default(),
        window: Default::default(),
        user_limits: None,
        sources: vec![tiktools_aggregation::model::SourceRule {
            source: "tiktok.gift".into(),
            metric: "gift.count".into(),
            weight: 1,
            enabled: true,
            gift_multipliers: Default::default(),
        }],
        limit: 10,
        capacity: 10,
    })
    .unwrap();
    db.insert_competition_definition(&Definition {
        schema_version: 1,
        id: "poll".into(),
        name: "Poll".into(),
        kind: CompetitionKind::Poll,
        ranking_id: "rank".into(),
        options: vec![OptionDefinition {
            id: "a".into(),
            label: "A".into(),
            commands: vec!["!a".into()],
            gift_ids: vec!["gift".into()],
            image_url: None,
            on_win: None,
        }],
        selection_mode: SelectionMode::HighestScore,
        tie_policy: TiePolicy::Stable,
        duration_ms: None,
        vote_limit: 1,
        allow_switch: false,
        minimum_participants: 0,
        target_score: None,
        on_user_win: None,
    })
    .unwrap();
    db.transition_competition_owned("poll", Transition::Start, 100, Some("owner"))
        .unwrap();
    let gift = |event_identity, observed_quantity, price| GiftInput {
        creator_id: "creator",
        room_id: "room",
        participant_id: "u",
        event_identity,
        gift_id: "gift",
        group_id: "combo",
        observed_quantity,
        price,
        received_at: 101,
    };
    let score = |quantity: u64, diamonds: u64, _: &str| Ok(quantity + diamonds);
    assert!(db
        .route_competition_gift("poll", &gift("late", 2, 3), "owner", score)
        .is_err());
    assert_eq!(
        db.route_competition_gift("poll", &gift("g0", 1, 3), "owner", score)
            .unwrap(),
        Some(GiftOutcome::Accepted {
            option_id: "a".into(),
            score: 4
        })
    );
    assert_eq!(
        db.route_competition_gift("poll", &gift("g1", 2, 3), "owner", score)
            .unwrap(),
        Some(GiftOutcome::Accepted {
            option_id: "a".into(),
            score: 4
        })
    );
    assert_eq!(
        db.route_competition_gift("poll", &gift("g1", 2, 3), "owner", score)
            .unwrap(),
        Some(GiftOutcome::Duplicate)
    );
    assert_eq!(
        db.route_competition_gift("poll", &gift("g1", 3, 3), "owner", score)
            .unwrap(),
        Some(GiftOutcome::IdentityConflict)
    );
    assert!(db
        .route_competition_gift("poll", &gift("g2", 4, 3), "owner", |_, _, _| Err(
            DatabaseError::Invalid("injected".into())
        ))
        .is_err());
    let reopened = DatabaseManager::new(paths.clone());
    assert_eq!(
        reopened
            .route_competition_gift("poll", &gift("g2", 4, 3), "owner", score)
            .unwrap(),
        Some(GiftOutcome::Accepted {
            option_id: "a".into(),
            score: 8
        })
    );
    assert_eq!(
        reopened
            .route_competition_gift("poll", &gift("final", 4, 3), "owner", score)
            .unwrap(),
        Some(GiftOutcome::Zero)
    );
    // A price change fails closed without advancing the combo or recording
    // the packet identity, allowing a corrected packet to be processed.
    assert!(reopened
        .route_competition_gift("poll", &gift("g3", 5, 4), "owner", score)
        .is_err());
    assert_eq!(
        reopened
            .route_competition_gift("poll", &gift("g3", 5, 3), "owner", score)
            .unwrap(),
        Some(GiftOutcome::Accepted {
            option_id: "a".into(),
            score: 4
        })
    );
    assert_eq!(
        reopened.competition_scored_totals("poll", 1).unwrap(),
        vec![("a".into(), 20)]
    );
    assert_eq!(
        reopened.competition_participant_count("poll", 1).unwrap(),
        1
    );
    // A direct gift does not select an option for future likes or commands.
    let connection = reopened.open(&reopened.automation_path()).unwrap();
    let selected: i64 = connection.query_row("SELECT COUNT(*) FROM competition_participants WHERE competition_id='poll' AND participant_id='u'", [], |row| row.get(0)).unwrap();
    assert_eq!(selected, 0);
    drop(connection);
    let command = tiktools_competitions::CommandParticipation {
        creator_id: "creator",
        room_id: "room",
        participant_id: "u",
        event_identity: "command",
        command: "!a",
        received_at: 102,
    };
    assert!(matches!(
        reopened
            .route_competition_command("poll", &command)
            .unwrap(),
        Some(tiktools_competitions::ParticipationOutcome::Accepted { .. })
    ));
    assert_eq!(
        reopened.competition_participant_count("poll", 1).unwrap(),
        1
    );

    let _ = fs::remove_dir_all(paths.root);
}
