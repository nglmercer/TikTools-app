//! Coherent local source evidence for a future reviewed Cloud cutover.
//!
//! This records SQLite state, not the filesystem or running workers. Callers
//! must also fence package/runtime mutations and the account/consent epoch.
#[cfg(test)]
use rusqlite::params;
use rusqlite::TransactionBehavior;

use super::{DatabaseError, DatabaseManager};

const MAX_SOURCE_ROWS: usize = 8192;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PluginCutoverRow {
    pub id: String,
    /// None represents a removed row whose generation remains as evidence.
    pub state: Option<(bool, bool)>,
    /// None represents a legacy row that predates source version tracking.
    pub generation: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PluginCutoverSource {
    pub rows: Vec<PluginCutoverRow>,
}

impl DatabaseManager {
    /// Capture all source revisions and plugin rows in one SQLite snapshot.
    /// Unfinished work cannot be collapsed into an installed selection. A
    /// caller must compare a fresh snapshot again under its mutation fence.
    pub(crate) fn plugin_cutover_source(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<PluginCutoverSource, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let snapshot = read_source(&tx, origin, account)?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Hold SQLite's write reservation around a synchronous source review and
    /// commit. Callers take invocation and package-store leases first, never
    /// await or re-enter this database in the callback, and must separately
    /// fence source files and other noncooperating writers. On any error or
    /// unwind the transaction releases its lock without changing source data.
    pub(crate) fn with_locked_plugin_cutover_source<R, E: From<DatabaseError>>(
        &self,
        origin: &str,
        account: &str,
        operation: impl FnOnce(PluginCutoverSource) -> Result<R, E>,
    ) -> Result<R, E> {
        let mut db = self.open(&self.automation_path()).map_err(E::from)?;
        db.busy_timeout(std::time::Duration::ZERO)
            .map_err(DatabaseError::from)
            .map_err(E::from)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(DatabaseError::from)
            .map_err(E::from)?;
        let snapshot = read_source(&tx, origin, account).map_err(E::from)?;
        let result = operation(snapshot);
        tx.rollback()
            .map_err(DatabaseError::from)
            .map_err(E::from)?;
        result
    }
}

fn read_source(
    db: &rusqlite::Connection,
    _origin: &str,
    _account: &str,
) -> Result<PluginCutoverSource, DatabaseError> {
    // Plugin state is device-global. A receipt cannot be safely imported
    // until its full pending intent, consent and runtime evidence are bound
    // to the reviewed v1 state under the same protocol lease.
    // A future reconciliation must compare identity, source generation,
    // pending/base/conflict, consent and runtime, then preserve or explicitly
    // resolve historical receipts; it must not silently delete them.
    // Legacy/local-only edits lack durable provenance; this check alone does
    // not establish that the remaining state is safe to expose to a user.
    let unresolved: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM plugin_package_mutation_intents)
               OR EXISTS(SELECT 1 FROM plugin_sync_apply_intents)
               OR EXISTS(SELECT 1 FROM plugin_sync_apply_finalizations)
               OR EXISTS(SELECT 1 FROM plugin_sync_outbox)
               OR EXISTS(SELECT 1 FROM plugin_sync_rebase_intents)
               OR EXISTS(SELECT 1 FROM plugin_sync_receipts)",
        [],
        |row| row.get(0),
    )?;
    if unresolved {
        return Err(DatabaseError::Invalid(
            "unfinished plugin operation needs review before Cloud migration".into(),
        ));
    }
    let rows = {
        let mut statement = db.prepare(
                "SELECT id,installed,enabled,generation FROM (
                    SELECT b.id AS id,b.installed AS installed,b.enabled AS enabled,s.generation AS generation
                    FROM behavior_plugins b LEFT JOIN plugin_sync_source_versions s ON s.plugin_id=b.id
                    UNION ALL
                    SELECT s.plugin_id,NULL,NULL,s.generation FROM plugin_sync_source_versions s
                    LEFT JOIN behavior_plugins b ON b.id=s.plugin_id WHERE b.id IS NULL
                 ) ORDER BY id COLLATE BINARY LIMIT ?1",
            )?;
        let mut query = statement.query([(MAX_SOURCE_ROWS + 1) as i64])?;
        let mut result = Vec::new();
        while let Some(row) = query.next()? {
            if result.len() == MAX_SOURCE_ROWS {
                return Err(DatabaseError::Invalid(
                    "too many plugin source revisions".into(),
                ));
            }
            let id: String = row.get(0)?;
            let installed: Option<i64> = row.get(1)?;
            let enabled: Option<i64> = row.get(2)?;
            let generation: Option<i64> = row.get(3)?;
            let state = match (installed, enabled) {
                (None, None) if generation.is_some() => None,
                (Some(i @ 0..=1), Some(e @ 0..=1)) => Some((i != 0, e != 0)),
                _ => return Err(DatabaseError::Invalid("invalid plugin source state".into())),
            };
            if !tiktools_plugin_api::manifest::is_valid_plugin_id(&id)
                || generation.is_some_and(|n| n <= 0)
            {
                return Err(DatabaseError::Invalid(
                    "invalid plugin source revision".into(),
                ));
            }
            result.push(PluginCutoverRow {
                id,
                state,
                generation,
            });
        }
        result
    };
    Ok(PluginCutoverSource { rows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AppPaths;

    struct TestDir(std::path::PathBuf);
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn database() -> (TestDir, DatabaseManager) {
        let dir = TestDir(std::env::temp_dir().join(format!(
            "plugin-cutover-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        )));
        let root = &dir.0;
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.to_owned(),
        };
        paths.ensure_directories().unwrap();
        for file in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            drop(rusqlite::Connection::open(file).unwrap());
        }
        let db = DatabaseManager::new(paths);
        (dir, db)
    }

    #[test]
    fn source_detects_same_value_changes_and_delete_recreate() {
        let (_dir, db) = database();
        let origin = "https://cloud.example";
        db.set_plugin_state("acme.plugin", true, false).unwrap();
        let original = db.plugin_cutover_source(origin, "alice").unwrap();
        db.set_plugin_state("acme.plugin", true, false).unwrap();
        let repeated = db.plugin_cutover_source(origin, "alice").unwrap();
        assert_ne!(original, repeated);
        assert_eq!(original.rows[0].state, repeated.rows[0].state);
        db.remove_plugin_state("acme.plugin").unwrap();
        let deleted = db.plugin_cutover_source(origin, "alice").unwrap();
        assert_eq!(deleted.rows[0].state, None);
        db.set_plugin_state("acme.plugin", true, false).unwrap();
        let recreated = db.plugin_cutover_source(origin, "alice").unwrap();
        assert_ne!(original, recreated);
        assert_eq!(original.rows[0].state, recreated.rows[0].state);
    }

    #[test]
    fn source_write_reservation_blocks_other_connections_and_releases_on_error() {
        let (_dir, db) = database();
        let origin = "https://cloud.example";
        db.set_plugin_state("acme.plugin", true, false).unwrap();
        let before = db.plugin_cutover_source(origin, "alice").unwrap();
        let other = db.open(&db.automation_path()).unwrap();
        other.busy_timeout(std::time::Duration::ZERO).unwrap();
        let result: Result<(), DatabaseError> = db.with_locked_plugin_cutover_source(origin, "alice", |source| {
            assert_eq!(source, before);
            let error = other.execute("UPDATE behavior_plugins SET enabled=1 WHERE id='acme.plugin'", []).unwrap_err();
            assert!(matches!(error, rusqlite::Error::SqliteFailure(ref error, _) if matches!(error.code, rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)));
            Err(DatabaseError::Invalid("abort fixture".into()))
        });
        assert!(result.is_err());
        assert_eq!(db.plugin_cutover_source(origin, "alice").unwrap(), before);
        db.set_plugin_state("acme.plugin", true, true).unwrap();
        assert_ne!(db.plugin_cutover_source(origin, "alice").unwrap(), before);
    }

    #[test]
    fn source_rejects_unfinished_work_and_corrupt_rows() {
        let (_dir, db) = database();
        let origin = "https://cloud.example";
        let conn = db.open(&db.automation_path()).unwrap();
        conn.execute("INSERT INTO plugin_package_mutation_intents(plugin_id,intent_id,operation,operation_digest,phase,source_generation) VALUES ('acme.plugin',?1,'install',?2,'prepared',1)",
            params!["a".repeat(64), "b".repeat(64)]).unwrap();
        assert!(db.plugin_cutover_source(origin, "alice").is_err());
        conn.execute("DELETE FROM plugin_package_mutation_intents", [])
            .unwrap();
        conn.execute("INSERT INTO plugin_sync_apply_finalizations(intent_id,plugin_id,payload_json,committed_generation,runtime_epoch) VALUES (?1,'acme.plugin','{}',1,?2)", params!["c".repeat(64), "d".repeat(64)]).unwrap();
        assert!(db.plugin_cutover_source(origin, "alice").is_err());
        conn.execute("DELETE FROM plugin_sync_apply_finalizations", [])
            .unwrap();
        conn.execute("INSERT INTO plugin_sync_outbox(origin,account_id,plugin_id,generation,config_generation,version,enabled) VALUES (?1,'bob','acme.plugin',1,'7','1.0.0',1)", [origin]).unwrap();
        assert!(db.plugin_cutover_source(origin, "bob").is_err());
        assert!(db.plugin_cutover_source(origin, "alice").is_err());
        conn.execute("DELETE FROM plugin_sync_outbox", []).unwrap();
        db.set_plugin_state("acme.plugin", true, true).unwrap();
        conn.execute("INSERT INTO plugin_sync_receipts(origin,account_id,plugin_id,generation,config_generation,runtime_epoch,pending_json) VALUES (?1,'bob','acme.plugin',1,'7',?2,'{}')", params![origin, "e".repeat(64)]).unwrap();
        assert!(db.plugin_cutover_source(origin, "alice").is_err());
        assert!(db.plugin_cutover_source(origin, "bob").is_err());
        let pending = tiktools_cloud_sync::V1Pending {
            base_revision: None,
            edit: tiktools_cloud_sync::V1Edit::Put(tiktools_cloud_sync::PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "1.0.0".into(),
                enabled: true,
            }),
            conflict: false,
        };
        let encoded = serde_json::to_string(&pending).unwrap();
        conn.execute(
            "UPDATE plugin_sync_receipts SET pending_json=?1",
            [&encoded],
        )
        .unwrap();
        // Even a valid own receipt is unresolved until compared against the
        // exact v1 pending edit and consent under one protocol lease.
        assert!(db.plugin_cutover_source(origin, "bob").is_err());
        assert!(db.plugin_cutover_source(origin, "alice").is_err());
        db.set_plugin_state("acme.plugin", true, true).unwrap();
        assert!(db.plugin_cutover_source(origin, "alice").is_err());
        // Future generations and invalid consent cannot authorize anything.
        conn.execute(
            "UPDATE plugin_sync_receipts SET generation=99,config_generation='0'",
            [],
        )
        .unwrap();
        assert!(db.plugin_cutover_source(origin, "bob").is_err());
        let retained: String = conn
            .query_row("SELECT pending_json FROM plugin_sync_receipts", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(retained, encoded);
        conn.execute("DELETE FROM plugin_sync_receipts", [])
            .unwrap();
        conn.execute("INSERT INTO plugin_sync_rebase_intents(origin,account_id,plugin_id,payload_json) VALUES (?1,'bob','acme.plugin','{}')", [origin]).unwrap();
        assert!(db.plugin_cutover_source(origin, "alice").is_err());
        conn.execute("DELETE FROM plugin_sync_rebase_intents", [])
            .unwrap();
        conn.execute("INSERT INTO plugin_sync_receipts(origin,account_id,plugin_id,generation,config_generation,runtime_epoch,pending_json) VALUES (?1,'bob','missing.plugin',1,'7',?2,'{}')", params![origin, "e".repeat(64)]).unwrap();
        assert!(db.plugin_cutover_source(origin, "alice").is_err());
        conn.execute("DELETE FROM plugin_sync_receipts", [])
            .unwrap();
        conn.execute("DELETE FROM behavior_plugins WHERE id='acme.plugin'", [])
            .unwrap();
        conn.execute(
            "DELETE FROM plugin_sync_source_versions WHERE plugin_id='acme.plugin'",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO behavior_plugins(id,installed,enabled,updated_at) VALUES ('legacy.plugin',1,1,0)", []).unwrap();
        let legacy = db.plugin_cutover_source(origin, "alice").unwrap();
        assert_eq!(legacy.rows[0].generation, None);
        conn.execute(
            "UPDATE behavior_plugins SET enabled=9 WHERE id='legacy.plugin'",
            [],
        )
        .unwrap();
        assert!(db.plugin_cutover_source(origin, "alice").is_err());
    }
}
