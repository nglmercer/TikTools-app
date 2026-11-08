//! Metadata-only owner journal. Business results remain in the shared ledger.
use super::{commands, DatabaseError, DatabaseManager};
use crate::{
    collaboration::{CommandRequest, ResourceKind},
    control::{
        automation_adoption::{adoption_now, adoption_rejection},
        AutomationKind, AutomationMutation, OwnerAdoptionIntent, OwnerAdoptionOutcome,
        OwnerAdoptionRecord, OwnerAdoptionState,
    },
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

// Insertion order is durable; wall-clock corrections cannot prune the newest
// completed intent. The single pending gate serializes begin/completion order.
const HISTORY: usize = 16;
const ENTRY_BYTES: usize = 8192;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    version: u8,
    host_id: String,
    payload_hash: String,
    intent: OwnerAdoptionIntent,
    record: Option<OwnerAdoptionRecord>,
}
fn invalid() -> DatabaseError {
    DatabaseError::Invalid("owner adoption journal is invalid".into())
}
fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && !value.contains('*')
        && !value.chars().any(char::is_control)
}
fn revision(value: &str) -> Option<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|n| *n >= 0 && n.to_string() == value)
}
fn scope(kind: AutomationKind) -> &'static str {
    match kind {
        AutomationKind::Event => "event",
        AutomationKind::Action => "action",
    }
}
fn validate(stored: &Stored, host: &str, pending: bool) -> Result<(), DatabaseError> {
    let intent = &stored.intent;
    let request = CommandRequest {
        command_id: intent.command_id.clone(),
        host_instance_id: intent.host_instance_id.clone(),
        expected_revision: intent.expected_revision.clone(),
    };
    if stored.version != 1
        || stored.host_id != host
        || !id(host)
        || !id(&intent.id)
        || request.validate().is_err()
        || !intent
            .command_id
            .strip_prefix("owner-adopt-")
            .is_some_and(|suffix| {
                suffix.len() == 32
                    && suffix
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        || revision(&intent.expected_revision).is_none()
        || !(0..=9_007_199_254_740_991).contains(&intent.created_at)
        || stored.payload_hash.len() != 64
        || !stored.payload_hash.bytes().all(|b| b.is_ascii_hexdigit())
        || pending != stored.record.is_none()
    {
        return Err(invalid());
    }
    if let Some(record) = &stored.record {
        if record.intent != *intent || !(0..=9_007_199_254_740_991).contains(&record.recorded_at) {
            return Err(invalid());
        }
        match record.outcome {
            OwnerAdoptionOutcome::Succeeded => {
                let before = revision(&intent.expected_revision).ok_or_else(invalid)?;
                let after = record
                    .revision
                    .as_deref()
                    .and_then(revision)
                    .ok_or_else(invalid)?;
                if after <= 0
                    || record.error_code.is_some()
                    || !(after == before || before.checked_add(1) == Some(after))
                {
                    return Err(invalid());
                }
            }
            OwnerAdoptionOutcome::Rejected => {
                if record.revision.is_some()
                    || !record.error_code.as_deref().is_some_and(adoption_rejection)
                {
                    return Err(invalid());
                }
            }
            OwnerAdoptionOutcome::ReviewedUnknown => {
                if record.revision.is_some() || record.error_code.is_some() {
                    return Err(invalid());
                }
            }
        }
    }
    Ok(())
}
fn rows(db: &Connection, host: &str) -> Result<Vec<Stored>, DatabaseError> {
    let mut stmt=db.prepare("SELECT command_id,pending,CASE WHEN length(CAST(payload_json AS BLOB))<=8192 THEN payload_json ELSE NULL END FROM owner_automation_adoptions ORDER BY rowid DESC LIMIT 18")?;
    let mut query = stmt.query([])?;
    let mut result = Vec::new();
    while let Some(row) = query.next()? {
        if result.len() > HISTORY {
            return Err(invalid());
        }
        match row.get_ref(0)? {
            rusqlite::types::ValueRef::Text(bytes) if bytes.len() <= 128 => {}
            _ => return Err(invalid()),
        }
        let command: String = row.get(0)?;
        let flag: i64 = row.get(1)?;
        if !matches!(flag, 0 | 1) {
            return Err(invalid());
        }
        let pending = flag == 1;
        let bytes: Option<String> = row.get(2)?;
        let stored: Stored = serde_json::from_str(&bytes.ok_or_else(invalid)?)?;
        validate(&stored, host, pending)?;
        if stored.intent.command_id != command {
            return Err(invalid());
        }
        result.push(stored);
    }
    if result.iter().filter(|r| r.record.is_none()).count() > 1 {
        return Err(invalid());
    }
    Ok(result)
}
fn pending(db: &Connection, host: &str) -> Result<Option<Stored>, DatabaseError> {
    Ok(rows(db, host)?.into_iter().find(|r| r.record.is_none()))
}
pub(super) fn has_pending(db: &Connection, host: &str) -> Result<bool, DatabaseError> {
    Ok(pending(db, host)?.is_some())
}
fn encoded(stored: &Stored) -> Result<String, DatabaseError> {
    let bytes = serde_json::to_string(stored)?;
    if bytes.len() > ENTRY_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}
fn complete(
    db: &Connection,
    mut stored: Stored,
    outcome: OwnerAdoptionOutcome,
    revision: Option<String>,
    error_code: Option<String>,
) -> Result<(), DatabaseError> {
    let recorded_at = adoption_now();
    stored.record = Some(OwnerAdoptionRecord {
        intent: stored.intent.clone(),
        outcome,
        revision,
        error_code,
        recorded_at,
    });
    validate(&stored, &stored.host_id, false)?;
    let changed=db.execute("UPDATE owner_automation_adoptions SET pending=0,payload_json=?2,updated_at=?3 WHERE command_id=?1 AND pending=1",params![stored.intent.command_id,encoded(&stored)?,recorded_at])?;
    if changed != 1 {
        return Err(invalid());
    }
    db.execute("DELETE FROM owner_automation_adoptions WHERE pending=0 AND command_id NOT IN (SELECT command_id FROM owner_automation_adoptions WHERE pending=0 ORDER BY rowid DESC LIMIT 16)",[])?;
    Ok(())
}
impl DatabaseManager {
    pub(crate) fn owner_adoption_state(
        &self,
        host: &str,
        instance: &str,
    ) -> Result<OwnerAdoptionState, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let mut result = OwnerAdoptionState {
            host_id: host.into(),
            host_instance_id: instance.into(),
            pending: None,
            recent: Vec::new(),
        };
        for stored in rows(&tx, host)? {
            if let Some(record) = stored.record {
                result.recent.push(record);
            } else {
                result.pending = Some(stored.intent);
            }
        }
        tx.commit()?;
        Ok(result)
    }
    pub(crate) fn owner_adoption_begin(
        &self,
        host: &str,
        intent: &OwnerAdoptionIntent,
        operation: &AutomationMutation,
    ) -> Result<(), DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if pending(&tx, host)?.is_some()
            || super::owner_approval::has_pending(&tx, host)?
            || super::owner_definition::has_pending(&tx, host)?
            || super::owner_library::has_pending(&tx, host)?
        {
            return Err(DatabaseError::OwnerAdoptionPending);
        }
        let (kind, target) = operation.target();
        if !matches!(operation, AutomationMutation::Adopt { .. })
            || kind != intent.kind
            || target != intent.id
        {
            return Err(invalid());
        }
        let hash = commands::payload_hash(
            "automation.adopt",
            &operation.params(),
            &intent.expected_revision,
        )?;
        let stored = Stored {
            version: 1,
            host_id: host.into(),
            payload_hash: hash,
            intent: intent.clone(),
            record: None,
        };
        validate(&stored, host, true)?;
        tx.execute("INSERT INTO owner_automation_adoptions(command_id,pending,payload_json,updated_at) VALUES (?1,1,?2,?3)",params![intent.command_id,encoded(&stored)?,intent.created_at])?;
        tx.commit()?;
        Ok(())
    }
    /// One final ledger lookup and journal transition, without dispatching work.
    /// A known result wins over the user's stale 'unknown' acknowledgement.
    pub(crate) fn owner_adoption_resolve(
        &self,
        host: &str,
        review_unknown: bool,
        rejection: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(stored) = pending(&tx, host)? else {
            return Ok(());
        };
        let intent = &stored.intent;
        let ledger:Option<(String,String)>=tx.query_row("SELECT payload_hash,session_id FROM control_commands WHERE principal_id='local-owner' AND command_id=?1",[&intent.command_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        match ledger {
            Some((hash, session)) => {
                if hash != stored.payload_hash || session != "local-owner" {
                    return Err(invalid());
                }
                match commands::lookup(&tx, "local-owner", &intent.command_id) {
                    Ok(Some(outcome)) => {
                        let resource = outcome.command.resource.as_ref().ok_or_else(invalid)?;
                        let primary = outcome.resource_versions.first().ok_or_else(invalid)?;
                        let body = outcome.result.as_object().ok_or_else(invalid)?;
                        if outcome.method != "automation.adopt"
                            || outcome.host_instance_id != intent.host_instance_id
                            || outcome.command.command_id != intent.command_id
                            || outcome.command.state
                                != crate::collaboration::CommandState::Succeeded
                            || resource.kind != ResourceKind::Automation
                            || resource.scope != scope(intent.kind)
                            || resource.id != intent.id
                            || outcome.resource_versions.len() != 1
                            || primary.kind != resource.kind
                            || primary.scope != resource.scope
                            || primary.id != resource.id
                            || body.len() != 2
                            || body.get("id").and_then(serde_json::Value::as_str)
                                != Some(intent.id.as_str())
                            || body.get("adopted") != Some(&serde_json::Value::Bool(true))
                        {
                            return Err(invalid());
                        }
                        let version = primary.revision.clone();
                        complete(
                            &tx,
                            stored,
                            OwnerAdoptionOutcome::Succeeded,
                            Some(version),
                            None,
                        )?;
                    }
                    Err(DatabaseError::CommandExpired) if review_unknown => {
                        complete(
                            &tx,
                            stored,
                            OwnerAdoptionOutcome::ReviewedUnknown,
                            None,
                            None,
                        )?;
                    }
                    Err(DatabaseError::CommandExpired) => {}
                    Err(error) => return Err(error),
                    Ok(None) => return Err(invalid()),
                }
            }
            None if rejection.is_some_and(adoption_rejection) => {
                complete(
                    &tx,
                    stored,
                    OwnerAdoptionOutcome::Rejected,
                    None,
                    rejection.map(str::to_owned),
                )?;
            }
            None if review_unknown => {
                complete(
                    &tx,
                    stored,
                    OwnerAdoptionOutcome::ReviewedUnknown,
                    None,
                    None,
                )?;
            }
            None => {}
        }
        tx.commit()?;
        Ok(())
    }
}
