use super::*;
use crate::{collaboration::*, paths::AppPaths};
struct Fixture {
    db: DatabaseManager,
    service: CollaborationService,
    root: std::path::PathBuf,
}
fn paths(root: &std::path::Path) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.into(),
    }
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-behavior-authority-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = paths(&root);
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        let db = DatabaseManager::new(paths);
        let service = CollaborationService::open(&root.join("identity")).unwrap();
        let mut config = service.configuration().unwrap();
        config.enabled = true;
        config.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(config).unwrap();
        Self { db, service, root }
    }
    fn auth(&self, grant: Grant) -> Authorization {
        let invitation = self
            .service
            .create_invitation(InviteCreate { grant })
            .unwrap();
        let exchange = "a".repeat(64);
        let pending = self
            .service
            .redeem(
                Redeem {
                    invite_id: invitation.invite_id,
                    secret: invitation.secret,
                    client_label: "Draft author".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "fixture",
            )
            .unwrap();
        self.service
            .decide_request(&pending.request_id, true)
            .unwrap();
        let credentials = self
            .service
            .collect(
                Collect {
                    request_id: pending.request_id,
                    exchange_secret: exchange,
                },
                "fixture",
            )
            .unwrap();
        self.service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap()
    }
    fn author(&self) -> Authorization {
        self.auth(Grant {
            capabilities: [Capability::AutomationRead, Capability::AutomationEdit].into(),
            ..Grant::default()
        })
    }
    fn draft(
        &self,
        auth: &Authorization,
        table: &str,
        record: Value,
        expected: &str,
        mode: BehaviorSave,
    ) -> Result<Versioned<Value>, DatabaseError> {
        let id = record.get("id").and_then(Value::as_str).unwrap().to_owned();
        self.db.behavior_transaction(|tx| {
            let permit = authorize_authoring(tx, table, &id, auth)?;
            let result = save_record(tx, table, &record, Some(expected), mode)?;
            mark_delegated_new(tx, table, &id, &permit)?;
            Ok(result)
        })
    }
    fn provenance(&self, table: &str, id: &str) -> BehaviorProvenance {
        self.db.behavior_provenance(table, id).unwrap().unwrap()
    }
    fn outbox(&self) -> i64 {
        self.db
            .open(&self.db.automation_path())
            .unwrap()
            .query_row(
                "SELECT count(*) FROM control_resource_outbox WHERE kind='automation'",
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
fn draft(id: &str) -> Value {
    json!({"id":id,"name":"Draft","enabled":false,"typeId":"core.log","trigger":"tiktok.chat","actionIds":["same"],"provenance":{"kind":"owner"},"authority":"owner"})
}

#[test]
fn runtime_projection_retains_the_exact_saved_subject_generation_revision_and_tombstone_state() {
    let f = Fixture::new();
    let auth = f.author();
    let identity = auth.snapshot().unwrap();
    f.db.save_behavior("behavior_actions", &draft("same"))
        .unwrap();
    f.draft(
        &auth,
        "behavior_events",
        draft("same"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    let (snapshot, authority) = f.db.load_behavior_runtime_snapshot().unwrap();
    assert_eq!(snapshot.resource_versions.len(), 2);
    let action = authority
        .iter()
        .find(|(scope, id, _)| scope == "action" && id == "same")
        .unwrap();
    assert_eq!(action.2.authority, BehaviorAuthority::Owner);
    assert_eq!(action.2.revision, 1);
    assert_eq!(action.2.generation, 1);
    assert!(!action.2.deleted);
    let event = authority
        .iter()
        .find(|(scope, id, _)| scope == "event" && id == "same")
        .unwrap();
    let trusted = BehaviorAuthority::Delegated {
        principal_id: identity.principal_id,
        session_id: identity.session_id,
    };
    assert_eq!(event.2.authority, trusted);
    assert_eq!(event.2.revision, 1);
    assert_eq!(event.2.generation, 1);
    assert!(!event.2.deleted);
    f.db.delete_behavior_versioned("behavior_events", "same", Some("1"))
        .unwrap();
    let (snapshot, authority) = f.db.load_behavior_runtime_snapshot().unwrap();
    assert!(snapshot.value["events"].as_array().unwrap().is_empty());
    assert_eq!(snapshot.resource_versions.len(), 1);
    let retained = authority
        .iter()
        .find(|(scope, id, _)| scope == "event" && id == "same")
        .unwrap();
    assert_eq!(retained.2.authority, trusted);
    assert_eq!(retained.2.revision, 2);
    assert_eq!(retained.2.generation, 2);
    assert!(retained.2.deleted);
    let reopened = DatabaseManager::new(f.db.paths().clone());
    let (_, reopened_authority) = reopened.load_behavior_runtime_snapshot().unwrap();
    assert_eq!(authority, reopened_authority);
}

#[test]
fn authority_migration_is_once_keeps_legacy_json_and_never_reseeds_a_missing_sidecar() {
    let f = Fixture::new();
    let c = f.db.open(&f.db.automation_path()).unwrap();
    c.execute_batch("DROP TABLE behavior_provenance; DROP TABLE behavior_provenance_migration;")
        .unwrap();
    let raw="{\"id\":\"same\",\"name\":\"legacy\",\"enabled\":true,\"authority\":{\"kind\":\"delegated\",\"principalId\":\"forged\"}}";
    for table in ["behavior_actions", "behavior_events"] {
        c.execute(&format!("INSERT INTO {table}(id,name,enabled,payload_json,created_at,updated_at) VALUES ('same','legacy',1,?1,11,22)"),[raw]).unwrap();
    }
    c.execute("INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted) VALUES ('automation','action','same',7,0),('automation','event','removed',9,1)",[]).unwrap();
    let upgraded = DatabaseManager::new(f.db.paths().clone());
    for (table, id) in [
        ("behavior_actions", "same"),
        ("behavior_events", "same"),
        ("behavior_events", "removed"),
    ] {
        assert_eq!(
            upgraded.behavior_provenance(table, id).unwrap().unwrap(),
            BehaviorProvenance {
                authority: BehaviorAuthority::Owner,
                generation: 1
            }
        );
    }
    let stored: String = c
        .query_row(
            "SELECT payload_json FROM behavior_actions WHERE id='same'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored, raw);
    c.execute(
        "DELETE FROM behavior_provenance WHERE scope='action' AND resource_id='same'",
        [],
    )
    .unwrap();
    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert!(reopened
        .behavior_provenance("behavior_actions", "same")
        .unwrap()
        .is_none());
    assert!(matches!(
        reopened.set_behavior_enabled_versioned("behavior_actions", "same", false, Some("7")),
        Err(DatabaseError::Invalid(_))
    ));
    assert_eq!(
        reopened
            .behavior_get_versioned("behavior_actions", "same")
            .unwrap()
            .resource_versions[0]
            .revision,
        "7"
    );
    assert_eq!(
        c.query_row(
            "SELECT enabled FROM behavior_actions WHERE id='same'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn delegated_provenance_survives_owner_edits_toggle_bulk_cascade_delete_and_recreation() {
    let f = Fixture::new();
    let auth = f.author();
    let identity = auth.snapshot().unwrap();
    f.db.save_behavior(
        "behavior_actions",
        &json!({"id":"same","name":"Owner action","enabled":true}),
    )
    .unwrap();
    f.draft(
        &auth,
        "behavior_events",
        draft("same"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    let trusted = BehaviorAuthority::Delegated {
        principal_id: identity.principal_id,
        session_id: identity.session_id,
    };
    assert_eq!(
        f.provenance("behavior_actions", "same").authority,
        BehaviorAuthority::Owner
    );
    assert_eq!(
        f.provenance("behavior_events", "same"),
        BehaviorProvenance {
            authority: trusted.clone(),
            generation: 1
        }
    );
    let mut edited = draft("same");
    edited["enabled"] = true.into();
    edited["name"] = "Owner edit, same authority".into();
    f.db.save_behavior_versioned("behavior_events", &edited, Some("1"), BehaviorSave::Update)
        .unwrap();
    f.db.set_behavior_enabled_versioned("behavior_events", "same", false, Some("2"))
        .unwrap();
    f.db.set_behavior_enablement(&[("behavior_events", "same", true)])
        .unwrap();
    assert_eq!(
        f.provenance("behavior_events", "same"),
        BehaviorProvenance {
            authority: trusted.clone(),
            generation: 4
        }
    );
    f.db.delete_behavior_versioned("behavior_actions", "same", Some("1"))
        .unwrap();
    assert_eq!(
        f.provenance("behavior_events", "same"),
        BehaviorProvenance {
            authority: trusted.clone(),
            generation: 5
        }
    );
    assert_eq!(
        f.db.behavior_get_versioned("behavior_events", "same")
            .unwrap()
            .value
            .unwrap()["actionIds"],
        json!([])
    );
    f.db.delete_behavior_versioned("behavior_events", "same", Some("5"))
        .unwrap();
    let (_, authorities) = f.db.load_behavior_runtime_snapshot().unwrap();
    assert!(
        authorities
            .iter()
            .any(|(scope, id, p)| scope == "event" && id == "same" && p.authority == trusted),
        "delegated tombstones must remain visible to runtime authority checks"
    );
    f.db.save_behavior_versioned(
        "behavior_events",
        &draft("same"),
        Some("6"),
        BehaviorSave::Create,
    )
    .unwrap();
    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert_eq!(
        reopened
            .behavior_provenance("behavior_events", "same")
            .unwrap()
            .unwrap(),
        BehaviorProvenance {
            authority: trusted,
            generation: 7
        }
    );
    assert_eq!(
        reopened
            .behavior_get_versioned("behavior_events", "same")
            .unwrap()
            .value
            .unwrap()["authority"],
        "owner",
        "JSON forgery stays ordinary content"
    );
}

#[test]
fn remote_authoring_cannot_take_owner_foreign_or_retained_ids_and_cannot_publish_enabled_work() {
    let f = Fixture::new();
    let author = f.author();
    let foreign = f.author();
    f.db.save_behavior("behavior_events", &draft("owner"))
        .unwrap();
    let before = f.outbox();
    assert!(matches!(
        f.draft(
            &author,
            "behavior_events",
            draft("owner"),
            "1",
            BehaviorSave::Update
        ),
        Err(DatabaseError::CommandAuthorization(_))
    ));
    assert_eq!(f.outbox(), before);
    f.draft(
        &author,
        "behavior_events",
        draft("shared"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    let original = f.provenance("behavior_events", "shared");
    assert!(matches!(
        f.draft(
            &foreign,
            "behavior_events",
            draft("shared"),
            "1",
            BehaviorSave::Update
        ),
        Err(DatabaseError::CommandAuthorization(_))
    ));
    f.db.delete_behavior_versioned("behavior_events", "shared", Some("1"))
        .unwrap();
    assert!(matches!(
        f.draft(
            &foreign,
            "behavior_events",
            draft("shared"),
            "2",
            BehaviorSave::Create
        ),
        Err(DatabaseError::CommandAuthorization(_))
    ));
    f.draft(
        &author,
        "behavior_events",
        draft("shared"),
        "2",
        BehaviorSave::Create,
    )
    .unwrap();
    assert_eq!(
        f.provenance("behavior_events", "shared").authority,
        original.authority
    );
    let mut enabled = draft("enabled");
    enabled["enabled"] = true.into();
    let count = f.outbox();
    assert!(f
        .draft(
            &author,
            "behavior_events",
            enabled,
            "0",
            BehaviorSave::Create
        )
        .is_err());
    assert!(f
        .db
        .behavior_get_versioned("behavior_events", "enabled")
        .unwrap()
        .value
        .is_none());
    assert!(f
        .db
        .behavior_provenance("behavior_events", "enabled")
        .unwrap()
        .is_none());
    assert_eq!(f.outbox(), count);
}

#[test]
fn authoring_checks_current_permissions_inside_the_writer_and_rolls_back_on_reservation_failure() {
    let f = Fixture::new();
    let restricted = f.auth(Grant {
        capabilities: [Capability::AutomationRead, Capability::AutomationEdit].into(),
        resources: [(ResourceKind::Automation, ["action:same".into()].into())].into(),
        ..Grant::default()
    });
    let no_read = f.auth(Grant {
        capabilities: [Capability::AutomationEdit].into(),
        ..Grant::default()
    });
    let creator = f.auth(Grant {
        capabilities: [Capability::AutomationRead, Capability::AutomationEdit].into(),
        resources: [(ResourceKind::Live, ["creator".into()].into())].into(),
        ..Grant::default()
    });
    for auth in [&restricted, &no_read, &creator] {
        assert!(matches!(
            f.draft(
                auth,
                "behavior_events",
                draft("same"),
                "0",
                BehaviorSave::Create
            ),
            Err(DatabaseError::CommandAuthorization(_))
        ));
    }
    let auth = f.author();
    let identity = auth.snapshot().unwrap();
    f.service
        .revoke_session(&identity.session_id, "owner")
        .unwrap();
    assert!(matches!(
        f.draft(
            &auth,
            "behavior_events",
            draft("same"),
            "0",
            BehaviorSave::Create
        ),
        Err(DatabaseError::CommandAuthorization(_))
    ));
    let auth = f.author();
    let c = f.db.open(&f.db.automation_path()).unwrap();
    c.execute_batch("CREATE TRIGGER fail_authority_reservation BEFORE INSERT ON control_resource_outbox WHEN NEW.kind='automation' BEGIN SELECT RAISE(ABORT,'fixture outbox failure'); END;").unwrap();
    assert!(f
        .draft(
            &auth,
            "behavior_events",
            draft("same"),
            "0",
            BehaviorSave::Create
        )
        .is_err());
    assert!(f
        .db
        .behavior_get_versioned("behavior_events", "same")
        .unwrap()
        .value
        .is_none());
    assert!(f
        .db
        .behavior_provenance("behavior_events", "same")
        .unwrap()
        .is_none());
}

#[test]
fn explicit_adoption_is_cas_bound_atomic_and_invalidates_the_generation_once() {
    let f = Fixture::new();
    let auth = f.author();
    f.draft(
        &auth,
        "behavior_events",
        draft("same"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    let before = f.provenance("behavior_events", "same");
    let count = f.outbox();
    assert!(matches!(
        f.db.adopt_behavior_owner("behavior_events", "same", "0"),
        Err(DatabaseError::RevisionConflict(_))
    ));
    assert_eq!(f.provenance("behavior_events", "same"), before);
    let c = f.db.open(&f.db.automation_path()).unwrap();
    c.execute_batch("CREATE TRIGGER fail_adoption BEFORE INSERT ON control_resource_outbox WHEN NEW.change='adopted' BEGIN SELECT RAISE(ABORT,'fixture adoption failure'); END;").unwrap();
    assert!(f
        .db
        .adopt_behavior_owner("behavior_events", "same", "1")
        .is_err());
    assert_eq!(f.provenance("behavior_events", "same"), before);
    assert_eq!(f.outbox(), count);
    c.execute_batch("DROP TRIGGER fail_adoption;").unwrap();
    let adopted =
        f.db.adopt_behavior_owner("behavior_events", "same", "1")
            .unwrap();
    assert_eq!(
        adopted.value,
        BehaviorProvenance {
            authority: BehaviorAuthority::Owner,
            generation: 2
        }
    );
    assert_eq!(adopted.resource_versions[0].revision, "2");
    assert_eq!(f.outbox(), count + 1);
    let same =
        f.db.adopt_behavior_owner("behavior_events", "same", "2")
            .unwrap();
    assert_eq!(same, adopted);
    assert_eq!(
        f.outbox(),
        count + 1,
        "already-owned adoption is a checked no-op"
    );
}

#[test]
fn adoption_of_owner_enabled_quarantined_work_requires_a_separate_enable_after_transfer() {
    let f = Fixture::new();
    let auth = f.author();
    f.draft(
        &auth,
        "behavior_events",
        draft("same"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    f.db.set_behavior_enabled_versioned("behavior_events", "same", true, Some("1"))
        .unwrap();
    assert!(matches!(
        f.provenance("behavior_events", "same").authority,
        BehaviorAuthority::Delegated { .. }
    ));
    let adopted =
        f.db.adopt_behavior_owner("behavior_events", "same", "2")
            .unwrap();
    assert_eq!(
        adopted.value,
        BehaviorProvenance {
            authority: BehaviorAuthority::Owner,
            generation: 3
        }
    );
    let body =
        f.db.behavior_get_versioned("behavior_events", "same")
            .unwrap();
    assert_eq!(body.resource_versions[0].revision, "3");
    assert_eq!(body.value.unwrap()["enabled"], false);
    let governance =
        f.db.behavior_provenance_versioned("behavior_events", "same")
            .unwrap();
    assert_eq!(governance.value, Some(adopted.value));
    assert_eq!(governance.resource_versions, adopted.resource_versions);
    f.db.set_behavior_enabled_versioned("behavior_events", "same", true, Some("3"))
        .unwrap();
    assert_eq!(
        f.db.behavior_get_versioned("behavior_events", "same")
            .unwrap()
            .value
            .unwrap()["enabled"],
        true
    );
}

#[test]
fn exhausted_authority_generation_rolls_back_payload_versions_and_outbox() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_events", &draft("same"))
        .unwrap();
    let c = f.db.open(&f.db.automation_path()).unwrap();
    c.execute(
        "UPDATE behavior_provenance SET generation=?1 WHERE scope='event' AND resource_id='same'",
        [i64::MAX],
    )
    .unwrap();
    let count = f.outbox();
    let mut changed = draft("same");
    changed["name"] = "Must roll back".into();
    assert!(matches!(
        f.db.save_behavior_versioned("behavior_events", &changed, Some("1"), BehaviorSave::Update),
        Err(DatabaseError::Invalid(_))
    ));
    let row =
        f.db.behavior_get_versioned("behavior_events", "same")
            .unwrap();
    assert_eq!(row.resource_versions[0].revision, "1");
    assert_eq!(row.value.unwrap()["name"], "Draft");
    assert_eq!(f.outbox(), count);
}

#[test]
#[ignore = "subprocess-only provenance adoption commit fixture"]
fn adoption_crash_child() {
    let root = std::path::PathBuf::from(std::env::var_os("TIKTOOLS_AUTHORITY_ROOT").unwrap());
    let db = DatabaseManager::new(paths(&root));
    db.adopt_behavior_owner("behavior_events", "same", "1")
        .unwrap();
    panic!("adoption crash boundary was not reached");
}
#[test]
fn process_death_never_splits_owner_adoption_authority_generation_version_or_outbox() {
    for stage in ["before", "after"] {
        let f = Fixture::new();
        let auth = f.author();
        f.draft(
            &auth,
            "behavior_events",
            draft("same"),
            "0",
            BehaviorSave::Create,
        )
        .unwrap();
        let original = f.provenance("behavior_events", "same");
        let count = f.outbox();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "db::sqlite::behavior::provenance::tests::adoption_crash_child",
                "--ignored",
            ])
            .env("TIKTOOLS_AUTHORITY_ROOT", &f.root)
            .env("TIKTOOLS_BEHAVIOR_CRASH", stage)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(result.code(), Some(if stage == "before" { 91 } else { 92 }));
        let reopened = DatabaseManager::new(f.db.paths().clone());
        let p = reopened
            .behavior_provenance("behavior_events", "same")
            .unwrap()
            .unwrap();
        assert_eq!(
            p,
            if stage == "before" {
                original
            } else {
                BehaviorProvenance {
                    authority: BehaviorAuthority::Owner,
                    generation: 2,
                }
            }
        );
        assert_eq!(
            reopened
                .behavior_get_versioned("behavior_events", "same")
                .unwrap()
                .resource_versions[0]
                .revision,
            if stage == "before" { "1" } else { "2" }
        );
        assert_eq!(f.outbox(), count + if stage == "before" { 0 } else { 1 });
    }
}

#[test]
fn owner_and_foreign_event_dependencies_reserve_even_missing_action_ids() {
    for enabled in [false, true] {
        let f = Fixture::new();
        let author = f.author();
        f.db.save_behavior("behavior_events",&json!({"id":"private-owner-event","name":"Do not expose this event","enabled":enabled,"actionIds":["reserved"]})).unwrap();
        let before = f.outbox();
        let error = f
            .draft(
                &author,
                "behavior_actions",
                draft("reserved"),
                "0",
                BehaviorSave::Create,
            )
            .unwrap_err();
        assert!(matches!(error, DatabaseError::CommandAuthorization(_)));
        assert!(!error.to_string().contains("private-owner-event"));
        assert!(f
            .db
            .behavior_get_versioned("behavior_actions", "reserved")
            .unwrap()
            .value
            .is_none());
        assert!(f
            .db
            .behavior_provenance("behavior_actions", "reserved")
            .unwrap()
            .is_none());
        assert_eq!(f.outbox(), before);
        // The trusted owner writer remains compatible with its own graph.
        f.db.save_behavior("behavior_actions", &draft("reserved"))
            .unwrap();
        assert_eq!(
            f.provenance("behavior_actions", "reserved").authority,
            BehaviorAuthority::Owner
        );
    }
    let f = Fixture::new();
    let author = f.author();
    let other = f.author();
    let mut event = draft("foreign-event");
    event["actionIds"] = json!(["missing"]);
    f.draft(&other, "behavior_events", event, "0", BehaviorSave::Create)
        .unwrap();
    assert!(matches!(
        f.draft(
            &author,
            "behavior_actions",
            draft("missing"),
            "0",
            BehaviorSave::Create
        ),
        Err(DatabaseError::CommandAuthorization(_))
    ));
    assert!(f
        .db
        .behavior_get_versioned("behavior_actions", "missing")
        .unwrap()
        .value
        .is_none());
}

#[test]
fn same_author_dependencies_allow_action_drafts_but_later_owner_or_foreign_refs_block_updates() {
    let f = Fixture::new();
    let author = f.author();
    let other = f.author();
    f.draft(
        &author,
        "behavior_events",
        draft("my-event"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    f.draft(
        &author,
        "behavior_actions",
        draft("same"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    let mut changed = draft("same");
    changed["name"] = "Revised own action".into();
    f.draft(
        &author,
        "behavior_actions",
        changed,
        "1",
        BehaviorSave::Update,
    )
    .unwrap();
    assert_eq!(f.provenance("behavior_actions", "same").generation, 2);
    f.draft(
        &other,
        "behavior_events",
        draft("other-event"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    let before = f.outbox();
    let mut changed = draft("same");
    changed["name"] = "Must remain unsaved".into();
    assert!(matches!(
        f.draft(
            &author,
            "behavior_actions",
            changed.clone(),
            "2",
            BehaviorSave::Update
        ),
        Err(DatabaseError::CommandAuthorization(_))
    ));
    assert_eq!(f.outbox(), before);
    assert_eq!(
        f.db.behavior_get_versioned("behavior_actions", "same")
            .unwrap()
            .value
            .unwrap()["name"],
        "Revised own action"
    );
    f.db.delete_behavior_versioned("behavior_events", "other-event", Some("1"))
        .unwrap();
    f.db.save_behavior("behavior_events", &draft("owner-event"))
        .unwrap();
    assert!(matches!(
        f.draft(
            &author,
            "behavior_actions",
            changed,
            "2",
            BehaviorSave::Update
        ),
        Err(DatabaseError::CommandAuthorization(_))
    ));
    assert_eq!(f.provenance("behavior_actions", "same").generation, 2);
}

#[test]
fn uninspectable_event_dependencies_or_authority_fail_closed_without_exposing_owner_data() {
    for corruption in [
        "json",
        "object",
        "array",
        "entries",
        "missing-authority",
        "corrupt-authority",
        "missing-version",
        "oversized",
    ] {
        let f = Fixture::new();
        let author = f.author();
        f.db.save_behavior("behavior_events",&json!({"id":"hidden-owner-event","name":"private-label","enabled":false,"actionIds":[]})).unwrap();
        let c = f.db.open(&f.db.automation_path()).unwrap();
        match corruption {
            "json" => {
                c.execute("UPDATE behavior_events SET payload_json='malformed private-source' WHERE id='hidden-owner-event'",[]).unwrap();
            }
            "object" => {
                c.execute(
                    "UPDATE behavior_events SET payload_json='[]' WHERE id='hidden-owner-event'",
                    [],
                )
                .unwrap();
            }
            "array" => {
                c.execute(
                    "UPDATE behavior_events SET payload_json=?1 WHERE id='hidden-owner-event'",
                    [r#"{"actionIds":"private-source"}"#],
                )
                .unwrap();
            }
            "entries" => {
                c.execute(
                    "UPDATE behavior_events SET payload_json=?1 WHERE id='hidden-owner-event'",
                    [r#"{"actionIds":[7]}"#],
                )
                .unwrap();
            }
            "missing-authority" => {
                c.execute("DELETE FROM behavior_provenance WHERE scope='event' AND resource_id='hidden-owner-event'",[]).unwrap();
            }
            "corrupt-authority" => {
                c.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE behavior_provenance SET authority='corrupt private-source' WHERE scope='event' AND resource_id='hidden-owner-event';").unwrap();
            }
            "oversized" => {
                c.execute(
                    "UPDATE behavior_events SET payload_json=?1 WHERE id='hidden-owner-event'",
                    [
                        json!({"actionIds":[],"source":"private-source".repeat(25_000)})
                            .to_string(),
                    ],
                )
                .unwrap();
            }
            "missing-version" => {
                c.execute("DELETE FROM control_resource_versions WHERE kind='automation' AND scope='event' AND resource_id='hidden-owner-event'",[]).unwrap();
            }
            _ => unreachable!(),
        }
        let before = f.outbox();
        let error = f
            .draft(
                &author,
                "behavior_actions",
                draft("unrelated-action"),
                "0",
                BehaviorSave::Create,
            )
            .unwrap_err();
        assert!(
            matches!(error, DatabaseError::CommandAuthorization(_)),
            "{corruption}"
        );
        for private in ["private-source", "private-label", "hidden-owner-event"] {
            assert!(!error.to_string().contains(private));
        }
        assert_eq!(f.outbox(), before);
        assert!(f
            .db
            .behavior_get_versioned("behavior_actions", "unrelated-action")
            .unwrap()
            .value
            .is_none());
        assert!(f
            .db
            .behavior_provenance("behavior_actions", "unrelated-action")
            .unwrap()
            .is_none());
    }
}

#[test]
fn corrupt_authority_does_not_hide_ordinary_records_versions_or_disabled_plugin_state() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_events", &draft("same"))
        .unwrap();
    f.db.set_plugin_state("acme.disabled", true, false).unwrap();
    let c = f.db.open(&f.db.automation_path()).unwrap();
    c.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE behavior_provenance SET generation=0 WHERE scope='event' AND resource_id='same';").unwrap();
    let normal = f.db.load_behavior_snapshot_versioned().unwrap();
    assert_eq!(normal.value["events"][0]["id"], "same");
    assert_eq!(normal.resource_versions[0].scope, "event");
    assert_eq!(normal.resource_versions[0].revision, "1");
    let plugin = normal.value["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "acme.disabled")
        .unwrap();
    assert_eq!(plugin["installed"], true);
    assert_eq!(plugin["enabled"], false);
    assert!(
        matches!(
            f.db.load_behavior_runtime_snapshot(),
            Err(DatabaseError::Invalid(_))
        ),
        "trusted runtime authority must still fail closed"
    );
}

fn catalog_fixture(table: &str) -> Fixture {
    let f = Fixture::new();
    f.db.save_behavior(table, &draft("healthy")).unwrap();
    f.db.save_behavior(table, &draft("broken")).unwrap();
    f.db.set_plugin_state("acme.disabled", true, false).unwrap();
    f
}

fn assert_catalog_failure_is_runtime_only(db: &DatabaseManager, table: &str) {
    let ordinary = db.load_behavior_snapshot_versioned().unwrap();
    let family = match table {
        "behavior_actions" => "actions",
        "behavior_events" => "events",
        _ => unreachable!(),
    };
    assert_eq!(ordinary.value[family].as_array().unwrap().len(), 1);
    assert_eq!(ordinary.value[family][0]["id"], "healthy");
    assert_eq!(ordinary.resource_versions.len(), 1);
    assert_eq!(
        ordinary.resource_versions[0].scope,
        versions::scope(table).unwrap()
    );
    assert_eq!(ordinary.resource_versions[0].id, "healthy");
    assert_eq!(ordinary.resource_versions[0].revision, "1");
    let plugin = ordinary.value["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "acme.disabled")
        .unwrap();
    assert_eq!(plugin["installed"], true);
    assert_eq!(plugin["enabled"], false);
    assert!(matches!(
        db.load_behavior_runtime_snapshot(),
        Err(DatabaseError::Invalid(_))
    ));
}

#[test]
fn orphan_owner_sidecars_without_versions_reject_runtime_and_survive_restart_without_repair() {
    for table in ["behavior_actions", "behavior_events"] {
        let f = catalog_fixture(table);
        f.db.delete_behavior_versioned(table, "broken", Some("1"))
            .unwrap();
        let c = f.db.open(&f.db.automation_path()).unwrap();
        c.execute(
            "DELETE FROM control_resource_versions WHERE kind='automation' AND scope=?1 AND resource_id='broken'",
            [versions::scope(table).unwrap()],
        )
        .unwrap();
        let before = f.outbox();
        assert_catalog_failure_is_runtime_only(&f.db, table);
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert_catalog_failure_is_runtime_only(&reopened, table);
        assert_eq!(versions::current(&c, table, "broken").unwrap(), 0);
        assert_eq!(
            reopened.behavior_provenance(table, "broken").unwrap(),
            Some(BehaviorProvenance {
                authority: BehaviorAuthority::Owner,
                generation: 2,
            })
        );
        assert_eq!(f.outbox(), before);
    }
}

#[test]
fn live_versions_without_bodies_reject_runtime_and_survive_restart_without_repair() {
    for table in ["behavior_actions", "behavior_events"] {
        let f = catalog_fixture(table);
        let c = f.db.open(&f.db.automation_path()).unwrap();
        c.execute(&format!("DELETE FROM {table} WHERE id='broken'"), [])
            .unwrap();
        let before = f.outbox();
        assert_catalog_failure_is_runtime_only(&f.db, table);
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert_catalog_failure_is_runtime_only(&reopened, table);
        let absent = reopened.behavior_get_versioned(table, "broken").unwrap();
        assert!(absent.value.is_none());
        assert_eq!(absent.resource_versions[0].revision, "1");
        let deleted: i64 = c
            .query_row(
                "SELECT deleted FROM control_resource_versions WHERE kind='automation' AND scope=?1 AND resource_id='broken'",
                [versions::scope(table).unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            deleted, 0,
            "startup cannot invent a tombstone to repair history"
        );
        assert_eq!(f.outbox(), before);
    }
}

#[test]
fn corrupt_owner_tombstones_reject_runtime_even_when_omitted_from_the_authority_projection() {
    for table in ["behavior_actions", "behavior_events"] {
        for corruption in ["generation", "owner-identity"] {
            let f = catalog_fixture(table);
            f.db.delete_behavior_versioned(table, "broken", Some("1"))
                .unwrap();
            let (_, authority) = f.db.load_behavior_runtime_snapshot().unwrap();
            assert!(authority.iter().all(|(_, id, _)| id != "broken"));
            let c = f.db.open(&f.db.automation_path()).unwrap();
            c.execute_batch("PRAGMA ignore_check_constraints=ON;")
                .unwrap();
            let update = match corruption {
                "generation" => "generation=0",
                "owner-identity" => "principal_id='invalid-owner-principal'",
                _ => unreachable!(),
            };
            c.execute(
                &format!("UPDATE behavior_provenance SET {update} WHERE scope=?1 AND resource_id='broken'"),
                [versions::scope(table).unwrap()],
            )
            .unwrap();
            let before = f.outbox();
            assert_catalog_failure_is_runtime_only(&f.db, table);
            let reopened = DatabaseManager::new(f.db.paths().clone());
            assert_catalog_failure_is_runtime_only(&reopened, table);
            assert!(matches!(
                reopened.behavior_provenance(table, "broken"),
                Err(DatabaseError::Invalid(_))
            ));
            assert_eq!(versions::current(&c, table, "broken").unwrap(), 2);
            assert_eq!(f.outbox(), before);
        }
    }
}

#[test]
fn owner_review_returns_every_dependency_in_order_with_canonical_bodies_and_trusted_authority() {
    let f = Fixture::new();
    let author = f.author();
    let identity = author.snapshot().unwrap();
    f.draft(
        &author,
        "behavior_actions",
        draft("same"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    f.db.save_behavior("behavior_actions", &draft("removed"))
        .unwrap();
    f.db.delete_behavior_versioned("behavior_actions", "removed", Some("1"))
        .unwrap();
    let mut disabled = draft("disabled");
    disabled["enabled"] = true.into();
    f.db.save_behavior("behavior_actions", &disabled).unwrap();
    let c = f.db.open(&f.db.automation_path()).unwrap();
    c.execute(
        "UPDATE behavior_actions SET enabled=0 WHERE id='disabled'",
        [],
    )
    .unwrap();
    f.db.save_behavior("behavior_events", &json!({"id":"same","name":"Owner event","enabled":false,"random":true,"actionIds":["missing","same","same","removed","disabled"]})).unwrap();
    let reviewed =
        f.db.behavior_review_versioned("behavior_events", "same")
            .unwrap();
    assert_eq!(reviewed.value["kind"], "event");
    assert_eq!(reviewed.value["record"]["name"], "Owner event");
    assert_eq!(
        reviewed.value["provenance"],
        json!({"authority":{"kind":"owner"},"generation":"1"})
    );
    let dependencies = reviewed.value["dependencies"].as_array().unwrap();
    assert_eq!(
        dependencies
            .iter()
            .map(|d| d["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["missing", "same", "removed", "disabled"]
    );
    assert_eq!(
        dependencies[0],
        json!({"id":"missing","record":null,"provenance":null})
    );
    assert_eq!(dependencies[1]["record"]["authority"], "owner");
    assert_eq!(
        dependencies[1]["provenance"],
        json!({"authority":{"kind":"delegated","principalId":identity.principal_id,"sessionId":identity.session_id},"generation":"1"})
    );
    assert!(dependencies[2]["record"].is_null());
    assert_eq!(dependencies[2]["provenance"]["authority"]["kind"], "owner");
    assert_eq!(dependencies[2]["provenance"]["generation"], "2");
    assert_eq!(dependencies[3]["record"]["enabled"], false);
    assert_eq!(
        reviewed
            .resource_versions
            .iter()
            .map(|v| (v.scope.as_str(), v.id.as_str(), v.revision.as_str()))
            .collect::<Vec<_>>(),
        [
            ("event", "same", "1"),
            ("action", "missing", "0"),
            ("action", "same", "1"),
            ("action", "removed", "2"),
            ("action", "disabled", "1")
        ]
    );
    let action =
        f.db.behavior_review_versioned("behavior_actions", "same")
            .unwrap();
    assert_eq!(action.value["kind"], "action");
    assert_eq!(
        action.value["dependencies"],
        json!([]),
        "actionIds in an action body are not event dependencies"
    );
    assert_eq!(action.resource_versions.len(), 1);
    assert_eq!(action.value["provenance"], dependencies[1]["provenance"]);
}

#[test]
fn owner_review_distinguishes_never_created_and_retained_missing_records() {
    let f = Fixture::new();
    let missing =
        f.db.behavior_review_versioned("behavior_events", "missing")
            .unwrap();
    assert_eq!(
        missing.value,
        json!({"kind":"event","id":"missing","record":null,"provenance":null,"dependencies":[]})
    );
    assert_eq!(missing.resource_versions[0].revision, "0");
    f.db.save_behavior("behavior_events", &draft("removed"))
        .unwrap();
    f.db.delete_behavior_versioned("behavior_events", "removed", Some("1"))
        .unwrap();
    let removed =
        f.db.behavior_review_versioned("behavior_events", "removed")
            .unwrap();
    assert!(removed.value["record"].is_null());
    assert_eq!(removed.value["dependencies"], json!([]));
    assert_eq!(
        removed.value["provenance"],
        json!({"authority":{"kind":"owner"},"generation":"2"})
    );
    assert_eq!(removed.resource_versions[0].revision, "2");
}

#[test]
fn owner_review_bodies_versions_authority_and_dependencies_share_the_same_read_snapshot() {
    let f = Fixture::new();
    f.db.save_behavior(
        "behavior_actions",
        &json!({"id":"same","name":"before","enabled":false}),
    )
    .unwrap();
    f.db.save_behavior(
        "behavior_events",
        &json!({"id":"same","name":"before","enabled":false,"actionIds":["same"]}),
    )
    .unwrap();
    let mut reader = f.db.open(&f.db.automation_path()).unwrap();
    reader.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
    let tx = reader.transaction().unwrap();
    tx.query_row("SELECT count(*) FROM behavior_events", [], |r| {
        r.get::<_, i64>(0)
    })
    .unwrap();
    f.db.behavior_transaction(|writer| {
        save_record(
            writer,
            "behavior_actions",
            &json!({"id":"same","name":"after","enabled":false}),
            Some("1"),
            BehaviorSave::Update,
        )?;
        save_record(
            writer,
            "behavior_events",
            &json!({"id":"same","name":"after","enabled":false,"actionIds":["same"]}),
            Some("1"),
            BehaviorSave::Update,
        )?;
        Ok(())
    })
    .unwrap();
    let before = review::snapshot(&tx, "behavior_events", "same").unwrap();
    assert_eq!(before.value["record"]["name"], "before");
    assert_eq!(before.value["dependencies"][0]["record"]["name"], "before");
    assert_eq!(before.value["provenance"]["generation"], "1");
    assert_eq!(
        before.value["dependencies"][0]["provenance"]["generation"],
        "1"
    );
    assert!(before.resource_versions.iter().all(|v| v.revision == "1"));
    tx.commit().unwrap();
    let after =
        f.db.behavior_review_versioned("behavior_events", "same")
            .unwrap();
    assert_eq!(after.value["record"]["name"], "after");
    assert_eq!(after.value["dependencies"][0]["record"]["name"], "after");
    assert_eq!(after.value["provenance"]["generation"], "2");
    assert_eq!(
        after.value["dependencies"][0]["provenance"]["generation"],
        "2"
    );
    assert!(after.resource_versions.iter().all(|v| v.revision == "2"));
}

#[test]
fn owner_review_rejects_malformed_noncanonical_and_excessive_dependencies_without_partial_results()
{
    let f = Fixture::new();
    for dependencies in [
        Value::Null,
        json!("same"),
        json!([7]),
        json!([""]),
        json!([" same"]),
        json!(["same*"]),
        json!(["same\n"]),
        json!(["x".repeat(129)]),
    ] {
        f.db.save_behavior(
            "behavior_events",
            &json!({"id":"same","name":"bad","enabled":false,"actionIds":dependencies}),
        )
        .unwrap();
        assert!(matches!(
            f.db.behavior_review_versioned("behavior_events", "same"),
            Err(DatabaseError::Invalid(_))
        ));
    }
    let ids = (0..512).map(|n| format!("missing-{n}")).collect::<Vec<_>>();
    f.db.save_behavior(
        "behavior_events",
        &json!({"id":"same","name":"bounded","enabled":false,"actionIds":ids}),
    )
    .unwrap();
    let accepted =
        f.db.behavior_review_versioned("behavior_events", "same")
            .unwrap();
    assert_eq!(
        accepted.value["dependencies"].as_array().unwrap().len(),
        512
    );
    assert_eq!(accepted.resource_versions.len(), 513);
    let mut ids = ids;
    ids.push("one-too-many".into());
    f.db.save_behavior(
        "behavior_events",
        &json!({"id":"same","name":"overfull","enabled":false,"actionIds":ids}),
    )
    .unwrap();
    assert!(matches!(
        f.db.behavior_review_versioned("behavior_events", "same"),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    let c = f.db.open(&f.db.automation_path()).unwrap();
    c.execute(
        "UPDATE behavior_events SET payload_json='[]' WHERE id='same'",
        [],
    )
    .unwrap();
    assert!(matches!(
        f.db.behavior_review_versioned("behavior_events", "same"),
        Err(DatabaseError::Invalid(_))
    ));
}

#[test]
fn owner_review_enforces_individual_and_aggregate_definition_budgets() {
    let f = Fixture::new();
    f.db.save_behavior(
        "behavior_events",
        &json!({"id":"same","name":"oversized","enabled":false,"source":"x".repeat(256*1024)}),
    )
    .unwrap();
    assert!(matches!(
        f.db.behavior_review_versioned("behavior_events", "same"),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    let mut ids = Vec::new();
    for n in 0..17 {
        let id = format!("large-{n}");
        f.db.save_behavior(
            "behavior_actions",
            &json!({"id":id,"name":"bounded","enabled":false,"source":"x".repeat(250_000)}),
        )
        .unwrap();
        ids.push(id);
    }
    f.db.save_behavior(
        "behavior_events",
        &json!({"id":"same","name":"bounded","enabled":false,"actionIds":&ids[..16]}),
    )
    .unwrap();
    assert_eq!(
        f.db.behavior_review_versioned("behavior_events", "same")
            .unwrap()
            .value["dependencies"]
            .as_array()
            .unwrap()
            .len(),
        16
    );
    f.db.save_behavior(
        "behavior_events",
        &json!({"id":"same","name":"overfull","enabled":false,"actionIds":ids}),
    )
    .unwrap();
    assert!(matches!(
        f.db.behavior_review_versioned("behavior_events", "same"),
        Err(DatabaseError::SnapshotTooLarge)
    ));
}

#[test]
fn owner_review_rejects_selected_catalog_corruption_without_blocking_unrelated_repair_reads() {
    for corruption in ["version", "sidecar", "deleted", "body", "authority"] {
        let f = Fixture::new();
        f.db.save_behavior("behavior_actions", &draft("broken"))
            .unwrap();
        f.db.save_behavior(
            "behavior_events",
            &json!({"id":"same","name":"review","enabled":false,"actionIds":["broken"]}),
        )
        .unwrap();
        f.db.save_behavior(
            "behavior_events",
            &json!({"id":"healthy","name":"review","enabled":false}),
        )
        .unwrap();
        let c = f.db.open(&f.db.automation_path()).unwrap();
        match corruption {
            "version" => {
                c.execute("DELETE FROM control_resource_versions WHERE kind='automation' AND scope='action' AND resource_id='broken'", []).unwrap();
            }
            "sidecar" => {
                c.execute(
                    "DELETE FROM behavior_provenance WHERE scope='action' AND resource_id='broken'",
                    [],
                )
                .unwrap();
            }
            "deleted" => {
                c.execute("UPDATE control_resource_versions SET deleted=1 WHERE kind='automation' AND scope='action' AND resource_id='broken'", []).unwrap();
            }
            "body" => {
                c.execute("DELETE FROM behavior_actions WHERE id='broken'", [])
                    .unwrap();
            }
            "authority" => {
                c.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE behavior_provenance SET generation=0 WHERE scope='action' AND resource_id='broken';").unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                f.db.behavior_review_versioned("behavior_actions", "broken"),
                Err(DatabaseError::Invalid(_))
            ),
            "{corruption}"
        );
        assert!(
            matches!(
                f.db.behavior_review_versioned("behavior_events", "same"),
                Err(DatabaseError::Invalid(_))
            ),
            "{corruption}"
        );
        assert!(f
            .db
            .behavior_review_versioned("behavior_events", "healthy")
            .is_ok());
    }
}

#[test]
fn adoption_checks_the_reviewed_dependency_versions_inside_its_transaction_and_hash() {
    use crate::control::{AutomationKind, AutomationMutation};
    let f = Fixture::new();
    let author = f.author();
    f.draft(
        &author,
        "behavior_actions",
        draft("same"),
        "0",
        BehaviorSave::Create,
    )
    .unwrap();
    let event =
        json!({"id":"rule","name":"proposal","enabled":false,"actionIds":["same","missing"]});
    f.draft(&author, "behavior_events", event, "0", BehaviorSave::Create)
        .unwrap();
    let original =
        f.db.behavior_review_versioned("behavior_events", "rule")
            .unwrap();
    let operation = |reviewed_versions| AutomationMutation::Adopt {
        kind: AutomationKind::Event,
        id: "rule".into(),
        reviewed_versions,
    };
    assert_eq!(
        operation(None).params(),
        json!({"kind":"event","id":"rule"}),
        "legacy adoption hashes remain unchanged"
    );
    let command = crate::db::CommandTransaction {
        principal_id: "local-owner".into(),
        session_id: "local-owner".into(),
        authorization: None,
        request: CommandRequest {
            command_id: "review-fence".into(),
            host_instance_id: f.service.instance_id().into(),
            expected_revision: "1".into(),
        },
        current_instance_id: f.service.instance_id().into(),
        max_ids_per_principal: 100,
    };
    let mut changed = draft("same");
    changed["name"] = "dependency changed after review".into();
    f.db.save_behavior_versioned(
        "behavior_actions",
        &changed,
        Some("1"),
        BehaviorSave::Update,
    )
    .unwrap();
    let before = f.outbox();
    let authority = f.provenance("behavior_events", "rule");
    assert!(matches!(
        f.db.execute_behavior_command(&operation(Some(original.resource_versions.clone())), &command),
        Err(DatabaseError::RevisionConflict(v)) if v.scope == "action" && v.id == "same" && v.revision == "2"
    ));
    assert_eq!(f.outbox(), before);
    assert_eq!(f.provenance("behavior_events", "rule"), authority);
    assert!(f
        .db
        .command_resource("local-owner", "review-fence")
        .unwrap()
        .is_none());
    let current =
        f.db.behavior_review_versioned("behavior_events", "rule")
            .unwrap()
            .resource_versions;
    let mut duplicate = current.clone();
    duplicate.push(current[1].clone());
    let mut extra = current.clone();
    extra.push(versions::version("behavior_actions", "extra", 0).unwrap());
    let mut wrong_family = current.clone();
    wrong_family[1].scope = "event".into();
    let mut wrong_revision = current.clone();
    wrong_revision[1].revision = "02".into();
    let mut reordered = current.clone();
    reordered.swap(1, 2);
    for invalid in [
        Vec::new(),
        duplicate,
        extra,
        wrong_family,
        wrong_revision,
        reordered,
    ] {
        assert!(matches!(
            f.db.execute_behavior_command(&operation(Some(invalid)), &command),
            Err(DatabaseError::Invalid(_))
        ));
        assert_eq!(f.outbox(), before);
        assert_eq!(f.provenance("behavior_events", "rule"), authority);
        assert!(f
            .db
            .command_resource("local-owner", "review-fence")
            .unwrap()
            .is_none());
    }
    let operation = operation(Some(current));
    let (adopted, fresh) = f.db.execute_behavior_command(&operation, &command).unwrap();
    assert!(fresh);
    assert_eq!(adopted.value, json!({"id":"rule","adopted":true}));
    assert_eq!(adopted.resource_versions[0].revision, "2");
    assert_eq!(f.outbox(), before + 1);
    assert_eq!(
        f.provenance("behavior_events", "rule"),
        BehaviorProvenance {
            authority: BehaviorAuthority::Owner,
            generation: 2
        }
    );
    assert_eq!(
        f.db.behavior_get_versioned("behavior_events", "rule")
            .unwrap()
            .value
            .unwrap()["enabled"],
        false
    );
    changed["name"] = "later dependency".into();
    f.db.save_behavior_versioned(
        "behavior_actions",
        &changed,
        Some("2"),
        BehaviorSave::Update,
    )
    .unwrap();
    let after = f.outbox();
    let (replayed, fresh) = f.db.execute_behavior_command(&operation, &command).unwrap();
    assert!(!fresh);
    assert_eq!(replayed, adopted);
    assert_eq!(f.outbox(), after);
    let changed_intent = AutomationMutation::Adopt {
        kind: AutomationKind::Event,
        id: "rule".into(),
        reviewed_versions: Some(original.resource_versions),
    };
    assert!(matches!(
        f.db.execute_behavior_command(&changed_intent, &command),
        Err(DatabaseError::CommandIdReused)
    ));
}

fn draft_command(
    f: &Fixture,
    auth: &Authorization,
    id: &str,
    expected: &str,
) -> crate::db::CommandTransaction {
    let identity = auth.snapshot().unwrap();
    crate::db::CommandTransaction {
        principal_id: identity.principal_id,
        session_id: identity.session_id,
        authorization: Some(auth.clone()),
        request: CommandRequest {
            command_id: id.into(),
            host_instance_id: f.service.instance_id().into(),
            expected_revision: expected.into(),
        },
        current_instance_id: f.service.instance_id().into(),
        max_ids_per_principal: 1000,
    }
}

#[test]
fn fresh_remote_drafts_store_editor_safe_defaults_without_changing_the_original_receipt_hash() {
    use crate::control::{AutomationKind, AutomationMutation};
    let f = Fixture::new();
    let auth = f.author();
    for (kind, table) in [
        (AutomationKind::Action, "behavior_actions"),
        (AutomationKind::Event, "behavior_events"),
    ] {
        let id = if kind == AutomationKind::Action {
            "action"
        } else {
            "event"
        };
        let mut body = json!({"id":id,"name":"Future plugin proposal","enabled":false,"source":"private-source","authority":{"kind":"owner"},"custom":{"keep":[1,2]}});
        body[if kind == AutomationKind::Action {
            "typeId"
        } else {
            "trigger"
        }] = "future.plugin.type".into();
        let operation = AutomationMutation::Create { kind, record: body };
        let command = draft_command(&f, &auth, &format!("create-{id}"), "0");
        let (created, fresh) = f.db.execute_behavior_command(&operation, &command).unwrap();
        assert!(fresh);
        assert_eq!(
            created.value,
            json!({"kind":id,"id":id,"enabled":false,"pendingReview":true})
        );
        let reviewed = f.db.behavior_review_versioned(table, id).unwrap();
        let body = &reviewed.value["record"];
        assert_eq!(body["source"], "private-source");
        assert_eq!(body["custom"], json!({"keep":[1,2]}));
        assert_eq!(
            reviewed.value["provenance"]["authority"]["kind"],
            "delegated"
        );
        if kind == AutomationKind::Action {
            assert_eq!(body["schemaVersion"], 2);
            assert_eq!(body["config"], json!({}));
        } else {
            assert_eq!(body["schemaVersion"], 1);
            assert_eq!(body["filters"], json!([]));
            assert_eq!(body["actionIds"], json!([]));
            assert_eq!(body["cooldownMs"], 0);
            assert_eq!(body["cooldownScope"], "global");
            assert_eq!(body["runMode"], "all");
        }
        let outbox = f.outbox();
        let (replayed, fresh) = f.db.execute_behavior_command(&operation, &command).unwrap();
        assert!(!fresh);
        assert_eq!(replayed, created);
        assert_eq!(f.outbox(), outbox);
        let changed_request = AutomationMutation::Create {
            kind,
            record: body.clone(),
        };
        assert!(matches!(
            f.db.execute_behavior_command(&changed_request, &command),
            Err(DatabaseError::CommandIdReused)
        ));
    }
    let operators = [
        "gte",
        "gt",
        "lte",
        "lt",
        "eq",
        "neq",
        "contains",
        "starts-with",
        "in",
        "is-true",
        "is-false",
    ];
    let filters = operators.iter().map(|op|json!({"path":"event.data.custom","operator":op,"values":["a","b"],"custom":"kept"})).collect::<Vec<_>>();
    let event = json!({"id":"event","name":"Reviewed filters","enabled":false,"trigger":"future.plugin.type","actionIds":["missing","missing"],"filters":filters,"cooldownMs":9_007_199_254_740_991_u64,"cooldownScope":"user","runMode":"random","schemaVersion":1});
    f.db.execute_behavior_command(
        &AutomationMutation::Update {
            kind: AutomationKind::Event,
            id: "event".into(),
            record: event,
        },
        &draft_command(&f, &auth, "update-event", "1"),
    )
    .unwrap();
    let event =
        f.db.behavior_review_versioned("behavior_events", "event")
            .unwrap()
            .value;
    assert_eq!(event["record"]["actionIds"], json!(["missing", "missing"]));
    assert_eq!(event["dependencies"].as_array().unwrap().len(), 1);
    for filter in event["record"]["filters"].as_array().unwrap() {
        assert_eq!(filter["value"], "");
        assert_eq!(filter["values"], json!(["a", "b"]));
        assert_eq!(filter["custom"], "kept");
    }
    let mut action =
        f.db.behavior_get_versioned("behavior_actions", "action")
            .unwrap()
            .value
            .unwrap();
    action["schemaVersion"] = 1.into();
    action["config"] = json!({"unknown":[true,1,{"value":"kept"}]});
    f.db.execute_behavior_command(
        &AutomationMutation::Update {
            kind: AutomationKind::Action,
            id: "action".into(),
            record: action.clone(),
        },
        &draft_command(&f, &auth, "update-action", "1"),
    )
    .unwrap();
    assert_eq!(
        f.db.behavior_get_versioned("behavior_actions", "action")
            .unwrap()
            .value
            .unwrap(),
        action
    );
}

#[test]
fn malformed_fresh_remote_drafts_never_change_body_authority_versions_or_any_outbox() {
    use crate::control::{AutomationKind, AutomationMutation};
    let f = Fixture::new();
    let auth = f.author();
    let mut counter = 0;
    for (kind, table, key) in [
        (AutomationKind::Action, "behavior_actions", "typeId"),
        (AutomationKind::Event, "behavior_events", "trigger"),
    ] {
        let valid = json!({"id":"candidate","name":"Draft","enabled":false,"typeId":"core.log","trigger":"tiktok.chat"});
        let command = draft_command(&f, &auth, &format!("valid-{key}"), "0");
        f.db.execute_behavior_command(
            &AutomationMutation::Create {
                kind,
                record: valid.clone(),
            },
            &command,
        )
        .unwrap();
        let before = f.db.behavior_get_versioned(table, "candidate").unwrap();
        let authority = f.provenance(table, "candidate");
        let outbox = f.outbox();
        let mut bad = Vec::new();
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(key);
        bad.push(missing);
        for value in [
            Value::Null,
            json!(7),
            json!(""),
            json!(" spaced"),
            json!("wild*"),
            json!("control\n"),
            json!("x".repeat(129)),
            json!("é".repeat(65)),
        ] {
            let mut body = valid.clone();
            body[key] = value;
            bad.push(body);
        }
        for value in [Value::Null, json!(0), json!(3), json!("1"), json!(-1)] {
            let mut body = valid.clone();
            body["schemaVersion"] = value;
            bad.push(body);
        }
        if kind == AutomationKind::Action {
            for config in [Value::Null, json!([]), json!("bad"), json!(7)] {
                let mut body = valid.clone();
                body["config"] = config;
                bad.push(body);
            }
        } else {
            let mut body = valid.clone();
            body["schemaVersion"] = 2.into();
            bad.push(body);
            for ids in [
                Value::Null,
                json!("missing"),
                json!([1]),
                json!([""]),
                json!([" space"]),
                json!(["wild*"]),
                json!(["control\n"]),
                json!(["x".repeat(129)]),
                json!(vec!["missing"; 513]),
            ] {
                let mut body = valid.clone();
                body["actionIds"] = ids;
                bad.push(body);
            }
            for filters in [
                Value::Null,
                json!({}),
                json!([null]),
                json!([{}]),
                json!([{"path":7,"operator":"eq"}]),
                json!([{"path":"event.data.x","operator":"unknown"}]),
                json!([{"path":"event.data.x","operator":"eq","value":null}]),
                json!([{"path":"event.data.x","operator":"eq","values":"bad"}]),
                json!([{"path":"event.data.x","operator":"eq","values":[7]}]),
                json!([{"path":"x".repeat(257),"operator":"eq"}]),
                json!((0..513)
                    .map(|_| json!({"path":"event.data.x","operator":"eq"}))
                    .collect::<Vec<_>>()),
                json!([{"path":"event.data.x","operator":"in","values":vec!["a";513]}]),
            ] {
                let mut body = valid.clone();
                body["filters"] = filters;
                bad.push(body);
            }
            for (key, values) in [
                (
                    "cooldownMs",
                    vec![
                        Value::Null,
                        json!("0"),
                        json!(-1),
                        json!(9_007_199_254_740_992_u64),
                    ],
                ),
                (
                    "cooldownScope",
                    vec![Value::Null, json!(7), json!("unknown")],
                ),
                ("runMode", vec![Value::Null, json!(7), json!("unknown")]),
            ] {
                for value in values {
                    let mut body = valid.clone();
                    body[key] = value;
                    bad.push(body);
                }
            }
        }
        for mut body in bad {
            for update in [false, true] {
                counter += 1;
                let id = if update { "candidate" } else { "new-candidate" };
                body["id"] = id.into();
                let op = if update {
                    AutomationMutation::Update {
                        kind,
                        id: id.into(),
                        record: body.clone(),
                    }
                } else {
                    AutomationMutation::Create {
                        kind,
                        record: body.clone(),
                    }
                };
                let command = draft_command(
                    &f,
                    &auth,
                    &format!("invalid-{counter}"),
                    if update { "1" } else { "0" },
                );
                assert!(
                    matches!(
                        f.db.execute_behavior_command(&op, &command),
                        Err(DatabaseError::Invalid(_))
                    ),
                    "{key}: {body}"
                );
                assert_eq!(
                    f.db.behavior_get_versioned(table, "candidate").unwrap(),
                    before
                );
                assert_eq!(f.provenance(table, "candidate"), authority);
                assert_eq!(f.outbox(), outbox);
                assert!(f
                    .db
                    .behavior_get_versioned(table, "new-candidate")
                    .unwrap()
                    .value
                    .is_none());
                assert!(f
                    .db
                    .behavior_provenance(table, "new-candidate")
                    .unwrap()
                    .is_none());
                assert!(f
                    .db
                    .command_resource(&command.principal_id, &command.request.command_id)
                    .unwrap()
                    .is_none());
                assert!(f.db.load_behavior_snapshot_versioned().is_ok());
            }
        }
    }
}

#[test]
fn remote_draft_target_denial_precedes_structure_validation_and_owner_legacy_writers_stay_unchanged(
) {
    use crate::control::{AutomationKind, AutomationMutation};
    let f = Fixture::new();
    let author = f.author();
    let foreign = f.author();
    for (kind, table) in [
        (AutomationKind::Action, "behavior_actions"),
        (AutomationKind::Event, "behavior_events"),
    ] {
        f.db.save_behavior(
            table,
            &json!({"id":"owner","name":"Legacy owner","enabled":false}),
        )
        .unwrap();
        f.draft(&foreign, table, draft("foreign"), "0", BehaviorSave::Create)
            .unwrap();
        for id in ["owner", "foreign"] {
            let operation = AutomationMutation::Update {
                kind,
                id: id.into(),
                record: json!({"id":id,"name":"Malformed","enabled":false,"typeId":null,"trigger":null,"config":[],"actionIds":null}),
            };
            let command = draft_command(
                &f,
                &author,
                &format!("denied-{}-{id}", versions::scope(table).unwrap()),
                "1",
            );
            assert!(matches!(
                f.db.execute_behavior_command(&operation, &command),
                Err(DatabaseError::CommandAuthorization(Error::Forbidden))
            ));
        }
        let operation = AutomationMutation::Create {
            kind,
            record: json!({"id":"owner-durable","name":"Legacy owner","enabled":false}),
        };
        let mut command = draft_command(
            &f,
            &author,
            &format!("owner-{}", versions::scope(table).unwrap()),
            "0",
        );
        command.authorization = None;
        command.principal_id = "local-owner".into();
        command.session_id = "local-owner".into();
        let (saved, fresh) = f.db.execute_behavior_command(&operation, &command).unwrap();
        assert!(fresh);
        assert_eq!(
            saved.value,
            json!({"id":"owner-durable","name":"Legacy owner","enabled":false})
        );
    }
}

#[test]
fn retained_pre_validation_remote_receipts_replay_without_revalidating_or_rewriting_old_definitions(
) {
    use crate::control::{AutomationKind, AutomationMutation};
    let f = Fixture::new();
    let author = f.author();
    let body = json!({"id":"historical","name":"Old draft","enabled":false,"filters":"old-unchecked-shape"});
    let operation = AutomationMutation::Create {
        kind: AutomationKind::Event,
        record: body.clone(),
    };
    let command = draft_command(&f, &author, "historical-create", "0");
    let hash =
        super::super::super::commands::payload_hash(operation.method(), &operation.params(), "0")
            .unwrap();
    let original=f.db.behavior_transaction(|tx| {
        let permit=authorize_authoring(tx,"behavior_events","historical",&author)?;
        let saved=save_record(tx,"behavior_events",&body,Some("0"),BehaviorSave::Create)?;
        mark_delegated_new(tx,"behavior_events","historical",&permit)?;
        super::super::super::commands::finish(tx,&command,&hash,operation.method(),"0",saved.map(|_|json!({"kind":"event","id":"historical","enabled":false,"pendingReview":true})))
    }).unwrap();
    let outbox = f.outbox();
    let authority = f.provenance("behavior_events", "historical");
    let (replayed, fresh) = f.db.execute_behavior_command(&operation, &command).unwrap();
    assert!(!fresh);
    assert_eq!(replayed, original);
    assert_eq!(f.outbox(), outbox);
    assert_eq!(f.provenance("behavior_events", "historical"), authority);
    assert_eq!(
        f.db.behavior_get_versioned("behavior_events", "historical")
            .unwrap()
            .value
            .unwrap(),
        body
    );
}
