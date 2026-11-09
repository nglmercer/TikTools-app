//! Plugin lifecycle, installation, and action execution operations.

use super::{clean_plugin_id, OperationError};
use crate::events::DomainEvent;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

/// A local mutation's durable sync candidate. This is not yet connected to
/// the product lifecycle: a v1 network drain must share its admission lease
/// and check the outbox before legacy staging can safely be removed.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
#[derive(Debug)]
struct ReservedLocalPluginMutation {
    candidate: Option<crate::db::PluginSyncOutboxEntry>,
    legacy_staging: bool,
}

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
#[derive(Clone, Copy)]
enum LocalMutationProtocol {
    V1,
    V2,
    Selected,
}

/// Holds the invocation boundary before the coordinator's short admission
/// lease. This is a momentary host-worker observation, not a promise that a
/// child process is healthy or that the observation survives this guard.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
pub(crate) struct PluginSyncRuntimeGuard<'a> {
    core: &'a AppCore,
    _invocation: std::sync::MutexGuard<'a, ()>,
}

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
impl PluginSyncRuntimeGuard<'_> {
    pub(crate) fn matches_selection(&self, id: &str, version: &str, enabled: bool) -> bool {
        let Ok(quarantine) = self.core.plugin_state.apply_quarantine.read() else {
            return false;
        };
        if quarantine
            .as_ref()
            .is_none_or(|blocked| blocked.contains(id))
        {
            return false;
        }
        drop(quarantine);
        let Ok(Some(plugin)) = self.core.plugins.get_for_admission(id) else {
            return false;
        };
        if plugin.manifest.id != id
            || plugin.manifest.version != version
            || (enabled && !plugin.available)
        {
            return false;
        }
        let Ok(activation) = self.core.plugin_state.activation.read() else {
            return false;
        };
        if !activation
            .get(id)
            .is_some_and(|value| value.installed && value.enabled == enabled)
        {
            return false;
        }
        drop(activation);
        let expected = if enabled {
            tiktools_plugin_loader::PluginWorkerObservation::WorkerPresent
        } else {
            tiktools_plugin_loader::PluginWorkerObservation::Stopped
        };
        self.core
            .plugins
            .observe_worker(id)
            .is_ok_and(|actual| actual == expected)
    }
}

/// Atomically records the desired local state and, when opted in, an
/// identity-bound candidate. The coordinator keeps protocol admission and
/// identity stable for the entire SQLite transaction. This helper performs
/// no plugin runtime operation and cannot publish to Cloud.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
#[allow(dead_code)]
fn reserve_local_plugin_mutation(
    sync: &tiktools_cloud_sync::SyncCoordinator,
    db: &crate::db::DatabaseManager,
    id: &str,
    version: &str,
    installed: bool,
    enabled: bool,
) -> Result<ReservedLocalPluginMutation, OperationError> {
    reserve_local_plugin_mutation_prepared(sync, db, id, version, installed, enabled, |_| Ok(()))
        .map(|(reservation, ())| reservation)
}

/// Prepare runtime confirmation while the exact opt-in/identity context is
/// held, before the SQLite source transaction. Local-only edits need no
/// runtime sync identity or entropy.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
fn reserve_local_plugin_mutation_prepared<T>(
    sync: &tiktools_cloud_sync::SyncCoordinator,
    db: &crate::db::DatabaseManager,
    id: &str,
    version: &str,
    installed: bool,
    enabled: bool,
    prepare: impl FnOnce(bool) -> Result<T, OperationError>,
) -> Result<(ReservedLocalPluginMutation, T), OperationError> {
    reserve_local_plugin_mutation_prepared_for(
        sync,
        db,
        id,
        version,
        installed,
        enabled,
        LocalMutationProtocol::V1,
        prepare,
    )
}

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
fn reserve_local_plugin_mutation_prepared_for<T>(
    sync: &tiktools_cloud_sync::SyncCoordinator,
    db: &crate::db::DatabaseManager,
    id: &str,
    version: &str,
    installed: bool,
    enabled: bool,
    protocol: LocalMutationProtocol,
    prepare: impl FnOnce(bool) -> Result<T, OperationError>,
) -> Result<(ReservedLocalPluginMutation, T), OperationError> {
    let operation =
        |context: Option<tiktools_cloud_sync::V1LocalEditContext>, legacy_staging, untracked| {
            let prepared = prepare(context.is_some())?;
            let intent = context
                .as_ref()
                .map(|context| crate::db::LocalPluginSyncIntent {
                    origin: &context.origin,
                    account_id: &context.account_id,
                    config_generation: context.config_generation,
                    version: installed.then_some(version),
                });
            let generation = if untracked {
                let fence = tiktools_cloud_sync::ProductEmissionFence::open(
                    db.paths().data.join("plugin-sync-emission"),
                )
                .map_err(|_| OperationError::unavailable("sync emission fence unavailable"))?;
                let _lease = fence
                    .try_untracked_edit()
                    .map_err(|_| OperationError::unavailable("sync emission is in progress"))?;
                db.set_plugin_state_with_untracked_sync(id, installed, enabled)
            } else {
                db.set_plugin_state_with_sync_intent(id, installed, enabled, intent)
            }
            .map_err(|error| OperationError::internal(error.to_string()))?;
            let candidate = context.map(|context| crate::db::PluginSyncOutboxEntry {
                origin: context.origin,
                account_id: context.account_id,
                plugin_id: id.to_owned(),
                generation,
                source_generation: generation,
                config_generation: context.config_generation,
                runtime_epoch: None,
                version: installed.then(|| version.to_owned()),
                enabled: installed.then_some(enabled),
            });
            Ok((
                ReservedLocalPluginMutation {
                    candidate,
                    legacy_staging,
                },
                prepared,
            ))
        };
    match protocol {
        LocalMutationProtocol::V1 => {
            sync.with_v1_local_edit(|context| operation(context, false, false))
        }
        LocalMutationProtocol::V2 => {
            sync.with_v2_local_edit(|context| operation(context, false, false))
        }
        LocalMutationProtocol::Selected => sync.with_product_local_edit(|selected| {
            use tiktools_cloud_sync::ProductLocalEditContext;
            match selected {
                ProductLocalEditContext::Legacy => operation(None, true, false),
                ProductLocalEditContext::OptedOut | ProductLocalEditContext::V2(None) => {
                    operation(None, false, true)
                }
                ProductLocalEditContext::V2(Some(context)) => {
                    operation(Some(context), false, false)
                }
            }
        }),
    }
    .map_err(|_| OperationError::unavailable("Cloud sync admission is unavailable"))?
}

