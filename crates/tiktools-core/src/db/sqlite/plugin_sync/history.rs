use super::*;

// A package's files and runtime can be between states while this journal is
// active. Treat an invalid NULL identity as global uncertainty, not absence.
pub(super) fn package_source_available(
    db: &rusqlite::Connection,
    id: &str,
) -> Result<bool, DatabaseError> {
    Ok(!db.query_row(
        "SELECT EXISTS(SELECT 1 FROM plugin_package_mutation_intents WHERE plugin_id=?1 OR plugin_id IS NULL)",
        [id], |row| row.get::<_, bool>(0),
    )?)
}

/// Retained, independently witnessed finalized applications are history, not
/// pending work. Validate both copies under the caller's SQLite transaction;
/// an active, malformed, orphaned or future-generation record blocks syncing.
pub(super) fn apply_history_allows_source(
    db: &rusqlite::Connection,
    id: &str,
) -> Result<bool, DatabaseError> {
    use super::plugin_sync_apply::{decode_stored, PluginApplyPhase};
    let generation: Option<i64> = db
        .query_row(
            "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id=?1",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    let mut query = db.prepare(
        "SELECT i.intent_id,i.plugin_id,i.phase,i.payload_json,i.committed_generation,i.runtime_epoch,
                f.intent_id,f.plugin_id,f.payload_json,f.committed_generation,f.runtime_epoch
         FROM plugin_sync_apply_intents i LEFT JOIN plugin_sync_apply_finalizations f ON f.intent_id=i.intent_id
         WHERE i.plugin_id=?1 OR f.plugin_id=?1
         UNION ALL
         SELECT NULL,NULL,NULL,NULL,NULL,NULL,f.intent_id,f.plugin_id,f.payload_json,f.committed_generation,f.runtime_epoch
         FROM plugin_sync_apply_finalizations f LEFT JOIN plugin_sync_apply_intents i ON i.intent_id=f.intent_id
         WHERE i.intent_id IS NULL AND f.plugin_id=?1 LIMIT ?2"
    )?;
    let mut rows = query.query(params![
        id,
        (super::plugin_apply_quarantine::MAX_RETAINED + 1) as i64
    ])?;
    let mut count = 0usize;
    while let Some(row) = rows.next()? {
        count += 1;
        if count > super::plugin_apply_quarantine::MAX_RETAINED {
            return Ok(false);
        }
        let Some(intent_id): Option<String> = row.get(0)? else {
            return Ok(false);
        };
        let plugin_id: String = row.get(1)?;
        let phase: String = row.get(2)?;
        let payload: String = row.get(3)?;
        let committed: Option<i64> = row.get(4)?;
        let epoch: Option<String> = row.get(5)?;
        if plugin_id != id
            || phase != "finalized"
            || row.get::<_, Option<String>>(6)?.as_deref() != Some(intent_id.as_str())
            || row.get::<_, Option<String>>(7)?.as_deref() != Some(id)
            || row.get::<_, Option<String>>(8)?.as_deref() != Some(payload.as_str())
            || row.get::<_, Option<i64>>(9)? != committed
            || row.get::<_, Option<String>>(10)? != epoch
            || !matches!((generation, committed), (Some(current), Some(saved)) if current >= saved)
        {
            return Ok(false);
        }
        let parsed = decode_stored(&intent_id, (plugin_id, phase, payload, committed, epoch))?;
        if parsed.phase != PluginApplyPhase::Finalized {
            return Ok(false);
        }
    }
    Ok(true)
}
