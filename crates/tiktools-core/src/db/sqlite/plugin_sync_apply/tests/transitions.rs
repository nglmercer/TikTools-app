use super::*;

#[test]
fn uncertain_absence_is_durable_and_requires_reconciliation_and_runtime_observation() {
    let f = Fixture::new();
    let intent = f.intent();
    let epoch = "b".repeat(64);
    assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    let mut changed = intent.clone();
    changed.expected_enabled = false;
    assert!(f.db.prepare_plugin_apply(&f.context, &changed).is_err());
    let mut other = intent.clone();
    other.intent_id = "c".repeat(64);
    assert!(!f.db.prepare_plugin_apply(&f.context, &other).unwrap());
    assert_eq!(
        f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
            .unwrap(),
        None
    );
    assert_eq!(f.state(), (1, 1, 1));
    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert_eq!(
        reopened
            .plugin_apply_intent(&intent.intent_id)
            .unwrap()
            .unwrap()
            .phase,
        PluginApplyPhase::Prepared
    );
    assert!(reopened
        .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
        .unwrap());
    let generation = reopened
        .commit_plugin_apply_state(&f.context, &intent, "1.0.0")
        .unwrap()
        .unwrap();
    assert_eq!(generation, 2);
    assert_eq!(f.state(), (2, 1, 0));
    assert_eq!(
        reopened
            .plugin_apply_intent(&intent.intent_id)
            .unwrap()
            .unwrap()
            .phase,
        PluginApplyPhase::RuntimePending
    );
    assert!(!reopened
        .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "1.0.0", true)
        .unwrap());
    assert!(!reopened
        .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "2.0.0", false)
        .unwrap());
    let mut other_context = f.context.clone();
    other_context.account_id = "bob".into();
    assert!(!reopened
        .confirm_plugin_apply_runtime(&other_context, &intent, generation, &epoch, "1.0.0", false)
        .unwrap());
    assert!(reopened
        .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "1.0.0", false)
        .unwrap());
    assert_eq!(
        reopened
            .plugin_apply_intent(&intent.intent_id)
            .unwrap()
            .unwrap()
            .phase,
        PluginApplyPhase::Applied
    );
    other.source_generation = generation;
    other.expected_enabled = false;
    assert!(!reopened.prepare_plugin_apply(&f.context, &other).unwrap());
    assert!(!reopened
        .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &"d".repeat(64))
        .unwrap());
    // A restarted host observes the runtime again under its new epoch;
    // retaining the old epoch alone must not authorize finalization.
    let restarted_epoch = "d".repeat(64);
    assert!(reopened
        .confirm_plugin_apply_runtime(
            &f.context,
            &intent,
            generation,
            &restarted_epoch,
            "1.0.0",
            false
        )
        .unwrap());
    assert!(!reopened
        .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &epoch)
        .unwrap());
    assert!(reopened
        .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &restarted_epoch)
        .unwrap());
    assert_eq!(
        reopened
            .plugin_apply_intent(&intent.intent_id)
            .unwrap()
            .unwrap()
            .phase,
        PluginApplyPhase::Finalized
    );
    assert!(reopened.prepare_plugin_apply(&f.context, &other).unwrap());
}

#[test]
fn failed_state_transition_rolls_back_and_cloud_never_installs_or_changes_version() {
    let f = Fixture::new();
    let mut intent = f.intent();
    intent.remote = Some(V1Record {
        collection: "plugin-selections".into(),
        key: "acme.plugin".into(),
        revision: 4,
        deleted: false,
        value: Some(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.plugin".into(),
            version: "2.0.0".into(),
            enabled: false,
        }),
    });
    assert!(!f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    assert!(!f
        .db
        .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
        .unwrap());
    assert_eq!(
        f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
            .unwrap(),
        None
    );
    assert_eq!(f.state(), (1, 1, 1));

    let g = Fixture::new();
    let intent = g.intent();
    assert!(g.db.prepare_plugin_apply(&g.context, &intent).unwrap());
    assert!(g
        .db
        .mark_plugin_apply_ready_after_native_reconciliation(&g.context, &intent)
        .unwrap());
    let db = g.db.open(&g.db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_apply BEFORE UPDATE ON plugin_sync_apply_intents WHEN NEW.phase='runtime_pending' BEGIN SELECT RAISE(FAIL,'apply failure'); END;").unwrap();
    drop(db);
    assert!(g
        .db
        .commit_plugin_apply_state(&g.context, &intent, "1.0.0")
        .is_err());
    assert_eq!(g.state(), (1, 1, 1));
    assert_eq!(
        g.db.plugin_apply_intent(&intent.intent_id)
            .unwrap()
            .unwrap()
            .phase,
        PluginApplyPhase::Ready
    );
}

#[test]
fn tombstone_disables_an_existing_plugin_without_uninstalling_it() {
    let f = Fixture::new();
    let mut intent = f.intent();
    intent.remote = Some(V1Record {
        collection: "plugin-selections".into(),
        key: "acme.plugin".into(),
        revision: 3,
        deleted: true,
        value: None,
    });
    assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    assert!(f
        .db
        .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
        .unwrap());
    assert_eq!(
        f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
            .unwrap(),
        Some(2)
    );
    assert_eq!(f.state(), (2, 1, 0));
}

#[test]
fn ignored_sqlite_update_rolls_back_the_entire_apply_transition() {
    for table in [
        "plugin_sync_source_versions",
        "behavior_plugins",
        "plugin_sync_apply_intents",
    ] {
        let f = Fixture::new();
        let intent = f.intent();
        assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        assert!(f
            .db
            .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
            .unwrap());
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch(&format!("CREATE TRIGGER ignore_transition BEFORE UPDATE ON {table} BEGIN SELECT RAISE(IGNORE); END;")).unwrap();
        drop(db);
        assert!(f
            .db
            .commit_plugin_apply_state(&f.context, &intent, "1.0.0")
            .is_err());
        assert_eq!(f.state(), (1, 1, 1));
        assert_eq!(
            f.db.plugin_apply_intent(&intent.intent_id)
                .unwrap()
                .unwrap()
                .phase,
            PluginApplyPhase::Ready
        );
    }
}
