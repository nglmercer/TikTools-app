use super::*;

#[tokio::test]
async fn legacy_promotion_preserves_pending_edits_and_resumes_after_interruption() {
    struct Source(Vec<PluginSelection>);
    impl crate::CutoverSource for Source {
        fn with_guarded_source<T>(
            &self,
            operation: impl FnOnce(
                &[PluginSelection],
                &[crate::V1CutoverReceipt],
            ) -> Result<T, SyncCoordError>,
        ) -> Result<T, SyncCoordError> {
            operation(&self.0, &[])
        }
    }
    let dir = TempDir::new("legacy-product-promotion");
    let mock = MockSync::start("promotion-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "promotion-login").await;
    let session = auth.session().await.unwrap();
    sync.record_local_change("acme.plugin", "1.0.0", false)
        .unwrap();
    let source = Source(vec![PluginSelection {
        schema_version: 1,
        plugin_id: "acme.plugin".into(),
        version: "1.0.0".into(),
        enabled: true,
    }]);
    let stale = sync
        .guarded_legacy_promotion_review(&session, &source.0)
        .unwrap();
    sync.record_local_change("acme.other", "1.0.0", true)
        .unwrap();
    assert!(sync
        .execute_cutover_and_scope_consent_with_source(&stale, &source)
        .is_err());
    assert!(matches!(
        sync.protocol
            .try_v1()
            .unwrap()
            .ensure_product_outbox(&mock.origin, "promotion-user"),
        Err(ProtocolFenceError::ProductOutboxNotEnabled)
    ));
    let legacy = sync
        .protocol
        .legacy_snapshot(&mock.origin, "promotion-user")
        .unwrap()
        .unwrap();
    let review = sync
        .guarded_legacy_promotion_review(&session, &source.0)
        .unwrap();
    assert!(review.preview().entries().iter().all(|entry| entry
        .pending
        .as_ref()
        .is_some_and(|pending| pending.status == WorkspacePendingStatus::Conflict)));
    // Both crash windows reconstruct the exact reviewed legacy source.
    sync.protocol
        .try_cutover()
        .unwrap()
        .ensure_legacy_promotion_marker(&mock.origin, "promotion-user")
        .unwrap();
    let review = sync
        .guarded_legacy_promotion_review(&session, &source.0)
        .unwrap();
    assert!(sync
        .execute_guarded_cutover_with_hooks(
            &review,
            &source.0,
            || Err(SyncCoordError::Local("interrupted".into())),
            || Ok(())
        )
        .is_err());
    let restarted = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    let changed = vec![PluginSelection {
        enabled: false,
        ..source.0[0].clone()
    }];
    assert!(restarted
        .guarded_legacy_promotion_review(&session, &changed)
        .is_err());
    let review = restarted
        .guarded_legacy_promotion_review(&session, &source.0)
        .unwrap();
    assert_eq!(
        restarted
            .execute_cutover_and_scope_consent_with_source(&review, &source)
            .unwrap(),
        crate::CutoverExecution::Completed
    );
    assert!(restarted.capture_v2_config_epoch(&session).is_some());
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "promotion-user".into(),
    )
    .unwrap();
    let state = workspace.snapshot().unwrap().state;
    assert!(state.is_product_guarded());
    assert!(state.entries.iter().all(|entry| entry
        .pending
        .as_ref()
        .is_some_and(|pending| pending.status == WorkspacePendingStatus::Conflict)));
    let after = restarted
        .protocol
        .legacy_snapshot(&mock.origin, "promotion-user")
        .unwrap()
        .unwrap();
    assert_eq!(legacy.generation, after.generation);
    assert_eq!(legacy.state, after.state);
    mock.stop();
}

