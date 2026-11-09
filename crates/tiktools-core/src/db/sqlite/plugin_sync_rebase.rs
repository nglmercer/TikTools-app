//! SQLite half of a reviewed v1 keep-local decision. The coordinator owns
//! the other half (the generation-checked JSON state and fresh Cloud read).
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use tiktools_cloud_sync::{
    PluginSelection, V1ConflictSourceProof, V1Edit, V1KeepLocalIntent, V1LocalEditContext,
    V1OutboxCandidate, V1Pending,
};

use super::{util::bool_int, DatabaseError, DatabaseManager};

fn invalid() -> DatabaseError {
    DatabaseError::Invalid("invalid sync rebase journal".into())
}

fn payload(
    context: &V1LocalEditContext,
    intent: &V1KeepLocalIntent,
) -> Result<String, DatabaseError> {
    if context.origin != intent.origin()
        || context.account_id != intent.account_id()
        || context.config_generation != intent.config_generation()
        || intent.source_generation() <= 0
        || intent.before_generation() == 0
        || intent.key().is_empty()
    {
        return Err(invalid());
    }
    let pending = intent
        .before()
        .pending
        .as_ref()
        .filter(|pending| pending.conflict)
        .ok_or_else(invalid)?;
    let cloud = intent.cloud();
    let expected = if matches!(pending.edit, V1Edit::Delete) && cloud.is_none() {
        None
    } else {
        Some(V1Pending {
            base_revision: cloud.map(|record| record.revision),
            edit: pending.edit.clone(),
            conflict: false,
        })
    };
    if intent.after().pending != expected
        || intent.after().remote.as_ref() != cloud
        || cloud.is_some_and(|record| record.key != intent.key() || record.revision <= 0)
    {
        return Err(invalid());
    }
    let encoded = serde_json::to_string(intent)?;
    if encoded.len() > 16 * 1024 {
        return Err(invalid());
    }
    Ok(encoded)
}

fn source_receipt(
    db: &Connection,
    context: &V1LocalEditContext,
    intent: &V1KeepLocalIntent,
    epoch: &str,
) -> Result<Option<String>, DatabaseError> {
    let pending = intent.before().pending.as_ref().ok_or_else(invalid)?;
    let (installed, enabled) = match &pending.edit {
        V1Edit::Put(value) if value.plugin_id == intent.key() => (true, Some(value.enabled)),
        V1Edit::Delete => (false, None),
        _ => return Ok(None),
    };
    let stored: Option<String> = db.query_row(
        "SELECT r.pending_json FROM plugin_sync_receipts r JOIN plugin_sync_source_versions s ON s.plugin_id=r.plugin_id
         JOIN behavior_plugins b ON b.id=r.plugin_id WHERE r.origin=?1 AND r.account_id=?2 AND r.plugin_id=?3
         AND r.generation=?4 AND s.generation=r.generation AND r.config_generation=?5 AND r.runtime_epoch=?6
         AND b.installed=?7 AND (?8 IS NULL OR b.enabled=?8)
         AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=r.plugin_id)",
        params![context.origin, context.account_id, intent.key(), intent.source_generation(), context.config_generation.to_string(), epoch,
            bool_int(installed), enabled.map(bool_int)], |row| row.get(0),
    ).optional()?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let Some(mut saved) = serde_json::from_str::<V1Pending>(&stored).ok() else {
        return Ok(None);
    };
    saved.conflict = true;
    Ok((saved == *pending).then_some(stored))
}

fn valid_proof(intent: &V1KeepLocalIntent, epoch: &str, proof: &V1ConflictSourceProof) -> bool {
    V1ConflictSourceProof::new(intent.source_generation(), epoch.to_owned()).as_ref() == Some(proof)
}

