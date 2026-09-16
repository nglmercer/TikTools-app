//! Database ownership boundary.
//!
//! The Rust host reserves the existing filenames and paths in every build.
//! The `persistence` feature adds rusqlite schema initialization and the
//! conservative record helpers used by the desktop binary.

#[cfg(feature = "persistence")]
mod sqlite;

#[cfg(feature = "persistence")]
mod analytics;

#[cfg(feature = "persistence")]
pub use sqlite::DatabaseError;

#[cfg(feature = "persistence")]
pub use analytics::{
    utc_day, AnalyticsDayRow, AnalyticsEventRecord, AnalyticsSummaryData, AnalyticsTopViewer,
    AnalyticsTotals,
};

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::paths::AppPaths;

#[derive(Debug, Clone)]
pub struct DatabaseManager {
    paths: AppPaths,
}

impl DatabaseManager {
    pub fn new(paths: AppPaths) -> Self {
        migrate_legacy_databases(&paths);
        let manager = Self { paths };
        #[cfg(feature = "persistence")]
        if let Err(error) = manager.initialize_schema() {
            tracing::warn!(%error, "could not initialize Rust SQLite schemas");
        }
        manager
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn points_path(&self) -> std::path::PathBuf {
        self.paths.points_database()
    }

    pub fn automation_path(&self) -> std::path::PathBuf {
        self.paths.automation_database()
    }

    pub fn analytics_path(&self) -> std::path::PathBuf {
        self.paths.analytics_database()
    }
}

/// Preserve the current TypeScript migration behavior: a legacy checkout may
/// still have `./data/*.db`, while a compiled Rust application writes under
/// the platform app-data directory. Existing destination files are never
/// overwritten.
fn migrate_legacy_databases(paths: &AppPaths) {
    for file_name in ["tiktok-points.db", "tiktok-automation.db"] {
        let legacy = PathBuf::from("data").join(file_name);
        let target = paths.data.join(file_name);
        if same_path(&legacy, &target) || target.exists() || !legacy.is_file() {
            continue;
        }
        if let Err(error) =
            fs::create_dir_all(&paths.data).and_then(|_| fs::copy(&legacy, &target).map(|_| ()))
        {
            tracing::warn!(%error, source = %legacy.display(), destination = %target.display(), "could not migrate legacy database");
            continue;
        }
        for suffix in ["-wal", "-shm"] {
            let legacy_sidecar = PathBuf::from(format!("{}{}", legacy.display(), suffix));
            let target_sidecar = PathBuf::from(format!("{}{}", target.display(), suffix));
            if legacy_sidecar.is_file() && !target_sidecar.exists() {
                if let Err(error) = fs::copy(&legacy_sidecar, &target_sidecar) {
                    tracing::warn!(%error, source = %legacy_sidecar.display(), destination = %target_sidecar.display(), "could not migrate SQLite sidecar");
                }
            }
        }
        tracing::info!(source = %legacy.display(), destination = %target.display(), "migrated legacy database");
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    if cfg!(target_os = "windows") {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}
