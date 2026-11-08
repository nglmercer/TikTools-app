use super::*;
use crate::paths::AppPaths;
struct Fixture {
    db: DatabaseManager,
    root: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-behavior-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent-builtin"),
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
        Self {
            db: DatabaseManager::new(paths),
            root,
        }
    }
    fn count(&self) -> i64 {
        self.db
            .open(&self.db.automation_path())
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM control_resource_outbox WHERE kind='automation'",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn action(id: &str) -> Value {
    json!({"id":id,"name":"Action","enabled":true,"typeId":"core.log","config":{"message":"one"}})
}
fn event(id: &str) -> Value {
    json!({"id":id,"name":"Event","enabled":true,"trigger":"tiktok.chat","actionIds":["same","same","other"]})
}

#[test]
fn migration_seeds_legacy_families_once_preserving_raw_payloads_and_retained_versions() {
    let f = Fixture::new();
    let c = f.db.open(&f.db.automation_path()).unwrap();
    // Model an existing installation before behavior metadata was introduced.
    let raw = "{\"id\":\"same\", \"name\":\"old JSON\", \"enabled\":true, \"custom\":{\"text\":\"keep\"}}";
    for table in ["behavior_actions", "behavior_events"] {
        c.execute(
            &format!("INSERT INTO {table}(id,name,enabled,payload_json,created_at,updated_at) VALUES ('same','SQL name',0,?1,11,22)"),
            [raw],
        ).unwrap();
    }
    c.execute("INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted) VALUES ('automation','action','same',7,0),('automation','event','removed',9,1)", []).unwrap();
    // This fixture represents a pre-authority installation. The one-time
    // authority marker cannot already exist in such a database.
    c.execute("DELETE FROM behavior_provenance_migration", [])
        .unwrap();
    drop(c);
    for _ in 0..2 {
        let reopened = DatabaseManager::new(f.db.paths().clone());
        let snapshot = reopened.load_behavior_snapshot_versioned().unwrap();
        assert_eq!(snapshot.value["actions"][0]["enabled"], false);
        assert_eq!(snapshot.value["events"][0]["enabled"], false);
        assert_eq!(snapshot.value["events"][0]["name"], "SQL name");
        for (table, revision) in [("behavior_actions", "7"), ("behavior_events", "1")] {
            assert_eq!(
                reopened
                    .behavior_get_versioned(table, "same")
                    .unwrap()
                    .resource_versions[0]
                    .revision,
                revision
            );
            let c = reopened.open(&reopened.automation_path()).unwrap();
            let row: (String, i64, i64) = c
                .query_row(
                    &format!(
                        "SELECT payload_json,created_at,updated_at FROM {table} WHERE id='same'"
                    ),
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .unwrap();
            assert_eq!(row, (raw.into(), 11, 22));
        }
        let missing = reopened
            .behavior_get_versioned("behavior_events", "removed")
            .unwrap();
        assert!(missing.value.is_none());
        assert_eq!(missing.resource_versions[0].revision, "9");
        let c = reopened.open(&reopened.automation_path()).unwrap();
        assert_eq!(c.query_row("SELECT deleted FROM control_resource_versions WHERE scope='event' AND resource_id='removed'", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
        assert_eq!(
            f.count(),
            0,
            "migration must not synthesize LIVE work or change notifications"
        );
    }
}

#[test]
fn scoped_versions_cas_and_delete_recreate_survive_reopen_without_changing_payloads() {
    let f = Fixture::new();
    let a =
        f.db.save_behavior_versioned(
            "behavior_actions",
            &action("same"),
            Some("0"),
            BehaviorSave::Create,
        )
        .unwrap();
    let e =
        f.db.save_behavior_versioned(
            "behavior_events",
            &event("same"),
            Some("0"),
            BehaviorSave::Create,
        )
        .unwrap();
    assert_eq!(a.value, action("same"));
    assert_eq!(a.resource_versions[0].scope, "action");
    assert_eq!(e.resource_versions[0].scope, "event");
    let before = f.count();
    assert!(matches!(
        f.db.save_behavior_versioned(
            "behavior_actions",
            &action("same"),
            Some("0"),
            BehaviorSave::Upsert
        ),
        Err(DatabaseError::RevisionConflict(_))
    ));
    assert_eq!(f.count(), before);
    f.db.delete_behavior_versioned("behavior_actions", "same", Some("1"))
        .unwrap();
    let missing =
        f.db.behavior_get_versioned("behavior_actions", "same")
            .unwrap();
    assert!(missing.value.is_none());
    assert_eq!(missing.resource_versions[0].revision, "2");
    assert!(f
        .db
        .save_behavior_versioned(
            "behavior_actions",
            &action("same"),
            Some("1"),
            BehaviorSave::Create
        )
        .is_err());
    let recreated =
        f.db.save_behavior_versioned(
            "behavior_actions",
            &action("same"),
            Some("2"),
            BehaviorSave::Create,
        )
        .unwrap();
    assert_eq!(recreated.resource_versions[0].revision, "3");
    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert_eq!(
        reopened
            .behavior_get_versioned("behavior_actions", "same")
            .unwrap()
            .resource_versions[0]
            .revision,
        "3"
    );
    assert!(matches!(
        f.db.save_behavior_versioned(
            "behavior_actions",
            &action("absent"),
            None,
            BehaviorSave::Update
        ),
        Err(DatabaseError::NotFound)
    ));
    assert!(f
        .db
        .save_behavior_versioned(
            "behavior_actions",
            &action("same"),
            None,
            BehaviorSave::Create
        )
        .is_err());
}

#[test]
fn cascade_is_atomic_advances_events_and_never_reenables_a_disabled_legacy_payload() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_actions", &action("same"))
        .unwrap();
    f.db.save_behavior("behavior_events", &event("event"))
        .unwrap();
    // Legacy toggles only changed the column; the old JSON can disagree.
    let c = f.db.open(&f.db.automation_path()).unwrap();
    c.execute("UPDATE behavior_events SET enabled=0 WHERE id='event'", [])
        .unwrap();
    let removed =
        f.db.delete_behavior_versioned("behavior_actions", "same", Some("1"))
            .unwrap();
    assert!(removed.value);
    assert_eq!(removed.resource_versions.len(), 2);
    assert_eq!(removed.resource_versions[0].scope, "action");
    assert_eq!(removed.resource_versions[1].scope, "event");
    assert_eq!(removed.resource_versions[1].id, "event");
    assert_eq!(removed.resource_versions[1].revision, "2");
    let e =
        f.db.behavior_get_versioned("behavior_events", "event")
            .unwrap();
    assert_eq!(e.resource_versions[0].revision, "2");
    assert_eq!(e.value.unwrap()["actionIds"], json!(["other"]));
    let snapshot = f.db.load_behavior_snapshot_versioned().unwrap();
    assert_eq!(snapshot.value["events"][0]["enabled"], false);
    assert_eq!(snapshot.value["events"][0]["actionIds"], json!(["other"]));
    assert_eq!(f.count(), 4);
    let pending = f.db.pending_coordination().unwrap();
    assert!(pending
        .iter()
        .any(|p| p.audit.method == "automation.references.update"
            && p.audit.resource.scope == "event"));
    f.db.save_behavior("behavior_actions", &action("same"))
        .unwrap();
    c.execute(
        "UPDATE behavior_events SET payload_json='malformed' WHERE id='event'",
        [],
    )
    .unwrap();
    let before = f.count();
    assert!(f
        .db
        .delete_behavior_versioned("behavior_actions", "same", Some("3"))
        .is_err());
    assert!(f
        .db
        .behavior_get_versioned("behavior_actions", "same")
        .unwrap()
        .value
        .is_some());
    assert_eq!(f.count(), before);
}

#[test]
fn bulk_both_families_rolls_back_every_write_when_one_version_or_outbox_fails() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_events", &event("event"))
        .unwrap();
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    let c = f.db.open(&f.db.automation_path()).unwrap();
    c.execute_batch("CREATE TRIGGER fail_action_outbox BEFORE INSERT ON control_resource_outbox WHEN NEW.kind='automation' AND NEW.scope='action' BEGIN SELECT RAISE(ABORT,'fixture failure'); END;").unwrap();
    assert!(f
        .db
        .set_behavior_enablement(&[
            ("behavior_events", "event", false),
            ("behavior_actions", "action", false)
        ])
        .is_err());
    assert_eq!(
        f.db.behavior_get_versioned("behavior_events", "event")
            .unwrap()
            .value
            .unwrap()["enabled"],
        true
    );
    assert_eq!(f.count(), 2);
    c.execute_batch("DROP TRIGGER fail_action_outbox").unwrap();
    let result =
        f.db.set_behavior_enablement(&[
            ("behavior_events", "event", false),
            ("behavior_actions", "action", false),
            ("behavior_events", "missing", true),
        ])
        .unwrap();
    assert_eq!(result.value, vec![("event".into(), "missing".into())]);
    assert_eq!(result.resource_versions.len(), 2);
    assert_eq!(f.count(), 4);
    for v in result.resource_versions {
        assert_eq!(v.revision, "2");
    }
    c.execute("UPDATE control_resource_versions SET revision=?1 WHERE kind='automation' AND scope='action' AND resource_id='action'",[i64::MAX]).unwrap();
    assert!(f
        .db
        .set_behavior_enabled_versioned("behavior_actions", "action", true, None)
        .is_err());
    assert_eq!(
        f.db.behavior_get_versioned("behavior_actions", "action")
            .unwrap()
            .value
            .unwrap()["enabled"],
        false
    );
    assert_eq!(f.count(), 4);
}

#[test]
fn bulk_missing_both_sidecars_rejects_without_owner_promotion_and_rolls_back_earlier_family() {
    for (valid_table, corrupt_table) in [
        ("behavior_actions", "behavior_events"),
        ("behavior_events", "behavior_actions"),
    ] {
        let f = Fixture::new();
        f.db.save_behavior_versioned(
            valid_table,
            &action("valid"),
            Some("0"),
            BehaviorSave::Create,
        )
        .unwrap();
        f.db.save_behavior_versioned(
            corrupt_table,
            &action("corrupt"),
            Some("0"),
            BehaviorSave::Create,
        )
        .unwrap();
        let c = f.db.open(&f.db.automation_path()).unwrap();
        let valid_scope = versions::scope(valid_table).unwrap();
        let corrupt_scope = versions::scope(corrupt_table).unwrap();
        c.execute("DELETE FROM control_resource_versions WHERE kind='automation' AND scope=?1 AND resource_id='corrupt'", [corrupt_scope]).unwrap();
        c.execute(
            "DELETE FROM behavior_provenance WHERE scope=?1 AND resource_id='corrupt'",
            [corrupt_scope],
        )
        .unwrap();
        // Do not mask synthesized Owner authority with a duplicate old outbox
        // UNIQUE error when the old implementation recreates revision one.
        c.execute("DELETE FROM control_resource_outbox WHERE kind='automation' AND scope=?1 AND resource_id='corrupt'", [corrupt_scope]).unwrap();
        let before = f.count();
        let before_row = |table: &str, id: &str| -> (i64, String, i64) {
            c.query_row(
                &format!("SELECT enabled,payload_json,updated_at FROM {table} WHERE id=?1"),
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap()
        };
        let valid_before = before_row(valid_table, "valid");
        let corrupt_before = before_row(corrupt_table, "corrupt");
        // The old current(...)=0 path succeeded here and minted Owner authority
        // for the corrupt live row after first changing the valid record.
        assert!(matches!(
            f.db.set_behavior_enablement(&[
                (valid_table, "valid", false),
                (corrupt_table, "corrupt", false),
            ]),
            Err(DatabaseError::Invalid(_))
        ));
        assert_eq!(before_row(valid_table, "valid"), valid_before);
        assert_eq!(before_row(corrupt_table, "corrupt"), corrupt_before);
        assert_eq!(f.count(), before);
        assert_eq!(versions::current(&c, valid_table, "valid").unwrap(), 1);
        assert_eq!(versions::current(&c, corrupt_table, "corrupt").unwrap(), 0);
        assert_eq!(c.query_row("SELECT count(*) FROM control_resource_versions WHERE kind='automation' AND scope=?1 AND resource_id='corrupt'",[corrupt_scope],|r|r.get::<_,i64>(0)).unwrap(),0);
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM behavior_provenance WHERE scope=?1 AND resource_id='corrupt'",
                [corrupt_scope],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        let retained:(String,i64)=c.query_row("SELECT authority,generation FROM behavior_provenance WHERE scope=?1 AND resource_id='valid'",[valid_scope],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(retained, ("owner".into(), 1));
    }
}

#[test]
fn competing_cas_saves_have_one_winner_and_consistent_snapshot_metadata() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for label in ["writer-a", "writer-b"] {
        let db = f.db.clone();
        let b = barrier.clone();
        workers.push(std::thread::spawn(move || {
            let mut record = action("action");
            record["name"] = label.into();
            b.wait();
            db.save_behavior_versioned("behavior_actions", &record, Some("1"), BehaviorSave::Update)
        }));
    }
    barrier.wait();
    let result: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(result.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(result
        .iter()
        .any(|r| matches!(r, Err(DatabaseError::RevisionConflict(_)))));
    let snapshot = f.db.load_behavior_snapshot_versioned().unwrap();
    assert_eq!(snapshot.resource_versions[0].revision, "2");
    assert_eq!(f.count(), 2);
}

#[test]
#[ignore = "subprocess-only behavior commit fixture"]
fn behavior_crash_child() {
    let path = std::path::PathBuf::from(std::env::var_os("TIKTOOLS_BEHAVIOR_ROOT").unwrap());
    let paths = AppPaths {
        data: path.join("data"),
        plugins: path.join("plugins"),
        plugin_data: path.join("plugin-data"),
        builtin_plugins: path.join("absent"),
        development_plugins: None,
        logs: path.join("logs"),
        temp: path.join("temp"),
        root: path,
    };
    let db = DatabaseManager::new(paths);
    db.delete_behavior_versioned("behavior_actions", "same", Some("1"))
        .unwrap();
    panic!("missing crash boundary");
}
#[test]
fn process_death_does_not_split_action_removal_reference_cleanup_versions_or_outboxes() {
    for stage in ["before", "after"] {
        let f = Fixture::new();
        f.db.save_behavior("behavior_actions", &action("same"))
            .unwrap();
        f.db.save_behavior("behavior_events", &event("event"))
            .unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "db::sqlite::behavior::tests::behavior_crash_child",
                "--ignored",
            ])
            .env("TIKTOOLS_BEHAVIOR_ROOT", &f.root)
            .env("TIKTOOLS_BEHAVIOR_CRASH", stage)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(result.code(), Some(if stage == "before" { 91 } else { 92 }));
        let a =
            f.db.behavior_get_versioned("behavior_actions", "same")
                .unwrap();
        let e =
            f.db.behavior_get_versioned("behavior_events", "event")
                .unwrap();
        assert_eq!(a.value.is_some(), stage == "before");
        assert_eq!(
            a.resource_versions[0].revision,
            if stage == "before" { "1" } else { "2" }
        );
        assert_eq!(
            e.resource_versions[0].revision,
            if stage == "before" { "1" } else { "2" }
        );
        assert_eq!(f.count(), if stage == "before" { 2 } else { 4 });
    }
}

#[test]
fn remote_summaries_never_materialize_payloads_and_keep_family_ids_versions_and_budgets() {
    use crate::collaboration::{AutomationFamily, Grant, ResourceKind};
    let f = Fixture::new();
    f.db.save_behavior("behavior_actions", &action("same"))
        .unwrap();
    f.db.save_behavior("behavior_events", &event("same"))
        .unwrap();
    f.db.save_behavior("behavior_events", &event("hidden"))
        .unwrap();
    let c = f.db.open(&f.db.automation_path()).unwrap();
    for table in ["behavior_actions", "behavior_events"] {
        // Malformed and oversized owner payloads cannot affect a status read.
        c.execute(
            &format!("UPDATE {table} SET payload_json=?1,name=?2 WHERE id='same'"),
            params!["owner-secret".repeat(100_000), "á".repeat(170)],
        )
        .unwrap();
    }
    let grant = Grant {
        resources: [(
            ResourceKind::Automation,
            ["event:same".into(), "action:same".into()].into(),
        )]
        .into(),
        ..Grant::default()
    };
    let rows =
        f.db.load_behavior_summaries(&grant, None, 4096, 8192)
            .unwrap();
    assert_eq!(rows.len(), 2);
    for (summary, version) in &rows {
        assert_eq!(summary.as_object().unwrap().len(), 5);
        assert_eq!(summary["id"], "same");
        assert_eq!(summary["name"].as_str().unwrap().chars().count(), 160);
        assert_eq!(summary["definitionRedacted"], true);
        assert_eq!(summary["kind"], version.scope);
        assert_eq!(version.revision, "1");
        assert!(!summary.to_string().contains("owner-secret"));
    }
    assert_ne!(rows[0].1.scope, rows[1].1.scope);
    assert!(matches!(
        f.db.load_behavior_summaries(&grant, None, 10, 8192),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    assert!(matches!(
        f.db.load_behavior_summaries(&grant, None, 4096, 10),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    let empty = Grant {
        resources: [(ResourceKind::Automation, Default::default())].into(),
        ..Grant::default()
    };
    assert!(f
        .db
        .load_behavior_summaries(&empty, None, 4096, 8192)
        .unwrap()
        .is_empty());
    let legacy = Grant {
        resources: [(ResourceKind::Automation, ["same".into()].into())].into(),
        ..Grant::default()
    };
    assert!(f
        .db
        .load_behavior_summaries(&legacy, None, 4096, 8192)
        .is_err());
    let one =
        f.db.behavior_summary(AutomationFamily::Action, "same", 4096)
            .unwrap();
    assert_eq!(one.value.unwrap()["kind"], "action");
    assert!(matches!(
        f.db.behavior_summary(AutomationFamily::Event, "same", 10),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    // Delete does not parse event payloads; missing summary retains its tombstone.
    f.db.delete_behavior_versioned("behavior_events", "same", Some("1"))
        .unwrap();
    let missing =
        f.db.behavior_summary(AutomationFamily::Event, "same", 4096)
            .unwrap();
    assert!(missing.value.is_none());
    assert_eq!(missing.resource_versions[0].revision, "2");
}

#[test]
#[ignore = "subprocess-only durable behavior commit fixture"]
fn behavior_command_crash_child() {
    use crate::{
        collaboration::CommandRequest,
        control::{AutomationKind, AutomationMutation},
    };
    let root = std::path::PathBuf::from(std::env::var_os("TIKTOOLS_BEHAVIOR_ROOT").unwrap());
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    };
    let db = DatabaseManager::new(paths);
    let command = super::super::CommandTransaction {
        principal_id: "local-owner".into(),
        session_id: "local-owner".into(),
        authorization: None,
        request: CommandRequest {
            command_id: "delete".into(),
            host_instance_id: "instance".into(),
            expected_revision: "1".into(),
        },
        current_instance_id: "instance".into(),
        max_ids_per_principal: 100,
    };
    db.execute_behavior_command(
        &AutomationMutation::Delete {
            kind: AutomationKind::Action,
            id: "same".into(),
        },
        &command,
    )
    .unwrap();
    panic!("missing command crash boundary");
}
#[test]
fn process_death_keeps_behavior_command_result_cascade_and_audit_outbox_atomic() {
    for stage in ["before", "after"] {
        let f = Fixture::new();
        f.db.save_behavior("behavior_actions", &action("same"))
            .unwrap();
        f.db.save_behavior("behavior_events", &event("event"))
            .unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "db::sqlite::behavior::tests::behavior_command_crash_child",
                "--ignored",
            ])
            .env("TIKTOOLS_BEHAVIOR_ROOT", &f.root)
            .env("TIKTOOLS_BEHAVIOR_CRASH", stage)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(result.code(), Some(if stage == "before" { 91 } else { 92 }));
        let db = DatabaseManager::new(f.db.paths().clone());
        let outcome = db.command_outcome("local-owner", "delete").unwrap();
        assert_eq!(outcome.is_some(), stage == "after");
        assert_eq!(
            db.behavior_get_versioned("behavior_actions", "same")
                .unwrap()
                .value
                .is_none(),
            stage == "after"
        );
        let event = db
            .behavior_get_versioned("behavior_events", "event")
            .unwrap();
        assert_eq!(
            event.resource_versions[0].revision,
            if stage == "before" { "1" } else { "2" }
        );
        let c = db.open(&db.automation_path()).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM control_command_outbox", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            if stage == "before" { 0 } else { 1 }
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM control_command_resources", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
            if stage == "before" { 0 } else { 1 }
        );
        if let Some(outcome) = outcome {
            assert_eq!(outcome.result, json!({"id":"same","deleted":true}));
            assert_eq!(outcome.resource_versions.len(), 2);
        }
    }
}
