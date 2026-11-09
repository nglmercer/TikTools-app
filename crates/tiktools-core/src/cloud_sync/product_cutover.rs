//! Private, informational cutover preview from the native plugin source.
//!
//! This is not authorization to migrate. Package filesystem I/O, independent
//! processes, account invalidation and final cutover still need one shared
//! admission boundary before any RPC or product activation can use it.
use std::{collections::BTreeMap, sync::Arc};

use cloud_client::{PluginSelection, Session};
use tiktools_cloud_sync::{CutoverPreview, SyncConfigEpoch};

use crate::{control::OperationError, db::PluginCutoverSource, AppCore};

#[derive(PartialEq, Eq)]
struct LocalSource {
    sqlite: PluginCutoverSource,
    catalog_revision: u64,
    selections: Vec<PluginSelection>,
}

/// Native-only state: private account data is never serialized to a caller.
pub(crate) struct NativeCutoverPreview {
    preview: CutoverPreview,
    source: LocalSource,
    session: Arc<Session>,
    consent: SyncConfigEpoch,
}

fn unavailable() -> OperationError {
    OperationError::unavailable("local plugin source is unavailable for Cloud migration review")
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
            // Missing state follows the same installed/enabled defaults as the
            // runtime catalog. A retained deleted row does not override them.
            let (installed, enabled) = states
                .get(id.as_str())
                .copied()
                .flatten()
                .unwrap_or((true, true));
            let (runtime_installed, runtime_enabled) = activation
                .get(&id)
                .map(|state| (state.installed, state.enabled))
                .unwrap_or((true, true));
            if (installed, enabled) != (runtime_installed, runtime_enabled) {
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
        // Do not silently omit an installed plugin merely because its package
        // cannot currently be discovered or its manifest cannot be read.
        if activation
            .iter()
            .any(|(id, state)| state.installed && !catalog.contains(id))
            || sqlite.rows.iter().any(|row| {
                row.state.is_some_and(|(installed, _)| installed) && !catalog.contains(&row.id)
            })
            || self.plugins.registry_revision() != revision
            || self
                .db
                .plugin_cutover_source(origin, account)
                .map_err(|_| unavailable())?
                != sqlite
        {
            return Err(unavailable());
        }
        Ok(LocalSource {
            sqlite,
            catalog_revision: revision,
            selections,
        })
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
        let preview = coordinator
            .preview_v2_cutover(&session, &source.selections)
            .map_err(|_| unavailable())?;
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
            preview,
            source,
            session,
            consent,
        })
    }
}

impl NativeCutoverPreview {
    pub(crate) fn preview(&self) -> &CutoverPreview {
        &self.preview
    }

    /// Only checks local source, session and consent for an informational
    /// preview. It does not recheck v1 state or authorize cutover; mutations
    /// can race after this method returns.
    pub(crate) fn check_local_fresh(&self, core: &AppCore) -> Result<(), OperationError> {
        let coordinator = core.sync.coordinator().map_err(|_| unavailable())?;
        let origin = self.session.instance_origin();
        let account = self.session.account_id();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{paths::AppPaths, HostEmitter, HostMessage, NoopMediaHost};

    struct Emitter;
    impl HostEmitter for Emitter {
        fn emit(&self, _: HostMessage) {}
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
}