#[tokio::test]
async fn v2_epoch_policy_setup_is_once_and_bound_to_current_consent() {
    let dir = TempDir::new("v2-policy-setup");
    let mock = MockSync::start("policy-setup-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "policy-setup").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "policy-setup-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "policy-setup-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    lease
        .finish(&preview, &mock.origin, "policy-setup-user")
        .unwrap();
    drop(lease);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    mock.inner.lock().unwrap().v2_epoch_required = None;
    let mut context = crate::V2LocalEditContext {
        origin: mock.origin.clone(),
        account_id: "policy-setup-user".into(),
        config_generation: sync.read_config().generation,
    };
    let expected = context.clone();
    assert!(sync
        .ensure_v2_write_epoch_policy(session.clone(), expected.clone())
        .await
        .is_err());
    assert_eq!(mock.inner.lock().unwrap().v2_require_requests, 0);
    mock.inner.lock().unwrap().v2_epoch_required = Some(false);
    context.config_generation ^= 1;
    assert!(sync
        .ensure_v2_write_epoch_policy(session.clone(), context)
        .await
        .is_err());
    assert_eq!(mock.inner.lock().unwrap().v2_require_requests, 0);
    assert!(sync
        .ensure_v2_write_epoch_policy(session.clone(), expected.clone())
        .await
        .unwrap());
    assert!(!sync
        .ensure_v2_write_epoch_policy(session.clone(), expected.clone())
        .await
        .unwrap());
    assert_eq!(mock.inner.lock().unwrap().v2_require_requests, 1);
    assert_eq!(mock.inner.lock().unwrap().v2_write_epoch, "8");
    auth.logout().await.unwrap();
    assert!(sync
        .ensure_v2_write_epoch_policy(session, expected)
        .await
        .is_err());
    assert_eq!(mock.inner.lock().unwrap().v2_require_requests, 1);
    mock.stop();
}

#[tokio::test]
async fn interrupted_completed_cutover_scopes_consent_without_reenabling_opt_out() {
    struct EmptySource;
    impl crate::CutoverSource for EmptySource {
        fn with_guarded_source<T>(
            &self,
            operation: impl FnOnce(
                &[PluginSelection],
                &[crate::V1CutoverReceipt],
            ) -> Result<T, SyncCoordError>,
        ) -> Result<T, SyncCoordError> {
            operation(&[], &[])
        }
    }
    for disabled in [false, true] {
        let dir = TempDir::new("v2-finish-scope-recovery");
        let mock = MockSync::start("scope-user").await;
        let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
        login(&auth, "scope-login").await;
        let session = auth.session().await.unwrap();
        let lease = sync.protocol.try_cutover().unwrap();
        lease
            .require_product_outbox(&mock.origin, "scope-user")
            .unwrap();
        drop(lease);
        let review = sync
            .guarded_marked_cutover_review(&session, &[], &[])
            .unwrap();
        sync.execute_cutover_with_source(&review, &EmptySource)
            .unwrap();
        assert!(sync.capture_v2_config_epoch(&session).is_none());
        if disabled {
            sync.set_enabled(false).unwrap();
        }
        let result = sync.complete_interrupted_v2_scope(&session);
        if disabled {
            assert!(result.is_err());
            assert!(!sync.is_enabled());
        } else {
            result.unwrap();
            assert!(sync.capture_v2_config_epoch(&session).is_some());
            assert!(sync.complete_interrupted_v2_scope(&session).is_err());
        }
        mock.stop();
    }
}

#[tokio::test]
async fn reconciliation_barrier_is_single_use_and_bound_to_current_workspace_and_session() {
    let dir = TempDir::new("v2-reconcile-barrier");
    let mock = MockSync::start("reconcile-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "first-reconcile-login").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let cutover = sync.protocol.try_cutover().unwrap();
    cutover
        .prepare(&preview, &mock.origin, "reconcile-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "reconcile-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    cutover
        .finish(&preview, &mock.origin, "reconcile-user")
        .unwrap();
    drop(cutover);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();

    // Cancellation after the server advances the epoch returns no proof and
    // must release local admission; the server's completed barrier is not
    // mistaken for an acknowledged review.
    let source_gate = crate::ProductEmissionFence::open(dir.join("plugin-sync-emission")).unwrap();
    mock.held_v2_barrier
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let worker_sync = sync.clone();
    let worker_session = session.clone();
    let task = tokio::spawn(async move {
        worker_sync
            .prepare_v2_reconciliation_barrier(worker_session, "acme.plugin".into())
            .await
    });
    tokio::time::timeout(
        Duration::from_secs(3),
        mock.held_v2_barrier.started.notified(),
    )
    .await
    .unwrap();
    assert!(source_gate.try_emission().is_err());
    task.abort();
    assert!(task.await.err().unwrap().is_cancelled());
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if source_gate.try_emission().is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    mock.held_v2_barrier.release.notify_one();

    mock.inner.lock().unwrap().v2_epoch_required = Some(false);
    assert!(sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .is_err());
    mock.inner.lock().unwrap().v2_epoch_required = Some(true);
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .unwrap();
    assert!(sync
        .with_v2_reconciliation_preview(&proof, |record| record.is_none())
        .unwrap());
    mock.inner.lock().unwrap().v2_read_record = Some(serde_json::json!({
        "collection": "plugin-selections", "key": "acme.plugin", "revision": 1,
        "deleted": false, "value": {"schemaVersion": 1, "pluginId": "acme.plugin", "version": "1.0.0", "enabled": true}
    }));
    assert!(matches!(
        sync.with_v2_reconciliation_barrier::<()>(&proof, |_| panic!("changed remote admitted"))
            .await,
        Err(crate::WorkspaceCoordError::State(
            cloud_client::WorkspaceStateError::StaleReview
        ))
    ));
    assert!(source_gate.try_emission().is_ok());
    assert!(sync.with_v2_reconciliation_preview(&proof, |_| ()).is_err());
    // A current server value is insufficient if the local cache has not yet
    // observed that same revision. A later matching cache may be admitted.
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .unwrap();
    assert!(sync
        .with_cached_v2_reconciliation_barrier::<()>(&proof, |_| panic!("uncached record admitted"))
        .await
        .is_err());
    let store =
        cloud_client::WorkspaceStore::open(dir.join("a-sync"), &mock.origin, "reconcile-user")
            .unwrap();
    let old = store.load().unwrap().unwrap();
    let mut cached = old.state.clone();
    let record =
        serde_json::from_value(mock.inner.lock().unwrap().v2_read_record.clone().unwrap()).unwrap();
    cached
        .install_rebuild(cloud_client::WorkspaceSnapshot {
            schema_version: 2,
            cursor: 1,
            records: vec![record],
        })
        .unwrap();
    store.save(Some(old.generation), &cached).unwrap();
    mock.inner.lock().unwrap().v2_read_record.as_mut().unwrap()["revision"] = 2.into();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .unwrap();
    assert!(sync
        .with_cached_v2_reconciliation_barrier::<()>(&proof, |_| panic!(
            "stale cached revision admitted"
        ))
        .await
        .is_err());
    mock.inner.lock().unwrap().v2_read_record.as_mut().unwrap()["revision"] = 1.into();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .unwrap();
    let revision = sync
        .with_cached_v2_reconciliation_barrier(&proof, |admission| {
            Ok(admission.remote().unwrap().revision)
        })
        .await
        .unwrap();
    assert_eq!(revision, 1);
    let current = store.load().unwrap().unwrap();
    store.save(Some(current.generation), &old.state).unwrap();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .unwrap();
    assert_eq!(
        sync.with_v2_reconciliation_preview(&proof, |record| record.unwrap().revision)
            .unwrap(),
        1
    );
    sync.with_v2_reconciliation_barrier(&proof, |_| Ok(()))
        .await
        .unwrap();
    mock.inner.lock().unwrap().v2_read_record = None;
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .unwrap();
    assert!(source_gate.try_emission().is_err());
    let context = sync
        .with_v2_reconciliation_barrier(&proof, |ctx| {
            assert!(ctx.matches_source_dir(&dir.join("plugin-sync-emission")));
            assert!(!ctx.matches_source_dir(&dir.join("other-source")));
            assert_eq!(ctx.target().collection, "plugin-selections");
            assert_eq!(ctx.target().key, "acme.plugin");
            let ctx = ctx.context();
            Ok((
                ctx.origin.clone(),
                ctx.account_id.clone(),
                ctx.config_generation,
            ))
        })
        .await
        .unwrap();
    assert_eq!(context.0, mock.origin);
    assert_eq!(context.1, "reconcile-user");
    assert_eq!(context.2, sync.read_config().generation);
    assert!(sync
        .with_v2_reconciliation_barrier(&proof, |_| Ok(()))
        .await
        .is_err());
    assert!(source_gate.try_emission().is_ok());

    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .unwrap();
    assert!(sync
        .with_v2_reconciliation_barrier::<()>(&proof, |_| Err(
            crate::WorkspaceCoordError::Ineligible
        ))
        .await
        .is_err());
    assert!(source_gate.try_emission().is_ok());
    assert!(sync
        .with_v2_reconciliation_barrier(&proof, |_| Ok(()))
        .await
        .is_err());

    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .unwrap();
    let store =
        cloud_client::WorkspaceStore::open(dir.join("a-sync"), &mock.origin, "reconcile-user")
            .unwrap();
    let mut snapshot = store.load().unwrap().unwrap();
    snapshot.state.stage(cloud_client::WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap(), cloud_client::WorkspaceEdit::Put(serde_json::json!({"schemaVersion":1,"pluginId":"acme.plugin","version":"1.0.0","enabled":true}))).unwrap();
    store
        .save(Some(snapshot.generation), &snapshot.state)
        .unwrap();
    assert!(sync
        .with_v2_reconciliation_barrier::<()>(&proof, |_| panic!("changed workspace admitted"))
        .await
        .is_err());
    drop(proof);
    assert!(sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .is_err());

    // Exact-session binding remains authoritative even for the same account.
    let store_snapshot = store.load().unwrap().unwrap();
    let mut clean =
        cloud_client::WorkspaceState::new(mock.origin.clone(), "reconcile-user".into()).unwrap();
    clean
        .install_rebuild(cloud_client::WorkspaceSnapshot {
            schema_version: 2,
            cursor: 0,
            records: vec![],
        })
        .unwrap();
    clean.mark_bootstrap_complete();
    clean.mark_product_guarded();
    store.save(Some(store_snapshot.generation), &clean).unwrap();
    let proof = sync
        .prepare_v2_reconciliation_barrier(session.clone(), "acme.plugin".into())
        .await
        .unwrap();
    auth.logout().await.unwrap();
    login(&auth, "second-reconcile-login").await;
    assert!(sync
        .with_v2_reconciliation_preview(&proof, |_| panic!("replaced session previewed"))
        .is_err());
    let reads = mock.inner.lock().unwrap().v2_record_get_requests;
    assert!(sync
        .with_v2_reconciliation_barrier::<()>(&proof, |_| panic!("replaced session admitted"))
        .await
        .is_err());
    assert_eq!(mock.inner.lock().unwrap().v2_record_get_requests, reads);
    mock.stop();
}
use cloud_client::{SyncError, WorkspaceEdit, WorkspaceKey, WorkspacePendingStatus};
use std::cell::Cell;

fn marked_preview(
    sync: &SyncCoordinator,
    session: &Arc<cloud_client::Session>,
) -> crate::CutoverPreview {
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .require_product_outbox(&session.instance_origin(), session.account_id())
        .unwrap();
    drop(lease);
    sync.preview_v2_cutover_inner(session, &[], Some(&[]), || {})
        .unwrap()
}

struct Source {
    decision: crate::V2ProductDecision,
    fail_ack: Cell<bool>,
    ack_attempts: Cell<usize>,
}
impl crate::V2ProductSource for Source {
    fn with_fenced_decision<T>(
        &self,
        stage: impl FnOnce(&crate::V2ProductDecision, Option<&str>) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError> {
        let value = stage(&self.decision, None)?;
        self.ack_attempts.set(self.ack_attempts.get() + 1);
        if self.fail_ack.replace(false) {
            return Err(SyncCoordError::Local(
                "simulated interrupted acknowledgement".into(),
            ));
        }
        Ok(value)
    }
}

#[tokio::test]
async fn v2_product_outbox_replays_receipt_after_ack_failure_and_obeys_consent() {
    let dir = TempDir::new("v2-product-stage");
    let mock = MockSync::start("v2-source-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "source-login").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "v2-source-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "v2-source-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    lease
        .finish(&preview, &mock.origin, "v2-source-user")
        .unwrap();
    drop(lease);
    let mut source = Source {
        decision: crate::V2ProductDecision {
            origin: mock.origin.clone(),
            account_id: "v2-source-user".into(),
            config_generation: sync.read_config().generation,
            source_generation: 9,
            plugin_id: "acme.plugin".into(),
            selection: Some(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "1.0.0".into(),
                enabled: true,
            }),
        },
        fail_ack: Cell::new(true),
        ack_attempts: Cell::new(0),
    };
    // A device-wide v1 decision is not consent to stage work in v2. Reject
    // before the workspace write or source acknowledgement, even after the
    // protocol marker itself has been moved to v2.
    let before = workspace.snapshot().unwrap();
    assert!(sync.stage_v2_product_decision(&source).is_err());
    assert_eq!(source.ack_attempts.get(), 0);
    assert_eq!(workspace.snapshot().unwrap().generation, before.generation);
    assert_eq!(workspace.snapshot().unwrap().state, before.state);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    source.decision.config_generation = sync.read_config().generation;
    assert!(sync.stage_v2_product_decision(&source).is_err());
    assert_eq!(source.ack_attempts.get(), 1);
    let saved = workspace.snapshot().unwrap();
    assert_eq!(saved.state.entries.len(), 1);
    let (_, staged) = sync.stage_v2_product_decision(&source).unwrap();
    assert_eq!(source.ack_attempts.get(), 2);
    assert!(!staged);
    assert_eq!(workspace.snapshot().unwrap().generation, saved.generation);
    assert_eq!(workspace.snapshot().unwrap().state, saved.state);
    source.decision.account_id = "someone-else".into();
    assert!(sync.stage_v2_product_decision(&source).is_err());
    source.decision.account_id = "v2-source-user".into();
    source.decision.selection.as_mut().unwrap().enabled = false;
    assert!(sync.stage_v2_product_decision(&source).is_err());
    assert_eq!(workspace.snapshot().unwrap().generation, saved.generation);
    assert_eq!(workspace.snapshot().unwrap().state, saved.state);
    sync.set_enabled(false).unwrap();
    assert!(sync.stage_v2_product_decision(&source).is_err());
    assert_eq!(workspace.snapshot().unwrap().generation, saved.generation);
    assert_eq!(workspace.snapshot().unwrap().state, saved.state);
    mock.stop();
}

#[tokio::test]
async fn stage_v2_product_edit_queues_preferences_once() {
    let dir = TempDir::new("v2-product-edit");
    let mock = MockSync::start("v2-edit-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "edit-login").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "v2-edit-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "v2-edit-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    lease
        .finish(&preview, &mock.origin, "v2-edit-user")
        .unwrap();
    drop(lease);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    let target = WorkspaceKey::new("preferences", "desktop").unwrap();
    let edit = WorkspaceEdit::Put(serde_json::json!({
        "schemaVersion": 1,
        "theme": "dark",
        "locale": "es-PE"
    }));
    let (generation, staged) = sync
        .stage_v2_product_edit(session.clone(), target.clone(), edit.clone())
        .unwrap();
    assert!(staged);
    assert!(generation > 0);
    let after_stage = workspace.snapshot().unwrap().generation;
    assert!(after_stage >= generation);
    let staged_snapshot = workspace.snapshot().unwrap();
    let entry = staged_snapshot
        .state
        .entries
        .iter()
        .find(|entry| entry.target == target)
        .expect("staged preferences entry");
    let pending = entry.pending.as_ref().expect("queued pending");
    assert_eq!(pending.status, WorkspacePendingStatus::Queued);
    let before_restage = workspace.snapshot().unwrap().generation;
    let (again, restaged) = sync
        .stage_v2_product_edit(session.clone(), target, edit)
        .unwrap();
    assert_eq!(again, generation);
    assert!(!restaged);
    assert_eq!(workspace.snapshot().unwrap().generation, before_restage);
    mock.stop();
}

#[tokio::test]
async fn v2_product_entries_lists_staged_edits_without_network_or_mutation() {
    let dir = TempDir::new("v2-product-entries");
    let mock = MockSync::start("v2-entries-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "entries-login").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "v2-entries-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "v2-entries-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    lease
        .finish(&preview, &mock.origin, "v2-entries-user")
        .unwrap();
    drop(lease);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    let requests_before = {
        let inner = mock.inner.lock().unwrap();
        (
            inner.pull_requests,
            inner.v2_put_requests,
            inner.v2_record_get_requests,
        )
    };
    let generation_before = workspace.snapshot().unwrap().generation;
    let empty = sync.v2_product_entries(&session).unwrap();
    assert!(empty.iter().all(|entry| entry.pending.is_none()));
    let target = WorkspaceKey::new("preferences", "desktop").unwrap();
    let edit = WorkspaceEdit::Put(serde_json::json!({
        "schemaVersion": 1,
        "theme": "dark",
        "locale": "es-PE"
    }));
    sync.stage_v2_product_edit(session.clone(), target.clone(), edit.clone())
        .unwrap();
    let staged_generation = workspace.snapshot().unwrap().generation;
    assert!(staged_generation >= generation_before);
    let entries = sync.v2_product_entries(&session).unwrap();
    let entry = entries
        .iter()
        .find(|entry| entry.target == target)
        .expect("staged entry is listed");
    let pending = entry.pending.as_ref().expect("pending edit is listed");
    assert_eq!(pending.status, WorkspacePendingStatus::Queued);
    assert_eq!(pending.edit, edit);
    // Reading is local-only and leaves the durable state untouched.
    assert_eq!(workspace.snapshot().unwrap().generation, staged_generation);
    let requests_after = {
        let inner = mock.inner.lock().unwrap();
        (
            inner.pull_requests,
            inner.v2_put_requests,
            inner.v2_record_get_requests,
        )
    };
    assert_eq!(requests_after, requests_before);
    mock.stop();
}

struct ResumeSource {
    selections: Vec<PluginSelection>,
    receipts: Vec<crate::V1CutoverReceipt>,
}

impl crate::CutoverSource for ResumeSource {
    fn with_guarded_source<T>(
        &self,
        operation: impl FnOnce(
            &[PluginSelection],
            &[crate::V1CutoverReceipt],
        ) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError> {
        operation(&self.selections, &self.receipts)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_local_edit_releases_account_mutex_but_fences_identity_transition() {
    let dir = TempDir::new("v2-product-local-identity-permit");
    let mock = MockSync::start("local-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "local-identity").await;
    let session = auth.session().await.unwrap();
    let source = ResumeSource {
        selections: vec![],
        receipts: vec![],
    };
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .require_product_outbox(&mock.origin, "local-user")
        .unwrap();
    drop(lease);
    let review = sync
        .guarded_marked_cutover_review(&session, &[], &[])
        .unwrap();
    sync.execute_cutover_with_source(&review, &source).unwrap();
    sync.set_enabled(false).unwrap();
    let resume = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&resume).unwrap();

    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let worker_sync = sync.clone();
    let worker = std::thread::spawn(move || {
        worker_sync.with_v2_local_edit(|context| {
            let context = context.expect("marked v2 edit must carry identity");
            entered_tx.send(context).unwrap();
            release_rx.recv().unwrap();
        })
    });
    let context = tokio::time::timeout(Duration::from_secs(3), entered_rx)
        .await
        .expect("local edit did not enter callback")
        .unwrap();
    assert_eq!(context.origin, mock.origin);
    assert_eq!(context.account_id, "local-user");
    let status = tokio::time::timeout(Duration::from_secs(2), auth.status())
        .await
        .expect("account status must not wait for the native edit");
    assert_eq!(status.account_id.as_deref(), Some("local-user"));
    let logout_auth = auth.clone();
    let mut logout = tokio::spawn(async move { logout_auth.logout().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut logout)
            .await
            .is_err(),
        "logout must not replace identity during the local transaction"
    );
    release_tx.send(()).unwrap();
    worker.join().unwrap().unwrap();
    tokio::time::timeout(Duration::from_secs(3), logout)
        .await
        .expect("logout must finish after the edit")
        .unwrap()
        .unwrap();
    assert!(auth.session().await.is_none());
    mock.stop();
}

#[tokio::test]
async fn v2_local_edit_requires_reviewed_protocol_and_scoped_product_marker() {
    for marked in [false, true] {
        let dir = TempDir::new("v2-product-local-edit");
        let mock = MockSync::start("local-user").await;
        let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
        login(&auth, "local-edit").await;
        let session = auth.session().await.unwrap();
        let invoked = Cell::new(false);
        // A legacy device-wide opt-in never authorizes a v2 candidate.
        assert!(sync.with_v2_local_edit(|_| invoked.set(true)).is_err());
        assert!(!invoked.get());
        assert!(matches!(
            sync.with_product_local_edit(|selection| selection).unwrap(),
            crate::ProductLocalEditContext::Legacy
        ));

        let source = ResumeSource {
            selections: vec![],
            receipts: vec![],
        };
        let review = if marked {
            let lease = sync.protocol.try_cutover().unwrap();
            lease
                .require_product_outbox(&mock.origin, "local-user")
                .unwrap();
            drop(lease);
            sync.guarded_marked_cutover_review(&session, &[], &[])
                .unwrap()
        } else {
            sync.guarded_cutover_review(&session, &[]).unwrap()
        };
        assert_eq!(
            sync.execute_cutover_with_source(&review, &source).unwrap(),
            crate::CutoverExecution::Completed
        );
        sync.set_enabled(false).unwrap();
        let resume = sync.review_v2_resume(&session, None).unwrap();
        sync.confirm_v2_resume(&resume).unwrap();

        if marked {
            let peer = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
            let expected_generation = sync.read_config().generation;
            sync.with_v2_local_edit(|context| {
                let context = context.expect("reviewed v2 edit must be identity-bound");
                assert_eq!(context.origin, mock.origin);
                assert_eq!(context.account_id, "local-user");
                assert_eq!(context.config_generation, expected_generation);
                assert!(matches!(
                    sync.set_enabled(false),
                    Err(SyncCoordError::Protocol(crate::ProtocolFenceError::Busy))
                ));
                assert!(matches!(
                    peer.with_package_journal_admission(|| ()),
                    Err(SyncCoordError::Protocol(crate::ProtocolFenceError::Busy))
                ));
            })
            .unwrap();
            sync.with_product_local_edit(|selection| {
                assert!(matches!(selection, crate::ProductLocalEditContext::V2(Some(context)) if context.config_generation == expected_generation));
                assert!(matches!(
                    peer.with_package_journal_admission(|| ()),
                    Err(SyncCoordError::Protocol(crate::ProtocolFenceError::Busy))
                ));
            }).unwrap();
            // A missing marker fails before the local transaction; corrupt or
            // deleted migration evidence is never treated as consent.
            let marker = std::fs::read_dir(dir.join("a-sync"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("v1-product-outbox-")
                })
                .expect("marked cutover created a product marker");
            std::fs::write(marker, b"invalid").unwrap();
        }
        assert!(sync.with_v2_local_edit(|_| invoked.set(true)).is_err());
        assert!(sync.with_product_local_edit(|_| invoked.set(true)).is_err());
        assert!(!invoked.get());
        // Opt-out preserves local operation without staging an account-bound
        // candidate, even when its previous product marker is damaged.
        sync.set_enabled(false).unwrap();
        assert!(sync
            .with_v2_local_edit(|context| context.is_none())
            .unwrap());
        assert!(matches!(
            sync.with_product_local_edit(|v| v).unwrap(),
            crate::ProductLocalEditContext::OptedOut
        ));
        mock.stop();
    }
}

#[tokio::test]
async fn marked_v2_resume_requires_exact_native_receipts_and_fresh_consent() {
    for after_bootstrap in [false, true] {
        let dir = TempDir::new("marked-v2-resume");
        let mock = MockSync::start("cutover-user").await;
        let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
        login(&auth, "marked-resume").await;
        sync.record_local_change("acme.plugin", "1.0.0", false)
            .unwrap();
        let session = auth.session().await.unwrap();
        let pending = sync
            .protocol
            .legacy_snapshot(&mock.origin, "cutover-user")
            .unwrap()
            .unwrap()
            .state
            .entries["acme.plugin"]
            .pending
            .clone()
            .unwrap();
        let receipt = crate::V1CutoverReceipt::new(
            "acme.plugin".into(),
            1,
            sync.read_config().generation,
            "a".repeat(64),
            pending,
        )
        .unwrap();
        let source = ResumeSource {
            selections: vec![PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "1.0.0".into(),
                enabled: false,
            }],
            receipts: vec![receipt.clone()],
        };
        let lease = sync.protocol.try_cutover().unwrap();
        lease
            .require_product_outbox(&mock.origin, "cutover-user")
            .unwrap();
        drop(lease);
        let review = sync
            .guarded_marked_cutover_review(&session, &source.selections, &source.receipts)
            .unwrap();
        assert!(sync
            .execute_guarded_cutover_with_hooks(
                &review,
                &source.selections,
                || if after_bootstrap {
                    Ok(())
                } else {
                    Err(SyncCoordError::Local("interrupted after prepare".into()))
                },
                || Err(SyncCoordError::Local("interrupted after bootstrap".into())),
            )
            .is_err());
        sync.set_enabled(false).unwrap();
        assert!(sync
            .review_v2_resume(&session, Some(&source.selections))
            .is_err());
        assert!(sync
            .review_marked_v2_resume(&session, &source.selections, &[])
            .is_err());
        let review = sync
            .review_marked_v2_resume(&session, &source.selections, &source.receipts)
            .unwrap();
        assert!(sync.confirm_v2_resume(&review).is_err());
        let mut changed = source.receipts[0].pending().clone();
        changed.base_revision = Some(17);
        let changed_receipt = crate::V1CutoverReceipt::new(
            "acme.plugin".into(),
            1,
            source.receipts[0].consent_generation(),
            "a".repeat(64),
            changed,
        )
        .unwrap();
        let stale = ResumeSource {
            selections: source.selections.clone(),
            receipts: vec![changed_receipt],
        };
        assert!(sync
            .confirm_marked_v2_resume_with_source(&review, &stale)
            .is_err());
        assert!(!sync.is_enabled());
        auth.logout().await.unwrap();
        login(&auth, "new-marked-session").await;
        assert!(sync
            .confirm_marked_v2_resume_with_source(&review, &source)
            .is_err());
        let session = auth.session().await.unwrap();
        let review = sync
            .review_marked_v2_resume(&session, &source.selections, &source.receipts)
            .unwrap();
        sync.confirm_marked_v2_resume_with_source(&review, &source)
            .unwrap();
        assert!(sync.capture_v2_config_epoch(&session).is_some());
        let review = sync
            .guarded_marked_cutover_review(&session, &source.selections, &source.receipts)
            .unwrap();
        assert_eq!(
            sync.execute_cutover_with_source(&review, &source).unwrap(),
            crate::CutoverExecution::Completed
        );
        let workspace = crate::WorkspaceCoordinator::open(
            dir.join("a-sync"),
            mock.origin.clone(),
            "cutover-user".into(),
        )
        .unwrap();
        let mut state = workspace.snapshot().unwrap().state;
        let key = cloud_client::WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
        assert_eq!(
            state.entry(&key).unwrap().pending.as_ref().unwrap().status,
            cloud_client::WorkspacePendingStatus::Conflict
        );
        assert!(state.prepare_push(&key).is_err());
        assert!(mock.inner.lock().unwrap().records.is_empty());
        mock.stop();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn product_worker_deadline_releases_v2_protocol_lease() {
    let dir = TempDir::new("v2-product-deadline");
    let mock = MockSync::start("deadline-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "deadline-login").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "deadline-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "deadline-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    lease
        .finish(&preview, &mock.origin, "deadline-user")
        .unwrap();
    drop(lease);
    let protocol = sync.protocol.clone();
    let origin = mock.origin.clone();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        crate::v2_product_outbox::run_product_worker_for(
            std::time::Duration::from_millis(250),
            move |_, _| async move {
                let _lease = protocol.try_v2(&origin, "deadline-user").unwrap();
                let _ = entered_tx.send(());
                std::future::pending::<Result<(), crate::WorkspaceCoordError>>().await
            },
        )
        .await
    });
    entered_rx.await.unwrap();
    assert!(matches!(
        sync.protocol.try_cutover(),
        Err(crate::ProtocolFenceError::Busy)
    ));
    assert!(matches!(
        task.await.unwrap(),
        Err(crate::WorkspaceCoordError::TimedOut)
    ));
    assert!(
        sync.protocol.try_cutover().is_ok(),
        "deadline must release the protocol lease once no native operation is blocked"
    );
    mock.stop();
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_native_worker_keeps_bounded_slot_until_it_exits() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let slots = Arc::new(tokio::sync::Semaphore::new(1));
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let occupied = slots.clone();
    let first = tokio::spawn(async move {
        crate::v2_product_outbox::run_product_worker_with_slots(
            Duration::from_secs(10),
            occupied,
            move |_, _| async move {
                let _ = entered_tx.send(());
                release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
                Ok::<_, crate::WorkspaceCoordError>(())
            },
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2), entered_rx)
        .await
        .unwrap()
        .unwrap();
    first.abort();
    let _ = first.await;
    assert_eq!(slots.available_permits(), 0);
    let invoked = Arc::new(AtomicBool::new(false));
    let marker = invoked.clone();
    let second = crate::v2_product_outbox::run_product_worker_with_slots(
        Duration::from_millis(75),
        slots.clone(),
        move |_, _| async move {
            marker.store(true, Ordering::Release);
            Ok::<_, crate::WorkspaceCoordError>(())
        },
    )
    .await;
    assert!(matches!(second, Err(crate::WorkspaceCoordError::TimedOut)));
    assert!(
        !invoked.load(Ordering::Acquire),
        "saturation must not start another worker"
    );
    release_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while slots.available_permits() == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("capacity must return when the blocking worker exits");
    crate::v2_product_outbox::run_product_worker_with_slots(
        Duration::from_secs(1),
        slots,
        |_, _| async { Ok::<_, crate::WorkspaceCoordError>(()) },
    )
    .await
    .unwrap();
}

struct BlockingUpload {
    entered: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}

struct ExactUpload {
    fence: crate::ProductEmissionFence,
    digest: String,
    calls: std::sync::atomic::AtomicUsize,
}

impl crate::V2ProductUploadSource for ExactUpload {
    fn acquire_emission_guard(&self) -> Result<Box<dyn Send>, crate::WorkspaceCoordError> {
        self.fence
            .try_emission()
            .map(|lease| Box::new(lease) as Box<dyn Send>)
            .map_err(|_| crate::WorkspaceCoordError::Ineligible)
    }
    fn with_fenced_upload(
        &self,
        decision: &crate::V2ProductDecision,
        admit: &mut dyn FnMut() -> Result<(), crate::WorkspaceCoordError>,
    ) -> Result<(), crate::WorkspaceCoordError> {
        if decision.digest()? != self.digest {
            return Err(cloud_client::WorkspaceStateError::StaleReview.into());
        }
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        admit()
    }
}

#[tokio::test(flavor = "current_thread")]
async fn real_auth_logout_or_session_replacement_during_v2_response_keeps_unconfirmed() {
    use cloud_client::{WorkspaceKey, WorkspacePendingStatus};
    use std::sync::atomic::Ordering;

    // Both acknowledged and conflicted writes require the session that sent
    // them. Control runs prove both mock responses can actually commit.
    for status in [200, 409, 412, 428] {
        for transition in [
            "logout",
            "replace",
            "unchanged",
            "cancel",
            "cancel_before_commit",
            "cancel_with_logout",
        ] {
            let dir = TempDir::new("v2-product-http-auth");
            let mock = MockSync::start("v2-http-user").await;
            let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
            login(&auth, "first-session").await;
            let session = auth.session().await.unwrap();
            let preview = marked_preview(&sync, &session);
            let lease = sync.protocol.try_cutover().unwrap();
            lease
                .prepare(&preview, &mock.origin, "v2-http-user", &[])
                .unwrap();
            let workspace = crate::WorkspaceCoordinator::open(
                dir.join("a-sync"),
                mock.origin.clone(),
                "v2-http-user".into(),
            )
            .unwrap();
            workspace.bootstrap_reviewed(&preview).unwrap();
            lease
                .finish(&preview, &mock.origin, "v2-http-user")
                .unwrap();
            drop(lease);
            sync.set_enabled(false).unwrap();
            let consent = sync.review_v2_resume(&session, None).unwrap();
            sync.confirm_v2_resume(&consent).unwrap();
            let decision = crate::V2ProductDecision {
                origin: mock.origin.clone(),
                account_id: "v2-http-user".into(),
                config_generation: sync.read_config().generation,
                source_generation: 7,
                plugin_id: "acme.plugin".into(),
                selection: Some(PluginSelection {
                    schema_version: 1,
                    plugin_id: "acme.plugin".into(),
                    version: "1.0.0".into(),
                    enabled: true,
                }),
            };
            let source = Source {
                decision: decision.clone(),
                fail_ack: Cell::new(false),
                ack_attempts: Cell::new(0),
            };
            sync.stage_v2_product_decision(&source).unwrap();
            let upload = Arc::new(ExactUpload {
                fence: crate::ProductEmissionFence::open(dir.join("product-emission")).unwrap(),
                digest: decision.digest().unwrap(),
                calls: std::sync::atomic::AtomicUsize::new(0),
            });
            mock.inner.lock().unwrap().v2_put_status = Some(status);
            if transition == "cancel_before_commit" {
                mock.held_v2_before_route
                    .armed
                    .store(true, Ordering::SeqCst);
            } else {
                mock.held_v2_write.armed.store(true, Ordering::SeqCst);
            }
            let worker = sync.clone();
            let captured = session.clone();
            let upload_for_worker = upload.clone();
            let task =
                tokio::spawn(
                    async move { worker.sync_v2_product(captured, upload_for_worker).await },
                );
            let reached_server = if transition == "cancel_before_commit" {
                mock.held_v2_before_route.started.notified()
            } else {
                mock.held_v2_write.committed.notified()
            };
            tokio::time::timeout(Duration::from_secs(3), reached_server)
                .await
                .expect("dispatcher must send the request");
            let key = WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
            let prepared = workspace.snapshot().unwrap();
            assert_eq!(
                prepared
                    .state
                    .entry(&key)
                    .unwrap()
                    .pending
                    .as_ref()
                    .unwrap()
                    .status,
                WorkspacePendingStatus::Unconfirmed
            );
            assert_eq!(
                prepared
                    .state
                    .entry(&key)
                    .unwrap()
                    .pending
                    .as_ref()
                    .unwrap()
                    .write_epoch
                    .as_ref()
                    .unwrap()
                    .decimal(),
                "7"
            );
            let reopened = crate::WorkspaceCoordinator::open(
                dir.join("a-sync"),
                mock.origin.clone(),
                "v2-http-user".into(),
            )
            .unwrap();
            assert_eq!(reopened.snapshot().unwrap().state, prepared.state);
            assert_eq!(upload.calls.load(Ordering::SeqCst), 2);
            assert_eq!(
                mock.inner.lock().unwrap().v2_put_requests,
                usize::from(transition != "cancel_before_commit")
            );
            if transition != "cancel_before_commit" {
                assert_eq!(
                    mock.inner.lock().unwrap().v2_received_epochs,
                    vec![Some("7".into())]
                );
            }
            assert!(matches!(
                upload.fence.try_untracked_edit(),
                Err(crate::ProtocolFenceError::Busy)
            ));
            // An account transition starting after emission admission must
            // wait for the request future, without blocking account reads.
            let logout = if matches!(transition, "logout" | "replace" | "cancel_with_logout") {
                let logout_auth = auth.clone();
                let logout = tokio::spawn(async move { logout_auth.logout().await });
                tokio::time::timeout(Duration::from_secs(3), async {
                    while auth.try_session_snapshot().is_ok() {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .expect("logout must reach transition admission");
                assert!(!logout.is_finished());
                tokio::time::timeout(Duration::from_secs(1), auth.status())
                    .await
                    .expect("account mutex must remain available");
                Some(logout)
            } else {
                None
            };
            if matches!(
                transition,
                "cancel" | "cancel_before_commit" | "cancel_with_logout"
            ) {
                task.abort();
                assert!(task.await.err().unwrap().is_cancelled());
                tokio::time::timeout(Duration::from_secs(3), async {
                    loop {
                        if let Ok(lease) = upload.fence.try_untracked_edit() {
                            drop(lease);
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
                .await
                .expect("cancellation must release the emission lease");
                let after = workspace.snapshot().unwrap();
                assert_eq!(after.generation, prepared.generation);
                assert_eq!(after.state, prepared.state);
                if let Some(logout) = logout {
                    tokio::time::timeout(Duration::from_secs(3), logout)
                        .await
                        .expect("cancellation must release session admission")
                        .unwrap()
                        .unwrap();
                }
                if transition == "cancel_before_commit" {
                    // The client is gone and the local lease is free, but
                    // an already received request can still execute later.
                    assert_eq!(mock.inner.lock().unwrap().v2_put_requests, 0);
                    mock.held_v2_before_route.release.notify_one();
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while mock.inner.lock().unwrap().v2_put_requests == 0 {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                    })
                    .await
                    .expect("server must be able to execute after client cancellation");
                    assert_eq!(
                        mock.inner.lock().unwrap().v2_received_epochs,
                        vec![Some("7".into())]
                    );
                } else {
                    assert_eq!(mock.inner.lock().unwrap().v2_put_requests, 1);
                    mock.held_v2_write.release.notify_one();
                }
                continue;
            }
            mock.held_v2_write.release.notify_one();
            let outcome = tokio::time::timeout(Duration::from_secs(3), task)
                .await
                .expect("dispatcher must finish after response")
                .unwrap();
            if let Some(logout) = logout {
                tokio::time::timeout(Duration::from_secs(3), logout)
                    .await
                    .expect("finished HTTP must release session admission")
                    .unwrap()
                    .unwrap();
                if transition == "replace" {
                    login(&auth, "replacement-session").await;
                    assert!(!Arc::ptr_eq(&session, &auth.session().await.unwrap()));
                }
            }
            let after = workspace.snapshot().unwrap();
            assert!(upload.fence.try_untracked_edit().is_ok());
            if transition != "unchanged" {
                assert!(matches!(
                    outcome,
                    Err(crate::WorkspaceCoordError::Ineligible)
                ));
                assert_eq!(after.generation, prepared.generation);
                assert_eq!(after.state, prepared.state);
                assert_eq!(
                    after
                        .state
                        .entry(&key)
                        .unwrap()
                        .pending
                        .as_ref()
                        .unwrap()
                        .status,
                    WorkspacePendingStatus::Unconfirmed
                );
            } else if status == 412 || status == 428 {
                assert!(
                    matches!(outcome, Err(crate::WorkspaceCoordError::Transport(SyncError::Http(code))) if code == status)
                );
                assert_eq!(after.state, prepared.state);
                // An explicit second sync must never replay the frozen
                // uncertain request even when the remote status changes.
                mock.inner.lock().unwrap().v2_put_status = Some(200);
                let original_session = auth.session().await.unwrap();
                sync.sync_v2_product(original_session, upload.clone())
                    .await
                    .unwrap();
                assert_eq!(mock.inner.lock().unwrap().v2_put_requests, 1);
            } else {
                let outcome = outcome.unwrap();
                if status == 200 {
                    assert_eq!(outcome.pushed, 1);
                    assert!(after.state.entry(&key).unwrap().pending.is_none());
                } else {
                    assert_eq!(outcome.conflicts, vec![key.clone()]);
                    assert_eq!(
                        after
                            .state
                            .entry(&key)
                            .unwrap()
                            .pending
                            .as_ref()
                            .unwrap()
                            .status,
                        WorkspacePendingStatus::Conflict
                    );
                }
            }
            assert!(
                sync.protocol.try_cutover().is_ok(),
                "request must release protocol lease"
            );
            mock.stop();
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn logout_after_session_admission_before_final_check_prevents_send() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let dir = TempDir::new("v2-product-emission-session");
    let mock = MockSync::start("emission-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "emission-session").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "emission-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "emission-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    lease
        .finish(&preview, &mock.origin, "emission-user")
        .unwrap();
    drop(lease);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    let decision = crate::V2ProductDecision {
        origin: mock.origin.clone(),
        account_id: "emission-user".into(),
        config_generation: sync.read_config().generation,
        source_generation: 7,
        plugin_id: "acme.plugin".into(),
        selection: Some(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.plugin".into(),
            version: "1.0.0".into(),
            enabled: true,
        }),
    };
    sync.stage_v2_product_decision(&Source {
        decision: decision.clone(),
        fail_ack: Cell::new(false),
        ack_attempts: Cell::new(0),
    })
    .unwrap();
    let source = Arc::new(ExactUpload {
        fence: crate::ProductEmissionFence::open(dir.join("product-emission")).unwrap(),
        digest: decision.digest().unwrap(),
        calls: AtomicUsize::new(0),
    });
    let source_check = source.clone();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let entered = Mutex::new(Some(entered_tx));
    let release = Mutex::new(release_rx);
    let worker_sync = sync.clone();
    let worker_session = session.clone();
    let origin = mock.origin.clone();
    let workspace_dir = dir.join("a-sync");
    let task = tokio::spawn(async move {
        crate::v2_product_outbox::run_product_worker(move |cancelled, deadline| async move {
            let epoch = worker_sync
                .capture_v2_config_epoch(&worker_session)
                .ok_or(crate::WorkspaceCoordError::Ineligible)?;
            let _protocol = worker_sync.protocol.try_v2(&origin, "emission-user")?;
            let eligibility = worker_sync
                .v2_product_eligibility(worker_session.clone(), epoch.clone(), cancelled, deadline)
                .with_test_emission_pause(move || {
                    entered.lock().unwrap().take().unwrap().send(()).unwrap();
                    release
                        .lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(4))
                        .unwrap();
                });
            let worker_workspace =
                crate::WorkspaceCoordinator::open(workspace_dir, origin, "emission-user".into())?;
            worker_workspace
                .sync_product_guarded(
                    &worker_session,
                    &eligibility,
                    &crate::workspace::ProductUploadAdmission {
                        source: source.as_ref(),
                        config_generation: epoch.config.generation,
                    },
                )
                .await
        })
        .await
    });
    tokio::time::timeout(Duration::from_secs(3), entered_rx)
        .await
        .unwrap()
        .unwrap();
    let logout_auth = auth.clone();
    let logout = tokio::spawn(async move { logout_auth.logout().await });
    tokio::time::timeout(Duration::from_secs(3), async {
        while auth.try_session_snapshot().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("logout must start while emission permit is held");
    assert!(!logout.is_finished());
    release_tx.send(()).unwrap();
    assert!(matches!(
        task.await.unwrap(),
        Err(crate::WorkspaceCoordError::Ineligible)
    ));
    tokio::time::timeout(Duration::from_secs(3), logout)
        .await
        .expect("failed final check must release session permit")
        .unwrap()
        .unwrap();
    assert_eq!(mock.inner.lock().unwrap().v2_put_requests, 0);
    let key = WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
    let state = workspace.snapshot().unwrap().state;
    let pending = state.entry(&key).unwrap().pending.as_ref().unwrap();
    assert_eq!(pending.status, WorkspacePendingStatus::Unconfirmed);
    assert_eq!(pending.write_epoch.as_ref().unwrap().decimal(), "7");
    assert_eq!(source_check.calls.load(Ordering::SeqCst), 2);
    mock.stop();
}

#[tokio::test(flavor = "current_thread")]
async fn logout_during_epoch_read_does_not_prepare_or_emit() {
    use std::sync::atomic::Ordering;
    let dir = TempDir::new("v2-product-epoch-logout");
    let mock = MockSync::start("epoch-logout-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "epoch-logout-login").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "epoch-logout-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "epoch-logout-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    lease
        .finish(&preview, &mock.origin, "epoch-logout-user")
        .unwrap();
    drop(lease);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    let decision = crate::V2ProductDecision {
        origin: mock.origin.clone(),
        account_id: "epoch-logout-user".into(),
        config_generation: sync.read_config().generation,
        source_generation: 1,
        plugin_id: "acme.plugin".into(),
        selection: Some(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.plugin".into(),
            version: "1.0.0".into(),
            enabled: true,
        }),
    };
    sync.stage_v2_product_decision(&Source {
        decision: decision.clone(),
        fail_ack: Cell::new(false),
        ack_attempts: Cell::new(0),
    })
    .unwrap();
    let upload = Arc::new(ExactUpload {
        fence: crate::ProductEmissionFence::open(dir.join("product-emission")).unwrap(),
        digest: decision.digest().unwrap(),
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    mock.held_v2_epoch.armed.store(true, Ordering::SeqCst);
    let worker = sync.clone();
    let source = upload.clone();
    let task = tokio::spawn(async move { worker.sync_v2_product(session, source).await });
    tokio::time::timeout(
        Duration::from_secs(3),
        mock.held_v2_epoch.started.notified(),
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(3), auth.logout())
        .await
        .unwrap()
        .unwrap();
    mock.held_v2_epoch.release.notify_one();
    assert!(matches!(
        task.await.unwrap(),
        Err(crate::WorkspaceCoordError::Ineligible)
    ));
    let after = workspace.snapshot().unwrap();
    let key = WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
    let pending = after.state.entry(&key).unwrap().pending.as_ref().unwrap();
    assert_eq!(pending.status, WorkspacePendingStatus::Queued);
    assert!(pending.write_epoch.is_none());
    assert_eq!(mock.inner.lock().unwrap().v2_put_requests, 0);
    assert_eq!(upload.calls.load(Ordering::SeqCst), 0);
    mock.stop();
}

#[tokio::test(flavor = "current_thread")]
async fn product_absent_delete_requires_and_uses_server_barrier() {
    let dir = TempDir::new("v2-product-delete-barrier");
    let mock = MockSync::start("delete-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "delete-login").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "delete-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "delete-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    lease.finish(&preview, &mock.origin, "delete-user").unwrap();
    drop(lease);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    let decision = crate::V2ProductDecision {
        origin: mock.origin.clone(),
        account_id: "delete-user".into(),
        config_generation: sync.read_config().generation,
        source_generation: 1,
        plugin_id: "acme.plugin".into(),
        selection: None,
    };
    sync.stage_v2_product_decision(&Source {
        decision: decision.clone(),
        fail_ack: Cell::new(false),
        ack_attempts: Cell::new(0),
    })
    .unwrap();
    let upload = Arc::new(ExactUpload {
        fence: crate::ProductEmissionFence::open(dir.join("product-emission")).unwrap(),
        digest: decision.digest().unwrap(),
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    mock.inner.lock().unwrap().v2_epoch_required = Some(false);
    let before = workspace.snapshot().unwrap();
    assert!(matches!(
        sync.sync_v2_product(session.clone(), upload.clone()).await,
        Err(crate::WorkspaceCoordError::Ineligible)
    ));
    let after = workspace.snapshot().unwrap();
    assert_eq!(after.state, before.state);
    let key = WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
    let pending = after.state.entry(&key).unwrap().pending.as_ref().unwrap();
    assert_eq!(pending.status, WorkspacePendingStatus::Queued);
    assert_eq!(pending.edit, WorkspaceEdit::Delete);
    assert_eq!(mock.inner.lock().unwrap().v2_record_get_requests, 0);
    assert_eq!(mock.inner.lock().unwrap().v2_put_requests, 0);
    assert_eq!(upload.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    mock.inner.lock().unwrap().v2_epoch_required = Some(true);
    let result = sync
        .sync_v2_product(session.clone(), upload.clone())
        .await
        .unwrap();
    assert_eq!(result.absent_deletes, 1);
    assert!(workspace.snapshot().unwrap().state.entry(&key).is_none());
    assert_eq!(mock.inner.lock().unwrap().v2_record_get_requests, 1);
    assert_eq!(mock.inner.lock().unwrap().v2_write_epoch, "8");
    assert_eq!(upload.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    let newer = crate::V2ProductDecision {
        source_generation: 2,
        ..decision
    };
    sync.stage_v2_product_decision(&Source {
        decision: newer.clone(),
        fail_ack: Cell::new(false),
        ack_attempts: Cell::new(0),
    })
    .unwrap();
    mock.inner.lock().unwrap().v2_read_record = Some(serde_json::json!({
        "collection": "plugin-selections", "key": "acme.plugin", "revision": 1,
        "deleted": false, "value": {"schemaVersion": 1, "pluginId": "acme.plugin", "version": "1.0.0", "enabled": true}
    }));
    let newer_upload = Arc::new(ExactUpload {
        fence: crate::ProductEmissionFence::open(dir.join("product-emission")).unwrap(),
        digest: newer.digest().unwrap(),
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    sync.sync_v2_product(session, newer_upload).await.unwrap();
    let after = workspace.snapshot().unwrap();
    assert_eq!(
        after
            .state
            .entry(&key)
            .unwrap()
            .pending
            .as_ref()
            .unwrap()
            .status,
        WorkspacePendingStatus::Conflict
    );
    mock.stop();
}

#[tokio::test(flavor = "current_thread")]
async fn product_dispatch_requires_fresh_server_epoch_enforcement() {
    for (required, status) in [
        (None, None),
        (Some(false), None),
        (Some(true), Some(429)),
        (Some(true), Some(503)),
    ] {
        let dir = TempDir::new("v2-product-epoch-policy");
        let mock = MockSync::start("policy-user").await;
        let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
        login(&auth, "policy-login").await;
        let session = auth.session().await.unwrap();
        let preview = marked_preview(&sync, &session);
        let lease = sync.protocol.try_cutover().unwrap();
        lease
            .prepare(&preview, &mock.origin, "policy-user", &[])
            .unwrap();
        let workspace = crate::WorkspaceCoordinator::open(
            dir.join("a-sync"),
            mock.origin.clone(),
            "policy-user".into(),
        )
        .unwrap();
        workspace.bootstrap_reviewed(&preview).unwrap();
        lease.finish(&preview, &mock.origin, "policy-user").unwrap();
        drop(lease);
        sync.set_enabled(false).unwrap();
        let consent = sync.review_v2_resume(&session, None).unwrap();
        sync.confirm_v2_resume(&consent).unwrap();
        let decision = crate::V2ProductDecision {
            origin: mock.origin.clone(),
            account_id: "policy-user".into(),
            config_generation: sync.read_config().generation,
            source_generation: 1,
            plugin_id: "acme.plugin".into(),
            selection: Some(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "1.0.0".into(),
                enabled: true,
            }),
        };
        sync.stage_v2_product_decision(&Source {
            decision: decision.clone(),
            fail_ack: Cell::new(false),
            ack_attempts: Cell::new(0),
        })
        .unwrap();
        let upload = Arc::new(ExactUpload {
            fence: crate::ProductEmissionFence::open(dir.join("product-emission")).unwrap(),
            digest: decision.digest().unwrap(),
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        {
            let mut inner = mock.inner.lock().unwrap();
            inner.v2_epoch_required = required;
            inner.v2_epoch_status = status;
        }
        let before = workspace.snapshot().unwrap();
        let outcome = sync.sync_v2_product(session, upload.clone()).await;
        match status {
            None => assert!(matches!(
                outcome,
                Err(crate::WorkspaceCoordError::Ineligible)
            )),
            Some(429) => assert!(matches!(
                outcome,
                Err(crate::WorkspaceCoordError::Transport(
                    SyncError::RateLimited { .. }
                ))
            )),
            Some(503) => assert!(matches!(
                outcome,
                Err(crate::WorkspaceCoordError::Transport(SyncError::Http(503)))
            )),
            _ => unreachable!(),
        }
        let after = workspace.snapshot().unwrap();
        let key = WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
        let pending = after.state.entry(&key).unwrap().pending.as_ref().unwrap();
        assert_eq!(pending.status, WorkspacePendingStatus::Queued);
        assert!(pending.write_epoch.is_none());
        assert_eq!(after.state, before.state);
        assert_eq!(upload.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(mock.inner.lock().unwrap().v2_put_requests, 0);
        mock.stop();
    }
}

impl crate::V2ProductUploadSource for BlockingUpload {
    fn acquire_emission_guard(&self) -> Result<Box<dyn Send>, crate::WorkspaceCoordError> {
        Ok(Box::new(()))
    }
    fn with_fenced_upload(
        &self,
        _decision: &crate::V2ProductDecision,
        admit: &mut dyn FnMut() -> Result<(), crate::WorkspaceCoordError>,
    ) -> Result<(), crate::WorkspaceCoordError> {
        if let Some(entered) = self.entered.lock().unwrap().take() {
            let _ = entered.send(());
        }
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(4))
            .unwrap();
        admit()
    }
}

#[tokio::test(flavor = "current_thread")]
async fn product_dispatch_does_not_block_logout_or_send_after_logout() {
    let dir = TempDir::new("v2-product-logout");
    let mock = MockSync::start("logout-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "logout-login").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "logout-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "logout-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    lease.finish(&preview, &mock.origin, "logout-user").unwrap();
    drop(lease);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    let selection = PluginSelection {
        schema_version: 1,
        plugin_id: "acme.plugin".into(),
        version: "1.0.0".into(),
        enabled: true,
    };
    let source = Source {
        decision: crate::V2ProductDecision {
            origin: mock.origin.clone(),
            account_id: "logout-user".into(),
            config_generation: sync.read_config().generation,
            source_generation: 7,
            plugin_id: "acme.plugin".into(),
            selection: Some(selection),
        },
        fail_ack: Cell::new(false),
        ack_attempts: Cell::new(0),
    };
    sync.stage_v2_product_decision(&source).unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let source = Arc::new(BlockingUpload {
        entered: Mutex::new(Some(entered_tx)),
        release: Mutex::new(release_rx),
    });
    let worker = sync.clone();
    let task = tokio::spawn(async move { worker.sync_v2_product(session, source).await });
    tokio::time::timeout(Duration::from_secs(2), entered_rx)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), auth.logout())
        .await
        .expect("source I/O must not block logout")
        .unwrap();
    release_tx.send(()).unwrap();
    assert!(matches!(
        task.await.unwrap(),
        Err(crate::WorkspaceCoordError::Ineligible)
    ));
    assert_eq!(mock.inner.lock().unwrap().v2_put_requests, 0);
    assert!(
        sync.protocol.try_cutover().is_ok(),
        "completed worker must release protocol lease"
    );
    mock.stop();
}

#[tokio::test(flavor = "current_thread")]
async fn logout_during_prepared_workspace_write_prevents_publication() {
    let dir = TempDir::new("v2-product-pull-logout");
    let mock = MockSync::start("pull-logout-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "pull-logout-login").await;
    let session = auth.session().await.unwrap();
    let preview = marked_preview(&sync, &session);
    let cutover = sync.protocol.try_cutover().unwrap();
    cutover
        .prepare(&preview, &mock.origin, "pull-logout-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "pull-logout-user".into(),
    )
    .unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    cutover
        .finish(&preview, &mock.origin, "pull-logout-user")
        .unwrap();
    drop(cutover);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    let epoch = sync.capture_v2_config_epoch(&session).unwrap();
    let _lease = sync
        .protocol
        .try_v2(&mock.origin, "pull-logout-user")
        .unwrap();
    let eligibility = sync.v2_product_eligibility(
        session,
        epoch,
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        std::time::Instant::now() + Duration::from_secs(10),
    );
    let store = cloud_client::WorkspaceStore::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "pull-logout-user",
    )
    .unwrap();
    let before = store.load().unwrap().unwrap();
    let mut candidate = before.state.clone();
    candidate.stage(cloud_client::WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap(), cloud_client::WorkspaceEdit::Put(serde_json::json!({"schemaVersion":1,"pluginId":"acme.plugin","version":"1.0.0","enabled":true}))).unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        store
            .save_with_publication_guard::<crate::WorkspaceCoordError>(
                Some(before.generation),
                &candidate,
                |publish| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(4)).unwrap();
                    eligibility.publish(publish)
                },
            )
            .map(|_| ())
    });
    tokio::time::timeout(Duration::from_secs(2), entered_rx)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), auth.logout())
        .await
        .expect("staging I/O must not block logout")
        .unwrap();
    release_tx.send(()).unwrap();
    assert!(matches!(
        worker.join().unwrap(),
        Err(crate::WorkspaceCoordError::Ineligible)
    ));
    let after = workspace.snapshot().unwrap();
    assert_eq!(after.generation, 1);
    assert!(after
        .state
        .entry(&cloud_client::WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap())
        .is_none());
    mock.stop();
}
