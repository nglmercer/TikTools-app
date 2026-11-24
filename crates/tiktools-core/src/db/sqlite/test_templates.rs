//! Pre-initialized database templates for tests. Running the full schema and
//! migration pass costs ~1s of fsync-bound DDL per fresh fixture; copying
//! finished database files costs milliseconds and yields the same state a
//! first `DatabaseManager::new` would produce (schema plus seed rows, no
//! user data). Tests that exercise first-run or migration behavior must keep
//! building truly empty databases instead.
#![cfg(test)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rusqlite::Connection;

use crate::db::DatabaseManager;
use crate::paths::AppPaths;

struct Templates {
    points: PathBuf,
    automation: PathBuf,
    analytics: PathBuf,
}

static TEMPLATES: OnceLock<Templates> = OnceLock::new();

fn build_templates() -> Templates {
    let dir = std::env::temp_dir().join(format!("tiktools-db-templates-{}", std::process::id()));
    let paths = AppPaths {
        data: dir.join("data"),
        plugins: dir.join("plugins"),
        plugin_data: dir.join("plugin-data"),
        builtin_plugins: dir.join("absent"),
        development_plugins: None,
        logs: dir.join("logs"),
        temp: dir.join("temp"),
        root: dir.clone(),
    };
    paths.ensure_directories().unwrap();
    let db = DatabaseManager::new(paths.clone());
    drop(db);
    // Fold any WAL sidecars back into the main files so a plain file copy is
    // a complete database.
    for path in [
        paths.points_database(),
        paths.automation_database(),
        paths.analytics_database(),
    ] {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
    }
    Templates {
        points: paths.points_database(),
        automation: paths.automation_database(),
        analytics: paths.analytics_database(),
    }
}

/// Returns `AppPaths` rooted at `root` whose three database files are copies
/// of the shared initialized templates. The caller still runs
/// `DatabaseManager::new`/`AppCore::with_paths`, which now take the cheap
/// already-initialized path (~50ms instead of ~1.2s of DDL commits).
pub(crate) fn materialize(root: &Path) -> AppPaths {
    let templates = TEMPLATES.get_or_init(build_templates);
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.to_path_buf(),
    };
    paths.ensure_directories().unwrap();
    for (template, target) in [
        (&templates.points, paths.points_database()),
        (&templates.automation, paths.automation_database()),
        (&templates.analytics, paths.analytics_database()),
    ] {
        std::fs::copy(template, target).unwrap();
    }
    paths
}
