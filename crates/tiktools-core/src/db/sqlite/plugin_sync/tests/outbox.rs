use super::*;

#[test]
fn legacy_base_review_preserves_candidate_until_exact_atomic_commit() {
    let f = Fixture::new();
    f.db.set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
        .unwrap();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute("UPDATE plugin_sync_outbox SET reviewed_base=NULL,runtime_epoch=?1 WHERE plugin_id='hotkeys'", ["a".repeat(64)]).unwrap();
    let old =
        f.db.plugin_sync_intent_for("https://cloud.example", "alice", "hotkeys")
            .unwrap()
            .unwrap();
    let mut stale = old.clone();
    stale.generation += 1;
    assert!(!f
        .db
        .review_legacy_plugin_sync_base(&stale, 8, "null")
        .unwrap());
    assert!(f
        .db
        .review_legacy_plugin_sync_base(&old, 8, "not-json")
        .is_err());
    let wrong_target = serde_json::json!({"collection":"plugin-selections","key":"other","revision":1,"deleted":true,"value":null}).to_string();
    assert!(f
        .db
        .review_legacy_plugin_sync_base(&old, 8, &wrong_target)
        .is_err());
    db.execute("INSERT INTO plugin_sync_outbox(origin,account_id,plugin_id,generation,config_generation,version,enabled) VALUES ('https://cloud.example','bob','hotkeys',?1,'7','1.0.0',1)", [old.generation]).unwrap();
    assert!(!f
        .db
        .review_legacy_plugin_sync_base(&old, 8, "null")
        .unwrap());
    db.execute("DELETE FROM plugin_sync_outbox WHERE account_id='bob'", [])
        .unwrap();
    db.execute_batch("CREATE TRIGGER reject_legacy_review BEFORE UPDATE ON plugin_sync_outbox BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(f
        .db
        .review_legacy_plugin_sync_base(&old, 8, "null")
        .is_err());
    assert_eq!(
        f.db.plugin_sync_intent_for("https://cloud.example", "alice", "hotkeys")
            .unwrap()
            .unwrap(),
        old
    );
    db.execute_batch("DROP TRIGGER reject_legacy_review")
        .unwrap();
    db.execute(
        "INSERT INTO plugin_sync_untracked_local(plugin_id,generation) VALUES ('hotkeys',?1)",
        [old.generation],
    )
    .unwrap();
    assert!(!f
        .db
        .review_legacy_plugin_sync_base(&old, 8, "null")
        .unwrap());
    db.execute(
        "DELETE FROM plugin_sync_untracked_local WHERE plugin_id='hotkeys'",
        [],
    )
    .unwrap();
    let base = serde_json::json!({"collection":"plugin-selections","key":"hotkeys","revision":4,"deleted":false,"value":{"schemaVersion":1,"pluginId":"hotkeys","version":"1.0.0","enabled":false}}).to_string();
    assert!(f.db.review_legacy_plugin_sync_base(&old, 8, &base).unwrap());
    let current =
        f.db.plugin_sync_intent_for("https://cloud.example", "alice", "hotkeys")
            .unwrap()
            .unwrap();
    assert_eq!(current.reviewed_base.as_deref(), Some(base.as_str()));
    assert_eq!(current.config_generation, 8);
    assert_eq!(current.runtime_epoch, None);
    assert!(!f
        .db
        .review_legacy_plugin_sync_base(&old, 8, "null")
        .unwrap());
}

#[test]
fn durable_identity_partition_and_generation_cas_preserve_newer_edits() {
    let f = Fixture::new();
    let a = "https://cloud.example";
    assert_eq!(
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
            .unwrap(),
        1
    );
    let old = f.db.next_plugin_sync_intent(a, "alice").unwrap().unwrap();
    assert!(f.db.next_plugin_sync_intent(a, "bob").unwrap().is_none());
    assert_eq!(
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
            .unwrap(),
        2
    );
    assert!(!f
        .db
        .acknowledge_plugin_sync_intent(&old, &"a".repeat(64))
        .unwrap());
    let newer = f.db.next_plugin_sync_intent(a, "alice").unwrap().unwrap();
    assert_eq!(newer.enabled, Some(false));
    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert_eq!(
        reopened.next_plugin_sync_intent(a, "alice").unwrap(),
        Some(newer.clone())
    );
    // A local-only ABA changes the source and prevents old intent ack.
    reopened
        .set_plugin_state_with_sync_intent("hotkeys", true, true, None)
        .unwrap();
    assert!(!reopened
        .acknowledge_plugin_sync_intent(&newer, &"a".repeat(64))
        .unwrap());
    let stale = reopened
        .next_plugin_sync_intent(a, "alice")
        .unwrap()
        .unwrap();
    assert_eq!((stale.generation, stale.source_generation), (2, 3));
}

