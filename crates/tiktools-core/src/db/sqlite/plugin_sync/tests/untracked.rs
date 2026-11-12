use super::*;

#[test]
fn reviewed_untracked_transfer_is_atomic_and_rejects_stale_source() {
    let f = Fixture::new();
    f.db.set_plugin_state_with_untracked_sync("hotkeys", true, false)
        .unwrap();
    let old = f.db.review_untracked_plugin("hotkeys").unwrap().unwrap();
    f.db.set_plugin_state_with_untracked_sync("hotkeys", true, true)
        .unwrap();
    assert!(f
        .db
        .reconcile_untracked_plugin(&old, &f.intent("alice"), None)
        .is_err());
    assert!(f.db.has_untracked_plugin_sync().unwrap());
    assert!(f
        .db
        .next_plugin_sync_intent("https://cloud.example", "alice")
        .unwrap()
        .is_none());
    let review = f.db.review_untracked_plugin("hotkeys").unwrap().unwrap();
    assert_ne!(review.generation, old.generation);
    assert!(review.enabled);
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER reject_marker_delete BEFORE DELETE ON plugin_sync_untracked_local BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(f
        .db
        .reconcile_untracked_plugin(&review, &f.intent("alice"), None)
        .is_err());
    assert!(f.db.has_untracked_plugin_sync().unwrap());
    assert!(f
        .db
        .next_plugin_sync_intent("https://cloud.example", "alice")
        .unwrap()
        .is_none());
    db.execute_batch("DROP TRIGGER reject_marker_delete")
        .unwrap();
    f.db.reconcile_untracked_plugin(&review, &f.intent("alice"), None)
        .unwrap();
    assert!(!f.db.has_untracked_plugin_sync().unwrap());
    let outbox =
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
    assert_eq!(outbox.generation, review.generation);
    assert_eq!(outbox.source_generation, review.generation);
    assert_eq!(outbox.enabled, Some(true));
    assert_eq!(outbox.runtime_epoch, None);
    assert!(f
        .db
        .reconcile_untracked_plugin(&review, &f.intent("alice"), None)
        .is_err());
}

#[test]
fn reviewed_untracked_transfer_preserves_other_accounts_and_tombstones() {
    let f = Fixture::new();
    f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("bob")))
        .unwrap();
    f.db.set_plugin_state_with_untracked_sync("hotkeys", true, true)
        .unwrap();
    let review = f.db.review_untracked_plugin("hotkeys").unwrap().unwrap();
    assert!(f
        .db
        .reconcile_untracked_plugin(&review, &f.intent("alice"), None)
        .is_err());
    assert!(f.db.has_untracked_plugin_sync().unwrap());
    assert!(f
        .db
        .next_plugin_sync_intent("https://cloud.example", "bob")
        .unwrap()
        .is_some());
    f.db.set_plugin_state_with_untracked_sync("removed", false, false)
        .unwrap();
    let removed = f.db.review_untracked_plugin("removed").unwrap().unwrap();
    assert!(!removed.installed);
    assert!(f
        .db
        .reconcile_untracked_plugin(&removed, &f.intent("alice"), None)
        .is_err());
    assert!(f.db.review_untracked_plugin("removed").unwrap().is_some());
    f.db.set_plugin_state_with_untracked_sync("legacy", true, false)
        .unwrap();
    let legacy = f.db.review_untracked_plugin("legacy").unwrap().unwrap();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute("INSERT INTO plugin_sync_receipts(origin,account_id,plugin_id,generation,config_generation,runtime_epoch,pending_json) VALUES (?1,?2,?3,?4,?5,?6,?7)", params!["https://cloud.example", "bob", "legacy", legacy.generation, "7", "a".repeat(64), "{}"] ).unwrap();
    assert!(f
        .db
        .reconcile_untracked_plugin(&legacy, &f.intent("alice"), None)
        .is_err());
    assert_eq!(
        f.db.review_untracked_plugin("legacy").unwrap(),
        Some(legacy)
    );
}
