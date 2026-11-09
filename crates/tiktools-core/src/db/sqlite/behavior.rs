use crate::collaboration::{ResourceVersion, Versioned};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
mod approval_review;
mod approvals;
pub(crate) use approvals::{
    BehaviorApprovalActivation, BehaviorApprovalBinding, BehaviorApprovalEnableState,
    BehaviorApprovalSnapshot,
};
mod commands;
mod drafts;
pub(super) mod library;
mod provenance;
pub(crate) use provenance::{BehaviorAuthority, BehaviorProvenance, BehaviorRuntimeAuthority};
pub(crate) type BehaviorRuntimeSnapshot = (
    Versioned<Value>,
    Vec<(String, String, BehaviorRuntimeAuthority)>,
);
#[cfg(test)]
mod plugin_source_tests;
mod remote;
mod review;
#[cfg(test)]
mod tests;
mod versions;
#[derive(Clone, Copy)]
pub(crate) enum BehaviorSave {
    Upsert,
    Create,
    Update,
}
use serde_json::{json, Value};

use super::{
    util::{bool_int, now, required_value_string},
    DatabaseError, DatabaseManager,
};

/// Advance on every accepted write, including a repeated value or deletion of
/// an absent row. The caller may be recording a fresh runtime/package
/// decision that cannot safely reuse evidence captured before this write.
/// Keep this in the same transaction as the source change: a generation must
/// never advance without its state, or vice versa.
pub(super) fn advance_plugin_source(tx: &Transaction<'_>, id: &str) -> Result<i64, DatabaseError> {
    tx.query_row(
        "INSERT INTO plugin_sync_source_versions(plugin_id,generation) VALUES (?1,1)
         ON CONFLICT(plugin_id) DO UPDATE SET generation=generation+1
         WHERE generation < 9223372036854775807 RETURNING generation",
        [id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| DatabaseError::Invalid("plugin source generation exhausted".into()))
}

impl DatabaseManager {
    pub(crate) fn load_behavior_snapshot(&self) -> Result<Value, DatabaseError> {
        self.load_behavior_snapshot_versioned().map(|v| v.value)
    }
    pub(crate) fn load_behavior_snapshot_versioned(
        &self,
    ) -> Result<Versioned<Value>, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let snapshot = Self::load_behavior_snapshot_from(&tx)?;
        tx.commit()?;
        Ok(snapshot)
    }
    /// Bodies, versions and trusted authority from one read transaction. The
    /// sidecar is returned separately and never embedded in editable JSON.
    #[cfg(test)]
    pub(crate) fn load_behavior_runtime_snapshot(
        &self,
    ) -> Result<BehaviorRuntimeSnapshot, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        provenance::validate_runtime_catalog(&tx)?;
        let snapshot = Self::load_behavior_snapshot_from(&tx)?;
        let authority = provenance::rows(&tx)?;
        tx.commit()?;
        Ok((snapshot, authority))
    }
    /// Definitions, authority and durable approvals share one read snapshot.
    /// Approval JSON remains a candidate; it cannot construct a runtime permit.
    pub(crate) fn load_behavior_runtime_approval_snapshot(
        &self,
    ) -> Result<(BehaviorRuntimeSnapshot, Vec<BehaviorApprovalSnapshot>), DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        provenance::validate_runtime_catalog(&tx)?;
        let snapshot = Self::load_behavior_snapshot_from(&tx)?;
        let authority = provenance::rows(&tx)?;
        let approvals = approvals::load(&tx)?;
        tx.commit()?;
        Ok(((snapshot, authority), approvals))
    }
    // Ordinary owner reads and plugin activation remain available for repair
    // when provenance is corrupt. Runtime admission uses the trusted loader
    // above and still fails closed; this helper never installs definitions.
    fn load_behavior_snapshot_from(db: &Connection) -> Result<Versioned<Value>, DatabaseError> {
        let actions = versions::rows(db, "behavior_actions")?;
        let events = versions::rows(db, "behavior_events")?;
        let plugins = Self::load_plugin_states_from(db)?;
        let mut meta = actions.resource_versions;
        meta.extend(events.resource_versions);
        Ok(Versioned::new(
            json!({"actions":actions.value,"events":events.value,"plugins":plugins,"actionTypes":[],"translations":{}}),
            meta,
        ))
    }
    pub(crate) fn behavior_get_versioned(
        &self,
        table: &str,
        id: &str,
    ) -> Result<Versioned<Option<Value>>, DatabaseError> {
        versions::scope(table)?;
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let value = versions::row(&tx, table, id)?;
        let revision = versions::current(&tx, table, id)?;
        let result = Versioned::new(value, vec![versions::version(table, id, revision)?]);
        tx.commit()?;
        Ok(result)
    }
    fn behavior_transaction<T>(
        &self,
        apply: impl FnOnce(&Transaction<'_>) -> Result<T, DatabaseError>,
    ) -> Result<T, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result = apply(&tx)?;
        #[cfg(test)]
        if std::env::var("TIKTOOLS_BEHAVIOR_CRASH").as_deref() == Ok("before") {
            std::process::exit(91);
        }
        tx.commit()?;
        #[cfg(test)]
        if std::env::var("TIKTOOLS_BEHAVIOR_CRASH").as_deref() == Ok("after") {
            std::process::exit(92);
        }
        Ok(result)
    }
    #[cfg(test)]
    pub(crate) fn save_behavior(&self, table: &str, value: &Value) -> Result<Value, DatabaseError> {
        self.save_behavior_versioned(table, value, None, BehaviorSave::Upsert)
            .map(|v| v.value)
    }
    pub(crate) fn save_behavior_versioned(
        &self,
        table: &str,
        value: &Value,
        expected: Option<&str>,
        mode: BehaviorSave,
    ) -> Result<Versioned<Value>, DatabaseError> {
        self.behavior_transaction(|tx| save_record(tx, table, value, expected, mode))
    }
    pub(crate) fn delete_behavior_versioned(
        &self,
        table: &str,
        id: &str,
        expected: Option<&str>,
    ) -> Result<Versioned<bool>, DatabaseError> {
        self.behavior_transaction(|tx| delete_record(tx, table, id, expected))
    }
    pub(crate) fn set_behavior_enabled_versioned(
        &self,
        table: &str,
        id: &str,
        enabled: bool,
        expected: Option<&str>,
    ) -> Result<Versioned<Value>, DatabaseError> {
        self.behavior_transaction(|tx| set_enabled(tx, table, id, enabled, expected))
    }
    /// Both families and every version/outbox reservation commit together.
    pub(crate) fn set_behavior_enablement(
        &self,
        updates: &[(&str, &str, bool)],
    ) -> Result<Versioned<Vec<(String, String)>>, DatabaseError> {
        for (table, _, _) in updates {
            versions::scope(table)?;
        }
        self.behavior_transaction(|tx| {
            let mut missing = Vec::new();
            let mut meta = std::collections::BTreeMap::new();
            for (table, id, enabled) in updates {
                let previous = versions::check(tx, table, id, None)?;
                if toggle(tx, table, id, *enabled)?.is_none() {
                    missing.push((versions::scope(table)?.into(), (*id).into()));
                    continue;
                }
                let v = versions::advance(
                    tx,
                    table,
                    id,
                    previous,
                    false,
                    if *enabled { "enabled" } else { "disabled" },
                    now(),
                )?;
                meta.insert((v.scope.clone(), v.id.clone()), v);
            }
            Ok(Versioned::new(missing, meta.into_values().collect()))
        })
    }
    pub(crate) fn set_plugin_state(
        &self,
        id: &str,
        installed: bool,
        enabled: bool,
    ) -> Result<Value, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        connection.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        advance_plugin_source(&tx, id)?;
        tx.execute(
            "INSERT INTO behavior_plugins (id, installed, enabled, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET installed = excluded.installed,
               enabled = excluded.enabled, updated_at = excluded.updated_at",
            params![id, bool_int(installed), bool_int(enabled), now()],
        )?;
        tx.commit()?;
        Ok(json!({"id": id, "installed": installed, "enabled": enabled}))
    }
    pub(crate) fn remove_plugin_state(&self, id: &str) -> Result<(), DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        connection.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        advance_plugin_source(&tx, id)?;
        tx.execute("DELETE FROM behavior_plugins WHERE id = ?", [id])?;
        tx.commit()?;
        Ok(())
    }

    /// Fence any prior sync review before a package's files can change. A
    /// failed install remains a fresh source decision and keeps the fence.
    #[cfg(feature = "plugin-install")]
    pub(crate) fn invalidate_plugin_package_source(&self, id: &str) -> Result<i64, DatabaseError> {
        if !tiktools_plugin_api::manifest::is_valid_plugin_id(id) {
            return Err(DatabaseError::Invalid("invalid plugin package id".into()));
        }
        let mut connection = self.open(&self.automation_path())?;
        connection.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let generation = advance_plugin_source(&tx, id)?;
        tx.commit()?;
        Ok(generation)
    }

    /// Fence the end of package removal and delete the prior state only when
    /// no newer source writer won. Even on mismatch the final fence commits;
    /// the concurrent writer's persisted state remains untouched.
    #[cfg(feature = "plugin-install")]
    pub(crate) fn remove_plugin_state_after_package(
        &self,
        id: &str,
        generation: i64,
    ) -> Result<bool, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        connection.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT generation FROM plugin_sync_source_versions WHERE plugin_id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()?;
        if current.is_none() || generation <= 0 {
            return Err(DatabaseError::Invalid(
                "invalid package source reservation".into(),
            ));
        }
        advance_plugin_source(&tx, id)?;
        let matched = current == Some(generation);
        if matched {
            tx.execute("DELETE FROM behavior_plugins WHERE id=?1", [id])?;
        }
        tx.commit()?;
        Ok(matched)
    }
    fn load_plugin_states_from(connection: &Connection) -> Result<Vec<Value>, DatabaseError> {
        let mut statement = connection
            .prepare("SELECT id, installed, enabled FROM behavior_plugins ORDER BY id ASC")?;
        let rows = statement.query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "installed": row.get::<_, i64>(1)? != 0,
                "enabled": row.get::<_, i64>(2)? != 0
            }))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

