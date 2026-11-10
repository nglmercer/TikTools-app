//! SQLite bridge for source-fenced v1 candidates and durable pending receipts.
//!
//! This bridge is intentionally not wired to product sync until every plugin
//! mutation participates in admission and the legacy bypass is fenced.
use crate::db::{DatabaseManager, PluginSyncOutboxEntry};
use tiktools_cloud_sync::{
    PluginSelection, SyncCoordError, V1ConflictSourceProof, V1Edit, V1KeepLocalIntent,
    V1LocalEditContext, V1OutboxCandidate, V1Pending, V1ProductOutbox,
};

use crate::{control::PluginSyncRuntimeGuard, AppCore};

#[cfg(test)]
mod abandon_tests;

#[allow(dead_code)]
pub(crate) struct SqliteProductOutbox<'a> {
    db: &'a DatabaseManager,
    runtime_epoch: &'a str,
    observed: Option<PluginSyncOutboxEntry>,
}

#[allow(dead_code)]
impl<'a> SqliteProductOutbox<'a> {
    pub(crate) fn new(db: &'a DatabaseManager, runtime_epoch: &'a str) -> Self {
        Self {
            db,
            runtime_epoch,
            observed: None,
        }
    }
}

fn unavailable() -> SyncCoordError {
    SyncCoordError::Local("local sync source needs review".into())
}

#[allow(dead_code)]
fn candidate(entry: &PluginSyncOutboxEntry) -> Result<V1OutboxCandidate, SyncCoordError> {
    let edit = match (&entry.version, entry.enabled) {
        (Some(version), Some(enabled)) => V1Edit::Put(PluginSelection {
            schema_version: 1,
            plugin_id: entry.plugin_id.clone(),
            version: version.clone(),
            enabled,
        }),
        (None, None) => V1Edit::Delete,
        _ => return Err(unavailable()),
    };
    Ok(V1OutboxCandidate {
        key: entry.plugin_id.clone(),
        edit,
        generation: entry.generation,
    })
}

