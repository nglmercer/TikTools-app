//! Native metadata-only definition intents. Recovery reads the immutable
//! original business ledger and never dispatches a saved mutation again.
use super::owner_journal_values::{
    digest, id, revision, scope, text_columns_within, CONTROL_COMMAND_TEXT_LIMITS,
};
use super::{commands, DatabaseError, DatabaseManager};
use crate::{
    collaboration::{CommandOutcome, CommandRequest, CommandState, ResourceKind},
    control::{
        owner_definition::{definition_now, definition_record_digest, definition_rejection},
        AutomationKind, AutomationMutation, OwnerDefinitionIntent, OwnerDefinitionOperation,
        OwnerDefinitionOutcome, OwnerDefinitionReceipt, OwnerDefinitionRecord,
        OwnerDefinitionState,
    },
};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const HISTORY: usize = 16;
const ENTRY_BYTES: usize = 8 * 1024;
const RECORD_BYTES: usize = 256 * 1024;
// Match the finite LocalOwner response allowance, without imposing the
// unrelated delegated-approval closure's 513-resource limit on action deletes.
const VERSIONS_BYTES: usize = 8 * 1024 * 1024;
const MAX_SAFE_TIMESTAMP: i64 = 9_007_199_254_740_991;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    version: u8,
    host_id: String,
    payload_hash: String,
    intent: OwnerDefinitionIntent,
    record: Option<OwnerDefinitionRecord>,
}

fn invalid() -> DatabaseError {
    DatabaseError::Invalid("owner definition journal is invalid".into())
}
fn operation_kind(
    operation: &AutomationMutation,
) -> Result<OwnerDefinitionOperation, DatabaseError> {
    Ok(match operation {
        AutomationMutation::Create { .. } => OwnerDefinitionOperation::Create,
        AutomationMutation::Update { .. } => OwnerDefinitionOperation::Update,
        AutomationMutation::Delete { .. } => OwnerDefinitionOperation::Delete,
        AutomationMutation::SetEnabled { enabled: true, .. } => OwnerDefinitionOperation::Enable,
        AutomationMutation::SetEnabled { enabled: false, .. } => OwnerDefinitionOperation::Disable,
        AutomationMutation::Adopt { .. } => return Err(invalid()),
    })
}
fn method(operation: OwnerDefinitionOperation) -> &'static str {
    match operation {
        OwnerDefinitionOperation::Create => "automation.create",
        OwnerDefinitionOperation::Update => "automation.update",
        OwnerDefinitionOperation::Delete => "automation.delete",
        OwnerDefinitionOperation::Enable => "automation.enable",
        OwnerDefinitionOperation::Disable => "automation.disable",
    }
}
fn save(operation: OwnerDefinitionOperation) -> bool {
    matches!(
        operation,
        OwnerDefinitionOperation::Create | OwnerDefinitionOperation::Update
    )
}
pub(super) fn bounded_record(value: &Value) -> Result<(), DatabaseError> {
    let mut stack = vec![(value, 0usize)];
    let mut nodes = 0usize;
    while let Some((value, depth)) = stack.pop() {
        nodes += 1;
        if depth > 64 || nodes > 65_536 {
            return Err(invalid());
        }
        let children: Box<dyn Iterator<Item = &Value> + '_> = match value {
            Value::Object(values) => Box::new(values.values()),
            Value::Array(values) => Box::new(values.iter()),
            _ => continue,
        };
        for child in children {
            if stack.len() + nodes >= 65_536 {
                return Err(invalid());
            }
            stack.push((child, depth + 1));
        }
    }
    if serde_json::to_vec(value)?.len() > RECORD_BYTES {
        return Err(invalid());
    }
    Ok(())
}