// Check the journal and candidate in the same SQLite snapshot. Callers must
// still hold the protocol admission lease and validate the current session.
fn newer_candidate(
    db: &Connection,
    context: &V1LocalEditContext,
    intent: &V1KeepLocalIntent,
    epoch: &str,
    encoded: &str,
) -> Result<Option<(V1OutboxCandidate, V1ConflictSourceProof)>, DatabaseError> {
    let row: Option<(i64, Option<String>, Option<i64>)> = db.query_row(
        "SELECT o.generation,o.version,o.enabled FROM plugin_sync_rebase_intents j
         JOIN plugin_sync_outbox o ON o.origin=j.origin AND o.account_id=j.account_id AND o.plugin_id=j.plugin_id
         JOIN plugin_sync_source_versions s ON s.plugin_id=o.plugin_id
         JOIN behavior_plugins b ON b.id=o.plugin_id
         WHERE j.origin=?1 AND j.account_id=?2 AND j.plugin_id=?3 AND j.payload_json=?4
         AND o.config_generation=?5 AND o.runtime_epoch=?6 AND o.generation>?7
         AND s.generation=o.generation
         AND b.installed=CASE WHEN o.version IS NULL THEN 0 ELSE 1 END
         AND (o.version IS NULL OR b.enabled=o.enabled)",
        params![context.origin, context.account_id, intent.key(), encoded,
            context.config_generation.to_string(), epoch, intent.source_generation()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional()?;
    let Some((generation, version, enabled)) = row else {
        return Ok(None);
    };
    let Some(proof) = V1ConflictSourceProof::new(generation, epoch.to_owned()) else {
        return Ok(None);
    };
    let edit = match (version, enabled) {
        (Some(version), Some(enabled @ (0 | 1)))
            if !version.is_empty()
                && version.len() <= 128
                && !version.chars().any(char::is_control) =>
        {
            V1Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: intent.key().to_owned(),
                version,
                enabled: enabled == 1,
            })
        }
        (None, None) => V1Edit::Delete,
        _ => return Err(invalid()),
    };
    Ok(Some((
        V1OutboxCandidate {
            key: intent.key().to_owned(),
            edit,
            generation,
        },
        proof,
    )))
}

// Only the global source revision crosses this boundary. In particular, do
// not inspect or disclose which account (if any) made the superseding edit.
fn superseding_generation(
    db: &Connection,
    context: &V1LocalEditContext,
    intent: &V1KeepLocalIntent,
    encoded: &str,
) -> Result<Option<i64>, DatabaseError> {
    Ok(db
        .query_row(
            "SELECT s.generation FROM plugin_sync_rebase_intents j
         JOIN plugin_sync_source_versions s ON s.plugin_id=j.plugin_id
         WHERE j.origin=?1 AND j.account_id=?2 AND j.plugin_id=?3
         AND j.payload_json=?4 AND s.generation>?5",
            params![
                context.origin,
                context.account_id,
                intent.key(),
                encoded,
                intent.source_generation()
            ],
            |row| row.get(0),
        )
        .optional()?)
}

impl DatabaseManager {
    pub(crate) fn superseded_plugin_sync_rebase_generation(
        &self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
    ) -> Result<Option<i64>, DatabaseError> {
        let encoded = payload(context, intent)?;
        let db = self.open(&self.automation_path())?;
        superseding_generation(&db, context, intent, &encoded)
    }

