use super::*;

impl DatabaseManager {
    /// Move the exact confirmed outbox entry to a durable receipt after its
    /// corresponding v1 pending value has been saved. The two SQLite writes
    /// share one transaction; interruption before commit retains the outbox.
    pub(crate) fn receipt_plugin_sync_intent(
        &self,
        entry: &PluginSyncOutboxEntry,
        runtime_epoch: &str,
        pending: &tiktools_cloud_sync::V1Pending,
    ) -> Result<bool, DatabaseError> {
        use tiktools_cloud_sync::{PluginSelection, V1Edit};

        if !valid_runtime_epoch(runtime_epoch)
            || entry.runtime_epoch.as_deref() != Some(runtime_epoch)
            || entry.source_generation != entry.generation
        {
            return Ok(false);
        }
        let expected = match (&entry.version, entry.enabled) {
            (Some(version), Some(enabled)) => V1Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: entry.plugin_id.clone(),
                version: version.clone(),
                enabled,
            }),
            (None, None) => V1Edit::Delete,
            _ => return Ok(false),
        };
        if pending.edit != expected {
            return Ok(false);
        }
        let pending_json = serde_json::to_string(pending)
            .map_err(|_| DatabaseError::Invalid("invalid staged sync value".into()))?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let matching: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_outbox o JOIN plugin_sync_source_versions s ON s.plugin_id=o.plugin_id
             WHERE o.origin=?1 AND o.account_id=?2 AND o.plugin_id=?3 AND o.generation=?4
             AND o.config_generation=?5 AND o.runtime_epoch=?6 AND o.version IS ?7 AND o.enabled IS ?8
             AND s.generation=o.generation
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=o.plugin_id OR p.plugin_id IS NULL))",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation,
                entry.config_generation.to_string(), runtime_epoch, entry.version, entry.enabled.map(bool_int)],
            |row| row.get(0),
        )?;
        if !matching {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO plugin_sync_receipts(origin,account_id,plugin_id,generation,config_generation,runtime_epoch,pending_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(origin,account_id,plugin_id) DO UPDATE SET generation=excluded.generation,
                 config_generation=excluded.config_generation,runtime_epoch=excluded.runtime_epoch,pending_json=excluded.pending_json",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation,
                entry.config_generation.to_string(), runtime_epoch, pending_json],
        )?;
        let removed = tx.execute(
            "DELETE FROM plugin_sync_outbox WHERE origin=?1 AND account_id=?2 AND plugin_id=?3 AND generation=?4",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation],
        )?;
        if removed != 1 {
            return Err(DatabaseError::Invalid(
                "sync receipt transaction changed".into(),
            ));
        }
        tx.commit()?;
        Ok(true)
    }

    /// A receipt authorizes only the identical pending value for this exact
    /// identity, consent epoch and observed runtime, while the local source
    /// generation remains unchanged. Call under the admission lease.
    pub(crate) fn validate_plugin_sync_receipt(
        &self,
        origin: &str,
        account: &str,
        plugin_id: &str,
        config_generation: u128,
        runtime_epoch: &str,
        pending: &tiktools_cloud_sync::V1Pending,
    ) -> Result<bool, DatabaseError> {
        if !valid_runtime_epoch(runtime_epoch) {
            return Ok(false);
        }
        let db = self.open(&self.automation_path())?;
        let stored: Option<String> = db.query_row(
            "SELECT r.pending_json FROM plugin_sync_receipts r JOIN plugin_sync_source_versions s ON s.plugin_id=r.plugin_id
             WHERE r.origin=?1 AND r.account_id=?2 AND r.plugin_id=?3 AND r.config_generation=?4
             AND r.runtime_epoch=?5 AND r.generation=s.generation
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=r.plugin_id OR p.plugin_id IS NULL)",
            params![origin, account, plugin_id, config_generation.to_string(), runtime_epoch],
            |row| row.get(0),
        ).optional()?;
        Ok(stored
            .and_then(|value| serde_json::from_str::<tiktools_cloud_sync::V1Pending>(&value).ok())
            .as_ref()
            == Some(pending))
    }

    /// A 409 may change only the pending conflict flag after staging. Return
    /// the exact current source generation only if the saved receipt still
    /// matches the reviewed edit and base, with no undrained edit for this
    /// plugin in any account. Call under the shared product admission lease.
    pub(crate) fn plugin_sync_conflict_source(
        &self,
        origin: &str,
        account: &str,
        plugin_id: &str,
        config_generation: u128,
        runtime_epoch: &str,
        pending: &tiktools_cloud_sync::V1Pending,
    ) -> Result<Option<i64>, DatabaseError> {
        use tiktools_cloud_sync::V1Edit;

        if !pending.conflict || !valid_runtime_epoch(runtime_epoch) {
            return Ok(None);
        }
        let (installed, enabled) = match &pending.edit {
            V1Edit::Put(value) if value.plugin_id == plugin_id => (true, Some(value.enabled)),
            V1Edit::Delete => (false, None),
            _ => return Ok(None),
        };
        let db = self.open(&self.automation_path())?;
        let stored: Option<(i64, String)> = db.query_row(
            "SELECT r.generation,r.pending_json FROM plugin_sync_receipts r JOIN plugin_sync_source_versions s ON s.plugin_id=r.plugin_id
             JOIN behavior_plugins b ON b.id=r.plugin_id
             WHERE r.origin=?1 AND r.account_id=?2 AND r.plugin_id=?3 AND r.config_generation=?4
             AND r.runtime_epoch=?5 AND r.generation=s.generation
             AND b.installed=?6 AND (?7 IS NULL OR b.enabled=?7)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=r.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=r.plugin_id OR p.plugin_id IS NULL)",
            params![origin, account, plugin_id, config_generation.to_string(), runtime_epoch,
                bool_int(installed), enabled.map(bool_int)],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        let Some((generation, json)) = stored else {
            return Ok(None);
        };
        let Some(mut saved) = serde_json::from_str::<tiktools_cloud_sync::V1Pending>(&json).ok()
        else {
            return Ok(None);
        };
        saved.conflict = true;
        Ok((generation > 0 && saved == *pending).then_some(generation))
    }

    /// Reconfirm a retained receipt after a host restart, only after the
    /// caller has observed the exact desired runtime and package version under
    /// its mutation gate. This does not itself start, stop or install a plugin.
    pub(crate) fn confirm_plugin_sync_receipt_runtime(
        &self,
        origin: &str,
        account: &str,
        plugin_id: &str,
        config_generation: u128,
        runtime_epoch: &str,
        pending: &tiktools_cloud_sync::V1Pending,
    ) -> Result<bool, DatabaseError> {
        use tiktools_cloud_sync::V1Edit;

        if !valid_runtime_epoch(runtime_epoch) {
            return Ok(false);
        }
        let (installed, enabled) = match &pending.edit {
            V1Edit::Put(value) if value.plugin_id == plugin_id => (true, Some(value.enabled)),
            V1Edit::Delete => (false, None),
            _ => return Ok(false),
        };
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored: Option<String> = tx.query_row(
            "SELECT r.pending_json FROM plugin_sync_receipts r JOIN plugin_sync_source_versions s ON s.plugin_id=r.plugin_id
             JOIN behavior_plugins b ON b.id=r.plugin_id WHERE r.origin=?1 AND r.account_id=?2 AND r.plugin_id=?3
             AND r.config_generation=?4 AND r.generation=s.generation AND b.installed=?5
             AND (?6 IS NULL OR b.enabled=?6)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=r.plugin_id OR p.plugin_id IS NULL)",
            params![origin, account, plugin_id, config_generation.to_string(), bool_int(installed), enabled.map(bool_int)],
            |row| row.get(0),
        ).optional()?;
        let Some(stored) = stored else {
            return Ok(false);
        };
        if serde_json::from_str::<tiktools_cloud_sync::V1Pending>(&stored)
            .ok()
            .as_ref()
            != Some(pending)
        {
            return Ok(false);
        }
        let changed = tx.execute(
            "UPDATE plugin_sync_receipts SET runtime_epoch=?1 WHERE origin=?2 AND account_id=?3 AND plugin_id=?4
             AND config_generation=?5 AND pending_json=?6",
            params![runtime_epoch, origin, account, plugin_id, config_generation.to_string(), stored],
        )?;
        tx.commit()?;
        Ok(changed == 1)
    }
}
