//! Coordinator tests against a mock Cloud speaking the real sync protocol.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore, TokenStore};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{SyncCoordError, SyncCoordinator};

#[derive(Default)]
struct ProductOutboxState {
    entries: BTreeMap<String, crate::V1OutboxCandidate>,
    receipts: BTreeMap<
        String,
        (
            crate::V1LocalEditContext,
            crate::V1OutboxCandidate,
            crate::V1Pending,
        ),
    >,
    source_generations: BTreeMap<String, i64>,
    fail_ack: bool,
    runtime_unconfirmed: bool,
    block_authorization: bool,
    proof_epoch_override: Option<String>,
    rebase: Option<(crate::V1LocalEditContext, crate::V1KeepLocalIntent)>,
    fail_rebase_prepare_after_persist: bool,
    fail_rebase_finish: bool,
    fail_rebase_abandon: bool,
}

#[derive(Clone, Default)]
struct ProductOutbox(Arc<Mutex<ProductOutboxState>>);

impl ProductOutbox {
    fn stage(&self, key: &str, generation: i64, enabled: bool) {
        let candidate = crate::V1OutboxCandidate {
            key: key.into(),
            generation,
            edit: crate::V1Edit::Put(crate::PluginSelection {
                schema_version: 1,
                plugin_id: key.into(),
                version: "1.0.0".into(),
                enabled,
            }),
        };
        let mut state = self.0.lock().unwrap();
        state.source_generations.insert(key.into(), generation);
        state.entries.insert(key.into(), candidate);
    }
}

impl crate::V1ProductOutbox for ProductOutbox {
    fn next(
        &mut self,
        _: &crate::V1LocalEditContext,
    ) -> Result<Option<crate::V1OutboxCandidate>, SyncCoordError> {
        let state = self.0.lock().unwrap();
        if state.runtime_unconfirmed && !state.entries.is_empty() {
            return Err(SyncCoordError::Local(
                "runtime confirmation required".into(),
            ));
        }
        Ok(state.entries.values().next().cloned())
    }

    fn acknowledge(
        &mut self,
        context: &crate::V1LocalEditContext,
        candidate: &crate::V1OutboxCandidate,
        staged: &crate::V1Pending,
    ) -> Result<bool, SyncCoordError> {
        let mut state = self.0.lock().unwrap();
        if state.fail_ack || state.entries.get(&candidate.key) != Some(candidate) {
            return Ok(false);
        }
        state.receipts.insert(
            candidate.key.clone(),
            (context.clone(), candidate.clone(), staged.clone()),
        );
        state.entries.remove(&candidate.key);
        Ok(true)
    }

    fn validate_pending(
        &mut self,
        context: &crate::V1LocalEditContext,
        key: &str,
        pending: &crate::V1Pending,
    ) -> Result<bool, SyncCoordError> {
        let state = self.0.lock().unwrap();
        Ok(!state.block_authorization
            && !state.runtime_unconfirmed
            && state
                .receipts
                .get(key)
                .is_some_and(|(saved_context, receipt, saved_pending)| {
                    saved_context == context
                        && receipt.key == key
                        && receipt.edit == pending.edit
                        && saved_pending == pending
                        && state.source_generations.get(key) == Some(&receipt.generation)
                }))
    }

    fn conflict_source_proof(
        &mut self,
        context: &crate::V1LocalEditContext,
        key: &str,
        pending: &crate::V1Pending,
    ) -> Result<Option<crate::V1ConflictSourceProof>, SyncCoordError> {
        let state = self.0.lock().unwrap();
        if state.runtime_unconfirmed || state.block_authorization || state.entries.contains_key(key)
        {
            return Ok(None);
        }
        Ok(state
            .receipts
            .get(key)
            .and_then(|(saved_context, receipt, saved)| {
                let compatible_conflict = pending.conflict
                    && saved.edit == pending.edit
                    && saved.base_revision == pending.base_revision;
                (saved_context == context
                    && compatible_conflict
                    && state.source_generations.get(key) == Some(&receipt.generation))
                .then(|| {
                    crate::V1ConflictSourceProof::new(
                        receipt.generation,
                        state
                            .proof_epoch_override
                            .clone()
                            .unwrap_or_else(|| "a".repeat(64)),
                    )
                })
                .flatten()
            }))
    }

    fn has_pending_rebase(
        &mut self,
        context: &crate::V1LocalEditContext,
    ) -> Result<bool, SyncCoordError> {
        let state = self.0.lock().unwrap();
        Ok(state.rebase.as_ref().is_some_and(|(saved, _)| {
            saved.origin == context.origin && saved.account_id == context.account_id
        }))
    }

    fn prepare_rebase(
        &mut self,
        context: &crate::V1LocalEditContext,
        intent: &crate::V1KeepLocalIntent,
        proof: &crate::V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        let pending = intent.before().pending.as_ref().unwrap();
        if self
            .conflict_source_proof(context, intent.key(), pending)?
            .as_ref()
            != Some(proof)
        {
            return Ok(false);
        }
        let mut state = self.0.lock().unwrap();
        if state.rebase.is_some() {
            return Ok(false);
        }
        state.rebase = Some((context.clone(), intent.clone()));
        if state.fail_rebase_prepare_after_persist {
            return Err(SyncCoordError::Local(
                "simulated lost prepare outcome".into(),
            ));
        }
        Ok(true)
    }

    fn load_rebase(
        &mut self,
        context: &crate::V1LocalEditContext,
        key: &str,
    ) -> Result<Option<crate::V1KeepLocalIntent>, SyncCoordError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .rebase
            .as_ref()
            .filter(|(saved, intent)| saved == context && intent.key() == key)
            .map(|(_, intent)| intent.clone()))
    }

    fn rebase_source_proof(
        &mut self,
        context: &crate::V1LocalEditContext,
        intent: &crate::V1KeepLocalIntent,
    ) -> Result<Option<crate::V1ConflictSourceProof>, SyncCoordError> {
        let state = self.0.lock().unwrap();
        if state.runtime_unconfirmed
            || state.rebase.as_ref() != Some(&(context.clone(), intent.clone()))
            || state.entries.contains_key(intent.key())
            || state.source_generations.get(intent.key()) != Some(&intent.source_generation())
        {
            return Ok(None);
        }
        let Some((saved_context, candidate, saved)) = state.receipts.get(intent.key()) else {
            return Ok(None);
        };
        let mut original = saved.clone();
        original.conflict = true;
        let matches_before = intent.before().pending.as_ref() == Some(&original);
        let matches_after = intent.after().pending.as_ref() == Some(saved);
        if saved_context != context
            || candidate.generation != intent.source_generation()
            || !(matches_before || matches_after)
        {
            return Ok(None);
        }
        Ok(crate::V1ConflictSourceProof::new(
            candidate.generation,
            state
                .proof_epoch_override
                .clone()
                .unwrap_or_else(|| "a".repeat(64)),
        ))
    }

    fn finish_rebase(
        &mut self,
        context: &crate::V1LocalEditContext,
        intent: &crate::V1KeepLocalIntent,
        proof: &crate::V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        if self.rebase_source_proof(context, intent)?.as_ref() != Some(proof) {
            return Ok(false);
        }
        let mut state = self.0.lock().unwrap();
        if state.fail_rebase_finish {
            return Ok(false);
        }
        if let Some(pending) = intent.after().pending.as_ref() {
            state.receipts.get_mut(intent.key()).unwrap().2 = pending.clone();
        } else {
            state.receipts.remove(intent.key());
        }
        state.rebase = None;
        Ok(true)
    }

    fn newer_rebase_candidate(
        &mut self,
        context: &crate::V1LocalEditContext,
        intent: &crate::V1KeepLocalIntent,
    ) -> Result<Option<(crate::V1OutboxCandidate, crate::V1ConflictSourceProof)>, SyncCoordError>
    {
        let state = self.0.lock().unwrap();
        if state.runtime_unconfirmed
            || state.block_authorization
            || state.rebase.as_ref() != Some(&(context.clone(), intent.clone()))
        {
            return Ok(None);
        }
        let Some(candidate) = state.entries.get(intent.key()) else {
            return Ok(None);
        };
        if candidate.generation <= intent.source_generation()
            || state.source_generations.get(intent.key()) != Some(&candidate.generation)
        {
            return Ok(None);
        }
        let proof = crate::V1ConflictSourceProof::new(
            candidate.generation,
            state
                .proof_epoch_override
                .clone()
                .unwrap_or_else(|| "a".repeat(64)),
        );
        Ok(proof.map(|proof| (candidate.clone(), proof)))
    }

    fn abandon_rebase(
        &mut self,
        context: &crate::V1LocalEditContext,
        intent: &crate::V1KeepLocalIntent,
        candidate: &crate::V1OutboxCandidate,
        proof: &crate::V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        if self.newer_rebase_candidate(context, intent)?.as_ref()
            != Some(&(candidate.clone(), proof.clone()))
        {
            return Ok(false);
        }
        let mut state = self.0.lock().unwrap();
        if state.fail_rebase_abandon {
            return Ok(false);
        }
        state.rebase = None;
        Ok(true)
    }

    fn superseded_rebase_generation(
        &mut self,
        context: &crate::V1LocalEditContext,
        intent: &crate::V1KeepLocalIntent,
    ) -> Result<Option<i64>, SyncCoordError> {
        let state = self.0.lock().unwrap();
        if state.rebase.as_ref() != Some(&(context.clone(), intent.clone())) {
            return Ok(None);
        }
        Ok(state
            .source_generations
            .get(intent.key())
            .copied()
            .filter(|generation| *generation > intent.source_generation()))
    }

    fn abandon_superseded_rebase(
        &mut self,
        context: &crate::V1LocalEditContext,
        intent: &crate::V1KeepLocalIntent,
        generation: i64,
    ) -> Result<bool, SyncCoordError> {
        let mut state = self.0.lock().unwrap();
        if state.fail_rebase_abandon
            || state.rebase.as_ref() != Some(&(context.clone(), intent.clone()))
            || generation <= intent.source_generation()
            || state.source_generations.get(intent.key()) != Some(&generation)
        {
            return Ok(false);
        }
        state.rebase = None;
        Ok(true)
    }
}

