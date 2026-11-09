//! Crash-boundary integration using the real coordinator and SQLite adapter.
use super::*;
use crate::{db::LocalPluginSyncIntent, paths::AppPaths};
use secrecy::SecretString;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tiktools_cloud_auth::{CloudAuth, Persistence, TokenStore, TokenStoreError};
use tiktools_cloud_sync::SyncCoordinator;
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
        Ok(Some(SecretString::from("fixture-token".to_owned())))
    }
    fn store(&self, _: &str, _: &str, _: &cloud_client::Session) -> Result<(), TokenStoreError> {
        Ok(())
    }
    fn delete(&self, _: &str, _: &str) -> Result<(), TokenStoreError> {
        Ok(())
    }
}

struct FixtureDir(PathBuf);
impl Drop for FixtureDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Inject failure only at the SQLite transaction boundary; all reads,
/// staging and persisted journal operations use the real adapter.
struct FaultOutbox<'a> {
    inner: SqliteProductOutbox<'a>,
    fail_finish: bool,
    fail_abandon: bool,
}
impl V1ProductOutbox for FaultOutbox<'_> {
    fn next(
        &mut self,
        c: &V1LocalEditContext,
    ) -> Result<Option<V1OutboxCandidate>, SyncCoordError> {
        self.inner.next(c)
    }
    fn acknowledge(
        &mut self,
        c: &V1LocalEditContext,
        value: &V1OutboxCandidate,
        pending: &V1Pending,
    ) -> Result<bool, SyncCoordError> {
        self.inner.acknowledge(c, value, pending)
    }
    fn validate_pending(
        &mut self,
        c: &V1LocalEditContext,
        key: &str,
        pending: &V1Pending,
    ) -> Result<bool, SyncCoordError> {
        self.inner.validate_pending(c, key, pending)
    }
    fn conflict_source_proof(
        &mut self,
        c: &V1LocalEditContext,
        key: &str,
        pending: &V1Pending,
    ) -> Result<Option<V1ConflictSourceProof>, SyncCoordError> {
        self.inner.conflict_source_proof(c, key, pending)
    }
    fn has_pending_rebase(&mut self, c: &V1LocalEditContext) -> Result<bool, SyncCoordError> {
        self.inner.has_pending_rebase(c)
    }
    fn prepare_rebase(
        &mut self,
        c: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        proof: &V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        self.inner.prepare_rebase(c, intent, proof)
    }
    fn load_rebase(
        &mut self,
        c: &V1LocalEditContext,
        key: &str,
    ) -> Result<Option<V1KeepLocalIntent>, SyncCoordError> {
        self.inner.load_rebase(c, key)
    }
    fn rebase_source_proof(
        &mut self,
        c: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
    ) -> Result<Option<V1ConflictSourceProof>, SyncCoordError> {
        self.inner.rebase_source_proof(c, intent)
    }
    fn finish_rebase(
        &mut self,
        c: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        proof: &V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        if self.fail_finish {
            Ok(false)
        } else {
            self.inner.finish_rebase(c, intent, proof)
        }
    }
    fn newer_rebase_candidate(
        &mut self,
        c: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
    ) -> Result<Option<(V1OutboxCandidate, V1ConflictSourceProof)>, SyncCoordError> {
        self.inner.newer_rebase_candidate(c, intent)
    }
    fn abandon_rebase(
        &mut self,
        c: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        value: &V1OutboxCandidate,
        proof: &V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        if self.fail_abandon {
            Ok(false)
        } else {
            self.inner.abandon_rebase(c, intent, value, proof)
        }
    }
}

fn reserve(sync: &SyncCoordinator, db: &DatabaseManager, epoch: &str, enabled: bool) {
    sync.with_v1_local_edit(|context| {
        let context = context.unwrap();
        db.set_plugin_state_with_sync_intent(
            "acme.plugin",
            true,
            enabled,
            Some(LocalPluginSyncIntent {
                origin: &context.origin,
                account_id: &context.account_id,
                config_generation: context.config_generation,
                version: Some("1.0.0"),
            }),
        )
        .unwrap();
        let entry = db
            .next_plugin_sync_intent(&context.origin, &context.account_id)
            .unwrap()
            .unwrap();
        assert!(db.confirm_plugin_sync_runtime(&entry, epoch).unwrap());
    })
    .unwrap();
}

