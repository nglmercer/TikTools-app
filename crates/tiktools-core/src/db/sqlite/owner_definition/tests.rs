use super::*;
use crate::{
    control::{
        owner_approval::ApprovalCommand, ApprovalRevokeParams, OwnerAdoptionIntent,
        OwnerApprovalIntent, OwnerApprovalOperation,
    },
    db::CommandTransaction,
    paths::AppPaths,
};
use serde_json::json;
use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
};

struct Fixture {
    db: DatabaseManager,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-owner-definition-db-{}-{}",
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
        // Never import a developer's legacy workspace databases.
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
    fn state(&self) -> OwnerDefinitionState {
        self.db.owner_definition_state("host", "instance").unwrap()
    }
    fn execute(&self, intent: &OwnerDefinitionIntent, operation: &AutomationMutation) {
        self.db
            .execute_behavior_command(
                operation,
                &CommandTransaction {
                    principal_id: "local-owner".into(),
                    session_id: "local-owner".into(),
                    authorization: None,
                    request: CommandRequest {
                        command_id: intent.command_id.clone(),
                        host_instance_id: intent.host_instance_id.clone(),
                        expected_revision: intent.expected_revision.clone(),
                    },
                    current_instance_id: intent.host_instance_id.clone(),
                    max_ids_per_principal: 10_000,
                },
            )
            .unwrap();
    }
    fn finish(
        &self,
        operation: &AutomationMutation,
        expected: &str,
        serial: u128,
    ) -> OwnerDefinitionRecord {
        let intent = intent(operation, expected, serial);
        self.db
            .owner_definition_begin("host", &intent, operation)
            .unwrap();
        self.db
            .owner_definition_validate_dispatch(operation, expected)
            .unwrap();
        self.execute(&intent, operation);
        self.db
            .owner_definition_resolve("host", false, None)
            .unwrap();
        let state = self.state();
        assert!(state.pending.is_none());
        let record = state
            .recent
            .into_iter()
            .find(|r| r.intent.command_id == intent.command_id)
            .unwrap();
        assert_eq!(record.outcome, OwnerDefinitionOutcome::Succeeded);
        record
    }
    fn body(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> crate::collaboration::Versioned<Option<Value>> {
        self.db.behavior_get_versioned(kind.table(), id).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn create(kind: AutomationKind, id: &str, name: &str) -> AutomationMutation {
    AutomationMutation::Create {
        kind,
        record: json!({"id":id,"name":name,"enabled":false,"typeId":"core.log","config":{"message":"OWNER-SOURCE-MUST-NOT-BE-JOURNALED"}}),
    }
}
fn intent(operation: &AutomationMutation, expected: &str, serial: u128) -> OwnerDefinitionIntent {
    let (kind, id) = operation.target();
    OwnerDefinitionIntent {
        command_id: format!("owner-definition-{serial:032x}"),
        operation: operation_kind(operation).unwrap(),
        kind,
        id: id.into(),
        expected_revision: expected.into(),
        host_instance_id: "instance".into(),
        record_digest: match operation {
            AutomationMutation::Create { record, .. }
            | AutomationMutation::Update { record, .. } => Some(definition_record_digest(record)),
            _ => None,
        },
        created_at: definition_now(),
    }
}

#[test]
fn all_five_operations_and_tombstone_recreation_recover_exact_original_receipts_without_source() {
    let f = Fixture::new();
    let mut serial = 1;
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        let first = f.finish(&create(kind, "same", "First"), "0", serial);
        serial += 1;
        let receipt = first.receipt.unwrap();
        assert_eq!(receipt.revision, "1");
        assert_eq!(receipt.record_digest, first.intent.record_digest);
        assert_eq!(receipt.enabled, Some(false));
        let enabled = f.finish(
            &AutomationMutation::SetEnabled {
                kind,
                id: "same".into(),
                enabled: true,
            },
            "1",
            serial,
        );
        serial += 1;
        assert_eq!(enabled.receipt.as_ref().unwrap().enabled, Some(true));
        assert_eq!(enabled.receipt.unwrap().revision, "2");
        let updated = f.finish(
            &AutomationMutation::Update {
                kind,
                id: "same".into(),
                record: json!({"id":"same","name":"Changed","enabled":true}),
            },
            "2",
            serial,
        );
        serial += 1;
        assert_eq!(updated.receipt.unwrap().revision, "3");
        let disabled = f.finish(
            &AutomationMutation::SetEnabled {
                kind,
                id: "same".into(),
                enabled: false,
            },
            "3",
            serial,
        );
        serial += 1;
        assert_eq!(disabled.receipt.as_ref().unwrap().enabled, Some(false));
        assert_eq!(disabled.receipt.unwrap().revision, "4");
        let deleted = f.finish(
            &AutomationMutation::Delete {
                kind,
                id: "same".into(),
            },
            "4",
            serial,
        );
        serial += 1;
        assert_eq!(deleted.receipt.as_ref().unwrap().deleted, Some(true));
        assert_eq!(deleted.receipt.unwrap().revision, "5");
        let absent = f.finish(
            &AutomationMutation::Delete {
                kind,
                id: "same".into(),
            },
            "5",
            serial,
        );
        serial += 1;
        assert_eq!(absent.receipt.as_ref().unwrap().deleted, Some(false));
        assert_eq!(absent.receipt.unwrap().revision, "5");
        assert_eq!(
            f.finish(&create(kind, "same", "Recreated"), "5", serial)
                .receipt
                .unwrap()
                .revision,
            "6"
        );
        serial += 1;
        f.db.owner_definition_resolve("host", false, None).unwrap();
        assert_eq!(f.body(kind, "same").resource_versions[0].revision, "6");
    }
    let connection = f.connection();
    let mut query = connection
        .prepare("SELECT payload_json FROM owner_automation_definitions")
        .unwrap();
    let payloads = query
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(payloads.len(), 14);
    assert!(payloads
        .iter()
        .all(|s| !s.contains("OWNER-SOURCE-MUST-NOT-BE-JOURNALED")));
}

#[test]
fn missing_updates_reject_and_known_original_success_wins_over_unknown_or_rejection() {
    let f = Fixture::new();
    let missing = AutomationMutation::Update {
        kind: AutomationKind::Action,
        id: "missing".into(),
        record: json!({"id":"missing","name":"Not a create"}),
    };
    let i = intent(&missing, "0", 1);
    f.db.owner_definition_begin("host", &i, &missing).unwrap();
    assert!(matches!(
        f.db.execute_behavior_command(
            &missing,
            &CommandTransaction {
                principal_id: "local-owner".into(),
                session_id: "local-owner".into(),
                authorization: None,
                request: CommandRequest {
                    command_id: i.command_id.clone(),
                    host_instance_id: "instance".into(),
                    expected_revision: "0".into()
                },
                current_instance_id: "instance".into(),
                max_ids_per_principal: 100
            }
        ),
        Err(DatabaseError::NotFound)
    ));
    f.db.owner_definition_resolve("host", false, Some("not_found"))
        .unwrap();
    assert_eq!(
        f.state().recent[0].outcome,
        OwnerDefinitionOutcome::Rejected
    );
    assert!(f.body(AutomationKind::Action, "missing").value.is_none());
    let operation = create(AutomationKind::Action, "a", "Saved");
    let i = intent(&operation, "0", 2);
    f.db.owner_definition_begin("host", &i, &operation).unwrap();
    f.execute(&i, &operation);
    f.db.owner_definition_resolve("host", true, Some("revision_conflict"))
        .unwrap();
    assert_eq!(
        f.state().recent[0].outcome,
        OwnerDefinitionOutcome::Succeeded
    );
    assert_eq!(f.state().recent[0].receipt.as_ref().unwrap().revision, "1");
}

#[test]
fn unresolved_and_expired_intents_never_dispatch_and_require_explicit_unknown_review() {
    let f = Fixture::new();
    let operation = create(AutomationKind::Action, "a", "Pending");
    let i = intent(&operation, "0", 1);
    f.db.owner_definition_begin("host", &i, &operation).unwrap();
    f.db.owner_definition_resolve("host", false, None).unwrap();
    assert_eq!(f.state().pending, Some(i.clone()));
    assert!(f.body(AutomationKind::Action, "a").value.is_none());
    f.db.owner_definition_resolve("host", true, None).unwrap();
    assert_eq!(
        f.state().recent[0].outcome,
        OwnerDefinitionOutcome::ReviewedUnknown
    );
    assert!(f.body(AutomationKind::Action, "a").value.is_none());

    let next = intent(&operation, "0", 2);
    f.db.owner_definition_begin("host", &next, &operation)
        .unwrap();
    f.execute(&next, &operation);
    f.connection().execute("UPDATE control_commands SET committed_at=0 WHERE principal_id='local-owner' AND command_id=?1",[&next.command_id]).unwrap();
    f.db.prune_command_outcomes().unwrap();
    f.db.owner_definition_resolve("host", false, None).unwrap();
    assert_eq!(f.state().pending, Some(next.clone()));
    f.db.owner_definition_resolve("host", true, None).unwrap();
    assert_eq!(
        f.state().recent[0].outcome,
        OwnerDefinitionOutcome::ReviewedUnknown
    );
    assert_eq!(
        f.body(AutomationKind::Action, "a").resource_versions[0].revision,
        "1"
    );
    let retained:(bool,bool)=f.connection().query_row("SELECT outcome_expired,result_json IS NULL AND versions_json IS NULL FROM control_commands WHERE principal_id='local-owner' AND command_id=?1",[&next.command_id],|row|Ok((row.get(0)?,row.get(1)?))).unwrap();
    assert_eq!(retained, (true, true));
    assert_eq!(
        f.connection()
            .query_row("SELECT count(*) FROM control_commands", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn result_persistence_failure_keeps_pending_and_recovery_does_not_overwrite_newer_source() {
    let f = Fixture::new();
    let operation = create(AutomationKind::Action, "a", "Original");
    let i = intent(&operation, "0", 1);
    f.connection().execute_batch("CREATE TRIGGER reject_definition_begin BEFORE INSERT ON owner_automation_definitions BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(f.db.owner_definition_begin("host", &i, &operation).is_err());
    assert!(f.state().pending.is_none());
    f.connection()
        .execute_batch("DROP TRIGGER reject_definition_begin")
        .unwrap();
    f.db.owner_definition_begin("host", &i, &operation).unwrap();
    f.execute(&i, &operation);
    f.connection().execute_batch("CREATE TRIGGER reject_definition_completion BEFORE UPDATE ON owner_automation_definitions BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(f.db.owner_definition_resolve("host", false, None).is_err());
    assert_eq!(f.state().pending, Some(i));
    f.db.save_behavior_versioned(
        "behavior_actions",
        &json!({"id":"a","name":"Newer source","enabled":true}),
        Some("1"),
        crate::db::BehaviorSave::Update,
    )
    .unwrap();
    f.connection()
        .execute_batch("DROP TRIGGER reject_definition_completion")
        .unwrap();
    f.db.owner_definition_resolve("host", false, None).unwrap();
    assert_eq!(f.state().recent[0].receipt.as_ref().unwrap().revision, "1");
    let current = f.body(AutomationKind::Action, "a");
    assert_eq!(current.value.unwrap()["name"], "Newer source");
    assert_eq!(current.resource_versions[0].revision, "2");
}

#[test]
fn action_delete_recovers_more_than_513_real_cascaded_event_versions_in_order() {
    let f = Fixture::new();
    f.finish(&create(AutomationKind::Action, "a", "Action"), "0", 1);
    let mut db = f.connection();
    let tx = db.transaction().unwrap();
    // Seed one consistent existing snapshot in one transaction, rather than
    // spending 600 fsyncs on setup. The delete itself uses the real writer.
    for index in 0..600 {
        let id = format!("event-{index:04}");
        let value =
            json!({"id":id,"name":id,"enabled":false,"trigger":"tiktok.chat","actionIds":["a"]});
        tx.execute("INSERT INTO behavior_events(id,name,enabled,payload_json,created_at,updated_at) VALUES (?1,?1,0,?2,1,1)",params![id,value.to_string()]).unwrap();
        tx.execute("INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted) VALUES ('automation','event',?1,1,0)",[&id]).unwrap();
        tx.execute("INSERT INTO behavior_provenance(scope,resource_id,authority,generation) VALUES ('event',?1,'owner',1)",[&id]).unwrap();
    }
    tx.commit().unwrap();
    let operation = AutomationMutation::Delete {
        kind: AutomationKind::Action,
        id: "a".into(),
    };
    let record = f.finish(&operation, "1", 2);
    assert_eq!(record.receipt.as_ref().unwrap().affected_resources, 601);
    assert_eq!(record.receipt.as_ref().unwrap().deleted, Some(true));
    assert_eq!(record.receipt.unwrap().revision, "2");
    for id in ["event-0000", "event-0599"] {
        let value = f.body(AutomationKind::Event, id);
        assert_eq!(value.resource_versions[0].revision, "2");
        assert_eq!(value.value.unwrap()["actionIds"], json!([]));
    }
}

#[test]
fn cross_journal_pending_admission_is_reciprocal_and_begin_is_atomic() {
    let f = Fixture::new();
    let operation = create(AutomationKind::Action, "a", "Pending");
    let definition = intent(&operation, "0", 1);
    let adoption = OwnerAdoptionIntent {
        command_id: format!("owner-adopt-{:032x}", 1),
        kind: AutomationKind::Action,
        id: "a".into(),
        expected_revision: "0".into(),
        host_instance_id: "instance".into(),
        created_at: definition_now(),
    };
    let adopt = AutomationMutation::Adopt {
        kind: AutomationKind::Action,
        id: "a".into(),
        reviewed_versions: None,
    };
    let approval = OwnerApprovalIntent {
        command_id: format!("owner-approval-{:032x}", 1),
        operation: OwnerApprovalOperation::Revoke,
        kind: AutomationKind::Action,
        id: "a".into(),
        expected_revision: "1".into(),
        host_instance_id: "instance".into(),
        created_at: definition_now(),
    };
    let revoke = ApprovalCommand::Revoke(ApprovalRevokeParams {
        kind: AutomationKind::Action,
        id: "a".into(),
        acknowledge_revocation: true,
    });
    f.db.owner_definition_begin("host", &definition, &operation)
        .unwrap();
    assert!(matches!(
        f.db.owner_adoption_begin("host", &adoption, &adopt),
        Err(DatabaseError::OwnerAdoptionPending)
    ));
    assert!(matches!(
        f.db.owner_approval_begin("host", &approval, &revoke),
        Err(DatabaseError::OwnerAdoptionPending)
    ));
    f.db.owner_definition_resolve("host", true, None).unwrap();
    f.db.owner_adoption_begin("host", &adoption, &adopt)
        .unwrap();
    assert!(matches!(
        f.db.owner_definition_begin("host", &definition, &operation),
        Err(DatabaseError::OwnerAdoptionPending)
    ));
    assert!(matches!(
        f.db.owner_approval_begin("host", &approval, &revoke),
        Err(DatabaseError::OwnerAdoptionPending)
    ));
    f.db.owner_adoption_resolve("host", true, None).unwrap();
    f.db.owner_approval_begin("host", &approval, &revoke)
        .unwrap();
    assert!(matches!(
        f.db.owner_definition_begin("host", &definition, &operation),
        Err(DatabaseError::OwnerAdoptionPending)
    ));
    assert!(matches!(
        f.db.owner_adoption_begin("host", &adoption, &adopt),
        Err(DatabaseError::OwnerAdoptionPending)
    ));
    f.db.owner_approval_resolve("host", true, None).unwrap();

    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for serial in [2, 3] {
        let db = f.db.clone();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            let operation = create(AutomationKind::Action, "concurrent", "Concurrent");
            let intent = intent(&operation, "0", serial);
            barrier.wait();
            db.owner_definition_begin("host", &intent, &operation)
        }));
    }
    barrier.wait();
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(DatabaseError::OwnerAdoptionPending)))
            .count(),
        1
    );
    assert!(f.state().pending.is_some());
}

#[test]
fn history_is_bounded_by_insertion_order_even_when_wall_timestamps_move_backwards() {
    let f = Fixture::new();
    for serial in 1..=22 {
        let record = f.finish(
            &create(AutomationKind::Action, &format!("a-{serial}"), "Saved"),
            "0",
            serial,
        );
        if serial == 1 {
            f.connection()
                .execute(
                    "UPDATE owner_automation_definitions SET updated_at=?2 WHERE command_id=?1",
                    params![record.intent.command_id, i64::MAX],
                )
                .unwrap();
        }
    }
    let state = f.state();
    assert_eq!(state.recent.len(), HISTORY);
    assert_eq!(state.recent[0].intent.id, "a-22");
    assert_eq!(state.recent[15].intent.id, "a-7");
    assert!(state.recent.iter().all(|record| record.intent.id != "a-1"));
}

#[test]
fn corrupt_receipts_are_bounded_and_cannot_be_archived_as_known_success() {
    for (column,replacement) in [
        ("method",Value::String("automation.update".into())),
        ("resource_scope",Value::String("event".into())),
        ("session_id",Value::String("other-owner".into())),
        ("host_instance_id",Value::String("another-instance".into())),
        ("payload_hash",Value::String("g".repeat(64))),
        ("result_json",Value::String(json!({"id":"other","name":"Other"}).to_string())),
        ("result_json",Value::String(" ".repeat(RECORD_BYTES+1))),
        ("versions_json",Value::String(json!([{"kind":"automation","scope":"action","id":"a","revision":"01"}]).to_string())),
        ("versions_json",Value::String(json!([{"kind":"automation","scope":"action","id":"a","revision":"1"},{"kind":"automation","scope":"event","id":"extra","revision":"1"}]).to_string())),
    ] {
        let f=Fixture::new();
        let operation=create(AutomationKind::Action,"a","Original");
        let intent=intent(&operation,"0",1);
        f.db.owner_definition_begin("host",&intent,&operation).unwrap();
        f.execute(&intent,&operation);
        f.connection().execute(&format!("UPDATE control_commands SET {column}=?2 WHERE principal_id='local-owner' AND command_id=?1"),params![intent.command_id,replacement.as_str().unwrap()]).unwrap();
        assert!(f.db.owner_definition_resolve("host",false,None).is_err(),"{column}");
        assert!(f.db.owner_definition_resolve("host",true,None).is_err(),"{column}");
        assert_eq!(f.state().pending,Some(intent));
        assert_eq!(f.body(AutomationKind::Action,"a").resource_versions[0].revision,"1");
    }
}

#[test]
fn oversized_or_inconsistent_native_metadata_fails_closed_without_source_reads() {
    for mutation in ["oversized", "wrong-digest", "wrong-intent", "unknown-field"] {
        let f = Fixture::new();
        let operation = create(AutomationKind::Action, "a", "Original");
        let intent = intent(&operation, "0", 1);
        f.db.owner_definition_begin("host", &intent, &operation)
            .unwrap();
        let db = f.connection();
        let encoded: String = db
            .query_row(
                "SELECT payload_json FROM owner_automation_definitions",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let changed = if mutation == "oversized" {
            " ".repeat(ENTRY_BYTES + 1)
        } else {
            let mut value: Value = serde_json::from_str(&encoded).unwrap();
            match mutation {
                "wrong-digest" => value["intent"]["recordDigest"] = json!("invalid"),
                "wrong-intent" => {
                    value["intent"]["commandId"] = json!(format!("owner-definition-{:032x}", 2))
                }
                _ => value["source"] = json!("never accepted"),
            }
            value.to_string()
        };
        db.execute(
            "UPDATE owner_automation_definitions SET payload_json=?1",
            [changed],
        )
        .unwrap();
        assert!(f.db.owner_definition_state("host", "instance").is_err());
        assert!(f.db.owner_definition_resolve("host", true, None).is_err());
        assert!(f.body(AutomationKind::Action, "a").value.is_none());
    }
}

#[test]
fn delete_dispatch_budget_counts_exact_serialized_versions_before_any_effect() {
    use crate::collaboration::ResourceVersion;
    let f = Fixture::new();
    f.finish(&create(AutomationKind::Action, "a", "Action"), "0", 1);
    for (serial, id) in [(2, "event-a"), (3, "event-z")] {
        f.finish(&AutomationMutation::Create{kind:AutomationKind::Event,record:json!({"id":id,"name":id,"enabled":false,"actionIds":[42,"a","a",{"id":"a"}]})},"0",serial);
    }
    let expected = vec![
        ResourceVersion {
            kind: ResourceKind::Automation,
            scope: "action".into(),
            id: "a".into(),
            revision: "2".into(),
        },
        ResourceVersion {
            kind: ResourceKind::Automation,
            scope: "event".into(),
            id: "event-a".into(),
            revision: "2".into(),
        },
        ResourceVersion {
            kind: ResourceKind::Automation,
            scope: "event".into(),
            id: "event-z".into(),
            revision: "2".into(),
        },
    ];
    let exact = serde_json::to_vec(&expected).unwrap().len();
    let operation = AutomationMutation::Delete {
        kind: AutomationKind::Action,
        id: "a".into(),
    };
    let db = f.connection();
    assert!(matches!(
        super::super::behavior::owner_definition_dispatch_preflight(
            &db,
            &operation,
            "1",
            exact - 1
        ),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    assert_eq!(
        f.body(AutomationKind::Action, "a").resource_versions[0].revision,
        "1"
    );
    assert_eq!(
        f.body(AutomationKind::Event, "event-a").resource_versions[0].revision,
        "1"
    );
    assert!(super::super::behavior::owner_definition_dispatch_preflight(
        &db, &operation, "1", exact
    )
    .is_ok());
    let record = f.finish(&operation, "1", 4);
    let actual:String=db.query_row("SELECT versions_json FROM control_commands WHERE principal_id='local-owner' AND command_id=?1",[&record.intent.command_id],|row|row.get(0)).unwrap();
    assert_eq!(actual.len(), exact);
    let actual: Vec<ResourceVersion> = serde_json::from_str(&actual).unwrap();
    assert_eq!(actual, expected);
    // Non-string values remain, and all repeated exact string references leave.
    assert_eq!(
        f.body(AutomationKind::Event, "event-a").value.unwrap()["actionIds"],
        json!([42,{"id":"a"}])
    );
}

#[test]
fn toggle_dispatch_rejects_oversized_legacy_output_before_mutation_and_accepts_exact_output_limit()
{
    for exact in [false, true] {
        let f = Fixture::new();
        let mut body = json!({"id":"a","name":"Legacy","enabled":false,"config":{"message":""}});
        let overhead = body.to_string().len();
        // Replacing false with true saves exactly one byte. The accepted case
        // deliberately has a preexisting source one byte above the output cap.
        let length = RECORD_BYTES - overhead + if exact { 1 } else { 2 };
        body["config"]["message"] = json!("x".repeat(length));
        f.db.save_behavior_versioned(
            "behavior_actions",
            &body,
            Some("0"),
            crate::db::BehaviorSave::Create,
        )
        .unwrap();
        let operation = AutomationMutation::SetEnabled {
            kind: AutomationKind::Action,
            id: "a".into(),
            enabled: true,
        };
        let plan = f.db.owner_definition_validate_dispatch(&operation, "1");
        if exact {
            assert!(plan.is_ok());
            let record = f.finish(&operation, "1", 1);
            assert_eq!(record.receipt.unwrap().enabled, Some(true));
            assert_eq!(
                f.body(AutomationKind::Action, "a")
                    .value
                    .unwrap()
                    .to_string()
                    .len(),
                RECORD_BYTES
            );
        } else {
            assert!(matches!(plan, Err(DatabaseError::SnapshotTooLarge)));
            let current = f.body(AutomationKind::Action, "a");
            assert_eq!(current.value.unwrap()["enabled"], false);
            assert_eq!(current.resource_versions[0].revision, "1");
            assert_eq!(
                f.connection()
                    .query_row("SELECT count(*) FROM control_commands", [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }
}

#[test]
fn dispatch_preflight_preserves_cas_missing_update_and_nonexistent_delete_semantics() {
    let f = Fixture::new();
    f.finish(&create(AutomationKind::Action, "a", "Action"), "0", 1);
    assert!(matches!(
        f.db.owner_definition_validate_dispatch(
            &AutomationMutation::SetEnabled {
                kind: AutomationKind::Action,
                id: "a".into(),
                enabled: true
            },
            "0"
        ),
        Err(DatabaseError::RevisionConflict(_))
    ));
    assert!(matches!(
        f.db.owner_definition_validate_dispatch(
            &AutomationMutation::SetEnabled {
                kind: AutomationKind::Action,
                id: "missing".into(),
                enabled: true
            },
            "0"
        ),
        Err(DatabaseError::NotFound)
    ));
    f.finish(&AutomationMutation::Create{kind:AutomationKind::Event,record:json!({"id":"dangling","name":"Dangling","enabled":false,"actionIds":["missing"]})},"0",2);
    let record = f.finish(
        &AutomationMutation::Delete {
            kind: AutomationKind::Action,
            id: "missing".into(),
        },
        "0",
        3,
    );
    assert_eq!(record.receipt.as_ref().unwrap().deleted, Some(false));
    assert_eq!(record.receipt.as_ref().unwrap().revision, "0");
    assert_eq!(record.receipt.unwrap().affected_resources, 1);
    let event = f.body(AutomationKind::Event, "dangling");
    assert_eq!(event.resource_versions[0].revision, "1");
    assert_eq!(event.value.unwrap()["actionIds"], json!(["missing"]));
}

#[test]
fn input_and_legacy_toggle_structure_budgets_are_checked_before_digest_or_effect() {
    let f = Fixture::new();
    let mut nested = json!("leaf");
    for _ in 0..65 {
        nested = json!({"child":nested});
    }
    let body = json!({"id":"a","name":"Deep legacy","enabled":false,"config":nested});
    let create = AutomationMutation::Create {
        kind: AutomationKind::Action,
        record: body.clone(),
    };
    assert!(validate_owner_definition_input(&create, "0").is_err());
    f.db.save_behavior_versioned(
        "behavior_actions",
        &body,
        Some("0"),
        crate::db::BehaviorSave::Create,
    )
    .unwrap();
    assert!(matches!(
        f.db.owner_definition_validate_dispatch(
            &AutomationMutation::SetEnabled {
                kind: AutomationKind::Action,
                id: "a".into(),
                enabled: true
            },
            "1"
        ),
        Err(DatabaseError::SnapshotTooLarge)
    ));
    assert_eq!(
        f.body(AutomationKind::Action, "a").resource_versions[0].revision,
        "1"
    );
    assert_eq!(
        f.body(AutomationKind::Action, "a").value.unwrap()["enabled"],
        false
    );
    assert!(validate_owner_definition_input(
        &AutomationMutation::Adopt {
            kind: AutomationKind::Action,
            id: "a".into(),
            reviewed_versions: None
        },
        "1"
    )
    .is_err());
}

#[test]
fn cascading_delete_cannot_promote_an_existing_event_when_its_retained_sidecars_are_missing() {
    let f = Fixture::new();
    f.finish(&create(AutomationKind::Action, "a", "Action"), "0", 1);
    for (serial, id) in [(2, "event-a"), (3, "event-z")] {
        f.finish(
            &AutomationMutation::Create {
                kind: AutomationKind::Event,
                record: json!({"id":id,"name":id,"enabled":false,"actionIds":["a"]}),
            },
            "0",
            serial,
        );
    }
    // Simulate loss after the one-time migration. A current event must never
    // acquire fresh Owner provenance just because deletion rewrites its refs.
    let db = f.connection();
    db.execute("DELETE FROM control_resource_versions WHERE kind='automation' AND scope='event' AND resource_id='event-z'",[]).unwrap();
    db.execute(
        "DELETE FROM behavior_provenance WHERE scope='event' AND resource_id='event-z'",
        [],
    )
    .unwrap();
    let operation = AutomationMutation::Delete {
        kind: AutomationKind::Action,
        id: "a".into(),
    };
    let intent = intent(&operation, "1", 4);
    f.db.owner_definition_begin("host", &intent, &operation)
        .unwrap();
    assert!(matches!(
        f.db.owner_definition_validate_dispatch(&operation, "1"),
        Err(DatabaseError::Invalid(_))
    ));
    // The generic business path uses the same retained-history check. It may
    // have visited the valid earlier event, but the whole transaction rolls
    // back before any deletion, revision or authority change becomes durable.
    let result = f.db.execute_behavior_command(
        &operation,
        &CommandTransaction {
            principal_id: "local-owner".into(),
            session_id: "local-owner".into(),
            authorization: None,
            request: CommandRequest {
                command_id: intent.command_id.clone(),
                host_instance_id: "instance".into(),
                expected_revision: "1".into(),
            },
            current_instance_id: "instance".into(),
            max_ids_per_principal: 10_000,
        },
    );
    assert!(matches!(result, Err(DatabaseError::Invalid(_))));
    assert_eq!(
        f.body(AutomationKind::Action, "a").resource_versions[0].revision,
        "1"
    );
    for (id, revision) in [("event-a", "1"), ("event-z", "0")] {
        let current = f.body(AutomationKind::Event, id);
        assert_eq!(current.resource_versions[0].revision, revision);
        assert_eq!(current.value.unwrap()["actionIds"], json!(["a"]));
    }
    assert_eq!(db.query_row("SELECT count(*) FROM behavior_provenance WHERE scope='event' AND resource_id='event-z'",[],|row|row.get::<_,i64>(0)).unwrap(),0);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM control_commands WHERE command_id=?1",
            [&intent.command_id],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(f.state().pending, Some(intent));
}
