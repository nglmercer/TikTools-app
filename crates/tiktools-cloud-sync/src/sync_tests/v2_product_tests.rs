use super::*;
use std::cell::Cell;

struct Source {
    decision: crate::V2ProductDecision,
    fail_ack: Cell<bool>,
    ack_attempts: Cell<usize>,
}
impl crate::V2ProductSource for Source {
    fn with_fenced_decision<T>(
        &self,
        stage: impl FnOnce(&crate::V2ProductDecision) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError> {
        let value = stage(&self.decision)?;
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
    let preview = sync.preview_v2_cutover(&session, &[]).unwrap();
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
    workspace
        .bootstrap(&preview.selections(), preview.legacy())
        .unwrap();
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