#[tokio::test]
async fn sqlite_coordinator_abandon_recovers_crashes_and_retains_newer_conflict() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let writes = Arc::new(AtomicUsize::new(0));
    let server = {
        let writes = writes.clone();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let writes = writes.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let header_end = loop {
                        if let Some(index) = request.windows(4).position(|x| x == b"\r\n\r\n") {
                            break index + 4;
                        }
                        let mut chunk = [0u8; 1024];
                        let Ok(n) = socket.read(&mut chunk).await else {
                            return;
                        };
                        if n == 0 || request.len() > 16384 {
                            return;
                        }
                        request.extend_from_slice(&chunk[..n]);
                    };
                    let headers =
                        String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    while request.len() < header_end + content_length {
                        let mut chunk = [0u8; 1024];
                        let Ok(n) = socket.read(&mut chunk).await else {
                            return;
                        };
                        if n == 0 || request.len() > 16384 {
                            return;
                        }
                        request.extend_from_slice(&chunk[..n]);
                    }
                    let first = headers.lines().next().unwrap_or_default();
                    let record = serde_json::json!({
                        "collection":"plugin-selections", "key":"acme.plugin", "revision":1,
                        "deleted":false, "value":{"schemaVersion":1,"pluginId":"acme.plugin","version":"1.0.0","enabled":false}
                    });
                    let (status, body) = if first.starts_with("get /api/v1/native/me ") {
                        ("200 OK", serde_json::json!({"id":"alice"}))
                    } else if first.starts_with("get /api/v1/native/sync/changes?") {
                        let cursor = first
                            .split("cursor=")
                            .nth(1)
                            .and_then(|v| v.split(['&', ' ']).next())
                            .and_then(|v| v.parse::<i64>().ok())
                            .unwrap_or(0);
                        let revision = i64::from(writes.load(Ordering::SeqCst) > 0);
                        let changes = if cursor < revision {
                            vec![record]
                        } else {
                            vec![]
                        };
                        (
                            "200 OK",
                            serde_json::json!({"changes":changes,"cursor":revision,"has_more":false}),
                        )
                    } else if first.starts_with("get /api/v1/native/sync/export ") {
                        (
                            "200 OK",
                            serde_json::json!({"schemaVersion":1,"cursor":1,"records":[record]}),
                        )
                    } else if first.starts_with(
                        "put /api/v1/native/sync/collections/plugin-selections/acme.plugin ",
                    ) {
                        writes.fetch_add(1, Ordering::SeqCst);
                        ("409 Conflict", serde_json::json!({"error":"conflict"}))
                    } else {
                        ("404 Not Found", serde_json::json!({}))
                    };
                    let body = serde_json::to_vec(&body).unwrap();
                    let head = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&body).await;
                });
            }
        })
    };
    let root = std::env::temp_dir().join(format!(
        "sync-abandon-e2e-{}-{}",
        std::process::id(),
        fastrand::u64(..)
    ));
    let _guard = FixtureDir(root.clone());
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    };
    paths.ensure_directories().unwrap();
    let sidecar = paths.data.join("account.json");
    std::fs::write(
        &sidecar,
        serde_json::to_vec(&serde_json::json!({"origin":origin,"account_id":"alice"})).unwrap(),
    )
    .unwrap();
    let auth = CloudAuth::new(Some(origin.clone()), sidecar, Arc::new(Store));
    auth.restore().await.unwrap();
    let sync = SyncCoordinator::new(auth.clone(), paths.data.join("cloud-sync")).unwrap();
    sync.set_enabled(true).unwrap();
    let db = DatabaseManager::new(paths.clone());
    let epoch = "a".repeat(64);
    reserve(&sync, &db, &epoch, true);
    let mut outbox = FaultOutbox {
        inner: SqliteProductOutbox::new(&db, &epoch),
        fail_finish: false,
        fail_abandon: false,
    };
    let first = sync.sync_now_with_outbox(&mut outbox).await.unwrap();
    assert_eq!(first.conflicts, vec!["acme.plugin"]);
    assert_eq!(writes.load(Ordering::SeqCst), 1);
    let review = sync
        .review_product_conflict("acme.plugin", &mut outbox)
        .await
        .unwrap();
    outbox.fail_finish = true;
    assert!(sync
        .keep_local_product_conflict(&review, &mut outbox)
        .await
        .is_err());
    reserve(&sync, &db, &epoch, false);
    let review = sync
        .review_keep_local_abandon("acme.plugin", &mut outbox)
        .await
        .unwrap();
    outbox.fail_abandon = true;
    assert!(sync
        .abandon_keep_local_recovery(&review, &mut outbox)
        .is_err());
    assert!(sync.sync_now_with_outbox(&mut outbox).await.is_err());
    let restarted = SyncCoordinator::new(auth, paths.data.join("cloud-sync")).unwrap();
    let new_epoch = "b".repeat(64);
    let reopened = DatabaseManager::new(paths);
    let candidate = reopened
        .next_plugin_sync_intent(&origin, "alice")
        .unwrap()
        .unwrap();
    let mut new_outbox = SqliteProductOutbox::new(&reopened, &new_epoch);
    assert!(restarted
        .review_keep_local_abandon("acme.plugin", &mut new_outbox)
        .await
        .is_err());
    assert!(reopened
        .confirm_plugin_sync_runtime(&candidate, &new_epoch)
        .unwrap());
    let review = restarted
        .review_keep_local_abandon("acme.plugin", &mut new_outbox)
        .await
        .unwrap();
    assert_eq!(review.candidate().generation, 2);
    restarted
        .abandon_keep_local_recovery(&review, &mut new_outbox)
        .unwrap();
    let outcome = restarted
        .sync_now_with_outbox(&mut new_outbox)
        .await
        .unwrap();
    assert_eq!(outcome.pushed, 0);
    assert_eq!(outcome.conflicts, vec!["acme.plugin"]);
    assert_eq!(writes.load(Ordering::SeqCst), 1);
    assert!(reopened
        .next_plugin_sync_intent(&origin, "alice")
        .unwrap()
        .is_none());
    let pending = V1Pending {
        base_revision: None,
        conflict: true,
        edit: V1Edit::Put(PluginSelection {
            schema_version: 1,
            plugin_id: "acme.plugin".into(),
            version: "1.0.0".into(),
            enabled: false,
        }),
    };
    let consent = restarted
        .with_v1_local_edit(|context| context.unwrap().config_generation)
        .unwrap();
    assert!(reopened
        .validate_plugin_sync_receipt(
            &origin,
            "alice",
            "acme.plugin",
            consent,
            &new_epoch,
            &pending
        )
        .unwrap());
    server.abort();
}

