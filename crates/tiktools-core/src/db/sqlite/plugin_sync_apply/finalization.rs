use super::*;

impl DatabaseManager {
    /// Finalize only after the caller has durably acknowledged the original
    /// coordinator state and marker under the same session/admission fence.
    /// Keeping applied intents active until then prevents a second review from
    /// replacing evidence while crash recovery is still required.
    pub(crate) fn finalize_plugin_apply_after_native_ack(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
        generation: i64,
        runtime_epoch: &str,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context) || !valid_token(runtime_epoch) {
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
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE plugin_sync_apply_intents SET phase='finalized' WHERE intent_id=?1 AND plugin_id=?2
             AND payload_json=?3 AND phase='applied' AND committed_generation=?4 AND runtime_epoch=?5
             AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
                WHERE s.plugin_id=?2 AND s.generation=?4 AND b.installed=1 AND b.enabled=?6
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u WHERE u.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![intent.intent_id,intent.plugin_id,encoded,generation,runtime_epoch,bool_int(enabled)],
        )?;
        if changed != 1 {
            return Ok(false);
        }
        let witnessed = tx.execute(
            "INSERT INTO plugin_sync_apply_finalizations(intent_id,plugin_id,payload_json,committed_generation,runtime_epoch)
             SELECT intent_id,plugin_id,payload_json,committed_generation,runtime_epoch
             FROM plugin_sync_apply_intents WHERE intent_id=?1 AND phase='finalized'",
            [&intent.intent_id],
        )?;
        if witnessed != 1 {
            return Err(DatabaseError::Invalid(
                "plugin apply finalization witness missing".into(),
            ));
        }
        tx.commit()?;
        Ok(true)
    }
}
