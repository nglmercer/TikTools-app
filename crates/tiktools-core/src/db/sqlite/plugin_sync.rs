//! Durable plugin source revisions and account-partitioned local sync intents.
//!
//! These methods provide only the storage transaction. Their caller must hold
//! the protocol and identity/opt-in lease, drain before any network sync, and
//! refuse stale source generations. Stored identity is not authorization.

use rusqlite::{params, OptionalExtension, TransactionBehavior};

use super::{
    behavior::advance_plugin_source,
    util::{bool_int, now},
    DatabaseError, DatabaseManager,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalPluginSyncIntent<'a> {
    pub origin: &'a str,
    pub account_id: &'a str,
    pub config_generation: u128,
    /// None is a tombstone; a selection contains the installed version.
    pub version: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginSyncOutboxEntry {
    pub origin: String,
    pub account_id: String,
    pub plugin_id: String,
    pub generation: i64,
    pub source_generation: i64,
    pub config_generation: u128,
    /// Confirmation is valid only for this particular native host lifetime.
    pub runtime_epoch: Option<String>,
    pub reviewed_base: Option<String>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
}

/// Exact local-only source observation. It carries no Cloud authorization;
/// callers must bind it to explicit consent, package identity and runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UntrackedPluginReview {
    pub plugin_id: String,
    pub generation: i64,
    pub installed: bool,
    pub enabled: bool,
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn valid_runtime_epoch(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

// A package's files and runtime can be between states while this journal is
// active. Treat an invalid NULL identity as global uncertainty, not absence.
fn package_source_available(db: &rusqlite::Connection, id: &str) -> Result<bool, DatabaseError> {
    Ok(!db.query_row(
        "SELECT EXISTS(SELECT 1 FROM plugin_package_mutation_intents WHERE plugin_id=?1 OR plugin_id IS NULL)",
        [id], |row| row.get::<_, bool>(0),
    )?)
}

impl DatabaseManager {
    /// Hold a nonblocking SQLite write reservation while a v2 upload is
    /// admitted. All source mutations must participate in the source journal;
    /// this does not fence package files or older/noncooperating processes.
    pub(crate) fn with_current_v2_upload_source<R, E: From<DatabaseError>>(
        &self,
        plugin_id: &str,
        generation: u64,
        enabled: bool,
        operation: impl FnOnce() -> Result<R, E>,
    ) -> Result<R, E> {
        let generation = i64::try_from(generation)
            .ok()
            .filter(|v| *v > 0)
            .ok_or_else(|| E::from(DatabaseError::Invalid("invalid v2 upload source".into())))?;
        let mut db = self.open(&self.automation_path()).map_err(E::from)?;
        db.busy_timeout(std::time::Duration::ZERO)
            .map_err(DatabaseError::from)
            .map_err(E::from)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(DatabaseError::from)
            .map_err(E::from)?;
        let valid: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
             WHERE s.plugin_id=?1 AND s.generation=?2 AND b.installed=1 AND b.enabled=?3
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_apply_intents a WHERE a.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_apply_finalizations f WHERE f.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=s.plugin_id))",
            params![plugin_id, generation, bool_int(enabled)], |row| row.get(0),
        ).map_err(DatabaseError::from).map_err(E::from)?;
        if !valid {
            return Err(E::from(DatabaseError::Invalid(
                "v2 upload source needs review".into(),
            )));
        }
        let result = operation();
        tx.rollback()
            .map_err(DatabaseError::from)
            .map_err(E::from)?;
        result
    }
    /// Record the source state and its local Cloud candidate atomically.
    /// Passing no intent records a local-only edit while still advancing the
    /// source generation; this deliberately leaves older intents stale.
    pub(crate) fn set_plugin_state_with_sync_intent(
        &self,
        id: &str,
        installed: bool,
        enabled: bool,
        intent: Option<LocalPluginSyncIntent<'_>>,
    ) -> Result<i64, DatabaseError> {
        self.set_plugin_state_with_sync_intent_inner(id, installed, enabled, intent, false)
    }

    /// Atomically mark a source changed without account/consent admission.
    /// Caller must hold the exclusive product-emission lease. The marker is
    /// never cleared by a later edit and requires explicit review.
    pub(crate) fn set_plugin_state_with_untracked_sync(
        &self,
        id: &str,
        installed: bool,
        enabled: bool,
    ) -> Result<i64, DatabaseError> {
        self.set_plugin_state_with_sync_intent_inner(id, installed, enabled, None, true)
    }

    pub(crate) fn has_untracked_plugin_sync(&self) -> Result<bool, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        Ok(db.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_untracked_local)",
            [],
            |row| row.get(0),
        )?)
    }

    /// Read one local marker without inferring an account or consent. A stale
    /// marker is rejected rather than silently attributed to a newer source.
    #[allow(dead_code)]
    pub(crate) fn review_untracked_plugin(
        &self,
        id: &str,
    ) -> Result<Option<UntrackedPluginReview>, DatabaseError> {
        if !valid_text(id, 128) {
            return Err(DatabaseError::Invalid("invalid plugin sync source".into()));
        }
        let db = self.open(&self.automation_path())?;
        let row: Option<(i64, i64, i64, i64)> = db.query_row(
            "SELECT u.generation, s.generation, b.installed, b.enabled FROM plugin_sync_untracked_local u JOIN plugin_sync_source_versions s ON s.plugin_id=u.plugin_id JOIN behavior_plugins b ON b.id=u.plugin_id WHERE u.plugin_id=?1",
            [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).optional()?;
        match row {
            Some((marker, current, installed, enabled))
                if marker > 0
                    && marker == current
                    && matches!(installed, 0 | 1)
                    && matches!(enabled, 0 | 1) =>
            {
                Ok(Some(UntrackedPluginReview {
                    plugin_id: id.into(),
                    generation: marker,
                    installed: installed == 1,
                    enabled: enabled == 1,
                }))
            }
            Some(_) => Err(DatabaseError::Invalid(
                "untracked plugin source needs review".into(),
            )),
            None => {
                let exists: bool = db.query_row(
                    "SELECT EXISTS(SELECT 1 FROM plugin_sync_untracked_local WHERE plugin_id=?1)",
                    [id],
                    |r| r.get(0),
                )?;
                if exists {
                    Err(DatabaseError::Invalid(
                        "untracked plugin source needs review".into(),
                    ))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Atomically transfer one explicitly reviewed marker into an unconfirmed
    /// account outbox entry. Caller must hold the exclusive emission lease,
    /// exact identity/consent and runtime/package fences, and verify `version`
    /// against the reviewed installed artifact. Tombstones require separate
    /// package reconciliation and are deliberately rejected here.
    #[allow(dead_code)]
    pub(crate) fn reconcile_untracked_plugin(
        &self,
        review: &UntrackedPluginReview,
        intent: &LocalPluginSyncIntent<'_>,
        reviewed_base: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let version = intent
            .version
            .filter(|v| valid_text(v, 128))
            .ok_or_else(|| DatabaseError::Invalid("reviewed installed version required".into()))?;
        if !review.installed
            || review.generation <= 0
            || intent.config_generation == 0
            || !valid_text(&review.plugin_id, 128)
            || !valid_text(intent.origin, 2048)
            || !valid_text(intent.account_id, 1024)
        {
            return Err(DatabaseError::Invalid(
                "invalid untracked plugin review".into(),
            ));
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let valid: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u JOIN plugin_sync_source_versions s ON s.plugin_id=u.plugin_id JOIN behavior_plugins b ON b.id=u.plugin_id WHERE u.plugin_id=?1 AND u.generation=?2 AND s.generation=?2 AND b.installed=1 AND b.enabled=?3 AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=u.plugin_id) AND NOT EXISTS(SELECT 1 FROM plugin_sync_receipts q WHERE q.plugin_id=u.plugin_id) AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=u.plugin_id OR p.plugin_id IS NULL) AND NOT EXISTS(SELECT 1 FROM plugin_sync_apply_intents a WHERE a.plugin_id=u.plugin_id) AND NOT EXISTS(SELECT 1 FROM plugin_sync_apply_finalizations f WHERE f.plugin_id=u.plugin_id) AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=u.plugin_id))",
            params![review.plugin_id, review.generation, bool_int(review.enabled)], |r| r.get(0),
        )?;
        if !valid {
            return Err(DatabaseError::Invalid(
                "untracked plugin review is stale or blocked".into(),
            ));
        }
        tx.execute("INSERT INTO plugin_sync_outbox(origin,account_id,plugin_id,generation,config_generation,version,enabled,runtime_epoch,reviewed_base) VALUES (?1,?2,?3,?4,?5,?6,?7,NULL,?8)", params![intent.origin, intent.account_id, review.plugin_id, review.generation, intent.config_generation.to_string(), version, bool_int(review.enabled), reviewed_base])?;
        if tx.execute(
            "DELETE FROM plugin_sync_untracked_local WHERE plugin_id=?1 AND generation=?2",
            params![review.plugin_id, review.generation],
        )? != 1
        {
            return Err(DatabaseError::Invalid(
                "untracked plugin review changed".into(),
            ));
        }
        tx.commit()?;
        Ok(())
    }

    fn set_plugin_state_with_sync_intent_inner(
        &self,
        id: &str,
        installed: bool,
        enabled: bool,
        intent: Option<LocalPluginSyncIntent<'_>>,
        untracked: bool,
    ) -> Result<i64, DatabaseError> {
        if !valid_text(id, 128)
            || intent.as_ref().is_some_and(|candidate| {
                !valid_text(candidate.origin, 2048)
                    || !valid_text(candidate.account_id, 1024)
                    || candidate.version.is_some_and(|v| !valid_text(v, 128))
                    || installed != candidate.version.is_some()
            })
        {
            return Err(DatabaseError::Invalid("invalid plugin sync source".into()));
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !package_source_available(&tx, id)? {
            return Err(DatabaseError::Invalid(
                "plugin package operation needs review".into(),
            ));
        }
        let generation = advance_plugin_source(&tx, id)?;
        tx.execute(
            "INSERT INTO behavior_plugins(id,installed,enabled,updated_at) VALUES (?1,?2,?3,?4)
             ON CONFLICT(id) DO UPDATE SET installed=excluded.installed,
                enabled=excluded.enabled,updated_at=excluded.updated_at",
            params![id, bool_int(installed), bool_int(enabled), now()],
        )?;
        if untracked {
            tx.execute(
                "INSERT INTO plugin_sync_untracked_local(plugin_id,generation) VALUES (?1,?2)
                 ON CONFLICT(plugin_id) DO UPDATE SET generation=excluded.generation",
                params![id, generation],
            )?;
        }
        if let Some(candidate) = intent {
            let sync_enabled = candidate.version.map(|_| bool_int(enabled));
            let written = tx.execute(
                "INSERT INTO plugin_sync_outbox(origin,account_id,plugin_id,generation,config_generation,version,enabled,reviewed_base)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,'direct')
                 ON CONFLICT(origin,account_id,plugin_id) DO UPDATE SET
                    generation=excluded.generation,config_generation=excluded.config_generation,
                    runtime_epoch=NULL,reviewed_base='direct',version=excluded.version,enabled=excluded.enabled
                 WHERE plugin_sync_outbox.config_generation IS NOT NULL",
                params![candidate.origin, candidate.account_id, id, generation, candidate.config_generation.to_string(), candidate.version, sync_enabled],
            )?;
            if written != 1 {
                return Err(DatabaseError::Invalid(
                    "plugin sync consent needs review".into(),
                ));
            }
        }
        tx.commit()?;
        Ok(generation)
    }

    /// Read one identity's candidate together with the current source revision.
    /// The caller must reject a mismatch and seek fresh review; it must not
    /// silently publish an earlier account's stale local state.
    pub(crate) fn next_plugin_sync_intent(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<Option<PluginSyncOutboxEntry>, DatabaseError> {
        self.find_plugin_sync_intent(origin, account, None)
    }

    pub(crate) fn plugin_sync_intent_for(
        &self,
        origin: &str,
        account: &str,
        plugin_id: &str,
    ) -> Result<Option<PluginSyncOutboxEntry>, DatabaseError> {
        if !valid_text(plugin_id, 128) {
            return Err(DatabaseError::Invalid("invalid plugin sync source".into()));
        }
        self.find_plugin_sync_intent(origin, account, Some(plugin_id))
    }

    fn find_plugin_sync_intent(
        &self,
        origin: &str,
        account: &str,
        plugin_id: Option<&str>,
    ) -> Result<Option<PluginSyncOutboxEntry>, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        let entry = db.query_row(
            "SELECT o.origin,o.account_id,o.plugin_id,o.generation,s.generation,o.config_generation,o.version,o.enabled,o.runtime_epoch,o.reviewed_base
             FROM plugin_sync_outbox o LEFT JOIN plugin_sync_source_versions s ON s.plugin_id=o.plugin_id
             WHERE o.origin=?1 AND o.account_id=?2 AND (?3 IS NULL OR o.plugin_id=?3) ORDER BY o.plugin_id COLLATE BINARY LIMIT 1",
            params![origin, account, plugin_id], |row| {
                let raw: Option<i64> = row.get(7)?;
                let source: Option<i64> = row.get(4)?;
                let config: Option<String> = row.get(5)?;
                Ok((PluginSyncOutboxEntry {
                    origin: row.get(0)?, account_id: row.get(1)?, plugin_id: row.get(2)?,
                    generation: row.get(3)?, source_generation: source.unwrap_or(0),
                    config_generation: config.as_deref().and_then(|s| s.parse::<u128>().ok()).unwrap_or_default(),
                    version: row.get(6)?, enabled: raw.map(|value| value != 0), runtime_epoch: row.get(8)?, reviewed_base: row.get(9)?,
                }, raw, config))
            },
        ).optional()?;
        match entry {
            Some((entry, raw, config))
                if entry.generation <= 0
                    || entry.source_generation <= 0
                    || !valid_text(&entry.origin, 2048)
                    || !valid_text(&entry.account_id, 1024)
                    || !valid_text(&entry.plugin_id, 128)
                    || entry.version.as_ref().is_some_and(|v| !valid_text(v, 128))
                    || entry.version.is_some() != entry.enabled.is_some()
                    || entry
                        .runtime_epoch
                        .as_deref()
                        .is_some_and(|value| !valid_runtime_epoch(value))
                    || config.as_deref() != Some(entry.config_generation.to_string().as_str())
                    || raw.is_some_and(|value| value != 0 && value != 1) =>
            {
                Err(DatabaseError::Invalid("invalid plugin sync outbox".into()))
            }
            Some((entry, _, _)) => {
                if !package_source_available(&db, &entry.plugin_id)? {
                    return Err(DatabaseError::Invalid(
                        "plugin package operation needs review".into(),
                    ));
                }
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Confirm only after this host has reconciled the desired plugin runtime
    /// with the persisted source. Epochs must be unpredictable, native-only,
    /// unique per host lifetime and shared by every drain in that lifetime.
    /// Restart or a second host requires a fresh observation and confirmation.
    pub(crate) fn confirm_plugin_sync_runtime(
        &self,
        entry: &PluginSyncOutboxEntry,
        runtime_epoch: &str,
    ) -> Result<bool, DatabaseError> {
        if !valid_runtime_epoch(runtime_epoch) {
            return Err(DatabaseError::Invalid(
                "invalid plugin runtime epoch".into(),
            ));
        }
        let db = self.open(&self.automation_path())?;
        let changed = db.execute(
            "UPDATE plugin_sync_outbox SET runtime_epoch=?6 WHERE origin=?1 AND account_id=?2 AND plugin_id=?3
             AND generation=?4 AND config_generation=?5 AND version IS ?7 AND enabled IS ?8 AND reviewed_base IS ?9
             AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s WHERE s.plugin_id=?3 AND s.generation=?4)
             AND EXISTS(SELECT 1 FROM behavior_plugins b WHERE b.id=?3
                 AND b.installed=CASE WHEN ?7 IS NULL THEN 0 ELSE 1 END
                 AND (?7 IS NULL OR b.enabled=?8))
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=?3 OR p.plugin_id IS NULL)",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation,
                entry.config_generation.to_string(), runtime_epoch, entry.version,
                entry.enabled.map(bool_int), entry.reviewed_base],
        )?;
        Ok(changed == 1)
    }

    /// Bind an older unclassified candidate to an explicit, post-barrier Cloud
    /// review. The caller owns the source, identity, consent and runtime gates.
    /// No old runtime confirmation survives the review.
    #[allow(dead_code)]
    pub(crate) fn review_legacy_plugin_sync_base(
        &self,
        entry: &PluginSyncOutboxEntry,
        current_config_generation: u128,
        reviewed_base: &str,
    ) -> Result<bool, DatabaseError> {
        let base: Option<cloud_client::WorkspaceRecord> = serde_json::from_str(reviewed_base)
            .map_err(|_| DatabaseError::Invalid("invalid reviewed Cloud base".into()))?;
        if entry.reviewed_base.is_some()
            || entry.generation <= 0
            || entry.config_generation == 0
            || current_config_generation == 0
            || entry.version.is_none()
            || entry.enabled.is_none()
            || reviewed_base.len() > 512 * 1024
            || base.as_ref().is_some_and(|r| {
                r.collection != "plugin-selections" || r.key != entry.plugin_id || r.revision <= 0
            })
        {
            return Err(DatabaseError::Invalid(
                "invalid legacy plugin review".into(),
            ));
        }
        let db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let changed = db.execute(
            "UPDATE plugin_sync_outbox SET reviewed_base=?9,config_generation=?10,runtime_epoch=NULL
             WHERE origin=?1 AND account_id=?2 AND plugin_id=?3 AND generation=?4
               AND config_generation=?5 AND version IS ?6 AND enabled IS ?7 AND runtime_epoch IS ?8
               AND reviewed_base IS NULL
               AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s WHERE s.plugin_id=?3 AND s.generation=?4)
               AND EXISTS(SELECT 1 FROM behavior_plugins b WHERE b.id=?3 AND b.installed=1 AND b.enabled=?7)
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=?3 AND (o.origin<>?1 OR o.account_id<>?2))
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_untracked_local u WHERE u.plugin_id=?3)
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_receipts q WHERE q.plugin_id=?3)
               AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=?3 OR p.plugin_id IS NULL)
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_apply_intents a WHERE a.plugin_id=?3)
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_apply_finalizations f WHERE f.plugin_id=?3)
               AND NOT EXISTS(SELECT 1 FROM plugin_sync_rebase_intents r WHERE r.plugin_id=?3)",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation,
                entry.config_generation.to_string(), entry.version, entry.enabled.map(bool_int),
                entry.runtime_epoch, reviewed_base, current_config_generation.to_string()],
        )?;
        Ok(changed == 1)
    }

    /// Only the exact generation and current host lifetime may be
    /// acknowledged after durable staging. A concurrent newer edit, a
    /// restart, or an unreconciled runtime is never removed.
    pub(crate) fn acknowledge_plugin_sync_intent(
        &self,
        entry: &PluginSyncOutboxEntry,
        runtime_epoch: &str,
    ) -> Result<bool, DatabaseError> {
        if !valid_runtime_epoch(runtime_epoch) {
            return Err(DatabaseError::Invalid(
                "invalid plugin runtime epoch".into(),
            ));
        }
        let db = self.open(&self.automation_path())?;
        let removed = db.execute(
            "DELETE FROM plugin_sync_outbox WHERE origin=?1 AND account_id=?2 AND plugin_id=?3
             AND generation=?4 AND config_generation=?5 AND runtime_epoch=?6
             AND version IS ?7 AND enabled IS ?8 AND reviewed_base IS ?9 AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s
                 WHERE s.plugin_id=?3 AND s.generation=?4)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=?3 OR p.plugin_id IS NULL)",
            params![
                entry.origin,
                entry.account_id,
                entry.plugin_id,
                entry.generation,
                entry.config_generation.to_string(),
                runtime_epoch,
                entry.version,
                entry.enabled.map(bool_int),
                entry.reviewed_base,
            ],
        )?;
        Ok(removed == 1)
    }

    /// Move the exact confirmed outbox entry to a durable receipt after its
    /// corresponding v1 pending value has been saved. The two SQLite writes
    /// share one transaction; interruption before commit retains the outbox.
    pub(crate) fn receipt_plugin_sync_intent(
        &self,
        entry: &PluginSyncOutboxEntry,
        runtime_epoch: &str,
        pending: &tiktools_cloud_sync::V1Pending,
    ) -> Result<bool, DatabaseError> {
        use tiktools_cloud_sync::{PluginSelection, V1Edit};

        if !valid_runtime_epoch(runtime_epoch)
            || entry.runtime_epoch.as_deref() != Some(runtime_epoch)
            || entry.source_generation != entry.generation
        {
            return Ok(false);
        }
        let expected = match (&entry.version, entry.enabled) {
            (Some(version), Some(enabled)) => V1Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: entry.plugin_id.clone(),
                version: version.clone(),
                enabled,
            }),
            (None, None) => V1Edit::Delete,
            _ => return Ok(false),
        };
        if pending.edit != expected {
            return Ok(false);
        }
        let pending_json = serde_json::to_string(pending)
            .map_err(|_| DatabaseError::Invalid("invalid staged sync value".into()))?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let matching: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_outbox o JOIN plugin_sync_source_versions s ON s.plugin_id=o.plugin_id
             WHERE o.origin=?1 AND o.account_id=?2 AND o.plugin_id=?3 AND o.generation=?4
             AND o.config_generation=?5 AND o.runtime_epoch=?6 AND o.version IS ?7 AND o.enabled IS ?8
             AND s.generation=o.generation
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=o.plugin_id OR p.plugin_id IS NULL))",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation,
                entry.config_generation.to_string(), runtime_epoch, entry.version, entry.enabled.map(bool_int)],
            |row| row.get(0),
        )?;
        if !matching {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO plugin_sync_receipts(origin,account_id,plugin_id,generation,config_generation,runtime_epoch,pending_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(origin,account_id,plugin_id) DO UPDATE SET generation=excluded.generation,
                 config_generation=excluded.config_generation,runtime_epoch=excluded.runtime_epoch,pending_json=excluded.pending_json",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation,
                entry.config_generation.to_string(), runtime_epoch, pending_json],
        )?;
        let removed = tx.execute(
            "DELETE FROM plugin_sync_outbox WHERE origin=?1 AND account_id=?2 AND plugin_id=?3 AND generation=?4",
            params![entry.origin, entry.account_id, entry.plugin_id, entry.generation],
        )?;
        if removed != 1 {
            return Err(DatabaseError::Invalid(
                "sync receipt transaction changed".into(),
            ));
        }
        tx.commit()?;
        Ok(true)
    }

    /// A receipt authorizes only the identical pending value for this exact
    /// identity, consent epoch and observed runtime, while the local source
    /// generation remains unchanged. Call under the admission lease.
    pub(crate) fn validate_plugin_sync_receipt(
        &self,
        origin: &str,
        account: &str,
        plugin_id: &str,
        config_generation: u128,
        runtime_epoch: &str,
        pending: &tiktools_cloud_sync::V1Pending,
    ) -> Result<bool, DatabaseError> {
        if !valid_runtime_epoch(runtime_epoch) {
            return Ok(false);
        }
        let db = self.open(&self.automation_path())?;
        let stored: Option<String> = db.query_row(
            "SELECT r.pending_json FROM plugin_sync_receipts r JOIN plugin_sync_source_versions s ON s.plugin_id=r.plugin_id
             WHERE r.origin=?1 AND r.account_id=?2 AND r.plugin_id=?3 AND r.config_generation=?4
             AND r.runtime_epoch=?5 AND r.generation=s.generation
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=r.plugin_id OR p.plugin_id IS NULL)",
            params![origin, account, plugin_id, config_generation.to_string(), runtime_epoch],
            |row| row.get(0),
        ).optional()?;
        Ok(stored
            .and_then(|value| serde_json::from_str::<tiktools_cloud_sync::V1Pending>(&value).ok())
            .as_ref()
            == Some(pending))
    }

    /// A 409 may change only the pending conflict flag after staging. Return
    /// the exact current source generation only if the saved receipt still
    /// matches the reviewed edit and base, with no undrained edit for this
    /// plugin in any account. Call under the shared product admission lease.
    pub(crate) fn plugin_sync_conflict_source(
        &self,
        origin: &str,
        account: &str,
        plugin_id: &str,
        config_generation: u128,
        runtime_epoch: &str,
        pending: &tiktools_cloud_sync::V1Pending,
    ) -> Result<Option<i64>, DatabaseError> {
        use tiktools_cloud_sync::V1Edit;

        if !pending.conflict || !valid_runtime_epoch(runtime_epoch) {
            return Ok(None);
        }
        let (installed, enabled) = match &pending.edit {
            V1Edit::Put(value) if value.plugin_id == plugin_id => (true, Some(value.enabled)),
            V1Edit::Delete => (false, None),
            _ => return Ok(None),
        };
        let db = self.open(&self.automation_path())?;
        let stored: Option<(i64, String)> = db.query_row(
            "SELECT r.generation,r.pending_json FROM plugin_sync_receipts r JOIN plugin_sync_source_versions s ON s.plugin_id=r.plugin_id
             JOIN behavior_plugins b ON b.id=r.plugin_id
             WHERE r.origin=?1 AND r.account_id=?2 AND r.plugin_id=?3 AND r.config_generation=?4
             AND r.runtime_epoch=?5 AND r.generation=s.generation
             AND b.installed=?6 AND (?7 IS NULL OR b.enabled=?7)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=r.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=r.plugin_id OR p.plugin_id IS NULL)",
            params![origin, account, plugin_id, config_generation.to_string(), runtime_epoch,
                bool_int(installed), enabled.map(bool_int)],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        let Some((generation, json)) = stored else {
            return Ok(None);
        };
        let Some(mut saved) = serde_json::from_str::<tiktools_cloud_sync::V1Pending>(&json).ok()
        else {
            return Ok(None);
        };
        saved.conflict = true;
        Ok((generation > 0 && saved == *pending).then_some(generation))
    }

    /// Reconfirm a retained receipt after a host restart, only after the
    /// caller has observed the exact desired runtime and package version under
    /// its mutation gate. This does not itself start, stop or install a plugin.
    pub(crate) fn confirm_plugin_sync_receipt_runtime(
        &self,
        origin: &str,
        account: &str,
        plugin_id: &str,
        config_generation: u128,
        runtime_epoch: &str,
        pending: &tiktools_cloud_sync::V1Pending,
    ) -> Result<bool, DatabaseError> {
        use tiktools_cloud_sync::V1Edit;

        if !valid_runtime_epoch(runtime_epoch) {
            return Ok(false);
        }
        let (installed, enabled) = match &pending.edit {
            V1Edit::Put(value) if value.plugin_id == plugin_id => (true, Some(value.enabled)),
            V1Edit::Delete => (false, None),
            _ => return Ok(false),
        };
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored: Option<String> = tx.query_row(
            "SELECT r.pending_json FROM plugin_sync_receipts r JOIN plugin_sync_source_versions s ON s.plugin_id=r.plugin_id
             JOIN behavior_plugins b ON b.id=r.plugin_id WHERE r.origin=?1 AND r.account_id=?2 AND r.plugin_id=?3
             AND r.config_generation=?4 AND r.generation=s.generation AND b.installed=?5
             AND (?6 IS NULL OR b.enabled=?6)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=r.plugin_id OR p.plugin_id IS NULL)",
            params![origin, account, plugin_id, config_generation.to_string(), bool_int(installed), enabled.map(bool_int)],
            |row| row.get(0),
        ).optional()?;
        let Some(stored) = stored else {
            return Ok(false);
        };
        if serde_json::from_str::<tiktools_cloud_sync::V1Pending>(&stored)
            .ok()
            .as_ref()
            != Some(pending)
        {
            return Ok(false);
        }
        let changed = tx.execute(
            "UPDATE plugin_sync_receipts SET runtime_epoch=?1 WHERE origin=?2 AND account_id=?3 AND plugin_id=?4
             AND config_generation=?5 AND pending_json=?6",
            params![runtime_epoch, origin, account, plugin_id, config_generation.to_string(), stored],
        )?;
        tx.commit()?;
        Ok(changed == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AppPaths;
    use rusqlite::Connection;

    struct Fixture {
        db: DatabaseManager,
        root: std::path::PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "tt-plugin-sync-{}-{}",
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
            Self {
                db: DatabaseManager::new(paths),
                root,
            }
        }
        fn intent(&self, account: &'static str) -> LocalPluginSyncIntent<'static> {
            LocalPluginSyncIntent {
                origin: "https://cloud.example",
                account_id: account,
                config_generation: 7,
                version: Some("1.0.0"),
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn legacy_base_review_preserves_candidate_until_exact_atomic_commit() {
        let f = Fixture::new();
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
            .unwrap();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute("UPDATE plugin_sync_outbox SET reviewed_base=NULL,runtime_epoch=?1 WHERE plugin_id='hotkeys'", ["a".repeat(64)]).unwrap();
        let old =
            f.db.plugin_sync_intent_for("https://cloud.example", "alice", "hotkeys")
                .unwrap()
                .unwrap();
        let mut stale = old.clone();
        stale.generation += 1;
        assert!(!f
            .db
            .review_legacy_plugin_sync_base(&stale, 8, "null")
            .unwrap());
        assert!(f
            .db
            .review_legacy_plugin_sync_base(&old, 8, "not-json")
            .is_err());
        let wrong_target = serde_json::json!({"collection":"plugin-selections","key":"other","revision":1,"deleted":true,"value":null}).to_string();
        assert!(f
            .db
            .review_legacy_plugin_sync_base(&old, 8, &wrong_target)
            .is_err());
        db.execute("INSERT INTO plugin_sync_outbox(origin,account_id,plugin_id,generation,config_generation,version,enabled) VALUES ('https://cloud.example','bob','hotkeys',?1,'7','1.0.0',1)", [old.generation]).unwrap();
        assert!(!f
            .db
            .review_legacy_plugin_sync_base(&old, 8, "null")
            .unwrap());
        db.execute("DELETE FROM plugin_sync_outbox WHERE account_id='bob'", [])
            .unwrap();
        db.execute_batch("CREATE TRIGGER reject_legacy_review BEFORE UPDATE ON plugin_sync_outbox BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(f
            .db
            .review_legacy_plugin_sync_base(&old, 8, "null")
            .is_err());
        assert_eq!(
            f.db.plugin_sync_intent_for("https://cloud.example", "alice", "hotkeys")
                .unwrap()
                .unwrap(),
            old
        );
        db.execute_batch("DROP TRIGGER reject_legacy_review")
            .unwrap();
        db.execute(
            "INSERT INTO plugin_sync_untracked_local(plugin_id,generation) VALUES ('hotkeys',?1)",
            [old.generation],
        )
        .unwrap();
        assert!(!f
            .db
            .review_legacy_plugin_sync_base(&old, 8, "null")
            .unwrap());
        db.execute(
            "DELETE FROM plugin_sync_untracked_local WHERE plugin_id='hotkeys'",
            [],
        )
        .unwrap();
        let base = serde_json::json!({"collection":"plugin-selections","key":"hotkeys","revision":4,"deleted":false,"value":{"schemaVersion":1,"pluginId":"hotkeys","version":"1.0.0","enabled":false}}).to_string();
        assert!(f.db.review_legacy_plugin_sync_base(&old, 8, &base).unwrap());
        let current =
            f.db.plugin_sync_intent_for("https://cloud.example", "alice", "hotkeys")
                .unwrap()
                .unwrap();
        assert_eq!(current.reviewed_base.as_deref(), Some(base.as_str()));
        assert_eq!(current.config_generation, 8);
        assert_eq!(current.runtime_epoch, None);
        assert!(!f
            .db
            .review_legacy_plugin_sync_base(&old, 8, "null")
            .unwrap());
    }

    #[test]
    fn reviewed_untracked_transfer_is_atomic_and_rejects_stale_source() {
        let f = Fixture::new();
        f.db.set_plugin_state_with_untracked_sync("hotkeys", true, false)
            .unwrap();
        let old = f.db.review_untracked_plugin("hotkeys").unwrap().unwrap();
        f.db.set_plugin_state_with_untracked_sync("hotkeys", true, true)
            .unwrap();
        assert!(f
            .db
            .reconcile_untracked_plugin(&old, &f.intent("alice"), None)
            .is_err());
        assert!(f.db.has_untracked_plugin_sync().unwrap());
        assert!(f
            .db
            .next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .is_none());
        let review = f.db.review_untracked_plugin("hotkeys").unwrap().unwrap();
        assert_ne!(review.generation, old.generation);
        assert!(review.enabled);
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("CREATE TRIGGER reject_marker_delete BEFORE DELETE ON plugin_sync_untracked_local BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(f
            .db
            .reconcile_untracked_plugin(&review, &f.intent("alice"), None)
            .is_err());
        assert!(f.db.has_untracked_plugin_sync().unwrap());
        assert!(f
            .db
            .next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .is_none());
        db.execute_batch("DROP TRIGGER reject_marker_delete")
            .unwrap();
        f.db.reconcile_untracked_plugin(&review, &f.intent("alice"), None)
            .unwrap();
        assert!(!f.db.has_untracked_plugin_sync().unwrap());
        let outbox =
            f.db.next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap();
        assert_eq!(outbox.generation, review.generation);
        assert_eq!(outbox.source_generation, review.generation);
        assert_eq!(outbox.enabled, Some(true));
        assert_eq!(outbox.runtime_epoch, None);
        assert!(f
            .db
            .reconcile_untracked_plugin(&review, &f.intent("alice"), None)
            .is_err());
    }

    #[test]
    fn reviewed_untracked_transfer_preserves_other_accounts_and_tombstones() {
        let f = Fixture::new();
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("bob")))
            .unwrap();
        f.db.set_plugin_state_with_untracked_sync("hotkeys", true, true)
            .unwrap();
        let review = f.db.review_untracked_plugin("hotkeys").unwrap().unwrap();
        assert!(f
            .db
            .reconcile_untracked_plugin(&review, &f.intent("alice"), None)
            .is_err());
        assert!(f.db.has_untracked_plugin_sync().unwrap());
        assert!(f
            .db
            .next_plugin_sync_intent("https://cloud.example", "bob")
            .unwrap()
            .is_some());
        f.db.set_plugin_state_with_untracked_sync("removed", false, false)
            .unwrap();
        let removed = f.db.review_untracked_plugin("removed").unwrap().unwrap();
        assert!(!removed.installed);
        assert!(f
            .db
            .reconcile_untracked_plugin(&removed, &f.intent("alice"), None)
            .is_err());
        assert!(f.db.review_untracked_plugin("removed").unwrap().is_some());
        f.db.set_plugin_state_with_untracked_sync("legacy", true, false)
            .unwrap();
        let legacy = f.db.review_untracked_plugin("legacy").unwrap().unwrap();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute("INSERT INTO plugin_sync_receipts(origin,account_id,plugin_id,generation,config_generation,runtime_epoch,pending_json) VALUES (?1,?2,?3,?4,?5,?6,?7)", params!["https://cloud.example", "bob", "legacy", legacy.generation, "7", "a".repeat(64), "{}"] ).unwrap();
        assert!(f
            .db
            .reconcile_untracked_plugin(&legacy, &f.intent("alice"), None)
            .is_err());
        assert_eq!(
            f.db.review_untracked_plugin("legacy").unwrap(),
            Some(legacy)
        );
    }

    #[test]
    fn v2_upload_source_reserves_database_and_rejects_stale_or_undrained_edits() {
        let f = Fixture::new();
        let generation =
            f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, None)
                .unwrap() as u64;
        let mut competing = f.db.open(&f.db.automation_path()).unwrap();
        competing.busy_timeout(std::time::Duration::ZERO).unwrap();
        let admitted = f
            .db
            .with_current_v2_upload_source::<_, DatabaseError>("hotkeys", generation, false, || {
                assert!(competing
                    .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                    .is_err());
                Ok(7)
            })
            .unwrap();
        assert_eq!(admitted, 7);
        assert!(f
            .db
            .with_current_v2_upload_source::<_, DatabaseError>("hotkeys", generation, true, || Ok(
                ()
            ))
            .is_err());
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
            .unwrap();
        assert!(f
            .db
            .with_current_v2_upload_source::<_, DatabaseError>("hotkeys", generation, false, || Ok(
                ()
            ))
            .is_err());
        assert!(
            f.db.with_current_v2_upload_source::<_, DatabaseError>(
                "hotkeys",
                generation + 1,
                false,
                || Ok(())
            )
            .is_err(),
            "undrained decision must not authorize upload"
        );
    }

    #[test]
    fn state_and_candidate_rollback_together_when_outbox_write_fails() {
        let f = Fixture::new();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_sync_insert BEFORE INSERT ON plugin_sync_outbox BEGIN SELECT RAISE(FAIL,'outbox failure'); END;").unwrap();
        drop(db);
        assert!(f
            .db
            .set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
            .is_err());
        let db = f.db.open(&f.db.automation_path()).unwrap();
        for table in [
            "behavior_plugins",
            "plugin_sync_source_versions",
            "plugin_sync_outbox",
        ] {
            let count: i64 = db
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "{table} must roll back");
        }
    }

    #[test]
    fn legacy_outbox_migration_preserves_unknown_consent_and_is_repeatable() {
        let f = Fixture::new();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        // Reproduce the interim schema in this test's disposable database.
        db.execute_batch(
            "DROP TABLE plugin_sync_outbox;
             CREATE TABLE plugin_sync_outbox (
               origin TEXT NOT NULL, account_id TEXT NOT NULL, plugin_id TEXT NOT NULL,
               generation INTEGER NOT NULL CHECK(generation > 0),
               version TEXT, enabled INTEGER CHECK(enabled IN (0,1)),
               CHECK((version IS NULL AND enabled IS NULL) OR (version IS NOT NULL AND enabled IS NOT NULL)),
               PRIMARY KEY(origin, account_id, plugin_id)
             );
             INSERT INTO plugin_sync_source_versions(plugin_id,generation) VALUES ('hotkeys',1);
             INSERT INTO plugin_sync_outbox VALUES ('https://cloud.example','alice','hotkeys',1,'1.0.0',1);"
        ).unwrap();
        drop(db);

        for _ in 0..2 {
            let reopened = DatabaseManager::new(f.db.paths().clone());
            let db = reopened.open(&reopened.automation_path()).unwrap();
            let row: (String, String, String, i64, Option<String>, Option<String>, Option<i64>) = db.query_row(
                "SELECT origin,account_id,plugin_id,generation,config_generation,version,enabled FROM plugin_sync_outbox",
                [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?)),
            ).unwrap();
            assert_eq!(
                row,
                (
                    "https://cloud.example".into(),
                    "alice".into(),
                    "hotkeys".into(),
                    1,
                    None,
                    Some("1.0.0".into()),
                    Some(1)
                )
            );
            assert!(matches!(
                reopened.next_plugin_sync_intent("https://cloud.example", "alice"),
                Err(DatabaseError::Invalid(_))
            ));
        }
        // A later write must not replace the unknown-consent row or advance
        // its source as a side effect of a failed reservation.
        assert!(f
            .db
            .set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
            .is_err());
        let db = f.db.open(&f.db.automation_path()).unwrap();
        let preserved: (i64, Option<String>) = db.query_row(
            "SELECT generation,config_generation FROM plugin_sync_outbox WHERE account_id='alice'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(preserved, (1, None));
        let source: i64 = db
            .query_row(
                "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id='hotkeys'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, 1);
        drop(db);
        // New candidates do carry consent and remain writable after migration.
        f.db.set_plugin_state_with_sync_intent("other", true, true, Some(f.intent("bob")))
            .unwrap();
        assert_eq!(
            f.db.next_plugin_sync_intent("https://cloud.example", "bob")
                .unwrap()
                .unwrap()
                .config_generation,
            7
        );
    }

    #[test]
    fn durable_identity_partition_and_generation_cas_preserve_newer_edits() {
        let f = Fixture::new();
        let a = "https://cloud.example";
        assert_eq!(
            f.db.set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
                .unwrap(),
            1
        );
        let old = f.db.next_plugin_sync_intent(a, "alice").unwrap().unwrap();
        assert!(f.db.next_plugin_sync_intent(a, "bob").unwrap().is_none());
        assert_eq!(
            f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
                .unwrap(),
            2
        );
        assert!(!f
            .db
            .acknowledge_plugin_sync_intent(&old, &"a".repeat(64))
            .unwrap());
        let newer = f.db.next_plugin_sync_intent(a, "alice").unwrap().unwrap();
        assert_eq!(newer.enabled, Some(false));
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert_eq!(
            reopened.next_plugin_sync_intent(a, "alice").unwrap(),
            Some(newer.clone())
        );
        // A local-only ABA changes the source and prevents old intent ack.
        reopened
            .set_plugin_state_with_sync_intent("hotkeys", true, true, None)
            .unwrap();
        assert!(!reopened
            .acknowledge_plugin_sync_intent(&newer, &"a".repeat(64))
            .unwrap());
        let stale = reopened
            .next_plugin_sync_intent(a, "alice")
            .unwrap()
            .unwrap();
        assert_eq!((stale.generation, stale.source_generation), (2, 3));
    }

    #[test]
    fn tombstone_remains_partitioned_and_corrupt_source_fails_closed() {
        let f = Fixture::new();
        let origin = "https://cloud.example";
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
            .unwrap();
        let deletion = LocalPluginSyncIntent {
            origin,
            account_id: "bob",
            config_generation: u128::MAX,
            version: None,
        };
        f.db.set_plugin_state_with_sync_intent("hotkeys", false, false, Some(deletion))
            .unwrap();
        let a =
            f.db.next_plugin_sync_intent(origin, "alice")
                .unwrap()
                .unwrap();
        let b =
            f.db.next_plugin_sync_intent(origin, "bob")
                .unwrap()
                .unwrap();
        assert_eq!((a.generation, a.source_generation), (1, 2));
        assert_eq!(
            (
                b.generation,
                b.source_generation,
                b.config_generation,
                b.version.as_deref(),
                b.enabled
            ),
            (2, 2, u128::MAX, None, None)
        );
        assert!(!f
            .db
            .acknowledge_plugin_sync_intent(&a, &"a".repeat(64))
            .unwrap());
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute(
            "DELETE FROM plugin_sync_source_versions WHERE plugin_id='hotkeys'",
            [],
        )
        .unwrap();
        assert!(f.db.next_plugin_sync_intent(origin, "bob").is_err());
    }

    #[test]
    fn runtime_confirmation_is_required_for_each_host_lifetime_and_source_generation() {
        let f = Fixture::new();
        let origin = "https://cloud.example";
        let first_epoch = "a".repeat(64);
        let next_epoch = "b".repeat(64);
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
            .unwrap();
        let original =
            f.db.next_plugin_sync_intent(origin, "alice")
                .unwrap()
                .unwrap();
        assert_eq!(original.runtime_epoch, None);
        // If applying the runtime change fails, the caller must not confirm it.
        assert!(!f
            .db
            .acknowledge_plugin_sync_intent(&original, &first_epoch)
            .unwrap());
        assert!(f
            .db
            .confirm_plugin_sync_runtime(&original, &first_epoch)
            .unwrap());
        let confirmed =
            f.db.next_plugin_sync_intent(origin, "alice")
                .unwrap()
                .unwrap();
        assert_eq!(
            confirmed.runtime_epoch.as_deref(),
            Some(first_epoch.as_str())
        );

        // A restarted host has a new epoch and cannot remove work merely
        // because the former host once observed the desired runtime state.
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert!(!reopened
            .acknowledge_plugin_sync_intent(&confirmed, &next_epoch)
            .unwrap());
        assert!(reopened
            .confirm_plugin_sync_runtime(&confirmed, &next_epoch)
            .unwrap());
        assert!(!f
            .db
            .acknowledge_plugin_sync_intent(&confirmed, &first_epoch)
            .unwrap());

        // A newer edit invalidates both confirmations, including an ABA that
        // returns to the same enabled value.
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
            .unwrap();
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
            .unwrap();
        assert!(!reopened
            .confirm_plugin_sync_runtime(&confirmed, &next_epoch)
            .unwrap());
        assert!(!reopened
            .acknowledge_plugin_sync_intent(&confirmed, &next_epoch)
            .unwrap());
        let current = reopened
            .next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .unwrap();
        assert_eq!(current.runtime_epoch, None);
        assert!(reopened
            .confirm_plugin_sync_runtime(&current, &next_epoch)
            .unwrap());
        assert!(reopened
            .acknowledge_plugin_sync_intent(&current, &next_epoch)
            .unwrap());
        assert!(reopened
            .next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .is_none());
    }

    #[test]
    fn receipt_move_is_atomic_and_fences_identity_source_consent_and_pending() {
        use tiktools_cloud_sync::{PluginSelection, V1Edit, V1Pending};

        let f = Fixture::new();
        let origin = "https://cloud.example";
        let epoch = "a".repeat(64);
        f.db.set_plugin_state_with_sync_intent("acme.plugin", true, false, Some(f.intent("alice")))
            .unwrap();
        let first =
            f.db.next_plugin_sync_intent(origin, "alice")
                .unwrap()
                .unwrap();
        assert!(f.db.confirm_plugin_sync_runtime(&first, &epoch).unwrap());
        let entry =
            f.db.next_plugin_sync_intent(origin, "alice")
                .unwrap()
                .unwrap();
        let pending = V1Pending {
            base_revision: Some(12),
            edit: V1Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "1.0.0".into(),
                enabled: false,
            }),
            conflict: false,
        };
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON plugin_sync_receipts BEGIN SELECT RAISE(FAIL,'receipt failure'); END;").unwrap();
        drop(db);
        assert!(f
            .db
            .receipt_plugin_sync_intent(&entry, &epoch, &pending)
            .is_err());
        assert_eq!(
            f.db.next_plugin_sync_intent(origin, "alice").unwrap(),
            Some(entry.clone())
        );
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch("DROP TRIGGER fail_receipt;").unwrap();
        drop(db);
        assert!(f
            .db
            .receipt_plugin_sync_intent(&entry, &epoch, &pending)
            .unwrap());
        assert!(f
            .db
            .next_plugin_sync_intent(origin, "alice")
            .unwrap()
            .is_none());

        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert!(reopened
            .validate_plugin_sync_receipt(origin, "alice", "acme.plugin", 7, &epoch, &pending)
            .unwrap());
        assert!(!reopened
            .validate_plugin_sync_receipt(origin, "bob", "acme.plugin", 7, &epoch, &pending)
            .unwrap());
        assert!(!reopened
            .validate_plugin_sync_receipt(origin, "alice", "acme.plugin", 8, &epoch, &pending)
            .unwrap());
        assert!(!reopened
            .validate_plugin_sync_receipt(
                origin,
                "alice",
                "acme.plugin",
                7,
                &"b".repeat(64),
                &pending
            )
            .unwrap());
        let mut rebased = pending.clone();
        rebased.base_revision = Some(13);
        assert!(!reopened
            .validate_plugin_sync_receipt(origin, "alice", "acme.plugin", 7, &epoch, &rebased)
            .unwrap());

        // A new host lifetime cannot reuse the previous runtime observation.
        // An explicit reconciliation confirms only the identical pending value.
        let restarted_epoch = "b".repeat(64);
        assert!(!reopened
            .confirm_plugin_sync_receipt_runtime(
                origin,
                "alice",
                "acme.plugin",
                7,
                &restarted_epoch,
                &rebased
            )
            .unwrap());
        assert!(reopened
            .confirm_plugin_sync_receipt_runtime(
                origin,
                "alice",
                "acme.plugin",
                7,
                &restarted_epoch,
                &pending
            )
            .unwrap());
        assert!(!reopened
            .validate_plugin_sync_receipt(origin, "alice", "acme.plugin", 7, &epoch, &pending)
            .unwrap());
        assert!(reopened
            .validate_plugin_sync_receipt(
                origin,
                "alice",
                "acme.plugin",
                7,
                &restarted_epoch,
                &pending
            )
            .unwrap());

        // Another account's edit advances the shared source generation, so
        // returning to Alice cannot dispatch the older pending value.
        reopened
            .set_plugin_state_with_sync_intent("acme.plugin", true, true, Some(f.intent("bob")))
            .unwrap();
        assert!(!reopened
            .validate_plugin_sync_receipt(
                origin,
                "alice",
                "acme.plugin",
                7,
                &restarted_epoch,
                &pending
            )
            .unwrap());
    }

    #[test]
    fn acknowledgement_cannot_erase_a_changed_payload_with_the_same_generation() {
        let f = Fixture::new();
        let epoch = "a".repeat(64);
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
            .unwrap();
        let entry =
            f.db.next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap();
        assert!(f.db.confirm_plugin_sync_runtime(&entry, &epoch).unwrap());

        // Even if retained storage is unexpectedly changed without advancing
        // its generation, an acknowledgement for the original payload must
        // preserve the new value for inspection instead of erasing it.
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute(
            "UPDATE plugin_sync_outbox SET enabled=1 WHERE plugin_id='hotkeys'",
            [],
        )
        .unwrap();
        drop(db);
        assert!(!f.db.acknowledge_plugin_sync_intent(&entry, &epoch).unwrap());
        assert_eq!(
            f.db.next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap()
                .enabled,
            Some(true)
        );
    }

    #[test]
    fn legacy_consent_bound_outbox_needs_runtime_reconciliation_after_migration() {
        let f = Fixture::new();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute_batch(
            "DROP TABLE plugin_sync_outbox;
             CREATE TABLE plugin_sync_outbox (
               origin TEXT NOT NULL, account_id TEXT NOT NULL, plugin_id TEXT NOT NULL,
               generation INTEGER NOT NULL, config_generation TEXT NOT NULL,
               version TEXT, enabled INTEGER, PRIMARY KEY(origin,account_id,plugin_id)
             );
             INSERT INTO plugin_sync_source_versions(plugin_id,generation) VALUES ('hotkeys',1);
             INSERT INTO behavior_plugins(id,installed,enabled,updated_at) VALUES ('hotkeys',1,1,0);
             INSERT INTO plugin_sync_outbox VALUES ('https://cloud.example','alice','hotkeys',1,'7','1.0.0',1);"
        ).unwrap();
        drop(db);
        for _ in 0..2 {
            let reopened = DatabaseManager::new(f.db.paths().clone());
            let entry = reopened
                .next_plugin_sync_intent("https://cloud.example", "alice")
                .unwrap()
                .unwrap();
            assert_eq!(entry.runtime_epoch, None);
            assert!(!reopened
                .acknowledge_plugin_sync_intent(&entry, &"a".repeat(64))
                .unwrap());
        }
    }

    #[test]
    fn package_journal_blocks_candidate_and_receipt_until_reconciled() {
        use tiktools_cloud_sync::{PluginSelection, V1Edit, V1Pending};
        let f = Fixture::new();
        let origin = "https://cloud.example";
        let epoch = "a".repeat(64);
        f.db.set_plugin_state_with_sync_intent("hotkeys", true, false, Some(f.intent("alice")))
            .unwrap();
        let entry =
            f.db.next_plugin_sync_intent(origin, "alice")
                .unwrap()
                .unwrap();
        assert!(f.db.confirm_plugin_sync_runtime(&entry, &epoch).unwrap());
        let confirmed =
            f.db.next_plugin_sync_intent(origin, "alice")
                .unwrap()
                .unwrap();
        let pending = V1Pending {
            base_revision: Some(1),
            conflict: false,
            edit: V1Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: "hotkeys".into(),
                version: "1.0.0".into(),
                enabled: false,
            }),
        };
        let insert_guard = || {
            let db = f.db.open(&f.db.automation_path()).unwrap();
            db.execute("INSERT INTO plugin_package_mutation_intents(plugin_id,intent_id,operation,operation_digest,phase,source_generation) VALUES ('hotkeys',?1,'update',?2,'mutating',1)",
                params!["b".repeat(64), "c".repeat(64)]).unwrap();
        };
        let remove_guard = || {
            let db = f.db.open(&f.db.automation_path()).unwrap();
            db.execute(
                "DELETE FROM plugin_package_mutation_intents WHERE plugin_id='hotkeys'",
                [],
            )
            .unwrap();
        };
        insert_guard();
        assert!(f.db.next_plugin_sync_intent(origin, "alice").is_err());
        assert!(!f.db.confirm_plugin_sync_runtime(&entry, &epoch).unwrap());
        assert!(!f
            .db
            .acknowledge_plugin_sync_intent(&confirmed, &epoch)
            .unwrap());
        assert!(!f
            .db
            .receipt_plugin_sync_intent(&confirmed, &epoch, &pending)
            .unwrap());
        assert!(f
            .db
            .set_plugin_state_with_sync_intent("hotkeys", true, true, Some(f.intent("alice")))
            .is_err());
        remove_guard();
        assert!(f
            .db
            .receipt_plugin_sync_intent(&confirmed, &epoch, &pending)
            .unwrap());
        insert_guard();
        assert!(!f
            .db
            .validate_plugin_sync_receipt(origin, "alice", "hotkeys", 7, &epoch, &pending)
            .unwrap());
        let mut conflict = pending.clone();
        conflict.conflict = true;
        assert!(f
            .db
            .plugin_sync_conflict_source(origin, "alice", "hotkeys", 7, &epoch, &conflict)
            .unwrap()
            .is_none());
        assert!(!f
            .db
            .confirm_plugin_sync_receipt_runtime(origin, "alice", "hotkeys", 7, &epoch, &pending)
            .unwrap());
        remove_guard();
        assert!(f
            .db
            .validate_plugin_sync_receipt(origin, "alice", "hotkeys", 7, &epoch, &pending)
            .unwrap());
    }
}
