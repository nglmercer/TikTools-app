use super::*;
use std::cell::Cell;

#[tokio::test]
async fn pulled_counts_the_final_page_and_does_not_issue_a_duplicate_probe() {
    let dir = TempDir::new("pull-record-count");
    let mock = MockSync::start("alice").await;
    let (source_auth, source) = coordinator(
        &dir,
        &mock,
        "count-source",
        Arc::new(MemoryTokenStore::new()),
    );
    let (target_auth, target) = coordinator(
        &dir,
        &mock,
        "count-target",
        Arc::new(MemoryTokenStore::new()),
    );
    login(&source_auth, "count-source").await;
    login(&target_auth, "count-target").await;
    assert!(source
        .record_local_change("acme.first", "1.0.0", true)
        .unwrap());
    assert!(source
        .record_local_change("acme.second", "1.0.0", true)
        .unwrap());
    assert_eq!(source.sync_now().await.unwrap().pushed, 2);
    let before = mock.inner.lock().unwrap().pull_requests;
    assert_eq!(target.sync_now().await.unwrap().pulled, 2);
    assert_eq!(mock.inner.lock().unwrap().pull_requests - before, 1);
    let before = mock.inner.lock().unwrap().pull_requests;
    assert_eq!(target.sync_now().await.unwrap().pulled, 0);
    assert_eq!(mock.inner.lock().unwrap().pull_requests - before, 1);
    assert!(source
        .record_local_change("acme.first", "1.0.0", false)
        .unwrap());
    assert_eq!(source.sync_now().await.unwrap().pushed, 1);
    let before = mock.inner.lock().unwrap().pull_requests;
    assert_eq!(target.sync_now().await.unwrap().pulled, 1);
    assert_eq!(mock.inner.lock().unwrap().pull_requests - before, 1);
    mock.stop();
}

#[tokio::test]
async fn oversized_backlog_checkpoints_before_upload_and_resumes_after_restart() {
    let dir = TempDir::new("pull-bounded-checkpoint");
    let mock = MockSync::start("alice").await;
    let (auth, sync) = coordinator(&dir, &mock, "bounded", Arc::new(MemoryTokenStore::new()));
    login(&auth, "bounded").await;
    {
        let mut state = mock.inner.lock().unwrap();
        state.page_size = Some(1);
        for revision in 1..=201 {
            let key = format!("acme.remote{revision:03}");
            state.records.insert(
                key.clone(),
                Stored {
                    revision,
                    deleted: false,
                    value: Some(serde_json::json!({
                        "schemaVersion": 1, "pluginId": key,
                        "version": "1.0.0", "enabled": true
                    })),
                },
            );
        }
        state.cursor = 201;
    }
    assert!(sync
        .record_local_change("acme.local", "1.0.0", true)
        .unwrap());
    assert!(matches!(
        sync.sync_now().await,
        Err(SyncCoordError::Engine(crate::SyncError::TooManyPages))
    ));
    assert!(!mock
        .inner
        .lock()
        .unwrap()
        .records
        .contains_key("acme.local"));
    assert_eq!(sync.status().await.pending, 1);
    let restarted = SyncCoordinator::new(auth, dir.join("bounded-sync")).unwrap();
    let outcome = restarted.sync_now().await.unwrap();
    assert_eq!(outcome.pulled, 1);
    assert_eq!(outcome.pushed, 1);
    assert!(mock
        .inner
        .lock()
        .unwrap()
        .records
        .contains_key("acme.local"));
    mock.stop();
}

async fn seeded() -> (TempDir, MockSync, CloudAuth, SyncCoordinator, ProductOutbox) {
    let dir = TempDir::new("product-remote-apply");
    let mock = MockSync::start("alice").await;
    let (source_auth, source) =
        coordinator(&dir, &mock, "source", Arc::new(MemoryTokenStore::new()));
    let (auth, sync) = coordinator(&dir, &mock, "target", Arc::new(MemoryTokenStore::new()));
    login(&source_auth, "remote-source").await;
    login(&auth, "remote-target").await;
    assert!(source
        .record_local_change("acme.player", "1.2.0", true)
        .unwrap());
    assert_eq!(source.sync_now().await.unwrap().pushed, 1);
    let mut outbox = ProductOutbox::default();
    assert!(!sync
        .sync_now_with_outbox(&mut outbox)
        .await
        .unwrap()
        .remote
        .is_empty());
    (dir, mock, auth, sync, outbox)
}

