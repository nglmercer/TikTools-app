use super::*;

impl DatabaseManager {
    /// Read one identity's candidate together with the current source revision.
    /// The caller must reject a mismatch and seek fresh review; it must not
    /// silently publish an earlier account's stale local state.
    pub(crate) fn next_plugin_sync_intent(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<Option<PluginSyncOutboxEntry>, DatabaseError> {
        self.find_plugin_sync_intent(origin, account, None)
    }

    pub(crate) fn plugin_sync_intent_for(
        &self,
        origin: &str,
        account: &str,
        plugin_id: &str,
    ) -> Result<Option<PluginSyncOutboxEntry>, DatabaseError> {
        if !valid_text(plugin_id, 128) {
            return Err(DatabaseError::Invalid("invalid plugin sync source".into()));
        }
        self.find_plugin_sync_intent(origin, account, Some(plugin_id))
    }

    fn find_plugin_sync_intent(
        &self,
        origin: &str,
        account: &str,
        plugin_id: Option<&str>,
    ) -> Result<Option<PluginSyncOutboxEntry>, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        let entry = db.query_row(
            "SELECT o.origin,o.account_id,o.plugin_id,o.generation,s.generation,o.config_generation,o.version,o.enabled,o.runtime_epoch,o.reviewed_base
             FROM plugin_sync_outbox o LEFT JOIN plugin_sync_source_versions s ON s.plugin_id=o.plugin_id
             WHERE o.origin=?1 AND o.account_id=?2 AND (?3 IS NULL OR o.plugin_id=?3) ORDER BY o.plugin_id COLLATE BINARY LIMIT 1",
            params![origin, account, plugin_id], |row| {
                let raw: Option<i64> = row.get(7)?;
                let source: Option<i64> = row.get(4)?;
                let config: Option<String> = row.get(5)?;
                Ok((PluginSyncOutboxEntry {
                    origin: row.get(0)?, account_id: row.get(1)?, plugin_id: row.get(2)?,
                    generation: row.get(3)?, source_generation: source.unwrap_or(0),
                    config_generation: config.as_deref().and_then(|s| s.parse::<u128>().ok()).unwrap_or_default(),
                    version: row.get(6)?, enabled: raw.map(|value| value != 0), runtime_epoch: row.get(8)?, reviewed_base: row.get(9)?,
                }, raw, config))
            },
        ).optional()?;
        match entry {
            Some((entry, raw, config))
                if entry.generation <= 0
                    || entry.source_generation <= 0
                    || !valid_text(&entry.origin, 2048)
                    || !valid_text(&entry.account_id, 1024)
                    || !valid_text(&entry.plugin_id, 128)
                    || entry.version.as_ref().is_some_and(|v| !valid_text(v, 128))
                    || entry.version.is_some() != entry.enabled.is_some()
                    || entry
                        .runtime_epoch
                        .as_deref()
                        .is_some_and(|value| !valid_runtime_epoch(value))
                    || config.as_deref() != Some(entry.config_generation.to_string().as_str())
                    || raw.is_some_and(|value| value != 0 && value != 1) =>
            {
                Err(DatabaseError::Invalid("invalid plugin sync outbox".into()))
            }
            Some((entry, _, _)) => {
                if !package_source_available(&db, &entry.plugin_id)? {
                    return Err(DatabaseError::Invalid(
                        "plugin package operation needs review".into(),
                    ));
                }
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Confirm only after this host has reconciled the desired plugin runtime
    /// with the persisted source. Epochs must be unpredictable, native-only,
    /// unique per host lifetime and shared by every drain in that lifetime.
    /// Restart or a second host requires a fresh observation and confirmation.
    pub(crate) fn confirm_plugin_sync_runtime(
        &self,
        entry: &PluginSyncOutboxEntry,
        runtime_epoch: &str,
    ) -> Result<bool, DatabaseError> {
        if !valid_runtime_epoch(runtime_epoch) {
            return Err(DatabaseError::Invalid(
                "invalid plugin runtime epoch".into(),
            ));
        }
        let db = self.open(&self.automation_path())?;
        let changed = db.execute(
            "UPDATE plugin_sync_outbox SET runtime_epoch=?6 WHERE origin=?1 AND account_id=?2 AND plugin_id=?3
             AND generation=?4 AND config_generation=?5 AND version IS ?7 AND enabled IS ?8 AND reviewed_base IS ?9
             AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s WHERE s.plugin_id=?3 AND s.generation=?4)
             AND EXISTS(SELECT 1 FROM behavior_plugins b WHERE b.id=?3
                 AND b.installed=CASE WHEN ?7 IS NULL THEN 0 ELSE 1 END
                 AND (?7 IS NULL OR b.enabled=?8))
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=?3 OR p.plugin_id IS NULL)",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation,
                entry.config_generation.to_string(), runtime_epoch, entry.version,
                entry.enabled.map(bool_int), entry.reviewed_base],
        )?;
        Ok(changed == 1)
    }