async fn prepare_product_conflict(
    sync: &SyncCoordinator,
    mock: &MockSync,
    outbox: &mut ProductOutbox,
) {
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 1, true))
        .unwrap();
    outbox.0.lock().unwrap().block_authorization = true;
    assert!(matches!(
        sync.sync_now_with_outbox(outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    {
        let mut server = mock.inner.lock().unwrap();
        server.cursor = 1;
        server.records.insert("acme.one".into(), Stored {
            revision: 1, deleted: false,
            value: Some(serde_json::json!({"schemaVersion":1,"pluginId":"acme.one","version":"1.0.0","enabled":false})),
        });
    }
    outbox.0.lock().unwrap().block_authorization = false;
    assert_eq!(
        sync.sync_now_with_outbox(outbox).await.unwrap().conflicts,
        vec!["acme.one"]
    );
}

#[tokio::test]
async fn product_conflict_review_is_read_only_and_rejects_source_aba() {
    let dir = TempDir::new("product-conflict-review");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-conflict-review").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let before = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    assert_eq!(review.key(), "acme.one");
    assert_eq!(review.source_generation(), 1);
    assert!(review.pending().conflict);
    assert_eq!(review.cloud().unwrap().revision, 1);
    assert!(!review.cloud().unwrap().value.as_ref().unwrap().enabled);
    assert!(sync
        .product_conflict_review_is_current(&review, &mut outbox)
        .unwrap());
    let other = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    assert!(!other
        .product_conflict_review_is_current(&review, &mut outbox)
        .unwrap());
    let after = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(before.generation, after.generation);
    assert_eq!(before.state, after.state);
    outbox.0.lock().unwrap().proof_epoch_override = Some("b".repeat(64));
    assert!(!sync
        .product_conflict_review_is_current(&review, &mut outbox)
        .unwrap());
    outbox.0.lock().unwrap().proof_epoch_override = None;
    assert!(sync
        .product_conflict_review_is_current(&review, &mut outbox)
        .unwrap());
    // A same-value ABA may reconstruct an identical receipt. Its monotonic
    // source generation still changes, invalidating the earlier review.
    {
        let mut state = outbox.0.lock().unwrap();
        state.source_generations.insert("acme.one".into(), 2);
        state.receipts.get_mut("acme.one").unwrap().1.generation = 2;
    }
    assert!(!sync
        .product_conflict_review_is_current(&review, &mut outbox)
        .unwrap());
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn product_conflict_review_rejects_new_edit_during_cloud_read() {
    let dir = TempDir::new("product-conflict-concurrent");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-conflict-concurrent").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let before = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    mock.held_export
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        let mut outbox = outbox.clone();
        tokio::spawn(async move { sync.review_product_conflict("acme.one", &mut outbox).await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_export.started.notified())
        .await
        .unwrap();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 2, false))
        .unwrap();
    mock.held_export.release.notify_one();
    assert!(matches!(
        running.await.unwrap(),
        Err(SyncCoordError::StaleReview)
    ));
    let after = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(before.generation, after.generation);
    assert_eq!(before.state, after.state);
    assert_eq!(outbox.0.lock().unwrap().entries["acme.one"].generation, 2);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn product_conflict_review_discards_response_after_same_account_relogin() {
    let dir = TempDir::new("product-conflict-relogin");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-conflict-before").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    mock.held_export
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        let mut outbox = outbox.clone();
        tokio::spawn(async move { sync.review_product_conflict("acme.one", &mut outbox).await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_export.started.notified())
        .await
        .unwrap();
    auth.logout().await.unwrap();
    login(&auth, "product-conflict-after").await;
    mock.held_export.release.notify_one();
    assert!(matches!(
        running.await.unwrap(),
        Err(SyncCoordError::StaleReview)
    ));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn product_conflict_review_discards_response_after_opt_out_and_in() {
    let dir = TempDir::new("product-conflict-opt-out");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-conflict-opt-out").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    mock.held_export
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        let mut outbox = outbox.clone();
        tokio::spawn(async move { sync.review_product_conflict("acme.one", &mut outbox).await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_export.started.notified())
        .await
        .unwrap();
    sync.set_enabled(false).unwrap();
    sync.set_enabled(true).unwrap();
    mock.held_export.release.notify_one();
    assert!(matches!(
        running.await.unwrap(),
        Err(SyncCoordError::StaleReview)
    ));
    mock.stop();
}

#[tokio::test]
async fn product_conflict_review_rejects_cloud_rewind_without_mutation() {
    let dir = TempDir::new("product-conflict-rewind");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-conflict-rewind").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let before = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    {
        let mut server = mock.inner.lock().unwrap();
        server.records.clear();
        server.cursor = 0;
    }
    assert!(matches!(
        sync.review_product_conflict("acme.one", &mut outbox).await,
        Err(SyncCoordError::Engine(crate::SyncError::Protocol))
    ));
    let after = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(before.generation, after.generation);
    assert_eq!(before.state, after.state);
    mock.stop();
}

#[tokio::test]
async fn product_conflict_review_requires_source_proof_before_network() {
    let dir = TempDir::new("product-conflict-source");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-conflict-source").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    outbox.0.lock().unwrap().block_authorization = true;
    mock.held_export
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(matches!(
        sync.review_product_conflict("acme.one", &mut outbox).await,
        Err(SyncCoordError::StaleReview)
    ));
    assert!(mock
        .held_export
        .armed
        .load(std::sync::atomic::Ordering::SeqCst));
    mock.stop();
}

#[tokio::test]
async fn product_keep_local_rebases_only_after_review_and_fresh_cloud_read() {
    let dir = TempDir::new("product-keep-local");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-keep-local").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    sync.keep_local_product_conflict(&review, &mut outbox)
        .await
        .unwrap();
    assert!(outbox.0.lock().unwrap().rebase.is_none());
    let snapshot = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    let pending = snapshot.state.entries["acme.one"].pending.as_ref().unwrap();
    assert_eq!(pending.base_revision, Some(1));
    assert!(!pending.conflict);
    assert_eq!(outbox.0.lock().unwrap().receipts["acme.one"].2, *pending);
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    assert_eq!(
        sync.sync_now_with_outbox(&mut outbox).await.unwrap().pushed,
        1
    );
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 2);
    mock.stop();
}

#[tokio::test]
async fn product_keep_local_absent_tombstone_finishes_without_a_write() {
    let dir = TempDir::new("product-keep-absent-delete");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-keep-absent-delete").await;
    let mut outbox = ProductOutbox::default();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 1, false))
        .unwrap();
    outbox
        .0
        .lock()
        .unwrap()
        .entries
        .get_mut("acme.one")
        .unwrap()
        .edit = crate::V1Edit::Delete;
    outbox.0.lock().unwrap().block_authorization = true;
    assert!(matches!(
        sync.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    outbox.0.lock().unwrap().block_authorization = false;
    assert_eq!(
        sync.sync_now_with_outbox(&mut outbox)
            .await
            .unwrap()
            .conflicts,
        vec!["acme.one"]
    );
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    assert!(review.cloud().is_none());
    sync.keep_local_product_conflict(&review, &mut outbox)
        .await
        .unwrap();
    let snapshot = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert!(snapshot.state.entries["acme.one"].pending.is_none());
    assert!(!outbox.0.lock().unwrap().receipts.contains_key("acme.one"));
    assert_eq!(
        sync.sync_now_with_outbox(&mut outbox).await.unwrap().pushed,
        0
    );
    assert!(!mock.inner.lock().unwrap().records.contains_key("acme.one"));
    mock.stop();
}

#[tokio::test]
async fn product_keep_local_rejects_changed_cloud_without_journal_or_mutation() {
    let dir = TempDir::new("product-keep-stale-cloud");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-keep-stale-cloud").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    let before = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    {
        let mut server = mock.inner.lock().unwrap();
        server.cursor = 2;
        server.records.get_mut("acme.one").unwrap().revision = 2;
    }
    assert!(matches!(
        sync.keep_local_product_conflict(&review, &mut outbox).await,
        Err(SyncCoordError::StaleReview)
    ));
    assert!(outbox.0.lock().unwrap().rebase.is_none());
    let after = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(before.generation, after.generation);
    assert_eq!(before.state, after.state);
    mock.stop();
}

#[tokio::test]
async fn product_keep_local_recovers_after_json_save_only_after_new_review() {
    let dir = TempDir::new("product-keep-recover-json");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-keep-recover-json").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_finish = true;
    assert!(matches!(
        sync.keep_local_product_conflict(&review, &mut outbox).await,
        Err(SyncCoordError::StaleReview)
    ));
    assert!(outbox.0.lock().unwrap().rebase.is_some());
    assert!(matches!(
        sync.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    let restarted = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    outbox.0.lock().unwrap().proof_epoch_override = Some("b".repeat(64));
    let recovery = restarted
        .review_keep_local_recovery("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_finish = false;
    restarted
        .resume_keep_local_recovery(&recovery, &mut outbox)
        .await
        .unwrap();
    assert!(outbox.0.lock().unwrap().rebase.is_none());
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    assert_eq!(
        restarted
            .sync_now_with_outbox(&mut outbox)
            .await
            .unwrap()
            .pushed,
        1
    );
    mock.stop();
}

#[tokio::test]
async fn product_keep_local_preserves_a_newer_edit_after_interrupted_rebase() {
    let dir = TempDir::new("product-keep-newer-after-crash");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-keep-newer-after-crash").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_finish = true;
    assert!(matches!(
        sync.keep_local_product_conflict(&review, &mut outbox).await,
        Err(SyncCoordError::StaleReview)
    ));
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 2, false))
        .unwrap();
    let restarted = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    assert!(matches!(
        restarted
            .review_keep_local_recovery("acme.one", &mut outbox)
            .await,
        Err(SyncCoordError::StaleReview)
    ));
    assert!(matches!(
        restarted.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    let state = outbox.0.lock().unwrap();
    assert!(state.rebase.is_some());
    assert_eq!(state.entries["acme.one"].generation, 2);
    drop(state);
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    mock.stop();
}

#[tokio::test]
async fn product_abandon_restores_original_conflict_before_draining_newer_candidate() {
    let dir = TempDir::new("product-abandon-before");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-abandon-before").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let original = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_prepare_after_persist = true;
    assert!(sync
        .keep_local_product_conflict(&review, &mut outbox)
        .await
        .is_err());
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 2, false))
        .unwrap();
    let review = sync
        .review_keep_local_abandon("acme.one", &mut outbox)
        .await
        .unwrap();
    assert_eq!(review.candidate().generation, 2);
    sync.abandon_keep_local_recovery(&review, &mut outbox)
        .unwrap();
    let restored = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(restored.generation, original.generation);
    assert_eq!(restored.state, original.state);
    assert!(outbox.0.lock().unwrap().rebase.is_none());
    let result = sync.sync_now_with_outbox(&mut outbox).await.unwrap();
    assert_eq!(result.pushed, 0);
    assert_eq!(result.conflicts, vec!["acme.one"]);
    let staged = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    let pending = staged.state.entries["acme.one"].pending.as_ref().unwrap();
    assert!(pending.conflict);
    assert_eq!(
        pending.base_revision,
        original.state.entries["acme.one"]
            .pending
            .as_ref()
            .unwrap()
            .base_revision
    );
    assert!(matches!(&pending.edit, crate::V1Edit::Put(selection) if !selection.enabled));
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    mock.stop();
}

#[tokio::test]
async fn product_abandon_recovers_after_restoring_json_and_retains_newer_edit() {
    let dir = TempDir::new("product-abandon-restored");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-abandon-restored").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let original = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_finish = true;
    assert!(sync
        .keep_local_product_conflict(&review, &mut outbox)
        .await
        .is_err());
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 2, false))
        .unwrap();
    let review = sync
        .review_keep_local_abandon("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_abandon = true;
    assert!(matches!(
        sync.abandon_keep_local_recovery(&review, &mut outbox),
        Err(SyncCoordError::StaleReview)
    ));
    let restored = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(restored.generation, original.generation + 2);
    assert_eq!(restored.state, original.state);
    assert!(outbox.0.lock().unwrap().rebase.is_some());
    assert!(matches!(
        sync.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    let restarted = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    assert!(matches!(
        restarted.abandon_keep_local_recovery(&review, &mut outbox),
        Err(SyncCoordError::StaleReview)
    ));
    outbox.0.lock().unwrap().proof_epoch_override = Some("b".repeat(64));
    let fresh = restarted
        .review_keep_local_abandon("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_abandon = false;
    restarted
        .abandon_keep_local_recovery(&fresh, &mut outbox)
        .unwrap();
    assert!(outbox.0.lock().unwrap().rebase.is_none());
    assert_eq!(outbox.0.lock().unwrap().entries["acme.one"].generation, 2);
    let result = restarted.sync_now_with_outbox(&mut outbox).await.unwrap();
    assert_eq!(result.pushed, 0);
    assert_eq!(result.conflicts, vec!["acme.one"]);
    mock.stop();
}

#[tokio::test]
async fn product_abandon_rejects_review_if_candidate_or_consent_changes() {
    let dir = TempDir::new("product-abandon-stale");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-abandon-stale").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_finish = true;
    assert!(sync
        .keep_local_product_conflict(&review, &mut outbox)
        .await
        .is_err());
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 2, false))
        .unwrap();
    let review = sync
        .review_keep_local_abandon("acme.one", &mut outbox)
        .await
        .unwrap();
    let before = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 3, true))
        .unwrap();
    assert!(matches!(
        sync.abandon_keep_local_recovery(&review, &mut outbox),
        Err(SyncCoordError::StaleReview)
    ));
    let after = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(before.generation, after.generation);
    assert_eq!(before.state, after.state);
    let current = sync
        .review_keep_local_abandon("acme.one", &mut outbox)
        .await
        .unwrap();
    sync.set_enabled(false).unwrap();
    assert!(sync
        .abandon_keep_local_recovery(&current, &mut outbox)
        .is_err());
    let state = outbox.0.lock().unwrap();
    assert!(state.rebase.is_some());
    assert_eq!(state.entries["acme.one"].generation, 3);
    drop(state);
    mock.stop();
}

#[tokio::test]
async fn superseded_rebase_local_only_source_restores_conflict_without_runtime_confirmation() {
    let dir = TempDir::new("superseded-rebase-local-only");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "superseded-rebase-local-only").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let original = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    let reviewed = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_prepare_after_persist = true;
    assert!(sync
        .keep_local_product_conflict(&reviewed, &mut outbox)
        .await
        .is_err());
    assert!(matches!(
        sync.review_superseded_rebase("acme.one", &mut outbox).await,
        Err(SyncCoordError::StaleReview)
    ));
    sync.with_v1_local_edit(|_| {
        let mut state = outbox.0.lock().unwrap();
        state.source_generations.insert("acme.one".into(), 2);
        state.runtime_unconfirmed = true;
    })
    .unwrap();
    let review = sync
        .review_superseded_rebase("acme.one", &mut outbox)
        .await
        .unwrap();
    assert_eq!(review.source_generation(), 2);
    assert_eq!(review.intent().account_id(), "user-a");
    sync.abandon_superseded_rebase(&review, &mut outbox)
        .unwrap();
    let restored = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(restored.generation, original.generation);
    assert_eq!(restored.state, original.state);
    assert!(outbox.0.lock().unwrap().rebase.is_none());
    // A stale receipt cannot authorize a new review or an upload, even after
    // native runtime confirmation. The restored conflict remains visible.
    outbox.0.lock().unwrap().runtime_unconfirmed = false;
    assert!(matches!(
        sync.review_product_conflict("acme.one", &mut outbox).await,
        Err(SyncCoordError::StaleReview)
    ));
    let result = sync.sync_now_with_outbox(&mut outbox).await.unwrap();
    assert_eq!(result.pushed, 0);
    assert_eq!(result.conflicts, vec!["acme.one"]);
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    mock.stop();
}

#[tokio::test]
async fn superseded_rebase_keeps_foreign_candidate_and_recovers_after_restoring_json() {
    let dir = TempDir::new("superseded-rebase-foreign-crash");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "superseded-rebase-foreign-crash").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let original = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    let reviewed = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_finish = true;
    assert!(sync
        .keep_local_product_conflict(&reviewed, &mut outbox)
        .await
        .is_err());
    // This represents another account's private candidate. The generic
    // review captures only the global generation, never the candidate body.
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 2, false))
        .unwrap();
    let foreign_candidate = outbox.0.lock().unwrap().entries["acme.one"].clone();
    let receipt = outbox.0.lock().unwrap().receipts["acme.one"].clone();
    let review = sync
        .review_superseded_rebase("acme.one", &mut outbox)
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_abandon = true;
    assert!(matches!(
        sync.abandon_superseded_rebase(&review, &mut outbox),
        Err(SyncCoordError::StaleReview)
    ));
    let restored = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(restored.generation, original.generation + 2);
    assert_eq!(restored.state, original.state);
    {
        let state = outbox.0.lock().unwrap();
        assert!(state.rebase.is_some());
        assert_eq!(state.entries["acme.one"], foreign_candidate);
        assert_eq!(state.receipts["acme.one"], receipt);
    }
    let restarted = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    assert!(matches!(
        restarted.abandon_superseded_rebase(&review, &mut outbox),
        Err(SyncCoordError::StaleReview)
    ));
    let before_change = restarted
        .review_superseded_rebase("acme.one", &mut outbox)
        .await
        .unwrap();
    restarted
        .with_v1_local_edit(|_| {
            outbox
                .0
                .lock()
                .unwrap()
                .source_generations
                .insert("acme.one".into(), 3);
        })
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_abandon = false;
    assert!(matches!(
        restarted.abandon_superseded_rebase(&before_change, &mut outbox),
        Err(SyncCoordError::StaleReview)
    ));
    let new_review = restarted
        .review_superseded_rebase("acme.one", &mut outbox)
        .await
        .unwrap();
    assert_eq!(new_review.source_generation(), 3);
    restarted
        .abandon_superseded_rebase(&new_review, &mut outbox)
        .unwrap();
    let state = outbox.0.lock().unwrap();
    assert!(state.rebase.is_none());
    assert_eq!(state.entries["acme.one"], foreign_candidate);
    assert_eq!(state.receipts["acme.one"], receipt);
    drop(state);
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    mock.stop();
}

