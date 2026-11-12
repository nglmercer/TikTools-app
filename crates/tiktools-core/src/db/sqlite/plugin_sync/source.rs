use super::*;

impl DatabaseManager {
    /// Record the source state and its local Cloud candidate atomically.
    /// Passing no intent records a local-only edit while still advancing the
    /// source generation; this deliberately leaves older intents stale.
    pub(crate) fn set_plugin_state_with_sync_intent(
        &self,
        id: &str,
        installed: bool,
        enabled: bool,
        intent: Option<LocalPluginSyncIntent<'_>>,
    ) -> Result<i64, DatabaseError> {
        self.set_plugin_state_with_sync_intent_inner(id, installed, enabled, intent, false)
    }

    /// Atomically mark a source changed without account/consent admission.
    /// Caller must hold the exclusive product-emission lease. The marker is
    /// never cleared by a later edit and requires explicit review.
    pub(crate) fn set_plugin_state_with_untracked_sync(
        &self,
        id: &str,
        installed: bool,
        enabled: bool,
    ) -> Result<i64, DatabaseError> {
        self.set_plugin_state_with_sync_intent_inner(id, installed, enabled, None, true)
    }

    fn set_plugin_state_with_sync_intent_inner(
        &self,
        id: &str,
        installed: bool,
        enabled: bool,
        intent: Option<LocalPluginSyncIntent<'_>>,
        untracked: bool,
    ) -> Result<i64, DatabaseError> {
        if !valid_text(id, 128)
            || intent.as_ref().is_some_and(|candidate| {
                !valid_text(candidate.origin, 2048)
                    || !valid_text(candidate.account_id, 1024)
                    || candidate.version.is_some_and(|v| !valid_text(v, 128))
                    || installed != candidate.version.is_some()
            })
        {
            return Err(DatabaseError::Invalid("invalid plugin sync source".into()));
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !package_source_available(&tx, id)? {
            return Err(DatabaseError::Invalid(
                "plugin package operation needs review".into(),
            ));
        }
        let generation = advance_plugin_source(&tx, id)?;
        tx.execute(
            "INSERT INTO behavior_plugins(id,installed,enabled,updated_at) VALUES (?1,?2,?3,?4)
             ON CONFLICT(id) DO UPDATE SET installed=excluded.installed,
                enabled=excluded.enabled,updated_at=excluded.updated_at",
            params![id, bool_int(installed), bool_int(enabled), now()],
        )?;
        if untracked {
            tx.execute(
                "INSERT INTO plugin_sync_untracked_local(plugin_id,generation) VALUES (?1,?2)
                 ON CONFLICT(plugin_id) DO UPDATE SET generation=excluded.generation",
                params![id, generation],
            )?;
        }
        if let Some(candidate) = intent {
            let sync_enabled = candidate.version.map(|_| bool_int(enabled));
            let written = tx.execute(
                "INSERT INTO plugin_sync_outbox(origin,account_id,plugin_id,generation,config_generation,version,enabled,reviewed_base)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,'direct')
                 ON CONFLICT(origin,account_id,plugin_id) DO UPDATE SET
                    generation=excluded.generation,config_generation=excluded.config_generation,
                    runtime_epoch=NULL,reviewed_base='direct',version=excluded.version,enabled=excluded.enabled
                 WHERE plugin_sync_outbox.config_generation IS NOT NULL",
                params![candidate.origin, candidate.account_id, id, generation, candidate.config_generation.to_string(), candidate.version, sync_enabled],
            )?;
            if written != 1 {
                return Err(DatabaseError::Invalid(
                    "plugin sync consent needs review".into(),
                ));
            }
        }
        tx.commit()?;
        Ok(generation)
    }
}
