use super::*;
use crate::db::DatabaseError;
use crate::services::effects::{ApprovedPointsKey, ApprovedPointsOperation, EffectExecution};

fn private_key() -> ApprovedPointsKey {
    EffectExecution::fresh()
        .unwrap()
        .points_key(
            ApprovedPointsOperation::Adjust,
            &crate::collaboration::hash_secret("saved points action"),
            &crate::collaboration::hash_secret("alice adjustment"),
        )
        .unwrap()
}

#[test]
fn approved_adjustment_commits_after_earlier_awards_and_before_later_awards() {
    let (database, service, root) = temp_service("approved-order");
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    let award = service
        .adjust_approved(&private_key(), "alice", 2.0, |effective, commit| {
            assert_eq!(effective, 2.0);
            commit()
        })
        .unwrap()
        .unwrap();
    assert_eq!(award.total_points, 3.0);
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        3.0
    );
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    service.shutdown();
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        4.0
    );
    assert_eq!(service.viewer("alice").unwrap()["points"], 4.0);
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn final_admission_can_reject_the_actual_rounded_delta_without_mutating_cache_or_database() {
    let (database, service, root) = temp_service("approved-reject");
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    let result = service.adjust_approved(&private_key(), "alice", 0.006, |effective, _commit| {
        assert!((effective - 0.01).abs() < 0.000001);
        Err(DatabaseError::Invalid(
            "final effective delta is outside the grant".into(),
        ))
    });
    assert!(result.is_err());
    service.shutdown();
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        1.0
    );
    assert_eq!(service.viewer("alice").unwrap()["points"], 1.0);
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn approved_adjustment_does_not_fabricate_a_missing_viewer_or_accept_an_uncommitted_callback() {
    let (database, service, root) = temp_service("approved-noop");
    assert!(matches!(
        service.adjust_approved(&private_key(), "missing", 1.0, |_, commit| commit()),
        Err(DatabaseError::NotFound)
    ));
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    assert!(service
        .adjust_approved(&private_key(), "alice", 1.0, |_, _| Ok(()))
        .is_err());
    service.shutdown();
    assert!(database.load_viewer_by_id("missing").unwrap().is_none());
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        1.0
    );
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn retained_adjustment_reauthorizes_original_effective_delta_without_overwriting_later_balance() {
    let (database, service, root) = temp_service("approved-retained");
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    let key = private_key();
    let award = service
        .adjust_approved(&key, "alice", 0.006, |effective, commit| {
            assert!((effective - 0.01).abs() < 0.000001);
            commit()
        })
        .unwrap()
        .unwrap();
    assert_eq!(award.total_points, 1.01);
    service.reset(Some("alice"));
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    let mut called = false;
    assert!(service
        .adjust_approved(&key, "alice", 0.006, |effective, commit| {
            called = true;
            assert!((effective - 0.01).abs() < 0.000001);
            commit()
        })
        .unwrap()
        .is_none());
    assert!(called, "retained success must pass final current admission");
    assert_eq!(service.viewer("alice").unwrap()["points"], 1.0);
    assert!(service
        .adjust_approved(&key, "alice", 0.006, |_, _| {
            Err(DatabaseError::Invalid("current author was revoked".into()))
        })
        .is_err());
    assert!(
        service
            .adjust_approved(&key, "alice", 0.006, |_, _| Ok(()))
            .is_err(),
        "retained callbacks must complete admission too"
    );
    service.shutdown();
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        1.0
    );
    let db = rusqlite::Connection::open(database.paths().points_database()).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM control_commands WHERE method='automation.points.adjust'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM control_command_outbox WHERE method='automation.points.adjust'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    drop(db);
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn clamped_zero_identity_cannot_debit_funds_awarded_later() {
    let (database, service, root) = temp_service("approved-zero");
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    service.reset(Some("alice"));
    let key = private_key();
    assert!(service
        .adjust_approved(&key, "alice", -5.0, |effective, commit| {
            assert_eq!(effective, 0.0);
            commit()
        })
        .unwrap()
        .is_some());
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    assert!(service
        .adjust_approved(&key, "alice", -5.0, |effective, commit| {
            assert_eq!(effective, 0.0, "reauthorize original effective delta");
            commit()
        })
        .unwrap()
        .is_none());
    assert_eq!(service.viewer("alice").unwrap()["points"], 1.0);
    service.shutdown();
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        1.0
    );
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ledger_failure_rolls_back_balance_revision_outbox_and_hot_cache() {
    let (database, service, root) = temp_service("approved-atomic");
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    wait_for_writer_idle(&service);
    let db = rusqlite::Connection::open(database.paths().points_database()).unwrap();
    let before: i64 = db.query_row("SELECT revision FROM control_resource_versions WHERE kind='viewer' AND resource_id='alice'", [], |row| row.get(0)).unwrap();
    db.execute_batch("CREATE TRIGGER reject_runtime_receipt BEFORE INSERT ON control_command_outbox WHEN NEW.method='automation.points.adjust' BEGIN SELECT RAISE(ABORT,'private fixture failure'); END").unwrap();
    let key = private_key();
    assert!(service
        .adjust_approved(&key, "alice", 5.0, |_, commit| commit())
        .is_err());
    assert_eq!(service.viewer("alice").unwrap()["points"], 1.0);
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        1.0
    );
    assert_eq!(db.query_row("SELECT revision FROM control_resource_versions WHERE kind='viewer' AND resource_id='alice'", [], |row| row.get::<_, i64>(0)).unwrap(), before);
    assert_eq!(
        db.query_row("SELECT count(*) FROM control_commands", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM control_resource_outbox", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        1
    );
    db.execute_batch("DROP TRIGGER reject_runtime_receipt")
        .unwrap();
    assert!(service
        .adjust_approved(&key, "alice", 5.0, |_, commit| commit())
        .unwrap()
        .is_some());
    assert_eq!(service.viewer("alice").unwrap()["points"], 6.0);
    drop(db);
    service.shutdown();
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn approved_adjustment_never_applies_subscriber_award_bonus() {
    let (database, service, root) = temp_service("approved-subscriber");
    service.update_config(serde_json::from_value(json!({"subBonusMultiplier":50.0})).unwrap());
    service
        .award_points(
            "alice",
            PointAction::Chat,
            AwardOptions {
                is_subscriber: Some(true),
                ..AwardOptions::default()
            },
        )
        .unwrap();
    let before = service.viewer("alice").unwrap()["points"].as_f64().unwrap();
    let award = service
        .adjust_approved(&private_key(), "alice", 2.0, |effective, commit| {
            assert_eq!(effective, 2.0);
            commit()
        })
        .unwrap()
        .unwrap();
    assert_eq!(award.delta, 2.0);
    assert_eq!(award.total_points, before + 2.0);
    service.shutdown();
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        before + 2.0
    );
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}