impl V1ProductOutbox for SqliteProductOutbox<'_> {
    fn next(
        &mut self,
        context: &V1LocalEditContext,
    ) -> Result<Option<V1OutboxCandidate>, SyncCoordError> {
        if self.observed.is_some() {
            return Err(unavailable());
        }
        let entry = self
            .db
            .next_plugin_sync_intent(&context.origin, &context.account_id)
            .map_err(|_| unavailable())?;
        let Some(entry) = entry else { return Ok(None) };
        if entry.origin != context.origin
            || entry.account_id != context.account_id
            || entry.config_generation != context.config_generation
            || entry.generation != entry.source_generation
            || entry.runtime_epoch.as_deref() != Some(self.runtime_epoch)
        {
            return Err(unavailable());
        }
        let value = candidate(&entry)?;
        self.observed = Some(entry);
        Ok(Some(value))
    }

    fn acknowledge(
        &mut self,
        context: &V1LocalEditContext,
        value: &V1OutboxCandidate,
        pending: &V1Pending,
    ) -> Result<bool, SyncCoordError> {
        let Some(entry) = self.observed.take() else {
            return Ok(false);
        };
        if entry.origin != context.origin
            || entry.account_id != context.account_id
            || entry.config_generation != context.config_generation
            || candidate(&entry)? != *value
        {
            return Ok(false);
        }
        self.db
            .receipt_plugin_sync_intent(&entry, self.runtime_epoch, pending)
            .map_err(|_| unavailable())
    }

    fn validate_pending(
        &mut self,
        context: &V1LocalEditContext,
        key: &str,
        pending: &V1Pending,
    ) -> Result<bool, SyncCoordError> {
        self.db
            .validate_plugin_sync_receipt(
                &context.origin,
                &context.account_id,
                key,
                context.config_generation,
                self.runtime_epoch,
                pending,
            )
            .map_err(|_| unavailable())
    }

    fn conflict_source_proof(
        &mut self,
        context: &V1LocalEditContext,
        key: &str,
        pending: &V1Pending,
    ) -> Result<Option<V1ConflictSourceProof>, SyncCoordError> {
        let generation = self
            .db
            .plugin_sync_conflict_source(
                &context.origin,
                &context.account_id,
                key,
                context.config_generation,
                self.runtime_epoch,
                pending,
            )
            .map_err(|_| unavailable())?;
        Ok(generation.and_then(|generation| {
            V1ConflictSourceProof::new(generation, self.runtime_epoch.to_owned())
        }))
    }

    fn has_pending_rebase(&mut self, context: &V1LocalEditContext) -> Result<bool, SyncCoordError> {
        self.db
            .has_plugin_sync_rebase(context)
            .map_err(|_| unavailable())
    }

    fn prepare_rebase(
        &mut self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        proof: &V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        self.db
            .prepare_plugin_sync_rebase(context, intent, self.runtime_epoch, proof)
            .map_err(|_| unavailable())
    }

    fn load_rebase(
        &mut self,
        context: &V1LocalEditContext,
        key: &str,
    ) -> Result<Option<V1KeepLocalIntent>, SyncCoordError> {
        self.db
            .load_plugin_sync_rebase(context, key)
            .map_err(|_| unavailable())
    }

    fn load_rebase_with_previous_consent(
        &mut self,
        context: &V1LocalEditContext,
        key: &str,
    ) -> Result<Option<V1KeepLocalIntent>, SyncCoordError> {
        self.db
            .load_plugin_sync_rebase_with_previous_consent(context, key)
            .map_err(|_| unavailable())
    }

    fn abandon_rebase_with_previous_consent(
        &mut self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
    ) -> Result<bool, SyncCoordError> {
        self.db
            .abandon_plugin_sync_rebase_with_previous_consent(context, intent)
            .map_err(|_| unavailable())
    }

    fn rebase_source_proof(
        &mut self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
    ) -> Result<Option<V1ConflictSourceProof>, SyncCoordError> {
        self.db
            .plugin_sync_rebase_source(context, intent, self.runtime_epoch)
            .map_err(|_| unavailable())
    }

    fn finish_rebase(
        &mut self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        proof: &V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        self.db
            .finish_plugin_sync_rebase(context, intent, self.runtime_epoch, proof)
            .map_err(|_| unavailable())
    }

    fn newer_rebase_candidate(
        &mut self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
    ) -> Result<Option<(V1OutboxCandidate, V1ConflictSourceProof)>, SyncCoordError> {
        self.db
            .newer_plugin_sync_rebase_candidate(context, intent, self.runtime_epoch)
            .map_err(|_| unavailable())
    }

    fn abandon_rebase(
        &mut self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        candidate: &V1OutboxCandidate,
        proof: &V1ConflictSourceProof,
    ) -> Result<bool, SyncCoordError> {
        self.db
            .abandon_plugin_sync_rebase(context, intent, self.runtime_epoch, candidate, proof)
            .map_err(|_| unavailable())
    }

    fn superseded_rebase_generation(
        &mut self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
    ) -> Result<Option<i64>, SyncCoordError> {
        self.db
            .superseded_plugin_sync_rebase_generation(context, intent)
            .map_err(|_| unavailable())
    }

    fn abandon_superseded_rebase(
        &mut self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        generation: i64,
    ) -> Result<bool, SyncCoordError> {
        self.db
            .abandon_superseded_plugin_sync_rebase(context, intent, generation)
            .map_err(|_| unavailable())
    }
}

/// Product-only adapter: the coordinator must acquire the runtime gate
/// before protocol admission and release it before network I/O. A matching
/// live worker is a momentary observation, not a restart or crash outcome.
/// It does not admit deletions or missing packages. This remains unwired
/// until reviewed migration and all local mutation paths share the boundary.
#[allow(dead_code)]
pub(crate) struct RuntimeProductOutbox<'a> {
    core: &'a AppCore,
    inner: SqliteProductOutbox<'a>,
}

