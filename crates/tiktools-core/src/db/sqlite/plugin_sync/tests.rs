mod admission;
mod outbox;
mod receipts;
mod source;
mod untracked;

use super::*;

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
        let paths = crate::db::test_templates::materialize(&root);
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
