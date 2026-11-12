use super::*;

impl DatabaseManager {
    /// Atomically reserve the desired persisted state and advance its global
    /// source revision. The caller must have observed the same installed
    /// package version while holding the local mutation gate. Cloud never
    /// installs or replaces a package; version mismatch leaves the intent.
    pub(crate) fn commit_plugin_apply_state(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
        observed_version: &str,
    ) -> Result<Option<i64>, DatabaseError> {
        if !matches_context(intent, context) || observed_version != intent.installed_version {
            return Ok(None);
        }
        let encoded = payload(intent)?;
        let enabled = match intent
            .remote
            .as_ref()
            .and_then(|record| record.value.as_ref())
        {
            Some(value) if value.version == intent.installed_version => value.enabled,
            Some(_) => return Ok(None),
            None => false,
        };
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let eligible: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_apply_intents i
             JOIN plugin_sync_source_versions s ON s.plugin_id=i.plugin_id
             JOIN behavior_plugins b ON b.id=s.plugin_id WHERE i.intent_id=?1 AND i.plugin_id=?2
             AND i.payload_json=?3 AND i.phase='ready' AND s.generation=?4 AND b.installed=1 AND b.enabled=?5
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u WHERE u.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![intent.intent_id, intent.plugin_id, encoded, intent.source_generation, bool_int(intent.expected_enabled)], |row| row.get(0),
        )?;
        if !eligible || intent.source_generation == i64::MAX {
            return Ok(None);
        }
        let generation = intent.source_generation + 1;
        let source = tx.execute("UPDATE plugin_sync_source_versions SET generation=?1 WHERE plugin_id=?2 AND generation=?3", params![generation,intent.plugin_id,intent.source_generation])?;
        let state = tx.execute(
            "UPDATE behavior_plugins SET enabled=?1,updated_at=?2 WHERE id=?3 AND installed=1",
            params![bool_int(enabled), now(), intent.plugin_id],
        )?;
        let journal = tx.execute("UPDATE plugin_sync_apply_intents SET phase='runtime_pending',committed_generation=?1 WHERE intent_id=?2 AND phase='ready'", params![generation,intent.intent_id])?;
        if source != 1 || state != 1 || journal != 1 {
            return Err(DatabaseError::Invalid(
                "plugin apply transition was incomplete".into(),
            ));
        }
        tx.commit()?;
        Ok(Some(generation))
    }

    /// Record an observed runtime outcome. Repeating this after restart is
    /// allowed only after a fresh observation, never by replaying start/stop.
    pub(crate) fn confirm_plugin_apply_runtime(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
        generation: i64,
        runtime_epoch: &str,
        observed_version: &str,
        observed_enabled: bool,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context)
            || !valid_token(runtime_epoch)
            || observed_version != intent.installed_version
        {
            return Ok(false);
        }
        let encoded = payload(intent)?;
        let enabled = match intent
            .remote
            .as_ref()
            .and_then(|record| record.value.as_ref())
        {
            Some(value) if value.version == intent.installed_version => value.enabled,
            Some(_) => return Ok(false),
            None => false,
        };
        if enabled != observed_enabled {
            return Ok(false);
        }
        let db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let changed = db.execute(
            "UPDATE plugin_sync_apply_intents SET phase='applied',runtime_epoch=?1 WHERE intent_id=?2
             AND plugin_id=?3 AND payload_json=?4 AND phase IN ('runtime_pending','applied') AND committed_generation=?5
             AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
                WHERE s.plugin_id=?3 AND s.generation=?5 AND b.installed=1 AND b.enabled=?6
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u WHERE u.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![runtime_epoch,intent.intent_id,intent.plugin_id,encoded,generation,bool_int(enabled)],
        )?;
        Ok(changed == 1)
    }
}
