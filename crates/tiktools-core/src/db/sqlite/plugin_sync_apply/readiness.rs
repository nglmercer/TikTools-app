use super::*;

impl DatabaseManager {
    /// Called only after native coordinator state has been reconciled under
    /// the same admission/session fence. The uncertain marker must remain
    /// durable until the final application outcome is acknowledged.
    pub(crate) fn mark_plugin_apply_ready_after_native_reconciliation(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context) {
            return Ok(false);
        }
        let encoded = payload(intent)?;
        let db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let changed = db.execute(
            "UPDATE plugin_sync_apply_intents SET phase='ready' WHERE intent_id=?1 AND plugin_id=?2
             AND payload_json=?3 AND phase='prepared' AND EXISTS(
                SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
                WHERE s.plugin_id=?2 AND s.generation=?4 AND b.installed=1 AND b.enabled=?5
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u WHERE u.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![intent.intent_id, intent.plugin_id, encoded, intent.source_generation, bool_int(intent.expected_enabled)],
        )?;
        Ok(changed == 1)
    }

    /// Revalidate a previously-ready decision against the same durable
    /// evidence and current source. This is observation only; the later
    /// state/runtime transition must independently compare those inputs.
    pub(crate) fn revalidate_ready_plugin_apply(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context) {
            return Ok(false);
        }
        let encoded = payload(intent)?;
        let db = self.open(&self.automation_path())?;
        Ok(db.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_apply_intents i
             JOIN plugin_sync_source_versions s ON s.plugin_id=i.plugin_id
             JOIN behavior_plugins b ON b.id=s.plugin_id
             WHERE i.intent_id=?1 AND i.plugin_id=?2 AND i.payload_json=?3
             AND i.phase='ready' AND s.generation=?4 AND b.installed=1 AND b.enabled=?5
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u WHERE u.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![
                intent.intent_id,
                intent.plugin_id,
                encoded,
                intent.source_generation,
                bool_int(intent.expected_enabled)
            ],
            |row| row.get(0),
        )?)
    }
}
