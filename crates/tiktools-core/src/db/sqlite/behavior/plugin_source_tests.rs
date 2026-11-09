use super::*;
use crate::paths::AppPaths;

struct Fixture {
    db: DatabaseManager,
    root: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "plugin-source-{}-{}",
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
        Self {
            db: DatabaseManager::new(paths),
            root,
        }
    }

    fn generation(&self, id: &str) -> Option<i64> {
        self.db
            .open(&self.db.automation_path())
            .unwrap()
            .query_row(
                "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .unwrap()
    }

    fn state(&self, id: &str) -> Option<(bool, bool)> {
        self.db
            .open(&self.db.automation_path())
            .unwrap()
            .query_row(
                "SELECT installed,enabled FROM behavior_plugins WHERE id=?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn source_generation_follows_local_remote_and_repeated_writes_and_survives_delete() {
    let f = Fixture::new();
    // Simulate an existing plugin row from a database predating sync fences.
    f.db.open(&f.db.automation_path()).unwrap().execute(
        "INSERT INTO behavior_plugins(id,installed,enabled,updated_at) VALUES ('legacy.plugin',1,1,0)", [],
    ).unwrap();
    assert_eq!(f.generation("legacy.plugin"), None);
    f.db.set_plugin_state("legacy.plugin", true, true).unwrap();
    assert_eq!(f.generation("legacy.plugin"), Some(1));
    // Repeated writes conservatively invalidate prior source reviews too.
    f.db.set_plugin_state("legacy.plugin", true, true).unwrap();
    assert_eq!(f.generation("legacy.plugin"), Some(2));
    // The same persistence method is used when applying a remote selection.
    f.db.set_plugin_state("legacy.plugin", true, false).unwrap();
    assert_eq!(f.generation("legacy.plugin"), Some(3));
    assert_eq!(f.state("legacy.plugin"), Some((true, false)));
    f.db.remove_plugin_state("legacy.plugin").unwrap();
    assert_eq!(f.generation("legacy.plugin"), Some(4));
    assert_eq!(f.state("legacy.plugin"), None);
    f.db.remove_plugin_state("legacy.plugin").unwrap();
    assert_eq!(f.generation("legacy.plugin"), Some(5));
    f.db.set_plugin_state("legacy.plugin", true, true).unwrap();
    assert_eq!(f.generation("legacy.plugin"), Some(6));
    assert_eq!(f.state("legacy.plugin"), Some((true, true)));
}

#[test]
fn source_and_plugin_state_rollback_together_on_write_and_delete_failure() {
    let f = Fixture::new();
    f.db.set_plugin_state("acme.plugin", true, true).unwrap();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER reject_source_update BEFORE UPDATE ON behavior_plugins BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    drop(db);
    assert!(f.db.set_plugin_state("acme.plugin", true, false).is_err());
    assert_eq!(f.state("acme.plugin"), Some((true, true)));
    assert_eq!(f.generation("acme.plugin"), Some(1));
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("DROP TRIGGER reject_source_update; CREATE TRIGGER reject_source_delete BEFORE DELETE ON behavior_plugins BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    drop(db);
    assert!(f.db.remove_plugin_state("acme.plugin").is_err());
    assert_eq!(f.state("acme.plugin"), Some((true, true)));
    assert_eq!(f.generation("acme.plugin"), Some(1));
}

#[cfg(feature = "cloud-auth")]
#[test]
fn transactional_outbox_advances_source_once_and_legacy_write_invalidates_it() {
    use super::super::plugin_sync::LocalPluginSyncIntent;
    let f = Fixture::new();
    let intent = LocalPluginSyncIntent {
        origin: "https://cloud.example",
        account_id: "alice",
        config_generation: 7,
        version: Some("1.0.0"),
    };
    assert_eq!(
        f.db.set_plugin_state_with_sync_intent("acme.plugin", true, false, Some(intent))
            .unwrap(),
        1
    );
    let first =
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
    assert_eq!((first.generation, first.source_generation), (1, 1));
    f.db.set_plugin_state("acme.plugin", true, true).unwrap();
    let stale =
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
    assert_eq!((stale.generation, stale.source_generation), (1, 2));
    assert!(!f
        .db
        .confirm_plugin_sync_runtime(&stale, &"a".repeat(64))
        .unwrap());
}

#[cfg(feature = "plugin-install")]
#[test]
fn package_invalidation_is_durable_before_io_and_removal_adds_a_final_fence() {
    let f = Fixture::new();
    f.db.set_plugin_state("acme.plugin", true, true).unwrap();
    let generation =
        f.db.invalidate_plugin_package_source("acme.plugin")
            .unwrap();
    assert_eq!(generation, 2);
    // Even if package I/O later fails, reopening observes the invalidation;
    // the failed attempt must not revive earlier source evidence.
    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert_eq!(f.generation("acme.plugin"), Some(2));
    assert_eq!(f.state("acme.plugin"), Some((true, true)));
    assert!(reopened
        .remove_plugin_state_after_package("acme.plugin", generation)
        .unwrap());
    assert_eq!(f.state("acme.plugin"), None);
    assert_eq!(f.generation("acme.plugin"), Some(3));
}

#[cfg(feature = "plugin-install")]
#[test]
fn package_completion_refuses_newer_state_and_preserves_rollback_evidence() {
    let f = Fixture::new();
    f.db.set_plugin_state("acme.plugin", true, true).unwrap();
    let generation =
        f.db.invalidate_plugin_package_source("acme.plugin")
            .unwrap();
    f.db.set_plugin_state("acme.plugin", true, false).unwrap();
    assert!(!f
        .db
        .remove_plugin_state_after_package("acme.plugin", generation)
        .unwrap());
    assert_eq!(f.state("acme.plugin"), Some((true, false)));
    assert_eq!(f.generation("acme.plugin"), Some(4));
    let generation =
        f.db.invalidate_plugin_package_source("acme.plugin")
            .unwrap();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_package_finalize BEFORE DELETE ON behavior_plugins BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    drop(db);
    assert!(f
        .db
        .remove_plugin_state_after_package("acme.plugin", generation)
        .is_err());
    assert_eq!(f.state("acme.plugin"), Some((true, false)));
    assert_eq!(f.generation("acme.plugin"), Some(5));
}

#[cfg(all(feature = "plugin-install", feature = "cloud-auth"))]
#[test]
fn post_package_fence_invalidates_candidate_confirmed_during_filesystem_work() {
    use super::super::plugin_sync::LocalPluginSyncIntent;
    let f = Fixture::new();
    f.db.set_plugin_state("acme.plugin", true, true).unwrap();
    assert_eq!(
        f.db.invalidate_plugin_package_source("acme.plugin")
            .unwrap(),
        2
    );
    f.db.set_plugin_state_with_sync_intent(
        "acme.plugin",
        true,
        false,
        Some(LocalPluginSyncIntent {
            origin: "https://cloud.example",
            account_id: "bob",
            config_generation: 7,
            version: Some("1.0.0"),
        }),
    )
    .unwrap();
    let candidate =
        f.db.next_plugin_sync_intent("https://cloud.example", "bob")
            .unwrap()
            .unwrap();
    assert!(f
        .db
        .confirm_plugin_sync_runtime(&candidate, &"a".repeat(64))
        .unwrap());
    assert_eq!(
        f.db.invalidate_plugin_package_source("acme.plugin")
            .unwrap(),
        4
    );
    let stale =
        f.db.next_plugin_sync_intent("https://cloud.example", "bob")
            .unwrap()
            .unwrap();
    assert_eq!((stale.generation, stale.source_generation), (3, 4));
    assert!(!f
        .db
        .acknowledge_plugin_sync_intent(&stale, &"a".repeat(64))
        .unwrap());
}

#[cfg(feature = "plugin-install")]
#[test]
fn package_reservation_failure_never_changes_source_or_state() {
    let f = Fixture::new();
    f.db.set_plugin_state("acme.plugin", true, true).unwrap();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_package_reservation BEFORE UPDATE ON plugin_sync_source_versions BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    drop(db);
    assert!(f
        .db
        .invalidate_plugin_package_source("acme.plugin")
        .is_err());
    assert!(f.db.invalidate_plugin_package_source("../invalid").is_err());
    assert_eq!(f.state("acme.plugin"), Some((true, true)));
    assert_eq!(f.generation("acme.plugin"), Some(1));
    assert_eq!(f.generation("../invalid"), None);
}
