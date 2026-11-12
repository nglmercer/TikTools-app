use super::*;

#[test]
fn state_and_candidate_rollback_together_when_outbox_write_fails() {
    let f = Fixture::new();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_sync_insert BEFORE INSERT ON plugin_sync_outbox BEGIN SELECT RAISE(FAIL,'outbox failure'); END;").unwrap();
    drop(db);
    assert!(f
        .db
        .set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
        .is_err());
    let db = f.db.open(&f.db.automation_path()).unwrap();
    for table in [
        "behavior_plugins",
        "plugin_sync_source_versions",
        "plugin_sync_outbox",
    ] {
        let count: i64 = db
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "{table} must roll back");
    }
}

#[test]
fn legacy_outbox_migration_preserves_unknown_consent_and_is_repeatable() {
    let f = Fixture::new();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    // Reproduce the interim schema in this test's disposable database.
    db.execute_batch(
        "DROP TABLE plugin_sync_outbox;
         CREATE TABLE plugin_sync_outbox (
           origin TEXT NOT NULL, account_id TEXT NOT NULL, plugin_id TEXT NOT NULL,
           generation INTEGER NOT NULL CHECK(generation > 0),
           version TEXT, enabled INTEGER CHECK(enabled IN (0,1)),
           CHECK((version IS NULL AND enabled IS NULL) OR (version IS NOT NULL AND enabled IS NOT NULL)),
           PRIMARY KEY(origin, account_id, plugin_id)
         );
         INSERT INTO plugin_sync_source_versions(plugin_id,generation) VALUES ('hotkeys',1);
         INSERT INTO plugin_sync_outbox VALUES ('https://cloud.example','alice','hotkeys',1,'1.0.0',1);"
    ).unwrap();
    drop(db);

    for _ in 0..2 {
        let reopened = DatabaseManager::new(f.db.paths().clone());
        let db = reopened.open(&reopened.automation_path()).unwrap();
        let row: (String, String, String, i64, Option<String>, Option<String>, Option<i64>) = db.query_row(
            "SELECT origin,account_id,plugin_id,generation,config_generation,version,enabled FROM plugin_sync_outbox",
            [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?)),
        ).unwrap();
        assert_eq!(
            row,
            (
                "https://cloud.example".into(),
                "alice".into(),
                "hotkeys".into(),
                1,
                None,
                Some("1.0.0".into()),
                Some(1)
            )
        );
        assert!(matches!(
            reopened.next_plugin_sync_intent("https://cloud.example", "alice"),
            Err(DatabaseError::Invalid(_))
        ));
    }
    // A later write must not replace the unknown-consent row or advance
    // its source as a side effect of a failed reservation.
    assert!(f
        .db
        .set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
        .is_err());
    let db = f.db.open(&f.db.automation_path()).unwrap();
    let preserved: (i64, Option<String>) = db
        .query_row(
            "SELECT generation,config_generation FROM plugin_sync_outbox WHERE account_id='alice'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(preserved, (1, None));
    let source: i64 = db
        .query_row(
            "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id='hotkeys'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(source, 1);
    drop(db);
    // New candidates do carry consent and remain writable after migration.
    f.db.set_plugin_state_with_sync_intent("other", true, true, Some(f.intent("bob")))
        .unwrap();
    assert_eq!(
        f.db.next_plugin_sync_intent("https://cloud.example", "bob")
            .unwrap()
            .unwrap()
            .config_generation,
        7
    );
}
