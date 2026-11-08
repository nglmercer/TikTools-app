use super::*;
use crate::{
    collaboration::{AuditQuery, CommandRequest},
    control::WorkflowMutation,
    ipc::messages::HostMessage,
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    root: PathBuf,
}
fn paths(root: PathBuf) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    }
}
fn boot(root: PathBuf) -> Arc<AppCore> {
    Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        paths(root),
    ))
}
impl Fixture {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("tt-delivery-{}-{nonce:x}", std::process::id()));
        Self {
            core: Some(boot(root.clone())),
            root,
        }
    }
    fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
    fn audit(&self) -> Vec<crate::collaboration::AuditRecord> {
        self.core()
            .collaboration()
            .unwrap()
            .audit(AuditQuery {
                after: 0,
                limit: Some(100),
            })
            .unwrap()
            .into_iter()
            .filter_map(|entry| entry.details)
            .collect()
    }
    fn command(&self, actor: &str, id: &str, expected: &str, operation: WorkflowMutation) {
        self.core()
            .db
            .execute_workflow_command(
                &operation,
                &crate::db::CommandTransaction {
                    authorization: None,
                    principal_id: actor.into(),
                    session_id: format!("device-{actor}"),
                    request: CommandRequest {
                        command_id: id.into(),
                        host_instance_id: "instance-a".into(),
                        expected_revision: expected.into(),
                    },
                    current_instance_id: "instance-a".into(),
                    max_ids_per_principal: 100_000,
                },
            )
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.core.take());
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn graph() -> serde_json::Value {
    json!({"id":"welcome","name":"Private graph body must never enter audit or notifications","enabled":false,"nodes":[],"edges":[]})
}