impl<'a> RuntimeProductOutbox<'a> {
    pub(crate) fn new(core: &'a AppCore) -> Result<Self, SyncCoordError> {
        let epoch = core.sync.runtime_epoch().map_err(|_| unavailable())?;
        Ok(Self {
            core,
            inner: SqliteProductOutbox::new(&core.db, epoch),
        })
    }
}

#[allow(dead_code)]
struct UnderRuntimeGuard<'a, 'g> {
    inner: &'a mut SqliteProductOutbox<'g>,
    runtime: &'a PluginSyncRuntimeGuard<'g>,
}

#[allow(dead_code)]
fn runtime_agrees(runtime: &PluginSyncRuntimeGuard<'_>, key: &str, edit: &V1Edit) -> bool {
    match edit {
        V1Edit::Put(selection) if selection.plugin_id == key => {
            runtime.matches_selection(key, &selection.version, selection.enabled)
        }
        // A package tombstone requires filesystem/receipt reconciliation,
        // not merely absence of a running process.
        _ => false,
    }
}

impl V1ProductOutbox for RuntimeProductOutbox<'_> {
    fn with_runtime_guard(
        &mut self,
        operation: &mut dyn FnMut(&mut dyn V1ProductOutbox) -> Result<(), SyncCoordError>,
    ) -> Result<(), SyncCoordError> {
        let runtime = self
            .core
            .try_plugin_sync_runtime_guard()
            .map_err(|_| unavailable())?;
        let mut guarded = UnderRuntimeGuard {
            inner: &mut self.inner,
            runtime: &runtime,
        };
        operation(&mut guarded)
    }

    fn next(
        &mut self,
        _: &V1LocalEditContext,
    ) -> Result<Option<V1OutboxCandidate>, SyncCoordError> {
        Err(unavailable())
    }

    fn acknowledge(
        &mut self,
        _: &V1LocalEditContext,
        _: &V1OutboxCandidate,
        _: &V1Pending,
    ) -> Result<bool, SyncCoordError> {
        Err(unavailable())
    }

    fn validate_pending(
        &mut self,
        _: &V1LocalEditContext,
        _: &str,
        _: &V1Pending,
    ) -> Result<bool, SyncCoordError> {
        Err(unavailable())
    }

    fn has_pending_rebase(&mut self, context: &V1LocalEditContext) -> Result<bool, SyncCoordError> {
        self.inner.has_pending_rebase(context)
    }

    fn load_rebase_with_previous_consent(
        &mut self,
        context: &V1LocalEditContext,
        key: &str,
    ) -> Result<Option<V1KeepLocalIntent>, SyncCoordError> {
        self.inner.load_rebase_with_previous_consent(context, key)
    }

    fn abandon_rebase_with_previous_consent(
        &mut self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
    ) -> Result<bool, SyncCoordError> {
        self.inner
            .abandon_rebase_with_previous_consent(context, intent)
    }
}

impl V1ProductOutbox for UnderRuntimeGuard<'_, '_> {
    fn next(
        &mut self,
        context: &V1LocalEditContext,
    ) -> Result<Option<V1OutboxCandidate>, SyncCoordError> {
        let entry = self.inner.next(context)?;
        if entry
            .as_ref()
            .is_some_and(|entry| !runtime_agrees(self.runtime, &entry.key, &entry.edit))
        {
            return Err(unavailable());
        }
        Ok(entry)
    }

    fn acknowledge(
        &mut self,
        context: &V1LocalEditContext,
        candidate: &V1OutboxCandidate,
        pending: &V1Pending,
    ) -> Result<bool, SyncCoordError> {
        if !runtime_agrees(self.runtime, &candidate.key, &candidate.edit) {
            return Ok(false);
        }
        self.inner.acknowledge(context, candidate, pending)
    }

