//! A durable, bounded at-most-once handoff ledger. No method in this module
//! dispatches a sink or turns retained history into a new execution permit.
use super::{util::now, DatabaseError, DatabaseManager};
use crate::{
    control::{EffectHistory, EffectKind, EffectReceipt, EffectState},
    services::effects::{valid_digest, EffectKey, EffectOutcome},
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use std::time::Duration;

const OUTCOME_TTL: i64 = 7 * 24 * 60 * 60 * 1000;
const MAX_EFFECT_IDENTITIES: i64 = 1_000_000;
// Drop may record cancellation on a Tokio worker. Bound the wait rather than
// block that worker indefinitely; restart recovery still resolves uncertainty.
const WRITER_WAIT: Duration = Duration::from_millis(100);

struct StoredEffect {
    receipt: EffectReceipt,
    binding_digest: String,
    request_digest: String,
    instance: String,
}

fn require_identity(value: &str) -> Result<(), DatabaseError> {
    if valid_digest(value) {
        Ok(())
    } else {
        Err(DatabaseError::Invalid(
            "invalid external effect identity".into(),
        ))
    }
}

fn current_host(db: &Connection, instance: &str) -> Result<(), DatabaseError> {
    let current = db
        .query_row(
            "SELECT instance_id FROM automation_external_effect_host WHERE singleton=1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if current.as_deref() == Some(instance) {
        Ok(())
    } else {
        Err(DatabaseError::EffectStateChanged)
    }
}

fn read_effect(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredEffect> {
    let kind = match row.get::<_, String>(1)?.as_str() {
        "http" => EffectKind::Http,
        "audio" => EffectKind::Audio,
        "plugin" => EffectKind::Plugin,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    let state = match row.get::<_, String>(5)?.as_str() {
        "accepted" => EffectState::Accepted,
        "dispatched" => EffectState::Dispatched,
        "succeeded" => EffectState::Succeeded,
        "failed" => EffectState::Failed,
        "unknown" => EffectState::Unknown,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(StoredEffect {
        receipt: EffectReceipt {
            effect_id: row.get(0)?,
            kind,
            state,
            accepted_at: row.get(6)?,
            updated_at: row.get(7)?,
            outcome_expired: row.get(8)?,
        },
        binding_digest: row.get(2)?,
        request_digest: row.get(3)?,
        instance: row.get(4)?,
    })
}

const COLUMNS: &str = "effect_id,kind,binding_digest,request_digest,instance_id,state,accepted_at,updated_at,outcome_expired";

fn lookup(db: &Connection, id: &str) -> Result<Option<StoredEffect>, DatabaseError> {
    let stored = db
        .query_row(
            &format!("SELECT {COLUMNS} FROM automation_external_effects WHERE effect_id=?1"),
            [id],
            read_effect,
        )
        .optional()?;
    if let Some(value) = &stored {
        // Malformed retained history is never interpreted as an absent key.
        for digest in [
            value.receipt.effect_id.as_str(),
            &value.binding_digest,
            &value.request_digest,
            &value.instance,
        ] {
            require_identity(digest)?;
        }
        if value.receipt.accepted_at < 0 || value.receipt.updated_at < value.receipt.accepted_at {
            return Err(DatabaseError::EffectStateChanged);
        }
    }
    Ok(stored)
}

fn check_semantics(stored: &StoredEffect, key: &EffectKey) -> Result<(), DatabaseError> {
    if stored.receipt.kind == key.kind()
        && stored.binding_digest == key.binding_digest()
        && stored.request_digest == key.request_digest()
    {
        Ok(())
    } else {
        Err(DatabaseError::EffectIdentityReused)
    }
}

fn present(mut receipt: EffectReceipt) -> EffectReceipt {
    let terminal = matches!(
        receipt.state,
        EffectState::Succeeded | EffectState::Failed | EffectState::Unknown
    );
    if receipt.outcome_expired
        || (terminal && receipt.updated_at < now().saturating_sub(OUTCOME_TTL))
    {
        receipt.outcome_expired = true;
        receipt.state = EffectState::Unknown;
    }
    receipt
}

fn expire(db: &Connection) -> Result<(), DatabaseError> {
    db.execute(
        "UPDATE automation_external_effects SET outcome_expired=1 WHERE outcome_expired=0 AND updated_at<?1 AND state IN ('succeeded','failed','unknown')",
        [now().saturating_sub(OUTCOME_TTL)],
    )?;
    Ok(())
}

/// The operation owns the transaction. It exposes only a one-shot commit
/// closure so the caller's live authority locks surround the actual commit,
/// not merely an inner callback whose transaction would commit afterwards.
fn commit_under_admission(
    transaction: Transaction<'_>,
    mutate: impl FnOnce(&Transaction<'_>) -> Result<(), DatabaseError>,
    admit: impl FnOnce(&mut dyn FnMut() -> Result<(), String>) -> Result<(), String>,
) -> Result<(), String> {
    let mut pending = Some((transaction, mutate));
    let mut committed = false;
    admit(&mut || {
        let (transaction, mutate) = pending
            .take()
            .ok_or("external effect commit was already consumed")?;
        mutate(&transaction).map_err(|error| error.to_string())?;
        crash_at("before_commit");
        transaction.commit().map_err(|error| error.to_string())?;
        committed = true;
        crash_at("after_commit");
        Ok(())
    })?;
    if !committed {
        return Err("external effect admission did not commit".into());
    }
    Ok(())
}

#[cfg(test)]
fn crash_at(stage: &str) {
    if std::env::var("TIKTOOLS_EFFECT_CRASH_STAGE").as_deref() == Ok(stage) {
        // Deliberately bypass destructors and rusqlite's rollback/drop logic.
        std::process::exit(if stage == "before_commit" { 86 } else { 87 });
    }
}
#[cfg(not(test))]
fn crash_at(_: &str) {}

impl DatabaseManager {
    pub(crate) fn recover_external_effects(&self, instance: &str) -> Result<(), DatabaseError> {
        require_identity(instance)?;
        let mut db = self.open(&self.automation_path())?;
        db.busy_timeout(WRITER_WAIT)?;
        let transaction = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO automation_external_effect_host(singleton,instance_id) VALUES(1,?1) ON CONFLICT(singleton) DO UPDATE SET instance_id=excluded.instance_id",
            [instance],
        )?;
        transaction.execute(
            "UPDATE automation_external_effects SET state='unknown',updated_at=MAX(updated_at,?1) WHERE instance_id<>?2 AND state IN ('accepted','dispatched')",
            params![now(), instance],
        )?;
        expire(&transaction)?;
        transaction.commit()?;
        Ok(())
    }

    /// `None` means a newly committed acceptance, the only path allowed to
    /// construct a fresh ticket. Every retained state returns a receipt only.
    pub(crate) fn reserve_external_effect(
        &self,
        key: &EffectKey,
        instance: &str,
        admit: impl FnOnce(&mut dyn FnMut() -> Result<(), String>) -> Result<(), String>,
    ) -> Result<Option<EffectReceipt>, String> {
        self.reserve_external_effect_bounded(key, instance, MAX_EFFECT_IDENTITIES, admit)
    }

    #[cfg(test)]
    pub(crate) fn reserve_external_effect_test_limit(
        &self,
        key: &EffectKey,
        instance: &str,
        max_identities: i64,
        admit: impl FnOnce(&mut dyn FnMut() -> Result<(), String>) -> Result<(), String>,
    ) -> Result<Option<EffectReceipt>, String> {
        self.reserve_external_effect_bounded(key, instance, max_identities, admit)
    }

    fn reserve_external_effect_bounded(
        &self,
        key: &EffectKey,
        instance: &str,
        max_identities: i64,
        admit: impl FnOnce(&mut dyn FnMut() -> Result<(), String>) -> Result<(), String>,
    ) -> Result<Option<EffectReceipt>, String> {
        let mut db = self
            .open(&self.automation_path())
            .map_err(|error| error.to_string())?;
        db.busy_timeout(WRITER_WAIT)
            .map_err(|error| error.to_string())?;
        let transaction = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        current_host(&transaction, instance).map_err(|error| error.to_string())?;
        if let Some(stored) = lookup(&transaction, key.id()).map_err(|error| error.to_string())? {
            check_semantics(&stored, key).map_err(|error| error.to_string())?;
            return Ok(Some(present(stored.receipt)));
        }
        let count: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM automation_external_effects",
                [],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if count >= max_identities {
            return Err(DatabaseError::EffectCapacity.to_string());
        }
        commit_under_admission(
            transaction,
            |transaction| {
                let timestamp = now();
                transaction.execute(
                    "INSERT INTO automation_external_effects(effect_id,kind,binding_digest,request_digest,instance_id,state,accepted_at,updated_at,outcome_expired) VALUES(?1,?2,?3,?4,?5,'accepted',?6,?6,0)",
                    params![key.id(), key.kind().as_str(), key.binding_digest(), key.request_digest(), instance, timestamp],
                )?;
                Ok(())
            },
            admit,
        )?;
        Ok(None)
    }

    pub(crate) fn dispatch_external_effect(
        &self,
        key: &EffectKey,
        instance: &str,
        admit: impl FnOnce(&mut dyn FnMut() -> Result<(), String>) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut db = self
            .open(&self.automation_path())
            .map_err(|error| error.to_string())?;
        db.busy_timeout(WRITER_WAIT)
            .map_err(|error| error.to_string())?;
        let transaction = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        current_host(&transaction, instance).map_err(|error| error.to_string())?;
        let stored = lookup(&transaction, key.id())
            .map_err(|error| error.to_string())?
            .ok_or_else(|| DatabaseError::EffectStateChanged.to_string())?;
        check_semantics(&stored, key).map_err(|error| error.to_string())?;
        if stored.instance != instance || stored.receipt.state != EffectState::Accepted {
            return Err(DatabaseError::EffectStateChanged.to_string());
        }
        commit_under_admission(
            transaction,
            |transaction| {
                if transaction.execute(
                    "UPDATE automation_external_effects SET state='dispatched',updated_at=MAX(updated_at,?1) WHERE effect_id=?2 AND instance_id=?3 AND state='accepted'",
                    params![now(), key.id(), instance],
                )? != 1 {
                    return Err(DatabaseError::EffectStateChanged);
                }
                Ok(())
            },
            admit,
        )
    }

    pub(crate) fn finish_external_effect(
        &self,
        key: &EffectKey,
        instance: &str,
        dispatched: bool,
        outcome: EffectOutcome,
    ) -> Result<EffectReceipt, String> {
        let mut db = self
            .open(&self.automation_path())
            .map_err(|error| error.to_string())?;
        db.busy_timeout(WRITER_WAIT)
            .map_err(|error| error.to_string())?;
        let transaction = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        current_host(&transaction, instance).map_err(|error| error.to_string())?;
        let stored = lookup(&transaction, key.id())
            .map_err(|error| error.to_string())?
            .ok_or_else(|| DatabaseError::EffectStateChanged.to_string())?;
        check_semantics(&stored, key).map_err(|error| error.to_string())?;
        let prior = if dispatched {
            EffectState::Dispatched
        } else {
            EffectState::Accepted
        };
        let state = match outcome {
            EffectOutcome::Succeeded if dispatched => "succeeded",
            EffectOutcome::Failed => "failed",
            EffectOutcome::Unknown if dispatched => "unknown",
            _ => return Err(DatabaseError::EffectStateChanged.to_string()),
        };
        if stored.instance != instance || stored.receipt.state != prior {
            return Err(DatabaseError::EffectStateChanged.to_string());
        }
        transaction.execute(
            "UPDATE automation_external_effects SET state=?1,updated_at=MAX(updated_at,?2) WHERE effect_id=?3",
            params![state, now(), key.id()],
        ).map_err(|error| error.to_string())?;
        let receipt = lookup(&transaction, key.id())
            .map_err(|error| error.to_string())?
            .ok_or_else(|| DatabaseError::EffectStateChanged.to_string())?
            .receipt;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(present(receipt))
    }

    pub(crate) fn load_effect_receipt(
        &self,
        id: &str,
    ) -> Result<Option<EffectReceipt>, DatabaseError> {
        require_identity(id)?;
        let db = self.open(&self.automation_path())?;
        Ok(lookup(&db, id)?.map(|stored| present(stored.receipt)))
    }

    pub(crate) fn load_effect_history(
        &self,
        before: Option<&str>,
        limit: u32,
    ) -> Result<EffectHistory, DatabaseError> {
        if let Some(before) = before {
            require_identity(before)?;
        }
        if !(1..=100).contains(&limit) {
            return Err(DatabaseError::Invalid(
                "effect history limit must be 1..=100".into(),
            ));
        }
        let db = self.open(&self.automation_path())?;
        let mut query = db.prepare(&format!(
            "SELECT {COLUMNS} FROM automation_external_effects WHERE (?1 IS NULL OR effect_id<?1) ORDER BY effect_id DESC LIMIT ?2"
        ))?;
        let mut items = query
            .query_map(params![before, i64::from(limit) + 1], read_effect)?
            .map(|row| row.map(|stored| present(stored.receipt)))
            .collect::<Result<Vec<_>, _>>()?;
        let next_before = if items.len() > limit as usize {
            items.truncate(limit as usize);
            items.last().map(|receipt| receipt.effect_id.clone())
        } else {
            None
        };
        Ok(EffectHistory { items, next_before })
    }
}
