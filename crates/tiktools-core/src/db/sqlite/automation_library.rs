//! Canonical profile/template metadata is colocated with behavior records.
//! Legacy PointsDB keys are read once; no transaction writes both databases.
use super::{DatabaseError, DatabaseManager};
use crate::{
    collaboration::{ResourceKind, ResourceVersion, Versioned},
    control::{
        owner_library::{library_profile_id, library_record_id},
        OwnerLibraryCatalog, OwnerLibraryPack, OWNER_LIBRARY_MAX_BYTES,
        OWNER_LIBRARY_MAX_CATALOG_BYTES,
    },
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const PACKS_KEY: &str = "behavior.profiles.packs";
pub(super) const ACTIVE_KEY: &str = "behavior.profiles.active";
pub(super) const TEMPLATES_KEY: &str = "behavior.templates.custom";
pub(super) const LIBRARY_METHOD: &str = "automation.library.apply";
pub(super) const RESULT_VERSIONS_BYTES: usize = OWNER_LIBRARY_MAX_BYTES - 256 * 1024;
pub(super) const SNAPSHOT_BYTES: usize = OWNER_LIBRARY_MAX_BYTES - 8 * 1024;

fn invalid() -> DatabaseError {
    DatabaseError::Invalid("automation library catalog is invalid".into())
}
pub(super) fn reserved_key(key: &str) -> bool {
    matches!(key, PACKS_KEY | ACTIVE_KEY | TEMPLATES_KEY)
}
pub(super) fn source_budget(value: &Value, byte_limit: usize) -> Result<(), DatabaseError> {
    let mut nodes = 0usize;
    let mut pending = vec![(value, 0usize)];
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if depth > 64 || nodes > 65_536 {
            return Err(DatabaseError::SnapshotTooLarge);
        }
        let children: Box<dyn Iterator<Item = &Value> + '_> = match value {
            Value::Object(values) => Box::new(values.values()),
            Value::Array(values) => Box::new(values.iter()),
            _ => continue,
        };
        for child in children {
            if nodes + pending.len() >= 65_536 {
                return Err(DatabaseError::SnapshotTooLarge);
            }
            pending.push((child, depth + 1));
        }
    }
    if serde_json::to_vec(value)?.len() > byte_limit {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    Ok(())
}
pub(super) fn validate_catalog(catalog: &OwnerLibraryCatalog) -> Result<(), DatabaseError> {
    let mut profiles = BTreeSet::new();
    for pack in &catalog.packs {
        if !library_profile_id(&pack.id)
            || pack.id == "default"
            || !profiles.insert(pack.id.as_str())
            || pack.name.trim().is_empty()
            || pack.name.chars().count() > 80
            || pack.description.chars().count() > 500
        {
            return Err(invalid());
        }
        for ids in [&pack.event_ids, &pack.action_ids] {
            let mut seen = BTreeSet::new();
            if ids
                .iter()
                .any(|id| !library_record_id(id) || !seen.insert(id.as_str()))
            {
                return Err(invalid());
            }
        }
    }
    if catalog.active_id != "default" && !profiles.contains(catalog.active_id.as_str()) {
        return Err(invalid());
    }
    let mut templates = BTreeSet::new();
    for value in &catalog.templates {
        source_budget(value, OWNER_LIBRARY_MAX_CATALOG_BYTES)?;
        let parsed =
            crate::services::templates::parse_rule_template(value).map_err(|_| invalid())?;
        let id = value
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        if parsed.id != id || !templates.insert(id) {
            return Err(invalid());
        }
    }
    if serde_json::to_vec(catalog)?.len() > OWNER_LIBRARY_MAX_CATALOG_BYTES {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    Ok(())
}
fn decode_catalog(raw: &str) -> Result<OwnerLibraryCatalog, DatabaseError> {
    if raw.len() > OWNER_LIBRARY_MAX_CATALOG_BYTES {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    let catalog: OwnerLibraryCatalog = serde_json::from_str(raw)?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}
fn legacy_catalog(values: &BTreeMap<String, String>) -> Result<OwnerLibraryCatalog, DatabaseError> {
    let nonempty = |key| {
        values
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.trim().is_empty())
    };
    let packs: Vec<OwnerLibraryPack> = nonempty(PACKS_KEY)
        .map(serde_json::from_str)
        .transpose()?
        .unwrap_or_default();
    let templates = if let Some(raw) = nonempty(TEMPLATES_KEY) {
        match serde_json::from_str::<Value>(raw)? {
            Value::Array(values) => values,
            value @ Value::Object(_) if value.get("templateVersion").is_some() => vec![value],
            Value::Object(mut object) if object.len() == 1 && object.contains_key("templates") => {
                object
                    .remove("templates")
                    .and_then(|value| value.as_array().cloned())
                    .ok_or_else(invalid)?
            }
            _ => return Err(invalid()),
        }
    } else {
        Vec::new()
    };
    let catalog = OwnerLibraryCatalog {
        packs,
        active_id: nonempty(ACTIVE_KEY).unwrap_or("default").trim().into(),
        templates,
    };
    validate_catalog(&catalog)?;
    Ok(catalog)
}
fn legacy_values(database: &DatabaseManager) -> Result<BTreeMap<String, String>, DatabaseError> {
    let mut db = database.open(&database.points_path())?;
    let tx = db.transaction()?;
    let mut result = BTreeMap::new();
    {
        let mut statement = tx.prepare("SELECT key,CASE WHEN length(CAST(value AS BLOB))<=4194304 THEN value ELSE NULL END FROM app_state WHERE key IN ('behavior.profiles.packs','behavior.profiles.active','behavior.templates.custom')")?;
        let mut rows = statement.query([])?;
        let mut bytes = 0usize;
        while let Some(row) = rows.next()? {
            let raw: Option<String> = row.get(1)?;
            let raw = raw.ok_or(DatabaseError::SnapshotTooLarge)?;
            bytes = bytes.saturating_add(raw.len());
            if bytes > OWNER_LIBRARY_MAX_CATALOG_BYTES {
                return Err(DatabaseError::SnapshotTooLarge);
            }
            result.insert(row.get(0)?, raw);
        }
    }
    tx.commit()?;
    Ok(result)
}
fn initialized(db: &Connection) -> Result<bool, DatabaseError> {
    let marker: Option<i64> = db
        .query_row(
            "SELECT version FROM automation_library_migration WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM automation_library_catalog WHERE id=1)",
        [],
        |row| row.get(0),
    )?;
    match (marker, exists) {
        (None, false) => Ok(false),
        (Some(1), true) => Ok(true),
        _ => Err(invalid()),
    }
}
pub(super) fn catalog_on(db: &Connection) -> Result<(OwnerLibraryCatalog, i64), DatabaseError> {
    if !initialized(db)? {
        return Err(invalid());
    }
    let (revision,raw):(i64,Option<String>) = db.query_row("SELECT revision,CASE WHEN length(CAST(catalog_json AS BLOB))<=4194304 THEN catalog_json ELSE NULL END FROM automation_library_catalog WHERE id=1",[],|row|Ok((row.get(0)?,row.get(1)?)))?;
    if revision < 0 {
        return Err(invalid());
    }
    validate_runtime_version(db)?;
    Ok((
        decode_catalog(&raw.ok_or(DatabaseError::SnapshotTooLarge)?)?,
        revision,
    ))
}
/// This scope carries no behavior authority. It must match its own canonical
/// singleton instead of acquiring fictitious Owner provenance.
pub(super) fn validate_runtime_version(db: &Connection) -> Result<(), DatabaseError> {
    let catalog: Option<i64> = db
        .query_row(
            "SELECT revision FROM automation_library_catalog WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let version: Option<(i64,i64)> = db.query_row("SELECT revision,deleted FROM control_resource_versions WHERE kind='automation' AND scope='library' AND resource_id='catalog'",[],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
    let marker: Option<i64> = db
        .query_row(
            "SELECT version FROM automation_library_migration WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match (catalog, version, marker) {
        (None, None, None) | (Some(0), None, Some(1)) => Ok(()),
        (Some(catalog), Some((version, 0)), Some(1)) if catalog > 0 && catalog == version => Ok(()),
        _ => Err(invalid()),
    }
}
pub(super) fn write_catalog_on(
    db: &Connection,
    catalog: &OwnerLibraryCatalog,
    before: i64,
) -> Result<ResourceVersion, DatabaseError> {
    validate_catalog(catalog)?;
    let next = before.checked_add(1).ok_or_else(invalid)?;
    let at = super::util::now();
    if db.execute("UPDATE automation_library_catalog SET revision=?2,catalog_json=?3,updated_at=?4 WHERE id=1 AND revision=?1",params![before,next,serde_json::to_string(catalog)?,at])? != 1 { return Err(invalid()); }
    db.execute("INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted) VALUES ('automation','library','catalog',?1,0) ON CONFLICT(kind,scope,resource_id) DO UPDATE SET revision=excluded.revision,deleted=0",[next])?;
    db.execute("INSERT INTO control_resource_outbox(kind,scope,resource_id,revision,change,committed_at) VALUES ('automation','library','catalog',?1,'library-updated',?2)",params![next,at])?;
    Ok(ResourceVersion {
        kind: ResourceKind::Automation,
        scope: "library".into(),
        id: "catalog".into(),
        revision: next.to_string(),
    })
}
fn projection(catalog: &OwnerLibraryCatalog, key: &str) -> Result<String, DatabaseError> {
    Ok(match key {
        PACKS_KEY => serde_json::to_string(&catalog.packs)?,
        ACTIVE_KEY => catalog.active_id.clone(),
        TEMPLATES_KEY => serde_json::to_string(&catalog.templates)?,
        _ => return Err(invalid()),
    })
}
impl DatabaseManager {
    pub(crate) fn ensure_automation_library(&self) -> Result<(), DatabaseError> {
        {
            let mut db = self.open(&self.automation_path())?;
            let tx = db.transaction()?;
            let ready = initialized(&tx)?;
            validate_runtime_version(&tx)?;
            tx.commit()?;
            if ready {
                return Ok(());
            }
        }
        // Close the read snapshot before taking the other database's writer.
        // The source remains untouched, so either side of a crash can retry.
        let values = legacy_values(self)?;
        let catalog = legacy_catalog(&values)?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !initialized(&tx)? {
            let version: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM control_resource_versions WHERE kind='automation' AND scope='library')",[],|row|row.get(0))?;
            if version {
                return Err(invalid());
            }
            tx.execute("INSERT INTO automation_library_catalog(id,revision,catalog_json,updated_at) VALUES (1,0,?1,?2)",params![serde_json::to_string(&catalog)?,super::util::now()])?;
            tx.execute(
                "INSERT INTO automation_library_migration(id,version) VALUES (1,1)",
                [],
            )?;
        }
        catalog_on(&tx)?;
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn automation_library_exists(&self) -> Result<bool, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let result = initialized(&tx)?;
        validate_runtime_version(&tx)?;
        tx.commit()?;
        Ok(result)
    }
    pub(crate) fn automation_library_legacy_exists(&self) -> Result<bool, DatabaseError> {
        let db = self.open(&self.points_path())?;
        Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM app_state WHERE key IN ('behavior.profiles.packs','behavior.profiles.active','behavior.templates.custom'))", [], |row|row.get(0))?)
    }
    pub(crate) fn automation_library_legacy_state(
        &self,
    ) -> Result<BTreeMap<String, String>, DatabaseError> {
        self.ensure_automation_library()?;
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let (catalog, _) = catalog_on(&tx)?;
        let mut values = BTreeMap::new();
        for key in [PACKS_KEY, ACTIVE_KEY, TEMPLATES_KEY] {
            values.insert(key.to_owned(), projection(&catalog, key)?);
        }
        tx.commit()?;
        Ok(values)
    }
    pub(crate) fn automation_library_snapshot(
        &self,
    ) -> Result<(OwnerLibraryCatalog, String, Versioned<Value>), DatabaseError> {
        self.ensure_automation_library()?;
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let (catalog, revision) = catalog_on(&tx)?;
        let definitions = super::behavior::library::snapshot_on(&tx)?;
        if serde_json::to_vec(&json!({"catalog":catalog,"revision":revision.to_string(),"definitions":definitions.value,"resourceVersions":definitions.resource_versions}))?.len() > SNAPSHOT_BYTES { return Err(DatabaseError::SnapshotTooLarge); }
        tx.commit()?;
        Ok((catalog, revision.to_string(), definitions))
    }
    #[cfg(test)]
    pub(crate) fn automation_library_legacy_get(
        &self,
        key: &str,
    ) -> Result<Option<String>, DatabaseError> {
        if !reserved_key(key) {
            return Ok(None);
        }
        self.ensure_automation_library()?;
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let (catalog, _) = catalog_on(&tx)?;
        let value = projection(&catalog, key)?;
        tx.commit()?;
        Ok(Some(value))
    }
    pub(crate) fn automation_library_legacy_set(
        &self,
        host: &str,
        key: &str,
        value: &str,
    ) -> Result<(), DatabaseError> {
        if !reserved_key(key) {
            return Err(invalid());
        }
        if value.len() > OWNER_LIBRARY_MAX_CATALOG_BYTES {
            return Err(DatabaseError::SnapshotTooLarge);
        }
        self.ensure_automation_library()?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if super::owner_adoption::has_pending(&tx, host)?
            || super::owner_approval::has_pending(&tx, host)?
            || super::owner_definition::has_pending(&tx, host)?
            || super::owner_library::has_pending(&tx, host)?
        {
            return Err(DatabaseError::OwnerAdoptionPending);
        }
        let (mut catalog, before) = catalog_on(&tx)?;
        match key {
            PACKS_KEY => {
                catalog.packs = serde_json::from_str(value)?;
                if catalog.active_id != "default"
                    && !catalog
                        .packs
                        .iter()
                        .any(|pack| pack.id == catalog.active_id)
                {
                    catalog.active_id = "default".into();
                }
            }
            ACTIVE_KEY => catalog.active_id = value.into(),
            TEMPLATES_KEY => catalog.templates = serde_json::from_str(value)?,
            _ => return Err(invalid()),
        }
        write_catalog_on(&tx, &catalog, before)?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
