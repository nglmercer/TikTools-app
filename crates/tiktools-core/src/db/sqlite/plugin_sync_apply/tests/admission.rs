use super::*;

#[test]
fn admission_fails_closed_at_retained_history_limit_without_discarding_rows() {
    let f = Fixture::new();
    let connection = f.db.open(&f.db.automation_path()).unwrap();
    connection.execute_batch("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<4096) INSERT INTO plugin_sync_apply_intents(intent_id,plugin_id,phase,payload_json) SELECT printf('%064x',x+100),'plugin.'||x,'finalized','{}' FROM n").unwrap();
    drop(connection);
    assert!(f.db.prepare_plugin_apply(&f.context, &f.intent()).is_err());
    assert!(f.db.plugin_apply_quarantine().is_err());
    let connection = f.db.open(&f.db.automation_path()).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM plugin_sync_apply_intents",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count,
        super::super::plugin_apply_quarantine::MAX_RETAINED as i64
    );
}

#[test]
fn pulled_source_observation_excludes_unresolved_edits_and_apply_journals() {
    let f = Fixture::new();
    assert_eq!(
        f.db.pulled_plugin_apply_source("acme.plugin").unwrap(),
        Some((1, true))
    );
    assert_eq!(
        f.db.pulled_plugin_apply_source("missing.plugin").unwrap(),
        None
    );
    assert!(f.db.prepare_plugin_apply(&f.context, &f.intent()).unwrap());
    assert_eq!(
        f.db.pulled_plugin_apply_source("acme.plugin").unwrap(),
        None
    );

    let other = Fixture::new();
    other
        .db
        .set_plugin_state_with_sync_intent(
            "acme.plugin",
            true,
            false,
            Some(super::super::LocalPluginSyncIntent {
                origin: &other.context.origin,
                account_id: &other.context.account_id,
                config_generation: other.context.config_generation,
                version: Some("1.0.0"),
            }),
        )
        .unwrap();
    assert_eq!(
        other.db.pulled_plugin_apply_source("acme.plugin").unwrap(),
        None
    );
}

#[test]
fn invalid_or_rewound_native_evidence_is_rejected() {
    let f = Fixture::new();
    let mut intent = f.intent();
    let base = V1Record {
        collection: "plugin-selections".into(),
        key: "acme.plugin".into(),
        revision: 5,
        deleted: false,
        value: Some(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.plugin".into(),
            version: "1.0.0".into(),
            enabled: true,
        }),
    };
    intent.base_record = Some(base.clone());
    intent.reviewed_pending.as_mut().unwrap().base_revision = Some(5);
    let mut old = base;
    old.revision = 4;
    intent.remote = Some(old);
    assert!(f.db.prepare_plugin_apply(&f.context, &intent).is_err());
    let mut context = f.context.clone();
    context.config_generation += 1;
    assert!(!f.db.prepare_plugin_apply(&context, &f.intent()).unwrap());
    f.db.set_plugin_state_with_sync_intent("acme.plugin", false, false, None)
        .unwrap();
    let mut uninstalled = f.intent();
    uninstalled.source_generation = 2;
    uninstalled.expected_enabled = false;
    assert!(!f.db.prepare_plugin_apply(&f.context, &uninstalled).unwrap());

    let g = Fixture::new();
    let candidate = super::super::LocalPluginSyncIntent {
        origin: &g.context.origin,
        account_id: "bob",
        config_generation: g.context.config_generation,
        version: Some("1.0.0"),
    };
    g.db.set_plugin_state_with_sync_intent("acme.plugin", true, true, Some(candidate))
        .unwrap();
    let mut stale_account = g.intent();
    stale_account.source_generation = 2;
    assert!(!g
        .db
        .prepare_plugin_apply(&g.context, &stale_account)
        .unwrap());
}