    /// Clear only the frozen journal, after the coordinator has restored its
    /// exact conflicted state. Never copy another account's source or outbox.
    pub(crate) fn abandon_superseded_plugin_sync_rebase(
        &self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        generation: i64,
    ) -> Result<bool, DatabaseError> {
        let encoded = payload(context, intent)?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if superseding_generation(&tx, context, intent, &encoded)? != Some(generation) {
            return Ok(false);
        }
        let removed = tx.execute(
            "DELETE FROM plugin_sync_rebase_intents WHERE origin=?1 AND account_id=?2 AND plugin_id=?3 AND payload_json=?4",
            params![context.origin, context.account_id, intent.key(), encoded],
        )?;
        if removed != 1 {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(true)
    }

    pub(crate) fn newer_plugin_sync_rebase_candidate(
        &self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        epoch: &str,
    ) -> Result<Option<(V1OutboxCandidate, V1ConflictSourceProof)>, DatabaseError> {
        let encoded = payload(context, intent)?;
        let db = self.open(&self.automation_path())?;
        newer_candidate(&db, context, intent, epoch, &encoded)
    }

    /// Remove only the exact journal after the coordinator has restored the
    /// conflict. A concurrent new candidate invalidates the reviewed proof;
    /// the old receipt and new candidate remain untouched on every path.
    pub(crate) fn abandon_plugin_sync_rebase(
        &self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        epoch: &str,
        candidate: &V1OutboxCandidate,
        proof: &V1ConflictSourceProof,
    ) -> Result<bool, DatabaseError> {
        let encoded = payload(context, intent)?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if newer_candidate(&tx, context, intent, epoch, &encoded)?.as_ref()
            != Some(&(candidate.clone(), proof.clone()))
        {
            return Ok(false);
        }
        let removed = tx.execute(
            "DELETE FROM plugin_sync_rebase_intents WHERE origin=?1 AND account_id=?2 AND plugin_id=?3 AND payload_json=?4",
            params![context.origin, context.account_id, intent.key(), encoded],
        )?;
        if removed != 1 {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(true)
    }

    pub(crate) fn has_plugin_sync_rebase(
        &self,
        context: &V1LocalEditContext,
    ) -> Result<bool, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents WHERE origin=?1 AND account_id=?2)",
            params![context.origin, context.account_id], |row| row.get(0))?)
    }

    pub(crate) fn prepare_plugin_sync_rebase(
        &self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        epoch: &str,
        proof: &V1ConflictSourceProof,
    ) -> Result<bool, DatabaseError> {
        let encoded = payload(context, intent)?;
        if intent.initial_runtime_epoch() != epoch || !valid_proof(intent, epoch, proof) {
            return Ok(false);
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if source_receipt(&tx, context, intent, epoch)?.is_none() {
            return Ok(false);
        }
        let existing: Option<String> = tx.query_row("SELECT payload_json FROM plugin_sync_rebase_intents WHERE origin=?1 AND account_id=?2",
            params![context.origin, context.account_id], |row| row.get(0)).optional()?;
        if let Some(existing) = existing {
            return Ok(existing == encoded);
        }
        let changed = tx.execute("INSERT INTO plugin_sync_rebase_intents(origin,account_id,plugin_id,payload_json) VALUES(?1,?2,?3,?4)",
            params![context.origin, context.account_id, intent.key(), encoded])?;
        if changed != 1 {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(true)
    }

    pub(crate) fn load_plugin_sync_rebase(
        &self,
        context: &V1LocalEditContext,
        key: &str,
    ) -> Result<Option<V1KeepLocalIntent>, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        let saved: Option<(String, String)> = db.query_row("SELECT plugin_id,payload_json FROM plugin_sync_rebase_intents WHERE origin=?1 AND account_id=?2",
            params![context.origin, context.account_id], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
        let Some((plugin_id, encoded)) = saved else {
            return Ok(None);
        };
        if plugin_id != key || encoded.len() > 16 * 1024 {
            return Err(invalid());
        }
        let intent: V1KeepLocalIntent = serde_json::from_str(&encoded).map_err(|_| invalid())?;
        if payload(context, &intent)? != encoded || intent.key() != key {
            return Err(invalid());
        }
        Ok(Some(intent))
    }

    pub(crate) fn plugin_sync_rebase_source(
        &self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        epoch: &str,
    ) -> Result<Option<V1ConflictSourceProof>, DatabaseError> {
        let encoded = payload(context, intent)?;
        let db = self.open(&self.automation_path())?;
        let saved: Option<String> = db.query_row("SELECT payload_json FROM plugin_sync_rebase_intents WHERE origin=?1 AND account_id=?2 AND plugin_id=?3",
            params![context.origin, context.account_id, intent.key()], |row| row.get(0)).optional()?;
        if saved.as_deref() != Some(encoded.as_str())
            || source_receipt(&db, context, intent, epoch)?.is_none()
        {
            return Ok(None);
        }
        Ok(V1ConflictSourceProof::new(
            intent.source_generation(),
            epoch.to_owned(),
        ))
    }

    /// After the coordinator has saved the exact frozen JSON result, replace
    /// or remove its receipt and clear the matching journal atomically.
    pub(crate) fn finish_plugin_sync_rebase(
        &self,
        context: &V1LocalEditContext,
        intent: &V1KeepLocalIntent,
        epoch: &str,
        proof: &V1ConflictSourceProof,
    ) -> Result<bool, DatabaseError> {
        let encoded = payload(context, intent)?;
        if !valid_proof(intent, epoch, proof) {
            return Ok(false);
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let saved: Option<String> = tx.query_row("SELECT payload_json FROM plugin_sync_rebase_intents WHERE origin=?1 AND account_id=?2 AND plugin_id=?3",
            params![context.origin, context.account_id, intent.key()], |row| row.get(0)).optional()?;
        let Some(previous) = source_receipt(&tx, context, intent, epoch)? else {
            return Ok(false);
        };
        if saved.as_deref() != Some(encoded.as_str()) {
            return Ok(false);
        }
        let changed = if let Some(pending) = &intent.after().pending {
            let next = serde_json::to_string(pending)?;
            tx.execute("UPDATE plugin_sync_receipts SET pending_json=?1 WHERE origin=?2 AND account_id=?3 AND plugin_id=?4 AND pending_json=?5",
                params![next, context.origin, context.account_id, intent.key(), previous])?
        } else {
            tx.execute("DELETE FROM plugin_sync_receipts WHERE origin=?1 AND account_id=?2 AND plugin_id=?3 AND pending_json=?4",
                params![context.origin, context.account_id, intent.key(), previous])?
        };
        if changed != 1 {
            return Err(invalid());
        }
        let removed = tx.execute("DELETE FROM plugin_sync_rebase_intents WHERE origin=?1 AND account_id=?2 AND plugin_id=?3 AND payload_json=?4",
            params![context.origin, context.account_id, intent.key(), encoded])?;
        if removed != 1 {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sqlite::plugin_sync::LocalPluginSyncIntent;
    use crate::paths::AppPaths;
    use tiktools_cloud_sync::{PluginSelection, V1Entry, V1Record};

    const ORIGIN: &str = "https://cloud.example";
    const KEY: &str = "acme.plugin";

    struct Fixture {
        db: DatabaseManager,
        root: std::path::PathBuf,
        context: V1LocalEditContext,
        epoch: String,
        pending: V1Pending,
        intent: V1KeepLocalIntent,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "tt-sync-rebase-{}-{}",
                std::process::id(),
                fastrand::u64(..)
            ));
            let paths = AppPaths {
                data: root.join("data"),
                plugins: root.join("plugins"),
                plugin_data: root.join("plugin-data"),
                builtin_plugins: root.join("absent"),
                development_plugins: None,
                logs: root.join("logs"),
                temp: root.join("temp"),
                root: root.clone(),
            };
            paths.ensure_directories().unwrap();
            for file in [
                paths.points_database(),
                paths.automation_database(),
                paths.analytics_database(),
            ] {
                drop(Connection::open(file).unwrap());
            }
            let db = DatabaseManager::new(paths);
            let context = V1LocalEditContext {
                origin: ORIGIN.into(),
                account_id: "alice".into(),
                config_generation: 7,
            };
            let epoch = "a".repeat(64);
            let selection = PluginSelection {
                schema_version: 1,
                plugin_id: KEY.into(),
                version: "1.0.0".into(),
                enabled: false,
            };
            let pending = V1Pending {
                base_revision: Some(1),
                edit: V1Edit::Put(selection.clone()),
                conflict: false,
            };
            assert_eq!(
                db.set_plugin_state_with_sync_intent(
                    KEY,
                    true,
                    false,
                    Some(LocalPluginSyncIntent {
                        origin: ORIGIN,
                        account_id: "alice",
                        config_generation: 7,
                        version: Some("1.0.0"),
                    })
                )
                .unwrap(),
                1
            );
            let candidate = db
                .next_plugin_sync_intent(ORIGIN, "alice")
                .unwrap()
                .unwrap();
            assert!(db.confirm_plugin_sync_runtime(&candidate, &epoch).unwrap());
            let candidate = db
                .next_plugin_sync_intent(ORIGIN, "alice")
                .unwrap()
                .unwrap();
            assert!(db
                .receipt_plugin_sync_intent(&candidate, &epoch, &pending)
                .unwrap());
            let before = V1Entry {
                remote: Some(V1Record {
                    collection: "plugin-selections".into(),
                    key: KEY.into(),
                    revision: 1,
                    deleted: false,
                    value: Some(selection.clone()),
                }),
                pending: Some(V1Pending {
                    conflict: true,
                    ..pending.clone()
                }),
            };
            let cloud = V1Record {
                collection: "plugin-selections".into(),
                key: KEY.into(),
                revision: 2,
                deleted: false,
                value: Some(selection),
            };
            let after = V1Entry {
                remote: Some(cloud.clone()),
                pending: Some(V1Pending {
                    base_revision: Some(2),
                    ..pending.clone()
                }),
            };
            let intent = serde_json::from_value(serde_json::json!({
                "formatVersion": 1, "origin": ORIGIN, "accountId": "alice", "configGeneration": 7,
                "sourceGeneration": 1, "initialRuntimeEpoch": epoch, "key": KEY, "beforeGeneration": 4,
                "before": before, "cloud": cloud, "after": after,
                "beforeDigest": "b".repeat(64), "afterDigest": "c".repeat(64),
            })).unwrap();
            Self {
                db,
                root,
                context,
                epoch,
                pending,
                intent,
            }
        }
        fn proof(&self) -> V1ConflictSourceProof {
            V1ConflictSourceProof::new(1, self.epoch.clone()).unwrap()
        }
        fn prepare(&self) {
            assert!(self
                .db
                .prepare_plugin_sync_rebase(&self.context, &self.intent, &self.epoch, &self.proof())
                .unwrap());
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn receipt_and_journal_update_are_atomic_across_failure_and_restart() {
        let f = Fixture::new();
        f.prepare();
        // Duplicate preparation is idempotent and reopening does not lose the intent.
        f.prepare();
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert_eq!(
            reopened.load_plugin_sync_rebase(&f.context, KEY).unwrap(),
            Some(f.intent.clone())
        );
        let db = reopened.open(&reopened.automation_path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_rebase_clear BEFORE DELETE ON plugin_sync_rebase_intents BEGIN SELECT RAISE(FAIL,'injected failure'); END;").unwrap();
        drop(db);
        assert!(reopened
            .finish_plugin_sync_rebase(&f.context, &f.intent, &f.epoch, &f.proof())
            .is_err());
        assert!(reopened.has_plugin_sync_rebase(&f.context).unwrap());
        assert!(reopened
            .validate_plugin_sync_receipt(ORIGIN, "alice", KEY, 7, &f.epoch, &f.pending)
            .unwrap());
        let db = reopened.open(&reopened.automation_path()).unwrap();
        db.execute_batch("DROP TRIGGER fail_rebase_clear;").unwrap();
        drop(db);
        assert!(reopened
            .finish_plugin_sync_rebase(&f.context, &f.intent, &f.epoch, &f.proof())
            .unwrap());
        assert!(!reopened.has_plugin_sync_rebase(&f.context).unwrap());
        assert!(!reopened
            .validate_plugin_sync_receipt(ORIGIN, "alice", KEY, 7, &f.epoch, &f.pending)
            .unwrap());
        assert!(reopened
            .validate_plugin_sync_receipt(
                ORIGIN,
                "alice",
                KEY,
                7,
                &f.epoch,
                f.intent.after().pending.as_ref().unwrap()
            )
            .unwrap());
    }

    #[test]
    fn journal_is_scoped_to_account_and_consent_without_erasing_stale_work() {
        let f = Fixture::new();
        f.prepare();
        let other = V1LocalEditContext {
            account_id: "bob".into(),
            ..f.context.clone()
        };
        assert!(!f.db.has_plugin_sync_rebase(&other).unwrap());
        assert_eq!(f.db.load_plugin_sync_rebase(&other, KEY).unwrap(), None);
        let changed_consent = V1LocalEditContext {
            config_generation: 8,
            ..f.context.clone()
        };
        assert!(f.db.has_plugin_sync_rebase(&changed_consent).unwrap());
        assert!(f.db.load_plugin_sync_rebase(&changed_consent, KEY).is_err());
        assert!(f
            .db
            .load_plugin_sync_rebase(&f.context, "another.plugin")
            .is_err());
        assert_eq!(
            f.db.load_plugin_sync_rebase(&f.context, KEY).unwrap(),
            Some(f.intent.clone())
        );
    }

    #[test]
    fn newer_source_even_from_another_account_blocks_rebase() {
        let f = Fixture::new();
        f.prepare();
        assert_eq!(
            f.db.set_plugin_state_with_sync_intent(
                KEY,
                true,
                false,
                Some(LocalPluginSyncIntent {
                    origin: ORIGIN,
                    account_id: "bob",
                    config_generation: 7,
                    version: Some("1.0.0"),
                })
            )
            .unwrap(),
            2
        );
        assert!(f
            .db
            .plugin_sync_rebase_source(&f.context, &f.intent, &f.epoch)
            .unwrap()
            .is_none());
        assert!(!f
            .db
            .finish_plugin_sync_rebase(&f.context, &f.intent, &f.epoch, &f.proof())
            .unwrap());
        assert!(f.db.has_plugin_sync_rebase(&f.context).unwrap());
        assert!(f
            .db
            .next_plugin_sync_intent(ORIGIN, "bob")
            .unwrap()
            .is_some());
    }

    #[test]
    fn restart_requires_exact_receipt_reconfirmation_for_new_epoch() {
        let f = Fixture::new();
        f.prepare();
        let reopened = DatabaseManager::new(f.db.paths().clone());
        let epoch = "d".repeat(64);
        assert!(reopened
            .plugin_sync_rebase_source(&f.context, &f.intent, &epoch)
            .unwrap()
            .is_none());
        let mut different = f.pending.clone();
        different.base_revision = Some(2);
        assert!(!reopened
            .confirm_plugin_sync_receipt_runtime(ORIGIN, "alice", KEY, 7, &epoch, &different)
            .unwrap());
        assert!(reopened
            .confirm_plugin_sync_receipt_runtime(ORIGIN, "alice", KEY, 7, &epoch, &f.pending)
            .unwrap());
        let proof = reopened
            .plugin_sync_rebase_source(&f.context, &f.intent, &epoch)
            .unwrap()
            .unwrap();
        assert!(!reopened
            .finish_plugin_sync_rebase(&f.context, &f.intent, &epoch, &f.proof())
            .unwrap());
        assert!(reopened
            .finish_plugin_sync_rebase(&f.context, &f.intent, &epoch, &proof)
            .unwrap());
        assert!(!reopened.has_plugin_sync_rebase(&f.context).unwrap());
    }

    #[test]
    fn abandon_requires_the_exact_newer_confirmed_candidate_and_keeps_both_edits() {
        let f = Fixture::new();
        f.prepare();
        let save = |account: &str, enabled: bool| {
            f.db.set_plugin_state_with_sync_intent(
                KEY,
                true,
                enabled,
                Some(LocalPluginSyncIntent {
                    origin: ORIGIN,
                    account_id: account,
                    config_generation: 7,
                    version: Some("1.0.0"),
                }),
            )
            .unwrap()
        };
        assert_eq!(save("bob", false), 2);
        assert!(f
            .db
            .newer_plugin_sync_rebase_candidate(&f.context, &f.intent, &f.epoch)
            .unwrap()
            .is_none());
        assert_eq!(save("alice", false), 3);
        assert!(f
            .db
            .newer_plugin_sync_rebase_candidate(&f.context, &f.intent, &f.epoch)
            .unwrap()
            .is_none());
        let confirmed =
            f.db.next_plugin_sync_intent(ORIGIN, "alice")
                .unwrap()
                .unwrap();
        assert!(f
            .db
            .confirm_plugin_sync_runtime(&confirmed, &f.epoch)
            .unwrap());
        let (candidate, proof) =
            f.db.newer_plugin_sync_rebase_candidate(&f.context, &f.intent, &f.epoch)
                .unwrap()
                .unwrap();
        assert_eq!(candidate.generation, 3);
        assert!(f
            .db
            .newer_plugin_sync_rebase_candidate(&f.context, &f.intent, &"d".repeat(64))
            .unwrap()
            .is_none());
        assert_eq!(save("alice", true), 4);
        assert!(!f
            .db
            .abandon_plugin_sync_rebase(&f.context, &f.intent, &f.epoch, &candidate, &proof)
            .unwrap());
        assert!(f.db.has_plugin_sync_rebase(&f.context).unwrap());
        let confirmed =
            f.db.next_plugin_sync_intent(ORIGIN, "alice")
                .unwrap()
                .unwrap();
        assert!(f
            .db
            .confirm_plugin_sync_runtime(&confirmed, &f.epoch)
            .unwrap());
        let (candidate, proof) =
            f.db.newer_plugin_sync_rebase_candidate(&f.context, &f.intent, &f.epoch)
                .unwrap()
                .unwrap();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_abandon BEFORE DELETE ON plugin_sync_rebase_intents BEGIN SELECT RAISE(FAIL,'injected failure'); END;").unwrap();
        drop(db);
        assert!(f
            .db
            .abandon_plugin_sync_rebase(&f.context, &f.intent, &f.epoch, &candidate, &proof)
            .is_err());
        assert!(f.db.has_plugin_sync_rebase(&f.context).unwrap());
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("DROP TRIGGER fail_abandon;").unwrap();
        drop(db);
        assert!(f
            .db
            .abandon_plugin_sync_rebase(&f.context, &f.intent, &f.epoch, &candidate, &proof)
            .unwrap());
        assert!(!f.db.has_plugin_sync_rebase(&f.context).unwrap());
        assert_eq!(
            f.db.next_plugin_sync_intent(ORIGIN, "alice")
                .unwrap()
                .unwrap()
                .generation,
            4
        );
        let db = f.db.open(&f.db.automation_path()).unwrap();
        let old_receipt: String = db.query_row("SELECT pending_json FROM plugin_sync_receipts WHERE origin=?1 AND account_id='alice' AND plugin_id=?2", params![ORIGIN, KEY], |row| row.get(0)).unwrap();
        assert_eq!(
            serde_json::from_str::<V1Pending>(&old_receipt).unwrap(),
            f.pending
        );
    }

    #[test]
    fn superseded_abandon_keeps_other_account_and_local_only_edits_private() {
        let f = Fixture::new();
        f.prepare();
        assert_eq!(
            f.db.superseded_plugin_sync_rebase_generation(&f.context, &f.intent)
                .unwrap(),
            None
        );
        assert_eq!(
            f.db.set_plugin_state_with_sync_intent(
                KEY,
                true,
                false,
                Some(LocalPluginSyncIntent {
                    origin: ORIGIN,
                    account_id: "bob",
                    config_generation: 7,
                    version: Some("1.0.0"),
                })
            )
            .unwrap(),
            2
        );
        assert_eq!(
            f.db.superseded_plugin_sync_rebase_generation(&f.context, &f.intent)
                .unwrap(),
            Some(2)
        );
        let foreign = V1LocalEditContext {
            account_id: "bob".into(),
            ..f.context.clone()
        };
        assert!(f
            .db
            .superseded_plugin_sync_rebase_generation(&foreign, &f.intent)
            .is_err());
        assert_eq!(
            f.db.set_plugin_state_with_sync_intent(KEY, true, true, None)
                .unwrap(),
            3
        );
        assert!(!f
            .db
            .abandon_superseded_plugin_sync_rebase(&f.context, &f.intent, 2)
            .unwrap());
        assert!(f.db.has_plugin_sync_rebase(&f.context).unwrap());
        assert_eq!(
            f.db.superseded_plugin_sync_rebase_generation(&f.context, &f.intent)
                .unwrap(),
            Some(3)
        );
        assert!(f
            .db
            .abandon_superseded_plugin_sync_rebase(&f.context, &f.intent, 3)
            .unwrap());
        assert!(!f.db.has_plugin_sync_rebase(&f.context).unwrap());
        let bob =
            f.db.next_plugin_sync_intent(ORIGIN, "bob")
                .unwrap()
                .unwrap();
        assert_eq!((bob.generation, bob.source_generation), (2, 3));
        assert!(!f
            .db
            .validate_plugin_sync_receipt(ORIGIN, "alice", KEY, 7, &f.epoch, &f.pending)
            .unwrap());
        let db = f.db.open(&f.db.automation_path()).unwrap();
        let old_receipt: String = db.query_row("SELECT pending_json FROM plugin_sync_receipts WHERE origin=?1 AND account_id='alice' AND plugin_id=?2", params![ORIGIN, KEY], |row| row.get(0)).unwrap();
        assert_eq!(
            serde_json::from_str::<V1Pending>(&old_receipt).unwrap(),
            f.pending
        );
    }
}