pub(crate) fn validate_owner_definition_input(
    operation: &AutomationMutation,
    expected: &str,
) -> Result<(), DatabaseError> {
    operation_kind(operation)?;
    let (_, target) = operation.target();
    if !id(target) || revision(expected).is_none() {
        return Err(invalid());
    }
    if let AutomationMutation::Create { record, .. } | AutomationMutation::Update { record, .. } =
        operation
    {
        bounded_record(record)?;
        let object = record.as_object().ok_or_else(invalid)?;
        if object.get("id").and_then(Value::as_str) != Some(target)
            || !object
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|v| !v.trim().is_empty())
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn metadata_operation(intent: &OwnerDefinitionIntent) -> Option<AutomationMutation> {
    match intent.operation {
        OwnerDefinitionOperation::Delete => Some(AutomationMutation::Delete {
            kind: intent.kind,
            id: intent.id.clone(),
        }),
        OwnerDefinitionOperation::Enable | OwnerDefinitionOperation::Disable => {
            Some(AutomationMutation::SetEnabled {
                kind: intent.kind,
                id: intent.id.clone(),
                enabled: intent.operation == OwnerDefinitionOperation::Enable,
            })
        }
        _ => None,
    }
}

fn validate_receipt(
    intent: &OwnerDefinitionIntent,
    receipt: &OwnerDefinitionReceipt,
) -> Result<(), DatabaseError> {
    let before = revision(&intent.expected_revision).ok_or_else(invalid)?;
    let after = revision(&receipt.revision).ok_or_else(invalid)?;
    if receipt.affected_resources == 0 {
        return Err(invalid());
    }
    match intent.operation {
        OwnerDefinitionOperation::Create | OwnerDefinitionOperation::Update => {
            if Some(after) != before.checked_add(1)
                || receipt.record_digest != intent.record_digest
                || !receipt.record_digest.as_deref().is_some_and(digest)
                || receipt.deleted.is_some()
                || receipt.affected_resources != 1
            {
                return Err(invalid());
            }
        }
        OwnerDefinitionOperation::Enable | OwnerDefinitionOperation::Disable => {
            if Some(after) != before.checked_add(1)
                || !receipt.record_digest.as_deref().is_some_and(digest)
                || receipt.enabled != Some(intent.operation == OwnerDefinitionOperation::Enable)
                || receipt.deleted.is_some()
                || receipt.affected_resources != 1
            {
                return Err(invalid());
            }
        }
        OwnerDefinitionOperation::Delete => {
            let deleted = receipt.deleted.ok_or_else(invalid)?;
            if (if deleted {
                before.checked_add(1)
            } else {
                Some(before)
            }) != Some(after)
                || receipt.record_digest.is_some()
                || receipt.enabled.is_some()
                || ((!deleted || intent.kind == AutomationKind::Event)
                    && receipt.affected_resources != 1)
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

fn validate(stored: &Stored, host: &str, pending: bool) -> Result<(), DatabaseError> {
    let i = &stored.intent;
    let request = CommandRequest {
        command_id: i.command_id.clone(),
        host_instance_id: i.host_instance_id.clone(),
        expected_revision: i.expected_revision.clone(),
    };
    if stored.version != 1
        || stored.host_id != host
        || !id(host)
        || !id(&i.id)
        || request.validate().is_err()
        || revision(&i.expected_revision).is_none()
        || !i
            .command_id
            .strip_prefix("owner-definition-")
            .is_some_and(|suffix| {
                suffix.len() == 32
                    && suffix
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        || !(0..=MAX_SAFE_TIMESTAMP).contains(&i.created_at)
        || !digest(&stored.payload_hash)
        || (if save(i.operation) {
            !i.record_digest.as_deref().is_some_and(digest)
        } else {
            i.record_digest.is_some()
        })
        || pending != stored.record.is_none()
    {
        return Err(invalid());
    }
    // Save source is deliberately absent from this journal. Its full business
    // hash is reconstructed only from the immutable original ledger result.
    if let Some(operation) = metadata_operation(i) {
        if commands::payload_hash(
            operation.method(),
            &operation.params(),
            &i.expected_revision,
        )? != stored.payload_hash
        {
            return Err(invalid());
        }
    }
    if let Some(record) = &stored.record {
        if record.intent != *i || !(0..=MAX_SAFE_TIMESTAMP).contains(&record.recorded_at) {
            return Err(invalid());
        }
        match record.outcome {
            OwnerDefinitionOutcome::Succeeded => {
                if record.error_code.is_some() {
                    return Err(invalid());
                }
                validate_receipt(i, record.receipt.as_ref().ok_or_else(invalid)?)?;
            }
            OwnerDefinitionOutcome::Rejected => {
                if record.receipt.is_some()
                    || !record
                        .error_code
                        .as_deref()
                        .is_some_and(definition_rejection)
                {
                    return Err(invalid());
                }
            }
            OwnerDefinitionOutcome::ReviewedUnknown => {
                if record.receipt.is_some() || record.error_code.is_some() {
                    return Err(invalid());
                }
            }
        }
    }
    Ok(())
}

fn rows(db: &Connection, host: &str) -> Result<Vec<Stored>, DatabaseError> {
    let mut statement = db.prepare("SELECT command_id,pending,CASE WHEN length(CAST(payload_json AS BLOB))<=8192 THEN payload_json ELSE NULL END FROM owner_automation_definitions ORDER BY rowid DESC LIMIT 18")?;
    let mut query = statement.query([])?;
    let mut entries = Vec::new();
    while let Some(row) = query.next()? {
        if entries.len() > HISTORY {
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
        let encoded: Option<String> = row.get(2)?;
        let stored: Stored = serde_json::from_str(&encoded.ok_or_else(invalid)?)?;
        validate(&stored, host, flag == 1)?;
        if stored.intent.command_id != command {
            return Err(invalid());
        }
        entries.push(stored);
    }
    if entries
        .iter()
        .filter(|entry| entry.record.is_none())
        .count()
        > 1
    {
        return Err(invalid());
    }
    Ok(entries)
}
fn pending(db: &Connection, host: &str) -> Result<Option<Stored>, DatabaseError> {
    Ok(rows(db, host)?
        .into_iter()
        .find(|entry| entry.record.is_none()))
}
pub(super) fn has_pending(db: &Connection, host: &str) -> Result<bool, DatabaseError> {
    Ok(pending(db, host)?.is_some())
}
fn encoded(stored: &Stored) -> Result<String, DatabaseError> {
    let value = serde_json::to_string(stored)?;
    if value.len() > ENTRY_BYTES {
        return Err(invalid());
    }
    Ok(value)
}
fn complete(
    db: &Connection,
    mut stored: Stored,
    outcome: OwnerDefinitionOutcome,
    receipt: Option<OwnerDefinitionReceipt>,
    error_code: Option<String>,
) -> Result<(), DatabaseError> {
    let at = definition_now();
    stored.record = Some(OwnerDefinitionRecord {
        intent: stored.intent.clone(),
        outcome,
        receipt,
        error_code,
        recorded_at: at,
    });
    validate(&stored, &stored.host_id, false)?;
    if db.execute("UPDATE owner_automation_definitions SET pending=0,payload_json=?2,updated_at=?3 WHERE command_id=?1 AND pending=1",params![stored.intent.command_id,encoded(&stored)?,at])? != 1 {
        return Err(invalid());
    }
    db.execute("DELETE FROM owner_automation_definitions WHERE pending=0 AND command_id NOT IN (SELECT command_id FROM owner_automation_definitions WHERE pending=0 ORDER BY rowid DESC LIMIT 16)",[])?;
    Ok(())
}

fn receipt(
    stored: &Stored,
    outcome: &CommandOutcome,
) -> Result<OwnerDefinitionReceipt, DatabaseError> {
    let i = &stored.intent;
    let resource = outcome.command.resource.as_ref().ok_or_else(invalid)?;
    let primary = outcome.resource_versions.first().ok_or_else(invalid)?;
    let body = outcome.result.as_object().ok_or_else(invalid)?;
    bounded_record(&outcome.result)?;
    if outcome.principal_id != "local-owner"
        || outcome.method != method(i.operation)
        || outcome.host_instance_id != i.host_instance_id
        || outcome.command.command_id != i.command_id
        || outcome.command.state != CommandState::Succeeded
        || !outcome
            .command
            .committed_at
            .is_some_and(|at| (1..=MAX_SAFE_TIMESTAMP).contains(&at))
        || resource.kind != ResourceKind::Automation
        || resource.scope != scope(i.kind)
        || resource.id != i.id
        || primary.kind != resource.kind
        || primary.scope != resource.scope
        || primary.id != resource.id
        || body.get("id").and_then(Value::as_str) != Some(i.id.as_str())
    {
        return Err(invalid());
    }
    let mut last_event: Option<&str> = None;
    for version in outcome.resource_versions.iter().skip(1) {
        if i.operation != OwnerDefinitionOperation::Delete
            || i.kind != AutomationKind::Action
            || body.get("deleted") != Some(&Value::Bool(true))
            || version.kind != ResourceKind::Automation
            || version.scope != "event"
            || !id(&version.id)
            || !revision(&version.revision).is_some_and(|value| value > 0)
            || last_event.is_some_and(|last| last >= version.id.as_str())
        {
            return Err(invalid());
        }
        last_event = Some(&version.id);
    }
    let operation = match i.operation {
        OwnerDefinitionOperation::Create => AutomationMutation::Create {
            kind: i.kind,
            record: outcome.result.clone(),
        },
        OwnerDefinitionOperation::Update => AutomationMutation::Update {
            kind: i.kind,
            id: i.id.clone(),
            record: outcome.result.clone(),
        },
        _ => metadata_operation(i).ok_or_else(invalid)?,
    };
    validate_owner_definition_input(&operation, &i.expected_revision)?;
    if commands::payload_hash(
        operation.method(),
        &operation.params(),
        &i.expected_revision,
    )? != stored.payload_hash
    {
        return Err(invalid());
    }
    let result = OwnerDefinitionReceipt {
        revision: primary.revision.clone(),
        record_digest: if i.operation == OwnerDefinitionOperation::Delete {
            None
        } else {
            Some(definition_record_digest(&outcome.result))
        },
        enabled: if i.operation == OwnerDefinitionOperation::Delete {
            None
        } else {
            body.get("enabled").and_then(Value::as_bool)
        },
        deleted: if i.operation == OwnerDefinitionOperation::Delete {
            Some(
                body.get("deleted")
                    .and_then(Value::as_bool)
                    .ok_or_else(invalid)?,
            )
        } else {
            None
        },
        affected_resources: u32::try_from(outcome.resource_versions.len())
            .map_err(|_| invalid())?,
    };
    if i.operation == OwnerDefinitionOperation::Delete && body.len() != 2 {
        return Err(invalid());
    }
    validate_receipt(i, &result)?;
    Ok(result)
}

fn bound_ledger(db: &Connection, stored: &Stored) -> Result<bool, DatabaseError> {
    let mut statement = db.prepare("SELECT payload_hash,session_id,method,resource_kind,resource_scope,resource_id,host_instance_id,state,length(CAST(result_json AS BLOB)),length(CAST(versions_json AS BLOB)),outcome_expired,committed_at FROM control_commands WHERE principal_id='local-owner' AND command_id=?1")?;
    let mut rows = statement.query([&stored.intent.command_id])?;
    let Some(row) = rows.next()? else {
        return Ok(false);
    };
    if !text_columns_within(row, &CONTROL_COMMAND_TEXT_LIMITS)? {
        return Err(invalid());
    }
    for (column, max) in [(8, RECORD_BYTES as i64), (9, VERSIONS_BYTES as i64)] {
        if row
            .get::<_, Option<i64>>(column)?
            .is_some_and(|size| size < 0 || size > max)
        {
            return Err(invalid());
        }
    }
    let i = &stored.intent;
    if row.get::<_, String>(0)? != stored.payload_hash
        || row.get::<_, String>(1)? != "local-owner"
        || row.get::<_, String>(2)? != method(i.operation)
        || row.get::<_, String>(3)? != "automation"
        || row.get::<_, String>(4)? != scope(i.kind)
        || row.get::<_, String>(5)? != i.id
        || row.get::<_, String>(6)? != i.host_instance_id
        || row.get::<_, String>(7)? != "succeeded"
        || !matches!(row.get::<_, i64>(10)?, 0 | 1)
        || !(0..=MAX_SAFE_TIMESTAMP).contains(&row.get::<_, i64>(11)?)
    {
        return Err(invalid());
    }
    Ok(true)
}

impl DatabaseManager {
    /// The caller holds shared coordination and the automation writer through
    /// this read and the following business command. Reject oversized native
    /// acknowledgements before any business mutation can commit.
    pub(crate) fn owner_definition_validate_dispatch(
        &self,
        operation: &AutomationMutation,
        expected: &str,
    ) -> Result<(), DatabaseError> {
        validate_owner_definition_input(operation, expected)?;
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        super::behavior::owner_definition_dispatch_preflight(
            &tx,
            operation,
            expected,
            VERSIONS_BYTES - RECORD_BYTES,
        )?;
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn owner_definition_state(
        &self,
        host: &str,
        instance: &str,
    ) -> Result<OwnerDefinitionState, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let mut state = OwnerDefinitionState {
            host_id: host.into(),
            host_instance_id: instance.into(),
            pending: None,
            recent: Vec::new(),
        };
        for stored in rows(&tx, host)? {
            if let Some(record) = stored.record {
                state.recent.push(record);
            } else {
                state.pending = Some(stored.intent);
            }
        }
        tx.commit()?;
        Ok(state)
    }
    pub(crate) fn owner_definition_begin(
        &self,
        host: &str,
        intent: &OwnerDefinitionIntent,
        operation: &AutomationMutation,
    ) -> Result<(), DatabaseError> {
        validate_owner_definition_input(operation, &intent.expected_revision)?;
        let (kind, target) = operation.target();
        let record_digest = match operation {
            AutomationMutation::Create { record, .. }
            | AutomationMutation::Update { record, .. } => Some(definition_record_digest(record)),
            _ => None,
        };
        if kind != intent.kind
            || target != intent.id
            || operation_kind(operation)? != intent.operation
            || record_digest != intent.record_digest
        {
            return Err(invalid());
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if pending(&tx, host)?.is_some()
            || super::owner_adoption::has_pending(&tx, host)?
            || super::owner_approval::has_pending(&tx, host)?
            || super::owner_library::has_pending(&tx, host)?
        {
            return Err(DatabaseError::OwnerAdoptionPending);
        }
        let stored = Stored {
            version: 1,
            host_id: host.into(),
            payload_hash: commands::payload_hash(
                operation.method(),
                &operation.params(),
                &intent.expected_revision,
            )?,
            intent: intent.clone(),
            record: None,
        };
        validate(&stored, host, true)?;
        tx.execute("INSERT INTO owner_automation_definitions(command_id,pending,payload_json,updated_at) VALUES (?1,1,?2,?3)",params![intent.command_id,encoded(&stored)?,intent.created_at])?;
        tx.commit()?;
        Ok(())
    }
    /// A known immutable ledger outcome wins over rejection/unknown assertions.
    pub(crate) fn owner_definition_resolve(
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
                        OwnerDefinitionOutcome::Succeeded,
                        Some(receipt),
                        None,
                    )?;
                }
                Err(DatabaseError::CommandExpired) if review_unknown => complete(
                    &tx,
                    stored,
                    OwnerDefinitionOutcome::ReviewedUnknown,
                    None,
                    None,
                )?,
                Err(DatabaseError::CommandExpired) => {}
                Err(error) => return Err(error),
                Ok(None) => return Err(invalid()),
            }
        } else if rejection.is_some_and(definition_rejection) {
            complete(
                &tx,
                stored,
                OwnerDefinitionOutcome::Rejected,
                None,
                rejection.map(str::to_owned),
            )?;
        } else if review_unknown {
            complete(
                &tx,
                stored,
                OwnerDefinitionOutcome::ReviewedUnknown,
                None,
                None,
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
