//! Metadata-only native library intents. No source is retained or replayed.
use super::{commands, DatabaseError, DatabaseManager};
use crate::{
    collaboration::{CommandOutcome, CommandRequest, CommandState, ResourceKind},
    control::{
        owner_library::{library_hash, library_now, library_record_id, library_rejection},
        owner_library_command_hash, owner_library_versions_digest, validate_owner_library_submit,
        OwnerLibraryIntent, OwnerLibraryOutcome, OwnerLibraryReceipt, OwnerLibraryRecord,
        OwnerLibraryState, OwnerLibrarySubmit,
    },
};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

const HISTORY: usize = 16;
const ENTRY_BYTES: usize = 8 * 1024;
const VERSIONS_BYTES: usize = 8 * 1024 * 1024 - 256 * 1024;
const MAX_TIMESTAMP: i64 = 9_007_199_254_740_991;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    version: u8,
    host_id: String,
    intent: OwnerLibraryIntent,
    record: Option<OwnerLibraryRecord>,
}
fn invalid() -> DatabaseError {
    DatabaseError::Invalid("Owner library journal is invalid".into())
}
fn revision(value: &str) -> Option<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|number| *number >= 0 && number.to_string() == value)
}

fn validate_receipt(
    intent: &OwnerLibraryIntent,
    receipt: &OwnerLibraryReceipt,
) -> Result<(), DatabaseError> {
    let before = revision(&intent.expected_revision).ok_or_else(invalid)?;
    if revision(&receipt.revision) != before.checked_add(1)
        || receipt.affected_resources == 0
        || !library_hash(&receipt.catalog_digest)
        || !library_hash(&receipt.definition_digest)
        || !library_hash(&receipt.versions_digest)
    {
        return Err(invalid());
    }
    Ok(())
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
        || !crate::collaboration::valid_command_id(host)
        || request.validate().is_err()
        || !library_hash(&intent.command_hash)
        || !library_hash(&intent.definition_digest)
        || !intent
            .command_id
            .strip_prefix("owner-library-")
            .is_some_and(|suffix| {
                suffix.len() == 32
                    && suffix
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        || intent.target.is_empty()
        || intent.target.len() > 256
        || intent.target.trim() != intent.target
        || intent.target.chars().any(char::is_control)
        || !(0..=MAX_TIMESTAMP).contains(&intent.created_at)
        || pending != stored.record.is_none()
    {
        return Err(invalid());
    }
    if let Some(record) = &stored.record {
        if record.intent != *intent || !(0..=MAX_TIMESTAMP).contains(&record.recorded_at) {
            return Err(invalid());
        }
        match record.outcome {
            OwnerLibraryOutcome::Succeeded => {
                if record.error_code.is_some() {
                    return Err(invalid());
                }
                validate_receipt(intent, record.receipt.as_ref().ok_or_else(invalid)?)?;
            }
            OwnerLibraryOutcome::Rejected => {
                if record.receipt.is_some()
                    || !record.error_code.as_deref().is_some_and(library_rejection)
                {
                    return Err(invalid());
                }
            }
            OwnerLibraryOutcome::ReviewedUnknown => {
                if record.receipt.is_some() || record.error_code.is_some() {
                    return Err(invalid());
                }
            }
        }
    }
    Ok(())
}
fn encoded(stored: &Stored) -> Result<String, DatabaseError> {
    let value = serde_json::to_string(stored)?;
    if value.len() > ENTRY_BYTES {
        return Err(invalid());
    }
    Ok(value)
}
fn rows(db: &Connection, host: &str) -> Result<Vec<Stored>, DatabaseError> {
    let mut statement=db.prepare("SELECT command_id,pending,CASE WHEN length(CAST(payload_json AS BLOB))<=8192 THEN payload_json ELSE NULL END FROM owner_automation_library ORDER BY rowid DESC LIMIT 18")?;
    let mut query = statement.query([])?;
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
        let pending: i64 = row.get(1)?;
        if !matches!(pending, 0 | 1) {
            return Err(invalid());
        }
        let encoded: Option<String> = row.get(2)?;
        let stored: Stored = serde_json::from_str(&encoded.ok_or_else(invalid)?)?;
        validate(&stored, host, pending == 1)?;
        if stored.intent.command_id != command {
            return Err(invalid());
        }
        result.push(stored);
    }
    if result.iter().filter(|row| row.record.is_none()).count() > 1 {
        return Err(invalid());
    }
    Ok(result)
}
fn pending(db: &Connection, host: &str) -> Result<Option<Stored>, DatabaseError> {
    Ok(rows(db, host)?.into_iter().find(|row| row.record.is_none()))
}
pub(super) fn has_pending(db: &Connection, host: &str) -> Result<bool, DatabaseError> {
    Ok(pending(db, host)?.is_some())
}

fn complete(
    db: &Connection,
    mut stored: Stored,
    outcome: OwnerLibraryOutcome,
    receipt: Option<OwnerLibraryReceipt>,
    error_code: Option<String>,
) -> Result<(), DatabaseError> {
    let at = library_now();
    stored.record = Some(OwnerLibraryRecord {
        intent: stored.intent.clone(),
        outcome,
        receipt,
        error_code,
        recorded_at: at,
    });
    validate(&stored, &stored.host_id, false)?;
    if db.execute("UPDATE owner_automation_library SET pending=0,payload_json=?2,updated_at=?3 WHERE command_id=?1 AND pending=1",params![stored.intent.command_id,encoded(&stored)?,at])? != 1 {return Err(invalid());}
    db.execute("DELETE FROM owner_automation_library WHERE pending=0 AND command_id NOT IN (SELECT command_id FROM owner_automation_library WHERE pending=0 ORDER BY rowid DESC LIMIT 16)",[])?;
    Ok(())
}

fn bound_ledger(db: &Connection, stored: &Stored) -> Result<bool, DatabaseError> {
    let mut statement=db.prepare("SELECT payload_hash,session_id,method,resource_kind,resource_scope,resource_id,host_instance_id,state,length(CAST(result_json AS BLOB)),length(CAST(versions_json AS BLOB)),outcome_expired,committed_at FROM control_commands WHERE principal_id='local-owner' AND command_id=?1")?;
    let mut query = statement.query([&stored.intent.command_id])?;
    let Some(row) = query.next()? else {
        return Ok(false);
    };
    for (column, max) in [
        (0, 64),
        (1, 128),
        (2, 64),
        (3, 32),
        (4, 7),
        (5, 128),
        (6, 128),
        (7, 16),
    ] {
        match row.get_ref(column)? {
            rusqlite::types::ValueRef::Text(bytes) if bytes.len() <= max => {}
            _ => return Err(invalid()),
        }
    }
    for (column, max) in [(8, ENTRY_BYTES as i64), (9, VERSIONS_BYTES as i64)] {
        if row
            .get::<_, Option<i64>>(column)?
            .is_some_and(|size| size < 0 || size > max)
        {
            return Err(invalid());
        }
    }
    if row.get::<_, String>(0)? != stored.intent.command_hash
        || row.get::<_, String>(1)? != "local-owner"
        || row.get::<_, String>(2)? != "automation.library.apply"
        || row.get::<_, String>(3)? != "automation"
        || row.get::<_, String>(4)? != "library"
        || row.get::<_, String>(5)? != "catalog"
        || row.get::<_, String>(6)? != stored.intent.host_instance_id
        || row.get::<_, String>(7)? != "succeeded"
        || !matches!(row.get::<_, i64>(10)?, 0 | 1)
        || !(1..=MAX_TIMESTAMP).contains(&row.get::<_, i64>(11)?)
    {
        return Err(invalid());
    }
    Ok(true)
}
fn receipt(
    stored: &Stored,
    outcome: &CommandOutcome,
) -> Result<OwnerLibraryReceipt, DatabaseError> {
    let intent = &stored.intent;
    let resource = outcome.command.resource.as_ref().ok_or_else(invalid)?;
    let primary = outcome.resource_versions.first().ok_or_else(invalid)?;
    let binding = outcome.result.as_object().ok_or_else(invalid)?;
    if binding.len() != 5
        || binding.get("operation") != Some(&serde_json::to_value(intent.operation)?)
        || binding.get("target").and_then(serde_json::Value::as_str) != Some(intent.target.as_str())
        || binding
            .get("expectedRevision")
            .and_then(serde_json::Value::as_str)
            != Some(intent.expected_revision.as_str())
        || binding
            .get("definitionDigest")
            .and_then(serde_json::Value::as_str)
            != Some(intent.definition_digest.as_str())
    {
        return Err(invalid());
    }
    let receipt: OwnerLibraryReceipt =
        serde_json::from_value(binding.get("receipt").cloned().ok_or_else(invalid)?)?;
    validate_receipt(intent, &receipt)?;
    if outcome.principal_id != "local-owner"
        || outcome.method != "automation.library.apply"
        || outcome.host_instance_id != intent.host_instance_id
        || outcome.command.command_id != intent.command_id
        || outcome.command.state != CommandState::Succeeded
        || !outcome
            .command
            .committed_at
            .is_some_and(|at| (1..=MAX_TIMESTAMP).contains(&at))
        || resource.kind != ResourceKind::Automation
        || resource.scope != "library"
        || resource.id != "catalog"
        || primary.kind != resource.kind
        || primary.scope != resource.scope
        || primary.id != resource.id
        || primary.revision != receipt.revision
        || usize::try_from(receipt.affected_resources).ok() != Some(outcome.resource_versions.len())
        || receipt.versions_digest != owner_library_versions_digest(&outcome.resource_versions)
    {
        return Err(invalid());
    }
    let mut last = None;
    for version in outcome.resource_versions.iter().skip(1) {
        if version.kind != ResourceKind::Automation
            || !matches!(version.scope.as_str(), "action" | "event")
            || !library_record_id(&version.id)
            || !revision(&version.revision).is_some_and(|number| number > 0)
            || last.is_some_and(|last| last >= (&version.scope, &version.id))
        {
            return Err(invalid());
        }
        last = Some((&version.scope, &version.id));
    }
    Ok(receipt)
}

impl DatabaseManager {
    pub(crate) fn owner_library_state(
        &self,
        host: &str,
        instance: &str,
    ) -> Result<OwnerLibraryState, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let mut state = OwnerLibraryState {
            host_id: host.into(),
            host_instance_id: instance.into(),
            pending: None,
            recent: Vec::new(),
        };
        for stored in rows(&tx, host)? {
            if let Some(row) = stored.record {
                state.recent.push(row);
            } else {
                state.pending = Some(stored.intent);
            }
        }
        tx.commit()?;
        Ok(state)
    }
    pub(crate) fn owner_library_begin(
        &self,
        host: &str,
        intent: &OwnerLibraryIntent,
        input: &OwnerLibrarySubmit,
    ) -> Result<(), DatabaseError> {
        validate_owner_library_submit(input).map_err(|_| invalid())?;
        if host != input.host_id
            || intent.operation != input.mutation.operation()
            || intent.target != input.mutation.target()
            || intent.expected_revision != input.expected_revision
            || intent.definition_digest != input.definition_digest
            || intent.host_instance_id != input.host_instance_id
            || intent.provenance != input.provenance
            || intent.command_hash != owner_library_command_hash(input)
        {
            return Err(invalid());
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if has_pending(&tx, host)?
            || super::owner_adoption::has_pending(&tx, host)?
            || super::owner_approval::has_pending(&tx, host)?
            || super::owner_definition::has_pending(&tx, host)?
        {
            return Err(DatabaseError::OwnerAdoptionPending);
        }
        let stored = Stored {
            version: 1,
            host_id: host.into(),
            intent: intent.clone(),
            record: None,
        };
        validate(&stored, host, true)?;
        tx.execute("INSERT INTO owner_automation_library(command_id,pending,payload_json,updated_at) VALUES (?1,1,?2,?3)",params![intent.command_id,encoded(&stored)?,intent.created_at])?;
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn owner_library_resolve(
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
        if bound_ledger(&tx, &stored)? {
            match commands::lookup(&tx, "local-owner", &stored.intent.command_id) {
                Ok(Some(outcome)) => {
                    let receipt = receipt(&stored, &outcome)?;
                    complete(
                        &tx,
                        stored,
                        OwnerLibraryOutcome::Succeeded,
                        Some(receipt),
                        None,
                    )?;
                }
                Err(DatabaseError::CommandExpired) if review_unknown => complete(
                    &tx,
                    stored,
                    OwnerLibraryOutcome::ReviewedUnknown,
                    None,
                    None,
                )?,
                Err(DatabaseError::CommandExpired) => {}
                Err(error) => return Err(error),
                Ok(None) => return Err(invalid()),
            }
        } else if rejection.is_some_and(library_rejection) {
            complete(
                &tx,
                stored,
                OwnerLibraryOutcome::Rejected,
                None,
                rejection.map(str::to_owned),
            )?;
        } else if review_unknown {
            complete(
                &tx,
                stored,
                OwnerLibraryOutcome::ReviewedUnknown,
                None,
                None,
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}
