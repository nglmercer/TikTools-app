//! Real process death exercises the balance/identity/outbox commit boundary.
use super::*;
use crate::{
    collaboration::hash_secret,
    db::DatabaseError,
    paths::AppPaths,
    services::effects::{ApprovedPointsKey, ApprovedPointsOperation, EffectExecution},
};
use std::path::PathBuf;
use tiktools_tiktok::events::{CanonicalLiveEvent, ChatEvent, EventMetadata};

const PRINCIPAL: &str = "__approved-points-runtime-v1";
const METHOD: &str = "automation.points.adjust";

fn paths(root: PathBuf) -> AppPaths {
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

fn key() -> ApprovedPointsKey {
    // Reconstruct a provider identity, never a JSON `id`, on each process.
    let event = crate::NativeLiveEvent {
        base: CanonicalLiveEvent::Chat(ChatEvent::default()),
        metadata: EventMetadata {
            method: "WebcastChatMessage".into(),
            msg_id: 4242,
            is_history: true,
        },
        gift: None,
    };
    let context = crate::LiveContext {
        unique_id: "crash-creator".into(),
        room_id: "crash-room".into(),
        connection_id: format!("process-{}", std::process::id()),
        creator_verified: true,
    };
    EffectExecution::native(&event, &context)
        .unwrap()
        .named_child("saved-event", "crash-rule")
        .unwrap()
        .named_child("saved-action", "crash-points")
        .unwrap()
        .child(0)
        .unwrap()
        .points_key(
            ApprovedPointsOperation::Adjust,
            &hash_secret("saved-approved-points-definition"),
            &hash_secret("alice-plus-two"),
        )
        .unwrap()
}

#[derive(Debug, PartialEq)]
struct Snapshot {
    points: f64,
    revision: i64,
    identities: i64,
    command_outbox: i64,
    resource_outbox: i64,
    links: i64,
}

fn snapshot(database: &DatabaseManager) -> Snapshot {
    let db = database.open_points_writer().unwrap();
    Snapshot {
        points: db
            .query_row("SELECT points FROM viewers WHERE unique_id='alice'", [], |row| {
                row.get(0)
            })
            .unwrap(),
        revision: db
            .query_row("SELECT revision FROM control_resource_versions WHERE kind='viewer' AND scope='host' AND resource_id='alice'", [], |row| row.get(0))
            .unwrap(),
        identities: db
            .query_row("SELECT count(*) FROM control_commands WHERE principal_id=?1 AND method=?2", [PRINCIPAL, METHOD], |row| row.get(0))
            .unwrap(),
        command_outbox: db
            .query_row("SELECT count(*) FROM control_command_outbox WHERE principal_id=?1 AND method=?2", [PRINCIPAL, METHOD], |row| row.get(0))
            .unwrap(),
        resource_outbox: db
            .query_row("SELECT count(*) FROM control_resource_outbox WHERE kind='viewer' AND resource_id='alice'", [], |row| row.get(0))
            .unwrap(),
        links: db
            .query_row("SELECT count(*) FROM control_command_resources l JOIN control_command_outbox c ON c.event_id=l.event_id WHERE c.principal_id=?1 AND c.method=?2", [PRINCIPAL, METHOD], |row| row.get(0))
            .unwrap(),
    }
}

fn initial() -> Snapshot {
    Snapshot {
        points: 1.0,
        revision: 1,
        identities: 0,
        command_outbox: 0,
        resource_outbox: 1,
        links: 0,
    }
}

fn committed() -> Snapshot {
    Snapshot {
        points: 3.0,
        revision: 2,
        identities: 1,
        command_outbox: 1,
        resource_outbox: 2,
        links: 1,
    }
}

fn seeded(tag: &str) -> PathBuf {
    let (database, service, root) = temp_service(tag);
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    wait_for_writer_idle(&service);
    service.shutdown();
    assert_eq!(snapshot(&database), initial());
    drop(service);
    drop(database);
    root
}

#[test]
#[ignore = "subprocess-only approved-points crash fixture"]
fn crash_child() {
    let root = PathBuf::from(
        std::env::var_os("TIKTOOLS_APPROVED_POINTS_CRASH_ROOT").expect("child fixture root"),
    );
    let database = Arc::new(DatabaseManager::new(paths(root)));
    drop(database.open_points_writer().unwrap());
    let service = PointsService::new(database);
    service
        .adjust_approved(&key(), "alice", 2.0, |effective, commit| {
            assert_eq!(effective, 2.0);
            commit()
        })
        .unwrap();
    panic!("approved-points failpoint did not terminate");
}

#[test]
fn process_death_keeps_approved_balance_identity_version_and_outboxes_atomic() {
    for (stage, exit_code) in [("before_commit", 98), ("after_commit", 99)] {
        let root = seeded(stage);
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "services::points::tests::approved_crash::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("TIKTOOLS_APPROVED_POINTS_CRASH_ROOT", &root)
            .env("TIKTOOLS_APPROVED_POINTS_CRASH_STAGE", stage)
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(exit_code),
            "stage {stage}: stdout={} stderr={}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr),
        );
        let database = Arc::new(DatabaseManager::new(paths(root.clone())));
        assert_eq!(
            snapshot(&database),
            if stage == "before_commit" {
                initial()
            } else {
                committed()
            },
        );
        drop(database.open_points_writer().unwrap());
        let service = PointsService::new(database.clone());
        let retry = service
            .adjust_approved(&key(), "alice", 2.0, |effective, commit| {
                assert_eq!(effective, 2.0);
                commit()
            })
            .unwrap();
        assert_eq!(retry.is_some(), stage == "before_commit");
        assert_eq!(snapshot(&database), committed());
        assert!(service
            .adjust_approved(&key(), "alice", 2.0, |effective, commit| {
                assert_eq!(effective, 2.0);
                commit()
            })
            .unwrap()
            .is_none());
        assert_eq!(snapshot(&database), committed());
        service.shutdown();
        drop(service);
        drop(database);
        let _ = std::fs::remove_dir_all(root);
    }
}

