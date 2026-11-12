use super::*;

#[test]
fn receipt_move_is_atomic_and_fences_identity_source_consent_and_pending() {
    use tiktools_cloud_sync::{PluginSelection, V1Edit, V1Pending};

    let f = Fixture::new();
    let origin = "https://cloud.example";
    let epoch = "a".repeat(64);
    f.db.set_plugin_state_with_sync_intent("acme.plugin", true, false, Some(f.intent("alice")))
        .unwrap();
    let first =
        f.db.next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .unwrap();
    assert!(f.db.confirm_plugin_sync_runtime(&first, &epoch).unwrap());
    let entry =
        f.db.next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .unwrap();
    let pending = V1Pending {
        base_revision: Some(12),
        edit: V1Edit::Put(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.plugin".into(),
            version: "1.0.0".into(),
            enabled: false,
        }),
        conflict: false,
    };
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON plugin_sync_receipts BEGIN SELECT RAISE(FAIL,'receipt failure'); END;").unwrap();
    drop(db);
    assert!(f
        .db
        .receipt_plugin_sync_intent(&entry, &epoch, &pending)
        .is_err());
    assert_eq!(
        f.db.next_plugin_sync_intent(origin, "alice").unwrap(),
        Some(entry.clone())
    );
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("DROP TRIGGER fail_receipt;").unwrap();
    drop(db);
    assert!(f
        .db
        .receipt_plugin_sync_intent(&entry, &epoch, &pending)
        .unwrap());
    assert!(f
        .db
        .next_plugin_sync_intent(origin, "alice")
        .unwrap()
        .is_none());

    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert!(reopened
        .validate_plugin_sync_receipt(origin, "alice", "acme.plugin", 7, &epoch, &pending)
        .unwrap());
    assert!(!reopened
        .validate_plugin_sync_receipt(origin, "bob", "acme.plugin", 7, &epoch, &pending)
        .unwrap());
    assert!(!reopened
        .validate_plugin_sync_receipt(origin, "alice", "acme.plugin", 8, &epoch, &pending)
        .unwrap());
    assert!(!reopened
        .validate_plugin_sync_receipt(origin, "alice", "acme.plugin", 7, &"b".repeat(64), &pending)
        .unwrap());
    let mut rebased = pending.clone();
    rebased.base_revision = Some(13);
    assert!(!reopened
        .validate_plugin_sync_receipt(origin, "alice", "acme.plugin", 7, &epoch, &rebased)
        .unwrap());

    // A new host lifetime cannot reuse the previous runtime observation.
    // An explicit reconciliation confirms only the identical pending value.
    let restarted_epoch = "b".repeat(64);
    assert!(!reopened
        .confirm_plugin_sync_receipt_runtime(
            origin,
            "alice",
            "acme.plugin",
            7,
            &restarted_epoch,
            &rebased
        )
        .unwrap());
    assert!(reopened
        .confirm_plugin_sync_receipt_runtime(
            origin,
            "alice",
            "acme.plugin",
            7,
            &restarted_epoch,
            &pending
        )
        .unwrap());
    assert!(!reopened
        .validate_plugin_sync_receipt(origin, "alice", "acme.plugin", 7, &epoch, &pending)
        .unwrap());
    assert!(reopened
        .validate_plugin_sync_receipt(
            origin,
            "alice",
            "acme.plugin",
            7,
            &restarted_epoch,
            &pending
        )
        .unwrap());

    // Another account's edit advances the shared source generation, so
    // returning to Alice cannot dispatch the older pending value.
    reopened
        .set_plugin_state_with_sync_intent("acme.plugin", true, true, Some(f.intent("bob")))
        .unwrap();
    assert!(!reopened
        .validate_plugin_sync_receipt(
            origin,
            "alice",
            "acme.plugin",
            7,
            &restarted_epoch,
            &pending
        )
        .unwrap());
}
