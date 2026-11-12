use super::*;

impl DatabaseManager {
    /// Inspect the persisted local source for a pulled-record decision. This
    /// observation is not authorization: `prepare_plugin_apply` rechecks it
    /// atomically against any later source, outbox or journal changes.
    pub(crate) fn pulled_plugin_apply_source(
        &self,
        plugin_id: &str,
    ) -> Result<Option<(i64, bool)>, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        let found: Option<(i64, i64)> = db
            .query_row(
                "SELECT s.generation,b.enabled FROM plugin_sync_source_versions s
             JOIN behavior_plugins b ON b.id=s.plugin_id
             WHERE s.plugin_id=?1 AND b.installed=1
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u WHERE u.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_apply_intents i
                 WHERE i.plugin_id=s.plugin_id AND i.phase!='finalized')",
                [plugin_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match found {
            Some((generation, enabled)) if generation > 0 && (enabled == 0 || enabled == 1) => {
                Ok(Some((generation, enabled == 1)))
            }
            Some(_) => Err(DatabaseError::Invalid("invalid plugin apply source".into())),
            None => Ok(None),
        }
    }

    /// Persist reviewed evidence before changing a pending record or marker.
    /// A newer local candidate, missing source revision or different installed
    /// state rejects admission without changing anything.
    pub(crate) fn prepare_plugin_apply(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context) {
            return Ok(false);
        }
        let encoded = payload(intent)?;
        // Refuse the decision before coordinator pending state is touched if
        // applying it would require installing or replacing a package.
        if intent
            .remote
            .as_ref()
            .and_then(|record| record.value.as_ref())
            .is_some_and(|value| value.version != intent.installed_version)
        {
            return Ok(false);
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<StoredRow> = tx
            .query_row(
                "SELECT plugin_id,phase,payload_json,committed_generation,runtime_epoch FROM plugin_sync_apply_intents WHERE intent_id=?1",
                [&intent.intent_id],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
            )
            .optional()?;
        if let Some(prior) = prior {
            if prior.2 != encoded {
                return Err(DatabaseError::Invalid(
                    "plugin apply identity reused".into(),
                ));
            }
            let stored = decode_stored(&intent.intent_id, prior)?;
            if stored.intent != *intent {
                return Err(DatabaseError::Invalid(
                    "plugin apply identity reused".into(),
                ));
            }
            let witness: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM plugin_sync_apply_finalizations f WHERE f.intent_id=?1)",
                [&intent.intent_id], |row| row.get(0),
            )?;
            let witness_matches: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM plugin_sync_apply_finalizations f JOIN plugin_sync_apply_intents i ON f.intent_id=i.intent_id WHERE i.intent_id=?1 AND f.plugin_id=i.plugin_id AND f.payload_json=i.payload_json AND f.committed_generation=i.committed_generation AND f.runtime_epoch=i.runtime_epoch)",
                [&intent.intent_id], |row| row.get(0),
            )?;
            if (stored.phase == PluginApplyPhase::Finalized && !witness_matches)
                || (stored.phase != PluginApplyPhase::Finalized && witness)
            {
                return Err(DatabaseError::Invalid(
                    "invalid plugin apply finalization witness".into(),
                ));
            }
            // Even an idempotent retry must not report admission while a
            // package transition could change the artifact on disk.
            let blocked: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=?1 OR p.plugin_id IS NULL)",
                [&intent.plugin_id], |row| row.get(0),
            )?;
            return Ok(!blocked);
        }
        // Retain outcome identities until a bounded compaction protocol can
        // preserve retry safety. Never let a new operation create history
        // that startup cannot fully validate.
        let retained: i64 = tx.query_row(
            "SELECT COUNT(*) FROM (SELECT 1 FROM plugin_sync_apply_intents LIMIT ?1)",
            [super::plugin_apply_quarantine::MAX_RETAINED as i64],
            |row| row.get(0),
        )?;
        if retained >= super::plugin_apply_quarantine::MAX_RETAINED as i64 {
            return Err(DatabaseError::Invalid(
                "plugin apply history capacity reached".into(),
            ));
        }
        let eligible: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
             WHERE s.plugin_id=?1 AND s.generation=?2 AND b.installed=1 AND b.enabled=?3
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u WHERE u.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![intent.plugin_id, intent.source_generation, bool_int(intent.expected_enabled)], |row| row.get(0),
        )?;
        if !eligible {
            return Ok(false);
        }
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO plugin_sync_apply_intents(intent_id,plugin_id,phase,payload_json)
             VALUES (?1,?2,'prepared',?3)", params![intent.intent_id, intent.plugin_id, encoded],
        )?;
        if inserted != 1 {
            return Ok(false);
        }
        tx.commit()?;
        Ok(true)
    }
}
