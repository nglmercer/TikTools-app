//! Pre-initialized database templates for tests. Running the full schema and
//! migration pass costs ~1s of DDL per fresh fixture; copying finished
//! database files costs milliseconds and yields the same state a first
//! `DatabaseManager::new` would produce (schema plus seed rows, no user
//! data). Tests that exercise first-run or migration behavior must keep
//! building truly empty databases instead.
//!
//! Templates are shared across every test process of the same test binary:
//! the directory is keyed by the executable's file name, which cargo's
//! metadata hash rotates on every rebuild, so a template can never outlive
//! the code that produced it. Concurrent builders coordinate through a
//! `.ready` marker; losers wait briefly, then fall back to a private build.
#![cfg(test)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use rusqlite::Connection;

use crate::db::DatabaseManager;
use crate::paths::AppPaths;

struct Templates {
    points: PathBuf,
    automation: PathBuf,
    analytics: PathBuf,
}

impl Templates {
    fn copy_into(&self, paths: &AppPaths) {
        for (template, target) in [
            (&self.points, paths.points_database()),
            (&self.automation, paths.automation_database()),
            (&self.analytics, paths.analytics_database()),
        ] {
            // Never clobber: reopens, crash children, and fixtures that
            // pre-create empty database files keep their own state.
            if !target.exists() {
                let _ = std::fs::copy(template, target);
            }
        }
    }
}

/// Per-process fallback used only when the shared template cannot be built
/// or waited for (for example a crashed builder early in the run).
static PRIVATE_TEMPLATES: OnceLock<Templates> = OnceLock::new();
static SHARED: OnceLock<Option<Templates>> = OnceLock::new();

fn private_templates() -> &'static Templates {
    PRIVATE_TEMPLATES.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("tiktools-db-templates-{}", std::process::id()));
        let paths = fixture_paths(&dir);
        paths.ensure_directories().unwrap();
        let db = DatabaseManager::new(paths.clone());
        drop(db);
        checkpoint(&paths);
        templates_at(&dir)
    })
}

fn fixture_paths(root: &Path) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.to_path_buf(),
    }
}

fn templates_at(dir: &Path) -> Templates {
    let paths = fixture_paths(dir);
    Templates {
        points: paths.points_database(),
        automation: paths.automation_database(),
        analytics: paths.analytics_database(),
    }
}

fn checkpoint(paths: &AppPaths) {
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
}

/// Builds the three databases inside `dir` and publishes the `.ready`
/// marker. Returns `None` (and cleans up) on any failure so waiters are not
/// stalled by a half-written directory.
fn build_shared(dir: &Path) -> Option<Templates> {
    let paths = fixture_paths(dir);
    paths.ensure_directories().ok()?;
    let db = DatabaseManager::new(paths.clone());
    drop(db);
    checkpoint(&paths);
    std::fs::write(dir.join(".ready"), b"ok").ok()?;
    Some(templates_at(dir))
}

fn shared_dir() -> PathBuf {
    // `current_exe` carries cargo's metadata hash: any source change yields
    // a different test binary name, so templates can never serve a stale
    // schema to a newer build.
    let key = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned());
    std::env::temp_dir().join(format!("tiktools-shared-db-templates-{key}"))
}

/// Returns the process-shared initialized templates, building them once per
/// test binary. Returns `None` when the shared build is unavailable; callers
/// then fall back to the per-process path.
fn shared_templates() -> Option<&'static Templates> {
    SHARED
        .get_or_init(|| {
            let dir = shared_dir();
            let ready = dir.join(".ready");
            if ready.exists() {
                return Some(templates_at(&dir));
            }
            match std::fs::create_dir(&dir) {
                Ok(()) => build_shared(&dir).or_else(|| {
                    let _ = std::fs::remove_dir_all(&dir);
                    None
                }),
                Err(_) => {
                    // Another process is building, or a stale directory
                    // survived a crashed builder. Directories older than
                    // ten minutes cannot belong to a live build (~1s), so
                    // reap and take over once.
                    let stale = std::fs::metadata(&dir)
                        .and_then(|meta| meta.modified())
                        .map(|modified| {
                            modified.elapsed().unwrap_or_default() > Duration::from_secs(600)
                        })
                        .unwrap_or(false);
                    if stale {
                        let _ = std::fs::remove_dir_all(&dir);
                        if std::fs::create_dir(&dir).is_ok() {
                            return build_shared(&dir).or_else(|| {
                                let _ = std::fs::remove_dir_all(&dir);
                                None
                            });
                        }
                        return None;
                    }
                    // Wait for the builder's marker; the build takes ~1s.
                    let deadline = Instant::now() + Duration::from_secs(30);
                    while Instant::now() < deadline {
                        if ready.exists() {
                            return Some(templates_at(&dir));
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    None
                }
            }
        })
        .as_ref()
}

/// Best-effort priming for `AppPaths::from_environment` homes: fills in the
/// three database files when they are absent so `DatabaseManager::new` takes
/// the already-initialized path. Existing files (reopens, crash children,
/// fixtures that pre-create empty databases) are never touched.
pub(crate) fn prime(paths: &AppPaths) {
    let templates = match shared_templates() {
        Some(templates) => templates,
        None => private_templates(),
    };
    if std::fs::create_dir_all(&paths.data).is_err() {
        return;
    }
    templates.copy_into(paths);
}

/// Returns `AppPaths` rooted at `root` whose three database files are copies
/// of the initialized templates. The caller still runs
/// `DatabaseManager::new`/`AppCore::with_paths`, which now take the cheap
/// already-initialized path.
pub(crate) fn materialize(root: &Path) -> AppPaths {
    let paths = fixture_paths(root);
    paths.ensure_directories().unwrap();
    match shared_templates() {
        Some(templates) => templates.copy_into(&paths),
        None => private_templates().copy_into(&paths),
    }
    paths
}
