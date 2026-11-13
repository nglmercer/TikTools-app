use super::*;
use crate::paths::AppPaths;

struct Fixture {
    db: DatabaseManager,
    root: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("tt-points-versions-{}-{nonce}", std::process::id()));
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
        // Even a direct test-executable launch from the checkout root must
        // not import the user's ./data legacy databases into this fixture.
        for file in [paths.points_database(), paths.automation_database()] {
            drop(rusqlite::Connection::open(file).unwrap());
        }
        Self {
            db: DatabaseManager::new(paths),
            root,
        }
    }
    fn revision(&self, id: &str) -> Option<i64> {
        use rusqlite::OptionalExtension;
        self.db.open(&self.db.points_path()).unwrap().query_row(
            "SELECT revision FROM control_resource_versions WHERE kind='viewer' AND scope='host' AND resource_id=?1", [id], |row| row.get(0)
        ).optional().unwrap()
    }
    fn outbox(&self) -> i64 {
        self.db
            .open(&self.db.points_path())
            .unwrap()
            .query_row("SELECT count(*) FROM control_resource_outbox", [], |row| {
                row.get(0)
            })
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn save(seq: u64, id: &str, total: f64, delta: f64) -> PointsWriteOp {
    PointsWriteOp::Save {
        seq,
        viewer: ViewerState::from_value(&serde_json::json!({
            "uniqueId":id,"nickname":"PRIVATE-VIEWER-NAME","points":total,"level":1,
            "totalChats":seq,"firstSeen":123,"lastSeen":456
        }))
        .unwrap(),
        reason: "chat",
        awarded: delta,
    }
}

#[test]
fn a_competing_writer_preserves_the_batch_until_the_lock_is_released() {
    let f = Fixture::new();
    let mut writer = f.db.open_points_writer().unwrap();
    writer.busy_timeout(std::time::Duration::ZERO).unwrap();
    let mut other = f.db.open(&f.db.points_path()).unwrap();
    let reservation = other
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    let batch = [save(1, "contended-viewer", 12.0, 12.0)];
    let error = f.db.apply_points_batch(&mut writer, &batch).unwrap_err();
    assert_eq!(
        error.sqlite_extended_code(),
        Some(rusqlite::ffi::SQLITE_BUSY)
    );
    assert_eq!(f.revision("contended-viewer"), None);
    assert_eq!(f.outbox(), 0);
    reservation.rollback().unwrap();
    f.db.apply_points_batch(&mut writer, &batch).unwrap();
    assert_eq!(f.revision("contended-viewer"), Some(1));
    assert_eq!(f.outbox(), 1);
}

#[test]
fn display_projection_is_explicit_bounded_and_omits_unselected_profile_fields() {
    use crate::collaboration::{Grant, ResourceKind, SNAPSHOT_BYTES};
    let f = Fixture::new();
    let mut writer = f.db.open_points_writer().unwrap();
    let ops: Vec<_> = (0..1005)
        .map(|n| save(n + 1, &format!("v-{n:04}"), n as f64, 1.0))
        .collect();
    f.db.apply_points_batch(&mut writer, &ops).unwrap();
    writer
        .execute(
            "UPDATE viewers SET avatar_url=?1 WHERE unique_id='v-1004'",
            ["PRIVATE".repeat(100_000)],
        )
        .unwrap();
    let grant = Grant::default();
    let view =
        f.db.points_display_snapshot(&grant, 20, Some("v-0000"), "Gems", 64 * 1024)
            .unwrap();
    assert_eq!(view.value["viewers"].as_array().unwrap().len(), 20);
    assert_eq!(view.value["viewers"][0]["uniqueId"], "v-1004");
    assert!(view.value["viewers"][0].get("avatarUrl").is_none());
    assert_eq!(view.value["selected"]["points"], 0.0);
    assert_eq!(view.resource_versions.len(), 21);
    assert_eq!(
        f.db.points_display_snapshot(&grant, 20, Some("v-1004"), "Gems", 64 * 1024)
            .unwrap()
            .resource_versions
            .len(),
        20
    );
    let missing =
        f.db.points_display_snapshot(&grant, 0, Some("new"), "Gems", 2048)
            .unwrap();
    assert!(missing.value["selected"].is_null());
    assert_eq!(missing.resource_versions[0].revision, "0");
    let projected =
        f.db.load_points_snapshot_limited(&grant, 2048, SNAPSHOT_BYTES, Some(20))
            .unwrap();
    assert_eq!(projected.len(), 20);
    assert!(f
        .db
        .load_points_snapshot(&grant, 2048, SNAPSHOT_BYTES)
        .is_err());
    let scope = Grant {
        resources: [(ResourceKind::Viewer, ["v-0000".into()].into())].into(),
        ..Default::default()
    };
    assert_eq!(
        f.db.points_display_snapshot(&scope, 20, None, "Gems", 2048)
            .unwrap()
            .value["viewers"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn recovery_snapshot_is_complete_scoped_and_bounded_before_loading_private_bodies() {
    use crate::collaboration::{Grant, ResourceKind, SNAPSHOT_BYTES};
    let f = Fixture::new();
    let mut writer = f.db.open_points_writer().unwrap();
    let ops: Vec<_> = (0..1_005)
        .map(|n| save(n + 1, &format!("viewer-{n:04}"), n as f64, 1.0))
        .collect();
    f.db.apply_points_batch(&mut writer, &ops).unwrap();
    let grant = Grant::default(); // Database reader is trusted; Core/Hub authorize.
    let records =
        f.db.load_points_snapshot(&grant, 2048, SNAPSHOT_BYTES)
            .unwrap();
    assert_eq!(records.len(), 1_005); // Never the public leaderboard's 1,000 limit.
    assert!(records
        .iter()
        .all(|(body, v)| body["uniqueId"] == v.id && v.revision == "1"));
    assert!(matches!(
        f.db.load_points_snapshot(&grant, 2048, 1024),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    writer
        .execute(
            "UPDATE viewers SET nickname=?1 WHERE unique_id='viewer-0000'",
            ["SECRET".repeat(100_000)],
        )
        .unwrap();
    let scoped = Grant {
        resources: [(ResourceKind::Viewer, ["viewer-0001".into()].into())].into(),
        ..Grant::default()
    };
    let records =
        f.db.load_points_snapshot(&scoped, 2048, SNAPSHOT_BYTES)
            .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].1.id, "viewer-0001");
    assert!(matches!(
        f.db.load_points_snapshot(&grant, 2048, SNAPSHOT_BYTES),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    assert!(f
        .db
        .load_points_snapshot(
            &Grant {
                resources: [(ResourceKind::Viewer, Default::default())].into(),
                ..Grant::default()
            },
            2048,
            SNAPSHOT_BYTES
        )
        .unwrap()
        .is_empty());
}

#[test]
fn batches_and_single_op_fallback_commit_matching_viewer_versions_and_invalidations() {
    let f = Fixture::new();
    let mut writer = f.db.open_points_writer().unwrap();
    f.db.apply_points_batch(
        &mut writer,
        &[
            save(1, "alice", 1.0, 1.0),
            save(2, "alice", 3.0, 2.0),
            PointsWriteOp::Adjust {
                seq: 3,
                unique_id: "alice".into(),
                points: 8.0,
                level: 1,
            },
            PointsWriteOp::Reset {
                seq: 4,
                unique_id: Some("alice".into()),
            },
        ],
    )
    .unwrap();
    assert_eq!(f.revision("alice"), Some(4));
    assert_eq!(f.outbox(), 4);
    f.db.apply_points_op(&mut writer, &save(5, "bob", 9.0, 9.0))
        .unwrap();
    f.db.apply_points_op(
        &mut writer,
        &PointsWriteOp::Reset {
            seq: 6,
            unique_id: None,
        },
    )
    .unwrap();
    assert_eq!(f.revision("alice"), Some(5));
    assert_eq!(f.revision("bob"), Some(2));
    assert_eq!(
        f.db.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        0.0
    );
    assert_eq!(
        f.db.load_viewer_by_id("bob").unwrap().unwrap()["points"],
        0.0
    );
    assert_eq!(f.outbox(), 7);
    let pending = f.db.pending_coordination().unwrap();
    assert_eq!(pending.len(), 7);
    assert_eq!(pending.iter().filter(|item| item.audit_required).count(), 3);
    assert!(pending
        .iter()
        .all(|item| matches!(item.token, super::super::DeliveryToken::PointsResource(_))));
    assert!(!format!("{pending:?}").contains("PRIVATE-VIEWER-NAME"));
    let tokens: Vec<_> = pending.iter().map(|item| item.token.clone()).collect();
    f.db.acknowledge_coordination(&tokens).unwrap();
    assert!(f.db.pending_coordination().unwrap().is_empty());
    assert_eq!(f.revision("alice"), Some(5));
}

#[test]
fn failed_outbox_reservation_rolls_back_every_balance_history_and_version_in_the_transaction() {
    let f = Fixture::new();
    let mut writer = f.db.open_points_writer().unwrap();
    writer.execute_batch("CREATE TRIGGER reject_point_projection BEFORE INSERT ON control_resource_outbox WHEN NEW.resource_id='bob' BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
    assert!(f
        .db
        .apply_points_batch(
            &mut writer,
            &[save(1, "alice", 1.0, 1.0), save(2, "bob", 5.0, 5.0)]
        )
        .is_err());
    assert!(f.db.load_viewer_by_id("alice").unwrap().is_none());
    assert!(f.db.load_viewer_by_id("bob").unwrap().is_none());
    assert_eq!(f.revision("alice"), None);
    assert_eq!(f.revision("bob"), None);
    assert_eq!(f.outbox(), 0);
    assert_eq!(
        writer
            .query_row("SELECT count(*) FROM points_transactions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(f
        .db
        .apply_points_op(&mut writer, &save(2, "bob", 5.0, 5.0))
        .is_err());
    assert!(f.db.load_viewer_by_id("bob").unwrap().is_none());
    writer
        .execute_batch("DROP TRIGGER reject_point_projection")
        .unwrap();
    f.db.apply_points_op(&mut writer, &save(1, "alice", 1.0, 1.0))
        .unwrap();
    assert_eq!(f.revision("alice"), Some(1));
    assert_eq!(f.outbox(), 1);
}

#[test]
fn revision_exhaustion_and_reset_failure_leave_previous_balances_and_revisions_unchanged() {
    let f = Fixture::new();
    let mut writer = f.db.open_points_writer().unwrap();
    f.db.apply_points_batch(
        &mut writer,
        &[save(1, "alice", 5.0, 5.0), save(2, "bob", 8.0, 8.0)],
    )
    .unwrap();
    writer
        .execute(
            "UPDATE control_resource_versions SET revision=?1 WHERE resource_id='bob'",
            [i64::MAX],
        )
        .unwrap();
    assert!(f
        .db
        .apply_points_op(
            &mut writer,
            &PointsWriteOp::Reset {
                seq: 3,
                unique_id: None
            }
        )
        .is_err());
    assert_eq!(
        f.db.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        5.0
    );
    assert_eq!(
        f.db.load_viewer_by_id("bob").unwrap().unwrap()["points"],
        8.0
    );
    assert_eq!(f.revision("alice"), Some(1));
    assert_eq!(f.revision("bob"), Some(i64::MAX));
    assert_eq!(f.outbox(), 2);
    assert!(f
        .db
        .apply_points_op(&mut writer, &save(4, "bob", 9.0, 1.0))
        .is_err());
    assert_eq!(
        f.db.load_viewer_by_id("bob").unwrap().unwrap()["points"],
        8.0
    );
}

#[test]
fn initialization_seeds_old_viewers_once_without_replaying_awards_or_changing_balance_history() {
    let f = Fixture::new();
    let db = f.db.open(&f.db.points_path()).unwrap();
    db.execute(
        "INSERT INTO viewers(unique_id,points,level,first_seen,last_seen) VALUES ('old',77,1,123,456)",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO points_transactions(unique_id,amount,reason) VALUES ('old',77,'manual')",
        [],
    )
    .unwrap();
    assert_eq!(f.revision("old"), None);
    for _ in 0..2 {
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert_eq!(
            reopened.load_viewer_by_id("old").unwrap().unwrap()["points"],
            77.0
        );
        assert_eq!(f.revision("old"), Some(1));
        assert_eq!(f.outbox(), 0);
    }
    assert_eq!(
        db.query_row("SELECT count(*) FROM points_transactions", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let mut writer = f.db.open_points_writer().unwrap();
    f.db.apply_points_op(&mut writer, &save(1, "old", 78.0, 1.0))
        .unwrap();
    let _ = DatabaseManager::new(f.db.paths().clone());
    assert_eq!(f.revision("old"), Some(2));
    assert_eq!(f.outbox(), 1);
}

#[test]
fn missing_viewer_adjustment_or_reset_never_fabricates_a_projection_resource() {
    let f = Fixture::new();
    let mut writer = f.db.open_points_writer().unwrap();
    f.db.apply_points_batch(
        &mut writer,
        &[
            PointsWriteOp::Adjust {
                seq: 1,
                unique_id: "missing".into(),
                points: 2.0,
                level: 1,
            },
            PointsWriteOp::Reset {
                seq: 2,
                unique_id: Some("missing".into()),
            },
            PointsWriteOp::Reset {
                seq: 3,
                unique_id: None,
            },
        ],
    )
    .unwrap();
    assert_eq!(f.revision("missing"), None);
    assert_eq!(f.outbox(), 0);
    assert!(f.db.load_viewer_by_id("missing").unwrap().is_none());
}

#[test]
fn projection_plans_bound_scoped_lookup_work_and_avoid_sorting_the_unrestricted_population() {
    let f = Fixture::new();
    let db = f.db.open_points_writer().unwrap();
    for scoped in [false, true] {
        let mut stmt = db
            .prepare(&format!(
                "EXPLAIN QUERY PLAN {}",
                projection::leaderboard_query(scoped)
            ))
            .unwrap();
        let ids = scoped.then_some("[\"alice\"]");
        let details = stmt
            .query_map(params![ids, "", 100], |r| r.get::<_, String>(3))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        if scoped {
            assert!(
                details
                    .iter()
                    .any(|d| d.contains("SEARCH viewers") && d.contains("unique_id=?")),
                "{details:?}"
            );
            assert!(
                !details.iter().any(|d| d.contains("SCAN viewers")),
                "{details:?}"
            );
        } else {
            assert!(
                details.iter().any(|d| d.contains("idx_viewers_rank")),
                "{details:?}"
            );
            assert!(
                !details.iter().any(|d| d.contains("TEMP B-TREE")),
                "{details:?}"
            );
        }
    }
    let grant = crate::collaboration::Grant::default();
    assert!(matches!(
        f.db.points_leaderboard_snapshot(&grant, 0, 0),
        Err(DatabaseError::SnapshotTooLarge)
    ));
}
