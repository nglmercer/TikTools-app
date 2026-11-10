//! Private, informational cutover preview from the native plugin source.
//!
//! This is not authorization to migrate. Package filesystem I/O, independent
//! processes, account invalidation and final cutover still need one shared
//! admission boundary before any RPC or product activation can use it.
use std::{collections::BTreeMap, sync::Arc};

use cloud_client::{PluginSelection, Session};
use tiktools_cloud_sync::{
    CutoverExecution, CutoverPreview, CutoverSource, GuardedCutoverReview, SyncConfigEpoch,
    SyncCoordError,
};

use crate::{control::OperationError, db::PluginCutoverSource, AppCore};

#[derive(PartialEq, Eq)]
struct LocalSource {
    sqlite: PluginCutoverSource,
    catalog_revision: u64,
    selections: Vec<PluginSelection>,
}

/// Native-only state: private account data is never serialized to a caller.
pub(crate) struct NativeCutoverPreview {
    review: GuardedCutoverReview,
    source: LocalSource,
    session: Arc<Session>,
    consent: SyncConfigEpoch,
}

fn unavailable() -> OperationError {
    OperationError::unavailable("local plugin source is unavailable for Cloud migration review")
}

fn source_error() -> SyncCoordError {
    SyncCoordError::Local("native plugin source is unavailable for Cloud migration".into())
}

fn local_source_from_catalog(
    sqlite: PluginCutoverSource,
    revision: u64,
    plugins: Vec<tiktools_plugin_loader::DiscoveredPlugin>,
    activation: &BTreeMap<String, crate::plugin_runtime::PluginActivation>,
) -> Result<LocalSource, OperationError> {
    let states = sqlite
        .rows
        .iter()
        .map(|row| (row.id.as_str(), row.state))
        .collect::<BTreeMap<_, _>>();
    let mut selections = Vec::with_capacity(plugins.len());
    let mut catalog = std::collections::BTreeSet::new();
    for plugin in plugins {
        let id = plugin.manifest.id;
        if !catalog.insert(id.clone()) {
            return Err(unavailable());
        }
        let (installed, enabled) = states
            .get(id.as_str())
            .copied()
            .flatten()
            .unwrap_or((true, true));
        let runtime = activation
            .get(&id)
            .map(|state| (state.installed, state.enabled))
            .unwrap_or((true, true));
        if (installed, enabled) != runtime {
            return Err(unavailable());
        }
        if installed {
            selections.push(PluginSelection {
                schema_version: 1,
                plugin_id: id,
                version: plugin.manifest.version,
                enabled,
            });
        }
    }
    if activation
        .iter()
        .any(|(id, state)| state.installed && !catalog.contains(id))
        || sqlite.rows.iter().any(|row| {
            row.state.is_some_and(|(installed, _)| installed) && !catalog.contains(&row.id)
        })
    {
        return Err(unavailable());
    }
    Ok(LocalSource {
        sqlite,
        catalog_revision: revision,
        selections,
    })
}

impl AppCore {
    fn cutover_local_source(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<LocalSource, OperationError> {
        let _runtime = self.try_plugin_sync_runtime_guard()?;
        let quarantine = self
            .plugin_state
            .apply_quarantine
            .read()
            .map_err(|_| unavailable())?;
        if !quarantine
            .as_ref()
            .is_some_and(|blocked| blocked.is_empty())
        {
            return Err(unavailable());
        }
        drop(quarantine);
        let sqlite = self
            .db
            .plugin_cutover_source(origin, account)
            .map_err(|_| unavailable())?;
        let (revision, plugins) = self
            .plugins
            .list_complete_for_cutover()
            .map_err(|_| unavailable())?;
        let activation = self
            .plugin_state
            .activation
            .read()
            .map_err(|_| unavailable())?;
        let source = local_source_from_catalog(sqlite, revision, plugins, &activation)?;
        if self.plugins.registry_revision() != revision
            || self
                .db
                .plugin_cutover_source(origin, account)
                .map_err(|_| unavailable())?
                != source.sqlite
        {
            return Err(unavailable());
        }
        Ok(source)
    }

    pub(crate) fn preview_local_v2_cutover(
        &self,
        session: Arc<Session>,
    ) -> Result<NativeCutoverPreview, OperationError> {
        let coordinator = self.sync.coordinator().map_err(|_| unavailable())?;
        let consent = coordinator
            .capture_config_epoch(&session)
            .ok_or_else(unavailable)?;
        let origin = session.instance_origin();
        let account = session.account_id();
        let source = self.cutover_local_source(&origin, account)?;
        let review = if source.sqlite.receipts.is_empty() {
            coordinator
                .guarded_legacy_promotion_review(&session, &source.selections)
                .or_else(|_| {
                    coordinator.guarded_marked_cutover_review(&session, &source.selections, &[])
                })
                .map_err(|_| unavailable())?
        } else {
            coordinator
                .guarded_marked_cutover_review(
                    &session,
                    &source.selections,
                    &source.sqlite.receipts,
                )
                .map_err(|_| unavailable())?
        };
        if self.cutover_local_source(&origin, account)? != source {
            return Err(unavailable());
        }
        self.cloud
            .auth()
            .with_current_session(&session, || {
                if !coordinator.is_config_epoch_current(&consent, &session) {
                    return Err(unavailable());
                }
                Ok(())
            })
            .map_err(|_| unavailable())??;
        Ok(NativeCutoverPreview {
            review,
            source,
            session,
            consent,
        })
    }
}

impl NativeCutoverPreview {
    pub(crate) fn preview(&self) -> &CutoverPreview {
        self.review.preview()
    }