#[tokio::test]
async fn pulled_record_is_admitted_only_under_session_and_protocol_gates() {
    let (_dir, mock, _auth, sync, mut outbox) = seeded().await;
    let review = sync
        .review_product_remote_apply("acme.player", &mut outbox)
        .await
        .unwrap();
    let mut seen = None;
    sync.with_reviewed_product_remote(&review, &mut outbox, |context, record| {
        assert_eq!(context.account_id, "alice");
        assert_eq!(context.origin, mock.origin);
        assert_eq!(record.key, "acme.player");
        assert_eq!(record.value.as_ref().unwrap().version, "1.2.0");
        assert!(matches!(
            sync.protocol.try_cutover(),
            Err(crate::ProtocolFenceError::Busy)
        ));
        let lease = sync.protocol.try_v1().unwrap();
        assert!(matches!(
            lease.try_admission(),
            Err(crate::ProtocolFenceError::Busy)
        ));
        seen = Some(record.clone());
        Ok(())
    })
    .unwrap();
    assert_eq!(seen.as_ref(), Some(review.record()));
    mock.stop();
}

#[tokio::test]
async fn newer_pending_or_snapshot_invalidates_the_entire_review() {
    let (_dir, mock, _auth, sync, mut outbox) = seeded().await;
    let review = sync
        .review_product_remote_apply("acme.player", &mut outbox)
        .await
        .unwrap();
    assert!(sync
        .record_local_change("acme.player", "1.2.0", false)
        .unwrap());
    let called = Cell::new(false);
    assert!(matches!(
        sync.with_reviewed_product_remote(&review, &mut outbox, |_, _| {
            called.set(true);
            Ok(())
        }),
        Err(SyncCoordError::StaleReview)
    ));
    assert!(!called.get());
    assert!(matches!(
        sync.review_product_remote_apply("acme.player", &mut outbox)
            .await,
        Err(SyncCoordError::StaleReview)
    ));
    // The other changeset is not a source proof for this review; even an
    // unrelated newly persisted snapshot invalidates a previously captured
    // state so no caller can use an old batch after a new run.
    assert_eq!(sync.sync_now().await.unwrap().pushed, 1);
    let fresh = sync
        .review_product_remote_apply("acme.player", &mut outbox)
        .await
        .unwrap();
    assert!(!fresh.record().value.as_ref().unwrap().enabled);
    assert!(matches!(
        sync.with_reviewed_product_remote(&review, &mut outbox, |_, _| Ok(())),
        Err(SyncCoordError::StaleReview)
    ));
    mock.stop();
}

#[tokio::test]
async fn opt_out_and_same_account_relogin_revoke_the_review() {
    let (_dir, mock, auth, sync, mut outbox) = seeded().await;
    let review = sync
        .review_product_remote_apply("acme.player", &mut outbox)
        .await
        .unwrap();
    sync.set_enabled(false).unwrap();
    sync.set_enabled(true).unwrap();
    let called = Cell::new(false);
    assert!(matches!(
        sync.with_reviewed_product_remote(&review, &mut outbox, |_, _| {
            called.set(true);
            Ok(())
        }),
        Err(SyncCoordError::StaleReview)
    ));
    assert!(!called.get());
    let fresh = sync
        .review_product_remote_apply("acme.player", &mut outbox)
        .await
        .unwrap();
    auth.logout().await.unwrap();
    login(&auth, "remote-target-relogin").await;
    assert!(matches!(
        sync.with_reviewed_product_remote(&fresh, &mut outbox, |_, _| {
            called.set(true);
            Ok(())
        }),
        Err(SyncCoordError::StaleReview)
    ));
    assert!(!called.get());
    mock.stop();
}

struct UnavailableJournal;
impl crate::V1ProductOutbox for UnavailableJournal {
    fn next(
        &mut self,
        _: &crate::V1LocalEditContext,
    ) -> Result<Option<crate::V1OutboxCandidate>, SyncCoordError> {
        Ok(None)
    }
    fn acknowledge(
        &mut self,
        _: &crate::V1LocalEditContext,
        _: &crate::V1OutboxCandidate,
        _: &crate::V1Pending,
    ) -> Result<bool, SyncCoordError> {
        Ok(false)
    }
    fn validate_pending(
        &mut self,
        _: &crate::V1LocalEditContext,
        _: &str,
        _: &crate::V1Pending,
    ) -> Result<bool, SyncCoordError> {
        Ok(false)
    }
    fn has_pending_rebase(
        &mut self,
        _: &crate::V1LocalEditContext,
    ) -> Result<bool, SyncCoordError> {
        Err(SyncCoordError::Local("journal unavailable".into()))
    }
}

#[tokio::test]
async fn unresolved_or_unreadable_journal_blocks_capture_and_admission() {
    let (_dir, mock, _auth, sync, mut outbox) = seeded().await;
    let review = sync
        .review_product_remote_apply("acme.player", &mut outbox)
        .await
        .unwrap();
    let mut unavailable = UnavailableJournal;
    assert!(sync
        .review_product_remote_apply("acme.player", &mut unavailable)
        .await
        .is_err());
    let called = Cell::new(false);
    assert!(sync
        .with_reviewed_product_remote(&review, &mut unavailable, |_, _| {
            called.set(true);
            Ok(())
        })
        .is_err());
    assert!(!called.get());
    mock.stop();
}
