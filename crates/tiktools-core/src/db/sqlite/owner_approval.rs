//! Bounded metadata-only native approval journal. Complete definitions remain
//! solely in the business database; recovery never dispatches an operation.
use super::owner_journal_values::{
    digest, id, revision, scope, text_columns_within, CONTROL_COMMAND_TEXT_LIMITS,
};
use super::{commands, DatabaseError, DatabaseManager};
use crate::{
    collaboration::{CommandRequest, ResourceKind, ResourceVersion},
    control::{
        owner_approval::{approval_now, approval_rejection, ApprovalCommand},
        ApprovalActivateParams, ApprovalEnablement, ApprovalRevokeParams, AutomationKind,
        OwnerApprovalIntent, OwnerApprovalOperation, OwnerApprovalOutcome, OwnerApprovalReceipt,
        OwnerApprovalRecord, OwnerApprovalState,
    },
};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const HISTORY: usize = 16;
const ENTRY_BYTES: usize = 256 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActivationProof {
    reviewed_versions: Vec<ResourceVersion>,
    manifest_digest: String,
    enable_plan: Vec<ApprovalEnablement>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    version: u8,
    host_id: String,
    payload_hash: String,
    intent: OwnerApprovalIntent,
    activation_proof: Option<ActivationProof>,
    record: Option<OwnerApprovalRecord>,
}
fn invalid() -> DatabaseError {
    DatabaseError::Invalid("owner approval journal is invalid".into())
}