    fn validate_pending(
        &mut self,
        context: &V1LocalEditContext,
        key: &str,
        pending: &V1Pending,
    ) -> Result<bool, SyncCoordError> {
        if !runtime_agrees(self.runtime, key, &pending.edit) {
            return Ok(false);
        }
        self.inner.validate_pending(context, key, pending)
    }

    fn has_pending_rebase(&mut self, context: &V1LocalEditContext) -> Result<bool, SyncCoordError> {
        self.inner.has_pending_rebase(context)
    }
}

impl AppCore {
    /// Private product-drain entry point. An explicit migration must first
    /// install the account marker and integrate all local mutation paths;
    /// until then no user-facing RPC should invoke this method.
    #[allow(dead_code)]
    pub(crate) async fn sync_plugins_with_marked_outbox(
        &self,
    ) -> Result<tiktools_cloud_sync::SyncOutcome, SyncCoordError> {
        let coordinator = self.sync.coordinator().map_err(|_| unavailable())?;
        let mut outbox = RuntimeProductOutbox::new(self)?;
        coordinator.sync_now_with_marked_outbox(&mut outbox).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::LocalPluginSyncIntent, paths::AppPaths};
    use secrecy::SecretString;
    use std::{path::PathBuf, sync::Arc};
    use tiktools_cloud_auth::{CloudAuth, Persistence, TokenStore, TokenStoreError};
    use tiktools_cloud_sync::SyncCoordinator;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::{Mutex, Notify},
    };

    struct Store;
    impl TokenStore for Store {
        fn persistence(&self) -> Persistence {
            Persistence::MemoryOnly
        }
        fn load(&self, _: &str, _: &str) -> Result<Option<SecretString>, TokenStoreError> {
            Ok(Some(SecretString::from("fixture-token".to_owned())))
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

    struct FixtureDir(PathBuf);
    impl Drop for FixtureDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn sqlite_outbox_keeps_a_newer_edit_during_an_admitted_write() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let received = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let remote = Arc::new(Mutex::new(None::<(i64, serde_json::Value)>));
        let server = {
            let received = received.clone();
            let release = release.clone();
            let remote = remote.clone();
            tokio::spawn(async move {
                while let Ok((mut socket, _)) = listener.accept().await {
                    let received = received.clone();
                    let release = release.clone();
                    let remote = remote.clone();
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
                                    .and_then(|v| v.trim().parse::<usize>().ok())
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
                        let (status, body) = if first.starts_with("get /api/v1/native/me ") {
                            ("200 OK", serde_json::json!({"id":"alice"}))
                        } else if first.starts_with("get /api/v1/native/sync/changes?") {
                            let cursor = first
                                .split("cursor=")
                                .nth(1)
                                .and_then(|v| v.split(['&', ' ']).next())
                                .and_then(|v| v.parse::<i64>().ok())
                                .unwrap_or(0);
                            let current = remote.lock().await.clone();
                            let revision = current.as_ref().map_or(0, |(revision, _)| *revision);
                            let changes = current.filter(|(revision, _)| *revision > cursor).map(|(revision, value)| vec![serde_json::json!({"collection":"plugin-selections","key":"acme.plugin","revision":revision,"deleted":false,"value":value})]).unwrap_or_default();
                            (
                                "200 OK",
                                serde_json::json!({"changes":changes,"cursor":revision,"has_more":false}),
                            )
                        } else if first.starts_with(
                            "put /api/v1/native/sync/collections/plugin-selections/acme.plugin ",
                        ) {
                            let Ok(value) = serde_json::from_slice::<serde_json::Value>(
                                &request[header_end..header_end + content_length],
                            ) else {
                                return;
                            };
                            let mut current = remote.lock().await;
                            let previous = current.as_ref().map(|(revision, _)| *revision);
                            let correct = match previous {
                                None => headers.contains("if-none-match: *"),
                                Some(revision) => {
                                    headers.contains(&format!("if-match: \"{revision}\""))
                                }
                            };
                            if !correct {
                                ("409 Conflict", serde_json::json!({"error":"conflict"}))
                            } else {
                                let revision = previous.unwrap_or(0) + 1;
                                *current = Some((revision, value.clone()));
                                drop(current);
                                if revision == 1 {
                                    received.notify_one();
                                    release.notified().await;
                                }
                                (
                                    "200 OK",
                                    serde_json::json!({"collection":"plugin-selections","key":"acme.plugin","revision":revision,"deleted":false,"value":value}),
                                )
                            }
                        } else {
                            ("404 Not Found", serde_json::json!({}))
                        };
                        let bytes = serde_json::to_vec(&body).unwrap();
                        let head = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len());
                        let _ = socket.write_all(head.as_bytes()).await;
                        let _ = socket.write_all(&bytes).await;
                    });
                }
            })
        };
        let root = std::env::temp_dir().join(format!(
            "sync-outbox-e2e-{}-{}",
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
        let db = Arc::new(DatabaseManager::new(paths));
        let epoch = "a".repeat(64);
        let reserve = |enabled| {
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
                assert!(db.confirm_plugin_sync_runtime(&entry, &epoch).unwrap());
            })
            .unwrap();
        };
        reserve(false);
        let task = {
            let sync = sync.clone();
            let db = db.clone();
            let epoch = epoch.clone();
            tokio::spawn(async move {
                let mut outbox = SqliteProductOutbox::new(&db, &epoch);
                sync.sync_now_with_outbox(&mut outbox).await
            })
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), received.notified())
            .await
            .unwrap();
        reserve(true);
        release.notify_one();
        assert_eq!(task.await.unwrap().unwrap().pushed, 1);
        // The first run may already drain the newer candidate, but it must
        // retain a source-bound pending receipt for the next run.
        let pending = V1Pending {
            base_revision: Some(1),
            edit: V1Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "1.0.0".into(),
                enabled: true,
            }),
            conflict: false,
        };
        let consent = sync
            .with_v1_local_edit(|context| context.unwrap().config_generation)
            .unwrap();
        assert!(db
            .validate_plugin_sync_receipt(
                &origin,
                "alice",
                "acme.plugin",
                consent,
                &epoch,
                &pending
            )
            .unwrap());
        let mut outbox = SqliteProductOutbox::new(&db, &epoch);
        assert_eq!(
            sync.sync_now_with_outbox(&mut outbox).await.unwrap().pushed,
            1
        );
        assert_eq!(remote.lock().await.as_ref().unwrap().1["enabled"], true);
        assert!(db
            .next_plugin_sync_intent(&origin, "alice")
            .unwrap()
            .is_none());

        // A second device changes this selection. Preparing its pulled value
        // must bind the durable source and leave the actual plugin state
        // untouched until a separately implemented runtime transition.
        *remote.lock().await = Some((
            3,
            serde_json::json!({
                "schemaVersion": 1, "pluginId": "acme.plugin", "version": "1.0.0", "enabled": false
            }),
        ));
        let outcome = sync.sync_now_with_outbox(&mut outbox).await.unwrap();
        assert!(!outcome.remote[0].selection.as_ref().unwrap().enabled);
        let review = sync
            .review_product_remote_apply("acme.plugin", &mut outbox)
            .await
            .unwrap();
        assert!(super::super::product_apply::prepare_pulled(
            &sync,
            &db,
            &review,
            &mut outbox,
            "2.0.0",
            true,
            true
        )
        .unwrap()
        .is_none());
        assert!(super::super::product_apply::prepare_pulled(
            &sync,
            &db,
            &review,
            &mut outbox,
            "1.0.0",
            true,
            false
        )
        .is_err());
        // Inject a crash-equivalent failure after the intent is durable but
        // before its ready phase is committed.
        let conn = rusqlite::Connection::open(db.automation_path()).unwrap();
        conn.execute_batch("CREATE TRIGGER fail_apply_ready BEFORE UPDATE OF phase ON plugin_sync_apply_intents WHEN NEW.phase='ready' BEGIN SELECT RAISE(FAIL,'injected failure'); END;").unwrap();
        drop(conn);
        assert!(super::super::product_apply::prepare_pulled(
            &sync,
            &db,
            &review,
            &mut outbox,
            "1.0.0",
            true,
            true,
        )
        .is_err());
        let context = sync.with_v1_local_edit(|context| context.unwrap()).unwrap();
        let saved = db.plugin_apply_recovery_page(&context, None, 16).unwrap();
        assert_eq!(saved.intents.len(), 1);
        let original = &saved.intents[0];
        assert_eq!(original.phase, crate::db::PluginApplyPhase::Prepared);
        let intent = original.intent.clone();
        let conn = rusqlite::Connection::open(db.automation_path()).unwrap();
        conn.execute_batch("DROP TRIGGER fail_apply_ready;")
            .unwrap();
        drop(conn);
        assert!(db
            .pulled_plugin_apply_source("acme.plugin")
            .unwrap()
            .is_none());
        let reopened = DatabaseManager::new(db.paths().clone());
        let restarted = SyncCoordinator::new(auth, db.paths().data.join("cloud-sync")).unwrap();
        let mut new_outbox = SqliteProductOutbox::new(&reopened, &epoch);
        assert!(super::super::product_apply::resume_pulled(
            &restarted,
            &reopened,
            &review,
            &mut new_outbox,
            &intent.intent_id,
            "1.0.0",
            true,
            true
        )
        .is_err());
        let fresh = restarted
            .review_product_remote_apply("acme.plugin", &mut new_outbox)
            .await
            .unwrap();
        assert!(super::super::product_apply::resume_pulled(
            &restarted,
            &reopened,
            &fresh,
            &mut new_outbox,
            &intent.intent_id,
            "2.0.0",
            true,
            true
        )
        .is_err());
        assert!(super::super::product_apply::resume_pulled(
            &restarted,
            &reopened,
            &fresh,
            &mut new_outbox,
            &intent.intent_id,
            "1.0.0",
            true,
            false
        )
        .is_err());
        assert_eq!(
            super::super::product_apply::resume_pulled(
                &restarted,
                &reopened,
                &fresh,
                &mut new_outbox,
                &intent.intent_id,
                "1.0.0",
                true,
                true
            )
            .unwrap(),
            Some(intent.clone())
        );
        let stored = reopened
            .plugin_apply_intent(&intent.intent_id)
            .unwrap()
            .unwrap();
        assert_eq!(stored.phase, crate::db::PluginApplyPhase::Ready);
        assert_eq!(stored.intent, intent);
        assert!(stored.committed_generation.is_none());
        assert!(stored.runtime_epoch.is_none());
        assert_eq!(
            super::super::product_apply::resume_pulled(
                &restarted,
                &reopened,
                &fresh,
                &mut new_outbox,
                &intent.intent_id,
                "1.0.0",
                true,
                true
            )
            .unwrap(),
            Some(intent.clone())
        );
        reopened
            .set_plugin_state_with_sync_intent("acme.plugin", true, false, None)
            .unwrap();
        assert!(super::super::product_apply::resume_pulled(
            &restarted,
            &reopened,
            &fresh,
            &mut new_outbox,
            &intent.intent_id,
            "1.0.0",
            true,
            true
        )
        .is_err());
        assert_eq!(
            reopened
                .plugin_apply_intent(&intent.intent_id)
                .unwrap()
                .unwrap()
                .intent,
            intent
        );
        server.abort();
    }
}
