//! Bounded audit import, retention and history queries.
use super::*;

impl CollaborationService {
    /// Idempotent import of closed metadata from an owning database's outbox.
    /// The source acknowledges only after this transaction and publication.
    pub fn import_audit(&self, records: &[AuditRecord]) -> Result<usize, Error> {
        if records.len() > 100 {
            return Err(Error::Invalid("audit import batch exceeds limit"));
        }
        let mut encoded = Vec::with_capacity(records.len());
        let mut bytes = 0usize;
        for record in records {
            record.validate()?;
            let json = serde_json::to_string(record)?;
            bytes = bytes.saturating_add(json.len());
            if bytes > 256 * 1024 {
                return Err(Error::Invalid("audit import exceeds byte limit"));
            }
            let hash = hash_secret(&json);
            encoded.push((record, json, hash));
        }
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        prune_audit(&tx)?;
        let cutoff = now().saturating_sub(30 * IDLE_TTL);
        let mut inserted = 0;
        for (record, json, hash) in encoded {
            if record.timestamp < cutoff {
                continue;
            }
            let previous: Option<String> = tx
                .query_row(
                    "SELECT payload_hash FROM audit_imports WHERE event_id=?1",
                    [&record.event_id],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(previous) = previous {
                if previous != hash {
                    return Err(Error::Invalid("audit event ID was reused"));
                }
                continue;
            }
            tx.execute("INSERT INTO audit(timestamp,principal_id,action,resource_id,outcome) VALUES (?1,?2,?3,?4,?5)",params![record.timestamp,record.principal_id,record.method,record.resource.id,record.result_code])?;
            let cursor = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO audit_details(cursor,details_json) VALUES (?1,?2)",
                params![cursor, json],
            )?;
            tx.execute("INSERT INTO audit_imports(event_id,payload_hash,source_timestamp) VALUES (?1,?2,?3)",params![record.event_id,hash,record.timestamp])?;
            inserted += 1;
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// The host calls this periodically only for an already-open store.
    pub fn prune_audit(&self) -> Result<(), Error> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        prune_audit(&tx)?;
        tx.commit()?;
        Ok(())
    }

    pub fn audit(&self, input: AuditQuery) -> Result<Vec<AuditEntry>, Error> {
        if input.after < 0 {
            return Err(Error::Invalid("invalid audit cursor"));
        }
        let limit = input.limit.unwrap_or(50);
        if !(1..=100).contains(&limit) {
            return Err(Error::Invalid("audit page limit must be between 1 and 100"));
        }
        self.prune_audit()?;
        let db = self.db()?;
        let mut stmt=db.prepare("SELECT a.cursor,a.timestamp,a.principal_id,a.action,a.resource_id,a.outcome,d.details_json FROM audit a LEFT JOIN audit_details d ON d.cursor=a.cursor WHERE a.cursor>?1 ORDER BY a.cursor LIMIT ?2")?;
        let rows = stmt
            .query_map(params![input.after, limit as i64], |r| {
                Ok(AuditEntry {
                    cursor: r.get(0)?,
                    timestamp: r.get(1)?,
                    principal_id: r.get(2)?,
                    action: r.get(3)?,
                    resource_id: r.get(4)?,
                    outcome: r.get(5)?,
                    details: r
                        .get::<_, Option<String>>(6)?
                        .map(|json| {
                            serde_json::from_str(&json).map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    6,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            })
                        })
                        .transpose()?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

pub(super) fn audit(
    db: &Connection,
    actor: &str,
    action: &str,
    resource: &str,
    outcome: &str,
) -> Result<(), Error> {
    db.execute("INSERT INTO audit(timestamp,principal_id,action,resource_id,outcome) VALUES (?1,?2,?3,?4,?5)", params![now(), actor, action, resource, outcome])?;
    prune_audit(db)?;
    Ok(())
}

pub(super) fn prune_audit(db: &Connection) -> Result<(), Error> {
    let cutoff = now().saturating_sub(30 * IDLE_TTL);
    db.execute("DELETE FROM audit WHERE timestamp<?1", [cutoff])?;
    db.execute(
        "DELETE FROM audit_imports WHERE source_timestamp<?1",
        [cutoff],
    )?;
    Ok(())
}
