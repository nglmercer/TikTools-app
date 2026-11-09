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
async fn reviewed_local_change_transfers_only_under_matching_session_and_barrier() {
    use secrecy::SecretString;
    use tiktools_cloud_auth::{Persistence, TokenStore, TokenStoreError};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    struct Store;
    impl TokenStore for Store {
        fn persistence(&self) -> Persistence {
            Persistence::MemoryOnly
        }
        fn load(&self, _: &str, _: &str) -> Result<Option<SecretString>, TokenStoreError> {
            Ok(Some(SecretString::from("test-token".to_owned())))
        }
        fn store(
            &self,
            _: &str,
            _: &str,
            _: &cloud_client::Session,
        ) -> Result<(), TokenStoreError> {
            Ok(())
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), TokenStoreError> {
            Ok(())
        }
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let remote_record = Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
    let server_record = remote_record.clone();
    let server = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let server_record = server_record.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let Ok(n) = socket.read(&mut buf).await else {
                    return;
                };
                let request = String::from_utf8_lossy(&buf[..n]);
                let line = request.lines().next().unwrap_or_default();
                let (status, value) = if line.starts_with("GET /api/v1/native/me ") {
                    ("200 OK", json!({"id":"alice"}))
                } else if line.starts_with("GET /api/v2/sync/write-epoch ") {
                    ("200 OK", json!({"writeEpoch":"7","epochRequired":true}))
                } else if line.starts_with("POST /api/v2/sync/write-barrier ") {
                    ("200 OK", json!({"writeEpoch":"8"}))
                } else if line.starts_with("GET /api/v2/sync/changes?") {
                    ("200 OK", json!({"changes":[],"cursor":0,"has_more":false}))
                } else if line
                    .starts_with("GET /api/v2/sync/collections/plugin-selections/acme.hotkeys ")
                {
                    match server_record.lock().unwrap().clone() {
                        Some(record) => ("200 OK", record),
                        None => ("404 Not Found", json!({})),
                    }
                } else if line
                    .starts_with("PUT /api/v2/sync/collections/plugin-selections/acme.hotkeys ")
                    && request
                        .to_ascii_lowercase()
                        .contains("x-workspace-write-epoch: 7")
                {
                    (
                        "200 OK",
                        json!({"collection":"plugin-selections","key":"acme.hotkeys","revision":1,"deleted":false,"value":{"schemaVersion":1,"pluginId":"acme.hotkeys","version":"1.0.0","enabled":true}}),
                    )
                } else {
                    ("404 Not Found", json!({}))
                };
                let body = serde_json::to_vec(&value).unwrap();
                let header = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = socket.write_all(header.as_bytes()).await;
                let _ = socket.write_all(&body).await;
            });
        }
    });
    let runtime = Arc::new(FakePluginState::default());
    let (mut core, _, root) = core_with_fake_plugins(runtime, &["acme.hotkeys"]);
    let paths = core.db.paths().clone();
    let sidecar = paths.data.join("reconcile-account.json");
    std::fs::write(
        &sidecar,
        serde_json::to_vec(&json!({"origin":origin,"account_id":"alice"})).unwrap(),
    )
    .unwrap();
    let auth = CloudAuth::new(Some(origin.clone()), sidecar, Arc::new(Store));
    auth.restore().await.unwrap();
    let session = auth.session().await.unwrap();
    Arc::get_mut(&mut core).unwrap().sync =
        Arc::new(crate::cloud_sync::CloudSync::new(&paths, &auth));
    let sync = core.sync.coordinator().unwrap().clone();
    sync.set_enabled(true).unwrap();
    let sync_dir = paths.data.join("cloud-sync");
    let workspace = WorkspaceCoordinator::open(&sync_dir, origin.clone(), "alice".into()).unwrap();
    workspace.bootstrap(&[], None).unwrap();
    let store = cloud_client::WorkspaceStore::open(&sync_dir, &origin, "alice").unwrap();
    let mut snapshot = store.load().unwrap().unwrap();
    snapshot.state.mark_product_guarded();
    store
        .save(Some(snapshot.generation), &snapshot.state)
        .unwrap();
    let identity = Sha256Digest::of_bytes(&serde_json::to_vec(&(&origin, "alice")).unwrap());
    let scope = Sha256Digest::of_bytes(
        &serde_json::to_vec(&("tiktools-v2-consent", &origin, "alice")).unwrap(),
    );
    std::fs::write(
        sync_dir.join("cloud-sync.json"),
        serde_json::to_vec(
            &json!({"version":2,"enabled":true,"generation":42,"scope":scope.as_bytes()}),
        )
        .unwrap(),
    )
    .unwrap();
    let protocol_marker = sync_dir.join(format!("protocol-{identity}.json"));
    std::fs::write(&protocol_marker, serde_json::to_vec(&json!({"formatVersion":3,"origin":origin,"accountId":"alice","mode":"v2","candidateSha256":"a".repeat(64)})).unwrap()).unwrap();
    let product_marker = sync_dir.join(format!("v1-product-outbox-{identity}.json"));
    std::fs::write(
        &product_marker,
        serde_json::to_vec(&json!({"formatVersion":1,"origin":origin,"accountId":"alice"}))
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

    core.plugin_set_enabled_with_untracked_intent("acme.hotkeys", false)
        .unwrap();
    let source =
        crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(&core, &origin, "alice");
    let stale = source
        .review_untracked_selection("acme.hotkeys")
        .unwrap()
        .unwrap();
    core.plugin_set_enabled_with_untracked_intent("acme.hotkeys", true)
        .unwrap();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.hotkeys".into())
        .await
        .unwrap();
    assert!(sync
        .with_v2_reconciliation_barrier(&proof, |admission| source
            .reconcile_reviewed_under_barrier(&stale, admission))
        .await
        .is_err());
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    assert!(core
        .db
        .next_plugin_sync_intent(&origin, "alice")
        .unwrap()
        .is_none());
    let reviewed = source
        .review_untracked_selection("acme.hotkeys")
        .unwrap()
        .unwrap();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.other".into())
        .await
        .unwrap();
    assert!(sync
        .with_v2_reconciliation_barrier(&proof, |admission| source
            .reconcile_reviewed_under_barrier(&reviewed, admission))
        .await
        .is_err());
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    let wrong_account =
        crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(&core, &origin, "bob");
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.hotkeys".into())
        .await
        .unwrap();
    assert!(sync
        .with_v2_reconciliation_barrier(&proof, |admission| wrong_account
            .reconcile_reviewed_under_barrier(&reviewed, admission))
        .await
        .is_err());
    assert!(core.db.has_untracked_plugin_sync().unwrap());

    let (other, _, other_root) =
        core_with_fake_plugins(Arc::new(FakePluginState::default()), &["acme.hotkeys"]);
    other.plugin_set_enabled("acme.hotkeys", false).unwrap();
    let other_source =
        crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(&other, &origin, "alice");
    let other_review = other_source
        .review_untracked_selection("acme.hotkeys")
        .unwrap()
        .unwrap();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.hotkeys".into())
        .await
        .unwrap();
    assert!(sync
        .with_v2_reconciliation_barrier(&proof, |admission| other_source
            .reconcile_reviewed_under_barrier(&other_review, admission))
        .await
        .is_err());
    assert!(other.db.has_untracked_plugin_sync().unwrap());
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    other.shutdown().await;
    drop(other_source);
    let _ = std::fs::remove_dir_all(other_root);

    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.hotkeys".into())
        .await
        .unwrap();
    sync.with_v2_reconciliation_barrier(&proof, |admission| {
        source.reconcile_reviewed_under_barrier(&reviewed, admission)
    })
    .await
    .unwrap();
    assert!(!core.db.has_untracked_plugin_sync().unwrap());
    let pending = core
        .db
        .next_plugin_sync_intent(&origin, "alice")
        .unwrap()
        .unwrap();
    assert_eq!(pending.config_generation, 42);
    assert_eq!(pending.enabled, Some(true));
    assert_eq!(pending.runtime_epoch, None);
    assert_eq!(pending.reviewed_base.as_deref(), Some("null"));
    assert!(sync
        .with_v2_reconciliation_barrier(&proof, |_| Ok(()))
        .await
        .is_err());
    // The old local review may be discarded: confirmation reconstructs the
    // candidate from SQLite for this host lifetime.
    let runtime_review = source
        .review_reconciled_runtime("acme.hotkeys")
        .unwrap()
        .unwrap();
    // Confirmation is a separate local observation; it cannot trust the
    // runtime that was present during the earlier review.
    core.plugins.stop("acme.hotkeys").unwrap();
    assert!(source.confirm_reconciled_runtime(&runtime_review).is_err());
    assert_eq!(
        core.db
            .plugin_sync_intent_for(&origin, "alice", "acme.hotkeys")
            .unwrap()
            .unwrap()
            .runtime_epoch,
        None
    );
    core.plugins.start("acme.hotkeys").unwrap();
    core.db
        .set_plugin_state_with_sync_intent(
            "aaa.other",
            true,
            false,
            Some(crate::db::LocalPluginSyncIntent {
                origin: &origin,
                account_id: "alice",
                config_generation: 42,
                version: Some("1.0.0"),
            }),
        )
        .unwrap();
    let connection = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_confirmation BEFORE UPDATE ON plugin_sync_outbox WHEN NEW.runtime_epoch IS NOT NULL BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(source.confirm_reconciled_runtime(&runtime_review).is_err());
    assert_eq!(
        core.db
            .plugin_sync_intent_for(&origin, "alice", "acme.hotkeys")
            .unwrap()
            .unwrap()
            .runtime_epoch,
        None
    );
    connection
        .execute_batch("DROP TRIGGER reject_confirmation")
        .unwrap();
    drop(connection);
    source.confirm_reconciled_runtime(&runtime_review).unwrap();
    let confirmed = core
        .db
        .plugin_sync_intent_for(&origin, "alice", "acme.hotkeys")
        .unwrap()
        .unwrap();
    assert_eq!(
        confirmed.runtime_epoch.as_deref(),
        Some(core.sync.runtime_epoch().unwrap())
    );
    assert_eq!(
        core.db
            .next_plugin_sync_intent(&origin, "alice")
            .unwrap()
            .unwrap()
            .plugin_id,
        "aaa.other"
    );
    let connection = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
    connection
        .execute(
            "DELETE FROM plugin_sync_outbox WHERE plugin_id='aaa.other'",
            [],
        )
        .unwrap();
    drop(connection);
    sync.stage_v2_product_decision(&source).unwrap();
    assert!(core
        .db
        .plugin_sync_intent_for(&origin, "alice", "acme.hotkeys")
        .unwrap()
        .is_none());
    let pending = workspace
        .snapshot()
        .unwrap()
        .state
        .entry(&WorkspaceKey::new("plugin-selections", "acme.hotkeys").unwrap())
        .unwrap()
        .pending
        .clone()
        .unwrap();
    assert_eq!(pending.base_record, None);
    assert_eq!(pending.base_revision, None);

    // Independent synthetic fixture state: the preceding candidate is not
    // actually uploaded, so reset the workspace only for this test scenario.
    let snapshot = store.load().unwrap().unwrap();
    let mut clean = cloud_client::WorkspaceState::new(origin.clone(), "alice".into()).unwrap();
    clean
        .install_rebuild(cloud_client::WorkspaceSnapshot {
            schema_version: 2,
            cursor: 0,
            records: vec![],
        })
        .unwrap();
    clean.mark_bootstrap_complete();
    clean.mark_product_guarded();
    store.save(Some(snapshot.generation), &clean).unwrap();
    core.db
        .set_plugin_state_with_sync_intent(
            "acme.hotkeys",
            true,
            true,
            Some(crate::db::LocalPluginSyncIntent {
                origin: &origin,
                account_id: "alice",
                config_generation: 41,
                version: Some("1.0.0"),
            }),
        )
        .unwrap();
    let connection = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
    connection
        .execute(
            "UPDATE plugin_sync_outbox SET reviewed_base=NULL WHERE plugin_id='acme.hotkeys'",
            [],
        )
        .unwrap();
    drop(connection);
    let legacy = source
        .review_reconciled_runtime("acme.hotkeys")
        .unwrap()
        .unwrap();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.other".into())
        .await
        .unwrap();
    assert!(sync
        .with_v2_reconciliation_barrier(&proof, |admission| source
            .review_legacy_candidate_under_barrier(&legacy, admission))
        .await
        .is_err());
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.hotkeys".into())
        .await
        .unwrap();
    core.plugins.stop("acme.hotkeys").unwrap();
    assert!(sync
        .with_v2_reconciliation_barrier(&proof, |admission| source
            .review_legacy_candidate_under_barrier(&legacy, admission))
        .await
        .is_err());
    assert_eq!(
        core.db
            .plugin_sync_intent_for(&origin, "alice", "acme.hotkeys")
            .unwrap()
            .unwrap()
            .reviewed_base,
        None
    );
    core.plugins.start("acme.hotkeys").unwrap();
    let legacy = source
        .review_reconciled_runtime("acme.hotkeys")
        .unwrap()
        .unwrap();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.hotkeys".into())
        .await
        .unwrap();
    sync.with_v2_reconciliation_barrier(&proof, |admission| {
        source.review_legacy_candidate_under_barrier(&legacy, admission)
    })
    .await
    .unwrap();
    let recovered = core
        .db
        .plugin_sync_intent_for(&origin, "alice", "acme.hotkeys")
        .unwrap()
        .unwrap();
    assert_eq!(recovered.reviewed_base.as_deref(), Some("null"));
    assert_eq!(recovered.config_generation, 42);
    assert_eq!(recovered.runtime_epoch, None);
    assert!(sync.stage_v2_product_decision(&source).is_err());
    let runtime_review = source
        .review_reconciled_runtime("acme.hotkeys")
        .unwrap()
        .unwrap();
    source.confirm_reconciled_runtime(&runtime_review).unwrap();
    drop(source);
    Arc::get_mut(&mut core).unwrap().cloud = Arc::new(
        crate::cloud_account::CloudAccount::with_test_auth(auth.clone()),
    );
    assert!(core.workspace_v2_selected().await.unwrap());
    core.start_cloud_sync_worker();
    core.start_cloud_sync_worker();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let snapshot = workspace.snapshot().unwrap();
            let key = WorkspaceKey::new("plugin-selections", "acme.hotkeys").unwrap();
            if snapshot
                .state
                .entry(&key)
                .is_some_and(|entry| entry.remote.is_some() && entry.pending.is_none())
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("background sync did not upload the confirmed selection");
    core.stop_cloud_sync_worker().await;
    assert!(core
        .db
        .plugin_sync_intent_for(&origin, "alice", "acme.hotkeys")
        .unwrap()
        .is_none());
    // The cached server revision and native runtime must agree before the
    // remote change can create a durable application intent.
    let remote = json!({"collection":"plugin-selections","key":"acme.hotkeys","revision":2,"deleted":false,"value":{"schemaVersion":1,"pluginId":"acme.hotkeys","version":"1.0.0","enabled":false}});
    *remote_record.lock().unwrap() = Some(remote.clone());
    let snapshot = store.load().unwrap().unwrap();
    let mut state = snapshot.state;
    state
        .install_rebuild(cloud_client::WorkspaceSnapshot {
            schema_version: 2,
            cursor: 2,
            records: vec![serde_json::from_value(remote).unwrap()],
        })
        .unwrap();
    store.save(Some(snapshot.generation), &state).unwrap();
    core.db
        .invalidate_plugin_package_source_untracked("acme.hotkeys")
        .unwrap();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.hotkeys".into())
        .await
        .unwrap();
    assert!(core
        .prepare_v2_pulled_plugin_apply(&proof)
        .await
        .unwrap()
        .is_none());
    let connection = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
    connection
        .execute(
            "DELETE FROM plugin_sync_untracked_local WHERE plugin_id='acme.hotkeys'",
            [],
        )
        .unwrap();
    drop(connection);
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.hotkeys".into())
        .await
        .unwrap();
    let intent = core
        .prepare_v2_pulled_plugin_apply(&proof)
        .await
        .unwrap()
        .unwrap();
    let stored = core
        .db
        .plugin_apply_intent(&intent.intent_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored.phase, crate::db::PluginApplyPhase::Ready);
    assert!(
        core.plugins.observe_worker("acme.hotkeys").is_ok_and(
            |state| state == tiktools_plugin_loader::PluginWorkerObservation::WorkerPresent
        )
    );
    let connection = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
    connection
        .execute(
            "DELETE FROM plugin_sync_apply_intents WHERE intent_id=?1",
            [&intent.intent_id],
        )
        .unwrap();
    drop(connection);
    // A successful package operation under the same session can transfer its
    // fresh marker into the ordinary v2 outbox without another approval.
    {
        let _invocation = core.plugin_state.invocation_gate.lock().unwrap();
        let _lease = crate::plugin_packages::try_product_mutation_lease(&core).unwrap();
        let account = crate::plugin_packages::capture_package_sync_account(&core);
        assert!(account.is_some());
        core.db
            .invalidate_plugin_package_source_untracked("acme.hotkeys")
            .unwrap();
        let generation = core
            .db
            .invalidate_plugin_package_source("acme.hotkeys")
            .unwrap();
        crate::plugin_packages::confirm_package_sync_source(
            &core,
            account,
            "acme.hotkeys",
            "1.0.0",
            generation,
        );
        assert!(!core.db.has_untracked_plugin_sync().unwrap());
        let candidate = core
            .db
            .plugin_sync_intent_for(&origin, "alice", "acme.hotkeys")
            .unwrap()
            .unwrap();
        assert_eq!(candidate.version.as_deref(), Some("1.0.0"));
        assert_eq!(
            candidate.runtime_epoch.as_deref(),
            Some(core.sync.runtime_epoch().unwrap())
        );
    }
    let connection = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
    connection
        .execute(
            "DELETE FROM plugin_sync_outbox WHERE plugin_id='acme.hotkeys'",
            [],
        )
        .unwrap();
    drop(connection);
    let old_account = {
        let _invocation = core.plugin_state.invocation_gate.lock().unwrap();
        let _lease = crate::plugin_packages::try_product_mutation_lease(&core).unwrap();
        crate::plugin_packages::capture_package_sync_account(&core)
    };
    assert!(old_account.is_some());
    auth.logout().await.unwrap();
    assert!(core.sync_v2_with_restore(false).await.is_err());
    assert!(auth.session().await.is_none());
    {
        let _invocation = core.plugin_state.invocation_gate.lock().unwrap();
        let _lease = crate::plugin_packages::try_product_mutation_lease(&core).unwrap();
        let generation = core
            .db
            .invalidate_plugin_package_source_untracked("acme.hotkeys")
            .unwrap();
        crate::plugin_packages::confirm_package_sync_source(
            &core,
            old_account,
            "acme.hotkeys",
            "1.0.0",
            generation,
        );
        assert!(core.db.has_untracked_plugin_sync().unwrap());
        assert!(core
            .db
            .plugin_sync_intent_for(&origin, "alice", "acme.hotkeys")
            .unwrap()
            .is_none());
    }
    core.shutdown().await;
    server.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn product_upload_source_rejects_new_admission_after_shutdown() {
    let runtime = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_plugins(runtime, &["acme.hotkeys"]);
    let source = crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(
        &core,
        "https://cloud.example",
        "alice",
    );
    drop(source.acquire_emission_guard().unwrap());
    core.shutdown().await;
    assert!(source.acquire_emission_guard().is_err());
    let decision = V2ProductDecision {
        origin: "https://cloud.example".into(),
        account_id: "alice".into(),
        config_generation: 42,
        source_generation: 1,
        plugin_id: "acme.hotkeys".into(),
        selection: Some(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.hotkeys".into(),
            version: "1.0.0".into(),
            enabled: false,
        }),
    };
    let mut admitted = false;
    assert!(source
        .with_fenced_upload(&decision, &mut || {
            admitted = true;
            Ok(())
        })
        .is_err());
    assert!(!admitted);
    drop(source);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn legacy_remote_selection_does_not_create_local_reconciliation_marker() {
    let runtime = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_plugins(runtime, &["acme.hotkeys"]);
    assert!(!core.db.has_untracked_plugin_sync().unwrap());
    assert!(core
        .plugin_apply_remote_selection("acme.hotkeys", false)
        .unwrap());
    assert!(!core.db.has_untracked_plugin_sync().unwrap());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn reconciled_runtime_confirmation_can_be_rebuilt_after_host_restart() {
    let state = Arc::new(FakePluginState::default());
    let (first, _, root) = core_with_fake_plugins(state, &["acme.hotkeys"]);
    let paths = first.db.paths().clone();
    first
        .db
        .set_plugin_state_with_sync_intent(
            "acme.hotkeys",
            true,
            true,
            Some(crate::db::LocalPluginSyncIntent {
                origin: "https://cloud.example",
                account_id: "alice",
                config_generation: 7,
                version: Some("1.0.0"),
            }),
        )
        .unwrap();
    first.set_plugin_activation("acme.hotkeys", true, true);
    first.plugins.start("acme.hotkeys").unwrap();
    let source = crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(
        &first,
        "https://cloud.example",
        "alice",
    );
    let review = source
        .review_reconciled_runtime("acme.hotkeys")
        .unwrap()
        .unwrap();
    source.confirm_reconciled_runtime(&review).unwrap();
    let old_epoch = first.sync.runtime_epoch().unwrap().to_owned();
    first.shutdown().await;
    drop(source);
    drop(first);

    let (manager, manager_root) =
        fake_hotkey_manager(Arc::new(FakePluginState::default()), &["acme.hotkeys"]);
    let mut restarted = AppCore::with_paths(
        Arc::new(RecordingEmitter::default()),
        Arc::new(NoopMediaHost),
        paths,
    );
    restarted.plugins = Arc::new(manager);
    let restarted = Arc::new(restarted);
    restarted.set_plugin_activation("acme.hotkeys", true, true);
    restarted.plugins.start("acme.hotkeys").unwrap();
    let source = crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(
        &restarted,
        "https://cloud.example",
        "alice",
    );
    let review = source
        .review_reconciled_runtime("acme.hotkeys")
        .unwrap()
        .unwrap();
    source.confirm_reconciled_runtime(&review).unwrap();
    let new_epoch = restarted.sync.runtime_epoch().unwrap();
    assert_ne!(new_epoch, old_epoch);
    assert_eq!(
        restarted
            .db
            .plugin_sync_intent_for("https://cloud.example", "alice", "acme.hotkeys")
            .unwrap()
            .unwrap()
            .runtime_epoch
            .as_deref(),
        Some(new_epoch)
    );
    restarted.shutdown().await;
    drop(source);
    let _ = std::fs::remove_dir_all(manager_root);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn untracked_local_review_revalidates_runtime_and_source_before_callback() {
    let runtime = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_plugins(runtime, &["acme.hotkeys"]);
    let source = crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(
        &core,
        "https://cloud.example",
        "alice",
    );
    assert!(source
        .review_untracked_selection("acme.hotkeys")
        .unwrap()
        .is_none());
    core.plugin_set_enabled("acme.hotkeys", false).unwrap();
    let review = source
        .review_untracked_selection("acme.hotkeys")
        .unwrap()
        .unwrap();
    let version = source
        .with_current_untracked_selection(&review, |entry, version| {
            assert!(!entry.enabled);
            assert!(entry.installed);
            Ok(version.to_owned())
        })
        .unwrap();
    assert_eq!(version, "1.0.0");
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    let fence = tiktools_cloud_sync::ProductEmissionFence::open(
        core.db.paths().data.join("plugin-sync-emission"),
    )
    .unwrap();
    let emission = fence.try_emission().unwrap();
    assert!(source
        .with_current_untracked_selection::<()>(&review, |_, _| panic!("busy source admitted"))
        .is_err());
    drop(emission);
    core.plugin_set_enabled("acme.hotkeys", true).unwrap();
    assert!(source
        .with_current_untracked_selection::<()>(&review, |_, _| panic!("stale source admitted"))
        .is_err());
    let review = source
        .review_untracked_selection("acme.hotkeys")
        .unwrap()
        .unwrap();
    core.plugins.stop("acme.hotkeys").unwrap();
    assert!(source
        .with_current_untracked_selection::<()>(&review, |_, _| panic!("stale runtime admitted"))
        .is_err());
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    // Exercise the storage handoff under the actual local source fence.
    // This synthetic fixture does not provide Cloud consent or a barrier.
    core.plugins.start("acme.hotkeys").unwrap();
    let review = source
        .review_untracked_selection("acme.hotkeys")
        .unwrap()
        .unwrap();
    source
        .with_current_untracked_selection(&review, |entry, version| {
            core.db
                .reconcile_untracked_plugin(
                    entry,
                    &crate::db::LocalPluginSyncIntent {
                        origin: "https://cloud.example",
                        account_id: "alice",
                        config_generation: 7,
                        version: Some(version),
                    },
                    None,
                )
                .map_err(|error| tiktools_cloud_sync::SyncCoordError::Local(error.to_string()))
        })
        .unwrap();
    assert!(!core.db.has_untracked_plugin_sync().unwrap());
    let pending = core
        .db
        .next_plugin_sync_intent("https://cloud.example", "alice")
        .unwrap()
        .unwrap();
    assert_eq!(pending.version.as_deref(), Some("1.0.0"));
    assert_eq!(pending.runtime_epoch, None);
    let runtime_review = source
        .review_reconciled_runtime("acme.hotkeys")
        .unwrap()
        .unwrap();
    source.confirm_reconciled_runtime(&runtime_review).unwrap();
    assert!(
        tiktools_cloud_sync::V2ProductSource::with_fenced_decision::<()>(&source, |_, _| panic!(
            "legacy unclassified candidate staged"
        ))
        .is_err()
    );
    assert!(core
        .db
        .plugin_sync_intent_for("https://cloud.example", "alice", "acme.hotkeys")
        .unwrap()
        .is_some());
    core.shutdown().await;
    drop(source);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn public_plugin_state_changes_exclude_inflight_product_emission() {
    let runtime = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_plugins(Arc::clone(&runtime), &["acme.hotkeys"]);
    let fence = tiktools_cloud_sync::ProductEmissionFence::open(
        core.db.paths().data.join("plugin-sync-emission"),
    )
    .unwrap();
    let connection = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
    let generation = || -> i64 {
        connection.query_row("SELECT COALESCE((SELECT generation FROM plugin_sync_source_versions WHERE plugin_id='acme.hotkeys'),0)", [], |row| row.get(0)).unwrap()
    };
    let before = generation();
    let request = fence.try_emission().unwrap();
    assert!(core.plugin_set_enabled("acme.hotkeys", true).is_err());
    assert!(core.plugin_set_installed("acme.hotkeys", false).is_err());
    assert_eq!(generation(), before);
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    assert!(!core.db.has_untracked_plugin_sync().unwrap());
    drop(request);
    connection.execute_batch("CREATE TRIGGER reject_local_marker BEFORE INSERT ON plugin_sync_untracked_local BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(core.plugin_set_enabled("acme.hotkeys", true).is_err());
    assert_eq!(generation(), before);
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    assert!(!core.db.has_untracked_plugin_sync().unwrap());
    connection
        .execute_batch("DROP TRIGGER reject_local_marker")
        .unwrap();
    let checked = Arc::new(AtomicBool::new(false));
    let observed = checked.clone();
    let runtime_fence = fence.clone();
    *runtime.on_load.lock().unwrap() = Some(Box::new(move || {
        observed.store(
            runtime_fence.try_emission().is_err(),
            AtomicOrdering::SeqCst,
        );
    }));
    core.plugin_set_enabled("acme.hotkeys", true).unwrap();
    assert!(checked.load(AtomicOrdering::SeqCst));
    assert!(generation() > before);
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    assert!(crate::db::DatabaseManager::new(core.db.paths().clone())
        .has_untracked_plugin_sync()
        .unwrap());
    let source = crate::cloud_sync::product_v2_outbox::SqliteV2ProductSource::new(
        &core,
        "https://cloud.example",
        "alice",
    );
    assert!(source.acquire_emission_guard().is_err());
    let current = generation();
    let request = fence.try_emission().unwrap();
    assert!(core.plugin_set_installed("acme.hotkeys", false).is_err());
    assert_eq!(generation(), current);
    assert!(core.plugins.get("acme.hotkeys").unwrap().running);
    drop(request);
    connection.execute_batch("CREATE TRIGGER reject_local_marker BEFORE INSERT ON plugin_sync_untracked_local BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(core.plugin_set_installed("acme.hotkeys", false).is_err());
    assert_eq!(generation(), current);
    assert!(core.plugins.get("acme.hotkeys").unwrap().running);
    connection
        .execute_batch("DROP TRIGGER reject_local_marker")
        .unwrap();
    core.plugin_set_installed("acme.hotkeys", false).unwrap();
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    assert!(generation() > current);
    assert!(fence.try_emission().is_ok());
    drop(connection);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn public_start_stop_exclude_product_emission_until_runtime_changes_finish() {
    let runtime = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_plugins(Arc::clone(&runtime), &["acme.hotkeys"]);
    let fence = tiktools_cloud_sync::ProductEmissionFence::open(
        core.db.paths().data.join("plugin-sync-emission"),
    )
    .unwrap();
    let request = fence.try_emission().unwrap();
    assert!(core.plugin_start("acme.hotkeys").is_err());
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    drop(request);
    let checked = Arc::new(AtomicBool::new(false));
    let observed = checked.clone();
    let runtime_fence = fence.clone();
    *runtime.on_load.lock().unwrap() = Some(Box::new(move || {
        observed.store(
            runtime_fence.try_emission().is_err(),
            AtomicOrdering::SeqCst,
        );
    }));
    core.plugin_start("acme.hotkeys").unwrap();
    assert!(checked.load(AtomicOrdering::SeqCst));
    let request = fence.try_emission().unwrap();
    assert!(core.plugin_stop("acme.hotkeys").is_err());
    assert!(core.plugins.get("acme.hotkeys").unwrap().running);
    drop(request);
    core.plugin_stop("acme.hotkeys").unwrap();
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    assert!(fence.try_emission().is_ok());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
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
    assert!(core.plugin_set_enabled("acme.hotkeys", false).is_err());
    assert!(core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .is_none());
    drop(in_flight);

    core.plugin_set_enabled("acme.hotkeys", false).unwrap();
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
    // Use the actual SQLite-backed upload adapter, not just a raw file lock.
    // The shared lease must exclude both tracked and public runtime writers.
    let in_flight = source.acquire_emission_guard().unwrap();
    let mut admitted = false;
    source
        .with_fenced_upload(&disabled_decision, &mut || {
            admitted = true;
            Ok(())
        })
        .unwrap();
    assert!(admitted);
    assert!(core
        .plugin_set_enabled_with_product_intent("acme.hotkeys", true)
        .is_err());
    assert!(core.plugin_set_enabled("acme.hotkeys", true).is_err());
    assert!(core.plugin_start("acme.hotkeys").is_err());
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    assert!(core
        .db
        .next_plugin_sync_intent(origin, account)
        .unwrap()
        .is_none());
    assert_staged_decision(&workspace, &disabled_decision);
    drop(in_flight);
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

    // Logical removal can be staged only with the package and runtime fence.
    core.plugin_set_installed("acme.hotkeys", false).unwrap();
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
    let removed_decision = V2ProductDecision {
        origin: origin.into(),
        account_id: account.into(),
        config_generation: 42,
        source_generation: removed.generation as u64,
        plugin_id: "acme.hotkeys".into(),
        selection: None,
    };
    admitted = false;
    #[cfg(feature = "plugin-install")]
    {
        coordinator.stage_v2_product_decision(&source).unwrap();
        assert_staged_decision(&workspace, &removed_decision);
        assert!(core
            .db
            .next_plugin_sync_intent(origin, account)
            .unwrap()
            .is_none());
        source
            .with_fenced_upload(&removed_decision, &mut || {
                admitted = true;
                Ok(())
            })
            .unwrap();
        assert!(admitted);
    }
    #[cfg(not(feature = "plugin-install"))]
    {
        let before_removal = workspace.snapshot().unwrap();
        assert!(coordinator.stage_v2_product_decision(&source).is_err());
        assert_eq!(workspace.snapshot().unwrap().state, before_removal.state);
        assert_eq!(
            core.db.next_plugin_sync_intent(origin, account).unwrap(),
            Some(removed.clone())
        );
        assert!(source
            .with_fenced_upload(&removed_decision, &mut || {
                admitted = true;
                Ok(())
            })
            .is_err());
        assert!(!admitted);
    }
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
    assert!(source
        .with_fenced_upload(&removed_decision, &mut || Ok(()))
        .is_err());

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

#[tokio::test]
async fn public_plugin_toggle_stays_local_when_cloud_admission_is_damaged() {
    let runtime = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_plugins(runtime, &["acme.hotkeys"]);
    core.sync.coordinator().unwrap().set_enabled(true).unwrap();
    std::fs::remove_file(core.db.paths().data.join("cloud-sync/cloud-sync.json")).unwrap();
    core.plugin_set_enabled("acme.hotkeys", true).unwrap();
    assert!(core.plugins.get("acme.hotkeys").unwrap().running);
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    core.plugin_set_installed("acme.hotkeys", false).unwrap();
    assert!(!core.plugins.get("acme.hotkeys").unwrap().running);
    assert!(core.db.has_untracked_plugin_sync().unwrap());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}