fn action_references(value: &Value, action_id: &str) -> bool {
    value.as_str() == Some(action_id)
}

/// Bounded native acknowledgement planning; the caller retains the host writer
/// gate between this coherent read and the actual business transaction.
pub(super) fn owner_definition_dispatch_preflight(
    connection: &Connection,
    operation: &crate::control::AutomationMutation,
    expected: &str,
    versions_budget: usize,
) -> Result<(), DatabaseError> {
    use crate::control::{AutomationKind, AutomationMutation};
    match operation {
        AutomationMutation::SetEnabled { kind, id, enabled } => {
            versions::check(connection, kind.table(), id, Some(expected))?;
            let mut value =
                versions::row(connection, kind.table(), id)?.ok_or(DatabaseError::NotFound)?;
            value["enabled"] = (*enabled).into();
            super::owner_definition::bounded_record(&value)
                .map_err(|_| DatabaseError::SnapshotTooLarge)?;
        }
        AutomationMutation::Delete {
            kind: AutomationKind::Action,
            id,
        } => {
            let previous = versions::check(connection, "behavior_actions", id, Some(expected))?;
            let exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM behavior_actions WHERE id=?1)",
                [id],
                |row| row.get(0),
            )?;
            let after = if exists {
                previous
                    .checked_add(1)
                    .ok_or_else(|| DatabaseError::Invalid("automation revision exhausted".into()))?
            } else {
                previous
            };
            let primary = versions::version("behavior_actions", id, after)?;
            // Array brackets plus exactly the serialized primary version.
            let mut bytes = 2usize.saturating_add(serde_json::to_vec(&primary)?.len());
            if bytes > versions_budget {
                return Err(DatabaseError::SnapshotTooLarge);
            }
            if !exists {
                return Ok(());
            }
            let mut statement = connection.prepare("SELECT id,payload_json FROM behavior_events WHERE payload_json IS NOT NULL ORDER BY id")?;
            let mut rows = statement.query([])?;
            while let Some(row) = rows.next()? {
                let payload: String = row.get(1)?;
                let value: Value = serde_json::from_str(&payload)?;
                let object = value.as_object().ok_or_else(|| {
                    DatabaseError::Invalid("behavior event is not an object".into())
                })?;
                if !object
                    .get("actionIds")
                    .and_then(Value::as_array)
                    .is_some_and(|ids| ids.iter().any(|value| action_references(value, id)))
                {
                    continue;
                }
                match row.get_ref(0)? {
                    rusqlite::types::ValueRef::Text(bytes) if bytes.len() <= 128 => {}
                    _ => return Err(DatabaseError::SnapshotTooLarge),
                }
                let event_id: String = row.get(0)?;
                // A committed native acknowledgement must also be addressable
                // by the strict clients that consume its resource versions.
                if event_id.is_empty()
                    || event_id.len() > 128
                    || event_id.trim() != event_id
                    || event_id.contains('*')
                    || event_id.chars().any(char::is_control)
                {
                    return Err(DatabaseError::SnapshotTooLarge);
                }
                let before = versions::check(connection, "behavior_events", &event_id, None)?;
                let next = before.checked_add(1).ok_or_else(|| {
                    DatabaseError::Invalid("automation revision exhausted".into())
                })?;
                let version = versions::version("behavior_events", &event_id, next)?;
                // Every additional element contributes one comma and its exact
                // JSON bytes. Never materialize the complete version vector.
                bytes = bytes
                    .saturating_add(1)
                    .saturating_add(serde_json::to_vec(&version)?.len());
                if bytes > versions_budget {
                    return Err(DatabaseError::SnapshotTooLarge);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn remove_action_references(
    connection: &Connection,
    action_id: &str,
) -> Result<Vec<ResourceVersion>, DatabaseError> {
    let mut statement = connection
        .prepare("SELECT id, name, enabled, payload_json FROM behavior_events WHERE payload_json IS NOT NULL ORDER BY id")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, bool>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut updates = Vec::new();
    for row in rows {
        let (id, name, enabled, payload) = row?;
        let mut object = serde_json::from_str::<Value>(&payload)?
            .as_object()
            .cloned()
            .ok_or_else(|| {
                DatabaseError::Invalid(format!("behavior_events/{id} is not an object"))
            })?;
        let Some(action_ids) = object.get_mut("actionIds").and_then(Value::as_array_mut) else {
            continue;
        };
        let original = action_ids.len();
        action_ids.retain(|value| !action_references(value, action_id));
        if action_ids.len() != original {
            object.insert("id".into(), id.clone().into());
            object.insert("name".into(), name.into());
            object.insert("enabled".into(), enabled.into());
            updates.push((id, Value::Object(object)));
        }
    }
    drop(statement);
    let mut metadata = Vec::with_capacity(updates.len());
    for (id, value) in updates {
        let object = value
            .as_object()
            .ok_or_else(|| DatabaseError::Invalid("event is not an object".to_owned()))?;
        let name = required_value_string(object, "name")?;
        let enabled = object
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let previous = versions::check(connection, "behavior_events", &id, None)?;
        connection.execute(
            "UPDATE behavior_events SET name = ?, enabled = ?, payload_json = ?, updated_at = ? WHERE id = ?",
            params![name, bool_int(enabled), serde_json::to_string(&value)?, now(), id],
        )?;
        metadata.push(versions::advance(
            connection,
            "behavior_events",
            &id,
            previous,
            false,
            "references-updated",
            now(),
        )?);
    }
    Ok(metadata)
}

fn toggle(
    db: &Connection,
    table: &str,
    id: &str,
    enabled: bool,
) -> Result<Option<Value>, DatabaseError> {
    let Some(mut value) = versions::row(db, table, id)? else {
        return Ok(None);
    };
    value["enabled"] = enabled.into();
    db.execute(
        &format!("UPDATE {table} SET enabled=?1,payload_json=?2,updated_at=?3 WHERE id=?4"),
        params![bool_int(enabled), serde_json::to_string(&value)?, now(), id],
    )?;
    Ok(Some(value))
}

fn save_record(
    tx: &Transaction<'_>,
    table: &str,
    value: &Value,
    expected: Option<&str>,
    mode: BehaviorSave,
) -> Result<Versioned<Value>, DatabaseError> {
    versions::scope(table)?;
    let object = value
        .as_object()
        .ok_or_else(|| DatabaseError::Invalid("behavior record must be an object".into()))?;
    let id = required_value_string(object, "id")?;
    let name = required_value_string(object, "name")?;
    let enabled = object
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let previous = versions::check(tx, table, &id, expected)?;
    let created: Option<i64> = tx
        .query_row(
            &format!("SELECT created_at FROM {table} WHERE id=?1"),
            [&id],
            |r| r.get(0),
        )
        .optional()?;
    if matches!(mode, BehaviorSave::Create) && created.is_some() {
        return Err(DatabaseError::RevisionConflict(versions::version(
            table, &id, previous,
        )?));
    }
    if matches!(mode, BehaviorSave::Update) && created.is_none() {
        return Err(DatabaseError::NotFound);
    }
    let at = now();
    tx.execute(
        &format!(
            "INSERT INTO {table}(id,name,enabled,payload_json,created_at,updated_at)
            VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET
            name=excluded.name,enabled=excluded.enabled,
            payload_json=excluded.payload_json,updated_at=excluded.updated_at"
        ),
        params![
            id,
            name,
            bool_int(enabled),
            serde_json::to_string(value)?,
            created.unwrap_or(at),
            at
        ],
    )?;
    let version = versions::advance(tx, table, &id, previous, false, "saved", at)?;
    Ok(Versioned::new(value.clone(), vec![version]))
}

fn delete_record(
    tx: &Transaction<'_>,
    table: &str,
    id: &str,
    expected: Option<&str>,
) -> Result<Versioned<bool>, DatabaseError> {
    versions::scope(table)?;
    let previous = versions::check(tx, table, id, expected)?;
    let removed = tx.execute(&format!("DELETE FROM {table} WHERE id=?1"), [id])? > 0;
    let cascaded = if removed && table == "behavior_actions" {
        remove_action_references(tx, id)?
    } else {
        Vec::new()
    };
    let version = if removed {
        versions::advance(tx, table, id, previous, true, "deleted", now())?
    } else {
        versions::version(table, id, previous)?
    };
    let mut metadata = vec![version];
    metadata.extend(cascaded);
    Ok(Versioned::new(removed, metadata))
}

fn set_enabled(
    tx: &Transaction<'_>,
    table: &str,
    id: &str,
    enabled: bool,
    expected: Option<&str>,
) -> Result<Versioned<Value>, DatabaseError> {
    versions::scope(table)?;
    let previous = versions::check(tx, table, id, expected)?;
    let value = toggle(tx, table, id, enabled)?.ok_or(DatabaseError::NotFound)?;
    let version = versions::advance(
        tx,
        table,
        id,
        previous,
        false,
        if enabled { "enabled" } else { "disabled" },
        now(),
    )?;
    Ok(Versioned::new(value, vec![version]))
}
