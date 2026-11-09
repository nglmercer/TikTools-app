//! SQLite bridge for source-fenced v1 candidates and durable pending receipts.
//!
//! This bridge is intentionally not wired to product sync until every plugin
//! mutation participates in admission and the legacy bypass is fenced.
use crate::db::{DatabaseManager, PluginSyncOutboxEntry};
use tiktools_cloud_sync::{
    PluginSelection, SyncCoordError, V1ConflictSourceProof, V1Edit, V1KeepLocalIntent,
    V1LocalEditContext, V1OutboxCandidate, V1Pending, V1ProductOutbox,
};

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
        let sync = SyncCoordinator::new(auth, paths.data.join("cloud-sync")).unwrap();
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
        server.abort();
    }
}
