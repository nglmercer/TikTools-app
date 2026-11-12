use super::*;

#[test]
fn package_journal_fences_every_apply_phase_and_missing_guard_fails_closed() {
    let f = Fixture::new();
    let intent = f.intent();
    let change_guard = |active: bool| {
        let db = f.db.open(&f.db.automation_path()).unwrap();
        if active {
            db.execute("INSERT INTO plugin_package_mutation_intents(plugin_id,intent_id,operation,operation_digest,phase,source_generation) VALUES ('acme.plugin','test-package','install','digest','prepared',1)", []).unwrap();
        } else {
            db.execute(
                "DELETE FROM plugin_package_mutation_intents WHERE plugin_id='acme.plugin'",
                [],
            )
            .unwrap();
        }
    };
    change_guard(true);
    assert!(f
        .db
        .pulled_plugin_apply_source("acme.plugin")
        .unwrap()
        .is_none());
    assert!(!f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    change_guard(false);
    assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    change_guard(true);
    assert!(!f
        .db
        .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
        .unwrap());
    assert!(!f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    change_guard(false);
    assert!(f
        .db
        .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
        .unwrap());
    change_guard(true);
    assert!(!f
        .db
        .revalidate_ready_plugin_apply(&f.context, &intent)
        .unwrap());
    assert!(f
        .db
        .commit_plugin_apply_state(&f.context, &intent, "1.0.0")
        .unwrap()
        .is_none());
    change_guard(false);
    assert!(f
        .db
        .revalidate_ready_plugin_apply(&f.context, &intent)
        .unwrap());
    let generation =
        f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
            .unwrap()
            .unwrap();
    let epoch = "b".repeat(64);
    change_guard(true);
    assert!(!f
        .db
        .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "1.0.0", false)
        .unwrap());
    change_guard(false);
    assert!(f
        .db
        .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "1.0.0", false)
        .unwrap());
    change_guard(true);
    assert!(!f
        .db
        .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &epoch)
        .unwrap());
    change_guard(false);
    assert!(f
        .db
        .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &epoch)
        .unwrap());

    // A malformed legacy row with no plugin identity blocks globally;
    // a missing table produces an error rather than granting admission.
    let other = Fixture::new();
    let db = other.db.open(&other.db.automation_path()).unwrap();
    db.execute_batch("DROP TABLE plugin_package_mutation_intents; CREATE TABLE plugin_package_mutation_intents(plugin_id TEXT); INSERT INTO plugin_package_mutation_intents VALUES (NULL);").unwrap();
    drop(db);
    assert!(other
        .db
        .pulled_plugin_apply_source("acme.plugin")
        .unwrap()
        .is_none());
    assert!(!other
        .db
        .prepare_plugin_apply(&other.context, &other.intent())
        .unwrap());
    let db = other.db.open(&other.db.automation_path()).unwrap();
    db.execute_batch("DROP TABLE plugin_package_mutation_intents;")
        .unwrap();
    drop(db);
    assert!(other.db.pulled_plugin_apply_source("acme.plugin").is_err());
    assert!(other
        .db
        .prepare_plugin_apply(&other.context, &other.intent())
        .is_err());
}

#[test]
fn local_changes_and_aba_make_prepared_or_applied_evidence_stale() {
    let f = Fixture::new();
    let intent = f.intent();
    let epoch = "b".repeat(64);
    assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    assert!(f
        .db
        .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
        .unwrap());
    f.db.set_plugin_state_with_sync_intent("acme.plugin", true, false, None)
        .unwrap();
    f.db.set_plugin_state_with_sync_intent("acme.plugin", true, true, None)
        .unwrap();
    assert_eq!(f.state(), (3, 1, 1));
    assert_eq!(
        f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
            .unwrap(),
        None
    );
    assert_eq!(
        f.db.plugin_apply_intent(&intent.intent_id)
            .unwrap()
            .unwrap()
            .phase,
        PluginApplyPhase::Ready
    );

    let g = Fixture::new();
    let intent = g.intent();
    assert!(g.db.prepare_plugin_apply(&g.context, &intent).unwrap());
    assert!(g
        .db
        .mark_plugin_apply_ready_after_native_reconciliation(&g.context, &intent)
        .unwrap());
    let generation =
        g.db.commit_plugin_apply_state(&g.context, &intent, "1.0.0")
            .unwrap()
            .unwrap();
    assert!(g
        .db
        .confirm_plugin_apply_runtime(&g.context, &intent, generation, &epoch, "1.0.0", false)
        .unwrap());
    g.db.set_plugin_state_with_sync_intent("acme.plugin", true, true, None)
        .unwrap();
    assert!(!g
        .db
        .finalize_plugin_apply_after_native_ack(&g.context, &intent, generation, &epoch)
        .unwrap());
    assert_eq!(
        g.db.plugin_apply_intent(&intent.intent_id)
            .unwrap()
            .unwrap()
            .phase,
        PluginApplyPhase::Applied
    );
}
