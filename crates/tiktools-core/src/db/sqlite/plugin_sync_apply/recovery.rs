use super::*;

impl DatabaseManager {
    /// Read retained evidence after a crash; it is never permission to replay.
    pub(crate) fn plugin_apply_intent(
        &self,
        id: &str,
    ) -> Result<Option<StoredPluginApplyIntent>, DatabaseError> {
        if !valid_token(id) {
            return Ok(None);
        }
        let db = self.open(&self.automation_path())?;
        let row: Option<StoredRow> = db.query_row(
            "SELECT plugin_id,phase,payload_json,committed_generation,runtime_epoch FROM plugin_sync_apply_intents WHERE intent_id=?1", [id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
        ).optional()?;
        row.map(|row| decode_stored(id, row)).transpose()
    }

    /// Discover unresolved native application journals after restart. This is
    /// only a bounded inventory, never authorization to apply or replay. The
    /// caller must hold the shared admission lease and current-session fence
    /// for the complete page sequence; otherwise concurrent insertions may
    /// sort before the cursor. Every scanned row is validated, including
    /// other accounts, so corruption is never silently hidden by filtering.
    /// Rows from another identity are not returned. Old consent for the same
    /// identity sets a blocking flag but does not grant its saved evidence.
    pub(crate) fn plugin_apply_recovery_page(
        &self,
        context: &V1LocalEditContext,
        after: Option<&str>,
        limit: usize,
    ) -> Result<PluginApplyRecoveryPage, DatabaseError> {
        if !(1..=64).contains(&limit) || after.is_some_and(|cursor| !valid_token(cursor)) {
            return Err(DatabaseError::Invalid("invalid plugin apply cursor".into()));
        }
        let db = self.open(&self.automation_path())?;
        let query = if after.is_some() {
            "SELECT intent_id,plugin_id,phase,payload_json,committed_generation,runtime_epoch
             FROM plugin_sync_apply_intents WHERE phase != 'finalized' AND intent_id > ?1
             ORDER BY intent_id LIMIT ?2"
        } else {
            // Include even an invalid empty id so corruption is not hidden.
            "SELECT intent_id,plugin_id,phase,payload_json,committed_generation,runtime_epoch
             FROM plugin_sync_apply_intents WHERE phase != 'finalized' AND intent_id >= ?1
             ORDER BY intent_id LIMIT ?2"
        };
        let mut stmt = db.prepare(query)?;
        let rows = stmt
            .query_map(params![after.unwrap_or(""), (limit + 1) as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ),
                ))
            })?
            .collect::<Result<Vec<(String, StoredRow)>, _>>()?;
        let more = rows.len() > limit;
        let mut intents = Vec::new();
        let mut stale_consent = false;
        let mut cursor = None;
        for (id, row) in rows.into_iter().take(limit) {
            let saved = decode_stored(&id, row)?;
            if saved.intent.origin == context.origin
                && saved.intent.account_id == context.account_id
            {
                if saved.intent.config_generation == context.config_generation {
                    intents.push(saved);
                } else {
                    stale_consent = true;
                }
            }
            cursor = Some(id);
        }
        Ok(PluginApplyRecoveryPage {
            intents,
            stale_consent,
            next_cursor: if more { cursor } else { None },
        })
    }
}
