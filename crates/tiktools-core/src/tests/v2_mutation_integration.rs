//! Native v2 outbox integration with a persisted protocol fixture and fake
//! plugin runtime; SQLite and the source/workspace adapters are real.
//!
//! Product migration does not yet expose the marker/consent creation path,
//! so this fixture seeds the same reviewed state on disk. The sync crate's
//! cutover tests independently exercise creation through its real APIs.
use super::*;
use cloud_client::{WorkspaceEdit, WorkspaceKey, WorkspacePendingStatus};
use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore};
use tiktools_cloud_sync::{
    PluginSelection, V2ProductDecision, V2ProductUploadSource, WorkspaceCoordinator,
};
use tiktools_plugin_package::Sha256Digest;

fn assert_staged_decision(workspace: &WorkspaceCoordinator, decision: &V2ProductDecision) {
    let state = workspace.snapshot().unwrap().state;
    let key = WorkspaceKey::new("plugin-selections", &decision.plugin_id).unwrap();
    let expected_edit = match decision.selection.as_ref() {
        Some(selection) => WorkspaceEdit::Put(serde_json::to_value(selection).unwrap()),
        None => WorkspaceEdit::Delete,
    };
    let pending = state.entry(&key).unwrap().pending.as_ref().unwrap();
    assert_eq!(pending.status, WorkspacePendingStatus::Queued);
    assert_eq!(pending.edit, expected_edit);
    let proof = state
        .product_queued_proof(&key)
        .expect("queued source proof");
    assert_eq!(proof.target(), &key);
    assert_eq!(proof.source_generation(), decision.source_generation);
    assert_eq!(proof.edit(), &expected_edit);
    assert_eq!(proof.local_generation(), pending.local_generation);
    let digest = Sha256Digest::of_bytes(&serde_json::to_vec(decision).unwrap()).to_string();
    assert_eq!(proof.decision_digest(), digest);
}