#[test]
fn delivery_retry_after_ack_failure_preserves_ids_without_business_event_reexecution() {
    let f = Fixture::new();
    let mut domains = f.core().events.subscribe_domain();
    let mut stream = f.core().subscribe_coordination().unwrap();
    f.core().workflow_save(graph()).unwrap();
    assert!(matches!(
        domains.try_recv().unwrap(),
        crate::events::DomainEvent::WorkflowChanged { .. }
    ));
    let db = f.core().db.open(&f.core().db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_delivery_ack BEFORE INSERT ON control_outbox_delivery BEGIN SELECT RAISE(ABORT,'test'); END").unwrap();
    assert!(f.core().deliver_coordination().is_err());
    let first = stream.try_recv().unwrap();
    assert_eq!(f.audit().len(), 1);
    assert_eq!(f.core().db.pending_coordination().unwrap().len(), 1);
    assert!(matches!(
        domains.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));
    db.execute_batch("DROP TRIGGER fail_delivery_ack").unwrap();
    drop(db);
    assert_eq!(f.core().deliver_coordination().unwrap(), 1);
    assert_eq!(stream.try_recv().unwrap(), first);
    assert!(matches!(
        domains.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));
    assert_eq!(f.audit().len(), 1);
    assert!(f.core().db.pending_coordination().unwrap().is_empty());
    assert_eq!(
        f.core()
            .workflow_get_versioned("welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    assert!(!serde_json::to_string(&*first)
        .unwrap()
        .contains("Private graph"));
    assert!(!serde_json::to_string(&f.audit())
        .unwrap()
        .contains("Private graph"));
}

#[test]
fn audit_import_failure_prevents_publication_and_source_acknowledgement() {
    let f = Fixture::new();
    let mut stream = f.core().subscribe_coordination().unwrap();
    let mut domains = f.core().events.subscribe_domain();
    f.command("moderator", "once", "0", WorkflowMutation::Save(graph()));
    let service = f.core().collaboration().unwrap();
    // This migration-style trigger fails at the destination, after a source
    // business commit, so retry must copy metadata rather than execute again.
    let audit_db =
        rusqlite::Connection::open(f.root.join("data/collaboration/collaboration.db")).unwrap();
    audit_db.execute_batch("CREATE TRIGGER fail_delivery_import BEFORE INSERT ON audit_details BEGIN SELECT RAISE(ABORT,'test'); END").unwrap();
    assert!(f.core().deliver_coordination().is_err());
    assert!(matches!(
        stream.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    assert_eq!(f.core().db.pending_coordination().unwrap().len(), 2);
    assert!(f.audit().is_empty());
    audit_db
        .execute_batch("DROP TRIGGER fail_delivery_import")
        .unwrap();
    drop(audit_db);
    assert_eq!(f.core().deliver_coordination().unwrap(), 2);
    assert_eq!(f.audit().len(), 1);
    assert!(f.core().db.pending_coordination().unwrap().is_empty());
    assert!(matches!(
        domains.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));
    assert_eq!(
        f.core()
            .workflow_get_versioned("welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    drop(service);
}

#[test]
fn no_op_commands_do_not_steal_attribution_from_the_resource_change_and_backfill_is_safe() {
    let f = Fixture::new();
    f.command("author-a", "save", "0", WorkflowMutation::Save(graph()));
    f.command(
        "author-a",
        "delete",
        "1",
        WorkflowMutation::Delete("welcome".into()),
    );
    f.command(
        "other-author",
        "no-op",
        "2",
        WorkflowMutation::Delete("welcome".into()),
    );
    {
        let db = f.core().db.open(&f.core().db.automation_path()).unwrap();
        db.execute("DELETE FROM control_command_resources", [])
            .unwrap();
    }
    // Reopening an old source schema repairs only commands whose versions
    // advanced. The no-op shares revision 2 but did not produce its event.
    let reopened = DatabaseManager::new(paths(f.root.clone()));
    let pending = reopened.pending_coordination().unwrap();
    let resource_changes: Vec<_> = pending
        .iter()
        .filter_map(|item| match &item.event {
            CoordinationEvent::Invalidated { principal_id, .. } => Some(principal_id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(resource_changes, vec!["author-a", "author-a"]);
    assert_eq!(pending.len(), 5);
    assert_eq!(f.core().deliver_coordination().unwrap(), 5);
    assert_eq!(f.audit().len(), 3);
    assert!(reopened.pending_coordination().unwrap().is_empty());
    assert!(f
        .core()
        .workflow_get_versioned("welcome")
        .unwrap()
        .value
        .is_none());
}

#[test]
fn bounded_batches_give_both_queues_capacity_and_deduplicate_linked_audit_rows() {
    let f = Fixture::new();
    for index in 0..60 {
        f.command(
            "author-a",
            &format!("batch-{index}"),
            &index.to_string(),
            WorkflowMutation::Save(graph()),
        );
    }
    let first = f.core().db.pending_coordination().unwrap();
    assert_eq!(first.len(), 100);
    assert_eq!(
        first
            .iter()
            .filter(|item| matches!(item.token, DeliveryToken::Resource(_)))
            .count(),
        50
    );
    assert_eq!(
        first
            .iter()
            .filter(|item| matches!(item.token, DeliveryToken::Command(_)))
            .count(),
        50
    );
    assert_eq!(f.core().deliver_coordination().unwrap(), 100);
    assert_eq!(f.core().deliver_coordination().unwrap(), 20);
    assert_eq!(f.core().deliver_coordination().unwrap(), 0);
    assert_eq!(f.audit().len(), 60);
    assert_eq!(
        f.core()
            .workflow_get_versioned("welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "60"
    );
}

#[test]
fn source_retention_keeps_undelivered_records_and_prunes_only_acknowledged_metadata() {
    let f = Fixture::new();
    f.command("author-a", "old", "0", WorkflowMutation::Save(graph()));
    let db = f.core().db.open(&f.core().db.automation_path()).unwrap();
    let old = now() - 31 * 24 * 60 * 60 * 1000;
    db.execute("UPDATE control_resource_outbox SET committed_at=?1", [old])
        .unwrap();
    db.execute("UPDATE control_command_outbox SET committed_at=?1", [old])
        .unwrap();
    f.core().db.acknowledge_coordination(&[]).unwrap();
    assert_eq!(f.core().db.pending_coordination().unwrap().len(), 2);
    assert_eq!(f.core().deliver_coordination().unwrap(), 2);
    assert!(f.core().db.pending_coordination().unwrap().is_empty());
    assert!(f.audit().is_empty());
    assert_eq!(
        db.query_row("SELECT count(*) FROM control_resource_outbox", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM control_command_outbox", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    // Delivery retention never removes the business command deduplication ID.
    assert_eq!(
        db.query_row("SELECT count(*) FROM control_commands", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        f.core()
            .workflow_get_versioned("welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
}

#[test]
fn cleanup_waits_for_both_linked_deliveries_and_preserves_command_deduplication() {
    for resource_first in [true, false] {
        let f = Fixture::new();
        f.command("author-a", "recent", "0", WorkflowMutation::Save(graph()));
        let pending = f.core().db.pending_coordination().unwrap();
        let resource = pending
            .iter()
            .find(|item| matches!(item.token, DeliveryToken::Resource(_)))
            .unwrap()
            .token
            .clone();
        let command = pending
            .iter()
            .find(|item| matches!(item.token, DeliveryToken::Command(_)))
            .unwrap()
            .token
            .clone();
        let (first, second) = if resource_first {
            (resource, command)
        } else {
            (command, resource)
        };
        f.core().db.acknowledge_coordination(&[first]).unwrap();
        let remaining = f.core().db.pending_coordination().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].token, second);
        // Its exact attribution is still available after the other queue has
        // acknowledged. Neither side may delete a still-needed linked row.
        assert_eq!(remaining[0].audit.principal_id, "author-a");
        f.core()
            .db
            .acknowledge_coordination(std::slice::from_ref(&second))
            .unwrap();
        f.core().db.acknowledge_coordination(&[second]).unwrap();
        let db = f.core().db.open(&f.core().db.automation_path()).unwrap();
        for table in [
            "control_resource_outbox",
            "control_command_outbox",
            "control_outbox_delivery",
            "control_command_resources",
        ] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                0,
                "{table}"
            );
        }
        assert_eq!(
            db.query_row("SELECT count(*) FROM control_commands", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        // An actual retry after source cleanup returns the saved outcome,
        // retaining version 1 and producing no second delivery.
        f.command("author-a", "recent", "0", WorkflowMutation::Save(graph()));
        assert!(f.core().db.pending_coordination().unwrap().is_empty());
        assert_eq!(
            f.core()
                .workflow_get_versioned("welcome")
                .unwrap()
                .resource_versions[0]
                .revision,
            "1"
        );
    }
}

#[test]
fn synchronous_host_startup_uses_its_runtime_and_shutdown_joins_the_worker() {
    let f = Fixture::new();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    assert!(tokio::runtime::Handle::try_current().is_err());
    let mut stream = f.core().subscribe_coordination().unwrap();
    // Match Winit: constructing an API outside an entered runtime cannot
    // start background work until the host supplies its owned handle.
    f.core().start_coordination_worker();
    f.core().start_coordination_worker_on(runtime.handle());
    f.core().start_coordination_worker_on(runtime.handle());
    f.core().workflow_save(graph()).unwrap();
    runtime.block_on(async {
        let event = tokio::time::timeout(std::time::Duration::from_secs(3), stream.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(
            matches!(&*event,CoordinationEvent::Invalidated{resource,..} if resource.revision=="1")
        );
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while !f.core().db.pending_coordination().unwrap().is_empty() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(f.audit().len(), 1);
        f.core().shutdown().await;
        assert!(f.core().update_shutdown_safe());
    });
}

#[test]
fn automatic_point_delivery_does_not_create_an_identity_store_or_replay_business_events() {
    let f = Fixture::new();
    let mut stream = f.core().subscribe_coordination().unwrap();
    let mut domains = f.core().events.subscribe_domain();
    f.core()
        .points
        .award_points(
            "alice",
            crate::services::PointAction::Chat,
            crate::services::AwardOptions::default(),
        )
        .unwrap();
    f.core().points.shutdown();
    let expected = f.core().db.pending_coordination().unwrap();
    assert_eq!(expected.len(), 1);
    assert!(!expected[0].audit_required);
    assert_eq!(expected[0].audit.principal_id, "host-runtime");
    assert_eq!(f.core().deliver_coordination().unwrap(), 1);
    assert_eq!(*stream.try_recv().unwrap(), expected[0].event);
    assert!(!f.root.join("data/collaboration").exists());
    assert!(f.core().collaboration.lock().unwrap().is_none());
    assert!(matches!(
        domains.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 1.0);
    assert!(f.core().db.pending_coordination().unwrap().is_empty());
}

#[test]
fn invalid_owner_viewer_reset_cannot_reset_all_balances_or_publish_a_reset_event() {
    let f = Fixture::new();
    f.core().points_adjust("alice", 5.0).unwrap();
    let mut domains = f.core().events.subscribe_domain();
    for id in ["", " ", "@@", "*", "a\nb", &"x".repeat(129)] {
        f.core().points_reset(Some(id));
        assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 5.0);
        assert!(matches!(
            domains.try_recv(),
            Err(crate::events::DomainTryRecvError::Empty)
        ));
    }
    f.core().points_reset(Some(" @alice "));
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 0.0);
    assert!(
        matches!(domains.try_recv().unwrap(), crate::events::DomainEvent::PointsReset {unique_id:Some(id)} if id=="alice")
    );
    f.core().points_adjust("bob", 7.0).unwrap();
    domains.try_recv().unwrap();
    f.core().points.shutdown();
    f.core().points_reset(None);
    assert_eq!(f.core().points_viewer("bob").unwrap()["points"], 7.0);
    assert!(matches!(
        domains.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));
}

#[test]
fn points_and_workflow_cursors_have_independent_acknowledgements() {
    let f = Fixture::new();
    f.core().workflow_save(graph()).unwrap();
    f.core().points_adjust("alice", 2.0).unwrap();
    f.core().points.shutdown();
    let pending = f.core().db.pending_coordination().unwrap();
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].token, DeliveryToken::Resource(1));
    assert_eq!(pending[1].token, DeliveryToken::PointsResource(1));
    assert_ne!(pending[0].event, pending[1].event);
    f.core()
        .db
        .acknowledge_coordination(std::slice::from_ref(&pending[1].token))
        .unwrap();
    let remaining = f.core().db.pending_coordination().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].token, pending[0].token);
    assert_eq!(f.core().deliver_coordination().unwrap(), 1);
    assert_eq!(f.audit().len(), 1);
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 2.0);
}

#[test]
fn second_audit_chunk_failure_keeps_all_point_projections_unpublished_and_retry_deduplicates() {
    let f = Fixture::new();
    for _ in 0..150 {
        f.core().points_adjust("alice", 1.0).unwrap();
    }
    f.core().points.shutdown();
    let mut stream = f.core().subscribe_coordination().unwrap();
    let mut domains = f.core().events.subscribe_domain();
    let _service = f.core().collaboration().unwrap();
    let audit_db =
        rusqlite::Connection::open(f.root.join("data/collaboration/collaboration.db")).unwrap();
    audit_db.execute_batch("CREATE TRIGGER fail_later_point_audit BEFORE INSERT ON audit_details WHEN CAST(json_extract(NEW.details_json,'$.afterRevision') AS INTEGER)>100 BEGIN SELECT RAISE(ABORT,'second chunk'); END").unwrap();
    assert!(f.core().deliver_coordination().is_err());
    assert_eq!(f.audit().len(), 100);
    assert_eq!(f.core().db.pending_coordination().unwrap().len(), 150);
    assert!(matches!(
        stream.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    audit_db
        .execute_batch("DROP TRIGGER fail_later_point_audit")
        .unwrap();
    assert_eq!(f.core().deliver_coordination().unwrap(), 150);
    assert_eq!(
        audit_db
            .query_row("SELECT count(*) FROM audit_details", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        150
    );
    for _ in 0..150 {
        stream.try_recv().unwrap();
    }
    assert!(matches!(
        domains.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));
    assert!(f.core().db.pending_coordination().unwrap().is_empty());
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 150.0);
}

#[test]
fn point_ack_failure_after_workflow_ack_replays_only_point_metadata_with_the_original_id() {
    let f = Fixture::new();
    f.core().workflow_save(graph()).unwrap();
    f.core()
        .points
        .award_points(
            "alice",
            crate::services::PointAction::Chat,
            crate::services::AwardOptions::default(),
        )
        .unwrap();
    f.core().points.shutdown();
    let db = f.core().db.open(&f.core().db.points_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_point_ack BEFORE INSERT ON control_outbox_delivery BEGIN SELECT RAISE(ABORT,'test'); END").unwrap();
    let mut stream = f.core().subscribe_coordination().unwrap();
    let mut domains = f.core().events.subscribe_domain();
    assert!(f.core().deliver_coordination().is_err());
    let _workflow = stream.try_recv().unwrap();
    let point = stream.try_recv().unwrap();
    let remaining = f.core().db.pending_coordination().unwrap();
    assert_eq!(remaining.len(), 1);
    assert!(matches!(
        remaining[0].token,
        DeliveryToken::PointsResource(_)
    ));
    db.execute_batch("DROP TRIGGER fail_point_ack").unwrap();
    assert_eq!(f.core().deliver_coordination().unwrap(), 1);
    assert_eq!(stream.try_recv().unwrap(), point);
    assert_eq!(f.audit().len(), 1);
    assert!(matches!(
        domains.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 1.0);
}

#[test]
fn high_volume_points_keep_independent_capacity_for_workflow_and_command_delivery() {
    let f = Fixture::new();
    for index in 0..60 {
        f.command(
            "author-a",
            &format!("fair-{index}"),
            &index.to_string(),
            WorkflowMutation::Save(graph()),
        );
    }
    for _ in 0..600 {
        f.core()
            .points
            .award_points(
                "alice",
                crate::services::PointAction::Chat,
                crate::services::AwardOptions::default(),
            )
            .unwrap();
    }
    f.core().points.shutdown();
    let pending = f.core().db.pending_coordination().unwrap();
    assert_eq!(pending.len(), 356);
    assert_eq!(
        pending
            .iter()
            .filter(|item| matches!(item.token, DeliveryToken::Resource(_)))
            .count(),
        50
    );
    assert_eq!(
        pending
            .iter()
            .filter(|item| matches!(item.token, DeliveryToken::Command(_)))
            .count(),
        50
    );
    assert_eq!(
        pending
            .iter()
            .filter(|item| matches!(item.token, DeliveryToken::PointsResource(_)))
            .count(),
        256
    );
    assert_eq!(f.core().deliver_coordination().unwrap(), 356);
    assert_eq!(f.core().deliver_coordination().unwrap(), 276);
    assert_eq!(f.core().deliver_coordination().unwrap(), 88);
    assert_eq!(f.core().deliver_coordination().unwrap(), 0);
    assert_eq!(f.audit().len(), 60);
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 600.0);
}

#[test]
#[ignore = "subprocess-only delivery crash fixture"]
fn delivery_crash_child() {
    let root = PathBuf::from(std::env::var_os("TIKTOOLS_DELIVERY_CRASH_ROOT").expect("child root"));
    let core = boot(root);
    let _receiver = core.subscribe_coordination().unwrap();
    core.deliver_coordination().unwrap();
    panic!("delivery crash failpoint did not terminate");
}

#[test]
fn process_death_after_audit_or_publication_replays_metadata_without_reapplying_the_command() {
    for (stage, exit) in [("after_audit", 88), ("after_publish", 89)] {
        let f = Fixture::new();
        f.command("author-a", "crash", "0", WorkflowMutation::Save(graph()));
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "db::sqlite::delivery::tests::delivery_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("TIKTOOLS_DELIVERY_CRASH_ROOT", &f.root)
            .env("TIKTOOLS_DELIVERY_CRASH_STAGE", stage)
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert_eq!(f.audit().len(), 1);
        assert_eq!(f.core().db.pending_coordination().unwrap().len(), 2);
        let mut stream = f.core().subscribe_coordination().unwrap();
        let mut domains = f.core().events.subscribe_domain();
        let expected: Vec<_> = f
            .core()
            .db
            .pending_coordination()
            .unwrap()
            .into_iter()
            .map(|item| item.event)
            .collect();
        assert_eq!(f.core().deliver_coordination().unwrap(), 2);
        for event in expected {
            assert_eq!(*stream.try_recv().unwrap(), event);
        }
        assert_eq!(f.audit().len(), 1);
        assert!(f.core().db.pending_coordination().unwrap().is_empty());
        assert!(matches!(
            domains.try_recv(),
            Err(crate::events::DomainTryRecvError::Empty)
        ));
        assert_eq!(
            f.core()
                .workflow_get_versioned("welcome")
                .unwrap()
                .resource_versions[0]
                .revision,
            "1"
        );
    }
}