#[test]
fn tombstone_remains_partitioned_and_corrupt_source_fails_closed() {
    let f = Fixture::new();
    let origin = "https://cloud.example";
    f.db.set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
        .unwrap();
    let deletion = LocalPluginSyncIntent {
        origin,
        account_id: "bob",
        config_generation: u128::MAX,
        version: None,
    };
    f.db.set_plugin_state_with_sync_intent("hotkeys", false, false, Some(deletion))
        .unwrap();
    let a =
        f.db.next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .unwrap();
    let b =
        f.db.next_plugin_sync_intent(origin, "bob")
            .unwrap()
            .unwrap();
    assert_eq!((a.generation, a.source_generation), (1, 2));
    assert_eq!(
        (
            b.generation,
            b.source_generation,
            b.config_generation,
            b.version.as_deref(),
            b.enabled
        ),
        (2, 2, u128::MAX, None, None)
    );
    assert!(!f
        .db
        .acknowledge_plugin_sync_intent(&a, &"a".repeat(64))
        .unwrap());
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute(
        "DELETE FROM plugin_sync_source_versions WHERE plugin_id='hotkeys'",
        [],
    )
    .unwrap();
    assert!(f.db.next_plugin_sync_intent(origin, "bob").is_err());
}

#[test]
fn runtime_confirmation_is_required_for_each_host_lifetime_and_source_generation() {
    let f = Fixture::new();
    let origin = "https://cloud.example";
    let first_epoch = "a".repeat(64);
    let next_epoch = "b".repeat(64);
    f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
        .unwrap();
    let original =
        f.db.next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .unwrap();
    assert_eq!(original.runtime_epoch, None);
    // If applying the runtime change fails, the caller must not confirm it.
    assert!(!f
        .db
        .acknowledge_plugin_sync_intent(&original, &first_epoch)
        .unwrap());
    assert!(f
        .db
        .confirm_plugin_sync_runtime(&original, &first_epoch)
        .unwrap());
    let confirmed =
        f.db.next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .unwrap();
    assert_eq!(
        confirmed.runtime_epoch.as_deref(),
        Some(first_epoch.as_str())
    );

    // A restarted host has a new epoch and cannot remove work merely
    // because the former host once observed the desired runtime state.
    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert!(!reopened
        .acknowledge_plugin_sync_intent(&confirmed, &next_epoch)
        .unwrap());
    assert!(reopened
        .confirm_plugin_sync_runtime(&confirmed, &next_epoch)
        .unwrap());
    assert!(!f
        .db
        .acknowledge_plugin_sync_intent(&confirmed, &first_epoch)
        .unwrap());

    // A newer edit invalidates both confirmations, including an ABA that
    // returns to the same enabled value.
    f.db.set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
        .unwrap();
    f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
        .unwrap();
    assert!(!reopened
        .confirm_plugin_sync_runtime(&confirmed, &next_epoch)
        .unwrap());
    assert!(!reopened
        .acknowledge_plugin_sync_intent(&confirmed, &next_epoch)
        .unwrap());
    let current = reopened
        .next_plugin_sync_intent(origin, "alice")
        .unwrap()
        .unwrap();
    assert_eq!(current.runtime_epoch, None);
    assert!(reopened
        .confirm_plugin_sync_runtime(&current, &next_epoch)
        .unwrap());
    assert!(reopened
        .acknowledge_plugin_sync_intent(&current, &next_epoch)
        .unwrap());
    assert!(reopened
        .next_plugin_sync_intent(origin, "alice")
        .unwrap()
        .is_none());
}

#[test]
fn acknowledgement_cannot_erase_a_changed_payload_with_the_same_generation() {
    let f = Fixture::new();
    let epoch = "a".repeat(64);
    f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
        .unwrap();
    let entry =
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
    assert!(f.db.confirm_plugin_sync_runtime(&entry, &epoch).unwrap());

    // Even if retained storage is unexpectedly changed without advancing
    // its generation, an acknowledgement for the original payload must
    // preserve the new value for inspection instead of erasing it.
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute(
        "UPDATE plugin_sync_outbox SET enabled=1 WHERE plugin_id='hotkeys'",
        [],
    )
    .unwrap();
    drop(db);
    assert!(!f.db.acknowledge_plugin_sync_intent(&entry, &epoch).unwrap());
    assert_eq!(
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap()
            .enabled,
        Some(true)
    );
}

#[test]
fn legacy_consent_bound_outbox_needs_runtime_reconciliation_after_migration() {
    let f = Fixture::new();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch(
        "DROP TABLE plugin_sync_outbox;
         CREATE TABLE plugin_sync_outbox (
           origin TEXT NOT NULL, account_id TEXT NOT NULL, plugin_id TEXT NOT NULL,
           generation INTEGER NOT NULL, config_generation TEXT NOT NULL,
           version TEXT, enabled INTEGER, PRIMARY KEY(origin,account_id,plugin_id)
         );
         INSERT INTO plugin_sync_source_versions(plugin_id,generation) VALUES ('hotkeys',1);
         INSERT INTO behavior_plugins(id,installed,enabled,updated_at) VALUES ('hotkeys',1,1,0);
         INSERT INTO plugin_sync_outbox VALUES ('https://cloud.example','alice','hotkeys',1,'7','1.0.0',1);"
    ).unwrap();
    drop(db);
    for _ in 0..2 {
        let reopened = DatabaseManager::new(f.db.paths().clone());
        let entry = reopened
            .next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
        assert_eq!(entry.runtime_epoch, None);
        assert!(!reopened
            .acknowledge_plugin_sync_intent(&entry, &"a".repeat(64))
            .unwrap());
    }
}