pub(crate) fn validate_owner_approval_input(
    operation: &ApprovalCommand,
    expected: &str,
) -> Result<(), DatabaseError> {
    let (kind, target) = operation.target();
    if !id(target) || !revision(expected).is_some_and(|n| n > 0) {
        return Err(invalid());
    }
    if let ApprovalCommand::Activate(p) = operation {
        if !p.acknowledge_effects
            || !digest(&p.manifest_digest)
            || p.reviewed_versions.is_empty()
            || p.reviewed_versions.len() > 513
            || p.enable_plan.is_empty()
            || p.enable_plan.len() > 513
        {
            return Err(invalid());
        }
        let mut seen = BTreeSet::new();
        for (position, v) in p.reviewed_versions.iter().enumerate() {
            if v.kind != ResourceKind::Automation
                || !id(&v.id)
                || revision(&v.revision).is_none()
                || !seen.insert((v.scope.as_str(), v.id.as_str()))
                || if position == 0 {
                    v.scope != scope(kind) || v.id != target || v.revision != expected
                } else {
                    kind != AutomationKind::Event || v.scope != "action"
                }
            {
                return Err(invalid());
            }
        }
        let mut planned = BTreeSet::new();
        let mut primary = false;
        for e in &p.enable_plan {
            let family = scope(e.kind);
            if !id(&e.id)
                || !seen.contains(&(family, e.id.as_str()))
                || !planned.insert((family, e.id.as_str()))
            {
                return Err(invalid());
            }
            if family == scope(kind) && e.id == target {
                if !e.enabled {
                    return Err(invalid());
                }
                primary = true;
            }
        }
        if !primary {
            return Err(invalid());
        }
    } else if let ApprovalCommand::Revoke(p) = operation {
        if !p.acknowledge_revocation {
            return Err(invalid());
        }
    }
    Ok(())
}
fn operation(stored: &Stored) -> Result<ApprovalCommand, DatabaseError> {
    let i = &stored.intent;
    match (i.operation, stored.activation_proof.as_ref()) {
        (OwnerApprovalOperation::Activate, Some(p)) => {
            Ok(ApprovalCommand::Activate(ApprovalActivateParams {
                kind: i.kind,
                id: i.id.clone(),
                reviewed_versions: p.reviewed_versions.clone(),
                manifest_digest: p.manifest_digest.clone(),
                enable_plan: p.enable_plan.clone(),
                acknowledge_effects: true,
            }))
        }
        (OwnerApprovalOperation::Revoke, None) => {
            Ok(ApprovalCommand::Revoke(ApprovalRevokeParams {
                kind: i.kind,
                id: i.id.clone(),
                acknowledge_revocation: true,
            }))
        }
        _ => Err(invalid()),
    }
}
fn validate_receipt(
    intent: &OwnerApprovalIntent,
    receipt: &OwnerApprovalReceipt,
) -> Result<(), DatabaseError> {
    if revision(&receipt.revision)
        != revision(&intent.expected_revision).and_then(|n| n.checked_add(1))
        || !revision(&receipt.approval_generation).is_some_and(|n| n > 0)
    {
        return Err(invalid());
    }
    match intent.operation {
        OwnerApprovalOperation::Activate
            if receipt.policy_version == Some(1)
                && receipt.manifest_digest.as_deref().is_some_and(digest) =>
        {
            Ok(())
        }
        OwnerApprovalOperation::Revoke
            if receipt.policy_version.is_none() && receipt.manifest_digest.is_none() =>
        {
            Ok(())
        }
        _ => Err(invalid()),
    }
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
        || !i
            .command_id
            .strip_prefix("owner-approval-")
            .is_some_and(|s| {
                s.len() == 32
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        || !(0..=9_007_199_254_740_991).contains(&i.created_at)
        || !digest(&stored.payload_hash)
        || pending != stored.record.is_none()
    {
        return Err(invalid());
    }
    let op = operation(stored)?;
    validate_owner_approval_input(&op, &i.expected_revision)?;
    if commands::payload_hash(op.method(), &op.params(), &i.expected_revision)?
        != stored.payload_hash
    {
        return Err(invalid());
    }
    if let Some(r) = &stored.record {
        if r.intent != *i || !(0..=9_007_199_254_740_991).contains(&r.recorded_at) {
            return Err(invalid());
        }
        match r.outcome {
            OwnerApprovalOutcome::Succeeded => {
                if r.error_code.is_some() {
                    return Err(invalid());
                }
                validate_receipt(i, r.receipt.as_ref().ok_or_else(invalid)?)?;
            }
            OwnerApprovalOutcome::Rejected => {
                if r.receipt.is_some() || !r.error_code.as_deref().is_some_and(approval_rejection) {
                    return Err(invalid());
                }
            }
            OwnerApprovalOutcome::ReviewedUnknown => {
                if r.receipt.is_some() || r.error_code.is_some() {
                    return Err(invalid());
                }
            }
        }
    }
    Ok(())
}
fn rows(db: &Connection, host: &str) -> Result<Vec<Stored>, DatabaseError> {
    let mut q=db.prepare("SELECT command_id,pending,CASE WHEN length(CAST(payload_json AS BLOB))<=262144 THEN payload_json ELSE NULL END FROM owner_automation_approvals ORDER BY rowid DESC LIMIT 18")?;
    let mut rows = q.query([])?;
    let mut result = Vec::new();
    while let Some(r) = rows.next()? {
        if result.len() > HISTORY {
            return Err(invalid());
        }
        match r.get_ref(0)? {
            rusqlite::types::ValueRef::Text(bytes) if bytes.len() <= 128 => {}
            _ => return Err(invalid()),
        }
        let command: String = r.get(0)?;
        let flag: i64 = r.get(1)?;
        if !matches!(flag, 0 | 1) {
            return Err(invalid());
        }
        let encoded: Option<String> = r.get(2)?;
        let stored: Stored = serde_json::from_str(&encoded.ok_or_else(invalid)?)?;
        validate(&stored, host, flag == 1)?;
        if stored.intent.command_id != command {
            return Err(invalid());
        }
        result.push(stored);
    }
    if result.iter().filter(|s| s.record.is_none()).count() > 1 {
        return Err(invalid());
    }
    Ok(result)
}
fn pending(db: &Connection, host: &str) -> Result<Option<Stored>, DatabaseError> {
    Ok(rows(db, host)?.into_iter().find(|s| s.record.is_none()))
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
    outcome: OwnerApprovalOutcome,
    receipt: Option<OwnerApprovalReceipt>,
    error_code: Option<String>,
) -> Result<(), DatabaseError> {
    let at = approval_now();
    stored.record = Some(OwnerApprovalRecord {
        intent: stored.intent.clone(),
        outcome,
        receipt,
        error_code,
        recorded_at: at,
    });
    validate(&stored, &stored.host_id, false)?;
    if db.execute("UPDATE owner_automation_approvals SET pending=0,payload_json=?2,updated_at=?3 WHERE command_id=?1 AND pending=1",params![stored.intent.command_id,encoded(&stored)?,at])?!=1{return Err(invalid());}
    db.execute("DELETE FROM owner_automation_approvals WHERE pending=0 AND command_id NOT IN (SELECT command_id FROM owner_automation_approvals WHERE pending=0 ORDER BY rowid DESC LIMIT 16)",[])?;
    Ok(())
}
fn receipt(
    stored: &Stored,
    outcome: &crate::collaboration::CommandOutcome,
) -> Result<OwnerApprovalReceipt, DatabaseError> {
    let i = &stored.intent;
    let resource = outcome.command.resource.as_ref().ok_or_else(invalid)?;
    let primary = outcome.resource_versions.first().ok_or_else(invalid)?;
    let body = outcome.result.as_object().ok_or_else(invalid)?;
    if outcome.method != operation(stored)?.method()
        || outcome.host_instance_id != i.host_instance_id
        || outcome.command.command_id != i.command_id
        || outcome.command.state != crate::collaboration::CommandState::Succeeded
        || !outcome
            .command
            .committed_at
            .is_some_and(|at| (1..=9_007_199_254_740_991).contains(&at))
        || resource.kind != ResourceKind::Automation
        || resource.scope != scope(i.kind)
        || resource.id != i.id
        || primary.kind != resource.kind
        || primary.scope != resource.scope
        || primary.id != resource.id
        || body.get("kind").and_then(serde_json::Value::as_str) != Some(scope(i.kind))
        || body.get("id").and_then(serde_json::Value::as_str) != Some(i.id.as_str())
    {
        return Err(invalid());
    }
    let generation = body
        .get("approvalGeneration")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(invalid)?;
    let result = match i.operation {
        OwnerApprovalOperation::Activate => {
            let proof = stored.activation_proof.as_ref().ok_or_else(invalid)?;
            if body.len() != 7
                || body.get("enabled") != Some(&serde_json::Value::Bool(true))
                || body.get("approved") != Some(&serde_json::Value::Bool(true))
                || outcome.resource_versions.len() != proof.reviewed_versions.len()
            {
                return Err(invalid());
            }
            let plan: BTreeSet<_> = proof
                .enable_plan
                .iter()
                .map(|p| (scope(p.kind), p.id.as_str()))
                .collect();
            for (pos, (final_version, before)) in outcome
                .resource_versions
                .iter()
                .zip(&proof.reviewed_versions)
                .enumerate()
            {
                let previous = revision(&before.revision).ok_or_else(invalid)?;
                let after = revision(&final_version.revision).ok_or_else(invalid)?;
                if final_version.kind != ResourceKind::Automation
                    || final_version.scope != before.scope
                    || final_version.id != before.id
                    || if pos == 0 {
                        Some(after) != previous.checked_add(1)
                    } else if plan.contains(&(before.scope.as_str(), before.id.as_str())) {
                        after != previous && Some(after) != previous.checked_add(1)
                    } else {
                        after != previous
                    }
                {
                    return Err(invalid());
                }
            }
            OwnerApprovalReceipt {
                revision: primary.revision.clone(),
                approval_generation: generation.into(),
                policy_version: body
                    .get("policyVersion")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|n| u32::try_from(n).ok()),
                manifest_digest: body
                    .get("manifestDigest")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            }
        }
        OwnerApprovalOperation::Revoke => {
            if body.len() != 4
                || body.get("revoked") != Some(&serde_json::Value::Bool(true))
                || outcome.resource_versions.len() != 1
            {
                return Err(invalid());
            }
            OwnerApprovalReceipt {
                revision: primary.revision.clone(),
                approval_generation: generation.into(),
                policy_version: None,
                manifest_digest: None,
            }
        }
    };
    validate_receipt(i, &result)?;
    Ok(result)
}
fn bound_ledger(
    db: &Connection,
    command_id: &str,
) -> Result<Option<(String, String)>, DatabaseError> {
    let mut q=db.prepare("SELECT payload_hash,session_id,method,resource_kind,resource_scope,resource_id,host_instance_id,state,length(CAST(result_json AS BLOB)),length(CAST(versions_json AS BLOB)) FROM control_commands WHERE principal_id='local-owner' AND command_id=?1")?;
    let mut rows = q.query([command_id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    if !text_columns_within(row, &CONTROL_COMMAND_TEXT_LIMITS)? {
        return Err(invalid());
    }
    for (column, max) in [(8, 4096), (9, ENTRY_BYTES as i64)] {
        if row
            .get::<_, Option<i64>>(column)?
            .is_some_and(|size| size < 0 || size > max)
        {
            return Err(invalid());
        }
    }
    Ok(Some((row.get(0)?, row.get(1)?)))
}
impl DatabaseManager {
    pub(crate) fn owner_approval_state(
        &self,
        host: &str,
        instance: &str,
    ) -> Result<OwnerApprovalState, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let mut state = OwnerApprovalState {
            host_id: host.into(),
            host_instance_id: instance.into(),
            pending: None,
            recent: Vec::new(),
        };
        for s in rows(&tx, host)? {
            if let Some(r) = s.record {
                state.recent.push(r);
            } else {
                state.pending = Some(s.intent);
            }
        }
        tx.commit()?;
        Ok(state)
    }
    pub(crate) fn owner_approval_begin(
        &self,
        host: &str,
        intent: &OwnerApprovalIntent,
        operation: &ApprovalCommand,
    ) -> Result<(), DatabaseError> {
        validate_owner_approval_input(operation, &intent.expected_revision)?;
        let (kind, target) = operation.target();
        if kind != intent.kind || target != intent.id || intent.operation != operation.operation() {
            return Err(invalid());
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if pending(&tx, host)?.is_some()
            || super::owner_adoption::has_pending(&tx, host)?
            || super::owner_definition::has_pending(&tx, host)?
            || super::owner_library::has_pending(&tx, host)?
        {
            return Err(DatabaseError::OwnerAdoptionPending);
        }
        let proof = match operation {
            ApprovalCommand::Activate(p) => Some(ActivationProof {
                reviewed_versions: p.reviewed_versions.clone(),
                manifest_digest: p.manifest_digest.clone(),
                enable_plan: p.enable_plan.clone(),
            }),
            ApprovalCommand::Revoke(_) => None,
        };
        let stored = Stored {
            version: 1,
            host_id: host.into(),
            payload_hash: commands::payload_hash(
                operation.method(),
                &operation.params(),
                &intent.expected_revision,
            )?,
            intent: intent.clone(),
            activation_proof: proof,
            record: None,
        };
        validate(&stored, host, true)?;
        tx.execute("INSERT INTO owner_automation_approvals(command_id,pending,payload_json,updated_at) VALUES (?1,1,?2,?3)",params![intent.command_id,encoded(&stored)?,intent.created_at])?;
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn owner_approval_resolve(
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
        match bound_ledger(&tx, &intent.command_id)? {
            Some((hash, session)) => {
                if hash != stored.payload_hash || session != "local-owner" {
                    return Err(invalid());
                }
                match commands::lookup(&tx, "local-owner", &intent.command_id) {
                    Ok(Some(outcome)) => {
                        let receipt = receipt(&stored, &outcome)?;
                        complete(
                            &tx,
                            stored,
                            OwnerApprovalOutcome::Succeeded,
                            Some(receipt),
                            None,
                        )?;
                    }
                    Err(DatabaseError::CommandExpired) if review_unknown => complete(
                        &tx,
                        stored,
                        OwnerApprovalOutcome::ReviewedUnknown,
                        None,
                        None,
                    )?,
                    Err(DatabaseError::CommandExpired) => {}
                    Err(error) => return Err(error),
                    Ok(None) => return Err(invalid()),
                }
            }
            None if rejection.is_some_and(approval_rejection) => complete(
                &tx,
                stored,
                OwnerApprovalOutcome::Rejected,
                None,
                rejection.map(str::to_owned),
            )?,
            None if review_unknown => complete(
                &tx,
                stored,
                OwnerApprovalOutcome::ReviewedUnknown,
                None,
                None,
            )?,
            None => {}
        }
        tx.commit()?;
        Ok(())
    }
}