#[tokio::test]
async fn product_keep_local_recovers_after_lost_prepare_outcome() {
    let dir = TempDir::new("product-keep-recover-before");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-keep-recover-before").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    let before = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_prepare_after_persist = true;
    assert!(matches!(
        sync.keep_local_product_conflict(&review, &mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    assert!(outbox.0.lock().unwrap().rebase.is_some());
    let after = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(before.generation, after.generation);
    assert_eq!(before.state, after.state);
    let restarted = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    let recovery = restarted
        .review_keep_local_recovery("acme.one", &mut outbox)
        .await
        .unwrap();
    restarted
        .resume_keep_local_recovery(&recovery, &mut outbox)
        .await
        .unwrap();
    assert!(outbox.0.lock().unwrap().rebase.is_none());
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn product_keep_local_rejects_new_source_during_fresh_cloud_read() {
    let dir = TempDir::new("product-keep-new-source");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-keep-new-source").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    mock.held_export
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        let mut outbox = outbox.clone();
        tokio::spawn(async move { sync.keep_local_product_conflict(&review, &mut outbox).await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_export.started.notified())
        .await
        .unwrap();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 2, false))
        .unwrap();
    mock.held_export.release.notify_one();
    assert!(matches!(
        running.await.unwrap(),
        Err(SyncCoordError::StaleReview)
    ));
    let state = outbox.0.lock().unwrap();
    assert!(state.rebase.is_none());
    assert_eq!(state.entries["acme.one"].generation, 2);
    drop(state);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn product_keep_local_journal_blocks_a_concurrent_sync_checkpoint() {
    let dir = TempDir::new("product-keep-concurrent-checkpoint");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-keep-concurrent-checkpoint").await;
    let mut outbox = ProductOutbox::default();
    prepare_product_conflict(&sync, &mock, &mut outbox).await;
    let review = sync
        .review_product_conflict("acme.one", &mut outbox)
        .await
        .unwrap();
    mock.held_pull
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        let mut outbox = outbox.clone();
        tokio::spawn(async move { sync.sync_now_with_outbox(&mut outbox).await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_pull.started.notified())
        .await
        .unwrap();
    outbox.0.lock().unwrap().fail_rebase_finish = true;
    assert!(matches!(
        sync.keep_local_product_conflict(&review, &mut outbox).await,
        Err(SyncCoordError::StaleReview)
    ));
    let saved = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    mock.held_pull.release.notify_one();
    assert!(matches!(
        running.await.unwrap(),
        Err(SyncCoordError::Local(_))
    ));
    let after = sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(saved.generation, after.generation);
    assert_eq!(saved.state, after.state);
    assert!(outbox.0.lock().unwrap().rebase.is_some());
    mock.stop();
}

#[tokio::test]
async fn product_outbox_mode_blocks_legacy_bypasses_across_restart() {
    let dir = TempDir::new("outbox-mode");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "outbox-mode").await;
    assert!(sync
        .record_local_change("acme.one", "1.0.0", false)
        .unwrap());
    let origin = mock.origin.clone();
    let before = sync
        .protocol
        .legacy_snapshot(&origin, "user-a")
        .unwrap()
        .unwrap();
    let gate = sync.protocol.try_cutover().unwrap();
    gate.require_product_outbox(&origin, "user-a").unwrap();
    gate.require_product_outbox(&origin, "user-a").unwrap();
    drop(gate);
    let restarted = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    assert!(matches!(
        restarted.record_local_change("acme.one", "1.0.0", true),
        Err(SyncCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    assert!(matches!(
        restarted.keep_local("acme.one"),
        Err(SyncCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    assert!(matches!(
        restarted.take_remote("acme.one"),
        Err(SyncCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    assert!(matches!(
        restarted.try_reset(),
        Err(SyncCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    let after = restarted
        .protocol
        .legacy_snapshot(&origin, "user-a")
        .unwrap()
        .unwrap();
    assert_eq!(after.generation, before.generation);
    assert_eq!(after.state, before.state);
    assert!(matches!(
        restarted.sync_now().await,
        Err(SyncCoordError::Protocol(
            crate::ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    let other = restarted.protocol.try_v1().unwrap();
    assert!(other.ensure_legacy_allowed(&origin, "user-b").is_ok());
    drop(other);
    let mut outbox = ProductOutbox::default();
    restarted
        .with_v1_local_edit(|_| outbox.stage("acme.one", 1, true))
        .unwrap();
    assert_eq!(
        restarted
            .sync_now_with_outbox(&mut outbox)
            .await
            .unwrap()
            .pushed,
        1
    );
    mock.stop();
}

#[tokio::test]
async fn legacy_reset_still_removes_pending_state_without_product_marker() {
    let dir = TempDir::new("legacy-reset-without-product-marker");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "legacy-reset").await;
    assert!(sync
        .record_local_change("acme.one", "1.0.0", false)
        .unwrap());
    assert!(sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .is_some());
    sync.try_reset().unwrap();
    assert!(sync
        .protocol
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .is_none());
    mock.stop();
}

#[tokio::test]
async fn product_marker_fences_legacy_staging_even_without_valid_enabled_config() {
    let dir = TempDir::new("product-marker-disabled-staging");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "product-marker-disabled").await;
    sync.set_enabled(false).unwrap();
    assert!(!sync
        .record_local_change("acme.one", "1.0.0", false)
        .unwrap());
    sync.set_enabled(true).unwrap();
    let gate = sync.protocol.try_cutover().unwrap();
    gate.require_product_outbox(&mock.origin, "user-a").unwrap();
    drop(gate);
    let rejected = || {
        assert!(matches!(
            sync.record_local_change("acme.one", "1.0.0", true),
            Err(SyncCoordError::Protocol(
                crate::ProtocolFenceError::ProductOutboxRequired
            ))
        ));
        assert!(matches!(
            sync.record_local_removal("acme.one"),
            Err(SyncCoordError::Protocol(
                crate::ProtocolFenceError::ProductOutboxRequired
            ))
        ));
    };
    rejected();
    sync.set_enabled(false).unwrap();
    rejected();
    std::fs::write(&sync.config_path, b"invalid").unwrap();
    rejected();
    std::fs::remove_file(&sync.config_path).unwrap();
    rejected();
    // A truly signed-out context remains local-only; no account's marker is
    // used to authorize or reject edits for an unknown identity.
    auth.logout().await.unwrap();
    assert!(!sync.record_local_change("acme.one", "1.0.0", true).unwrap());
    mock.stop();
}

#[tokio::test]
async fn product_outbox_receipt_rejects_unreviewed_conflict_rebase() {
    let dir = TempDir::new("outbox-receipt-rebase");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "receipt-rebase").await;
    let mut outbox = ProductOutbox::default();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 1, true))
        .unwrap();
    outbox.0.lock().unwrap().block_authorization = true;
    assert!(matches!(
        sync.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    {
        let mut server = mock.inner.lock().unwrap();
        server.cursor = 1;
        server.records.insert("acme.one".into(), Stored {
            revision: 1, deleted: false,
            value: Some(serde_json::json!({"schemaVersion":1,"pluginId":"acme.one","version":"1.0.0","enabled":false})),
        });
    }
    outbox.0.lock().unwrap().block_authorization = false;
    let conflict = sync.sync_now_with_outbox(&mut outbox).await.unwrap();
    assert_eq!(conflict.conflicts, vec!["acme.one"]);
    // The unfenced legacy API changes the base; the old receipt must not
    // authorize dispatch of that newly rebased candidate.
    sync.keep_local("acme.one").unwrap();
    assert!(matches!(
        sync.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    assert_eq!(mock.inner.lock().unwrap().records["acme.one"].revision, 1);
    mock.stop();
}

#[tokio::test]
async fn product_outbox_requires_receipt_for_legacy_pending() {
    let dir = TempDir::new("outbox-legacy-pending");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "legacy-pending").await;
    sync.record_local_change("acme.one", "1.0.0", true).unwrap();
    let mut outbox = ProductOutbox::default();
    assert!(matches!(
        sync.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    assert!(!mock.inner.lock().unwrap().records.contains_key("acme.one"));
    assert_eq!(sync.status().await.pending, 1);
    mock.stop();
}

#[tokio::test]
async fn product_outbox_receipt_rejects_another_accounts_newer_source_after_reopen() {
    let dir = TempDir::new("outbox-receipt-identity");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "receipt-a").await;
    let mut outbox = ProductOutbox::default();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 1, true))
        .unwrap();
    // Emulate interruption after the outbox-to-receipt transaction and
    // before a marker is published. No HTTP write is dispatched.
    outbox.0.lock().unwrap().block_authorization = true;
    assert!(matches!(
        sync.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    assert!(outbox.0.lock().unwrap().entries.is_empty());
    assert!(outbox.0.lock().unwrap().receipts.contains_key("acme.one"));
    assert!(!mock.inner.lock().unwrap().records.contains_key("acme.one"));
    auth.logout().await.unwrap();
    mock.inner.lock().unwrap().account_id = "user-b".into();
    login(&auth, "receipt-b").await;
    sync.with_v1_local_edit(|ctx| {
        assert_eq!(ctx.unwrap().account_id, "user-b");
        // A different identity changes the same global local source.
        outbox
            .0
            .lock()
            .unwrap()
            .source_generations
            .insert("acme.one".into(), 2);
    })
    .unwrap();
    auth.logout().await.unwrap();
    mock.inner.lock().unwrap().account_id = "user-a".into();
    login(&auth, "receipt-a-again").await;
    drop(sync);
    let reopened = SyncCoordinator::new(auth, dir.join("a-sync")).unwrap();
    outbox.0.lock().unwrap().block_authorization = false;
    assert!(matches!(
        reopened.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    assert!(!mock.inner.lock().unwrap().records.contains_key("acme.one"));
    assert_eq!(reopened.status().await.pending, 1);
    mock.stop();
}

#[tokio::test]
async fn product_outbox_unconfirmed_edit_blocks_old_pending_dispatch() {
    let dir = TempDir::new("outbox-unconfirmed");
    let mock = MockSync::start("v1-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "unconfirmed").await;
    sync.record_local_change("acme.one", "1.0.0", true).unwrap();
    let mut outbox = ProductOutbox::default();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 1, false))
        .unwrap();
    outbox.0.lock().unwrap().runtime_unconfirmed = true;
    assert!(matches!(
        sync.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    assert!(!mock.inner.lock().unwrap().records.contains_key("acme.one"));
    assert_eq!(sync.status().await.pending, 1);
    assert_eq!(outbox.0.lock().unwrap().entries.len(), 1);
    outbox.0.lock().unwrap().runtime_unconfirmed = false;
    assert_eq!(
        sync.sync_now_with_outbox(&mut outbox).await.unwrap().pushed,
        1
    );
    assert_eq!(
        mock.inner.lock().unwrap().records["acme.one"]
            .value
            .as_ref()
            .unwrap()["enabled"],
        false
    );
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn product_outbox_edit_before_marker_supersedes_old_staged_edit() {
    let dir = TempDir::new("outbox-before-marker");
    let mock = MockSync::start("v1-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "before-marker").await;
    sync.record_local_change("acme.one", "1.0.0", true).unwrap();
    let outbox = ProductOutbox::default();
    mock.held_pull
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        let mut outbox = outbox.clone();
        tokio::spawn(async move { sync.sync_now_with_outbox(&mut outbox).await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_pull.started.notified())
        .await
        .unwrap();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 1, false))
        .unwrap();
    mock.held_pull.release.notify_one();
    assert_eq!(running.await.unwrap().unwrap().pushed, 1);
    assert_eq!(
        mock.inner.lock().unwrap().records["acme.one"]
            .value
            .as_ref()
            .unwrap()["enabled"],
        false
    );
    assert!(outbox.0.lock().unwrap().entries.is_empty());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn product_outbox_edit_after_marker_survives_for_next_sync() {
    let dir = TempDir::new("outbox-after-marker");
    let mock = MockSync::start("v1-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "after-marker").await;
    let outbox = ProductOutbox::default();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 1, true))
        .unwrap();
    mock.held_write
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        let mut outbox = outbox.clone();
        tokio::spawn(async move { sync.sync_now_with_outbox(&mut outbox).await })
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        mock.held_write.committed.notified(),
    )
    .await
    .unwrap();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 2, false))
        .unwrap();
    mock.held_write.release.notify_one();
    assert_eq!(running.await.unwrap().unwrap().pushed, 1);
    assert_eq!(
        mock.inner.lock().unwrap().records["acme.one"]
            .value
            .as_ref()
            .unwrap()["enabled"],
        true
    );
    assert_eq!(sync.status().await.pending, 1);
    let mut outbox = outbox.clone();
    assert_eq!(
        sync.sync_now_with_outbox(&mut outbox).await.unwrap().pushed,
        1
    );
    assert_eq!(
        mock.inner.lock().unwrap().records["acme.one"]
            .value
            .as_ref()
            .unwrap()["enabled"],
        false
    );
    mock.stop();
}

#[tokio::test]
async fn product_outbox_failed_ack_preserves_candidate_across_reopen() {
    let dir = TempDir::new("outbox-ack-reopen");
    let mock = MockSync::start("v1-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "ack-reopen").await;
    let mut outbox = ProductOutbox::default();
    sync.with_v1_local_edit(|_| outbox.stage("acme.one", 1, true))
        .unwrap();
    outbox.0.lock().unwrap().fail_ack = true;
    assert!(matches!(
        sync.sync_now_with_outbox(&mut outbox).await,
        Err(SyncCoordError::Local(_))
    ));
    assert_eq!(sync.status().await.pending, 1);
    assert!(!mock.inner.lock().unwrap().records.contains_key("acme.one"));
    assert_eq!(outbox.0.lock().unwrap().entries.len(), 1);
    drop(sync);
    let reopened = SyncCoordinator::new(auth, dir.join("a-sync")).unwrap();
    outbox.0.lock().unwrap().fail_ack = false;
    assert_eq!(
        reopened
            .sync_now_with_outbox(&mut outbox)
            .await
            .unwrap()
            .pushed,
        1
    );
    assert_eq!(reopened.status().await.pending, 0);
    assert!(outbox.0.lock().unwrap().entries.is_empty());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_held_pull_does_not_upload_after_same_account_relogin() {
    let dir = TempDir::new("v1-pull-relogin");
    let mock = MockSync::start("v1-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "v1-before").await;
    sync.record_local_change("acme.held", "1.0.0", true)
        .unwrap();
    mock.held_pull
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_pull.started.notified())
        .await
        .unwrap();
    auth.logout().await.unwrap();
    login(&auth, "v1-after").await;
    mock.held_pull.release.notify_one();
    assert!(matches!(
        running.await.unwrap(),
        Err(SyncCoordError::StaleReview)
    ));
    assert!(!mock.inner.lock().unwrap().records.contains_key("acme.held"));
    assert_eq!(sync.status().await.pending, 1);
    let gate = sync.protocol.try_cutover().unwrap();
    assert_eq!(gate.uncertain_uploads(&mock.origin, "v1-user").unwrap(), 0);
    drop(gate);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_logout_after_dispatched_write_preserves_uncertain_marker() {
    let dir = TempDir::new("v1-write-logout");
    let mock = MockSync::start("v1-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "v1-write-before").await;
    sync.record_local_change("acme.held", "1.0.0", true)
        .unwrap();
    mock.held_write
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        mock.held_write.committed.notified(),
    )
    .await
    .unwrap();
    auth.logout().await.unwrap();
    login(&auth, "v1-write-after").await;
    mock.held_write.release.notify_one();
    assert!(matches!(
        running.await.unwrap(),
        Err(SyncCoordError::StaleReview)
    ));
    assert!(mock.inner.lock().unwrap().records.contains_key("acme.held"));
    assert_eq!(sync.status().await.pending, 1);
    let gate = sync.protocol.try_cutover().unwrap();
    let records = gate.uncertain_details(&mock.origin, "v1-user").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].key(), "acme.held");
    drop(gate);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_changed_config_after_pull_prevents_later_dispatch() {
    let dir = TempDir::new("v1-pull-opt-out");
    let mock = MockSync::start("v1-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "v1-optout").await;
    sync.record_local_change("acme.held", "1.0.0", true)
        .unwrap();
    mock.held_pull
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let running = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_pull.started.notified())
        .await
        .unwrap();
    assert!(matches!(
        sync.set_enabled(false),
        Err(SyncCoordError::Protocol(crate::ProtocolFenceError::Busy))
    ));
    // Simulate an external noncooperating config writer; the coordinator
    // must still discard the response and avoid a later write.
    let config = sync.read_config();
    sync.write_config(&crate::SyncConfig {
        enabled: false,
        generation: config.generation + 1,
    })
    .unwrap();
    mock.held_pull.release.notify_one();
    assert!(matches!(
        running.await.unwrap(),
        Err(SyncCoordError::StaleReview)
    ));
    assert!(!mock.inner.lock().unwrap().records.contains_key("acme.held"));
    assert!(!sync.is_enabled());
    mock.stop();
}

#[tokio::test]
async fn v1_local_edit_and_epoch_bind_identity_and_consent() {
    let dir = TempDir::new("v1-edit-identity");
    let mock = MockSync::start("v1-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "v1-epoch-before").await;
    let session = auth.session().await.unwrap();
    let epoch = sync.capture_config_epoch(&session).unwrap();
    let context = sync
        .with_v1_local_edit(|context| {
            assert!(matches!(
                sync.set_enabled(false),
                Err(SyncCoordError::Protocol(crate::ProtocolFenceError::Busy))
            ));
            context.unwrap()
        })
        .unwrap();
    assert_eq!(context.origin, mock.origin);
    assert_eq!(context.account_id, "v1-user");
    assert_eq!(context.config_generation, sync.read_config().generation);
    assert!(auth
        .with_current_session(&session, || sync.is_config_epoch_current(&epoch, &session))
        .unwrap());
    auth.logout().await.unwrap();
    login(&auth, "v1-epoch-after").await;
    let new_session = auth.session().await.unwrap();
    assert!(!auth
        .with_current_session(&new_session, || sync
            .is_config_epoch_current(&epoch, &new_session))
        .unwrap());
    mock.stop();
}

#[tokio::test]
async fn consent_identity_never_reuses_legacy_epoch_after_config_loss_or_corruption() {
    let dir = TempDir::new("v1-consent-identity");
    let mock = MockSync::start("v1-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "consent-identity").await;
    let session = auth.session().await.unwrap();
    // Real pre-migration files remain usable and do not silently rewrite or
    // discard their pending work when read.
    std::fs::write(&sync.config_path, br#"{"enabled":true,"generation":1}"#).unwrap();
    let old = sync.capture_config_epoch(&session).unwrap();
    assert_eq!(sync.read_config().generation, 1);
    std::fs::write(&sync.config_path, b"{corrupt").unwrap();
    assert!(!sync.is_enabled());
    sync.set_enabled(true).unwrap();
    let after_corruption = sync.read_config();
    assert!(after_corruption.generation > u64::MAX.into());
    assert!(!auth
        .with_current_session(&session, || sync.is_config_epoch_current(&old, &session))
        .unwrap());
    let refreshed = sync.capture_config_epoch(&session).unwrap();
    std::fs::remove_file(&sync.config_path).unwrap();
    sync.set_enabled(true).unwrap();
    let after_loss = sync.read_config();
    assert!(after_loss.generation > u64::MAX.into());
    assert_ne!(after_loss.generation, after_corruption.generation);
    assert!(!auth
        .with_current_session(&session, || sync
            .is_config_epoch_current(&refreshed, &session))
        .unwrap());
    mock.stop();
}

struct Stored {
    revision: i64,
    deleted: bool,
    value: Option<serde_json::Value>,
}

struct MockInner {
    account_id: String,
    tokens: HashSet<String>,
    codes: HashSet<String>,
    records: BTreeMap<String, Stored>,
    cursor: i64,
    expire_once: bool,
    fail_sync: bool,
}

struct MockSync {
    origin: String,
    task: tokio::task::JoinHandle<()>,
    held_write: Arc<HeldWrite>,
    held_export: Arc<HeldExport>,
    held_pull: Arc<HeldExport>,
    inner: Arc<Mutex<MockInner>>,
}

#[derive(Default)]
struct HeldExport {
    armed: std::sync::atomic::AtomicBool,
    started: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

#[derive(Default)]
struct HeldWrite {
    armed: std::sync::atomic::AtomicBool,
    skip: std::sync::atomic::AtomicUsize,
    committed: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

impl MockSync {
    async fn start(account_id: &str) -> Self {
        Self::start_seeded(account_id, 0, HashSet::new()).await
    }

    async fn start_on(account_id: &str, port: u16) -> Self {
        Self::start_seeded(account_id, port, HashSet::new()).await
    }

    async fn start_seeded(account_id: &str, port: u16, tokens: HashSet<String>) -> Self {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let inner = Arc::new(Mutex::new(MockInner {
            account_id: account_id.to_owned(),
            tokens,
            codes: HashSet::new(),
            records: BTreeMap::new(),
            cursor: 0,
            expire_once: false,
            fail_sync: false,
        }));
        let held_write = Arc::new(HeldWrite::default());
        let held_export = Arc::new(HeldExport::default());
        let held_pull = Arc::new(HeldExport::default());
        let task = tokio::spawn({
            let inner = inner.clone();
            let held_write = held_write.clone();
            let held_export = held_export.clone();
            let held_pull = held_pull.clone();
            async move {
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        break;
                    };
                    let inner = inner.clone();
                    let held_write = held_write.clone();
                    let held_export = held_export.clone();
                    let held_pull = held_pull.clone();
                    tokio::spawn(async move {
                        let mut buffer = vec![0_u8; 32 * 1024];
                        let Ok(read) = socket.read(&mut buffer).await else {
                            return;
                        };
                        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                        let mut lines = request.lines();
                        let head = lines.next().unwrap_or_default().to_owned();
                        let mut parts = head.split_whitespace();
                        let method = parts.next().unwrap_or_default().to_owned();
                        let target = parts.next().unwrap_or("/").to_owned();
                        let (path, query) = target
                            .split_once('?')
                            .map_or_else(|| (target.as_str(), ""), |(p, q)| (p, q));
                        let headers: HashMap<String, String> = lines
                            .take_while(|line| !line.is_empty())
                            .filter_map(|line| line.split_once(':'))
                            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
                            .collect();
                        let body_start = request.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
                        let body = request[body_start.min(request.len())..].to_owned();
                        let (status, payload) =
                            route(&method, path, query, &headers, &body, &inner);
                        if method == "GET"
                            && path == "/api/v1/native/sync/export"
                            && held_export
                                .armed
                                .swap(false, std::sync::atomic::Ordering::SeqCst)
                        {
                            held_export.started.notify_one();
                            held_export.release.notified().await;
                        }
                        if method == "GET"
                            && path == "/api/v1/native/sync/changes"
                            && held_pull
                                .armed
                                .swap(false, std::sync::atomic::Ordering::SeqCst)
                        {
                            held_pull.started.notify_one();
                            held_pull.release.notified().await;
                        }
                        if method == "PUT"
                            && path
                                .starts_with("/api/v1/native/sync/collections/plugin-selections/")
                            && status.starts_with("200 ")
                            && held_write.armed.load(std::sync::atomic::Ordering::SeqCst)
                        {
                            if held_write.skip.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                                held_write
                                    .skip
                                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                            } else if held_write
                                .armed
                                .swap(false, std::sync::atomic::Ordering::SeqCst)
                            {
                                held_write.committed.notify_one();
                                held_write.release.notified().await;
                            }
                        }
                        let head = format!(
                            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                            payload.len()
                        );
                        let _ = socket.write_all(head.as_bytes()).await;
                        let _ = socket.write_all(payload.as_bytes()).await;
                    });
                }
            }
        });
        Self {
            origin,
            task,
            held_write,
            held_export,
            held_pull,
            inner,
        }
    }

    fn stop(self) {
        self.task.abort();
    }
}

fn json(status: u16, value: serde_json::Value) -> (String, String) {
    (
        format!("{status} {}", reason(status)),
        serde_json::to_string(&value).unwrap(),
    )
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        410 => "Gone",
        500 => "Internal Server Error",
        _ => "Error",
    }
}

fn route(
    method: &str,
    path: &str,
    query: &str,
    headers: &HashMap<String, String>,
    body: &str,
    state: &Arc<Mutex<MockInner>>,
) -> (String, String) {
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default()
        .to_owned();
    let mut state = state.lock().unwrap();
    // Test hooks never need auth.
    if (method, path) == ("POST", "/test/expire-cursor") {
        // One-shot: the next stale-cursor poll gets 410, then the server
        // behaves again, mirroring real retention expiry.
        state.expire_once = true;
        return ("200 OK".to_owned(), String::new());
    }
    if (method, path) == ("GET", "/test/export-tokens") {
        let tokens: Vec<&String> = state.tokens.iter().collect();
        return json(200, serde_json::json!({"tokens": tokens}));
    }
    if (method, path) == ("POST", "/test/fail-sync") {
        state.fail_sync = true;
        return ("200 OK".to_owned(), String::new());
    }
    if (method, path) == ("POST", "/test/recover") {
        state.expire_once = false;
        state.fail_sync = false;
        return ("200 OK".to_owned(), String::new());
    }
    if (method, path) == ("POST", "/api/v1/native/handoff/redeem") {
        let value: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
        let code = value
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or_default();
        if code.is_empty() || !state.codes.insert(code.to_owned()) {
            return json(400, serde_json::json!({"error": "bad or reused code"}));
        }
        if value
            .get("codeVerifier")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .is_empty()
        {
            return json(400, serde_json::json!({"error": "missing verifier"}));
        }
        let token = format!("token-{code}");
        state.tokens.insert(token.clone());
        return json(
            200,
            serde_json::json!({"sessionToken": token, "expiresAt": 9_999_999_999_u64}),
        );
    }
    if !state.tokens.contains(&bearer) {
        return json(401, serde_json::json!({"error": "unauthorized"}));
    }
    if state.fail_sync && path.starts_with("/api/v1/native/sync") {
        return json(500, serde_json::json!({"error": "boom"}));
    }
    match (method, path) {
        ("GET", "/api/v1/native/me") => json(
            200,
            serde_json::json!({"id": state.account_id, "email": "user@example.com"}),
        ),
        ("GET", "/api/v1/native/sync/changes") => {
            let cursor: i64 = query
                .split('&')
                .find_map(|p| p.strip_prefix("cursor="))
                .and_then(|n| n.parse().ok())
                .unwrap_or(-1);
            if cursor < 0 {
                return json(400, serde_json::json!({"error": "bad cursor"}));
            }
            if state.expire_once && cursor < state.cursor {
                state.expire_once = false;
                return ("410 Gone".to_owned(), String::new());
            }
            let mut changes: Vec<(i64, String, Stored)> = state
                .records
                .iter()
                .filter(|(_, r)| r.revision > cursor)
                .map(|(k, r)| {
                    (
                        r.revision,
                        k.clone(),
                        Stored {
                            revision: r.revision,
                            deleted: r.deleted,
                            value: r.value.clone(),
                        },
                    )
                })
                .collect();
            changes.sort_by_key(|(rev, _, _)| *rev);
            let payload: Vec<serde_json::Value> = changes
                .into_iter()
                .map(|(_, key, r)| {
                    serde_json::json!({
                        "collection": "plugin-selections",
                        "key": key,
                        "revision": r.revision,
                        "deleted": r.deleted,
                        "value": r.value,
                    })
                })
                .collect();
            json(
                200,
                // NOTE: the engine's Page struct uses snake_case with no
                // rename_all; only this endpoint speaks snake_case.
                serde_json::json!({"changes": payload, "cursor": state.cursor, "has_more": false}),
            )
        }
        ("GET", "/api/v1/native/sync/head") => {
            json(200, serde_json::json!({"cursor": state.cursor}))
        }
        ("GET", "/api/v1/native/sync/export") => {
            let records: Vec<_> = state
                .records
                .iter()
                .map(|(key, record)| {
                    serde_json::json!({
                        "collection": "plugin-selections", "key": key,
                        "revision": record.revision, "deleted": record.deleted,
                        "value": record.value,
                    })
                })
                .collect();
            json(
                200,
                serde_json::json!({"schemaVersion": 1, "cursor": state.cursor, "records": records}),
            )
        }
        ("GET", "/api/v1/native/sync/collections/plugin-selections") => {
            let after = query
                .split('&')
                .find_map(|p| p.strip_prefix("after="))
                .unwrap_or_default();
            let mut keys: Vec<&String> = state
                .records
                .keys()
                .filter(|k| k.as_str() > after)
                .collect();
            keys.sort();
            let page: Vec<&String> = keys.into_iter().take(2).collect();
            let next_after = if page.len() == 2 && state.records.keys().any(|k| k > page[1]) {
                Some(page[1].clone())
            } else {
                None
            };
            let records: Vec<serde_json::Value> = page
                .iter()
                .map(|key| {
                    let r = &state.records[*key];
                    serde_json::json!({
                        "collection": "plugin-selections",
                        "key": key,
                        "revision": r.revision,
                        "deleted": r.deleted,
                        "value": r.value,
                    })
                })
                .collect();
            json(
                200,
                serde_json::json!({"records": records, "nextAfter": next_after}),
            )
        }
        _ if path.starts_with("/api/v1/native/sync/collections/plugin-selections/") => {
            let key = path
                .strip_prefix("/api/v1/native/sync/collections/plugin-selections/")
                .unwrap_or_default()
                .to_owned();
            if key.is_empty() {
                return json(400, serde_json::json!({"error": "missing key"}));
            }
            let if_match = headers
                .get("if-match")
                .map(|v| v.trim_matches('"').to_owned());
            let if_none = headers.get("if-none-match").cloned();
            let current = state.records.get(&key).map(|r| r.revision);
            let matches = |want: &Option<String>| {
                want.as_ref()
                    .and_then(|w| w.parse::<i64>().ok())
                    .zip(current)
                    .is_some_and(|(w, rev)| w == rev)
            };
            // Preconditions mirror the sync wire contract: If-Match must name
            // the current revision, If-None-Match forbids an existing record.
            if method == "PUT" {
                let conflict = match (&current, &if_match, &if_none) {
                    (Some(_), _, Some(_)) => true,
                    (Some(_), Some(_), _) => !matches(&if_match),
                    (None, Some(_), _) => true,
                    _ => false,
                };
                if conflict {
                    return json(409, serde_json::json!({"error": "revision mismatch"}));
                }
                let value: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
                if value.get("pluginId").and_then(|v| v.as_str()) != Some(key.as_str()) {
                    return json(400, serde_json::json!({"error": "key mismatch"}));
                }
                state.cursor += 1;
                let revision = state.cursor;
                state.records.insert(
                    key.clone(),
                    Stored {
                        revision,
                        deleted: false,
                        value: Some(value),
                    },
                );
                return json(
                    200,
                    serde_json::json!({
                        "collection": "plugin-selections",
                        "key": key,
                        "revision": revision,
                        "deleted": false,
                        "value": state.records[&key].value,
                    }),
                );
            }
            if method == "DELETE" {
                let conflict = match (&current, &if_match) {
                    (Some(_), Some(_)) => !matches(&if_match),
                    (None, Some(_)) => true,
                    _ => false,
                };
                if conflict {
                    return json(409, serde_json::json!({"error": "revision mismatch"}));
                }
                state.cursor += 1;
                let revision = state.cursor;
                state.records.insert(
                    key.clone(),
                    Stored {
                        revision,
                        deleted: true,
                        value: None,
                    },
                );
                return json(
                    200,
                    serde_json::json!({
                        "collection": "plugin-selections",
                        "key": key,
                        "revision": revision,
                        "deleted": true,
                        "value": serde_json::Value::Null,
                    }),
                );
            }
            json(404, serde_json::json!({"error": "not found"}))
        }
        _ => json(404, serde_json::json!({"error": "not found"})),
    }
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tiktools-sync-{name}-{}-{suffix}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Drive the loopback callback while `complete_login` is parked, mirroring
/// the real browser flow.
async fn deliver(browser_url: &str, code: &str) {
    let parsed = url::Url::parse(browser_url).unwrap();
    let redirect = parsed
        .query_pairs()
        .find(|(key, _)| key == "redirectUri")
        .map(|(_, value)| value.to_string())
        .unwrap();
    let expected = parsed
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .unwrap();
    let response = reqwest::get(format!("{redirect}?code={code}&state={expected}"))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

async fn login(auth: &CloudAuth, code: &str) {
    let begun = auth.begin_login().await.unwrap();
    let waiting = auth.clone();
    let waiter = tokio::spawn(async move { waiting.complete_login().await });
    deliver(&begun.browser_url, code).await;
    waiter.await.unwrap().unwrap();
}

fn coordinator(
    dir: &TempDir,
    mock: &MockSync,
    name: &str,
    store: Arc<dyn TokenStore>,
) -> (CloudAuth, SyncCoordinator) {
    let auth = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join(&format!("{name}-account.json")),
        store,
    );
    let sync = SyncCoordinator::new(auth.clone(), dir.join(&format!("{name}-sync"))).unwrap();
    sync.set_enabled(true).unwrap();
    (auth, sync)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_devices_share_selections() {
    let dir = TempDir::new("two-devices");
    let mock = MockSync::start("user-1").await;
    let store_a: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let store_b: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let (auth_a, sync_a) = coordinator(&dir, &mock, "a", store_a);
    let (auth_b, sync_b) = coordinator(&dir, &mock, "b", store_b);
    login(&auth_a, "code-a1").await;
    login(&auth_b, "code-b1").await;
    assert!(sync_a
        .record_local_change("acme.player", "1.2.0", true)
        .unwrap());
    let outcome = sync_a.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);

    let outcome = sync_b.sync_now().await.unwrap();
    let remote: HashMap<_, _> = outcome
        .remote
        .into_iter()
        .map(|r| (r.key, r.selection))
        .collect();
    let selection = remote.get("acme.player").unwrap().as_ref().unwrap();
    assert_eq!(selection.version, "1.2.0");
    assert!(selection.enabled);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offline_edits_survive_and_upload_later() {
    let dir = TempDir::new("offline");
    let port = 18771;
    let mock = MockSync::start_on("user-2", port).await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-off1").await;
    // A restarted server keeps its issued tokens (durable auth store); carry
    // them over so the resume below exercises sync, not re-login.
    let exported: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/test/export-tokens", mock.origin))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tokens: HashSet<String> = exported
        .get("tokens")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    mock.stop();

    // Offline: staging works, syncing fails, nothing is lost.
    assert!(sync
        .record_local_change("acme.offline", "0.9.0", false)
        .unwrap());
    let error = sync.sync_now().await.unwrap_err();
    assert!(matches!(error, SyncCoordError::Engine(_)), "{error:?}");
    assert_eq!(sync.status().await.pending, 1);

    // Reconnect: the pending edit uploads on the next run.
    let mock = MockSync::start_seeded("user-2", port, tokens).await;
    let outcome = sync.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    assert_eq!(sync.status().await.pending, 0);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conflicting_changes_keep_local_on_choice() {
    let dir = TempDir::new("conflict-keep");
    let mock = MockSync::start("user-3").await;
    let (auth_a, sync_a) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth_a, "code-c1").await;
    let auth_b = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join("b-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_b, "code-c2").await;
    let sync_b = SyncCoordinator::new(auth_b.clone(), dir.join("b-sync")).unwrap();
    sync_b.set_enabled(true).unwrap();

    // Both sides edit the same key while apart; A uploads first.
    assert!(sync_a
        .record_local_change("acme.shared", "1.0.0", true)
        .unwrap());
    assert!(sync_b
        .record_local_change("acme.shared", "1.0.0", false)
        .unwrap());
    sync_a.sync_now().await.unwrap();
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.conflicts, vec!["acme.shared".to_owned()]);

    // Merely discovering the conflict must not apply the remote enabled
    // state to a running local plugin before the user makes a choice.
    assert!(outcome.remote.iter().all(|r| r.key != "acme.shared"));
    let unresolved = sync_b.sync_now().await.unwrap();
    assert_eq!(unresolved.conflicts, vec!["acme.shared".to_owned()]);
    assert!(unresolved.remote.iter().all(|r| r.key != "acme.shared"));

    // Keep local: rebase over the remote revision and upload.
    sync_b.keep_local("acme.shared").unwrap();
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    assert!(outcome.conflicts.is_empty());
    let outcome = sync_a.sync_now().await.unwrap();
    let remote: HashMap<_, _> = outcome
        .remote
        .into_iter()
        .map(|r| (r.key, r.selection))
        .collect();
    assert!(!remote.get("acme.shared").unwrap().as_ref().unwrap().enabled);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conflicting_changes_take_remote_on_choice() {
    let dir = TempDir::new("conflict-take");
    let mock = MockSync::start("user-4").await;
    let (auth_a, sync_a) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth_a, "code-d1").await;
    let auth_b = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join("b-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_b, "code-d2").await;
    let sync_b = SyncCoordinator::new(auth_b.clone(), dir.join("b-sync")).unwrap();
    sync_b.set_enabled(true).unwrap();

    assert!(sync_a
        .record_local_change("acme.shared", "2.0.0", true)
        .unwrap());
    assert!(sync_b
        .record_local_change("acme.shared", "2.0.0", false)
        .unwrap());
    sync_a.sync_now().await.unwrap();
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.conflicts, vec!["acme.shared".to_owned()]);

    // Take remote: the local edit is dropped and the server value applies.
    let remote = sync_b.take_remote("acme.shared").unwrap().unwrap();
    assert!(remote.enabled);
    assert_eq!(sync_b.status().await.pending, 0);
    let outcome = sync_b.sync_now().await.unwrap();
    assert!(outcome.conflicts.is_empty());
    assert!(outcome
        .remote
        .iter()
        .any(|r| r.key == "acme.shared" && r.selection.as_ref().is_some_and(|s| s.enabled)));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_cursor_rebuilds_from_snapshot() {
    let dir = TempDir::new("expiry");
    let mock = MockSync::start("user-5").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-e1").await;
    assert!(sync
        .record_local_change("acme.keep", "1.0.0", true)
        .unwrap());
    sync.sync_now().await.unwrap();

    reqwest::Client::new()
        .post(format!("{}/test/expire-cursor", mock.origin))
        .send()
        .await
        .unwrap();
    assert!(sync
        .record_local_change("acme.late", "1.0.0", true)
        .unwrap());
    let outcome = sync.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    let keys: HashSet<_> = outcome.remote.into_iter().map(|r| r.key).collect();
    assert!(keys.contains("acme.keep"));
    assert!(keys.contains("acme.late"));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_propagates_as_tombstone() {
    let dir = TempDir::new("delete");
    let mock = MockSync::start("user-6").await;
    let (auth_a, sync_a) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth_a, "code-f1").await;
    let auth_b = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join("b-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_b, "code-f2").await;
    let sync_b = SyncCoordinator::new(auth_b.clone(), dir.join("b-sync")).unwrap();
    sync_b.set_enabled(true).unwrap();

    assert!(sync_a
        .record_local_change("acme.gone", "1.0.0", true)
        .unwrap());
    sync_a.sync_now().await.unwrap();
    sync_b.sync_now().await.unwrap();

    assert!(sync_a.record_local_removal("acme.gone").unwrap());
    sync_a.sync_now().await.unwrap();
    let outcome = sync_b.sync_now().await.unwrap();
    let tombstoned = outcome
        .remote
        .into_iter()
        .find(|r| r.key == "acme.gone")
        .unwrap();
    assert!(tombstoned.selection.is_none());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_pulls_are_idempotent() {
    let dir = TempDir::new("dupes");
    let mock = MockSync::start("user-7").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-g1").await;
    assert!(sync
        .record_local_change("acme.idem", "3.1.0", true)
        .unwrap());
    let first = sync.sync_now().await.unwrap();
    let second = sync.sync_now().await.unwrap();
    assert_eq!(second.pulled, 0);
    assert_eq!(second.pushed, 0);
    assert_eq!(first.remote, second.remote);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_failure_keeps_pending_and_retries() {
    let dir = TempDir::new("failure");
    let mock = MockSync::start("user-8").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-h1").await;
    assert!(sync
        .record_local_change("acme.retry", "1.0.0", true)
        .unwrap());

    reqwest::Client::new()
        .post(format!("{}/test/fail-sync", mock.origin))
        .send()
        .await
        .unwrap();
    let error = sync.sync_now().await.unwrap_err();
    assert!(matches!(error, SyncCoordError::Engine(_)), "{error:?}");
    assert_eq!(sync.status().await.pending, 1);
    assert!(sync.status().await.last_error.is_some());

    reqwest::Client::new()
        .post(format!("{}/test/recover", mock.origin))
        .send()
        .await
        .unwrap();
    let outcome = sync.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    assert!(sync.status().await.last_error.is_none());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn account_switch_starts_fresh_and_isolates() {
    let dir = TempDir::new("switch");
    let alice = MockSync::start("alice").await;
    let shared = dir.join("shared-sync");
    let auth_a = CloudAuth::new(
        Some(alice.origin.clone()),
        dir.join("a-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_a, "code-i1").await;
    let sync_a = SyncCoordinator::new(auth_a.clone(), &shared).unwrap();
    sync_a.set_enabled(true).unwrap();
    assert!(sync_a
        .record_local_change("acme.alice", "1.0.0", true)
        .unwrap());
    sync_a.sync_now().await.unwrap();
    alice.stop();

    // Same device directory, different account: the stale file is replaced,
    // never merged, and the new account syncs cleanly.
    let bob = MockSync::start("bob").await;
    let auth_b = CloudAuth::new(
        Some(bob.origin.clone()),
        dir.join("b-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth_b, "code-i2").await;
    let sync_b = SyncCoordinator::new(auth_b.clone(), &shared).unwrap();
    assert!(sync_b.is_enabled());
    let outcome = sync_b.sync_now().await.unwrap();
    assert!(outcome.remote.is_empty());
    assert!(sync_b
        .record_local_change("acme.bob", "1.0.0", true)
        .unwrap());
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    bob.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_and_outcomes_carry_no_secrets() {
    let dir = TempDir::new("secrets");
    let mock = MockSync::start("user-9").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-j1").await;
    assert!(sync
        .record_local_change("acme.clean", "1.0.0", true)
        .unwrap());
    let outcome = sync.sync_now().await.unwrap();
    let status = sync.status().await;
    // The coordinator only ever handles selections and status: assert the
    // exact serialized surface carries no credential material.
    let status_text = serde_json::to_value(&status)
        .unwrap()
        .to_string()
        .to_lowercase();
    assert!(!status_text.contains("token"), "{status_text}");
    for remote in &outcome.remote {
        assert!(!remote.key.to_lowercase().contains("token"));
        if let Some(selection) = &remote.selection {
            let text = serde_json::to_value(selection)
                .unwrap()
                .to_string()
                .to_lowercase();
            assert!(!text.contains("token"), "{text}");
        }
    }
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_now_restores_stored_session() {
    let dir = TempDir::new("restore-sync");
    let mock = MockSync::start("user-11").await;
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let sidecar = dir.join("shared-account.json");

    // First "process": log in, then drop every in-memory handle.
    let auth_a = CloudAuth::new(Some(mock.origin.clone()), sidecar.clone(), store.clone());
    login(&auth_a, "code-r1").await;
    drop(auth_a);

    // Second "process": new handles over the same sidecar and keychain.
    // Staging works without a live session; sync reconnects on demand.
    let auth_b = CloudAuth::new(Some(mock.origin.clone()), sidecar, store);
    assert!(auth_b.session().await.is_none());
    let sync_b = SyncCoordinator::new(auth_b.clone(), dir.join("b-sync")).unwrap();
    sync_b.set_enabled(true).unwrap();
    assert!(sync_b
        .record_local_change("acme.restored", "1.0.0", true)
        .unwrap());
    let outcome = sync_b.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    assert!(auth_b.session().await.is_some());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disabled_sync_stages_and_syncs_nothing() {
    let dir = TempDir::new("optout");
    let mock = MockSync::start("user-10").await;
    let auth = CloudAuth::new(
        Some(mock.origin.clone()),
        dir.join("a-account.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    login(&auth, "code-k1").await;
    let sync = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    assert!(!sync.is_enabled());
    assert!(!sync.record_local_change("acme.off", "1.0.0", true).unwrap());
    assert!(matches!(
        sync.sync_now().await.unwrap_err(),
        SyncCoordError::Disabled
    ));
    sync.set_enabled(true).unwrap();
    assert!(sync.record_local_change("acme.off", "1.0.0", true).unwrap());
    sync.set_enabled(false).unwrap();
    assert!(matches!(
        sync.sync_now().await.unwrap_err(),
        SyncCoordError::Disabled
    ));
    // The staged edit waits; re-enabling resumes without data loss.
    sync.set_enabled(true).unwrap();
    let outcome = sync.sync_now().await.unwrap();
    assert_eq!(outcome.pushed, 1);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_sync_releases_the_running_flag() {
    let dir = TempDir::new("cancelled");
    let mock = MockSync::start("user-cancel").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-cancel").await;
    assert!(sync
        .record_local_change("acme.cancel", "1.0.0", true)
        .unwrap());

    // Swap the Cloud for a listener that accepts but never answers, so the
    // run parks on the network while holding the running flag.
    let address = mock.origin.trim_start_matches("http://").to_owned();
    mock.stop();
    // The aborted accept loop releases the port asynchronously.
    let stalled = loop {
        match tokio::net::TcpListener::bind(&address).await {
            Ok(listener) => break listener,
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(10)).await,
        }
    };
    let stall = tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((socket, _)) = stalled.accept().await {
            held.push(socket);
        }
    });

    let mut run = Box::pin(sync.sync_now());
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sync.status().await.syncing {
        assert!(tokio::time::Instant::now() < deadline, "sync never started");
        tokio::select! {
            result = &mut run => panic!("sync finished against a stalled Cloud: {result:?}"),
            () = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
        }
    }
    // Drop the parked run the way the control plane's request timeout does.
    drop(run);

    assert!(!sync.status().await.syncing);
    // A fresh run gets past the guard (and parks again) instead of failing
    // fast with `AlreadyRunning`.
    let retry = tokio::time::timeout(std::time::Duration::from_millis(200), sync.sync_now()).await;
    assert!(retry.is_err(), "retry did not reach the network: {retry:?}");
    stall.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_v1_ack_preserves_newer_stage_and_frozen_original_on_cas_conflict() {
    let dir = TempDir::new("held-v1-ack");
    let mock = MockSync::start("user-held").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-held").await;
    sync.record_local_change("acme.held", "1.0.0", false)
        .unwrap();
    mock.held_write
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let run = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        mock.held_write.committed.notified(),
    )
    .await
    .unwrap();
    sync.record_local_change("acme.held", "2.0.0", true)
        .unwrap();
    mock.held_write.release.notify_one();
    assert!(matches!(
        run.await.unwrap(),
        Err(SyncCoordError::Store(
            cloud_client::StoreError::ConcurrentChange
        ))
    ));
    let saved = cloud_client::StateStore::open(dir.join("a-sync"))
        .unwrap()
        .load()
        .unwrap()
        .unwrap();
    assert!(
        matches!(&saved.state.entries["acme.held"].pending.as_ref().unwrap().edit, cloud_client::Edit::Put(value) if value.version == "2.0.0" && value.enabled)
    );
    let gate = crate::ProtocolFence::open(dir.join("a-sync"))
        .unwrap()
        .try_cutover()
        .unwrap();
    let records = gate.uncertain_details(&mock.origin, "user-held").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].key(), "acme.held");
    assert!(
        matches!(&records[0].pending().edit, cloud_client::Edit::Put(value) if value.version == "1.0.0" && !value.enabled)
    );
    assert!(records[0].base_record().is_none());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_v1_write_leaves_durable_original_and_releases_cutover_lease() {
    let dir = TempDir::new("cancelled-v1-write");
    let mock = MockSync::start("user-write").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-write").await;
    sync.record_local_change("acme.write", "1.0.0", false)
        .unwrap();
    assert_eq!(sync.sync_now().await.unwrap().pushed, 1);
    sync.record_local_change("acme.write", "1.2.3", true)
        .unwrap();
    mock.held_write
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let run = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        mock.held_write.committed.notified(),
    )
    .await
    .unwrap();
    let fence = crate::ProtocolFence::open(dir.join("a-sync")).unwrap();
    assert!(matches!(
        fence.try_cutover(),
        Err(crate::ProtocolFenceError::Busy)
    ));
    assert!(matches!(
        sync.set_enabled(false),
        Err(SyncCoordError::Protocol(crate::ProtocolFenceError::Busy))
    ));
    assert!(matches!(
        sync.keep_local("acme.write"),
        Err(SyncCoordError::Protocol(crate::ProtocolFenceError::Busy))
    ));
    assert!(matches!(
        sync.try_reset(),
        Err(SyncCoordError::Protocol(crate::ProtocolFenceError::Busy))
    ));
    assert!(cloud_client::StateStore::open(dir.join("a-sync"))
        .unwrap()
        .load()
        .unwrap()
        .is_some());
    run.abort();
    assert!(run.await.unwrap_err().is_cancelled());
    let gate = fence.try_cutover().unwrap();
    let records = gate.uncertain_details(&mock.origin, "user-write").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].key(), "acme.write");
    assert!(
        matches!(&records[0].pending().edit, cloud_client::Edit::Put(value) if value.version == "1.2.3")
    );
    let base = records[0].base_record().unwrap();
    assert_eq!(base.revision, 1);
    assert_eq!(base.value.as_ref().unwrap().version, "1.0.0");
    drop(gate);
    assert!(matches!(
        sync.sync_now().await,
        Err(SyncCoordError::Protocol(
            crate::ProtocolFenceError::UncertainV1
        ))
    ));
    sync.set_enabled(false).unwrap();
    mock.held_write.release.notify_one();
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn uncertain_take_cloud_is_fail_closed_until_product_apply_is_durable() {
    let dir = TempDir::new("recovery-cloud-apply-pending");
    let mock = MockSync::start("user-recovery").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-recovery").await;
    sync.record_local_change("acme.recovery", "1.2.3", true)
        .unwrap();
    mock.held_write
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let run = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        mock.held_write.committed.notified(),
    )
    .await
    .unwrap();
    run.abort();
    assert!(run.await.unwrap_err().is_cancelled());
    mock.held_write.release.notify_one();

    let store = cloud_client::StateStore::open(dir.join("a-sync")).unwrap();
    let before = store.load().unwrap().unwrap();
    assert_eq!(
        sync.uncertain_v1_keys().await.unwrap(),
        vec!["acme.recovery"]
    );
    let review = sync.review_uncertain_v1("acme.recovery").await.unwrap();
    assert_eq!(
        review.remote().unwrap().value.as_ref().unwrap().version,
        "1.2.3"
    );
    for choice in [
        crate::V1RecoveryChoice::TakeCloudIfUnchanged,
        crate::V1RecoveryChoice::TakeCloudIncludingNewer,
    ] {
        assert!(matches!(
            sync.resolve_uncertain_v1(&review, choice).await,
            Err(SyncCoordError::Local(_))
        ));
        let after = store.load().unwrap().unwrap();
        assert_eq!(after.generation, before.generation);
        assert_eq!(after.state, before.state);
        assert_eq!(
            sync.uncertain_v1_keys().await.unwrap(),
            vec!["acme.recovery"]
        );
    }
    assert!(matches!(
        sync.sync_now().await,
        Err(SyncCoordError::Protocol(
            crate::ProtocolFenceError::UncertainV1
        ))
    ));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn uncertain_take_absent_cloud_preserves_marker_and_local_selection() {
    let dir = TempDir::new("recovery-absent-cloud");
    let mock = MockSync::start("user-recovery-absent").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-recovery-absent").await;
    sync.record_local_change("acme.recovery", "1.2.3", true)
        .unwrap();
    let store = cloud_client::StateStore::open(dir.join("a-sync")).unwrap();
    let before = store.load().unwrap().unwrap();
    // Simulate interruption after the marker was flushed but before dispatch.
    let lease = sync.protocol.try_v1().unwrap();
    drop(lease.prepare_upload(&before, "acme.recovery").unwrap());
    drop(lease);

    let review = sync.review_uncertain_v1("acme.recovery").await.unwrap();
    assert!(review.remote().is_none());
    for choice in [
        crate::V1RecoveryChoice::TakeCloudIfUnchanged,
        crate::V1RecoveryChoice::TakeCloudIncludingNewer,
    ] {
        assert!(matches!(
            sync.resolve_uncertain_v1(&review, choice).await,
            Err(SyncCoordError::Local(_))
        ));
        let after = store.load().unwrap().unwrap();
        assert_eq!(after.generation, before.generation);
        assert_eq!(after.state, before.state);
        assert_eq!(
            sync.uncertain_v1_keys().await.unwrap(),
            vec!["acme.recovery"]
        );
    }
    assert!(matches!(
        sync.sync_now().await,
        Err(SyncCoordError::Protocol(
            crate::ProtocolFenceError::UncertainV1
        ))
    ));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn uncertain_review_and_resolution_do_not_block_local_staging_during_cloud_reads() {
    let dir = TempDir::new("recovery-held-export");
    let mock = MockSync::start("user-recovery-held").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-recovery-held").await;
    sync.record_local_change("acme.recovery", "1.0.0", false)
        .unwrap();
    mock.held_write
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let run = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        mock.held_write.committed.notified(),
    )
    .await
    .unwrap();
    run.abort();
    assert!(run.await.unwrap_err().is_cancelled());
    mock.held_write.release.notify_one();

    mock.held_export
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let pending_review = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.review_uncertain_v1("acme.recovery").await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_export.started.notified())
        .await
        .unwrap();
    sync.record_local_change("acme.recovery", "2.0.0", true)
        .unwrap();
    mock.held_export.release.notify_one();
    let review = pending_review.await.unwrap().unwrap();
    assert!(
        matches!(&review.local().unwrap().pending.as_ref().unwrap().edit,
        cloud_client::Edit::Put(value) if value.version == "2.0.0" && value.enabled)
    );

    mock.held_export
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let pending_resolution = {
        let sync = sync.clone();
        tokio::spawn(async move {
            sync.resolve_uncertain_v1(&review, crate::V1RecoveryChoice::KeepLocal)
                .await
        })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_export.started.notified())
        .await
        .unwrap();
    sync.record_local_change("acme.recovery", "3.0.0", false)
        .unwrap();
    mock.held_export.release.notify_one();
    assert!(matches!(
        pending_resolution.await.unwrap(),
        Err(SyncCoordError::StaleReview)
    ));
    let saved = cloud_client::StateStore::open(dir.join("a-sync"))
        .unwrap()
        .load()
        .unwrap()
        .unwrap();
    assert!(
        matches!(&saved.state.entries["acme.recovery"].pending.as_ref().unwrap().edit,
        cloud_client::Edit::Put(value) if value.version == "3.0.0" && !value.enabled)
    );
    assert_eq!(
        sync.uncertain_v1_keys().await.unwrap(),
        vec!["acme.recovery"]
    );
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn uncertain_review_discards_cloud_response_after_logout() {
    let dir = TempDir::new("recovery-export-logout");
    let mock = MockSync::start("user-recovery-logout").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-recovery-logout").await;
    sync.record_local_change("acme.recovery", "1.0.0", false)
        .unwrap();
    mock.held_write
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let run = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        mock.held_write.committed.notified(),
    )
    .await
    .unwrap();
    run.abort();
    assert!(run.await.unwrap_err().is_cancelled());
    mock.held_write.release.notify_one();
    mock.held_export
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let pending_review = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.review_uncertain_v1("acme.recovery").await })
    };
    tokio::time::timeout(Duration::from_secs(10), mock.held_export.started.notified())
        .await
        .unwrap();
    auth.logout().await.unwrap();
    mock.held_export.release.notify_one();
    assert!(matches!(
        pending_review.await.unwrap(),
        Err(SyncCoordError::StaleReview)
    ));
    assert!(matches!(
        sync.uncertain_v1_keys().await,
        Err(SyncCoordError::NotSignedIn)
    ));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn completed_first_v1_write_is_saved_before_second_reply_is_lost() {
    let dir = TempDir::new("two-v1-writes");
    let mock = MockSync::start("user-two-writes").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "code-two-writes").await;
    sync.record_local_change("acme.first", "1.0.0", true)
        .unwrap();
    sync.record_local_change("acme.second", "2.0.0", true)
        .unwrap();
    mock.held_write
        .skip
        .store(1, std::sync::atomic::Ordering::SeqCst);
    mock.held_write
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let run = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        mock.held_write.committed.notified(),
    )
    .await
    .unwrap();
    run.abort();
    assert!(run.await.unwrap_err().is_cancelled());
    let state = cloud_client::StateStore::open(dir.join("a-sync"))
        .unwrap()
        .load()
        .unwrap()
        .unwrap()
        .state;
    assert!(state.entries["acme.first"].pending.is_none());
    assert_eq!(
        state.entries["acme.first"]
            .remote
            .as_ref()
            .unwrap()
            .revision,
        1
    );
    assert!(state.entries["acme.second"].pending.is_some());
    let gate = crate::ProtocolFence::open(dir.join("a-sync"))
        .unwrap()
        .try_cutover()
        .unwrap();
    let records = gate
        .uncertain_details(&mock.origin, "user-two-writes")
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].key(), "acme.second");
    assert!(
        matches!(&records[0].pending().edit, cloud_client::Edit::Put(value) if value.version == "2.0.0")
    );
    mock.held_write.release.notify_one();
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switched_account_preserves_uncertain_and_other_queued_legacy_edits() {
    let dir = TempDir::new("v1-account-uncertain");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "account-a-code").await;
    sync.record_local_change("acme.a", "1.0.0", false).unwrap();
    sync.record_local_change("acme.z", "2.0.0", true).unwrap();
    mock.held_write
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let run = {
        let sync = sync.clone();
        tokio::spawn(async move { sync.sync_now().await })
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        mock.held_write.committed.notified(),
    )
    .await
    .unwrap();
    run.abort();
    assert!(run.await.unwrap_err().is_cancelled());
    let root = dir.join("a-sync");
    let original = fs::read(root.join("sync-state.json")).unwrap();
    auth.logout().await.unwrap();
    mock.inner.lock().unwrap().account_id = "user-b".into();
    login(&auth, "account-b-code").await;
    sync.set_enabled(true).unwrap();
    sync.record_local_change("acme.b", "3.0.0", true).unwrap();
    assert_eq!(fs::read(root.join("sync-state.json")).unwrap(), original);
    let fence = crate::ProtocolFence::open(&root).unwrap();
    let state_a = fence
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap()
        .state;
    let state_b = fence
        .legacy_snapshot(&mock.origin, "user-b")
        .unwrap()
        .unwrap()
        .state;
    assert!(state_a.entries["acme.a"].pending.is_some());
    assert!(state_a.entries["acme.z"].pending.is_some());
    assert_eq!(state_b.entries.len(), 1);
    assert!(state_b.entries["acme.b"].pending.is_some());
    let gate = fence.try_cutover().unwrap();
    let unknown = gate.uncertain_details(&mock.origin, "user-a").unwrap();
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].key(), "acme.a");
    assert_eq!(gate.uncertain_uploads(&mock.origin, "user-b").unwrap(), 0);
    mock.held_write.release.notify_one();
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switched_account_can_use_v1_without_touching_completed_v2_accounts_legacy_file() {
    let dir = TempDir::new("v1-account-cutover");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "cutover-a-code").await;
    sync.record_local_change("acme.a", "1.0.0", true).unwrap();
    let root = dir.join("a-sync");
    let original = fs::read(root.join("sync-state.json")).unwrap();
    let fence = crate::ProtocolFence::open(&root).unwrap();
    let legacy = fence.legacy_snapshot(&mock.origin, "user-a").unwrap();
    let preview =
        crate::CutoverPreview::new(mock.origin.clone(), "user-a".into(), &[], legacy.as_ref())
            .unwrap();
    let gate = fence.try_cutover().unwrap();
    gate.prepare(&preview, &mock.origin, "user-a", &[]).unwrap();
    let workspace =
        crate::WorkspaceCoordinator::open(&root, mock.origin.clone(), "user-a".into()).unwrap();
    workspace
        .bootstrap(&preview.selections(), preview.legacy())
        .unwrap();
    gate.finish(&preview, &mock.origin, "user-a").unwrap();
    drop(gate);
    auth.logout().await.unwrap();
    mock.inner.lock().unwrap().account_id = "user-b".into();
    login(&auth, "cutover-b-code").await;
    sync.set_enabled(true).unwrap();
    sync.record_local_change("acme.b", "2.0.0", false).unwrap();
    assert_eq!(fs::read(root.join("sync-state.json")).unwrap(), original);
    let state_b = fence
        .legacy_snapshot(&mock.origin, "user-b")
        .unwrap()
        .unwrap()
        .state;
    assert!(state_b.entries["acme.b"].pending.is_some());
    assert!(fence.try_v2(&mock.origin, "user-a").is_ok());
    assert!(matches!(
        fence.try_v2(&mock.origin, "user-b"),
        Err(crate::ProtocolFenceError::StaleCutover)
    ));
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simultaneous_first_stages_for_distinct_identities_preserve_both() {
    let dir = TempDir::new("concurrent-distinct-identities");
    let a = MockSync::start("user-a").await;
    let b = MockSync::start("user-b").await;
    let auth_a = CloudAuth::new(
        Some(a.origin.clone()),
        dir.join("account-a.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    let auth_b = CloudAuth::new(
        Some(b.origin.clone()),
        dir.join("account-b.json"),
        Arc::new(MemoryTokenStore::new()),
    );
    let root = dir.join("shared-sync");
    let sync_a = SyncCoordinator::new(auth_a.clone(), &root).unwrap();
    let sync_b = SyncCoordinator::new(auth_b.clone(), &root).unwrap();
    sync_a.set_enabled(true).unwrap();
    login(&auth_a, "simultaneous-a").await;
    login(&auth_b, "simultaneous-b").await;
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let left = {
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            sync_a.record_local_change("acme.first", "1.0.0", true)
        })
    };
    let right = {
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            sync_b.record_local_change("acme.second", "2.0.0", false)
        })
    };
    barrier.wait();
    assert!(left.join().unwrap().unwrap());
    assert!(right.join().unwrap().unwrap());
    let fence = crate::ProtocolFence::open(&root).unwrap();
    let first = fence.legacy_snapshot(&a.origin, "user-a").unwrap().unwrap();
    let second = fence.legacy_snapshot(&b.origin, "user-b").unwrap().unwrap();
    assert!(first.state.entries.contains_key("acme.first"));
    assert!(!first.state.entries.contains_key("acme.second"));
    assert!(second.state.entries.contains_key("acme.second"));
    assert!(!second.state.entries.contains_key("acme.first"));
    let original = fs::read(root.join("sync-state.json")).unwrap();
    // Either identity can win the legacy root race. Cut over whichever one
    // landed in a private partition, preserving the root owner's file.
    let root_state = cloud_client::StateStore::open(&root)
        .unwrap()
        .load()
        .unwrap()
        .unwrap()
        .state;
    let (partition_origin, partition_account, root_origin, root_account, partition_snapshot) =
        if root_state.account_id == "user-a" {
            (&b.origin, "user-b", &a.origin, "user-a", &second)
        } else {
            (&a.origin, "user-a", &b.origin, "user-b", &first)
        };
    let preview = crate::CutoverPreview::new(
        partition_origin.clone(),
        partition_account.into(),
        &[],
        Some(partition_snapshot),
    )
    .unwrap();
    let gate = fence.try_cutover().unwrap();
    gate.prepare(&preview, partition_origin, partition_account, &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        &root,
        partition_origin.clone(),
        partition_account.into(),
    )
    .unwrap();
    workspace
        .bootstrap(&preview.selections(), preview.legacy())
        .unwrap();
    gate.finish(&preview, partition_origin, partition_account)
        .unwrap();
    drop(gate);
    assert_eq!(fs::read(root.join("sync-state.json")).unwrap(), original);
    assert_eq!(
        fence
            .legacy_snapshot(partition_origin, partition_account)
            .unwrap()
            .unwrap()
            .state,
        partition_snapshot.state
    );
    assert!(fence.try_v2(partition_origin, partition_account).is_ok());
    let lease = fence.try_v1().unwrap();
    assert!(lease.ensure_v1(root_origin, root_account).is_ok());
    assert!(matches!(
        lease.ensure_v1(partition_origin, partition_account),
        Err(crate::ProtocolFenceError::NotV1)
    ));
    drop(lease);
    a.stop();
    b.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simultaneous_stages_for_same_identity_preserve_both_edits() {
    let dir = TempDir::new("concurrent-same-identity");
    let mock = MockSync::start("user-a").await;
    let (auth, sync_a) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "simultaneous-same").await;
    let sync_b = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let left = {
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            sync_a.record_local_change("acme.first", "1.0.0", true)
        })
    };
    let right = {
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            sync_b.record_local_change("acme.second", "2.0.0", false)
        })
    };
    barrier.wait();
    assert!(left.join().unwrap().unwrap());
    assert!(right.join().unwrap().unwrap());
    let state = crate::ProtocolFence::open(dir.join("a-sync"))
        .unwrap()
        .legacy_snapshot(&mock.origin, "user-a")
        .unwrap()
        .unwrap()
        .state;
    assert_eq!(state.entries.len(), 2);
    assert!(state.entries["acme.first"].pending.is_some());
    assert!(state.entries["acme.second"].pending.is_some());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn toggling_reports_the_live_session() {
    let dir = TempDir::new("toggle-status");
    let mock = MockSync::start("user-toggle").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    assert!(!sync.set_enabled(true).unwrap().signed_in);
    login(&auth, "code-toggle").await;
    let enabled = sync.set_enabled(true).unwrap();
    assert!(enabled.enabled && enabled.signed_in && !enabled.syncing);
    let disabled = sync.set_enabled(false).unwrap();
    assert!(!disabled.enabled && disabled.signed_in);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logout_without_sidecar_does_not_stage_or_expose_previous_identity() {
    let dir = TempDir::new("signed-out-no-staging");
    let mock = MockSync::start("user-a").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "signed-out-code").await;
    sync.record_local_change("acme.keep", "1.0.0", true)
        .unwrap();
    let root = dir.join("a-sync");
    let original = fs::read(root.join("sync-state.json")).unwrap();
    auth.logout().await.unwrap();
    assert!(!sync
        .record_local_change("acme.after", "2.0.0", true)
        .unwrap());
    assert_eq!(sync.status().await.pending, 0);
    assert_eq!(fs::read(root.join("sync-state.json")).unwrap(), original);
    mock.stop();
}