/// Confirm only after the caller, under its plugin mutation gate, has
/// observed that the runtime reflects the reserved source state. Stale or
/// replaced reservations remain durable and cannot be acknowledged.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
#[allow(dead_code)]
fn confirm_local_plugin_mutation(
    db: &crate::db::DatabaseManager,
    reservation: &ReservedLocalPluginMutation,
    runtime_epoch: &str,
) -> Result<(), OperationError> {
    let Some(candidate) = reservation.candidate.as_ref() else {
        return Ok(());
    };
    let confirmed = db
        .confirm_plugin_sync_runtime(candidate, runtime_epoch)
        .map_err(|error| OperationError::internal(error.to_string()))?;
    if !confirmed {
        return Err(OperationError::conflict(
            "plugin state changed before sync confirmation",
        ));
    }
    Ok(())
}

#[cfg(all(test, feature = "cloud-auth", feature = "persistence"))]
mod durable_local_mutation_tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc};
    use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore};
    use tiktools_cloud_sync::SyncCoordinator;

    struct Fixture {
        root: PathBuf,
        db: crate::db::DatabaseManager,
        auth: CloudAuth,
        sync: SyncCoordinator,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "plugin-mutation-{}-{}",
                std::process::id(),
                fastrand::u64(..)
            ));
            let paths = crate::paths::AppPaths {
                data: root.join("data"),
                plugins: root.join("plugins"),
                plugin_data: root.join("plugin-data"),
                builtin_plugins: root.join("absent"),
                development_plugins: None,
                logs: root.join("logs"),
                temp: root.join("temp"),
                root: root.clone(),
            };
            paths.ensure_directories().unwrap();
            let sidecar = paths.data.join("account.json");
            std::fs::write(
                &sidecar,
                br#"{"origin":"https://cloud.example","account_id":"alice"}"#,
            )
            .unwrap();
            let auth = CloudAuth::new(
                Some("https://cloud.example".into()),
                sidecar,
                Arc::new(MemoryTokenStore::new()),
            );
            let sync = SyncCoordinator::new(auth.clone(), paths.data.join("cloud-sync")).unwrap();
            let db = crate::db::DatabaseManager::new(paths);
            Self {
                root,
                db,
                auth,
                sync,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn reservation_requires_runtime_confirmation_and_detects_a_newer_edit() {
        let f = Fixture::new();
        f.sync.set_enabled(true).unwrap();
        let first =
            reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, false).unwrap();
        let entry =
            f.db.next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap();
        assert_eq!(entry, *first.candidate.as_ref().unwrap());
        let epoch = "a".repeat(64);
        // No runtime observation yet: no consumer can acknowledge this edit.
        assert!(!f.db.acknowledge_plugin_sync_intent(&entry, &epoch).unwrap());
        let second =
            reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, true).unwrap();
        assert!(confirm_local_plugin_mutation(&f.db, &first, &epoch).is_err());
        confirm_local_plugin_mutation(&f.db, &second, &epoch).unwrap();
        let updated =
            f.db.next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap();
        assert_eq!(updated.enabled, Some(true));
        assert_eq!(updated.runtime_epoch.as_deref(), Some(epoch.as_str()));
        assert!(!f.db.acknowledge_plugin_sync_intent(&entry, &epoch).unwrap());
    }

    #[test]
    fn opt_out_records_local_edit_without_a_new_cloud_candidate() {
        let f = Fixture::new();
        f.sync.set_enabled(true).unwrap();
        reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, true).unwrap();
        f.sync.set_enabled(false).unwrap();
        let local =
            reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, false).unwrap();
        assert!(local.candidate.is_none());
        let old =
            f.db.next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap();
        assert_ne!(old.generation, old.source_generation);
    }

    #[test]
    fn v2_local_reservation_preserves_local_edits_and_rejects_legacy_consent() {
        let f = Fixture::new();
        f.sync.set_enabled(true).unwrap();
        reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, true).unwrap();
        let previous =
            f.db.next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap();
        let prepared = std::cell::Cell::new(false);
        assert!(reserve_local_plugin_mutation_prepared_for(
            &f.sync,
            &f.db,
            "hotkeys",
            "1.0.0",
            true,
            false,
            LocalMutationProtocol::V2,
            |_| {
                prepared.set(true);
                Ok(())
            },
        )
        .is_err());
        assert!(!prepared.get());
        assert_eq!(
            f.db.next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap(),
            previous
        );

        f.sync.set_enabled(false).unwrap();
        let (local, ()) = reserve_local_plugin_mutation_prepared_for(
            &f.sync,
            &f.db,
            "hotkeys",
            "1.0.0",
            true,
            false,
            LocalMutationProtocol::V2,
            |has_candidate| {
                assert!(!has_candidate);
                Ok(())
            },
        )
        .unwrap();
        assert!(local.candidate.is_none());
        let pending =
            f.db.next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap();
        assert_eq!(pending.generation, previous.generation);
        assert_ne!(pending.source_generation, previous.source_generation);
    }

    #[test]
    fn runtime_identity_is_needed_only_for_an_opted_in_candidate() {
        let f = Fixture::new();
        let (local, value) = reserve_local_plugin_mutation_prepared(
            &f.sync,
            &f.db,
            "acme.local",
            "1.0.0",
            true,
            false,
            |has_candidate| {
                assert!(!has_candidate);
                Ok("local-only")
            },
        )
        .unwrap();
        assert!(local.candidate.is_none());
        assert_eq!(value, "local-only");

        f.sync.set_enabled(true).unwrap();
        let error = reserve_local_plugin_mutation_prepared(
            &f.sync,
            &f.db,
            "acme.blocked",
            "1.0.0",
            true,
            true,
            |has_candidate| {
                assert!(has_candidate);
                Err::<(), _>(OperationError::unavailable("no runtime identity"))
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "unavailable");
        assert!(f
            .db
            .next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .is_none());
        let snapshot = f.db.load_behavior_snapshot().unwrap();
        assert!(!snapshot["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|plugin| plugin["id"] == "acme.blocked"));
    }

    #[test]
    fn sqlite_adapter_requires_confirmed_exact_source_and_retains_receipt() {
        use crate::cloud_sync::product_outbox::SqliteProductOutbox;
        use tiktools_cloud_sync::{V1LocalEditContext, V1Pending, V1ProductOutbox};

        let f = Fixture::new();
        f.sync.set_enabled(true).unwrap();
        let reservation =
            reserve_local_plugin_mutation(&f.sync, &f.db, "acme.plugin", "1.0.0", true, false)
                .unwrap();
        let entry = reservation.candidate.as_ref().unwrap();
        let context = V1LocalEditContext {
            origin: entry.origin.clone(),
            account_id: entry.account_id.clone(),
            config_generation: entry.config_generation,
        };
        let epoch = "a".repeat(64);
        let mut adapter = SqliteProductOutbox::new(&f.db, &epoch);
        assert!(adapter.next(&context).is_err());
        confirm_local_plugin_mutation(&f.db, &reservation, &epoch).unwrap();
        let candidate = adapter.next(&context).unwrap().unwrap();
        let pending = V1Pending {
            base_revision: Some(4),
            edit: candidate.edit.clone(),
            conflict: false,
        };
        let mut changed = candidate.clone();
        changed.generation += 1;
        assert!(!adapter.acknowledge(&context, &changed, &pending).unwrap());
        assert!(f
            .db
            .next_plugin_sync_intent(&context.origin, &context.account_id)
            .unwrap()
            .is_some());
        let candidate = adapter.next(&context).unwrap().unwrap();
        assert!(adapter.acknowledge(&context, &candidate, &pending).unwrap());
        assert!(adapter
            .validate_pending(&context, "acme.plugin", &pending)
            .unwrap());
        let mut conflict = pending.clone();
        conflict.conflict = true;
        let proof = adapter
            .conflict_source_proof(&context, "acme.plugin", &conflict)
            .unwrap()
            .unwrap();
        assert_eq!(proof.generation(), candidate.generation);
        let mut rebased = pending.clone();
        rebased.base_revision = Some(5);
        assert!(!adapter
            .validate_pending(&context, "acme.plugin", &rebased)
            .unwrap());
        rebased.conflict = true;
        assert!(adapter
            .conflict_source_proof(&context, "acme.plugin", &rebased)
            .unwrap()
            .is_none());
        // Even an older writer that failed to advance the source generation
        // must not authorize review while the persisted state differs.
        rusqlite::Connection::open(f.db.paths().automation_database())
            .unwrap()
            .execute(
                "UPDATE behavior_plugins SET enabled=1 WHERE id='acme.plugin'",
                [],
            )
            .unwrap();
        assert!(adapter
            .conflict_source_proof(&context, "acme.plugin", &conflict)
            .unwrap()
            .is_none());
        reserve_local_plugin_mutation(&f.sync, &f.db, "acme.plugin", "1.0.0", true, true).unwrap();
        assert!(!adapter
            .validate_pending(&context, "acme.plugin", &pending)
            .unwrap());
        assert!(adapter
            .conflict_source_proof(&context, "acme.plugin", &conflict)
            .unwrap()
            .is_none());
    }

    #[test]
    fn runtime_epoch_is_stable_per_host_and_changes_after_recreation() {
        let f = Fixture::new();
        let first = crate::cloud_sync::CloudSync::new(f.db.paths(), &f.auth);
        let second = crate::cloud_sync::CloudSync::new(f.db.paths(), &f.auth);
        let a = first.runtime_epoch().unwrap();
        assert_eq!(a, first.runtime_epoch().unwrap());
        assert_eq!(a.len(), 64);
        assert_ne!(a, second.runtime_epoch().unwrap());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionOutcome {
    pub action_type: String,
    pub ok: bool,
    pub summary: String,
    pub logs: Vec<String>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallResult {
    pub id: String,
    pub version: String,
    pub directory: String,
    pub replaced: bool,
}

/// Resolved isolated-UI target: which plugin page to open and which
/// directory the desktop asset server may serve it from.
#[derive(Debug, Clone)]
pub struct PluginUiTarget {
    pub id: String,
    pub name: String,
    pub directory: std::path::PathBuf,
    pub asset_root: std::path::PathBuf,
    pub entry_file: String,
    pub page_id: String,
}

/// Page-independent UI assets for the shared (multi-plugin) asset server.
#[derive(Debug, Clone)]
pub struct PluginUiAssets {
    pub id: String,
    pub name: String,
    pub directory: std::path::PathBuf,
    pub asset_root: std::path::PathBuf,
    pub entry: String,
    pub entry_file: String,
    pub page_ids: Vec<String>,
}

/// Mirrors the discovery-time `is_valid_ui_entry` rule so a tampered
/// manifest can never widen the servable directory at open time.
fn is_confined_ui_entry(entry: &str) -> bool {
    if entry.len() > 256 || !entry.starts_with("ui/") || !entry.ends_with(".html") {
        return false;
    }
    if entry.contains('\\') {
        return false;
    }
    let path = std::path::Path::new(entry);
    !path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
}

impl AppCore {
    /// Obtain before protocol admission. The returned guard must be released
    /// before any network request. Only cooperative callers sharing this core
    /// are fenced; filesystem writers and other processes remain separate.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn try_plugin_sync_runtime_guard(
        &self,
    ) -> Result<PluginSyncRuntimeGuard<'_>, OperationError> {
        let invocation =
            self.plugin_state.invocation_gate.try_lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        Ok(PluginSyncRuntimeGuard {
            core: self,
            _invocation: invocation,
        })
    }

    /// Native local enable/disable with an account-bound durable intent.
    /// The invocation gate spans the source transaction,
    /// runtime effect and confirmation, while the coordinator holds its
    /// shorter admission/identity lease only for the SQLite transaction.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn plugin_set_enabled_with_durable_intent(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<(), OperationError> {
        self.plugin_set_enabled_with_durable_intent_for(id, enabled, LocalMutationProtocol::V1)
    }

    /// V2 counterpart for the future product path. This remains private until
    /// every local writer and package mutation joins the same admission and
    /// runtime reconciliation protocol.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn plugin_set_enabled_with_v2_durable_intent(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<(), OperationError> {
        self.plugin_set_enabled_with_durable_intent_for(id, enabled, LocalMutationProtocol::V2)
    }

    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    fn plugin_set_enabled_with_durable_intent_for(
        &self,
        id: &str,
        enabled: bool,
        protocol: LocalMutationProtocol,
    ) -> Result<(), OperationError> {
        let invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        let sync = self
            .sync
            .coordinator()
            .map_err(|_| OperationError::unavailable("Cloud sync admission is unavailable"))?;
        // Only an opted-in identity needs entropy, acquired before any source
        // write; local-only editing must not depend on a Cloud runtime epoch.
        let (reservation, runtime_epoch) = reserve_local_plugin_mutation_prepared_for(
            sync,
            &self.db,
            &id,
            &plugin.manifest.version,
            true,
            enabled,
            protocol,
            |has_candidate| {
                if has_candidate {
                    self.sync.runtime_epoch().map(Some).map_err(|_| {
                        OperationError::unavailable("plugin runtime identity is unavailable")
                    })
                } else {
                    Ok(None)
                }
            },
        )?;

        // A failed or interrupted start/stop keeps the committed candidate
        // unconfirmed; it must be reconciled rather than uploaded or replayed.
        if enabled {
            self.plugins.start_generation_before_events(&id, || {
                self.set_plugin_activation(&id, true, true);
            })?;
        } else {
            self.plugins.stop(&id)?;
            self.set_plugin_activation(&id, true, false);
        }
        let confirmation = match runtime_epoch {
            Some(epoch) => confirm_local_plugin_mutation(&self.db, &reservation, epoch),
            None => Ok(()),
        };
        drop(invocation);

        // Even when confirmation fails, the local effect already happened.
        // Refresh local projections before returning that failure to callers.
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        self.events.publish_domain(if enabled {
            DomainEvent::PluginStarted {
                plugin_id: id.clone(),
            }
        } else {
            DomainEvent::PluginStopped {
                plugin_id: id.clone(),
            }
        });
        if reservation.legacy_staging {
            self.stage_selection(&id, &plugin.manifest.version, enabled);
        }
        confirmation
    }

    // ------------------------------------------------------------------
    // Plugins.
    // ------------------------------------------------------------------

    /// Safe plugin summaries. Never includes settings values.
    pub fn plugin_list(&self) -> Vec<Value> {
        let snapshot = self.load_merged_behavior_snapshot();
        snapshot
            .get("plugins")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    pub fn plugin_get(&self, id: &str) -> Result<Value, OperationError> {
        // Canonical existence check first so every plugin op shares one
        // not-found message.
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id;
        self.plugin_list()
            .into_iter()
            .find(|plugin| {
                plugin
                    .get("descriptor")
                    .and_then(|descriptor| descriptor.get("id"))
                    .and_then(Value::as_str)
                    == Some(id.as_str())
                    || plugin.get("id").and_then(Value::as_str) == Some(id.as_str())
            })
            .ok_or_else(|| OperationError::not_found(format!("Plugin `{id}` is not installed.")))
    }

    /// Resolves the isolated-UI target for a webview-mode plugin page.
    /// Fails when the plugin is missing, has no webview UI, the page is
    /// unknown, or the built entry asset is absent. The entry path is
    /// re-validated here (defense in depth: discovery already checked it)
    /// and the asset root is confined to the entry's own directory, so the
    /// desktop asset server can only serve built UI files — never the
    /// plugin manifest, settings, or backend.
    pub fn plugin_ui_target(
        &self,
        id: &str,
        page_id: &str,
    ) -> Result<PluginUiTarget, OperationError> {
        let assets = self.plugin_ui_assets(id)?;
        if !assets.page_ids.iter().any(|page| page == page_id) {
            return Err(OperationError::not_found(format!(
                "Plugin `{}` has no UI page `{page_id}`.",
                assets.id
            )));
        }
        let file = assets.directory.join(&assets.entry);
        if !file.is_file() {
            return Err(OperationError::unavailable(format!(
                "Plugin `{}` UI is not built (missing `{}`).",
                assets.id, assets.entry
            )));
        }
        Ok(PluginUiTarget {
            id: assets.id,
            name: assets.name,
            directory: assets.directory,
            asset_root: assets.asset_root,
            entry_file: assets.entry_file,
            page_id: page_id.to_owned(),
        })
    }

    /// Resolves a webview-mode plugin's servable UI assets without
    /// selecting a page. The main window's shared asset server calls this
    /// per request to confine inline plugin frames; unknown plugins,
    /// declarative plugins, and escaping entries fail before any file is
    /// touched. Missing build output is left to the file server, which
    /// 404s naturally per asset.
    pub fn plugin_ui_assets(&self, id: &str) -> Result<PluginUiAssets, OperationError> {
        let plugin = self.require_discovered(id)?;
        let manifest = &plugin.manifest;
        let ui = manifest.ui.as_ref().ok_or_else(|| {
            OperationError::unavailable(format!("Plugin `{}` has no custom UI.", manifest.id))
        })?;
        if ui.mode != tiktools_plugin_api::ui::PluginUiMode::Webview {
            return Err(OperationError::unavailable(format!(
                "Plugin `{}` uses declarative UI; no isolated webview.",
                manifest.id
            )));
        }
        let entry = ui.entry.as_deref().ok_or_else(|| {
            OperationError::unavailable(format!(
                "Plugin `{}` webview UI has no entry asset.",
                manifest.id
            ))
        })?;
        if !is_confined_ui_entry(entry) {
            return Err(OperationError::invalid(format!(
                "Plugin `{}` UI entry `{entry}` escapes the ui/ asset directory.",
                manifest.id
            )));
        }
        let file = plugin.directory.join(entry);
        let asset_root = file
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| plugin.directory.clone());
        let entry_file = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "index.html".to_owned());
        Ok(PluginUiAssets {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            directory: plugin.directory.clone(),
            asset_root,
            entry: entry.to_owned(),
            entry_file,
            page_ids: ui.pages.iter().map(|page| page.id.clone()).collect(),
        })
    }

    pub(crate) fn require_discovered(
        &self,
        id: &str,
    ) -> Result<tiktools_plugin_loader::DiscoveredPlugin, OperationError> {
        let id = clean_plugin_id(id)?;
        self.plugins
            .get(&id)
            .ok_or_else(|| OperationError::not_found(format!("Plugin `{id}` is not installed.")))
    }

    pub fn plugin_set_installed(&self, id: &str, installed: bool) -> Result<(), OperationError> {
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        #[cfg(feature = "cloud-auth")]
        let version = plugin.manifest.version.clone();
        #[cfg(feature = "persistence")]
        self.db
            .set_plugin_state(&id, installed, true)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        if !installed {
            // Stopping a non-running plugin is a no-op success; surface real
            // shutdown failures.
            self.plugins.stop(&id)?;
        }
        self.set_plugin_activation(&id, installed, true);
        drop(_invocation);
        if installed {
            self.start_enabled_event_subscribers();
        }
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        #[cfg(feature = "cloud-auth")]
        if installed {
            self.stage_selection(&id, &version, true);
        } else {
            self.stage_removal(&id);
        }
        Ok(())
    }

    /// Select the current protocol and reserve the source while holding the
    /// same short admission used by cutover. Runtime failure or an uncertain
    /// worker leaves the intent durable but unconfirmed for later review.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)] // Product selection remains fenced until every writer participates.
    pub(crate) fn plugin_set_installed_with_product_intent(
        &self,
        id: &str,
        installed: bool,
    ) -> Result<(), OperationError> {
        let invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        let sync = self
            .sync
            .coordinator()
            .map_err(|_| OperationError::unavailable("Cloud sync admission is unavailable"))?;
        let (reservation, runtime_epoch) = reserve_local_plugin_mutation_prepared_for(
            sync,
            &self.db,
            &id,
            &plugin.manifest.version,
            installed,
            true,
            LocalMutationProtocol::Selected,
            |has_candidate| {
                if has_candidate {
                    self.sync.runtime_epoch().map(Some).map_err(|_| {
                        OperationError::unavailable("plugin runtime identity is unavailable")
                    })
                } else {
                    Ok(None)
                }
            },
        )?;
        if !installed {
            self.plugins.stop(&id)?;
        }
        self.set_plugin_activation(&id, installed, true);
        let expected = if installed {
            tiktools_plugin_loader::PluginWorkerObservation::WorkerPresent
        } else {
            tiktools_plugin_loader::PluginWorkerObservation::Stopped
        };
        let confirmation = match runtime_epoch {
            Some(epoch)
                if self
                    .plugins
                    .observe_worker(&id)
                    .is_ok_and(|actual| actual == expected) =>
            {
                confirm_local_plugin_mutation(&self.db, &reservation, epoch)
            }
            Some(_) => {
                tracing::warn!(plugin = %id, "local plugin sync intent needs runtime reconciliation");
                Ok(())
            }
            None => Ok(()),
        };
        drop(invocation);
        if installed {
            self.start_enabled_event_subscribers();
        }
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        if reservation.legacy_staging {
            if installed {
                self.stage_selection(&id, &plugin.manifest.version, true);
            } else {
                self.stage_removal(&id);
            }
        }
        confirmation
    }

    pub fn plugin_set_enabled(&self, id: &str, enabled: bool) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        #[cfg(feature = "cloud-auth")]
        let version = plugin.manifest.version.clone();
        self.set_enabled_impl(&id, enabled, false)?;
        #[cfg(feature = "cloud-auth")]
        self.stage_selection(&id, &version, enabled);
        Ok(())
    }

    /// Product-selector prototype. Not reachable from product operations
    /// until network emission and local-only fallback share a durable fence.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn plugin_set_enabled_with_product_intent(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<(), OperationError> {
        self.plugin_set_enabled_with_durable_intent_for(
            id,
            enabled,
            LocalMutationProtocol::Selected,
        )
    }

    /// Prototype for local operation when Cloud admission cannot be opened.
    /// The marker and local source commit atomically while excluding a
    /// cooperating product emission. No Cloud identity is inferred here.
    /// This remains private until every product emitter uses this fence and
    /// a user-facing reconciliation flow exists.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    #[allow(dead_code)]
    pub(crate) fn plugin_set_enabled_with_untracked_intent(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<(), OperationError> {
        let invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id;
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        let fence = tiktools_cloud_sync::ProductEmissionFence::open(
            self.db.paths().data.join("plugin-sync-emission"),
        )
        .map_err(|_| OperationError::unavailable("sync emission fence unavailable"))?;
        let lease = fence
            .try_untracked_edit()
            .map_err(|_| OperationError::unavailable("sync emission is in progress"))?;
        self.db
            .set_plugin_state_with_untracked_sync(&id, true, enabled)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        drop(lease);
        if enabled {
            self.plugins.start_generation_before_events(&id, || {
                self.set_plugin_activation(&id, true, true);
            })?;
        } else {
            self.plugins.stop(&id)?;
            self.set_plugin_activation(&id, true, false);
        }
        drop(invocation);
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        self.events.publish_domain(if enabled {
            DomainEvent::PluginStarted { plugin_id: id }
        } else {
            DomainEvent::PluginStopped { plugin_id: id }
        });
        Ok(())
    }

    fn set_enabled_impl(
        &self,
        id: &str,
        enabled: bool,
        only_installed: bool,
    ) -> Result<bool, OperationError> {
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        if self.plugin_apply_quarantined(id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        if only_installed {
            // Check the installed state under the same gate as the mutation:
            // a concurrent uninstall must never be undone by a remote value.
            if self.require_discovered(id).is_err() {
                return Ok(false);
            }
            let activation =
                recover_rwlock_read(&self.plugin_state.activation, "plugin activation");
            if let Some(state) = activation.get(id) {
                if !state.installed || state.enabled == enabled {
                    return Ok(false);
                }
            } else if enabled {
                // Discovered plugins without an activation row default to
                // installed and enabled, matching plugin_ready().
                return Ok(false);
            }
        }
        #[cfg(feature = "persistence")]
        self.db
            .set_plugin_state(id, true, enabled)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let result = if enabled {
            self.plugins
                .start_generation_before_events(id, || {
                    self.set_plugin_activation(id, true, true);
                })
                .map(|_| ())
        } else {
            self.plugins.stop(id).map(|()| {
                self.set_plugin_activation(id, true, false);
            })
        };
        result?;
        drop(_invocation);
        self.rebuild_processor_index();
        self.refresh_automation_snapshot();
        self.events.publish_domain(if enabled {
            DomainEvent::PluginStarted {
                plugin_id: id.to_owned(),
            }
        } else {
            DomainEvent::PluginStopped {
                plugin_id: id.to_owned(),
            }
        });
        Ok(true)
    }

    /// Legacy low-level operation, retained only for existing compatibility
    /// tests. It does not bind account, consent, Cloud revision or crash
    /// recovery and must not be called from a product sync path.
    #[cfg(all(feature = "cloud-auth", test))]
    pub(crate) fn plugin_apply_remote_selection(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<bool, OperationError> {
        let plugin = match self.require_discovered(id) {
            Ok(plugin) => plugin,
            Err(_) => return Ok(false),
        };
        self.set_enabled_impl(&plugin.manifest.id, enabled, true)
    }

    /// Stage a local selection change for upload. Inert unless sync is
    /// enabled and an account is known; never fails the local operation.
    #[cfg(feature = "cloud-auth")]
    fn stage_selection(&self, id: &str, version: &str, enabled: bool) {
        let Ok(coordinator) = self.sync.coordinator() else {
            return;
        };
        if let Err(error) = coordinator.record_local_change(id, version, enabled) {
            tracing::warn!(plugin = %id, %error, "cloud sync staging failed");
        }
    }

    /// Stage a local uninstall for upload. Same opt-in and failure
    /// semantics as [`Self::stage_selection`].
    #[cfg(feature = "cloud-auth")]
    fn stage_removal(&self, id: &str) {
        let Ok(coordinator) = self.sync.coordinator() else {
            return;
        };
        if let Err(error) = coordinator.record_local_removal(id) {
            tracing::warn!(plugin = %id, %error, "cloud sync staging failed");
        }
    }

    pub fn plugin_start(&self, id: &str) -> Result<(), OperationError> {
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        if self.plugin_apply_quarantined(&id) {
            return Err(OperationError::unavailable(
                "plugin application needs recovery",
            ));
        }
        self.plugins.start(&id)?;
        drop(_invocation);
        self.events
            .publish_domain(DomainEvent::PluginStarted { plugin_id: id });
        Ok(())
    }

    pub fn plugin_stop(&self, id: &str) -> Result<(), OperationError> {
        let _invocation =
            self.plugin_state.invocation_gate.lock().map_err(|_| {
                OperationError::unavailable("plugin invocation lease is unavailable")
            })?;
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.plugins.stop(&id)?;
        drop(_invocation);
        self.events
            .publish_domain(DomainEvent::PluginStopped { plugin_id: id });
        Ok(())
    }

    #[cfg(feature = "plugin-install")]
    pub fn plugin_install(
        &self,
        archive: &str,
        replace_existing: bool,
    ) -> Result<PluginInstallResult, OperationError> {
        let trimmed = archive.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 {
            return Err(OperationError::invalid(
                "plugin archive path must be 1..=4096 characters",
            ));
        }
        let outcome = self
            .install_plugin(std::path::Path::new(trimmed), replace_existing)
            .map_err(OperationError::install)?;
        self.after_package_change(&outcome);
        Ok(PluginInstallResult {
            id: outcome.receipt.id,
            version: outcome.receipt.version,
            directory: outcome.directory,
            replaced: outcome.previous_version.is_some(),
        })
    }

    #[cfg(not(feature = "plugin-install"))]
    pub fn plugin_install(
        &self,
        _archive: &str,
        _replace_existing: bool,
    ) -> Result<PluginInstallResult, OperationError> {
        Err(OperationError::unavailable(
            "plugin installation was disabled in this build",
        ))
    }

    #[cfg(feature = "plugin-install")]
    pub fn plugin_uninstall(&self, id: &str, purge_data: bool) -> Result<(), OperationError> {
        let plugin = self.require_discovered(id)?;
        let id = plugin.manifest.id.clone();
        self.uninstall_plugin(&id, purge_data)
            .map_err(OperationError::install)?;
        self.refresh_automation_snapshot();
        self.events
            .publish_domain(DomainEvent::PluginUninstalled { plugin_id: id });
        Ok(())
    }

    #[cfg(not(feature = "plugin-install"))]
    pub fn plugin_uninstall(&self, _id: &str, _purge_data: bool) -> Result<(), OperationError> {
        Err(OperationError::unavailable(
            "plugin installation was disabled in this build",
        ))
    }

    /// Resolves `action-type/field` option documents for action configs.
    /// Returns the items plus the server-reported selection, if any.
    /// `refresh` bypasses the option cache for one read (manual Refresh,
    /// post-mutation re-read) without disabling caching globally.
    pub async fn plugin_action_options(
        self: &Arc<Self>,
        source: &str,
        refresh: bool,
    ) -> Result<(Vec<Value>, Option<String>), OperationError> {
        let source = source.trim();
        if source.is_empty() || source.len() > 256 {
            return Err(OperationError::invalid(
                "option source must be 1..=256 characters",
            ));
        }
        let (options, selected, error) = self.resolve_action_options(source, refresh).await;
        if let Some(error) = error {
            return Err(OperationError::internal(error));
        }
        Ok((options, selected))
    }

    /// Scoped option read for untrusted callers (plugin UI broker): the
    /// requesting plugin may only read option sources owned by its own
    /// actions. Ownership resolves through the action catalog, never by
    /// trusting name prefixes. Host-owned sources (`host.gifts`) belong to
    /// no plugin and are readable by every caller.
    pub async fn plugin_action_options_scoped(
        self: &Arc<Self>,
        requesting_plugin: &str,
        source: &str,
        refresh: bool,
    ) -> Result<(Vec<Value>, Option<String>), OperationError> {
        let requesting = clean_plugin_id(requesting_plugin)?;
        let source = source.trim();
        if source.is_empty() || source.len() > 256 {
            return Err(OperationError::invalid(
                "option source must be 1..=256 characters",
            ));
        }
        if crate::services::option_sources::is_host_option_source(source) {
            return self.plugin_action_options(source, refresh).await;
        }
        let Some((action_type, _)) = crate::services::option_sources::parse_option_source(source)
        else {
            return Err(OperationError::invalid(format!(
                "Unknown option source `{source}`."
            )));
        };
        self.verify_action_owner(&action_type, &requesting)?;
        self.plugin_action_options(source, refresh).await
    }

    /// Resolves the owning plugin of an action type from the action catalog
    /// and rejects callers outside that scope. Unknown actions report
    /// `not_found` (not `forbidden`) so scanners learn nothing about which
    /// plugin owns what.
    pub(crate) fn verify_action_owner(
        &self,
        action_type: &str,
        requesting_plugin: &str,
    ) -> Result<(), OperationError> {
        let Some((plugin, _)) = self.plugin_for_action(action_type) else {
            return Err(OperationError::not_found(format!(
                "Action type `{action_type}` is not available in this host."
            )));
        };
        if plugin.manifest.id != requesting_plugin {
            return Err(OperationError::forbidden(format!(
                "Action type `{action_type}` belongs to plugin `{}`.",
                plugin.manifest.id
            )));
        }
        Ok(())
    }

    /// Scoped action execution for untrusted callers (plugin UI broker):
    /// the requesting plugin may only execute its own actions. Ownership
    /// resolves through the action catalog, never by trusting name prefixes.
    pub async fn plugin_action_execute_scoped(
        self: &Arc<Self>,
        requesting_plugin: &str,
        action_type: &str,
        config: BTreeMap<String, Value>,
        live: bool,
    ) -> Result<PluginActionOutcome, OperationError> {
        let requesting = clean_plugin_id(requesting_plugin)?;
        let action_type = action_type.trim();
        if action_type.is_empty() || action_type.len() > 128 || !is_identifier(action_type) {
            return Err(OperationError::invalid(
                "action type is not a valid identifier",
            ));
        }
        self.verify_action_owner(action_type, &requesting)?;
        self.plugin_action_execute(action_type, config, live).await
    }

    /// Executes one plugin action. `live = false` is a dry run (no HTTP is
    /// sent, no side effects); `live = true` performs the real execution.
    pub async fn plugin_action_execute(
        self: &Arc<Self>,
        action_type: &str,
        config: BTreeMap<String, Value>,
        live: bool,
    ) -> Result<PluginActionOutcome, OperationError> {
        let action_type = action_type.trim();
        if action_type.is_empty() || action_type.len() > 128 || !is_identifier(action_type) {
            return Err(OperationError::invalid(
                "action type is not a valid identifier",
            ));
        }
        if config.len() > 64 {
            return Err(OperationError::invalid("too many config values (max 64)"));
        }
        // Actions declaring a `text` field (TTS speak) require bounded
        // spoken text, mirroring the WebView voice-tester path.
        if self.action_declares_field(action_type, "text") {
            let text = config
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if text.trim().is_empty() {
                return Err(OperationError::invalid(
                    "config.text must not be empty for this action type",
                ));
            }
            if text.len() > 4_096 {
                return Err(OperationError::invalid(
                    "Text is too long (4,096 character limit).",
                ));
            }
        }
        let started = now_millis();
        let action = json!({
            "typeId": action_type,
            "config": Value::Object(config.into_iter().collect()),
        });
        let event = json!({
            "id": format!("control-{}", started),
            "type": "control.execute",
            "timestamp": started,
            "data": {},
        });
        let mut logs = Vec::new();
        match self
            .execute_plugin_action(
                action_type,
                &action,
                &event,
                &crate::automation_runtime::context::ExecutionContext::owner(),
                &mut logs,
                !live,
            )
            .await
        {
            Ok(summary) => {
                // A live execution may have changed server state (audio
                // output switches, renames): drop this action's cached
                // option lists so the follow-up re-read reports the new
                // server selection instead of the pre-mutation one. Dry
                // runs send nothing, so their cache entries stay valid.
                if live {
                    self.option_sources.invalidate_action(action_type);
                }
                Ok(PluginActionOutcome {
                    action_type: action_type.to_owned(),
                    ok: true,
                    summary,
                    logs: logs.into_iter().take(20).collect(),
                    duration_ms: now_millis().saturating_sub(started),
                    error: None,
                })
            }
            Err(error) => Ok(PluginActionOutcome {
                action_type: action_type.to_owned(),
                ok: false,
                summary: error.clone(),
                logs: logs.into_iter().take(20).collect(),
                duration_ms: now_millis().saturating_sub(started),
                error: Some(error),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tiktools_plugin_loader::{PluginManager, PluginRoot, PluginSource};

    struct NullEmitter;

    impl crate::HostEmitter for NullEmitter {
        fn emit(&self, _message: crate::HostMessage) {}
    }

    fn manifest(id: &str, actions: &[&str]) -> String {
        let actions = actions
            .iter()
            .map(|action| {
                format!(
                    r#"{{"id": "{action}", "title": {{"default": "{action}"}}, "tag": "test", "requiredCapabilities": []}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"schemaVersion": 3, "id": "{id}", "name": "{id}", "version": "1.0.0", "runtime": "declarative", "capabilities": [], "permissions": [], "actionTypes": [{actions}]}}"#
        )
    }

    /// Builds an AppCore whose catalog holds two declarative plugins. The
    /// actions declare no `http` block, so dry runs resolve and return
    /// without touching any runtime or network.
    fn core_with_two_plugins() -> (Arc<AppCore>, std::path::PathBuf) {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "tiktools-ownership-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        // plugin-b declares `plugina.spoof`: the action NAME looks like
        // plugin-a's, but the catalog owner is plugin-b. Ownership must
        // resolve through the catalog, never by prefix.
        for (dir, id, actions) in [
            ("plugin-a", "plugina", &["plugina.echo"][..]),
            (
                "plugin-b",
                "pluginb",
                &["pluginb.echo", "plugina.spoof"][..],
            ),
        ] {
            let dir = root.join(dir);
            std::fs::create_dir_all(&dir).expect("test plugin dir");
            std::fs::write(dir.join("plugin.json"), manifest(id, actions)).expect("manifest");
        }
        let manager = PluginManager::new(vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }]);
        manager.scan().expect("test scan");
        assert_eq!(manager.list().len(), 2);
        let mut core = AppCore::new(Arc::new(NullEmitter));
        core.plugins = Arc::new(manager);
        (Arc::new(core), root)
    }

    fn cleanup(root: &std::path::Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn scoped_execute_allows_own_actions() {
        let (core, root) = core_with_two_plugins();
        let outcome = core
            .plugin_action_execute_scoped("plugina", "plugina.echo", BTreeMap::new(), false)
            .await
            .expect("own action executes");
        assert!(outcome.ok, "dry run should succeed: {outcome:?}");
        cleanup(&root);
    }

    #[tokio::test]
    async fn scoped_execute_rejects_cross_plugin_actions() {
        let (core, root) = core_with_two_plugins();
        // plugin-a cannot invoke plugin-b's action.
        let error = core
            .plugin_action_execute_scoped("plugina", "pluginb.echo", BTreeMap::new(), false)
            .await
            .expect_err("cross-plugin execute must fail");
        assert_eq!(error.code(), "forbidden");
        // Prefix spoofing buys nothing: `plugina.spoof` is owned by
        // plugin-b even though the name starts with plugin-a's id.
        let error = core
            .plugin_action_execute_scoped("plugina", "plugina.spoof", BTreeMap::new(), false)
            .await
            .expect_err("prefix-spoofed execute must fail");
        assert_eq!(error.code(), "forbidden");
        // Unknown actions report not_found, not forbidden.
        let error = core
            .plugin_action_execute_scoped("plugina", "nobody.there", BTreeMap::new(), false)
            .await
            .expect_err("unknown action must fail");
        assert_eq!(error.code(), "not_found");
        cleanup(&root);
    }

    #[tokio::test]
    async fn scoped_options_rejects_cross_plugin_sources() {
        let (core, root) = core_with_two_plugins();
        // plugin-a cannot read plugin-b's option source.
        let error = core
            .plugin_action_options_scoped(
                "plugina",
                "plugin-action-options:pluginb.echo:field",
                false,
            )
            .await
            .expect_err("cross-plugin options read must fail");
        assert_eq!(error.code(), "forbidden");
        // Prefix-spoofed source resolves to plugin-b and is rejected.
        let error = core
            .plugin_action_options_scoped(
                "plugina",
                "plugin-action-options:plugina.spoof:field",
                false,
            )
            .await
            .expect_err("prefix-spoofed options read must fail");
        assert_eq!(error.code(), "forbidden");
        // Own source passes ownership: it proceeds to resolution, which
        // reports the (declared-shape) absence of options — never forbidden.
        let error = core
            .plugin_action_options_scoped(
                "plugina",
                "plugin-action-options:plugina.echo:field",
                false,
            )
            .await
            .expect_err("undeclared options must fail resolution");
        assert_ne!(error.code(), "forbidden");
        cleanup(&root);
    }

    /// Stages the real Minecraft example bytes (not an inline fixture)
    /// so discovery runs against exactly what ships in `examples/`.
    fn core_with_minecraft_example() -> (Arc<AppCore>, std::path::PathBuf) {
        static COUNTER: AtomicU64 = AtomicU64::new(2000);
        let root = std::env::temp_dir().join(format!(
            "tiktools-minecraft-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let dir = root.join("minecraft.server");
        std::fs::create_dir_all(&dir).expect("test plugin dir");
        std::fs::write(
            dir.join("plugin.json"),
            include_str!("../../../../tests/fixtures/plugins/minecraft-server.plugin.json"),
        )
        .expect("manifest");
        let manager = PluginManager::new(vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }]);
        manager.scan().expect("test scan");
        assert_eq!(manager.list().len(), 1);
        let mut core = AppCore::new(Arc::new(NullEmitter));
        core.plugins = Arc::new(manager);
        (Arc::new(core), root)
    }

    #[test]
    fn minecraft_example_merges_into_behavior_snapshot() {
        let (core, root) = core_with_minecraft_example();
        let snapshot = core.load_merged_behavior_snapshot();
        let action_types = snapshot
            .get("actionTypes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let minecraft: Vec<&Value> = action_types
            .iter()
            .filter(|descriptor| {
                descriptor
                    .get("source")
                    .and_then(|source| source.get("pluginId"))
                    .and_then(Value::as_str)
                    == Some("minecraft.server")
            })
            .collect();
        // The six structured actions surface as picker cards with their
        // presentation metadata and plugin origin stamp intact.
        assert_eq!(minecraft.len(), 6);
        for descriptor in minecraft {
            assert_eq!(
                descriptor.get("source"),
                Some(&serde_json::json!({"kind": "plugin", "pluginId": "minecraft.server"}))
            );
            assert!(descriptor
                .get("icon")
                .and_then(Value::as_str)
                .is_some_and(|icon| !icon.is_empty()));
            assert!(descriptor
                .get("category")
                .and_then(Value::as_str)
                .is_some_and(|category| !category.is_empty()));
            assert!(descriptor
                .get("fields")
                .and_then(Value::as_array)
                .is_some_and(|fields| !fields.is_empty()));
        }
        cleanup(&root);
    }

    #[tokio::test]
    async fn scoped_options_allow_host_sources_for_any_plugin() {
        let (core, root) = core_with_two_plugins();
        // Host-owned data belongs to no plugin: either caller may read it.
        for requesting in ["plugina", "pluginb"] {
            let (options, selected) = core
                .plugin_action_options_scoped(requesting, "host.gifts", false)
                .await
                .expect("host source must resolve for any plugin");
            assert!(selected.is_none());
            assert_eq!(
                options,
                crate::services::option_sources::map_host_gift_options(&core.gift_catalog())
            );
        }
        cleanup(&root);
    }

    fn webview_manifest(id: &str) -> String {
        format!(
            r#"{{"schemaVersion": 3, "id": "{id}", "name": "{id}", "version": "1.0.0", "runtime": "declarative", "capabilities": [], "permissions": [], "actionTypes": [], "ui": {{"apiVersion": 1, "mode": "webview", "entry": "ui/dist/index.html", "pages": [{{"id": "main", "title": {{"default": "Main"}}}}]}}}}"#
        )
    }

    fn core_with_webview_plugin() -> (Arc<AppCore>, std::path::PathBuf) {
        static COUNTER: AtomicU64 = AtomicU64::new(1000);
        let root = std::env::temp_dir().join(format!(
            "tiktools-uitarget-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let dir = root.join("webui");
        std::fs::create_dir_all(dir.join("ui/dist")).expect("test ui dir");
        std::fs::write(dir.join("plugin.json"), webview_manifest("webui")).expect("manifest");
        std::fs::write(dir.join("ui/dist/index.html"), "<html></html>").expect("entry");
        let manager = PluginManager::new(vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }]);
        manager.scan().expect("test scan");
        let mut core = AppCore::new(Arc::new(NullEmitter));
        core.plugins = Arc::new(manager);
        (Arc::new(core), root)
    }

    #[test]
    fn ui_target_resolves_entry_and_confines_asset_root() {
        let (core, root) = core_with_webview_plugin();
        let target = core.plugin_ui_target("webui", "main").expect("ui target");
        assert_eq!(target.id, "webui");
        assert_eq!(target.asset_root, root.join("webui/ui/dist"));
        // Unknown page and unknown plugin fail; nothing resolves.
        assert_eq!(
            core.plugin_ui_target("webui", "nope")
                .expect_err("unknown page must fail")
                .code(),
            "not_found"
        );
        assert_eq!(
            core.plugin_ui_target("missing", "main")
                .expect_err("unknown plugin must fail")
                .code(),
            "not_found"
        );
        // An unbuilt UI (entry asset absent) is unavailable, not servable.
        std::fs::remove_file(root.join("webui/ui/dist/index.html")).expect("remove entry");
        assert_eq!(
            core.plugin_ui_target("webui", "main")
                .expect_err("missing entry must fail")
                .code(),
            "unavailable"
        );
        cleanup(&root);
    }

    #[test]
    fn ui_assets_resolve_without_selecting_a_page() {
        let (core, root) = core_with_webview_plugin();
        let assets = core.plugin_ui_assets("webui").expect("ui assets");
        assert_eq!(assets.asset_root, root.join("webui/ui/dist"));
        assert_eq!(assets.entry_file, "index.html");
        assert_eq!(assets.page_ids, vec!["main".to_owned()]);
        // The shared asset server rejects declarative plugins before
        // touching the filesystem.
        let (legacy, legacy_root) = core_with_two_plugins();
        assert_eq!(
            legacy
                .plugin_ui_assets("plugina")
                .expect_err("declarative must fail")
                .code(),
            "unavailable"
        );
        cleanup(&root);
        cleanup(&legacy_root);
    }

    #[test]
    fn ui_target_rejects_declarative_plugins() {
        let (core, root) = core_with_two_plugins();
        let error = core
            .plugin_ui_target("plugina", "main")
            .expect_err("declarative plugin must fail");
        assert_eq!(error.code(), "unavailable");
        cleanup(&root);
    }

    #[test]
    fn confined_ui_entry_blocks_escape() {
        assert!(is_confined_ui_entry("ui/dist/index.html"));
        assert!(!is_confined_ui_entry("../plugin.json"));
        assert!(!is_confined_ui_entry("ui/../../secret.html"));
        assert!(!is_confined_ui_entry("/etc/ui.html"));
        assert!(!is_confined_ui_entry("ui/dist/app.js"));
        assert!(!is_confined_ui_entry("ui\\dist\\index.html"));
        assert!(!is_confined_ui_entry("assets/index.html"));
    }
}
