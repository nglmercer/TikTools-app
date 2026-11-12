use super::*;

#[test]
fn corrupt_evidence_fails_closed_after_restart() {
    let f = Fixture::new();
    let intent = f.intent();
    assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
    let mut altered = intent.clone();
    altered.plugin_id = "other.plugin".into();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute(
        "UPDATE plugin_sync_apply_intents SET payload_json=?1 WHERE intent_id=?2",
        params![serde_json::to_string(&altered).unwrap(), intent.intent_id],
    )
    .unwrap();
    drop(db);
    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert!(reopened.plugin_apply_intent(&intent.intent_id).is_err());
    assert!(!reopened
        .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
        .unwrap());
    assert_eq!(f.state(), (1, 1, 1));
}

#[test]
fn recovery_discovery_is_bounded_partitioned_and_survives_restart() {
    let f = Fixture::new();
    for (n, account, config, origin) in [
        (1, "alice", 9, "https://cloud.example"),
        (2, "bob", 9, "https://cloud.example"),
        (3, "alice", 8, "https://cloud.example"),
        (4, "alice", 9, "https://other.example"),
        (5, "alice", 9, "https://cloud.example"),
        (6, "alice", 9, "https://cloud.example"),
    ] {
        let key = format!("acme.plugin{n}");
        assert_eq!(
            f.db.set_plugin_state_with_sync_intent(&key, true, true, None)
                .unwrap(),
            1
        );
        let context = V1LocalEditContext {
            origin: origin.into(),
            account_id: account.into(),
            config_generation: config,
        };
        let mut intent = f.intent();
        intent.intent_id = format!("{n:064x}");
        intent.origin = context.origin.clone();
        intent.account_id = context.account_id.clone();
        intent.config_generation = config;
        intent.plugin_id = key.clone();
        if let Some(V1Pending {
            edit: V1Edit::Put(value),
            ..
        }) = &mut intent.reviewed_pending
        {
            value.plugin_id = key;
        } else {
            panic!("fixture pending value");
        }
        assert!(f.db.prepare_plugin_apply(&context, &intent).unwrap());
        if n == 6 {
            assert!(f
                .db
                .mark_plugin_apply_ready_after_native_reconciliation(&context, &intent)
                .unwrap());
            let generation =
                f.db.commit_plugin_apply_state(&context, &intent, "1.0.0")
                    .unwrap()
                    .unwrap();
            let epoch = "b".repeat(64);
            assert!(f
                .db
                .confirm_plugin_apply_runtime(&context, &intent, generation, &epoch, "1.0.0", false)
                .unwrap());
            assert!(f
                .db
                .finalize_plugin_apply_after_native_ack(&context, &intent, generation, &epoch)
                .unwrap());
        }
    }
    let reopened = DatabaseManager::new(f.db.paths().clone());
    let first = reopened
        .plugin_apply_recovery_page(&f.context, None, 2)
        .unwrap();
    assert_eq!(
        first
            .intents
            .iter()
            .map(|v| v.intent.plugin_id.as_str())
            .collect::<Vec<_>>(),
        ["acme.plugin1"]
    );
    assert!(!first.stale_consent);
    assert_eq!(
        first.next_cursor.as_deref(),
        Some(format!("{:064x}", 2).as_str())
    );
    let second = reopened
        .plugin_apply_recovery_page(&f.context, first.next_cursor.as_deref(), 2)
        .unwrap();
    assert!(second.intents.is_empty());
    assert!(second.stale_consent);
    let third = reopened
        .plugin_apply_recovery_page(&f.context, second.next_cursor.as_deref(), 2)
        .unwrap();
    assert_eq!(
        third
            .intents
            .iter()
            .map(|v| v.intent.plugin_id.as_str())
            .collect::<Vec<_>>(),
        ["acme.plugin5"]
    );
    assert!(!third.stale_consent);
    assert!(third.next_cursor.is_none());
    assert!(reopened
        .plugin_apply_recovery_page(&f.context, None, 0)
        .is_err());
    assert!(reopened
        .plugin_apply_recovery_page(&f.context, None, 65)
        .is_err());
    assert!(reopened
        .plugin_apply_recovery_page(&f.context, Some("invalid"), 1)
        .is_err());
}

#[test]
fn recovery_discovery_never_skips_a_corrupt_foreign_identity() {
    let f = Fixture::new();
    let mut intent = f.intent();
    intent.account_id = "bob".into();
    let bob = V1LocalEditContext {
        account_id: "bob".into(),
        ..f.context.clone()
    };
    assert!(f.db.prepare_plugin_apply(&bob, &intent).unwrap());
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute(
        "UPDATE plugin_sync_apply_intents SET payload_json = '{broken' WHERE intent_id = ?1",
        [&intent.intent_id],
    )
    .unwrap();
    drop(db);
    assert!(f
        .db
        .plugin_apply_recovery_page(&f.context, None, 10)
        .is_err());

    // An invalid empty primary key sorts before valid generated ids. The
    // first page must still inspect it rather than treating it as a cursor.
    let other = Fixture::new();
    let intent = other.intent();
    assert!(other
        .db
        .prepare_plugin_apply(&other.context, &intent)
        .unwrap());
    let db = other.db.open(&other.db.automation_path()).unwrap();
    db.execute(
        "UPDATE plugin_sync_apply_intents SET intent_id = '' WHERE intent_id = ?1",
        [&intent.intent_id],
    )
    .unwrap();
    drop(db);
    assert!(other
        .db
        .plugin_apply_recovery_page(&other.context, None, 10)
        .is_err());
}
