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
    let preview = sync.preview_v2_cutover(&session, &[]).unwrap();
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
    workspace
        .bootstrap(&preview.selections(), preview.legacy())
        .unwrap();
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
    for status in [200, 409] {
        for transition in [
            "logout",
            "replace",
            "unchanged",
            "cancel",
            "cancel_before_commit",
        ] {
            let dir = TempDir::new("v2-product-http-auth");
            let mock = MockSync::start("v2-http-user").await;
            let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
            login(&auth, "first-session").await;
            let session = auth.session().await.unwrap();
            let preview = sync.preview_v2_cutover(&session, &[]).unwrap();
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
            workspace
                .bootstrap(&preview.selections(), preview.legacy())
                .unwrap();
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
            assert_eq!(upload.calls.load(Ordering::SeqCst), 1);
            assert_eq!(
                mock.inner.lock().unwrap().v2_put_requests,
                usize::from(transition != "cancel_before_commit")
            );
            assert!(matches!(
                upload.fence.try_untracked_edit(),
                Err(crate::ProtocolFenceError::Busy)
            ));
            if transition == "cancel" || transition == "cancel_before_commit" {
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
                } else {
                    assert_eq!(mock.inner.lock().unwrap().v2_put_requests, 1);
                    mock.held_v2_write.release.notify_one();
                }
                continue;
            }
            if transition != "unchanged" {
                tokio::time::timeout(Duration::from_secs(2), auth.logout())
                    .await
                    .expect("in-flight HTTP must not block logout")
                    .unwrap();
                if transition == "replace" {
                    login(&auth, "replacement-session").await;
                    assert!(!Arc::ptr_eq(&session, &auth.session().await.unwrap()));
                }
            }
            mock.held_v2_write.release.notify_one();
            let outcome = tokio::time::timeout(Duration::from_secs(3), task)
                .await
                .expect("dispatcher must finish after response")
                .unwrap();
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
    let preview = sync.preview_v2_cutover(&session, &[]).unwrap();
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
    workspace
        .bootstrap(&preview.selections(), preview.legacy())
        .unwrap();
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
    let preview = sync.preview_v2_cutover(&session, &[]).unwrap();
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
    workspace
        .bootstrap(&preview.selections(), preview.legacy())
        .unwrap();
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
