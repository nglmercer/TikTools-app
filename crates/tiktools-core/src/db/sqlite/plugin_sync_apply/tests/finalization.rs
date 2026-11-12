use super::*;

#[test]
fn finalization_witness_is_atomic_and_required_for_startup() {
    let f = Fixture::new();
    let intent = f.intent();
    assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    assert!(f
        .db
        .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
        .unwrap());
    let generation =
        f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
            .unwrap()
            .unwrap();
    let epoch = "b".repeat(64);
    assert!(f
        .db
        .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "1.0.0", false)
        .unwrap());
    assert!(f
        .db
        .plugin_apply_quarantine()
        .unwrap()
        .contains("acme.plugin"));
    let connection = f.db.open(&f.db.automation_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER ignore_apply_witness BEFORE INSERT ON plugin_sync_apply_finalizations BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    drop(connection);
    assert!(f
        .db
        .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &epoch)
        .is_err());
    assert_eq!(
        f.db.plugin_apply_intent(&intent.intent_id)
            .unwrap()
            .unwrap()
            .phase,
        PluginApplyPhase::Applied
    );
    assert!(f
        .db
        .plugin_apply_quarantine()
        .unwrap()
        .contains("acme.plugin"));
    let connection = f.db.open(&f.db.automation_path()).unwrap();
    connection
        .execute_batch("DROP TRIGGER ignore_apply_witness")
        .unwrap();
    drop(connection);
    assert!(f
        .db
        .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &epoch)
        .unwrap());
    assert!(f.db.plugin_apply_quarantine().unwrap().is_empty());
    let connection = f.db.open(&f.db.automation_path()).unwrap();
    connection
        .execute(
            "DELETE FROM plugin_sync_apply_finalizations WHERE intent_id=?1",
            [&intent.intent_id],
        )
        .unwrap();
    drop(connection);
    assert!(f.db.plugin_apply_quarantine().is_err());
}