struct BobStore;
impl TokenStore for BobStore {
    fn persistence(&self) -> Persistence {
        Persistence::MemoryOnly
    }
    fn load(&self, _: &str, _: &str) -> Result<Option<SecretString>, TokenStoreError> {
        Ok(Some(SecretString::from("bob-token".to_owned())))
    }
    fn store(&self, _: &str, _: &str, _: &cloud_client::Session) -> Result<(), TokenStoreError> {
        Ok(())
    }
    fn delete(&self, _: &str, _: &str) -> Result<(), TokenStoreError> {
        Ok(())
    }
}

#[tokio::test]
async fn sqlite_coordinator_abandon_preserves_another_accounts_candidate() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let writes = Arc::new(AtomicUsize::new(0));
    let server = {
        let writes = writes.clone();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let writes = writes.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let header_end = loop {
                        if let Some(index) = request.windows(4).position(|x| x == b"\r\n\r\n") {
                            break index + 4;
                        }
                        let mut chunk = [0; 1024];
                        let Ok(n) = socket.read(&mut chunk).await else {
                            return;
                        };
                        if n == 0 || request.len() > 16384 {
                            return;
                        }
                        request.extend_from_slice(&chunk[..n]);
                    };
                    let headers =
                        String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    while request.len() < header_end + content_length {
                        let mut chunk = [0; 1024];
                        let Ok(n) = socket.read(&mut chunk).await else {
                            return;
                        };
                        if n == 0 || request.len() > 16384 {
                            return;
                        }
                        request.extend_from_slice(&chunk[..n]);
                    }
                    let first = headers.lines().next().unwrap_or_default();
                    let record = serde_json::json!({"collection":"plugin-selections", "key":"acme.plugin",
                        "revision":1, "deleted":false,
                        "value":{"schemaVersion":1,"pluginId":"acme.plugin","version":"1.0.0","enabled":false}});
                    let (status, body) = if first.starts_with("get /api/v1/native/me ") {
                        let id = if headers.contains("authorization: bearer bob-token") {
                            "bob"
                        } else {
                            "alice"
                        };
                        ("200 OK", serde_json::json!({"id":id}))
                    } else if first.starts_with("get /api/v1/native/sync/changes?") {
                        let cursor = first
                            .split("cursor=")
                            .nth(1)
                            .and_then(|v| v.split(['&', ' ']).next())
                            .and_then(|v| v.parse::<i64>().ok())
                            .unwrap_or(0);
                        let revision = i64::from(writes.load(Ordering::SeqCst) > 0);
                        let changes = if cursor < revision {
                            vec![record]
                        } else {
                            vec![]
                        };
                        (
                            "200 OK",
                            serde_json::json!({"changes":changes,"cursor":revision,"has_more":false}),
                        )
                    } else if first.starts_with("get /api/v1/native/sync/export ") {
                        (
                            "200 OK",
                            serde_json::json!({"schemaVersion":1,"cursor":1,"records":[record]}),
                        )
                    } else if first.starts_with(
                        "put /api/v1/native/sync/collections/plugin-selections/acme.plugin ",
                    ) {
                        writes.fetch_add(1, Ordering::SeqCst);
                        ("409 Conflict", serde_json::json!({"error":"conflict"}))
                    } else {
                        ("404 Not Found", serde_json::json!({}))
                    };
                    let body = serde_json::to_vec(&body).unwrap();
                    let head = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&body).await;
                });
            }
        })
    };
    let root = std::env::temp_dir().join(format!(
        "sync-abandon-foreign-{}-{}",
        std::process::id(),
        fastrand::u64(..)
    ));
    let _guard = FixtureDir(root.clone());
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    };
    paths.ensure_directories().unwrap();
    let alice_sidecar = paths.data.join("alice.json");
    let bob_sidecar = paths.data.join("bob.json");
    std::fs::write(
        &alice_sidecar,
        serde_json::to_vec(&serde_json::json!({"origin":origin,"account_id":"alice"})).unwrap(),
    )
    .unwrap();
    std::fs::write(
        &bob_sidecar,
        serde_json::to_vec(&serde_json::json!({"origin":origin,"account_id":"bob"})).unwrap(),
    )
    .unwrap();
    let alice_auth = CloudAuth::new(Some(origin.clone()), alice_sidecar, Arc::new(Store));
    let bob_auth = CloudAuth::new(Some(origin.clone()), bob_sidecar, Arc::new(BobStore));
    alice_auth.restore().await.unwrap();
    bob_auth.restore().await.unwrap();
    let alice = SyncCoordinator::new(alice_auth, paths.data.join("cloud-sync")).unwrap();
    alice.set_enabled(true).unwrap();
    let bob = SyncCoordinator::new(bob_auth, paths.data.join("cloud-sync")).unwrap();
    let db = DatabaseManager::new(paths);
    let epoch = "a".repeat(64);
    reserve(&alice, &db, &epoch, true);
    let mut fault = FaultOutbox {
        inner: SqliteProductOutbox::new(&db, &epoch),
        fail_finish: false,
        fail_abandon: false,
    };
    assert_eq!(
        alice
            .sync_now_with_outbox(&mut fault)
            .await
            .unwrap()
            .conflicts,
        vec!["acme.plugin"]
    );
    let review = alice
        .review_product_conflict("acme.plugin", &mut fault)
        .await
        .unwrap();
    fault.fail_finish = true;
    assert!(alice
        .keep_local_product_conflict(&review, &mut fault)
        .await
        .is_err());

    // The second authenticated account changes the global local source and
    // leaves its private candidate unconfirmed in its own partition.
    bob.with_v1_local_edit(|context| {
        let context = context.unwrap();
        assert_eq!(context.account_id, "bob");
        assert_eq!(
            db.set_plugin_state_with_sync_intent(
                "acme.plugin",
                true,
                false,
                Some(LocalPluginSyncIntent {
                    origin: &context.origin,
                    account_id: &context.account_id,
                    config_generation: context.config_generation,
                    version: Some("9.1.0")
                })
            )
            .unwrap(),
            2
        );
    })
    .unwrap();
    let foreign = db.next_plugin_sync_intent(&origin, "bob").unwrap().unwrap();
    assert!(foreign.runtime_epoch.is_none());
    assert!(db
        .next_plugin_sync_intent(&origin, "alice")
        .unwrap()
        .is_none());

    let mut adapter = SqliteProductOutbox::new(&db, &epoch);
    assert!(alice
        .review_keep_local_abandon("acme.plugin", &mut adapter)
        .await
        .is_err());
    let review = alice
        .review_superseded_rebase("acme.plugin", &mut adapter)
        .await
        .unwrap();
    assert_eq!(review.source_generation(), 2);
    assert_eq!(review.intent().account_id(), "alice");
    alice
        .abandon_superseded_rebase(&review, &mut adapter)
        .unwrap();
    assert_eq!(
        db.next_plugin_sync_intent(&origin, "bob").unwrap().unwrap(),
        foreign
    );
    assert!(db
        .next_plugin_sync_intent(&origin, "alice")
        .unwrap()
        .is_none());
    assert!(alice
        .review_product_conflict("acme.plugin", &mut adapter)
        .await
        .is_err());
    let outcome = alice.sync_now_with_outbox(&mut adapter).await.unwrap();
    assert_eq!(outcome.pushed, 0);
    assert_eq!(outcome.conflicts, vec!["acme.plugin"]);
    assert_eq!(writes.load(Ordering::SeqCst), 1);
    assert_eq!(
        db.next_plugin_sync_intent(&origin, "bob").unwrap().unwrap(),
        foreign
    );
    server.abort();
}
