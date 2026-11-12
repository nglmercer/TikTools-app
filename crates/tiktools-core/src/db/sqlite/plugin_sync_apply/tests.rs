mod admission;
mod fencing;
mod finalization;
mod recovery;
mod transitions;

use super::*;
use crate::paths::AppPaths;
use std::path::PathBuf;

struct Fixture {
    db: DatabaseManager,
    root: PathBuf,
    context: V1LocalEditContext,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "plugin-apply-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
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
        let db = DatabaseManager::new(paths);
        assert_eq!(
            db.set_plugin_state_with_sync_intent("acme.plugin", true, true, None)
                .unwrap(),
            1
        );
        Self {
            db,
            root,
            context: V1LocalEditContext {
                origin: "https://cloud.example".into(),
                account_id: "alice".into(),
                config_generation: 9,
            },
        }
    }
    fn intent(&self) -> PluginApplyIntent {
        PluginApplyIntent {
            intent_id: "a".repeat(64),
            origin: self.context.origin.clone(),
            account_id: self.context.account_id.clone(),
            config_generation: self.context.config_generation,
            plugin_id: "acme.plugin".into(),
            source_generation: 1,
            installed_version: "1.0.0".into(),
            expected_enabled: true,
            reviewed_pending: Some(V1Pending {
                base_revision: None,
                edit: V1Edit::Put(PluginSelection {
                    schema_version: 1,
                    plugin_id: "acme.plugin".into(),
                    version: "1.0.0".into(),
                    enabled: true,
                }),
                conflict: false,
            }),
            base_record: None,
            remote: None,
            reason: PluginApplyReason::ReviewedUncertain,
        }
    }
    fn state(&self) -> (i64, i64, i64) {
        let db = self.db.open(&self.db.automation_path()).unwrap();
        db.query_row("SELECT s.generation,b.installed,b.enabled FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id WHERE s.plugin_id='acme.plugin'", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