    /// Bind an older unclassified candidate to an explicit, post-barrier Cloud
    /// review. The caller owns the source, identity, consent and runtime gates.
    /// No old runtime confirmation survives the review.
    #[allow(dead_code)]
    pub(crate) fn review_legacy_plugin_sync_base(
        &self,
        entry: &PluginSyncOutboxEntry,
        current_config_generation: u128,
        reviewed_base: &str,
    ) -> Result<bool, DatabaseError> {
        let base: Option<cloud_client::WorkspaceRecord> = serde_json::from_str(reviewed_base)
            .map_err(|_| DatabaseError::Invalid("invalid reviewed Cloud base".into()))?;
        if entry.reviewed_base.is_some()
            || entry.generation <= 0
            || entry.config_generation == 0
            || current_config_generation == 0
            || entry.version.is_none()
            || entry.enabled.is_none()
            || reviewed_base.len() > 512 * 1024
            || base.as_ref().is_some_and(|r| {
                r.collection != "plugin-selections" || r.key != entry.plugin_id || r.revision <= 0
            })
        {
            return Err(DatabaseError::Invalid(
                "invalid legacy plugin review".into(),
            ));
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !apply_history_allows_source(&tx, &entry.plugin_id)? {
            return Ok(false);
        }
        let changed = tx.execute(
            "UPDATE plugin_sync_outbox SET reviewed_base=?9,config_generation=?10,runtime_epoch=NULL
             WHERE origin=?1 AND account_id=?2 AND plugin_id=?3 AND generation=?4
               AND config_generation=?5 AND version IS ?6 AND enabled IS ?7 AND runtime_epoch IS ?8
               AND reviewed_base IS NULL
               AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s WHERE s.plugin_id=?3 AND s.generation=?4)
               AND EXISTS(SELECT 1 FROM behavior_plugins b WHERE b.id=?3 AND b.installed=1 AND b.enabled=?7)
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=?3 AND (o.origin<>?1 OR o.account_id<>?2))
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u WHERE u.plugin_id=?3)
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_receipts q WHERE q.plugin_id=?3)
               AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=?3 OR p.plugin_id IS NULL)
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=?3)",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation,
                entry.config_generation.to_string(), entry.version, entry.enabled.map(bool_int),
                entry.runtime_epoch, reviewed_base, current_config_generation.to_string()],
        )?;
        tx.commit()?;
        Ok(changed == 1)
    }

    /// Only the exact generation and current host lifetime may be
    /// acknowledged after durable staging. A concurrent newer edit, a
    /// restart, or an unreconciled runtime is never removed.
    pub(crate) fn acknowledge_plugin_sync_intent(
        &self,
        entry: &PluginSyncOutboxEntry,
        runtime_epoch: &str,
    ) -> Result<bool, DatabaseError> {
        if !valid_runtime_epoch(runtime_epoch) {
            return Err(DatabaseError::Invalid(
                "invalid plugin runtime epoch".into(),
            ));
        }
        let db = self.open(&self.automation_path())?;
        let removed = db.execute(
            "DELETE FROM plugin_sync_outbox WHERE origin=?1 AND account_id=?2 AND plugin_id=?3
             AND generation=?4 AND config_generation=?5 AND runtime_epoch=?6
             AND version IS ?7 AND enabled IS ?8 AND reviewed_base IS ?9 AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s
                 WHERE s.plugin_id=?3 AND s.generation=?4)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=?3 OR p.plugin_id IS NULL)",
            params![
                entry.origin,
                entry.account_id,
                entry.plugin_id,
                entry.generation,
                entry.config_generation.to_string(),
                runtime_epoch,
                entry.version,
                entry.enabled.map(bool_int),
                entry.reviewed_base,
            ],
        )?;
        Ok(removed == 1)
    }
}
