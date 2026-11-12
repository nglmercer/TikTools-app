use super::*;

impl DatabaseManager {
    pub(crate) fn has_untracked_plugin_sync(&self) -> Result<bool, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        Ok(db.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_untracked_local)",
            [],
            |row| row.get(0),
        )?)
    }

    /// Read one local marker without inferring an account or consent. A stale
    /// marker is rejected rather than silently attributed to a newer source.
    #[allow(dead_code)]
    pub(crate) fn review_untracked_plugin(
        &self,
        id: &str,
    ) -> Result<Option<UntrackedPluginReview>, DatabaseError> {
        if !valid_text(id, 128) {
            return Err(DatabaseError::Invalid("invalid plugin sync source".into()));
        }
        let db = self.open(&self.automation_path())?;
        let row: Option<(i64, i64, i64, i64)> = db.query_row(
            "SELECT u.generation, s.generation, b.installed, b.enabled FROM plugin_sync_untracked_local u JOIN plugin_sync_source_versions s ON s.plugin_id=u.plugin_id JOIN behavior_plugins b ON b.id=u.plugin_id WHERE u.plugin_id=?1",
            [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).optional()?;
        match row {
            Some((marker, current, installed, enabled))
                if marker > 0
                    && marker == current
                    && matches!(installed, 0 | 1)
                    && matches!(enabled, 0 | 1) =>
            {
                Ok(Some(UntrackedPluginReview {
                    plugin_id: id.into(),
                    generation: marker,
                    installed: installed == 1,
                    enabled: enabled == 1,
                }))
            }
            Some(_) => Err(DatabaseError::Invalid(
                "untracked plugin source needs review".into(),
            )),
            None => {
                let exists: bool = db.query_row(
                    "SELECT EXISTS(SELECT 1 FROM plugin_sync_untracked_local WHERE plugin_id=?1)",
                    [id],
                    |r| r.get(0),
                )?;
                if exists {
                    Err(DatabaseError::Invalid(
                        "untracked plugin source needs review".into(),
                    ))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Atomically transfer one explicitly reviewed marker into an unconfirmed
    /// account outbox entry. Caller must hold the exclusive emission lease,
    /// exact identity/consent and runtime/package fences, and verify `version`
    /// against the reviewed installed artifact. Tombstones require separate
    /// package reconciliation and are deliberately rejected here.
    #[allow(dead_code)]
    pub(crate) fn reconcile_untracked_plugin(
        &self,
        review: &UntrackedPluginReview,
        intent: &LocalPluginSyncIntent<'_>,
        reviewed_base: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let version = intent
            .version
            .filter(|v| valid_text(v, 128))
            .ok_or_else(|| DatabaseError::Invalid("reviewed installed version required".into()))?;
        if !review.installed
            || review.generation <= 0
            || intent.config_generation == 0
            || !valid_text(&review.plugin_id, 128)
            || !valid_text(intent.origin, 2048)
            || !valid_text(intent.account_id, 1024)
        {
            return Err(DatabaseError::Invalid(
                "invalid untracked plugin review".into(),
            ));
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !apply_history_allows_source(&tx, &review.plugin_id)? {
            return Err(DatabaseError::Invalid(
                "plugin application history needs review".into(),
            ));
        }
        let valid: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u JOIN plugin_sync_source_versions s ON s.plugin_id=u.plugin_id JOIN behavior_plugins b ON b.id=u.plugin_id WHERE u.plugin_id=?1 AND u.generation=?2 AND s.generation=?2 AND b.installed=1 AND b.enabled=?3 AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=u.plugin_id) AND NOT EXISTS(SELECT 1 FROM plugin_sync_receipts q WHERE q.plugin_id=u.plugin_id) AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=u.plugin_id OR p.plugin_id IS NULL) AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=u.plugin_id))",
            params![review.plugin_id, review.generation, bool_int(review.enabled)], |r| r.get(0),
        )?;
        if !valid {
            return Err(DatabaseError::Invalid(
                "untracked plugin review is stale or blocked".into(),
            ));
        }
        tx.execute("INSERT INTO plugin_sync_outbox(origin,account_id,plugin_id,generation,config_generation,version,enabled,runtime_epoch,reviewed_base) VALUES (?1,?2,?3,?4,?5,?6,?7,NULL,?8)", params![intent.origin, intent.account_id, review.plugin_id, review.generation, intent.config_generation.to_string(), version, bool_int(review.enabled), reviewed_base])?;
        if tx.execute(
            "DELETE FROM plugin_sync_untracked_local WHERE plugin_id=?1 AND generation=?2",
            params![review.plugin_id, review.generation],
        )? != 1
        {
            return Err(DatabaseError::Invalid(
                "untracked plugin review changed".into(),
            ));
        }
        tx.commit()?;
        Ok(())
    }
}
