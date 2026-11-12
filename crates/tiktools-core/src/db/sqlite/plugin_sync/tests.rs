mod admission;
mod outbox;
mod receipts;
mod source;
mod untracked;

use super::*;
use crate::paths::AppPaths;
use rusqlite::Connection;

struct Fixture {
    db: DatabaseManager,
    root: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-plugin-sync-{}-{}",
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
        for file in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            drop(Connection::open(file).unwrap());
        }
        Self {
            db: DatabaseManager::new(paths),
            root,
        }
    }
    fn intent(&self, account: &'static str) -> LocalPluginSyncIntent<'static> {
        LocalPluginSyncIntent {
            origin: "https://cloud.example",
            account_id: account,
            config_generation: 7,
            version: Some("1.0.0"),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