    /// Rechecks local source, v1 snapshot, receipts, session and consent for
    /// an informational preview. Mutations can still race after this returns;
    /// only the separately fenced cutover can authorize a migration.
    pub(crate) fn check_local_fresh(&self, core: &AppCore) -> Result<(), OperationError> {
        let coordinator = core.sync.coordinator().map_err(|_| unavailable())?;
        let origin = self.session.instance_origin();
        let account = self.session.account_id();
        if core.cutover_local_source(&origin, account)? != self.source {
            return Err(unavailable());
        }
        coordinator
            .verify_guarded_cutover_review(
                &self.review,
                &self.source.selections,
                &self.source.sqlite.receipts,
            )
            .map_err(|_| unavailable())?;
        if core.cutover_local_source(&origin, account)? != self.source {
            return Err(unavailable());
        }
        core.cloud
            .auth()
            .with_current_session(&self.session, || {
                if !coordinator.is_config_epoch_current(&self.consent, &self.session) {
                    return Err(unavailable());
                }
                Ok(())
            })
            .map_err(|_| unavailable())??;
        Ok(())
    }
}

/// The calling host owns review and user approval. This adapter only guards
/// cooperative local writes for the duration of the synchronous operation;
/// it does not establish trust in arbitrary filesystem writers or implement
/// v2 staging for later local mutations.
struct NativeCutoverSource<'a> {
    core: &'a AppCore,
    expected: &'a LocalSource,
    origin: &'a str,
    account: &'a str,
}

#[cfg(feature = "plugin-install")]
impl CutoverSource for NativeCutoverSource<'_> {
    fn with_guarded_source<T>(
        &self,
        operation: impl FnOnce(
            &[PluginSelection],
            &[tiktools_cloud_sync::V1CutoverReceipt],
        ) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError> {
        // Global order: invocation -> cooperative package store -> SQLite ->
        // catalog/activation -> protocol -> session. No network or await.
        let runtime = self
            .core
            .try_plugin_sync_runtime_guard()
            .map_err(|_| source_error())?;
        let quarantine = self
            .core
            .plugin_state
            .apply_quarantine
            .read()
            .map_err(|_| source_error())?;
        if !quarantine
            .as_ref()
            .is_some_and(|blocked| blocked.is_empty())
        {
            return Err(source_error());
        }
        let packages = self.core.plugin_packages().map_err(|_| source_error())?;
        let store = packages
            .installer
            .try_lock_store()
            .map_err(|_| source_error())?;
        if !store
            .matches_roots(
                &self.core.db.paths().plugins,
                &self.core.db.paths().plugin_store(),
            )
            .map_err(|_| source_error())?
        {
            return Err(source_error());
        }
        let result = self
            .core
            .db
            .with_locked_plugin_cutover_source::<_, crate::db::DatabaseError>(
                self.origin,
                self.account,
                |sqlite| {
                    let result =
                        self.core
                            .plugins
                            .with_complete_cutover_catalog(|revision, plugins| {
                                let activation = self
                                    .core
                                    .plugin_state
                                    .activation
                                    .read()
                                    .map_err(|_| source_error())?;
                                let source = local_source_from_catalog(
                                    sqlite,
                                    revision,
                                    plugins,
                                    &activation,
                                )
                                .map_err(|_| source_error())?;
                                if &source != self.expected {
                                    return Err(SyncCoordError::StaleReview);
                                }
                                let outcome =
                                    operation(&source.selections, &source.sqlite.receipts);
                                drop(activation);
                                outcome
                            });
                    Ok(result.unwrap_or_else(|_| Err(source_error())))
                },
            )
            .map_err(|_| source_error())?;
        drop(store);
        drop(quarantine);
        drop(runtime);
        result
    }
}

