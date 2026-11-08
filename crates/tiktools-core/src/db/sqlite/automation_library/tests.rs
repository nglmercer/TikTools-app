use super::*;
use crate::{
    collaboration::CommandRequest,
    control::{
        owner_library_definition_digest, owner_library_digest, owner_library_versions_digest,
        AutomationKind, OwnerLibraryEntry, OwnerLibraryMutation, OwnerLibraryProfile,
        OwnerLibraryReceipt, OwnerLibrarySubmit,
    },
    db::{BehaviorSave, CommandTransaction},
    paths::AppPaths,
};
use std::path::PathBuf;

struct Fixture {
    db: DatabaseManager,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-library-db-{}-{}",
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
        Self {
            db: DatabaseManager::new(paths),
            root,
        }
    }
    fn connection(&self) -> Connection {
        self.db.open(&self.db.automation_path()).unwrap()
    }
    fn legacy(&self, key: &str, value: &str) {
        self.db.open(&self.db.points_path()).unwrap().execute(
            "INSERT INTO app_state(key,value,updated_at) VALUES (?1,?2,1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key,value],
        ).unwrap();
    }
    fn snapshot(&self) -> (OwnerLibraryCatalog, String, Versioned<Value>) {
        self.db.automation_library_snapshot().unwrap()
    }
    fn input(&self, mutation: OwnerLibraryMutation) -> OwnerLibrarySubmit {
        let (_, revision, definitions) = self.snapshot();
        OwnerLibrarySubmit {
            mutation,
            expected_revision: revision,
            definition_digest: owner_library_definition_digest(
                &definitions.value,
                &definitions.resource_versions,
            ),
            host_id: "host".into(),
            host_instance_id: "instance".into(),
        }
    }
    fn execute(
        &self,
        input: &OwnerLibrarySubmit,
        serial: u128,
    ) -> Result<(Versioned<Value>, bool), DatabaseError> {
        self.db.execute_automation_library_command(
            input,
            &CommandTransaction {
                principal_id: "local-owner".into(),
                session_id: "local-owner".into(),
                authorization: None,
                request: CommandRequest {
                    command_id: format!("owner-library-{serial:032x}"),
                    host_instance_id: input.host_instance_id.clone(),
                    expected_revision: input.expected_revision.clone(),
                },
                current_instance_id: "instance".into(),
                max_ids_per_principal: 10_000,
            },
        )
    }
    fn apply(&self, mutation: OwnerLibraryMutation, serial: u128) -> Versioned<Value> {
        let input = self.input(mutation);
        let (result, fresh) = self.execute(&input, serial).unwrap();
        assert!(fresh);
        let receipt: OwnerLibraryReceipt = serde_json::from_value(result.value.clone()).unwrap();
        let (catalog, revision, definitions) = self.snapshot();
        assert_eq!(receipt.revision, revision);
        assert_eq!(
            receipt.catalog_digest,
            owner_library_digest(&json!(catalog))
        );
        assert_eq!(
            receipt.definition_digest,
            owner_library_definition_digest(&definitions.value, &definitions.resource_versions)
        );
        assert_eq!(
            receipt.versions_digest,
            owner_library_versions_digest(&result.resource_versions)
        );
        assert_eq!(
            receipt.affected_resources as usize,
            result.resource_versions.len()
        );
        assert_eq!(result.resource_versions[0].scope, "library");
        assert_eq!(result.resource_versions[0].id, "catalog");
        assert!(result
            .resource_versions
            .iter()
            .skip(1)
            .map(|v| (&v.scope, &v.id))
            .is_sorted());
        result
    }
    fn save(&self, kind: AutomationKind, id: &str) {
        self.db
            .save_behavior_versioned(kind.table(), &record(id), Some("0"), BehaviorSave::Create)
            .unwrap();
    }
    fn body(&self, kind: AutomationKind, id: &str) -> Versioned<Option<Value>> {
        self.db.behavior_get_versioned(kind.table(), id).unwrap()
    }
    fn counts(&self) -> (i64, i64, i64, i64, i64, i64) {
        self.connection().query_row("SELECT (SELECT count(*) FROM behavior_actions),(SELECT count(*) FROM behavior_events),(SELECT count(*) FROM behavior_provenance),(SELECT count(*) FROM control_resource_versions),(SELECT count(*) FROM control_resource_outbox),(SELECT count(*) FROM control_commands)", [], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn record(id: &str) -> Value {
    json!({"id":id,"name":format!("Record {id}"),"enabled":false,"typeId":"core.log","config":{"message":"private-source"}})
}
fn entry(action: &str, event: &str) -> OwnerLibraryEntry {
    OwnerLibraryEntry {
        actions: vec![record(action)],
        event: json!({"id":event,"name":format!("Event {event}"),"enabled":false,"trigger":"test","actionIds":[action]}),
    }
}
fn template(id: &str) -> Value {
    json!({"templateVersion":1,"id":id,"title":"Template","actions":[{"name":"Log","typeId":"core.log","config":{"message":"private-template-source"}}],"event":{"name":"Gift","trigger":"tiktok.gift"}})
}
fn profile(id: &str) -> OwnerLibraryMutation {
    OwnerLibraryMutation::CreateProfile {
        id: id.into(),
        name: format!("Profile {id}"),
        description: "Description".into(),
    }
}
fn apply(entry: OwnerLibraryEntry, profile_id: &str) -> OwnerLibraryMutation {
    OwnerLibraryMutation::ApplyTemplate {
        entry,
        profile_id: profile_id.into(),
        additional_entries: None,
        templates: None,
    }
}

#[test]
fn migration_is_copy_once_revision_zero_and_preserves_unicode_and_points_source() {
    let f = Fixture::new();
    assert!(!f.db.automation_library_exists().unwrap());
    assert!(!f.db.automation_library_legacy_exists().unwrap());
    let packs =
        json!([{"id":"ñ","name":"Perfil ñ","description":"","actionIds":["absent"],"eventIds":[]}])
            .to_string();
    f.legacy(PACKS_KEY, &packs);
    assert!(f.db.automation_library_legacy_exists().unwrap());
    f.legacy(ACTIVE_KEY, "ñ");
    f.legacy(
        TEMPLATES_KEY,
        &json!({"templates":[template("legacy-template")]}).to_string(),
    );
    f.legacy("unrelated.secret", "DO-NOT-EXPORT");
    let (catalog, revision, definitions) = f.snapshot();
    assert_eq!(revision, "0");
    assert_eq!(catalog.active_id, "ñ");
    assert_eq!(catalog.packs[0].id, "ñ");
    assert_eq!(catalog.templates.len(), 1);
    assert!(definitions.resource_versions.is_empty());
    assert_eq!(f.counts(), (0, 0, 0, 0, 0, 0));
    f.legacy(PACKS_KEY, "CORRUPTED-AFTER-COPY");
    assert_eq!(f.snapshot().0, catalog);
    let values = f.db.automation_library_legacy_state().unwrap();
    assert_eq!(values.len(), 3);
    assert!(!format!("{values:?}").contains("DO-NOT-EXPORT"));
    assert_eq!(values[ACTIVE_KEY], "ñ");
    assert_eq!(
        f.db.automation_library_legacy_get("unrelated.secret")
            .unwrap(),
        None
    );
    let source: String =
        f.db.open(&f.db.points_path())
            .unwrap()
            .query_row(
                "SELECT value FROM app_state WHERE key=?1",
                [PACKS_KEY],
                |r| r.get(0),
            )
            .unwrap();
    assert_eq!(source, "CORRUPTED-AFTER-COPY");
}

#[test]
fn corrupt_legacy_values_and_duplicate_profiles_never_partially_initialize() {
    for (key,value) in [
        (PACKS_KEY,"{".to_owned()),
        (PACKS_KEY,json!([{"id":"a","name":"A","description":"","eventIds":[],"actionIds":[]},{"id":"a","name":"B","description":"","eventIds":[],"actionIds":[]}]).to_string()),
        (TEMPLATES_KEY,json!([template("valid-template"),{"templateVersion":9,"id":"invalid"}]).to_string()),
        (TEMPLATES_KEY,json!([template("same-template"),template("same-template")]).to_string()),
        (ACTIVE_KEY,"missing-profile".to_owned()),
    ] {
        let f=Fixture::new(); f.legacy(key,&value);
        assert!(f.db.ensure_automation_library().is_err());
        assert!(!f.db.automation_library_exists().unwrap());
        let counts:(i64,i64)=f.connection().query_row("SELECT (SELECT count(*) FROM automation_library_catalog),(SELECT count(*) FROM automation_library_migration)",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(counts,(0,0));
        let stored:String=f.db.open(&f.db.points_path()).unwrap().query_row("SELECT value FROM app_state WHERE key=?1",[key],|r|r.get(0)).unwrap();
        assert_eq!(stored,value);
    }
}

#[test]
fn closed_operations_and_noops_advance_catalog_once_with_exact_receipts() {
    let f = Fixture::new();
    f.apply(profile("a"), 1);
    let created = f.apply(apply(entry("same", "same"), "a"), 2);
    assert_eq!(
        created
            .resource_versions
            .iter()
            .map(|v| v.scope.as_str())
            .collect::<Vec<_>>(),
        ["library", "action", "event"]
    );
    f.apply(profile("b"), 3);
    f.apply(
        OwnerLibraryMutation::AssignRecord {
            kind: AutomationKind::Action,
            id: "same".into(),
            profile_id: "b".into(),
        },
        4,
    );
    assert!(f.snapshot().0.packs[1].action_ids.is_empty()); // Existing ownership never transfers implicitly.
    let switched = f.apply(OwnerLibraryMutation::SwitchProfile { id: "a".into() }, 5);
    assert_eq!(switched.resource_versions.len(), 3);
    assert_eq!(
        f.body(AutomationKind::Action, "same").value.unwrap()["enabled"],
        true
    );
    assert_eq!(
        f.body(AutomationKind::Event, "same").resource_versions[0].revision,
        "2"
    );
    f.apply(OwnerLibraryMutation::SwitchProfile { id: "a".into() }, 6);
    assert_eq!(
        f.body(AutomationKind::Action, "same").resource_versions[0].revision,
        "3"
    );
    f.apply(
        OwnerLibraryMutation::ImportTemplates {
            templates: vec![template("custom-one")],
        },
        7,
    );
    let mut replacement = template("custom-one");
    replacement["title"] = "Replacement".into();
    f.apply(
        OwnerLibraryMutation::ImportTemplates {
            templates: vec![replacement],
        },
        8,
    );
    assert_eq!(f.snapshot().0.templates.len(), 1);
    assert_eq!(f.snapshot().0.templates[0]["title"], "Replacement");
    f.apply(
        OwnerLibraryMutation::DeleteTemplate {
            id: "custom-one".into(),
        },
        9,
    );
    f.apply(
        OwnerLibraryMutation::DeleteTemplate {
            id: "custom-one".into(),
        },
        10,
    );
    f.apply(
        OwnerLibraryMutation::ImportProfile {
            profile: OwnerLibraryProfile {
                id: "b".into(),
                name: "Imported".into(),
                description: "".into(),
            },
            entries: vec![entry("new-a", "new-e")],
            templates: None,
        },
        11,
    );
    assert_eq!(f.snapshot().0.active_id, "b");
    f.apply(OwnerLibraryMutation::DeleteProfile { id: "b".into() }, 12);
    assert_eq!(f.snapshot().1, "12");
    assert_eq!(f.snapshot().0.active_id, "default");
    assert!(f.body(AutomationKind::Action, "new-a").value.is_none());
    assert!(f.db.load_behavior_runtime_snapshot().is_ok()); // Catalog metadata has no fictitious behavior authority.
}

#[test]
fn catalog_cas_and_universe_digest_detect_new_deleted_and_changed_definitions() {
    let f = Fixture::new();
    let stale = f.input(profile("stale"));
    f.apply(profile("first"), 1);
    assert!(
        matches!(f.execute(&stale,2),Err(DatabaseError::RevisionConflict(v)) if v.scope=="library" && v.revision=="1")
    );
    for change in 0..3 {
        let reviewed = f.input(profile(&format!("conflict-{change}")));
        if change == 0 {
            f.save(AutomationKind::Action, "external");
        } else if change == 1 {
            f.db.set_behavior_enabled_versioned("behavior_actions", "external", true, Some("1"))
                .unwrap();
        } else {
            f.db.delete_behavior_versioned("behavior_actions", "external", Some("2"))
                .unwrap();
        }
        assert!(matches!(
            f.execute(&reviewed, 10 + change),
            Err(DatabaseError::LibraryDefinitionConflict)
        ));
        assert_eq!(f.snapshot().1, "1");
    }
}

#[test]
fn graph_existing_and_tombstone_collisions_roll_back_every_store() {
    let f = Fixture::new();
    f.save(AutomationKind::Event, "collision");
    for tombstone in [false, true] {
        if tombstone {
            f.db.delete_behavior_versioned("behavior_events", "collision", Some("1"))
                .unwrap();
        }
        let input = f.input(apply(entry("first-new-action", "collision"), "default"));
        let before = f.counts();
        assert!(matches!(
            f.execute(&input, 1 + u128::from(tombstone)),
            Err(DatabaseError::RevisionConflict(_))
        ));
        assert_eq!(f.counts(), before);
        assert!(f
            .body(AutomationKind::Action, "first-new-action")
            .value
            .is_none());
        assert_eq!(f.snapshot().1, "0");
    }
}

#[test]
fn compound_graph_gallery_import_is_atomic_and_replacing_profile_leaves_old_graph_default() {
    let f = Fixture::new();
    f.apply(profile("p"), 1);
    f.apply(apply(entry("old-a", "old-e"), "p"), 2);
    let input = f.input(OwnerLibraryMutation::ApplyTemplate {
        entry: entry("new-a", "new-e"),
        profile_id: "p".into(),
        additional_entries: Some(vec![entry("extra-a", "extra-e")]),
        templates: Some(vec![template("same-template"), template("same-template")]),
    });
    let before = f.counts();
    assert!(f.execute(&input, 3).is_err());
    assert_eq!(f.counts(), before);
    assert!(f.body(AutomationKind::Action, "new-a").value.is_none());
    f.apply(
        OwnerLibraryMutation::ImportProfile {
            profile: OwnerLibraryProfile {
                id: "p".into(),
                name: "Replacement".into(),
                description: "".into(),
            },
            entries: vec![entry("new-a", "new-e"), entry("extra-a", "extra-e")],
            templates: Some(vec![template("saved-template")]),
        },
        4,
    );
    assert_eq!(f.snapshot().0.packs[0].action_ids, ["new-a", "extra-a"]);
    assert_eq!(f.snapshot().0.templates.len(), 1);
    assert!(f.body(AutomationKind::Action, "old-a").value.is_some());
    f.apply(
        OwnerLibraryMutation::SwitchProfile {
            id: "default".into(),
        },
        5,
    );
    assert_eq!(
        f.body(AutomationKind::Action, "old-a").value.unwrap()["enabled"],
        true
    );
    assert_eq!(
        f.body(AutomationKind::Action, "new-a").value.unwrap()["enabled"],
        false
    );
}

#[test]
fn delete_profile_coalesces_cascades_and_prunes_all_overlapping_or_absent_memberships() {
    let f = Fixture::new();
    f.apply(profile("p"), 1);
    f.apply(profile("other"), 2);
    f.apply(
        OwnerLibraryMutation::ApplyTemplate {
            entry: entry("a1", "deleted-event"),
            profile_id: "p".into(),
            additional_entries: Some(vec![entry("a2", "deleted-event2")]),
            templates: None,
        },
        3,
    );
    f.db.save_behavior_versioned("behavior_events",&json!({"id":"survivor","name":"Survivor","enabled":true,"actionIds":["a1","keep","a2","a1","already-absent"]}),Some("0"),BehaviorSave::Create).unwrap();
    let mut catalog = f.snapshot().0;
    catalog.packs[0].action_ids.push("already-absent".into());
    catalog.packs[1].action_ids = vec!["a1".into(), "already-absent".into()];
    catalog.packs[1].event_ids = vec!["deleted-event".into()];
    f.db.automation_library_legacy_set(
        "host",
        PACKS_KEY,
        &serde_json::to_string(&catalog.packs).unwrap(),
    )
    .unwrap();
    // The cascade must preserve trusted delegated identity rather than promote it.
    f.connection().execute("UPDATE behavior_provenance SET authority='delegated',principal_id='moderator',session_id='session' WHERE scope='event' AND resource_id='survivor'",[]).unwrap();
    let result = f.apply(OwnerLibraryMutation::DeleteProfile { id: "p".into() }, 4);
    assert_eq!(result.resource_versions.len(), 6); // catalog + two actions + three events.
    let survivor = f.body(AutomationKind::Event, "survivor");
    assert_eq!(survivor.resource_versions[0].revision, "2");
    assert_eq!(
        survivor.value.unwrap()["actionIds"],
        json!(["keep", "already-absent"])
    );
    assert_eq!(
        f.body(AutomationKind::Event, "deleted-event")
            .resource_versions[0]
            .revision,
        "2"
    );
    assert!(f.snapshot().0.packs[0].action_ids.is_empty());
    assert!(f.snapshot().0.packs[0].event_ids.is_empty());
    let authority:(String,String,i64)=f.connection().query_row("SELECT authority,principal_id,generation FROM behavior_provenance WHERE scope='event' AND resource_id='survivor'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(authority, ("delegated".into(), "moderator".into(), 2));
    assert!(f.db.load_behavior_runtime_snapshot().is_ok());
}

#[test]
fn business_trigger_failures_roll_back_catalog_graph_authority_outbox_and_ledger() {
    for trigger in [
        "CREATE TRIGGER fail_library BEFORE INSERT ON control_resource_outbox WHEN NEW.scope='library' BEGIN SELECT RAISE(ABORT,'fixture'); END;",
        "CREATE TRIGGER fail_library BEFORE INSERT ON control_commands BEGIN SELECT RAISE(ABORT,'fixture'); END;",
    ] {
        let f=Fixture::new(); let input=f.input(apply(entry("new-a","new-e"),"default"));
        let before=f.counts(); f.connection().execute_batch(trigger).unwrap();
        assert!(f.execute(&input,1).is_err());
        assert_eq!(f.counts(),before);
        assert_eq!(f.snapshot().1,"0");
        assert_eq!(f.snapshot().0,OwnerLibraryCatalog::default());
    }
}

#[test]
fn replay_returns_immutable_original_receipt_before_new_catalog_or_source_checks() {
    let f = Fixture::new();
    let input = f.input(apply(entry("first-a", "first-e"), "default"));
    let (original, fresh) = f.execute(&input, 1).unwrap();
    assert!(fresh);
    let saved: String = f
        .connection()
        .query_row(
            "SELECT result_json FROM control_commands WHERE command_id=?1",
            [format!("owner-library-{:032x}", 1)],
            |row| row.get(0),
        )
        .unwrap();
    let proof: Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(proof.as_object().unwrap().len(), 5);
    assert_eq!(proof["operation"], "apply-template");
    assert_eq!(proof["target"], "default");
    assert_eq!(proof["expectedRevision"], "0");
    assert_eq!(proof["receipt"], original.value);
    assert!(!saved.contains("private-source"));
    f.apply(profile("later"), 2);
    f.db.set_behavior_enabled_versioned("behavior_actions", "first-a", true, Some("1"))
        .unwrap();
    let counts = f.counts();
    let (replay, fresh) = f.execute(&input, 1).unwrap();
    assert!(!fresh);
    assert_eq!(replay.value, original.value);
    assert_eq!(replay.resource_versions, original.resource_versions);
    assert_eq!(f.counts(), counts);
    let mut corrupt = proof;
    corrupt["target"] = "other-profile".into();
    f.connection()
        .execute(
            "UPDATE control_commands SET result_json=?1 WHERE command_id=?2",
            params![corrupt.to_string(), format!("owner-library-{:032x}", 1)],
        )
        .unwrap();
    assert!(matches!(
        f.execute(&input, 1),
        Err(DatabaseError::Invalid(_))
    ));
    f.connection()
        .execute(
            "UPDATE control_commands SET result_json=?1 WHERE command_id=?2",
            params![saved, format!("owner-library-{:032x}", 1)],
        )
        .unwrap();
    let mut changed = input.clone();
    changed.mutation = profile("different-payload");
    assert!(matches!(
        f.execute(&changed, 1),
        Err(DatabaseError::CommandIdReused)
    ));
    f.connection()
        .execute(
            "UPDATE control_commands SET committed_at=0 WHERE command_id=?1",
            [format!("owner-library-{:032x}", 1)],
        )
        .unwrap();
    assert!(matches!(
        f.execute(&input, 1),
        Err(DatabaseError::CommandExpired)
    ));
    assert_eq!(f.counts(), counts);
}

#[test]
fn switch_preserves_delegated_provenance_and_invalidates_existing_approvals() {
    let f = Fixture::new();
    f.save(AutomationKind::Action, "approved");
    let db = f.connection();
    db.execute("UPDATE behavior_provenance SET authority='delegated',principal_id='moderator',session_id='session' WHERE scope='action' AND resource_id='approved'",[]).unwrap();
    db.execute("INSERT INTO behavior_execution_approvals(scope,resource_id,generation,state,policy_version,manifest_digest,binding_json,owner_principal_id,owner_session_id,approved_at,updated_at) VALUES ('action','approved',1,'active',1,?1,'{}','local-owner','local-owner',1,1)",["0".repeat(64)]).unwrap();
    db.execute("INSERT INTO behavior_execution_approval_dependencies(approval_scope,approval_id,position,definition_scope,definition_id,revision,provenance_generation,present) VALUES ('action','approved',0,'action','approved',1,1,1)",[]).unwrap();
    f.apply(
        OwnerLibraryMutation::SwitchProfile {
            id: "default".into(),
        },
        1,
    );
    let authority:(String,String,i64)=db.query_row("SELECT authority,principal_id,generation FROM behavior_provenance WHERE resource_id='approved'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(authority, ("delegated".into(), "moderator".into(), 2));
    let approval:(String,i64)=db.query_row("SELECT state,generation FROM behavior_execution_approvals WHERE resource_id='approved'",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(approval, ("invalidated".into(), 2));
}

#[test]
fn missing_sidecars_or_unknown_catalog_scope_fail_closed_without_promoting_records() {
    for corrupt in [
        "DELETE FROM control_resource_versions WHERE scope='action'; DELETE FROM behavior_provenance WHERE scope='action';",
        "INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted) VALUES ('automation','unknown','catalog',1,0);",
        "INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted) VALUES ('automation','library','other',1,0);",
    ] {
        let f=Fixture::new(); f.save(AutomationKind::Action,"record");
        let input=f.input(OwnerLibraryMutation::SwitchProfile{id:"default".into()});
        f.connection().execute_batch(corrupt).unwrap();
        assert!(f.execute(&input,1).is_err());
        assert_eq!(f.body(AutomationKind::Action,"record").value.unwrap()["enabled"],false);
        assert_eq!(f.connection().query_row("SELECT revision FROM automation_library_catalog",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        assert!(f.db.load_behavior_runtime_snapshot().is_err());
    }
    let f = Fixture::new();
    f.apply(profile("valid"), 1);
    assert!(f.db.load_behavior_runtime_snapshot().is_ok());
    f.connection()
        .execute(
            "UPDATE control_resource_versions SET deleted=1 WHERE scope='library'",
            [],
        )
        .unwrap();
    assert!(f.db.load_behavior_runtime_snapshot().is_err());
    assert!(f.db.automation_library_snapshot().is_err());
}

#[test]
fn compatibility_projection_is_canonical_and_all_pending_journals_block_changes() {
    let f = Fixture::new();
    f.apply(profile("a"), 1);
    let outboxes = f.counts().4;
    f.db.automation_library_legacy_set("host", ACTIVE_KEY, "a")
        .unwrap();
    assert_eq!(f.snapshot().1, "2");
    assert_eq!(f.counts().4, outboxes + 1);
    let state = f.db.automation_library_legacy_state().unwrap();
    assert_eq!(state[ACTIVE_KEY], "a");
    assert_eq!(
        serde_json::from_str::<Vec<OwnerLibraryPack>>(&state[PACKS_KEY]).unwrap(),
        f.snapshot().0.packs
    );
    f.db.automation_library_legacy_set("host", PACKS_KEY, "[]")
        .unwrap();
    assert_eq!(f.snapshot().0.active_id, "default");
    for table in [
        "owner_automation_adoptions",
        "owner_automation_approvals",
        "owner_automation_definitions",
        "owner_automation_library",
    ] {
        f.connection().execute(&format!("INSERT INTO {table}(command_id,pending,payload_json,updated_at) VALUES ('corrupt',1,'{{}}',1)"),[]).unwrap();
        let revision = f.snapshot().1;
        assert!(f
            .db
            .automation_library_legacy_set("host", TEMPLATES_KEY, "[]")
            .is_err());
        assert_eq!(f.snapshot().1, revision);
        f.connection()
            .execute(&format!("DELETE FROM {table}"), [])
            .unwrap();
    }
}

#[test]
fn full_universe_over_512_is_readable_and_switches_atomically() {
    let f = Fixture::new();
    f.db.ensure_automation_library().unwrap();
    let mut db = f.connection();
    let tx = db.transaction().unwrap();
    for serial in 0..600 {
        let id = format!("record-{serial:04}");
        tx.execute("INSERT INTO behavior_actions(id,name,enabled,payload_json,created_at,updated_at) VALUES (?1,?1,0,?2,1,1)",params![id,serde_json::to_string(&record(&id)).unwrap()]).unwrap();
        tx.execute("INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted) VALUES ('automation','action',?1,1,0)",[&id]).unwrap();
        tx.execute("INSERT INTO behavior_provenance(scope,resource_id,authority,generation) VALUES ('action',?1,'owner',1)",[&id]).unwrap();
    }
    tx.commit().unwrap();
    assert_eq!(f.snapshot().2.resource_versions.len(), 600);
    let result = f.apply(
        OwnerLibraryMutation::SwitchProfile {
            id: "default".into(),
        },
        1,
    );
    assert_eq!(result.resource_versions.len(), 601);
    assert_eq!(
        f.body(AutomationKind::Action, "record-0599").value.unwrap()["enabled"],
        true
    );
}

#[test]
fn raw_source_admission_precedes_decode_and_oversized_toggle_rolls_back() {
    let f = Fixture::new();
    f.save(AutomationKind::Action, "large");
    let mut large = record("large");
    large["large"] = Value::String("x".repeat(256 * 1024));
    f.connection()
        .execute(
            "UPDATE behavior_actions SET payload_json=?1 WHERE id='large'",
            [large.to_string()],
        )
        .unwrap();
    // Existing source can still be reviewed, while every rewritten output has the stricter record limit.
    let input = f.input(OwnerLibraryMutation::SwitchProfile {
        id: "default".into(),
    });
    let counts = f.counts();
    assert!(matches!(
        f.execute(&input, 1),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    assert_eq!(f.counts(), counts);
    assert_eq!(f.snapshot().1, "0");
    f.connection()
        .execute(
            "UPDATE behavior_actions SET payload_json=zeroblob(?1)",
            [i64::try_from(OWNER_LIBRARY_MAX_BYTES + 1).unwrap()],
        )
        .unwrap();
    assert!(matches!(
        f.db.automation_library_snapshot(),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    let g = Fixture::new();
    g.db.ensure_automation_library().unwrap();
    g.connection()
        .execute(
            "UPDATE automation_library_catalog SET catalog_json=zeroblob(?1)",
            [i64::try_from(OWNER_LIBRARY_MAX_CATALOG_BYTES + 1).unwrap()],
        )
        .unwrap();
    assert!(matches!(
        g.db.automation_library_snapshot(),
        Err(DatabaseError::SnapshotTooLarge)
    ));
}

#[test]
fn fresh_graph_membership_uses_requested_profile_and_clears_never_created_ghosts() {
    let f = Fixture::new();
    f.apply(profile("other"), 1);
    let mut catalog = f.snapshot().0;
    catalog.packs[0].action_ids = vec!["fresh-a".into()];
    catalog.packs[0].event_ids = vec!["fresh-e".into()];
    f.db.automation_library_legacy_set(
        "host",
        PACKS_KEY,
        &serde_json::to_string(&catalog.packs).unwrap(),
    )
    .unwrap();
    f.apply(apply(entry("fresh-a", "fresh-e"), "default"), 2);
    assert!(f.snapshot().0.packs[0].action_ids.is_empty());
    assert!(f.snapshot().0.packs[0].event_ids.is_empty());
    f.apply(
        OwnerLibraryMutation::SwitchProfile {
            id: "default".into(),
        },
        3,
    );
    assert_eq!(
        f.body(AutomationKind::Action, "fresh-a").value.unwrap()["enabled"],
        true
    );
    assert_eq!(
        f.body(AutomationKind::Event, "fresh-e").value.unwrap()["enabled"],
        true
    );
}

#[test]
fn compatibility_catalog_outbox_has_exact_audit_classification_and_drains() {
    let f = Fixture::new();
    f.snapshot();
    f.db.automation_library_legacy_set("host", TEMPLATES_KEY, "[]")
        .unwrap();
    let pending = f.db.pending_coordination().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].audit.method, "automation.library.apply");
    assert_eq!(pending[0].audit.resource.scope, "library");
    assert_eq!(pending[0].audit.resource.id, "catalog");
    f.db.acknowledge_coordination(std::slice::from_ref(&pending[0].token))
        .unwrap();
    assert!(f.db.pending_coordination().unwrap().is_empty());
    f.connection().execute("INSERT INTO control_resource_outbox(kind,scope,resource_id,revision,change,committed_at) VALUES ('automation','unknown','catalog',1,'library-updated',1)", []).unwrap();
    assert!(f.db.pending_coordination().is_err());
}

#[test]
fn generic_state_reads_skip_oversized_retained_legacy_source() {
    let f = Fixture::new();
    f.legacy("unrelated", "available");
    f.db.open(&f.db.points_path())
        .unwrap()
        .execute(
            "INSERT INTO app_state(key,value,updated_at) VALUES (?1,zeroblob(?2),1)",
            params![
                TEMPLATES_KEY,
                i64::try_from(OWNER_LIBRARY_MAX_CATALOG_BYTES + 1).unwrap()
            ],
        )
        .unwrap();
    assert!(f.db.automation_library_legacy_exists().unwrap());
    let state = f.db.load_app_state().unwrap();
    assert_eq!(state.len(), 1);
    assert_eq!(state["unrelated"], "available");
    assert!(matches!(
        f.db.ensure_automation_library(),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    assert!(!f.db.automation_library_exists().unwrap());
}

#[test]
fn trusted_dynamic_snapshot_budget_rolls_back_new_effects_but_does_not_block_retained_replay() {
    let f = Fixture::new();
    let input = f.input(apply(entry("new-a", "new-e"), "default"));
    let command = CommandTransaction {
        principal_id: "local-owner".into(),
        session_id: "local-owner".into(),
        authorization: None,
        request: CommandRequest {
            command_id: format!("owner-library-{:032x}", 1),
            host_instance_id: input.host_instance_id.clone(),
            expected_revision: input.expected_revision.clone(),
        },
        current_instance_id: input.host_instance_id.clone(),
        max_ids_per_principal: 10_000,
    };
    let before = f.counts();
    assert!(matches!(
        f.db.execute_automation_library_command_with_budget(&input, &command, 32),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    assert_eq!(f.counts(), before);
    assert_eq!(f.snapshot().1, "0");
    let (original, fresh) =
        f.db.execute_automation_library_command_with_budget(&input, &command, SNAPSHOT_BYTES)
            .unwrap();
    assert!(fresh);
    let counts = f.counts();
    let (replayed, fresh) =
        f.db.execute_automation_library_command_with_budget(&input, &command, 0)
            .unwrap();
    assert!(!fresh);
    assert_eq!(replayed.value, original.value);
    assert_eq!(replayed.resource_versions, original.resource_versions);
    assert_eq!(f.counts(), counts);
}
