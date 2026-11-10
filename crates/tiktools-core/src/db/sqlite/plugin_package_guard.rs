//! Native-only durable package mutation reservations.
//!
//! This storage primitive is not wired to package operations or sync yet. A
//! caller must hold the invocation gate and the cross-process admission lease
//! before every transition, and must never expose completion directly to a
//! remote caller. A mutating intent survives failure and requires a fresh
//! native reconciliation of the actual package and runtime.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::{behavior::advance_plugin_source, DatabaseError, DatabaseManager};

// Every active operation reserves one slot for its terminal receipt. No
// receipt may be silently evicted, since that could make an old ID replayable.
const MAX_PACKAGE_OPERATIONS: i64 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackageMutationKind {
    Install,
    Update,
    Rollback,
    Uninstall,
}
impl PackageMutationKind {
    fn wire(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Update => "update",
            Self::Rollback => "rollback",
            Self::Uninstall => "uninstall",
        }
    }
    fn parse(value: &str) -> Option<Self> {
        match value {
            "install" => Some(Self::Install),
            "update" => Some(Self::Update),
            "rollback" => Some(Self::Rollback),
            "uninstall" => Some(Self::Uninstall),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackageMutationPhase {
    Prepared,
    Mutating,
}
impl PackageMutationPhase {
    fn wire(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Mutating => "mutating",
        }
    }
    fn parse(value: &str) -> Option<Self> {
        match value {
            "prepared" => Some(Self::Prepared),
            "mutating" => Some(Self::Mutating),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageMutationIntent {
    pub plugin_id: String,
    pub intent_id: String,
    pub operation: PackageMutationKind,
    /// Opaque digest of the entire native operation, including plugin, kind,
    /// exact package content/version and options. The future caller must
    /// canonicalize and verify these inputs before allowing filesystem I/O.
    pub operation_digest: String,
    pub phase: PackageMutationPhase,
    pub source_generation: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackageMutationResolution {
    Cancelled,
    Reviewed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageMutationReceipt {
    pub plugin_id: String,
    pub intent_id: String,
    pub operation: PackageMutationKind,
    pub operation_digest: String,
    pub resolution: PackageMutationResolution,
    pub source_generation: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageMutationPage {
    pub intents: Vec<PackageMutationIntent>,
    pub next_cursor: Option<String>,
}

/// Native-only snapshot of a journal and its current source generation.
/// Neither account identity nor a pending candidate is included. The caller
/// must separately inspect the installed artifact and runtime while holding
/// the product's package and admission locks before treating it as reviewed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageMutationReview {
    intent: PackageMutationIntent,
    observed_generation: i64,
}

fn invalid() -> DatabaseError {
    DatabaseError::Invalid("invalid package mutation journal".into())
}
fn valid_token(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
}
fn valid_id(value: &str) -> bool {
    tiktools_plugin_api::manifest::is_valid_plugin_id(value)
}

type Stored = (String, String, String, Option<String>, String, i64);
fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Stored> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}
fn decode(
    (plugin_id, intent_id, operation, operation_digest, phase, source_generation): Stored,
) -> Result<PackageMutationIntent, DatabaseError> {
    let operation_digest = operation_digest
        .filter(|value| valid_token(value))
        .ok_or_else(invalid)?;
    if !valid_id(&plugin_id) || !valid_token(&intent_id) || source_generation <= 0 {
        return Err(invalid());
    }
    Ok(PackageMutationIntent {
        plugin_id,
        intent_id,
        operation: PackageMutationKind::parse(&operation).ok_or_else(invalid)?,
        operation_digest,
        phase: PackageMutationPhase::parse(&phase).ok_or_else(invalid)?,
        source_generation,
    })
}
fn load(db: &Connection, plugin_id: &str) -> Result<Option<PackageMutationIntent>, DatabaseError> {
    db.query_row("SELECT plugin_id,intent_id,operation,operation_digest,phase,source_generation FROM plugin_package_mutation_intents WHERE plugin_id=?1",
        [plugin_id], row).optional()?.map(decode).transpose()
}

fn ensure_unidentified_journal_absent(db: &Connection) -> Result<(), DatabaseError> {
    let corrupt: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM plugin_package_mutation_intents WHERE plugin_id IS NULL)",
        [],
        |row| row.get(0),
    )?;
    if corrupt {
        return Err(invalid());
    }
    Ok(())
}

fn receipt(
    db: &Connection,
    intent_id: &str,
) -> Result<Option<PackageMutationReceipt>, DatabaseError> {
    let saved: Option<(String, String, String, String, i64)> = db.query_row(
        "SELECT plugin_id,operation,operation_digest,resolution,source_generation FROM plugin_package_mutation_receipts WHERE intent_id=?1",
        [intent_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
    ).optional()?;
    let Some((plugin_id, operation, operation_digest, resolution, source_generation)) = saved
    else {
        return Ok(None);
    };
    if !valid_id(&plugin_id)
        || !valid_token(intent_id)
        || !valid_token(&operation_digest)
        || source_generation <= 0
    {
        return Err(invalid());
    }
    let resolution = match resolution.as_str() {
        "cancelled" => PackageMutationResolution::Cancelled,
        "reviewed" => PackageMutationResolution::Reviewed,
        _ => return Err(invalid()),
    };
    Ok(Some(PackageMutationReceipt {
        plugin_id,
        intent_id: intent_id.to_owned(),
        operation: PackageMutationKind::parse(&operation).ok_or_else(invalid)?,
        operation_digest,
        resolution,
        source_generation,
    }))
}

fn retain_receipt(
    db: &Connection,
    intent: &PackageMutationIntent,
    resolution: &str,
    generation: i64,
) -> Result<(), DatabaseError> {
    let inserted = db.execute(
        "INSERT INTO plugin_package_mutation_receipts(intent_id,plugin_id,operation,operation_digest,resolution,source_generation) VALUES(?1,?2,?3,?4,?5,?6)",
        params![intent.intent_id, intent.plugin_id, intent.operation.wire(), intent.operation_digest, resolution, generation],
    )?;
    if inserted != 1 {
        return Err(invalid());
    }
    Ok(())
}
fn matches_source(db: &Connection, intent: &PackageMutationIntent) -> Result<bool, DatabaseError> {
    let current: Option<i64> = db
        .query_row(
            "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id=?1",
            [&intent.plugin_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(current == Some(intent.source_generation))
}

impl DatabaseManager {
    /// Reserve a global plugin operation and advance its source in one durable
    /// transaction. Repeating the same identity returns the original intent;
    /// a different unresolved operation for the plugin is never overwritten.
    pub(crate) fn prepare_package_mutation(
        &self,
        plugin_id: &str,
        intent_id: &str,
        operation: PackageMutationKind,
        operation_digest: &str,
    ) -> Result<Option<PackageMutationIntent>, DatabaseError> {
        if !valid_id(plugin_id) || !valid_token(intent_id) || !valid_token(operation_digest) {
            return Err(invalid());
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_unidentified_journal_absent(&tx)?;
        if receipt(&tx, intent_id)?.is_some() {
            return Err(DatabaseError::Invalid(
                "package operation is already resolved".into(),
            ));
        }
        if let Some(existing) = load(&tx, plugin_id)? {
            if existing.intent_id == intent_id {
                if existing.operation != operation || existing.operation_digest != operation_digest
                {
                    return Err(invalid());
                }
                return Ok(Some(existing));
            }
            return Ok(None);
        }
        let reused: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_package_mutation_intents WHERE intent_id=?1)",
            [intent_id],
            |row| row.get(0),
        )?;
        if reused {
            return Err(invalid());
        }
        let reserved: i64 = tx.query_row("SELECT (SELECT COUNT(*) FROM plugin_package_mutation_intents) + (SELECT COUNT(*) FROM plugin_package_mutation_receipts)", [], |row| row.get(0))?;
        if reserved >= MAX_PACKAGE_OPERATIONS {
            return Err(DatabaseError::Invalid(
                "package operation history is full".into(),
            ));
        }
        let source_generation = advance_plugin_source(&tx, plugin_id)?;
        let inserted = tx.execute("INSERT INTO plugin_package_mutation_intents(plugin_id,intent_id,operation,operation_digest,phase,source_generation) VALUES(?1,?2,?3,?4,'prepared',?5)",
            params![plugin_id, intent_id, operation.wire(), operation_digest, source_generation])?;
        if inserted != 1 {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(Some(PackageMutationIntent {
            plugin_id: plugin_id.into(),
            intent_id: intent_id.into(),
            operation,
            operation_digest: operation_digest.to_owned(),
            phase: PackageMutationPhase::Prepared,
            source_generation,
        }))
    }

    pub(crate) fn package_mutation(
        &self,
        plugin_id: &str,
    ) -> Result<Option<PackageMutationIntent>, DatabaseError> {
        if !valid_id(plugin_id) {
            return Err(invalid());
        }
        let db = self.open(&self.automation_path())?;
        ensure_unidentified_journal_absent(&db)?;
        load(&db, plugin_id)
    }

    /// A resolved ID stays reserved permanently, including cancellation and
    /// an uncertain filesystem outcome subsequently reviewed by native code.
    pub(crate) fn package_mutation_receipt(
        &self,
        intent_id: &str,
    ) -> Result<Option<PackageMutationReceipt>, DatabaseError> {
        if !valid_token(intent_id) {
            return Err(invalid());
        }
        let db = self.open(&self.automation_path())?;
        receipt(&db, intent_id)
    }

    /// Discover all unresolved intents, including packages no longer present
    /// on disk. Identity and consent are deliberately not partition keys.
    pub(crate) fn package_mutation_page(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<PackageMutationPage, DatabaseError> {
        if !(1..=64).contains(&limit) || after.is_some_and(|id| !valid_id(id)) {
            return Err(invalid());
        }
        let db = self.open(&self.automation_path())?;
        ensure_unidentified_journal_absent(&db)?;
        let sql = if after.is_some() {
            "SELECT plugin_id,intent_id,operation,operation_digest,phase,source_generation FROM plugin_package_mutation_intents WHERE plugin_id>?1 ORDER BY plugin_id COLLATE BINARY LIMIT ?2"
        } else {
            "SELECT plugin_id,intent_id,operation,operation_digest,phase,source_generation FROM plugin_package_mutation_intents WHERE plugin_id>=?1 ORDER BY plugin_id COLLATE BINARY LIMIT ?2"
        };
        let mut stmt = db.prepare(sql)?;
        let rows = stmt
            .query_map(params![after.unwrap_or(""), (limit + 1) as i64], row)?
            .collect::<Result<Vec<_>, _>>()?;
        let more = rows.len() > limit;
        let intents = rows
            .into_iter()
            .take(limit)
            .map(decode)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = if more {
            intents.last().map(|item| item.plugin_id.clone())
        } else {
            None
        };
        Ok(PackageMutationPage {
            intents,
            next_cursor,
        })
    }

    /// Transition before any filesystem effect. A false return, including a
    /// repeated call after a lost reply, must never cause effects to replay.
    pub(crate) fn begin_package_mutation(
        &self,
        intent: &PackageMutationIntent,
    ) -> Result<bool, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_unidentified_journal_absent(&tx)?;
        if intent.phase != PackageMutationPhase::Prepared
            || load(&tx, &intent.plugin_id)?.as_ref() != Some(intent)
            || !matches_source(&tx, intent)?
        {
            return Ok(false);
        }
        let changed = tx.execute("UPDATE plugin_package_mutation_intents SET phase='mutating' WHERE plugin_id=?1 AND intent_id=?2 AND operation=?3 AND operation_digest=?4 AND phase='prepared' AND source_generation=?5",
            params![intent.plugin_id, intent.intent_id, intent.operation.wire(), intent.operation_digest, intent.source_generation])?;
        if changed != 1 {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(true)
    }

    /// Only the original still-prepared operation can be abandoned without a
    /// filesystem review. The initial source advance is intentionally retained.
    pub(crate) fn cancel_prepared_package_mutation(
        &self,
        intent: &PackageMutationIntent,
    ) -> Result<bool, DatabaseError> {
        self.remove_package_mutation(intent, PackageMutationPhase::Prepared, false)
    }

    /// Capture the journal and current global source generation for a native
    /// recovery review. A newer generation is retained as metadata only; no
    /// account, candidate, plugin contents or runtime value is copied here.
    pub(crate) fn review_package_mutation(
        &self,
        plugin_id: &str,
        intent_id: &str,
    ) -> Result<Option<PackageMutationReview>, DatabaseError> {
        if !valid_id(plugin_id) || !valid_token(intent_id) {
            return Err(invalid());
        }
        let db = self.open(&self.automation_path())?;
        ensure_unidentified_journal_absent(&db)?;
        let Some(intent) = load(&db, plugin_id)? else {
            return Ok(None);
        };
        if intent.intent_id != intent_id {
            return Ok(None);
        }
        let current: Option<i64> = db
            .query_row(
                "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id=?1",
                [plugin_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(observed_generation) =
            current.filter(|generation| *generation >= intent.source_generation)
        else {
            return Err(invalid());
        };
        Ok(Some(PackageMutationReview {
            intent,
            observed_generation,
        }))
    }

    /// A prepared intent has not permitted filesystem work. After a fresh
    /// native review, it may be removed even if a different source writer
    /// advanced the generation. No source, candidate, or runtime is changed.
    pub(crate) fn cancel_package_mutation_after_native_review(
        &self,
        review: &PackageMutationReview,
    ) -> Result<bool, DatabaseError> {
        let intent = &review.intent;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_unidentified_journal_absent(&tx)?;
        if intent.phase != PackageMutationPhase::Prepared
            || load(&tx, &intent.plugin_id)?.as_ref() != Some(intent)
        {
            return Ok(false);
        }
        let current: Option<i64> = tx
            .query_row(
                "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id=?1",
                [&intent.plugin_id],
                |row| row.get(0),
            )
            .optional()?;
        if current != Some(review.observed_generation)
            || review.observed_generation < intent.source_generation
        {
            return Ok(false);
        }
        retain_receipt(&tx, intent, "cancelled", review.observed_generation)?;
        let deleted = tx.execute("DELETE FROM plugin_package_mutation_intents WHERE plugin_id=?1 AND intent_id=?2 AND operation=?3 AND operation_digest=?4 AND phase='prepared' AND source_generation=?5",
            params![intent.plugin_id, intent.intent_id, intent.operation.wire(), intent.operation_digest, intent.source_generation])?;
        if deleted != 1 {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(true)
    }

    /// Retire a mutating intent only after the native caller has reviewed the
    /// actual package and runtime under the common mutation locks. The exact
    /// observed generation must still match, even after another account's
    /// edit. This never changes runtime, source state, or any outbox row.
    pub(crate) fn finish_package_mutation_after_native_review(
        &self,
        review: &PackageMutationReview,
    ) -> Result<bool, DatabaseError> {
        let intent = &review.intent;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_unidentified_journal_absent(&tx)?;
        if intent.phase != PackageMutationPhase::Mutating
            || load(&tx, &intent.plugin_id)?.as_ref() != Some(intent)
        {
            return Ok(false);
        }
        let current: Option<i64> = tx
            .query_row(
                "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id=?1",
                [&intent.plugin_id],
                |row| row.get(0),
            )
            .optional()?;
        if current != Some(review.observed_generation)
            || review.observed_generation < intent.source_generation
        {
            return Ok(false);
        }
        let resulting_generation = advance_plugin_source(&tx, &intent.plugin_id)?;
        retain_receipt(&tx, intent, "reviewed", resulting_generation)?;
        let deleted = tx.execute("DELETE FROM plugin_package_mutation_intents WHERE plugin_id=?1 AND intent_id=?2 AND operation=?3 AND operation_digest=?4 AND phase='mutating' AND source_generation=?5",
            params![intent.plugin_id, intent.intent_id, intent.operation.wire(), intent.operation_digest, intent.source_generation])?;
        if deleted != 1 {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(true)
    }

    fn remove_package_mutation(
        &self,
        intent: &PackageMutationIntent,
        phase: PackageMutationPhase,
        advance: bool,
    ) -> Result<bool, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_unidentified_journal_absent(&tx)?;
        if intent.phase != phase
            || load(&tx, &intent.plugin_id)?.as_ref() != Some(intent)
            || !matches_source(&tx, intent)?
        {
            return Ok(false);
        }
        let generation = if advance {
            advance_plugin_source(&tx, &intent.plugin_id)?
        } else {
            intent.source_generation
        };
        retain_receipt(
            &tx,
            intent,
            if advance { "reviewed" } else { "cancelled" },
            generation,
        )?;
        let deleted = tx.execute("DELETE FROM plugin_package_mutation_intents WHERE plugin_id=?1 AND intent_id=?2 AND operation=?3 AND operation_digest=?4 AND phase=?5 AND source_generation=?6",
            params![intent.plugin_id, intent.intent_id, intent.operation.wire(), intent.operation_digest, phase.wire(), intent.source_generation])?;
        if deleted != 1 {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AppPaths;
    use std::{
        path::PathBuf,
        sync::{Arc, Barrier},
    };

    fn paths(root: PathBuf) -> AppPaths {
        AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root,
        }
    }
    struct Fixture {
        db: DatabaseManager,
        root: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "package-guard-{}-{}",
                std::process::id(),
                fastrand::u64(..)
            ));
            let paths = paths(root.clone());
            paths.ensure_directories().unwrap();
            Self {
                db: DatabaseManager::new(paths),
                root,
            }
        }
        fn generation(&self, plugin: &str) -> Option<i64> {
            self.db
                .open(&self.db.automation_path())
                .unwrap()
                .query_row(
                    "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id=?1",
                    [plugin],
                    |row| row.get(0),
                )
                .optional()
                .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn journal_reserves_atomically_and_never_clobbers_another_identity() {
        let f = Fixture::new();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_package_journal BEFORE INSERT ON plugin_package_mutation_intents BEGIN SELECT RAISE(FAIL,'injected'); END;").unwrap();
        drop(db);
        assert!(f
            .db
            .prepare_package_mutation(
                "acme.plugin",
                &"a".repeat(64),
                PackageMutationKind::Install,
                &"c".repeat(64)
            )
            .is_err());
        assert_eq!(f.generation("acme.plugin"), None);
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("DROP TRIGGER fail_package_journal;")
            .unwrap();
        drop(db);
        let first =
            f.db.prepare_package_mutation(
                "acme.plugin",
                &"a".repeat(64),
                PackageMutationKind::Install,
                &"c".repeat(64),
            )
            .unwrap()
            .unwrap();
        assert_eq!(first.source_generation, 1);
        assert_eq!(
            f.db.prepare_package_mutation(
                "acme.plugin",
                &"a".repeat(64),
                PackageMutationKind::Install,
                &"c".repeat(64),
            )
            .unwrap(),
            Some(first.clone())
        );
        assert_eq!(f.generation("acme.plugin"), Some(1));
        assert!(f
            .db
            .prepare_package_mutation(
                "acme.plugin",
                &"b".repeat(64),
                PackageMutationKind::Update,
                &"c".repeat(64)
            )
            .unwrap()
            .is_none());
        assert!(f
            .db
            .prepare_package_mutation(
                "other.plugin",
                &"a".repeat(64),
                PackageMutationKind::Install,
                &"c".repeat(64),
            )
            .is_err());
        assert_eq!(f.db.package_mutation("acme.plugin").unwrap(), Some(first));
        assert_eq!(f.generation("other.plugin"), None);
    }

    #[test]
    fn frozen_digest_and_terminal_receipts_prevent_changed_or_replayed_operations() {
        let f = Fixture::new();
        let id = "a".repeat(64);
        let digest = "c".repeat(64);
        let first = f
            .db
            .prepare_package_mutation("acme.plugin", &id, PackageMutationKind::Install, &digest)
            .unwrap()
            .unwrap();
        assert!(f
            .db
            .prepare_package_mutation(
                "acme.plugin",
                &id,
                PackageMutationKind::Install,
                &"d".repeat(64)
            )
            .is_err());
        assert!(f
            .db
            .prepare_package_mutation("acme.plugin", &id, PackageMutationKind::Rollback, &digest)
            .is_err());
        assert!(f.db.begin_package_mutation(&first).unwrap());
        let review =
            f.db.review_package_mutation("acme.plugin", &id)
                .unwrap()
                .unwrap();
        assert!(f
            .db
            .finish_package_mutation_after_native_review(&review)
            .unwrap());
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert_eq!(
            reopened.package_mutation_receipt(&id).unwrap(),
            Some(PackageMutationReceipt {
                plugin_id: "acme.plugin".into(),
                intent_id: id.clone(),
                operation: PackageMutationKind::Install,
                operation_digest: digest.clone(),
                resolution: PackageMutationResolution::Reviewed,
                source_generation: 2,
            })
        );
        assert!(reopened
            .prepare_package_mutation("acme.plugin", &id, PackageMutationKind::Install, &digest)
            .is_err());
        assert!(reopened
            .prepare_package_mutation("another.plugin", &id, PackageMutationKind::Install, &digest)
            .is_err());
        assert_eq!(reopened.package_mutation("acme.plugin").unwrap(), None);
        assert_eq!(f.generation("acme.plugin"), Some(2));
        let cancel_id = "b".repeat(64);
        let cancel = reopened
            .prepare_package_mutation(
                "acme.plugin",
                &cancel_id,
                PackageMutationKind::Uninstall,
                &digest,
            )
            .unwrap()
            .unwrap();
        assert!(reopened.cancel_prepared_package_mutation(&cancel).unwrap());
        assert_eq!(
            reopened
                .package_mutation_receipt(&cancel_id)
                .unwrap()
                .unwrap()
                .resolution,
            PackageMutationResolution::Cancelled
        );
        assert!(reopened
            .prepare_package_mutation(
                "acme.plugin",
                &cancel_id,
                PackageMutationKind::Uninstall,
                &digest
            )
            .is_err());
    }

    #[test]
    fn full_receipt_history_refuses_new_reservation_without_source_change() {
        let f = Fixture::new();
        let mut db = f.db.open(&f.db.automation_path()).unwrap();
        let tx = db.transaction().unwrap();
        {
            let mut insert = tx.prepare("INSERT INTO plugin_package_mutation_receipts(intent_id,plugin_id,operation,operation_digest,resolution,source_generation) VALUES (?1,'other.plugin','install',?2,'cancelled',1)").unwrap();
            let digest = "c".repeat(64);
            for number in 0..MAX_PACKAGE_OPERATIONS {
                insert
                    .execute(params![format!("{number:064x}"), digest])
                    .unwrap();
            }
        }
        tx.commit().unwrap();
        drop(db);
        assert!(f
            .db
            .prepare_package_mutation(
                "acme.plugin",
                &"f".repeat(64),
                PackageMutationKind::Install,
                &"c".repeat(64)
            )
            .is_err());
        assert_eq!(f.generation("acme.plugin"), None);
        assert_eq!(f.db.package_mutation("acme.plugin").unwrap(), None);
    }

    #[test]
    fn legacy_unknown_digest_and_null_identity_are_preserved_and_fail_closed() {
        let f = Fixture::new();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("DROP TABLE plugin_package_mutation_intents; CREATE TABLE plugin_package_mutation_intents (plugin_id TEXT PRIMARY KEY, intent_id TEXT NOT NULL UNIQUE, operation TEXT NOT NULL, phase TEXT NOT NULL, source_generation INTEGER NOT NULL);").unwrap();
        db.execute("INSERT INTO plugin_package_mutation_intents VALUES ('acme.plugin',?1,'install','mutating',1)", ["a".repeat(64)]).unwrap();
        drop(db);
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert!(reopened.package_mutation("acme.plugin").is_err());
        assert!(reopened.package_mutation_page(None, 10).is_err());
        let db = reopened.open(&reopened.automation_path()).unwrap();
        let digest: Option<String> = db.query_row("SELECT operation_digest FROM plugin_package_mutation_intents WHERE plugin_id='acme.plugin'", [], |row| row.get(0)).unwrap();
        assert_eq!(digest, None);
        db.execute(
            "DELETE FROM plugin_package_mutation_intents WHERE plugin_id='acme.plugin'",
            [],
        )
        .unwrap();
        db.execute("INSERT INTO plugin_package_mutation_intents(plugin_id,intent_id,operation,phase,source_generation,operation_digest) VALUES (NULL,?1,'install','prepared',1,?2)", params!["b".repeat(64), "c".repeat(64)]).unwrap();
        drop(db);
        assert!(reopened.package_mutation_page(None, 10).is_err());
        assert!(reopened.package_mutation("another.plugin").is_err());
        assert!(reopened
            .prepare_package_mutation(
                "another.plugin",
                &"d".repeat(64),
                PackageMutationKind::Install,
                &"c".repeat(64)
            )
            .is_err());
    }

    #[test]
    fn discovery_pages_include_uninstalled_ids_and_reject_corrupt_rows() {
        let f = Fixture::new();
        for (n, plugin) in ["a.plugin", "b.plugin", "c.plugin"].into_iter().enumerate() {
            f.db.prepare_package_mutation(
                plugin,
                &format!("{n:064x}"),
                PackageMutationKind::Uninstall,
                &"c".repeat(64),
            )
            .unwrap()
            .unwrap();
        }
        let first = f.db.package_mutation_page(None, 2).unwrap();
        assert_eq!(
            first
                .intents
                .iter()
                .map(|v| v.plugin_id.as_str())
                .collect::<Vec<_>>(),
            ["a.plugin", "b.plugin"]
        );
        assert_eq!(first.next_cursor.as_deref(), Some("b.plugin"));
        let second =
            f.db.package_mutation_page(first.next_cursor.as_deref(), 2)
                .unwrap();
        assert_eq!(second.intents[0].plugin_id, "c.plugin");
        assert_eq!(second.next_cursor, None);
        assert!(f.db.package_mutation_page(None, 0).is_err());
        assert!(f.db.package_mutation_page(Some("../invalid"), 2).is_err());
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute("UPDATE plugin_package_mutation_intents SET intent_id='broken' WHERE plugin_id='a.plugin'", []).unwrap();
        drop(db);
        assert!(f.db.package_mutation_page(None, 2).is_err());
        assert!(f.db.package_mutation("a.plugin").is_err());
    }

    #[test]
    fn started_operation_requires_native_review_and_atomic_finalization() {
        let f = Fixture::new();
        let before =
            f.db.prepare_package_mutation(
                "acme.plugin",
                &"a".repeat(64),
                PackageMutationKind::Update,
                &"c".repeat(64),
            )
            .unwrap()
            .unwrap();
        assert!(f.db.begin_package_mutation(&before).unwrap());
        // A lost start reply is uncertain, never permission to repeat effects.
        assert!(!f.db.begin_package_mutation(&before).unwrap());
        assert!(!f.db.cancel_prepared_package_mutation(&before).unwrap());
        let started = f.db.package_mutation("acme.plugin").unwrap().unwrap();
        assert_eq!(started.phase, PackageMutationPhase::Mutating);
        let review =
            f.db.review_package_mutation("acme.plugin", &started.intent_id)
                .unwrap()
                .unwrap();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_package_clear BEFORE DELETE ON plugin_package_mutation_intents BEGIN SELECT RAISE(FAIL,'injected'); END;").unwrap();
        drop(db);
        assert!(f
            .db
            .finish_package_mutation_after_native_review(&review)
            .is_err());
        assert_eq!(
            f.db.package_mutation_receipt(&before.intent_id).unwrap(),
            None
        );
        assert_eq!(f.generation("acme.plugin"), Some(1));
        assert_eq!(
            f.db.package_mutation("acme.plugin").unwrap(),
            Some(started.clone())
        );
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("DROP TRIGGER fail_package_clear;")
            .unwrap();
        drop(db);
        assert!(f
            .db
            .finish_package_mutation_after_native_review(&review)
            .unwrap());
        assert_eq!(f.generation("acme.plugin"), Some(2));
        assert_eq!(f.db.package_mutation("acme.plugin").unwrap(), None);
        assert!(!f
            .db
            .finish_package_mutation_after_native_review(&review)
            .unwrap());
    }

    #[test]
    fn newer_source_requires_fresh_review_without_touching_another_accounts_candidate() {
        let f = Fixture::new();
        let before =
            f.db.prepare_package_mutation(
                "acme.plugin",
                &"a".repeat(64),
                PackageMutationKind::Install,
                &"c".repeat(64),
            )
            .unwrap()
            .unwrap();
        f.db.set_plugin_state("acme.plugin", true, false).unwrap();
        assert!(!f.db.begin_package_mutation(&before).unwrap());
        assert!(!f.db.cancel_prepared_package_mutation(&before).unwrap());
        assert_eq!(
            f.db.package_mutation("acme.plugin").unwrap(),
            Some(before.clone())
        );
        let stale_prepared =
            f.db.review_package_mutation("acme.plugin", &before.intent_id)
                .unwrap()
                .unwrap();
        f.db.set_plugin_state("acme.plugin", true, true).unwrap();
        assert!(!f
            .db
            .cancel_package_mutation_after_native_review(&stale_prepared)
            .unwrap());
        let reviewed =
            f.db.review_package_mutation("acme.plugin", &before.intent_id)
                .unwrap()
                .unwrap();
        assert!(f
            .db
            .cancel_package_mutation_after_native_review(&reviewed)
            .unwrap());
        assert_eq!(f.generation("acme.plugin"), Some(3));
        assert_eq!(f.db.package_mutation("acme.plugin").unwrap(), None);

        let prepared =
            f.db.prepare_package_mutation(
                "other.plugin",
                &"b".repeat(64),
                PackageMutationKind::Rollback,
                &"c".repeat(64),
            )
            .unwrap()
            .unwrap();
        assert!(f.db.begin_package_mutation(&prepared).unwrap());
        let started = f.db.package_mutation("other.plugin").unwrap().unwrap();
        let stale_review =
            f.db.review_package_mutation("other.plugin", &started.intent_id)
                .unwrap()
                .unwrap();
        f.db.set_plugin_state("other.plugin", true, true).unwrap();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute("INSERT INTO plugin_sync_outbox(origin,account_id,plugin_id,generation,config_generation,version,enabled) VALUES ('https://cloud.example','bob','other.plugin',2,'7','1.0.0',1)", []).unwrap();
        drop(db);
        assert!(!f
            .db
            .finish_package_mutation_after_native_review(&stale_review)
            .unwrap());
        assert_eq!(
            f.db.package_mutation("other.plugin").unwrap(),
            Some(started.clone())
        );
        let review =
            f.db.review_package_mutation("other.plugin", &started.intent_id)
                .unwrap()
                .unwrap();
        assert_eq!(review.observed_generation, 2);
        assert!(!format!("{review:?}").contains("bob"));
        assert!(f
            .db
            .finish_package_mutation_after_native_review(&review)
            .unwrap());
        assert_eq!(f.generation("other.plugin"), Some(3));
        let db = f.db.open(&f.db.automation_path()).unwrap();
        let other: (String, String, i64, String, i64) = db.query_row("SELECT origin,account_id,generation,version,enabled FROM plugin_sync_outbox WHERE plugin_id='other.plugin'", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))).unwrap();
        assert_eq!(
            other,
            (
                "https://cloud.example".into(),
                "bob".into(),
                2,
                "1.0.0".into(),
                1
            )
        );
        let state: (i64, i64) = db
            .query_row(
                "SELECT installed,enabled FROM behavior_plugins WHERE id='other.plugin'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(state, (1, 1));
    }

    #[test]
    fn competing_reservations_keep_exactly_one_journal() {
        let f = Fixture::new();
        let db = Arc::new(f.db.clone());
        let barrier = Arc::new(Barrier::new(3));
        let threads: Vec<_> = ["a", "b"]
            .into_iter()
            .map(|hex| {
                let db = db.clone();
                let barrier = barrier.clone();
                let id = hex.repeat(64);
                std::thread::spawn(move || {
                    barrier.wait();
                    db.prepare_package_mutation(
                        "acme.plugin",
                        &id,
                        PackageMutationKind::Update,
                        &"c".repeat(64),
                    )
                })
            })
            .collect();
        barrier.wait();
        let results: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Ok(Some(_))))
                .count(),
            1
        );
        assert!(results.iter().all(|result| result.is_ok()));
        assert_eq!(f.generation("acme.plugin"), Some(1));
        assert!(f.db.package_mutation("acme.plugin").unwrap().is_some());
    }

    #[test]
    fn crash_child() {
        let Some(root) = std::env::var_os("TIKTOOLS_PACKAGE_GUARD_CRASH_FIXTURE") else {
            return;
        };
        let db = DatabaseManager::new(paths(PathBuf::from(root)));
        let prepared = db.package_mutation("acme.plugin").unwrap().unwrap();
        assert!(db.begin_package_mutation(&prepared).unwrap());
        std::process::exit(31);
    }

    #[test]
    fn process_death_after_begin_leaves_uncertain_intent_without_replay() {
        let f = Fixture::new();
        let prepared =
            f.db.prepare_package_mutation(
                "acme.plugin",
                &"a".repeat(64),
                PackageMutationKind::Uninstall,
                &"c".repeat(64),
            )
            .unwrap()
            .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "db::sqlite::plugin_package_guard::tests::crash_child",
            ])
            .env("TIKTOOLS_PACKAGE_GUARD_CRASH_FIXTURE", &f.root)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(31));
        let reopened = DatabaseManager::new(f.db.paths().clone());
        let journal = reopened.package_mutation("acme.plugin").unwrap().unwrap();
        assert_eq!(journal.phase, PackageMutationPhase::Mutating);
        assert_eq!(journal.intent_id, prepared.intent_id);
        assert!(!reopened.begin_package_mutation(&prepared).unwrap());
        assert!(!reopened
            .cancel_prepared_package_mutation(&prepared)
            .unwrap());
    }
}
