use super::*;

#[test]
fn v2_upload_source_reserves_database_and_rejects_stale_or_undrained_edits() {
    let f = Fixture::new();
    let generation =
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, None)
            .unwrap() as u64;
    let mut competing = f.db.open(&f.db.automation_path()).unwrap();
    competing.busy_timeout(std::time::Duration::ZERO).unwrap();
    let admitted = f
        .db
        .with_current_v2_upload_source::<_, DatabaseError>("hotkeys", generation, false, || {
            assert!(competing
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .is_err());
            Ok(7)
        })
        .unwrap();
    assert_eq!(admitted, 7);
    assert!(f
        .db
        .with_current_v2_upload_source::<_, DatabaseError>("hotkeys", generation, true, || Ok(()))
        .is_err());
    f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
        .unwrap();
    assert!(f
        .db
        .with_current_v2_upload_source::<_, DatabaseError>("hotkeys", generation, false, || Ok(()))
        .is_err());
    assert!(
        f.db.with_current_v2_upload_source::<_, DatabaseError>(
            "hotkeys",
            generation + 1,
            false,
            || Ok(())
        )
        .is_err(),
        "undrained decision must not authorize upload"
    );
}

#[test]
fn package_journal_blocks_candidate_and_receipt_until_reconciled() {
    use tiktools_cloud_sync::{PluginSelection, V1Edit, V1Pending};
    let f = Fixture::new();
    let origin = "https://cloud.example";
    let epoch = "a".repeat(64);
    f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
        .unwrap();
    let entry =
        f.db.next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .unwrap();
    assert!(f.db.confirm_plugin_sync_runtime(&entry, &epoch).unwrap());
    let confirmed =
        f.db.next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .unwrap();
    let pending = V1Pending {
        base_revision: Some(1),
        conflict: false,
        edit: V1Edit::Put(PluginSelection {
            schema_version: 1,
            plugin_id: "hotkeys".into(),
            version: "1.0.0".into(),
            enabled: false,
        }),
    };
    let insert_guard = || {
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute("INSERT INTO plugin_package_mutation_intents(plugin_id,intent_id,operation,operation_digest,phase,source_generation) VALUES ('hotkeys',?1,'update',?2,'mutating',1)",
            params!["b".repeat(64), "c".repeat(64)]).unwrap();
    };
    let remove_guard = || {
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute(
            "DELETE FROM plugin_package_mutation_intents WHERE plugin_id='hotkeys'",
            [],
        )
        .unwrap();
    };
    insert_guard();
    assert!(f.db.next_plugin_sync_intent(origin, "alice").is_err());
    assert!(!f.db.confirm_plugin_sync_runtime(&entry, &epoch).unwrap());
    assert!(!f
        .db
        .acknowledge_plugin_sync_intent(&confirmed, &epoch)
        .unwrap());
    assert!(!f
        .db
        .receipt_plugin_sync_intent(&confirmed, &epoch, &pending)
        .unwrap());
    assert!(f
        .db
        .set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
        .is_err());
    remove_guard();
    assert!(f
        .db
        .receipt_plugin_sync_intent(&confirmed, &epoch, &pending)
        .unwrap());
    insert_guard();
    assert!(!f
        .db
        .validate_plugin_sync_receipt(origin, "alice", "hotkeys", 7, &epoch, &pending)
        .unwrap());
    let mut conflict = pending.clone();
    conflict.conflict = true;
    assert!(f
        .db
        .plugin_sync_conflict_source(origin, "alice", "hotkeys", 7, &epoch, &conflict)
        .unwrap()
        .is_none());
    assert!(!f
        .db
        .confirm_plugin_sync_receipt_runtime(origin, "alice", "hotkeys", 7, &epoch, &pending)
        .unwrap());
    remove_guard();
    assert!(f
        .db
        .validate_plugin_sync_receipt(origin, "alice", "hotkeys", 7, &epoch, &pending)
        .unwrap());
}