#[tokio::test]
async fn v2_without_identity_records_untracked_change_under_emission_fence() {
    let runtime = Arc::new(FakePluginState::default());
    let (mut core, _, root) = core_with_fake_plugins(Arc::clone(&runtime), &["acme.hotkeys"]);
    let paths = core.db.paths().clone();
    let origin = "https://cloud.example";
    let auth = CloudAuth::new(
        Some(origin.into()),
        paths.data.join("missing-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    Arc::get_mut(&mut core).unwrap().sync =
        Arc::new(crate::cloud_sync::CloudSync::new(&paths, &auth));
    let coordinator = core.sync.coordinator().unwrap();
    coordinator.set_enabled(true).unwrap();
    let scope = Sha256Digest::of_bytes(
        &serde_json::to_vec(&("tiktools-v2-consent", origin, "alice")).unwrap(),
    );
    std::fs::write(
        paths.data.join("cloud-sync/cloud-sync.json"),
        serde_json::to_vec(&serde_json::json!({
            "version": 2, "enabled": true, "generation": 42, "scope": scope.as_bytes()
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        coordinator
            .with_product_local_edit(|choice| choice)
            .unwrap(),
        tiktools_cloud_sync::ProductLocalEditContext::V2(None)
    ));
    let fence =
        tiktools_cloud_sync::ProductEmissionFence::open(paths.data.join("plugin-sync-emission"))
            .unwrap();
    let in_flight = fence.try_emission().unwrap();
    assert!(core
        .plugin_set_enabled_with_product_intent("acme.hotkeys", true)
        .is_err());
    assert!(!core.db.has_untracked_plugin_sync().unwrap());
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    drop(in_flight);
    let observed = Arc::new(AtomicBool::new(false));
    let observed_at_load = observed.clone();
    let fence_at_load = fence.clone();
    *runtime.on_load.lock().unwrap() = Some(Box::new(move || {
        observed_at_load.store(
            fence_at_load.try_emission().is_err(),
            AtomicOrdering::SeqCst,
        );
    }));
    core.plugin_set_enabled_with_product_intent("acme.hotkeys", true)
        .unwrap();
    assert!(observed.load(AtomicOrdering::SeqCst));
    assert!(fence.try_emission().is_ok());
    assert!(core.plugins.get("acme.hotkeys").unwrap().running);
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    core.plugin_set_enabled_with_product_intent("acme.hotkeys", false)
        .unwrap();
    observed.store(false, AtomicOrdering::SeqCst);
    let observed_at_load = observed.clone();
    let fence_at_load = fence.clone();
    *runtime.on_load.lock().unwrap() = Some(Box::new(move || {
        observed_at_load.store(
            fence_at_load.try_emission().is_err(),
            AtomicOrdering::SeqCst,
        );
    }));
    core.plugin_set_enabled_with_untracked_intent("acme.hotkeys", true)
        .unwrap();
    assert!(observed.load(AtomicOrdering::SeqCst));
    assert!(fence.try_emission().is_ok());
    assert!(crate::db::DatabaseManager::new(paths)
        .has_untracked_plugin_sync()
        .unwrap());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn v2_native_mutations_stage_confirmed_sqlite_intents_and_reject_stale_runtime() {
    let runtime = Arc::new(FakePluginState::default());
    let (mut core, _, root) = core_with_fake_plugins(Arc::clone(&runtime), &["acme.hotkeys"]);
    let paths = core.db.paths().clone();
    let origin = "https://cloud.example";
    let account = "alice";
    let sidecar = paths.data.join("v2-integration-account.json");
    std::fs::write(
        &sidecar,
        serde_json::to_vec(&serde_json::json!({"origin": origin, "account_id": account})).unwrap(),
    )
    .unwrap();
    let auth = CloudAuth::new(
        Some(origin.into()),
        sidecar,
        Arc::new(MemoryTokenStore::new()),
    );
    Arc::get_mut(&mut core).unwrap().sync =
        Arc::new(crate::cloud_sync::CloudSync::new(&paths, &auth));
    let coordinator = core.sync.coordinator().unwrap();
    coordinator.set_enabled(true).unwrap(); // Creates the real consent sentinel.
    let sync_dir = paths.data.join("cloud-sync");
    let identity = Sha256Digest::of_bytes(&serde_json::to_vec(&(origin, account)).unwrap());
    let scope = Sha256Digest::of_bytes(
        &serde_json::to_vec(&("tiktools-v2-consent", origin, account)).unwrap(),
    );
    std::fs::write(
        sync_dir.join("cloud-sync.json"),
        serde_json::to_vec(&serde_json::json!({
            "version": 2, "enabled": true, "generation": 42, "scope": scope.as_bytes()
        }))
        .unwrap(),
    )
    .unwrap();
    // This test seeds the reviewed cutover directly; create its empty
    // workspace before publishing the product protocol markers.
    let workspace = WorkspaceCoordinator::open(&sync_dir, origin.into(), account.into()).unwrap();
    workspace.bootstrap(&[], None).unwrap();
    let protocol_marker = sync_dir.join(format!("protocol-{identity}.json"));
    std::fs::write(
        &protocol_marker,
        serde_json::to_vec(&serde_json::json!({
            "formatVersion": 3, "origin": origin, "accountId": account,
            "mode": "v2", "candidateSha256": "a".repeat(64)
        }))
        .unwrap(),
    )
    .unwrap();
    let product_marker = sync_dir.join(format!("v1-product-outbox-{identity}.json"));
    std::fs::write(
        &product_marker,
        serde_json::to_vec(&serde_json::json!({
            "formatVersion": 1, "origin": origin, "accountId": account
        }))
        .unwrap(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [&protocol_marker, &product_marker] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
    // The seeded fixture simulates the completed workspace-format upgrade.
    let store = cloud_client::WorkspaceStore::open(&sync_dir, origin, account).unwrap();
    let mut snapshot = store.load().unwrap().unwrap();
    snapshot.state.mark_product_guarded();
    store
        .save(Some(snapshot.generation), &snapshot.state)
        .unwrap();
    assert_eq!(
        coordinator
            .with_v2_local_edit(|context| context.unwrap().config_generation)
            .unwrap(),
        42
    );
    let source =
        crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(&core, origin, account);

    // A cooperating HTTP emission excludes even an account-bound tracked
    // edit; Busy must leave both source and runtime unchanged.
    let emission_fence =
        tiktools_cloud_sync::ProductEmissionFence::open(paths.data.join("plugin-sync-emission"))
            .unwrap();
    let in_flight = emission_fence.try_emission().unwrap();
    assert!(core
        .plugin_set_enabled_with_product_intent("acme.hotkeys", false)
        .is_err());
    assert!(core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .is_none());
    drop(in_flight);

    core.plugin_set_enabled_with_product_intent("acme.hotkeys", false)
        .unwrap();
    let disabled = core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .unwrap();
    assert_eq!(disabled.enabled, Some(false));
    assert_eq!(disabled.config_generation, 42);
    assert_eq!(disabled.generation, disabled.source_generation);
    assert_eq!(
        disabled.runtime_epoch.as_deref(),
        Some(core.sync.runtime_epoch().unwrap())
    );
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    coordinator.stage_v2_product_decision(&source).unwrap();
    assert!(core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .is_none());
    let disabled_decision = V2ProductDecision {
        origin: origin.into(),
        account_id: account.into(),
        config_generation: 42,
        source_generation: disabled.generation as u64,
        plugin_id: "acme.hotkeys".into(),
        selection: Some(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.hotkeys".into(),
            version: "1.0.0".into(),
            enabled: false,
        }),
    };
    assert_staged_decision(&workspace, &disabled_decision);
    let before_failure = workspace.snapshot().unwrap();

    // A failed start persists a newer but unconfirmed intent; it cannot be
    // staged over the already retained workspace decision.
    runtime.fail_next_load.store(true, AtomicOrdering::SeqCst);
    assert!(core
        .plugin_set_enabled_with_product_intent("acme.hotkeys", true)
        .is_err());
    let failed = core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .unwrap();
    assert!(failed.generation > disabled.generation);
    assert_eq!(failed.runtime_epoch, None);
    assert!(coordinator.stage_v2_product_decision(&source).is_err());
    assert_eq!(workspace.snapshot().unwrap().state, before_failure.state);

    core.plugin_set_enabled_with_product_intent("acme.hotkeys", true)
        .unwrap();
    let enabled = core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .unwrap();
    assert!(enabled.generation > failed.generation);
    assert_eq!(
        enabled.runtime_epoch.as_deref(),
        Some(core.sync.runtime_epoch().unwrap())
    );
    coordinator.stage_v2_product_decision(&source).unwrap();
    let enabled_decision = V2ProductDecision {
        origin: origin.into(),
        account_id: account.into(),
        config_generation: 42,
        source_generation: enabled.generation as u64,
        plugin_id: "acme.hotkeys".into(),
        selection: Some(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.hotkeys".into(),
            version: "1.0.0".into(),
            enabled: true,
        }),
    };
    assert_staged_decision(&workspace, &enabled_decision);
    let mut admitted = false;
    source
        .with_fenced_upload(&enabled_decision, &mut || {
            admitted = true;
            Ok(())
        })
        .unwrap();
    assert!(admitted);

    // Logical removal is reserved atomically but cannot be staged until a
    // durable package/filesystem provenance contract is implemented.
    core.plugin_set_installed_with_product_intent("acme.hotkeys", false)
        .unwrap();
    let removed = core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .unwrap();
    assert_eq!((removed.version.as_ref(), removed.enabled), (None, None));
    assert_eq!(
        removed.runtime_epoch.as_deref(),
        Some(core.sync.runtime_epoch().unwrap())
    );
    let before_removal = workspace.snapshot().unwrap();
    assert!(coordinator.stage_v2_product_decision(&source).is_err());
    assert_eq!(workspace.snapshot().unwrap().state, before_removal.state);
    assert_eq!(
        core.db.next_plugin_sync_intent(origin, account).unwrap(),
        Some(removed.clone())
    );
    let removed_decision = V2ProductDecision {
        origin: origin.into(),
        account_id: account.into(),
        config_generation: 42,
        source_generation: removed.generation as u64,
        plugin_id: "acme.hotkeys".into(),
        selection: None,
    };
    admitted = false;
    assert!(source
        .with_fenced_upload(&removed_decision, &mut || {
            admitted = true;
            Ok(())
        })
        .is_err());
    assert!(!admitted);
    core.plugin_set_enabled_with_product_intent("acme.hotkeys", true)
        .unwrap();
    let restored = core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .unwrap();
    coordinator.stage_v2_product_decision(&source).unwrap();
    let enabled_decision = V2ProductDecision {
        source_generation: restored.generation as u64,
        ..enabled_decision
    };

    // Deliberately stop the fake worker without changing SQLite source or
    // host activation, modelling a worker lost after confirmation. The
    // source-generation check must still pass independently while runtime
    // admission refuses the same previously confirmed decision.
    core.plugins.stop("acme.hotkeys").unwrap();
    core.db
        .with_current_v2_upload_source::<_, crate::db::DatabaseError>(
            "acme.hotkeys",
            enabled_decision.source_generation,
            true,
            || Ok(()),
        )
        .unwrap();
    admitted = false;
    assert!(source
        .with_fenced_upload(&enabled_decision, &mut || {
            admitted = true;
            Ok(())
        })
        .is_err());
    assert!(!admitted);
    assert_staged_decision(&workspace, &enabled_decision);
    core.plugins.start("acme.hotkeys").unwrap();
    source
        .with_fenced_upload(&enabled_decision, &mut || {
            admitted = true;
            Ok(())
        })
        .unwrap();
    assert!(admitted);

    // Opting out prevents staging an already confirmed source candidate.
    core.plugin_set_enabled_with_product_intent("acme.hotkeys", false)
        .unwrap();
    let pending_before_opt_out = core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .unwrap();
    coordinator.set_enabled(false).unwrap();
    let before_opt_out_stage = workspace.snapshot().unwrap();
    assert!(coordinator.stage_v2_product_decision(&source).is_err());
    assert_eq!(
        workspace.snapshot().unwrap().state,
        before_opt_out_stage.state
    );
    assert_eq!(
        core.db
            .next_plugin_sync_intent(origin, account)
            .unwrap()
            .unwrap(),
        pending_before_opt_out
    );

    // Further local-only work still succeeds and advances the source;
    // neither the old SQLite candidate nor the prior staged edit is current.
    let fence =
        tiktools_cloud_sync::ProductEmissionFence::open(paths.data.join("plugin-sync-emission"))
            .unwrap();
    let request = fence.try_emission().unwrap();
    assert!(core
        .plugin_set_enabled_with_product_intent("acme.hotkeys", true)
        .is_err());
    assert!(!core.db.has_untracked_plugin_sync().unwrap());
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    drop(request);
    core.plugin_set_enabled_with_product_intent("acme.hotkeys", true)
        .unwrap();
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    let stale = core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .unwrap();
    assert_eq!(stale.generation, pending_before_opt_out.generation);
    assert_ne!(stale.source_generation, stale.generation);
    assert!(core.plugins.get("acme.hotkeys").unwrap().running);
    admitted = false;
    assert!(source
        .with_fenced_upload(&enabled_decision, &mut || {
            admitted = true;
            Ok(())
        })
        .is_err());
    assert!(!admitted);

    // A cooperating in-flight request excludes the offline fallback without
    // waiting; SQLite and the runtime remain untouched on Busy.
    let request = fence.try_emission().unwrap();
    assert!(core
        .plugin_set_enabled_with_untracked_intent("acme.hotkeys", false)
        .is_err());
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    assert!(core.plugins.get("acme.hotkeys").unwrap().running);
    drop(request);

    // A failed marker insert rolls the whole SQLite transaction back before
    // any runtime change; the previous outbox candidate is preserved.
    let conn = rusqlite::Connection::open(paths.automation_database()).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_untracked BEFORE INSERT ON plugin_sync_untracked_local BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(core
        .plugin_set_enabled_with_untracked_intent("acme.hotkeys", false)
        .is_err());
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    assert!(core.plugins.get("acme.hotkeys").unwrap().running);
    assert_eq!(
        core.db
            .next_plugin_sync_intent(origin, account)
            .unwrap()
            .unwrap(),
        stale
    );
    conn.execute_batch("DROP TRIGGER fail_untracked").unwrap();

    core.plugin_set_enabled_with_untracked_intent("acme.hotkeys", false)
        .unwrap();
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    assert!(source.acquire_emission_guard().is_err());
    let reopened = crate::db::DatabaseManager::new(paths.clone());
    assert!(reopened.has_untracked_plugin_sync().unwrap());
    core.plugin_set_enabled_with_product_intent("acme.hotkeys", true)
        .unwrap();
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    assert!(source.acquire_emission_guard().is_err());
    drop(source);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}