#[cfg(not(feature = "plugin-install"))]
impl CutoverSource for NativeCutoverSource<'_> {
    fn with_guarded_source<T>(
        &self,
        _operation: impl FnOnce(
            &[PluginSelection],
            &[tiktools_cloud_sync::V1CutoverReceipt],
        ) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError> {
        Err(source_error())
    }
}

impl AppCore {
    /// Private native wiring; invoke only on a blocking worker because it can
    /// hold plugin and database fences during durable filesystem writes.
    /// Product activation still needs a source fence for all roots/writers,
    /// v2 staging and an explicit user review API.
    pub(crate) fn execute_local_v2_cutover(
        &self,
        review: &NativeCutoverPreview,
    ) -> Result<CutoverExecution, OperationError> {
        let coordinator = self.sync.coordinator().map_err(|_| unavailable())?;
        let origin = review.session.instance_origin();
        let source = NativeCutoverSource {
            core: self,
            expected: &review.source,
            origin: &origin,
            account: review.session.account_id(),
        };
        coordinator
            .execute_cutover_and_scope_consent_with_source(&review.review, &source)
            .map_err(|_| unavailable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{paths::AppPaths, HostEmitter, HostMessage, NoopMediaHost};

    struct Emitter;
    impl HostEmitter for Emitter {
        fn emit(&self, _: HostMessage) {}
    }

    #[tokio::test]
    async fn owner_preview_reports_local_counts_without_migrating() {
        use secrecy::SecretString;
        use tiktools_cloud_auth::{CloudAuth, Persistence, TokenStore, TokenStoreError};
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
            fn store(&self, _: &str, _: &str, _: &Session) -> Result<(), TokenStoreError> {
                Ok(())
            }
            fn delete(&self, _: &str, _: &str) -> Result<(), TokenStoreError> {
                Ok(())
            }
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0; 4096];
                let _ = socket.read(&mut buffer).await;
                let body = r#"{"id":"alice"}"#;
                let _ = socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await;
            }
        });
        let root = std::env::temp_dir().join(format!(
            "preview-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        std::fs::create_dir_all(&paths.builtin_plugins).unwrap();
        let dir = paths.plugins.join("demo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.json"), r#"{"schemaVersion":3,"id":"demo","name":"Demo","version":"1.0.0","runtime":"declarative"}"#).unwrap();
        let sidecar = paths.data.join("account.json");
        std::fs::write(
            &sidecar,
            serde_json::to_vec(&serde_json::json!({"origin":origin,"account_id":"alice"})).unwrap(),
        )
        .unwrap();
        let auth = CloudAuth::new(Some(origin), sidecar, Arc::new(Store));
        auth.restore().await.unwrap();
        let mut core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        ));
        Arc::get_mut(&mut core).unwrap().cloud = Arc::new(
            crate::cloud_account::CloudAccount::with_test_auth(auth.clone()),
        );
        Arc::get_mut(&mut core).unwrap().sync =
            Arc::new(crate::cloud_sync::CloudSync::new(&paths, &auth));
        core.sync.coordinator().unwrap().set_enabled(true).unwrap();
        let result = core.workspace_v2_preview().await.unwrap();
        assert_eq!(result.selections, 1);
        assert_eq!(result.pending, 1);
        assert!(result.migration_available);
        core.db
            .invalidate_plugin_package_source_untracked("demo")
            .unwrap();
        assert!(core.workspace_v2_preview().await.is_err());
        core.shutdown().await;
        server.abort();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn local_source_combines_catalog_and_persisted_state_without_dropping_installs() {
        let root = std::env::temp_dir().join(format!(
            "plugin-cutover-native-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        std::fs::create_dir_all(&paths.builtin_plugins).unwrap();
        for file in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            drop(rusqlite::Connection::open(file).unwrap());
        }
        let plugin_dir = paths.plugins.join("demo");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(plugin_dir.join("plugin.json"), r#"{"schemaVersion":3,"id":"demo","name":"Demo","version":"1.0.0","runtime":"declarative"}"#).unwrap();
        let core = AppCore::with_paths(Arc::new(Emitter), Arc::new(NoopMediaHost), paths);
        let origin = "https://cloud.example";
        let initial = core.cutover_local_source(origin, "alice").unwrap();
        assert_eq!(initial.selections.len(), 1);
        assert_eq!(initial.selections[0].plugin_id, "demo");
        assert!(initial.selections[0].enabled);
        core.db.set_plugin_state("demo", true, false).unwrap();
        assert!(core.cutover_local_source(origin, "alice").is_err());
        core.plugin_state.activation.write().unwrap().insert(
            "demo".into(),
            crate::plugin_runtime::PluginActivation {
                installed: true,
                enabled: false,
            },
        );
        let disabled = core.cutover_local_source(origin, "alice").unwrap();
        assert!(!disabled.selections[0].enabled);
        assert!(initial != disabled);
        core.db
            .set_plugin_state("missing.plugin", true, true)
            .unwrap();
        assert!(core.cutover_local_source(origin, "alice").is_err());
        core.db
            .set_plugin_state("missing.plugin", false, false)
            .unwrap();
        assert!(core.cutover_local_source(origin, "alice").is_ok());
        drop(core);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(feature = "plugin-install")]
    #[test]
    fn guarded_source_holds_and_releases_runtime_package_and_sqlite_fences() {
        let root = std::env::temp_dir().join(format!(
            "plugin-cutover-fences-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        std::fs::create_dir_all(&paths.builtin_plugins).unwrap();
        // This isolated fixture must never open an operator-provided store.
        assert!(paths.plugin_store().starts_with(&root));
        for file in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            drop(rusqlite::Connection::open(file).unwrap());
        }
        let plugin_dir = paths.plugins.join("demo");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(plugin_dir.join("plugin.json"), r#"{"schemaVersion":3,"id":"acme.demo","name":"Demo","version":"1.0.0","runtime":"declarative"}"#).unwrap();
        let core = AppCore::with_paths(Arc::new(Emitter), Arc::new(NoopMediaHost), paths);
        let origin = "https://cloud.example";
        let expected = core.cutover_local_source(origin, "alice").unwrap();
        let source = NativeCutoverSource {
            core: &core,
            expected: &expected,
            origin,
            account: "alice",
        };
        let other_db = rusqlite::Connection::open(core.db.automation_path()).unwrap();
        other_db.busy_timeout(std::time::Duration::ZERO).unwrap();
        let result: Result<(), SyncCoordError> = source.with_guarded_source(|selections, receipts| {
            assert_eq!(selections.len(), 1);
            assert!(receipts.is_empty());
            assert!(core.try_plugin_sync_runtime_guard().is_err());
            assert!(core.plugin_packages().unwrap().installer.try_lock_store().is_err());
            let error = other_db.execute("INSERT INTO behavior_plugins(id,installed,enabled,updated_at) VALUES ('other',1,1,0)", []).unwrap_err();
            assert!(matches!(error, rusqlite::Error::SqliteFailure(ref error, _) if matches!(error.code, rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)));
            Err(SyncCoordError::StaleReview)
        });
        assert!(matches!(result, Err(SyncCoordError::StaleReview)));
        assert!(core.try_plugin_sync_runtime_guard().is_ok());
        assert!(core
            .plugin_packages()
            .unwrap()
            .installer
            .try_lock_store()
            .is_ok());
        // Rewriting the same apparent installed/enabled value still changes
        // source evidence and must invalidate the reviewed native source.
        core.db.set_plugin_state("acme.demo", true, true).unwrap();
        let mut ran = false;
        let stale = source.with_guarded_source(|_, _| {
            ran = true;
            Ok(())
        });
        assert!(matches!(stale, Err(SyncCoordError::StaleReview)));
        assert!(!ran);
        let pending = tiktools_cloud_sync::V1Pending {
            base_revision: None,
            edit: tiktools_cloud_sync::V1Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.demo".into(),
                version: "1.0.0".into(),
                enabled: true,
            }),
            conflict: false,
        };
        other_db.execute("INSERT INTO plugin_sync_receipts(origin,account_id,plugin_id,generation,config_generation,runtime_epoch,pending_json) VALUES (?1,'alice','acme.demo',1,'7',?2,?3)", rusqlite::params![origin, "a".repeat(64), serde_json::to_string(&pending).unwrap()]).unwrap();
        let reviewed = core.cutover_local_source(origin, "alice").unwrap();
        assert_eq!(reviewed.sqlite.receipts.len(), 1);
        let marked = NativeCutoverSource {
            core: &core,
            expected: &reviewed,
            origin,
            account: "alice",
        };
        marked
            .with_guarded_source(|_, receipts| {
                assert_eq!(receipts.len(), 1);
                assert_eq!(receipts[0].pending(), &pending);
                Ok(())
            })
            .unwrap();
        let mut replaced = pending.clone();
        replaced.base_revision = Some(4);
        other_db.execute("UPDATE plugin_sync_receipts SET pending_json=?1 WHERE origin=?2 AND account_id='alice' AND plugin_id='acme.demo'", rusqlite::params![serde_json::to_string(&replaced).unwrap(),origin]).unwrap();
        let mut invoked = false;
        let stale = marked.with_guarded_source(|_, _| {
            invoked = true;
            Ok(())
        });
        assert!(matches!(stale, Err(SyncCoordError::StaleReview)));
        assert!(!invoked);
        drop(other_db);
        drop(core);
        let _ = std::fs::remove_dir_all(root);
    }
}
