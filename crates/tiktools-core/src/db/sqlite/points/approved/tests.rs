use super::*;
use crate::{
    collaboration::hash_secret,
    paths::AppPaths,
    services::effects::{ApprovedPointsOperation, EffectExecution},
};
use std::{path::PathBuf, time::Duration};

struct Fixture {
    db: DatabaseManager,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-approved-points-db-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            drop(Connection::open(path).unwrap());
        }
        let db = DatabaseManager::new(paths);
        let mut writer = db.open_points_writer().unwrap();
        db.apply_points_op(
            &mut writer,
            &PointsWriteOp::Save {
                seq: 1,
                viewer: ViewerState::from_value(&serde_json::json!({
                    "uniqueId":"alice","nickname":"Alice","points":10.0,"level":1,
                }))
                .unwrap(),
                reason: "manual",
                awarded: 10.0,
            },
        )
        .unwrap();
        Self { db, root }
    }
    fn connection(&self) -> Connection {
        self.db.open(&self.db.points_path()).unwrap()
    }
    fn counts(&self) -> (i64, i64, i64, i64) {
        let db = self.connection();
        (
            db.query_row("SELECT count(*) FROM control_commands", [], |row| {
                row.get(0)
            })
            .unwrap(),
            db.query_row("SELECT count(*) FROM control_command_outbox", [], |row| {
                row.get(0)
            })
            .unwrap(),
            db.query_row("SELECT count(*) FROM control_resource_outbox", [], |row| {
                row.get(0)
            })
            .unwrap(),
            db.query_row(
                "SELECT count(*) FROM control_command_resources",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        )
    }
    fn total(&self) -> f64 {
        self.db.load_viewer_by_id("alice").unwrap().unwrap()["points"]
            .as_f64()
            .unwrap()
    }
    fn apply(
        &self,
        key: &ApprovedPointsKey,
        delta: f64,
    ) -> Result<(Option<PointAward>, Option<ViewerState>), DatabaseError> {
        self.db.adjust_approved_points(
            key,
            "alice",
            delta,
            &PointsConfig::default(),
            Duration::from_millis(100),
            |_, commit| commit(),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn runtime_key() -> ApprovedPointsKey {
    EffectExecution::fresh()
        .unwrap()
        .points_key(
            ApprovedPointsOperation::Adjust,
            &hash_secret("exact approved saved action"),
            &hash_secret("rendered target and requested amount"),
        )
        .unwrap()
}

#[test]
fn business_receipt_and_both_outboxes_roll_back_together_on_storage_failure() {
    let f = Fixture::new();
    let key = runtime_key();
    f.connection()
        .execute_batch(
            "CREATE TRIGGER reject_runtime_outbox BEFORE INSERT ON control_command_outbox BEGIN SELECT RAISE(ABORT,'injected failure'); END;",
        )
        .unwrap();
    assert!(f.apply(&key, 2.0).is_err());
    assert_eq!(f.total(), 10.0);
    assert_eq!(f.counts(), (0, 0, 1, 0));
    f.connection()
        .execute_batch("DROP TRIGGER reject_runtime_outbox")
        .unwrap();
    assert!(f.apply(&key, 2.0).unwrap().0.is_some());
    assert_eq!(f.total(), 12.0);
    assert_eq!(f.counts(), (1, 1, 2, 1));
    assert!(f.apply(&key, 2.0).unwrap().0.is_none());
    assert_eq!(f.total(), 12.0);
    assert_eq!(f.counts(), (1, 1, 2, 1));
}

#[test]
fn swallowed_commit_failure_is_not_reported_as_a_fresh_adjustment() {
    let f = Fixture::new();
    let key = runtime_key();
    f.connection()
        .execute_batch(
            "CREATE TRIGGER reject_runtime_receipt BEFORE INSERT ON control_commands BEGIN SELECT RAISE(ABORT,'injected failure'); END;",
        )
        .unwrap();
    let result = f.db.adjust_approved_points(
        &key,
        "alice",
        2.0,
        &PointsConfig::default(),
        Duration::from_millis(100),
        |_, commit| {
            assert!(commit().is_err());
            Ok(())
        },
    );
    assert!(result.is_err());
    assert_eq!(f.total(), 10.0);
    assert_eq!(f.counts(), (0, 0, 1, 0));
}

#[test]
fn same_private_identity_cannot_change_sink_arguments_even_with_same_binding() {
    let f = Fixture::new();
    let key = runtime_key();
    f.apply(&key, 2.0).unwrap();
    assert!(matches!(
        f.apply(&key, 3.0),
        Err(DatabaseError::CommandIdReused)
    ));
    assert!(matches!(
        f.db.adjust_approved_points(
            &key,
            "another-viewer",
            2.0,
            &PointsConfig::default(),
            Duration::from_millis(100),
            |_, commit| commit(),
        ),
        Err(DatabaseError::CommandIdReused)
    ));
    assert_eq!(f.total(), 12.0);
    assert_eq!(f.counts(), (1, 1, 2, 1));
}

#[test]
fn expired_outcome_retains_identity_and_cannot_be_applied_again() {
    let f = Fixture::new();
    let key = runtime_key();
    f.apply(&key, 2.0).unwrap();
    f.connection()
        .execute(
            "UPDATE control_commands SET committed_at=0 WHERE principal_id=?1 AND command_id=?2",
            params![RUNTIME_NAMESPACE, key.id()],
        )
        .unwrap();
    assert!(matches!(
        f.apply(&key, 2.0),
        Err(DatabaseError::CommandExpired)
    ));
    assert_eq!(f.total(), 12.0);
    assert_eq!(f.counts(), (1, 1, 2, 1));
    let other = runtime_key();
    f.apply(&other, 1.0).unwrap();
    let tombstone: (bool, bool) = f
        .connection()
        .query_row(
            "SELECT outcome_expired,result_json IS NULL AND versions_json IS NULL FROM control_commands WHERE principal_id=?1 AND command_id=?2",
            params![RUNTIME_NAMESPACE,key.id()], |row| Ok((row.get(0)?,row.get(1)?)),
        )
        .unwrap();
    assert_eq!(tombstone, (true, true));
    assert!(matches!(
        f.apply(&key, 2.0),
        Err(DatabaseError::CommandExpired)
    ));
    assert_eq!(f.total(), 13.0);
    assert_eq!(f.counts(), (2, 2, 3, 2));
}

#[test]
fn malformed_or_oversized_retained_result_cannot_recover_or_mutate_state() {
    for malformed in [
        "null".to_owned(),
        "{\"effectiveDelta\":2.0,\"award\":{}}".to_owned(),
        "{\"effectiveDelta\":2.0,\"award\":{\"uniqueId\":\"mallory\",\"delta\":2.0,\"totalPoints\":12.0,\"level\":1,\"currencyName\":\"Gems\"}}".to_owned(),
        " ".repeat(RESULT_BYTES as usize + 1),
    ] {
        let f = Fixture::new();
        let key = runtime_key();
        f.apply(&key, 2.0).unwrap();
        f.connection()
            .execute(
                "UPDATE control_commands SET result_json=?3 WHERE principal_id=?1 AND command_id=?2",
                params![RUNTIME_NAMESPACE,key.id(),malformed],
            )
            .unwrap();
        assert!(matches!(
            f.apply(&key, 2.0),
            Err(DatabaseError::PointsStateInvalid)
        ));
        assert_eq!(f.total(), 12.0);
        assert_eq!(f.counts(), (1, 1, 2, 1));
    }
}

#[test]
fn oversized_metadata_and_malformed_hash_fail_before_generic_recovery() {
    for (column, malformed) in [
        ("method", "x".repeat(128 * 1024)),
        ("resource_kind", "x".repeat(128 * 1024)),
        ("resource_scope", "x".repeat(128 * 1024)),
        ("resource_id", "x".repeat(128 * 1024)),
        ("session_id", "x".repeat(128 * 1024)),
        ("host_instance_id", "x".repeat(128 * 1024)),
        ("payload_hash", "g".repeat(64)),
        ("payload_hash", "A".repeat(64)),
        ("payload_hash", "a".repeat(65)),
        (
            "payload_hash",
            format!("{}\0{}", "a".repeat(31), "a".repeat(32)),
        ),
    ] {
        let f = Fixture::new();
        let key = runtime_key();
        f.apply(&key, 2.0).unwrap();
        f.connection()
            .execute(
                &format!("UPDATE control_commands SET {column}=?3 WHERE principal_id=?1 AND command_id=?2"),
                params![RUNTIME_NAMESPACE,key.id(),malformed],
            )
            .unwrap();
        assert!(
            matches!(f.apply(&key, 2.0), Err(DatabaseError::PointsStateInvalid)),
            "stored {column} must fail preflight",
        );
        assert_eq!(f.total(), 12.0);
        assert_eq!(f.counts(), (1, 1, 2, 1));
    }
}
