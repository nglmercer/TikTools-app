use super::*;

impl DatabaseManager {
    /// Transfer a fresh package marker to the same admitted v2 account only
    /// after the caller observes the exact installed version and runtime.
    /// Existing decisions from any account and all recovery journals win.
    pub(crate) fn confirm_package_sync_candidate(
        &self,
        id: &str,
        marker_generation: i64,
        enabled: bool,
        intent: &LocalPluginSyncIntent<'_>,
        runtime_epoch: &str,
    ) -> Result<bool, DatabaseError> {
        if intent
            .version
            .is_some_and(|version| !valid_text(version, 128))
        {
            return Err(DatabaseError::Invalid(
                "invalid package sync version".into(),
            ));
        }
        if !valid_text(id, 128)
            || !valid_text(intent.origin, 2048)
            || !valid_text(intent.account_id, 1024)
            || marker_generation <= 0
            || !valid_runtime_epoch(runtime_epoch)
        {
            return Err(DatabaseError::Invalid(
                "invalid package sync candidate".into(),
            ));
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !apply_history_allows_source(&tx, id)? {
            return Ok(false);
        }
        let allowed: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u JOIN plugin_sync_source_versions s ON s.plugin_id=u.plugin_id WHERE u.plugin_id=?1 AND u.generation=?2 AND s.generation=?2 AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=?1) AND NOT EXISTS(SELECT 1 FROM plugin_sync_receipts r WHERE r.plugin_id=?1) AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=?1 OR p.plugin_id IS NULL) AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=?1))", params![id, marker_generation], |row| row.get(0))?;
        if !allowed {
            return Ok(false);
        }
        let generation = advance_plugin_source(&tx, id)?;
        tx.execute("INSERT INTO behavior_plugins(id,installed,enabled,updated_at) VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET installed=excluded.installed,enabled=excluded.enabled,updated_at=excluded.updated_at", params![id, bool_int(intent.version.is_some()), bool_int(enabled), now()])?;
        tx.execute("INSERT INTO plugin_sync_outbox(origin,account_id,plugin_id,generation,config_generation,version,enabled,runtime_epoch,reviewed_base) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'direct')", params![intent.origin, intent.account_id, id, generation, intent.config_generation.to_string(), intent.version, intent.version.map(|_| bool_int(enabled)), runtime_epoch])?;
        if tx.execute(
            "DELETE FROM plugin_sync_untracked_local WHERE plugin_id=?1 AND generation=?2",
            params![id, marker_generation],
        )? != 1
        {
            return Err(DatabaseError::Invalid("package marker changed".into()));
        }
        tx.commit()?;
        Ok(true)
    }

    /// Hold a nonblocking SQLite write reservation while a v2 upload is
    /// admitted. All source mutations must participate in the source journal;
    /// this does not fence package files or older/noncooperating processes.
    pub(crate) fn with_current_v2_upload_source<R, E: From<DatabaseError>>(
        &self,
        plugin_id: &str,
        generation: u64,
        enabled: impl Into<Option<bool>>,
        operation: impl FnOnce() -> Result<R, E>,
    ) -> Result<R, E> {
        let enabled = enabled.into();
        let generation = i64::try_from(generation)
            .ok()
            .filter(|v| *v > 0)
            .ok_or_else(|| E::from(DatabaseError::Invalid("invalid v2 upload source".into())))?;
        let mut db = self.open(&self.automation_path()).map_err(E::from)?;
        db.busy_timeout(std::time::Duration::ZERO)
            .map_err(DatabaseError::from)
            .map_err(E::from)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(DatabaseError::from)
            .map_err(E::from)?;
        if !apply_history_allows_source(&tx, plugin_id).map_err(E::from)? {
            return Err(E::from(DatabaseError::Invalid(
                "v2 application history needs review".into(),
            )));
        }
        let valid: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
             WHERE s.plugin_id=?1 AND s.generation=?2 AND b.installed=CASE WHEN ?3 IS NULL THEN 0 ELSE 1 END AND (?3 IS NULL OR b.enabled=?3)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=s.plugin_id))",
            params![plugin_id, generation, enabled.map(bool_int)], |row| row.get(0),
        ).map_err(DatabaseError::from).map_err(E::from)?;
        if !valid {
            return Err(E::from(DatabaseError::Invalid(
                "v2 upload source needs review".into(),
            )));
        }
        let result = operation();
        tx.rollback()
            .map_err(DatabaseError::from)
            .map_err(E::from)?;
        result
    }
}