#[test]
fn expired_approved_identity_survives_reopen_without_mutation_or_outbox_replay() {
    let root = seeded("approved-expired");
    let database = Arc::new(DatabaseManager::new(paths(root.clone())));
    let service = PointsService::new(database.clone());
    assert!(service
        .adjust_approved(&key(), "alice", 2.0, |_, commit| commit())
        .unwrap()
        .is_some());
    service.shutdown();
    let db = database.open_points_writer().unwrap();
    let original_hash: String = db
        .query_row(
            "SELECT payload_hash FROM control_commands WHERE principal_id=?1 AND command_id=?2",
            [PRINCIPAL, key().id()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(original_hash.len(), 64);
    db.execute(
        "UPDATE control_commands SET committed_at=0 WHERE principal_id=?1 AND command_id=?2",
        [PRINCIPAL, key().id()],
    )
    .unwrap();
    database.prune_command_outcomes().unwrap();
    let tombstone: (String, Option<String>, Option<String>, bool) = db
        .query_row("SELECT payload_hash,result_json,versions_json,outcome_expired FROM control_commands WHERE principal_id=?1 AND command_id=?2", [PRINCIPAL,key().id()], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))
        .unwrap();
    assert_eq!(tombstone.0, original_hash);
    assert!(tombstone.1.is_none() && tombstone.2.is_none() && tombstone.3);
    assert_eq!(snapshot(&database), committed());
    drop(db);
    drop(service);
    drop(database);

    let reopened = Arc::new(DatabaseManager::new(paths(root.clone())));
    let service = PointsService::new(reopened.clone());
    assert!(matches!(
        service.adjust_approved(&key(), "alice", 2.0, |_, commit| commit()),
        Err(DatabaseError::CommandExpired)
    ));
    assert_eq!(snapshot(&reopened), committed());
    service.shutdown();
    drop(service);
    drop(reopened);
    let _ = std::fs::remove_dir_all(root);
}
