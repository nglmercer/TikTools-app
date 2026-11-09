//! Durable source-fenced local application decisions for v1 sync.
//!
//! Storage is not authorization. The caller must hold the shared protocol
//! admission and local plugin mutation gates, revalidate the exact live
//! session and opt-in epoch, and check the coordinator's durable state before
//! advancing any phase. No runtime operation or network request occurs here.
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use tiktools_cloud_sync::{
    PluginSelection, SyncState, V1Edit, V1LocalEditContext, V1Pending, V1Record,
};

use super::{
    util::{bool_int, now},
    DatabaseError, DatabaseManager,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PluginApplyReason {
    Pulled,
    ReviewedConflict,
    ReviewedUncertain,
}

/// Native-only snapshot. A UI must never construct or edit this evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PluginApplyIntent {
    pub intent_id: String,
    pub origin: String,
    pub account_id: String,
    #[serde(with = "tiktools_cloud_sync::consent_generation")]
    pub config_generation: u128,
    pub plugin_id: String,
    pub source_generation: i64,
    pub installed_version: String,
    pub expected_enabled: bool,
    pub reviewed_pending: Option<V1Pending>,
    /// The frozen earlier base when known. None does not prove it was absent.
    pub base_record: Option<V1Record>,
    /// None means absent in Cloud and therefore disables, never uninstalls.
    pub remote: Option<V1Record>,
    pub reason: PluginApplyReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginApplyPhase {
    Prepared,
    Ready,
    RuntimePending,
    Applied,
    Finalized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredPluginApplyIntent {
    pub intent: PluginApplyIntent,
    pub phase: PluginApplyPhase,
    pub committed_generation: Option<i64>,
    pub runtime_epoch: Option<String>,
}

/// Native-only discovery result. A page bounds the number of *all* active
/// journal rows scanned, even if none belong to this identity. The cursor is
/// opaque and must never be exposed to the webview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginApplyRecoveryPage {
    pub intents: Vec<StoredPluginApplyIntent>,
    pub stale_consent: bool,
    pub next_cursor: Option<String>,
}

fn valid_token(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn matches_context(intent: &PluginApplyIntent, context: &V1LocalEditContext) -> bool {
    intent.origin == context.origin
        && intent.account_id == context.account_id
        && intent.config_generation == context.config_generation
}

fn valid_intent(intent: &PluginApplyIntent) -> bool {
    if !valid_token(&intent.intent_id)
        || intent.source_generation <= 0
        || intent.origin.is_empty()
        || intent.origin.len() > 2048
        || intent.account_id.is_empty()
        || intent.account_id.len() > 1024
        || intent
            .origin
            .chars()
            .chain(intent.account_id.chars())
            .any(char::is_control)
    {
        return false;
    }
    let mut validator = SyncState::new(intent.origin.clone(), intent.account_id.clone());
    let installed = V1Edit::Put(PluginSelection {
        schema_version: 1,
        plugin_id: intent.plugin_id.clone(),
        version: intent.installed_version.clone(),
        enabled: intent.expected_enabled,
    });
    if validator
        .stage(intent.plugin_id.clone(), installed)
        .is_err()
    {
        return false;
    }
    if let Some(pending) = &intent.reviewed_pending {
        if pending.base_revision.is_some_and(|revision| revision <= 0)
            || validator
                .stage(intent.plugin_id.clone(), pending.edit.clone())
                .is_err()
        {
            return false;
        }
    }
    for record in [&intent.base_record, &intent.remote].into_iter().flatten() {
        if record.collection != "plugin-selections"
            || record.key != intent.plugin_id
            || record.revision <= 0
            || record.deleted != record.value.is_none()
            || record.value.as_ref().is_some_and(|selection| {
                selection.schema_version != 1
                    || selection.plugin_id != intent.plugin_id
                    || validator
                        .stage(intent.plugin_id.clone(), V1Edit::Put(selection.clone()))
                        .is_err()
            })
        {
            return false;
        }
    }
    if let (Some(base), Some(pending)) = (&intent.base_record, &intent.reviewed_pending) {
        if pending.base_revision != Some(base.revision) {
            return false;
        }
    }
    // A previously observed record cannot become an absent export without
    // an explicit reset protocol; treat that as a rewind, not a deletion.
    if intent.base_record.is_some() && intent.remote.is_none() {
        return false;
    }
    if let (Some(base), Some(remote)) = (&intent.base_record, &intent.remote) {
        if remote.revision < base.revision || (remote.revision == base.revision && remote != base) {
            return false;
        }
    }
    match intent.reason {
        PluginApplyReason::Pulled => intent.reviewed_pending.is_none() && intent.remote.is_some(),
        PluginApplyReason::ReviewedConflict => intent
            .reviewed_pending
            .as_ref()
            .is_some_and(|pending| pending.conflict),
        PluginApplyReason::ReviewedUncertain => intent.reviewed_pending.is_some(),
    }
}

fn payload(intent: &PluginApplyIntent) -> Result<String, DatabaseError> {
    if !valid_intent(intent) {
        return Err(DatabaseError::Invalid("invalid plugin apply intent".into()));
    }
    let serialized = serde_json::to_string(intent)?;
    if serialized.len() > 16 * 1024 {
        return Err(DatabaseError::Invalid(
            "plugin apply intent too large".into(),
        ));
    }
    Ok(serialized)
}

type StoredRow = (String, String, String, Option<i64>, Option<String>);

fn decode_stored(id: &str, row: StoredRow) -> Result<StoredPluginApplyIntent, DatabaseError> {
    let (plugin_id, phase, encoded, generation, epoch) = row;
    if !valid_token(id) || encoded.len() > 16 * 1024 {
        return Err(DatabaseError::Invalid("invalid plugin apply intent".into()));
    }
    let intent: PluginApplyIntent = serde_json::from_str(&encoded)?;
    if intent.intent_id != id || intent.plugin_id != plugin_id || !valid_intent(&intent) {
        return Err(DatabaseError::Invalid("invalid plugin apply intent".into()));
    }
    let phase = match (phase.as_str(), generation, epoch.as_deref()) {
        ("prepared", None, None) => PluginApplyPhase::Prepared,
        ("ready", None, None) => PluginApplyPhase::Ready,
        ("runtime_pending", Some(g), None)
            if g == intent.source_generation.checked_add(1).unwrap_or(0) =>
        {
            PluginApplyPhase::RuntimePending
        }
        ("applied", Some(g), Some(e))
            if g == intent.source_generation.checked_add(1).unwrap_or(0) && valid_token(e) =>
        {
            PluginApplyPhase::Applied
        }
        ("finalized", Some(g), Some(e))
            if g == intent.source_generation.checked_add(1).unwrap_or(0) && valid_token(e) =>
        {
            PluginApplyPhase::Finalized
        }
        _ => return Err(DatabaseError::Invalid("invalid plugin apply phase".into())),
    };
    Ok(StoredPluginApplyIntent {
        intent,
        phase,
        committed_generation: generation,
        runtime_epoch: epoch,
    })
}

impl DatabaseManager {
    /// Inspect the persisted local source for a pulled-record decision. This
    /// observation is not authorization: `prepare_plugin_apply` rechecks it
    /// atomically against any later source, outbox or journal changes.
    pub(crate) fn pulled_plugin_apply_source(
        &self,
        plugin_id: &str,
    ) -> Result<Option<(i64, bool)>, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        let found: Option<(i64, i64)> = db
            .query_row(
                "SELECT s.generation,b.enabled FROM plugin_sync_source_versions s
             JOIN behavior_plugins b ON b.id=s.plugin_id
             WHERE s.plugin_id=?1 AND b.installed=1
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL)
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_apply_intents i
                 WHERE i.plugin_id=s.plugin_id AND i.phase!='finalized')",
                [plugin_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match found {
            Some((generation, enabled)) if generation > 0 && (enabled == 0 || enabled == 1) => {
                Ok(Some((generation, enabled == 1)))
            }
            Some(_) => Err(DatabaseError::Invalid("invalid plugin apply source".into())),
            None => Ok(None),
        }
    }

    /// Persist reviewed evidence before changing a pending record or marker.
    /// A newer local candidate, missing source revision or different installed
    /// state rejects admission without changing anything.
    pub(crate) fn prepare_plugin_apply(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context) {
            return Ok(false);
        }
        let encoded = payload(intent)?;
        // Refuse the decision before coordinator pending state is touched if
        // applying it would require installing or replacing a package.
        if intent
            .remote
            .as_ref()
            .and_then(|record| record.value.as_ref())
            .is_some_and(|value| value.version != intent.installed_version)
        {
            return Ok(false);
        }
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<String> = tx
            .query_row(
                "SELECT payload_json FROM plugin_sync_apply_intents WHERE intent_id=?1",
                [&intent.intent_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(prior) = prior {
            if prior != encoded {
                return Err(DatabaseError::Invalid(
                    "plugin apply identity reused".into(),
                ));
            }
            // Even an idempotent retry must not report admission while a
            // package transition could change the artifact on disk.
            let blocked: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=?1 OR p.plugin_id IS NULL)",
                [&intent.plugin_id], |row| row.get(0),
            )?;
            return Ok(!blocked);
        }
        let eligible: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
             WHERE s.plugin_id=?1 AND s.generation=?2 AND b.installed=1 AND b.enabled=?3
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![intent.plugin_id, intent.source_generation, bool_int(intent.expected_enabled)], |row| row.get(0),
        )?;
        if !eligible {
            return Ok(false);
        }
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO plugin_sync_apply_intents(intent_id,plugin_id,phase,payload_json)
             VALUES (?1,?2,'prepared',?3)", params![intent.intent_id, intent.plugin_id, encoded],
        )?;
        if inserted != 1 {
            return Ok(false);
        }
        tx.commit()?;
        Ok(true)
    }

    /// Called only after native coordinator state has been reconciled under
    /// the same admission/session fence. The uncertain marker must remain
    /// durable until the final application outcome is acknowledged.
    pub(crate) fn mark_plugin_apply_ready_after_native_reconciliation(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context) {
            return Ok(false);
        }
        let encoded = payload(intent)?;
        let db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let changed = db.execute(
            "UPDATE plugin_sync_apply_intents SET phase='ready' WHERE intent_id=?1 AND plugin_id=?2
             AND payload_json=?3 AND phase='prepared' AND EXISTS(
                SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
                WHERE s.plugin_id=?2 AND s.generation=?4 AND b.installed=1 AND b.enabled=?5
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![intent.intent_id, intent.plugin_id, encoded, intent.source_generation, bool_int(intent.expected_enabled)],
        )?;
        Ok(changed == 1)
    }

    /// Revalidate a previously-ready decision against the same durable
    /// evidence and current source. This is observation only; the later
    /// state/runtime transition must independently compare those inputs.
    pub(crate) fn revalidate_ready_plugin_apply(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context) {
            return Ok(false);
        }
        let encoded = payload(intent)?;
        let db = self.open(&self.automation_path())?;
        Ok(db.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_apply_intents i
             JOIN plugin_sync_source_versions s ON s.plugin_id=i.plugin_id
             JOIN behavior_plugins b ON b.id=s.plugin_id
             WHERE i.intent_id=?1 AND i.plugin_id=?2 AND i.payload_json=?3
             AND i.phase='ready' AND s.generation=?4 AND b.installed=1 AND b.enabled=?5
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![
                intent.intent_id,
                intent.plugin_id,
                encoded,
                intent.source_generation,
                bool_int(intent.expected_enabled)
            ],
            |row| row.get(0),
        )?)
    }

    /// Atomically reserve the desired persisted state and advance its global
    /// source revision. The caller must have observed the same installed
    /// package version while holding the local mutation gate. Cloud never
    /// installs or replaces a package; version mismatch leaves the intent.
    pub(crate) fn commit_plugin_apply_state(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
        observed_version: &str,
    ) -> Result<Option<i64>, DatabaseError> {
        if !matches_context(intent, context) || observed_version != intent.installed_version {
            return Ok(None);
        }
        let encoded = payload(intent)?;
        let enabled = match intent
            .remote
            .as_ref()
            .and_then(|record| record.value.as_ref())
        {
            Some(value) if value.version == intent.installed_version => value.enabled,
            Some(_) => return Ok(None),
            None => false,
        };
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let eligible: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM plugin_sync_apply_intents i
             JOIN plugin_sync_source_versions s ON s.plugin_id=i.plugin_id
             JOIN behavior_plugins b ON b.id=s.plugin_id WHERE i.intent_id=?1 AND i.plugin_id=?2
             AND i.payload_json=?3 AND i.phase='ready' AND s.generation=?4 AND b.installed=1 AND b.enabled=?5
             AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
             AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![intent.intent_id, intent.plugin_id, encoded, intent.source_generation, bool_int(intent.expected_enabled)], |row| row.get(0),
        )?;
        if !eligible || intent.source_generation == i64::MAX {
            return Ok(None);
        }
        let generation = intent.source_generation + 1;
        let source = tx.execute("UPDATE plugin_sync_source_versions SET generation=?1 WHERE plugin_id=?2 AND generation=?3", params![generation,intent.plugin_id,intent.source_generation])?;
        let state = tx.execute(
            "UPDATE behavior_plugins SET enabled=?1,updated_at=?2 WHERE id=?3 AND installed=1",
            params![bool_int(enabled), now(), intent.plugin_id],
        )?;
        let journal = tx.execute("UPDATE plugin_sync_apply_intents SET phase='runtime_pending',committed_generation=?1 WHERE intent_id=?2 AND phase='ready'", params![generation,intent.intent_id])?;
        if source != 1 || state != 1 || journal != 1 {
            return Err(DatabaseError::Invalid(
                "plugin apply transition was incomplete".into(),
            ));
        }
        tx.commit()?;
        Ok(Some(generation))
    }

    /// Record an observed runtime outcome. Repeating this after restart is
    /// allowed only after a fresh observation, never by replaying start/stop.
    pub(crate) fn confirm_plugin_apply_runtime(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
        generation: i64,
        runtime_epoch: &str,
        observed_version: &str,
        observed_enabled: bool,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context)
            || !valid_token(runtime_epoch)
            || observed_version != intent.installed_version
        {
            return Ok(false);
        }
        let encoded = payload(intent)?;
        let enabled = match intent
            .remote
            .as_ref()
            .and_then(|record| record.value.as_ref())
        {
            Some(value) if value.version == intent.installed_version => value.enabled,
            Some(_) => return Ok(false),
            None => false,
        };
        if enabled != observed_enabled {
            return Ok(false);
        }
        let db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let changed = db.execute(
            "UPDATE plugin_sync_apply_intents SET phase='applied',runtime_epoch=?1 WHERE intent_id=?2
             AND plugin_id=?3 AND payload_json=?4 AND phase IN ('runtime_pending','applied') AND committed_generation=?5
             AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
                WHERE s.plugin_id=?3 AND s.generation=?5 AND b.installed=1 AND b.enabled=?6
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![runtime_epoch,intent.intent_id,intent.plugin_id,encoded,generation,bool_int(enabled)],
        )?;
        Ok(changed == 1)
    }

    /// Finalize only after the caller has durably acknowledged the original
    /// coordinator state and marker under the same session/admission fence.
    /// Keeping applied intents active until then prevents a second review from
    /// replacing evidence while crash recovery is still required.
    pub(crate) fn finalize_plugin_apply_after_native_ack(
        &self,
        context: &V1LocalEditContext,
        intent: &PluginApplyIntent,
        generation: i64,
        runtime_epoch: &str,
    ) -> Result<bool, DatabaseError> {
        if !matches_context(intent, context) || !valid_token(runtime_epoch) {
            return Ok(false);
        }
        let encoded = payload(intent)?;
        let enabled = match intent
            .remote
            .as_ref()
            .and_then(|record| record.value.as_ref())
        {
            Some(value) if value.version == intent.installed_version => value.enabled,
            Some(_) => return Ok(false),
            None => false,
        };
        let db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let changed = db.execute(
            "UPDATE plugin_sync_apply_intents SET phase='finalized' WHERE intent_id=?1 AND plugin_id=?2
             AND payload_json=?3 AND phase='applied' AND committed_generation=?4 AND runtime_epoch=?5
             AND EXISTS(SELECT 1 FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id
                WHERE s.plugin_id=?2 AND s.generation=?4 AND b.installed=1 AND b.enabled=?6
                AND NOT EXISTS(SELECT 1 FROM plugin_sync_outbox o WHERE o.plugin_id=s.plugin_id)
                AND NOT EXISTS(SELECT 1 FROM plugin_package_mutation_intents p WHERE p.plugin_id=s.plugin_id OR p.plugin_id IS NULL))",
            params![intent.intent_id,intent.plugin_id,encoded,generation,runtime_epoch,bool_int(enabled)],
        )?;
        Ok(changed == 1)
    }

    /// Read retained evidence after a crash; it is never permission to replay.
    pub(crate) fn plugin_apply_intent(
        &self,
        id: &str,
    ) -> Result<Option<StoredPluginApplyIntent>, DatabaseError> {
        if !valid_token(id) {
            return Ok(None);
        }
        let db = self.open(&self.automation_path())?;
        let row: Option<StoredRow> = db.query_row(
            "SELECT plugin_id,phase,payload_json,committed_generation,runtime_epoch FROM plugin_sync_apply_intents WHERE intent_id=?1", [id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
        ).optional()?;
        row.map(|row| decode_stored(id, row)).transpose()
    }

    /// Discover unresolved native application journals after restart. This is
    /// only a bounded inventory, never authorization to apply or replay. The
    /// caller must hold the shared admission lease and current-session fence
    /// for the complete page sequence; otherwise concurrent insertions may
    /// sort before the cursor. Every scanned row is validated, including
    /// other accounts, so corruption is never silently hidden by filtering.
    /// Rows from another identity are not returned. Old consent for the same
    /// identity sets a blocking flag but does not grant its saved evidence.
    pub(crate) fn plugin_apply_recovery_page(
        &self,
        context: &V1LocalEditContext,
        after: Option<&str>,
        limit: usize,
    ) -> Result<PluginApplyRecoveryPage, DatabaseError> {
        if !(1..=64).contains(&limit) || after.is_some_and(|cursor| !valid_token(cursor)) {
            return Err(DatabaseError::Invalid("invalid plugin apply cursor".into()));
        }
        let db = self.open(&self.automation_path())?;
        let query = if after.is_some() {
            "SELECT intent_id,plugin_id,phase,payload_json,committed_generation,runtime_epoch
             FROM plugin_sync_apply_intents WHERE phase != 'finalized' AND intent_id > ?1
             ORDER BY intent_id LIMIT ?2"
        } else {
            // Include even an invalid empty id so corruption is not hidden.
            "SELECT intent_id,plugin_id,phase,payload_json,committed_generation,runtime_epoch
             FROM plugin_sync_apply_intents WHERE phase != 'finalized' AND intent_id >= ?1
             ORDER BY intent_id LIMIT ?2"
        };
        let mut stmt = db.prepare(query)?;
        let rows = stmt
            .query_map(params![after.unwrap_or(""), (limit + 1) as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ),
                ))
            })?
            .collect::<Result<Vec<(String, StoredRow)>, _>>()?;
        let more = rows.len() > limit;
        let mut intents = Vec::new();
        let mut stale_consent = false;
        let mut cursor = None;
        for (id, row) in rows.into_iter().take(limit) {
            let saved = decode_stored(&id, row)?;
            if saved.intent.origin == context.origin
                && saved.intent.account_id == context.account_id
            {
                if saved.intent.config_generation == context.config_generation {
                    intents.push(saved);
                } else {
                    stale_consent = true;
                }
            }
            cursor = Some(id);
        }
        Ok(PluginApplyRecoveryPage {
            intents,
            stale_consent,
            next_cursor: if more { cursor } else { None },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AppPaths;
    use std::path::PathBuf;

    struct Fixture {
        db: DatabaseManager,
        root: PathBuf,
        context: V1LocalEditContext,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "plugin-apply-{}-{}",
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
            let db = DatabaseManager::new(paths);
            assert_eq!(
                db.set_plugin_state_with_sync_intent("acme.plugin", true, true, None)
                    .unwrap(),
                1
            );
            Self {
                db,
                root,
                context: V1LocalEditContext {
                    origin: "https://cloud.example".into(),
                    account_id: "alice".into(),
                    config_generation: 9,
                },
            }
        }
        fn intent(&self) -> PluginApplyIntent {
            PluginApplyIntent {
                intent_id: "a".repeat(64),
                origin: self.context.origin.clone(),
                account_id: self.context.account_id.clone(),
                config_generation: self.context.config_generation,
                plugin_id: "acme.plugin".into(),
                source_generation: 1,
                installed_version: "1.0.0".into(),
                expected_enabled: true,
                reviewed_pending: Some(V1Pending {
                    base_revision: None,
                    edit: V1Edit::Put(PluginSelection {
                        schema_version: 1,
                        plugin_id: "acme.plugin".into(),
                        version: "1.0.0".into(),
                        enabled: true,
                    }),
                    conflict: false,
                }),
                base_record: None,
                remote: None,
                reason: PluginApplyReason::ReviewedUncertain,
            }
        }
        fn state(&self) -> (i64, i64, i64) {
            let db = self.db.open(&self.db.automation_path()).unwrap();
            db.query_row("SELECT s.generation,b.installed,b.enabled FROM plugin_sync_source_versions s JOIN behavior_plugins b ON b.id=s.plugin_id WHERE s.plugin_id='acme.plugin'", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap()
        }
    }

    #[test]
    fn pulled_source_observation_excludes_unresolved_edits_and_apply_journals() {
        let f = Fixture::new();
        assert_eq!(
            f.db.pulled_plugin_apply_source("acme.plugin").unwrap(),
            Some((1, true))
        );
        assert_eq!(
            f.db.pulled_plugin_apply_source("missing.plugin").unwrap(),
            None
        );
        assert!(f.db.prepare_plugin_apply(&f.context, &f.intent()).unwrap());
        assert_eq!(
            f.db.pulled_plugin_apply_source("acme.plugin").unwrap(),
            None
        );

        let other = Fixture::new();
        other
            .db
            .set_plugin_state_with_sync_intent(
                "acme.plugin",
                true,
                false,
                Some(super::super::LocalPluginSyncIntent {
                    origin: &other.context.origin,
                    account_id: &other.context.account_id,
                    config_generation: other.context.config_generation,
                    version: Some("1.0.0"),
                }),
            )
            .unwrap();
        assert_eq!(
            other.db.pulled_plugin_apply_source("acme.plugin").unwrap(),
            None
        );
    }

    #[test]
    fn package_journal_fences_every_apply_phase_and_missing_guard_fails_closed() {
        let f = Fixture::new();
        let intent = f.intent();
        let change_guard = |active: bool| {
            let db = f.db.open(&f.db.automation_path()).unwrap();
            if active {
                db.execute("INSERT INTO plugin_package_mutation_intents(plugin_id,intent_id,operation,operation_digest,phase,source_generation) VALUES ('acme.plugin','test-package','install','digest','prepared',1)", []).unwrap();
            } else {
                db.execute(
                    "DELETE FROM plugin_package_mutation_intents WHERE plugin_id='acme.plugin'",
                    [],
                )
                .unwrap();
            }
        };
        change_guard(true);
        assert!(f
            .db
            .pulled_plugin_apply_source("acme.plugin")
            .unwrap()
            .is_none());
        assert!(!f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        change_guard(false);
        assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        change_guard(true);
        assert!(!f
            .db
            .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
            .unwrap());
        assert!(!f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        change_guard(false);
        assert!(f
            .db
            .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
            .unwrap());
        change_guard(true);
        assert!(!f
            .db
            .revalidate_ready_plugin_apply(&f.context, &intent)
            .unwrap());
        assert!(f
            .db
            .commit_plugin_apply_state(&f.context, &intent, "1.0.0")
            .unwrap()
            .is_none());
        change_guard(false);
        assert!(f
            .db
            .revalidate_ready_plugin_apply(&f.context, &intent)
            .unwrap());
        let generation =
            f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
                .unwrap()
                .unwrap();
        let epoch = "b".repeat(64);
        change_guard(true);
        assert!(!f
            .db
            .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "1.0.0", false)
            .unwrap());
        change_guard(false);
        assert!(f
            .db
            .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "1.0.0", false)
            .unwrap());
        change_guard(true);
        assert!(!f
            .db
            .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &epoch)
            .unwrap());
        change_guard(false);
        assert!(f
            .db
            .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &epoch)
            .unwrap());

        // A malformed legacy row with no plugin identity blocks globally;
        // a missing table produces an error rather than granting admission.
        let other = Fixture::new();
        let db = other.db.open(&other.db.automation_path()).unwrap();
        db.execute_batch("DROP TABLE plugin_package_mutation_intents; CREATE TABLE plugin_package_mutation_intents(plugin_id TEXT); INSERT INTO plugin_package_mutation_intents VALUES (NULL);").unwrap();
        drop(db);
        assert!(other
            .db
            .pulled_plugin_apply_source("acme.plugin")
            .unwrap()
            .is_none());
        assert!(!other
            .db
            .prepare_plugin_apply(&other.context, &other.intent())
            .unwrap());
        let db = other.db.open(&other.db.automation_path()).unwrap();
        db.execute_batch("DROP TABLE plugin_package_mutation_intents;")
            .unwrap();
        drop(db);
        assert!(other.db.pulled_plugin_apply_source("acme.plugin").is_err());
        assert!(other
            .db
            .prepare_plugin_apply(&other.context, &other.intent())
            .is_err());
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn uncertain_absence_is_durable_and_requires_reconciliation_and_runtime_observation() {
        let f = Fixture::new();
        let intent = f.intent();
        let epoch = "b".repeat(64);
        assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        let mut changed = intent.clone();
        changed.expected_enabled = false;
        assert!(f.db.prepare_plugin_apply(&f.context, &changed).is_err());
        let mut other = intent.clone();
        other.intent_id = "c".repeat(64);
        assert!(!f.db.prepare_plugin_apply(&f.context, &other).unwrap());
        assert_eq!(
            f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
                .unwrap(),
            None
        );
        assert_eq!(f.state(), (1, 1, 1));
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert_eq!(
            reopened
                .plugin_apply_intent(&intent.intent_id)
                .unwrap()
                .unwrap()
                .phase,
            PluginApplyPhase::Prepared
        );
        assert!(reopened
            .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
            .unwrap());
        let generation = reopened
            .commit_plugin_apply_state(&f.context, &intent, "1.0.0")
            .unwrap()
            .unwrap();
        assert_eq!(generation, 2);
        assert_eq!(f.state(), (2, 1, 0));
        assert_eq!(
            reopened
                .plugin_apply_intent(&intent.intent_id)
                .unwrap()
                .unwrap()
                .phase,
            PluginApplyPhase::RuntimePending
        );
        assert!(!reopened
            .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "1.0.0", true)
            .unwrap());
        assert!(!reopened
            .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "2.0.0", false)
            .unwrap());
        let mut other_context = f.context.clone();
        other_context.account_id = "bob".into();
        assert!(!reopened
            .confirm_plugin_apply_runtime(
                &other_context,
                &intent,
                generation,
                &epoch,
                "1.0.0",
                false
            )
            .unwrap());
        assert!(reopened
            .confirm_plugin_apply_runtime(&f.context, &intent, generation, &epoch, "1.0.0", false)
            .unwrap());
        assert_eq!(
            reopened
                .plugin_apply_intent(&intent.intent_id)
                .unwrap()
                .unwrap()
                .phase,
            PluginApplyPhase::Applied
        );
        other.source_generation = generation;
        other.expected_enabled = false;
        assert!(!reopened.prepare_plugin_apply(&f.context, &other).unwrap());
        assert!(!reopened
            .finalize_plugin_apply_after_native_ack(
                &f.context,
                &intent,
                generation,
                &"d".repeat(64)
            )
            .unwrap());
        // A restarted host observes the runtime again under its new epoch;
        // retaining the old epoch alone must not authorize finalization.
        let restarted_epoch = "d".repeat(64);
        assert!(reopened
            .confirm_plugin_apply_runtime(
                &f.context,
                &intent,
                generation,
                &restarted_epoch,
                "1.0.0",
                false
            )
            .unwrap());
        assert!(!reopened
            .finalize_plugin_apply_after_native_ack(&f.context, &intent, generation, &epoch)
            .unwrap());
        assert!(reopened
            .finalize_plugin_apply_after_native_ack(
                &f.context,
                &intent,
                generation,
                &restarted_epoch
            )
            .unwrap());
        assert_eq!(
            reopened
                .plugin_apply_intent(&intent.intent_id)
                .unwrap()
                .unwrap()
                .phase,
            PluginApplyPhase::Finalized
        );
        assert!(reopened.prepare_plugin_apply(&f.context, &other).unwrap());
    }

    #[test]
    fn local_changes_and_aba_make_prepared_or_applied_evidence_stale() {
        let f = Fixture::new();
        let intent = f.intent();
        let epoch = "b".repeat(64);
        assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        assert!(f
            .db
            .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
            .unwrap());
        f.db.set_plugin_state_with_sync_intent("acme.plugin", true, false, None)
            .unwrap();
        f.db.set_plugin_state_with_sync_intent("acme.plugin", true, true, None)
            .unwrap();
        assert_eq!(f.state(), (3, 1, 1));
        assert_eq!(
            f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
                .unwrap(),
            None
        );
        assert_eq!(
            f.db.plugin_apply_intent(&intent.intent_id)
                .unwrap()
                .unwrap()
                .phase,
            PluginApplyPhase::Ready
        );

        let g = Fixture::new();
        let intent = g.intent();
        assert!(g.db.prepare_plugin_apply(&g.context, &intent).unwrap());
        assert!(g
            .db
            .mark_plugin_apply_ready_after_native_reconciliation(&g.context, &intent)
            .unwrap());
        let generation =
            g.db.commit_plugin_apply_state(&g.context, &intent, "1.0.0")
                .unwrap()
                .unwrap();
        assert!(g
            .db
            .confirm_plugin_apply_runtime(&g.context, &intent, generation, &epoch, "1.0.0", false)
            .unwrap());
        g.db.set_plugin_state_with_sync_intent("acme.plugin", true, true, None)
            .unwrap();
        assert!(!g
            .db
            .finalize_plugin_apply_after_native_ack(&g.context, &intent, generation, &epoch)
            .unwrap());
        assert_eq!(
            g.db.plugin_apply_intent(&intent.intent_id)
                .unwrap()
                .unwrap()
                .phase,
            PluginApplyPhase::Applied
        );
    }

    #[test]
    fn failed_state_transition_rolls_back_and_cloud_never_installs_or_changes_version() {
        let f = Fixture::new();
        let mut intent = f.intent();
        intent.remote = Some(V1Record {
            collection: "plugin-selections".into(),
            key: "acme.plugin".into(),
            revision: 4,
            deleted: false,
            value: Some(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "2.0.0".into(),
                enabled: false,
            }),
        });
        assert!(!f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        assert!(!f
            .db
            .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
            .unwrap());
        assert_eq!(
            f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
                .unwrap(),
            None
        );
        assert_eq!(f.state(), (1, 1, 1));

        let g = Fixture::new();
        let intent = g.intent();
        assert!(g.db.prepare_plugin_apply(&g.context, &intent).unwrap());
        assert!(g
            .db
            .mark_plugin_apply_ready_after_native_reconciliation(&g.context, &intent)
            .unwrap());
        let db = g.db.open(&g.db.automation_path()).unwrap();
        db.execute_batch("CREATE TRIGGER fail_apply BEFORE UPDATE ON plugin_sync_apply_intents WHEN NEW.phase='runtime_pending' BEGIN SELECT RAISE(FAIL,'apply failure'); END;").unwrap();
        drop(db);
        assert!(g
            .db
            .commit_plugin_apply_state(&g.context, &intent, "1.0.0")
            .is_err());
        assert_eq!(g.state(), (1, 1, 1));
        assert_eq!(
            g.db.plugin_apply_intent(&intent.intent_id)
                .unwrap()
                .unwrap()
                .phase,
            PluginApplyPhase::Ready
        );
    }

    #[test]
    fn invalid_or_rewound_native_evidence_is_rejected() {
        let f = Fixture::new();
        let mut intent = f.intent();
        let base = V1Record {
            collection: "plugin-selections".into(),
            key: "acme.plugin".into(),
            revision: 5,
            deleted: false,
            value: Some(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "1.0.0".into(),
                enabled: true,
            }),
        };
        intent.base_record = Some(base.clone());
        intent.reviewed_pending.as_mut().unwrap().base_revision = Some(5);
        let mut old = base;
        old.revision = 4;
        intent.remote = Some(old);
        assert!(f.db.prepare_plugin_apply(&f.context, &intent).is_err());
        let mut context = f.context.clone();
        context.config_generation += 1;
        assert!(!f.db.prepare_plugin_apply(&context, &f.intent()).unwrap());
        f.db.set_plugin_state_with_sync_intent("acme.plugin", false, false, None)
            .unwrap();
        let mut uninstalled = f.intent();
        uninstalled.source_generation = 2;
        uninstalled.expected_enabled = false;
        assert!(!f.db.prepare_plugin_apply(&f.context, &uninstalled).unwrap());

        let g = Fixture::new();
        let candidate = super::super::LocalPluginSyncIntent {
            origin: &g.context.origin,
            account_id: "bob",
            config_generation: g.context.config_generation,
            version: Some("1.0.0"),
        };
        g.db.set_plugin_state_with_sync_intent("acme.plugin", true, true, Some(candidate))
            .unwrap();
        let mut stale_account = g.intent();
        stale_account.source_generation = 2;
        assert!(!g
            .db
            .prepare_plugin_apply(&g.context, &stale_account)
            .unwrap());
    }

    #[test]
    fn tombstone_disables_an_existing_plugin_without_uninstalling_it() {
        let f = Fixture::new();
        let mut intent = f.intent();
        intent.remote = Some(V1Record {
            collection: "plugin-selections".into(),
            key: "acme.plugin".into(),
            revision: 3,
            deleted: true,
            value: None,
        });
        assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        assert!(f
            .db
            .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
            .unwrap());
        assert_eq!(
            f.db.commit_plugin_apply_state(&f.context, &intent, "1.0.0")
                .unwrap(),
            Some(2)
        );
        assert_eq!(f.state(), (2, 1, 0));
    }

    #[test]
    fn ignored_sqlite_update_rolls_back_the_entire_apply_transition() {
        for table in [
            "plugin_sync_source_versions",
            "behavior_plugins",
            "plugin_sync_apply_intents",
        ] {
            let f = Fixture::new();
            let intent = f.intent();
            assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
            assert!(f
                .db
                .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
                .unwrap());
            let db = f.db.open(&f.db.automation_path()).unwrap();
            db.execute_batch(&format!("CREATE TRIGGER ignore_transition BEFORE UPDATE ON {table} BEGIN SELECT RAISE(IGNORE); END;")).unwrap();
            drop(db);
            assert!(f
                .db
                .commit_plugin_apply_state(&f.context, &intent, "1.0.0")
                .is_err());
            assert_eq!(f.state(), (1, 1, 1));
            assert_eq!(
                f.db.plugin_apply_intent(&intent.intent_id)
                    .unwrap()
                    .unwrap()
                    .phase,
                PluginApplyPhase::Ready
            );
        }
    }

    #[test]
    fn corrupt_evidence_fails_closed_after_restart() {
        let f = Fixture::new();
        let intent = f.intent();
        assert!(f.db.prepare_plugin_apply(&f.context, &intent).unwrap());
        let mut altered = intent.clone();
        altered.plugin_id = "other.plugin".into();
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute(
            "UPDATE plugin_sync_apply_intents SET payload_json=?1 WHERE intent_id=?2",
            params![serde_json::to_string(&altered).unwrap(), intent.intent_id],
        )
        .unwrap();
        drop(db);
        let reopened = DatabaseManager::new(f.db.paths().clone());
        assert!(reopened.plugin_apply_intent(&intent.intent_id).is_err());
        assert!(!reopened
            .mark_plugin_apply_ready_after_native_reconciliation(&f.context, &intent)
            .unwrap());
        assert_eq!(f.state(), (1, 1, 1));
    }

    #[test]
    fn recovery_discovery_is_bounded_partitioned_and_survives_restart() {
        let f = Fixture::new();
        for (n, account, config, origin) in [
            (1, "alice", 9, "https://cloud.example"),
            (2, "bob", 9, "https://cloud.example"),
            (3, "alice", 8, "https://cloud.example"),
            (4, "alice", 9, "https://other.example"),
            (5, "alice", 9, "https://cloud.example"),
            (6, "alice", 9, "https://cloud.example"),
        ] {
            let key = format!("acme.plugin{n}");
            assert_eq!(
                f.db.set_plugin_state_with_sync_intent(&key, true, true, None)
                    .unwrap(),
                1
            );
            let context = V1LocalEditContext {
                origin: origin.into(),
                account_id: account.into(),
                config_generation: config,
            };
            let mut intent = f.intent();
            intent.intent_id = format!("{n:064x}");
            intent.origin = context.origin.clone();
            intent.account_id = context.account_id.clone();
            intent.config_generation = config;
            intent.plugin_id = key.clone();
            if let Some(V1Pending {
                edit: V1Edit::Put(value),
                ..
            }) = &mut intent.reviewed_pending
            {
                value.plugin_id = key;
            } else {
                panic!("fixture pending value");
            }
            assert!(f.db.prepare_plugin_apply(&context, &intent).unwrap());
            if n == 6 {
                assert!(f
                    .db
                    .mark_plugin_apply_ready_after_native_reconciliation(&context, &intent)
                    .unwrap());
                let generation =
                    f.db.commit_plugin_apply_state(&context, &intent, "1.0.0")
                        .unwrap()
                        .unwrap();
                let epoch = "b".repeat(64);
                assert!(f
                    .db
                    .confirm_plugin_apply_runtime(
                        &context, &intent, generation, &epoch, "1.0.0", false
                    )
                    .unwrap());
                assert!(f
                    .db
                    .finalize_plugin_apply_after_native_ack(&context, &intent, generation, &epoch)
                    .unwrap());
            }
        }
        let reopened = DatabaseManager::new(f.db.paths().clone());
        let first = reopened
            .plugin_apply_recovery_page(&f.context, None, 2)
            .unwrap();
        assert_eq!(
            first
                .intents
                .iter()
                .map(|v| v.intent.plugin_id.as_str())
                .collect::<Vec<_>>(),
            ["acme.plugin1"]
        );
        assert!(!first.stale_consent);
        assert_eq!(
            first.next_cursor.as_deref(),
            Some(format!("{:064x}", 2).as_str())
        );
        let second = reopened
            .plugin_apply_recovery_page(&f.context, first.next_cursor.as_deref(), 2)
            .unwrap();
        assert!(second.intents.is_empty());
        assert!(second.stale_consent);
        let third = reopened
            .plugin_apply_recovery_page(&f.context, second.next_cursor.as_deref(), 2)
            .unwrap();
        assert_eq!(
            third
                .intents
                .iter()
                .map(|v| v.intent.plugin_id.as_str())
                .collect::<Vec<_>>(),
            ["acme.plugin5"]
        );
        assert!(!third.stale_consent);
        assert!(third.next_cursor.is_none());
        assert!(reopened
            .plugin_apply_recovery_page(&f.context, None, 0)
            .is_err());
        assert!(reopened
            .plugin_apply_recovery_page(&f.context, None, 65)
            .is_err());
        assert!(reopened
            .plugin_apply_recovery_page(&f.context, Some("invalid"), 1)
            .is_err());
    }

    #[test]
    fn recovery_discovery_never_skips_a_corrupt_foreign_identity() {
        let f = Fixture::new();
        let mut intent = f.intent();
        intent.account_id = "bob".into();
        let bob = V1LocalEditContext {
            account_id: "bob".into(),
            ..f.context.clone()
        };
        assert!(f.db.prepare_plugin_apply(&bob, &intent).unwrap());
        let db = f.db.open(&f.db.automation_path()).unwrap();
        db.execute(
            "UPDATE plugin_sync_apply_intents SET payload_json = '{broken' WHERE intent_id = ?1",
            [&intent.intent_id],
        )
        .unwrap();
        drop(db);
        assert!(f
            .db
            .plugin_apply_recovery_page(&f.context, None, 10)
            .is_err());

        // An invalid empty primary key sorts before valid generated ids. The
        // first page must still inspect it rather than treating it as a cursor.
        let other = Fixture::new();
        let intent = other.intent();
        assert!(other
            .db
            .prepare_plugin_apply(&other.context, &intent)
            .unwrap());
        let db = other.db.open(&other.db.automation_path()).unwrap();
        db.execute(
            "UPDATE plugin_sync_apply_intents SET intent_id = '' WHERE intent_id = ?1",
            [&intent.intent_id],
        )
        .unwrap();
        drop(db);
        assert!(other
            .db
            .plugin_apply_recovery_page(&other.context, None, 10)
            .is_err());
    }
}
