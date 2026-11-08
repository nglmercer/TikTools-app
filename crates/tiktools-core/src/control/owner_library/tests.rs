use super::*;
use crate::{paths::AppPaths, AppCore, HostEmitter, NoopMediaHost};
use rusqlite::Connection;
use std::{path::PathBuf, sync::Arc};

const SOURCE: &str = "private-native-library-source";
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}
fn paths(root: PathBuf) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    }
}
fn open(paths: &AppPaths) -> Arc<AppCore> {
    Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        paths.clone(),
    ))
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    paths: AppPaths,
}
impl Fixture {
    fn new() -> Self {
        let paths = paths(std::env::temp_dir().join(format!(
            "tt-owner-library-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        )));
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        Self {
            core: Some(open(&paths)),
            paths,
        }
    }
    fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
    fn db(&self) -> Connection {
        Connection::open(self.paths.automation_database()).unwrap()
    }
    fn input(&self, mutation: OwnerLibraryMutation) -> OwnerLibrarySubmit {
        input(self.core(), mutation)
    }
    fn submit(&self, mutation: OwnerLibraryMutation) -> OwnerLibrarySubmission {
        self.core()
            .owner_library_submit(self.input(mutation))
            .unwrap()
    }
    fn reopen(&mut self) {
        drop(self.core.take());
        self.core = Some(open(&self.paths));
    }
    fn seed(&self) {
        self.core()
            .automation_create_versioned(AutomationKind::Action, action("existing-a"), Some("0"))
            .unwrap();
        self.core()
            .automation_create_versioned(AutomationKind::Action, action("loose-a"), Some("0"))
            .unwrap();
        self.core()
            .automation_create_versioned(
                AutomationKind::Event,
                event("existing-e", &["existing-a"]),
                Some("0"),
            )
            .unwrap();
        self.core().app_state_set("behavior.profiles.packs",&json!([{"id":"stage","name":"Stage","description":"","eventIds":["existing-e"],"actionIds":["existing-a"]}]).to_string()).unwrap();
        self.core()
            .app_state_set(
                "behavior.templates.custom",
                &json!([template("saved-template")]).to_string(),
            )
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.core.take());
        let _ = std::fs::remove_dir_all(&self.paths.root);
    }
}
fn action(id: &str) -> Value {
    json!({"id":id,"name":id,"enabled":false,"typeId":"core.log","config":{"message":SOURCE}})
}
fn event(id: &str, ids: &[&str]) -> Value {
    json!({"id":id,"name":id,"enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all","actionIds":ids})
}
fn entry(action_id: &str, event_id: &str) -> OwnerLibraryEntry {
    OwnerLibraryEntry {
        actions: vec![action(action_id)],
        event: event(event_id, &[action_id]),
    }
}
fn template(id: &str) -> Value {
    json!({"templateVersion":1,"id":id,"title":"Saved template","description":"","icon":"plugin","actions":[{"name":"Action","typeId":"core.log","enabled":false,"config":{"message":SOURCE}}],"event":{"name":"Event","enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all"}})
}
fn input(core: &AppCore, mutation: OwnerLibraryMutation) -> OwnerLibrarySubmit {
    let snapshot = core.owner_library_snapshot().unwrap();
    OwnerLibrarySubmit {
        mutation,
        expected_revision: snapshot.revision,
        definition_digest: snapshot.definition_digest,
        host_id: snapshot.host_id,
        host_instance_id: snapshot.host_instance_id,
    }
}
fn mutation(operation: OwnerLibraryOperation) -> OwnerLibraryMutation {
    match operation {
        OwnerLibraryOperation::CreateProfile => OwnerLibraryMutation::CreateProfile {
            id: "new-profile".into(),
            name: "Nuevo 🎵".into(),
            description: "Música".into(),
        },
        OwnerLibraryOperation::SwitchProfile => {
            OwnerLibraryMutation::SwitchProfile { id: "stage".into() }
        }
        OwnerLibraryOperation::DeleteProfile => {
            OwnerLibraryMutation::DeleteProfile { id: "stage".into() }
        }
        OwnerLibraryOperation::ApplyTemplate => OwnerLibraryMutation::ApplyTemplate {
            entry: entry("created-a", "created-e"),
            profile_id: "stage".into(),
            additional_entries: None,
            templates: None,
        },
        OwnerLibraryOperation::ImportProfile => OwnerLibraryMutation::ImportProfile {
            profile: OwnerLibraryProfile {
                id: "new-profile".into(),
                name: "New".into(),
                description: "".into(),
            },
            entries: vec![entry("imported-a", "imported-e")],
            templates: Some(vec![template("new-template")]),
        },
        OwnerLibraryOperation::AssignRecord => OwnerLibraryMutation::AssignRecord {
            kind: AutomationKind::Action,
            id: "loose-a".into(),
            profile_id: "stage".into(),
        },
        OwnerLibraryOperation::ImportTemplates => OwnerLibraryMutation::ImportTemplates {
            templates: vec![template("new-template")],
        },
        OwnerLibraryOperation::DeleteTemplate => OwnerLibraryMutation::DeleteTemplate {
            id: "saved-template".into(),
        },
    }
}

#[test]
fn every_library_operation_has_fresh_metadata_only_proof_and_retained_history() {
    let mut f = Fixture::new();
    f.seed();
    let operations = [
        OwnerLibraryOperation::CreateProfile,
        OwnerLibraryOperation::ApplyTemplate,
        OwnerLibraryOperation::ImportProfile,
        OwnerLibraryOperation::SwitchProfile,
        OwnerLibraryOperation::AssignRecord,
        OwnerLibraryOperation::ImportTemplates,
        OwnerLibraryOperation::DeleteTemplate,
        OwnerLibraryOperation::DeleteProfile,
    ];
    let mut ids = std::collections::BTreeSet::new();
    for operation in operations {
        let request = f.input(mutation(operation));
        let before = request.expected_revision.parse::<i64>().unwrap();
        let expected_hash = owner_library_command_hash(&request);
        let submitted = f.core().owner_library_submit(request).unwrap();
        assert!(submitted.state.pending.is_none());
        let row = &submitted.state.recent[0];
        assert_eq!(row.outcome, OwnerLibraryOutcome::Succeeded);
        assert_eq!(row.intent.operation, operation);
        assert_eq!(row.intent.command_hash, expected_hash);
        assert!(ids.insert(row.intent.command_id.clone()));
        let ack = submitted.acknowledgement.unwrap();
        assert_eq!(ack.value.revision, (before + 1).to_string());
        assert_eq!(
            ack.value.versions_digest,
            owner_library_versions_digest(&ack.resource_versions)
        );
        assert_eq!(ack.resource_versions[0].scope, "library");
        assert_eq!(ack.resource_versions[0].id, "catalog");
        assert_eq!(
            ack.value.affected_resources as usize,
            ack.resource_versions.len()
        );
        assert_eq!(ack.command.command_id, row.intent.command_id);
        assert!(!serde_json::to_string(&submitted.state)
            .unwrap()
            .contains(SOURCE));
        assert!(!serde_json::to_string(&ack).unwrap().contains(SOURCE));
        let snapshot = f.core().owner_library_snapshot().unwrap();
        assert_eq!(snapshot.revision, ack.value.revision);
        assert_eq!(
            owner_library_digest(&json!(snapshot.catalog)),
            ack.value.catalog_digest
        );
        assert_eq!(snapshot.definition_digest, ack.value.definition_digest);
    }
    f.reopen();
    let state = f.core().owner_library_resolve().unwrap();
    assert_eq!(state.recent.len(), 8);
    assert_eq!(
        state
            .recent
            .iter()
            .map(|row| row.intent.command_id.clone())
            .collect::<std::collections::BTreeSet<_>>(),
        ids
    );
    assert!(!serde_json::to_string(&state).unwrap().contains(SOURCE));
}

#[test]
fn stale_definition_universe_and_mid_graph_failure_reject_without_partial_state() {
    let f = Fixture::new();
    f.seed();
    let stale = f.input(mutation(OwnerLibraryOperation::SwitchProfile));
    f.core()
        .automation_create_versioned(AutomationKind::Action, action("phantom"), Some("0"))
        .unwrap();
    let before = f.core().owner_library_snapshot().unwrap();
    let rejected = f.core().owner_library_submit(stale).unwrap();
    assert!(rejected.acknowledgement.is_none());
    assert_eq!(
        rejected.state.recent[0].error_code.as_deref(),
        Some("revision_conflict")
    );
    assert_eq!(f.core().owner_library_snapshot().unwrap(), before);
    let failed = OwnerLibraryMutation::ImportProfile {
        profile: OwnerLibraryProfile {
            id: "rollback".into(),
            name: "Rollback".into(),
            description: "".into(),
        },
        entries: vec![
            entry("temporary-a", "temporary-e"),
            entry("existing-a", "collision-e"),
        ],
        templates: Some(vec![template("not-saved")]),
    };
    let rejected = f.submit(failed);
    assert_eq!(
        rejected.state.recent[0].outcome,
        OwnerLibraryOutcome::Rejected
    );
    assert_eq!(f.core().owner_library_snapshot().unwrap(), before);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM control_commands WHERE method='automation.library.apply'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert!(f
        .core()
        .automation_get(AutomationKind::Action, "temporary-a")
        .is_err());
}

#[test]
fn library_pending_blocks_all_other_journals_and_exact_unknown_review_never_dispatches() {
    use crate::control::{OwnerAdoptionSubmit, OwnerApprovalSubmit, OwnerDefinitionSubmit};
    let f = Fixture::new();
    f.seed();
    let request = f.input(mutation(OwnerLibraryOperation::CreateProfile));
    let intent = OwnerLibraryIntent {
        command_id: format!("owner-library-{}", "a".repeat(32)),
        operation: request.mutation.operation(),
        target: request.mutation.target().into(),
        expected_revision: request.expected_revision.clone(),
        definition_digest: request.definition_digest.clone(),
        command_hash: owner_library_command_hash(&request),
        host_instance_id: request.host_instance_id.clone(),
        created_at: library_now(),
    };
    f.core()
        .db
        .owner_library_begin(&request.host_id, &intent, &request)
        .unwrap();
    assert_eq!(
        f.core()
            .owner_library_submit(request.clone())
            .unwrap_err()
            .code,
        "library_pending"
    );
    assert_eq!(
        f.core()
            .owner_definition_submit(OwnerDefinitionSubmit::Disable {
                kind: AutomationKind::Action,
                id: "existing-a".into(),
                expected_revision: "1".into(),
                host_id: request.host_id.clone(),
                host_instance_id: request.host_instance_id.clone()
            })
            .unwrap_err()
            .code,
        "command_pending"
    );
    assert_eq!(
        f.core()
            .owner_approval_submit(OwnerApprovalSubmit::Revoke {
                kind: AutomationKind::Action,
                id: "existing-a".into(),
                expected_revision: "1".into(),
                host_id: request.host_id.clone(),
                host_instance_id: request.host_instance_id.clone(),
                acknowledge_revocation: true
            })
            .unwrap_err()
            .code,
        "command_pending"
    );
    let reviewed = f
        .core()
        .automation_review(AutomationKind::Action, "existing-a")
        .unwrap();
    assert_eq!(
        f.core()
            .owner_adoption_submit(OwnerAdoptionSubmit {
                kind: AutomationKind::Action,
                id: "existing-a".into(),
                expected_revision: "1".into(),
                host_id: request.host_id.clone(),
                host_instance_id: request.host_instance_id.clone(),
                reviewed_versions: reviewed.resource_versions,
                acknowledge_authority: true
            })
            .unwrap_err()
            .code,
        "command_pending"
    );
    assert_eq!(
        f.core().owner_library_resolve().unwrap().pending,
        Some(intent.clone())
    );
    assert_eq!(
        f.core()
            .owner_library_review_unknown(OwnerLibraryReviewUnknown {
                command_id: "stale".into(),
                acknowledge_unknown: true
            })
            .unwrap_err()
            .code,
        "stale_view"
    );
    assert_eq!(
        f.core()
            .owner_library_review_unknown(OwnerLibraryReviewUnknown {
                command_id: intent.command_id.clone(),
                acknowledge_unknown: false
            })
            .unwrap_err()
            .code,
        "confirmation_required"
    );
    let archived = f
        .core()
        .owner_library_review_unknown(OwnerLibraryReviewUnknown {
            command_id: intent.command_id,
            acknowledge_unknown: true,
        })
        .unwrap();
    assert!(archived.pending.is_none());
    assert_eq!(
        archived.recent[0].outcome,
        OwnerLibraryOutcome::ReviewedUnknown
    );
    assert!(!f
        .core()
        .owner_library_snapshot()
        .unwrap()
        .catalog
        .packs
        .iter()
        .any(|pack| pack.id == "new-profile"));
}

#[test]
fn canonical_numbers_keys_and_exact_large_integers_match_the_browser_contract() {
    let value: Value = serde_json::from_str(
        r#"{"2":1.0,"10":0.000001,"𐀀":-0.0,"":1e-7,"large":18446744073709551615}"#,
    )
    .unwrap();
    assert_eq!(owner_library_canonical_json(&value),"{\"10\":0.000001,\"2\":1,\"large\":18446744073709551615,\"\u{e000}\":1e-7,\"\u{10000}\":0}");
    assert_eq!(
        owner_library_digest(&json!({"a":1.0})),
        owner_library_digest(&json!({"a":1}))
    );
    assert_ne!(
        owner_library_digest(&json!({"a":9007199254740992u64})),
        owner_library_digest(&json!({"a":9007199254740993u64}))
    );
}

#[test]
fn committed_effect_survives_history_failure_and_corrupt_proof_cannot_be_archived() {
    let f = Fixture::new();
    f.seed();
    f.db().execute_batch("CREATE TRIGGER fail_library_history BEFORE UPDATE ON owner_automation_library WHEN NEW.pending=0 BEGIN SELECT RAISE(ABORT,'history fixture'); END;").unwrap();
    let request = f.input(mutation(OwnerLibraryOperation::CreateProfile));
    assert_eq!(
        f.core().owner_library_submit(request).unwrap_err().code,
        "library_journal_unavailable"
    );
    let pending = f.core().owner_library_state().unwrap().pending.unwrap();
    let snapshot = f.core().owner_library_snapshot().unwrap();
    assert!(snapshot
        .catalog
        .packs
        .iter()
        .any(|pack| pack.id == "new-profile"));
    f.db()
        .execute_batch("DROP TRIGGER fail_library_history;")
        .unwrap();
    f.db()
        .execute(
            "UPDATE control_commands SET payload_hash=?2 WHERE command_id=?1",
            rusqlite::params![pending.command_id, "b".repeat(64)],
        )
        .unwrap();
    assert_eq!(
        f.core().owner_library_resolve().unwrap_err().code,
        "library_journal_unavailable"
    );
    assert_eq!(
        f.core()
            .owner_library_review_unknown(OwnerLibraryReviewUnknown {
                command_id: pending.command_id.clone(),
                acknowledge_unknown: true
            })
            .unwrap_err()
            .code,
        "library_journal_unavailable"
    );
    assert_eq!(
        f.core().owner_library_state().unwrap().pending,
        Some(pending.clone())
    );
    f.db()
        .execute(
            "UPDATE control_commands SET payload_hash=?2 WHERE command_id=?1",
            rusqlite::params![pending.command_id, pending.command_hash],
        )
        .unwrap();
    let stored: String = f
        .db()
        .query_row(
            "SELECT payload_json FROM owner_automation_library WHERE command_id=?1",
            [&pending.command_id],
            |row| row.get(0),
        )
        .unwrap();
    f.db().execute("UPDATE owner_automation_library SET payload_json=json_set(payload_json,'$.intent.target','changed-target','$.intent.operation','switch-profile') WHERE command_id=?1",[&pending.command_id]).unwrap();
    assert_eq!(
        f.core().owner_library_resolve().unwrap_err().code,
        "library_journal_unavailable"
    );
    f.db()
        .execute(
            "UPDATE owner_automation_library SET payload_json=?2 WHERE command_id=?1",
            rusqlite::params![pending.command_id, stored],
        )
        .unwrap();
    let recovered = f
        .core()
        .owner_library_review_unknown(OwnerLibraryReviewUnknown {
            command_id: pending.command_id,
            acknowledge_unknown: true,
        })
        .unwrap();
    assert!(recovered.pending.is_none());
    assert_eq!(recovered.recent[0].outcome, OwnerLibraryOutcome::Succeeded);
    assert_eq!(f.core().owner_library_snapshot().unwrap(), snapshot);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM control_commands WHERE method='automation.library.apply'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn legacy_projection_migrates_once_and_failed_compatibility_write_never_reports_success() {
    let f = Fixture::new();
    let points = Connection::open(f.paths.points_database()).unwrap();
    let original =
        json!([{"id":"a","name":"Música 🎵","description":"","eventIds":[],"actionIds":[]}])
            .to_string();
    points
        .execute(
            "INSERT INTO app_state(key,value,updated_at) VALUES ('behavior.profiles.packs',?1,1)",
            [&original],
        )
        .unwrap();
    let key = "behavior.profiles.packs".to_owned();
    let projected = f
        .core()
        .app_state_get(Some(std::slice::from_ref(&key)))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&projected[&key]).unwrap(),
        serde_json::from_str::<Value>(&original).unwrap()
    );
    assert_eq!(
        points
            .query_row(
                "SELECT value FROM app_state WHERE key='behavior.profiles.packs'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        original
    );
    points
        .execute(
            "UPDATE app_state SET value='[]' WHERE key='behavior.profiles.packs'",
            [],
        )
        .unwrap();
    assert_eq!(
        f.core()
            .app_state_get(Some(std::slice::from_ref(&key)))
            .unwrap(),
        projected
    );
    let catalog_before = f.core().owner_library_snapshot().unwrap();
    f.db().execute_batch("CREATE TRIGGER fail_catalog BEFORE UPDATE ON automation_library_catalog BEGIN SELECT RAISE(ABORT,'catalog fixture'); END;").unwrap();
    assert!(f.core().app_state_set(&key, "[]").is_err());
    assert_eq!(
        f.core()
            .app_state_get(Some(std::slice::from_ref(&key)))
            .unwrap(),
        projected
    );
    assert_eq!(f.core().owner_library_snapshot().unwrap(), catalog_before);
    f.db().execute_batch("DROP TRIGGER fail_catalog;").unwrap();
    let replacement =
        json!([{"id":"b","name":"New","description":"","eventIds":[],"actionIds":[]}]).to_string();
    f.core().app_state_set(&key, &replacement).unwrap();
    assert_eq!(
        f.core().owner_library_snapshot().unwrap().catalog.packs[0].id,
        "b"
    );
    assert_eq!(
        points
            .query_row(
                "SELECT value FROM app_state WHERE key='behavior.profiles.packs'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "[]"
    );
}

#[test]
fn restarting_the_host_rejects_an_old_library_review_without_creating_an_intent() {
    let mut f = Fixture::new();
    f.seed();
    let old = f.input(mutation(OwnerLibraryOperation::SwitchProfile));
    f.reopen();
    assert_eq!(
        f.core().owner_library_submit(old).unwrap_err().code,
        "host_restarted"
    );
    assert!(f.core().owner_library_state().unwrap().recent.is_empty());
    assert!(f.core().owner_library_state().unwrap().pending.is_none());
}

#[test]
#[ignore = "only invoked by the native library process-death parent"]
fn crash_child() {
    let root = std::env::var_os("TIKTOOLS_OWNER_LIBRARY_ROOT").expect("isolated child root");
    let paths = paths(root.into());
    let core = open(&paths);
    let operation: OwnerLibraryOperation = serde_json::from_value(json!(std::env::var(
        "TIKTOOLS_OWNER_LIBRARY_OPERATION"
    )
    .unwrap()))
    .unwrap();
    let request = input(&core, mutation(operation));
    core.owner_library_submit(request).unwrap();
    panic!("library crash hook did not terminate the child");
}

#[test]
fn actual_process_termination_for_all_library_operations_recovers_original_receipts_without_replay()
{
    for operation in [
        OwnerLibraryOperation::CreateProfile,
        OwnerLibraryOperation::SwitchProfile,
        OwnerLibraryOperation::DeleteProfile,
        OwnerLibraryOperation::ApplyTemplate,
        OwnerLibraryOperation::ImportProfile,
        OwnerLibraryOperation::AssignRecord,
        OwnerLibraryOperation::ImportTemplates,
        OwnerLibraryOperation::DeleteTemplate,
    ] {
        for (stage, committed, code) in [
            ("before_intent", false, 90),
            ("after_intent", false, 90),
            ("before_dispatch", false, 90),
            ("business_before", false, 91),
            ("business_after", true, 92),
            ("after_effect", true, 90),
            ("after_record", true, 90),
        ] {
            let mut f = Fixture::new();
            f.seed();
            let before = f
                .core()
                .owner_library_snapshot()
                .unwrap()
                .revision
                .parse::<i64>()
                .unwrap();
            drop(f.core.take());
            let mut child = std::process::Command::new(std::env::current_exe().unwrap());
            child
                .args([
                    "--exact",
                    "control::owner_library::tests::crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("TIKTOOLS_OWNER_LIBRARY_ROOT", &f.paths.root)
                .env(
                    "TIKTOOLS_OWNER_LIBRARY_OPERATION",
                    serde_json::to_value(operation).unwrap().as_str().unwrap(),
                );
            if let Some(stage) = stage.strip_prefix("business_") {
                child.env("TIKTOOLS_BEHAVIOR_CRASH", stage);
            } else {
                child.env("TIKTOOLS_OWNER_LIBRARY_CRASH", stage);
            }
            let output = child.output().unwrap();
            assert_eq!(
                output.status.code(),
                Some(code),
                "{operation:?}/{stage}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            f.core = Some(open(&f.paths));
            let mut newer = None;
            if committed {
                // A newer owner write (including recreation after profile
                // deletion) precedes outcome lookup, so replay would be visible.
                let old = f
                    .core()
                    .automation_get_versioned(AutomationKind::Action, "existing-a")
                    .unwrap();
                let revision = old.resource_versions[0].revision.clone();
                let mut source = action("existing-a");
                source["name"] = json!("newer after process death");
                if old.value.is_some() {
                    f.core()
                        .automation_update_versioned(
                            AutomationKind::Action,
                            "existing-a",
                            source.clone(),
                            Some(&revision),
                        )
                        .unwrap();
                } else {
                    f.core()
                        .automation_create_versioned(
                            AutomationKind::Action,
                            source.clone(),
                            Some(&revision),
                        )
                        .unwrap();
                }
                newer = Some(source);
            }
            let resolved = f.core().owner_library_resolve().unwrap();
            if committed {
                assert!(resolved.pending.is_none());
                assert_eq!(resolved.recent.len(), 1);
                let row = &resolved.recent[0];
                assert_eq!(row.outcome, OwnerLibraryOutcome::Succeeded);
                assert_eq!(
                    row.receipt.as_ref().unwrap().revision,
                    (before + 1).to_string()
                );
                assert_eq!(
                    f.core()
                        .automation_get(AutomationKind::Action, "existing-a")
                        .unwrap(),
                    newer.unwrap()
                );
                let snapshot = f.core().owner_library_snapshot().unwrap();
                assert_ne!(
                    row.receipt.as_ref().unwrap().definition_digest,
                    snapshot.definition_digest
                );
            } else if stage == "before_intent" {
                assert!(resolved.pending.is_none());
                assert!(resolved.recent.is_empty());
            } else {
                let pending = resolved.pending.unwrap();
                let archived = f
                    .core()
                    .owner_library_review_unknown(OwnerLibraryReviewUnknown {
                        command_id: pending.command_id,
                        acknowledge_unknown: true,
                    })
                    .unwrap();
                assert_eq!(
                    archived.recent[0].outcome,
                    OwnerLibraryOutcome::ReviewedUnknown
                );
            }
            assert_eq!(
                f.core().owner_library_snapshot().unwrap().revision,
                (before + i64::from(committed)).to_string()
            );
            assert_eq!(f.db().query_row("SELECT count(*) FROM control_commands WHERE method='automation.library.apply'",[],|row|row.get::<_,i64>(0)).unwrap(),i64::from(committed));
            assert_eq!(f.db().query_row("SELECT count(*) FROM control_command_outbox o JOIN control_command_resources l ON l.event_id=o.event_id WHERE o.method='automation.library.apply'",[],|row|row.get::<_,i64>(0)).unwrap(),i64::from(committed));
        }
    }
}
