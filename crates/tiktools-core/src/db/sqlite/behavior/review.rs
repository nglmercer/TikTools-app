//! Owner review reads the selected definition and every direct action dependency
//! from one read snapshot. It does not install or repair runtime authority.
use super::*;
use std::collections::HashSet;

const DEFINITION_BYTES: usize = 256 * 1024;
const DEPENDENCIES: usize = 512;
const REVIEW_BYTES: usize = crate::collaboration::SNAPSHOT_BYTES;

fn canonical_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.trim() == id
        && !id.contains('*')
        && !id.chars().any(char::is_control)
}

fn invalid() -> DatabaseError {
    DatabaseError::Invalid("behavior review identity or retained history is inconsistent".into())
}

fn provenance_value(p: Option<BehaviorProvenance>) -> Value {
    p.map(|p| json!({"authority":p.authority,"generation":p.generation.to_string()}))
        .unwrap_or(Value::Null)
}

fn record(
    db: &Connection,
    table: &str,
    id: &str,
    used: &mut usize,
) -> Result<(Option<Value>, Value, ResourceVersion), DatabaseError> {
    if !canonical_id(id) {
        return Err(invalid());
    }
    let scope = versions::scope(table)?;
    // Inspect lengths before materializing arbitrary owner definition text.
    let size: Option<i64> = db
        .query_row(
            &format!("SELECT length(CAST(id AS BLOB))+length(CAST(name AS BLOB))+length(CAST(payload_json AS BLOB)) FROM {table} WHERE id=?1"),
            [id],
            |r| r.get(0),
        )
        .optional()?;
    if size.is_some_and(|bytes| {
        bytes < 0
            || bytes as u64 > DEFINITION_BYTES as u64
            || bytes as u64 > REVIEW_BYTES.saturating_sub(*used) as u64
    }) {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    let authority_size: Option<i64> = db
        .query_row(
            "SELECT length(CAST(authority AS BLOB))+coalesce(length(CAST(principal_id AS BLOB)),0)+coalesce(length(CAST(session_id AS BLOB)),0) FROM behavior_provenance WHERE scope=?1 AND resource_id=?2",
            params![scope,id],
            |r| r.get(0),
        )
        .optional()?;
    if authority_size.is_some_and(|bytes| !(0..=1024).contains(&bytes)) {
        return Err(invalid());
    }
    let version: Option<(i64, i64)> = db
        .query_row(
            "SELECT revision,deleted FROM control_resource_versions WHERE kind='automation' AND scope=?1 AND resource_id=?2",
            params![scope,id],
            |r| Ok((r.get(0)?,r.get(1)?)),
        )
        .optional()?;
    let provenance = provenance::get(db, table, id)?;
    let revision = match (size.is_some(), version, provenance.is_some()) {
        (false, None, false) => 0,
        (true, Some((revision, 0)), true) | (false, Some((revision, 1)), true) if revision > 0 => {
            revision
        }
        _ => return Err(invalid()),
    };
    let record = if size.is_some() {
        versions::row(db, table, id)?
    } else {
        None
    };
    let definition_bytes = serde_json::to_vec(&record)?.len();
    if definition_bytes > DEFINITION_BYTES {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    let provenance = provenance_value(provenance);
    let version = versions::version(table, id, revision)?;
    let provenance_bytes = serde_json::to_vec(&provenance)?.len();
    let version_bytes = serde_json::to_vec(&version)?.len();
    *used = used
        .checked_add(definition_bytes)
        .and_then(|n| n.checked_add(provenance_bytes))
        .and_then(|n| n.checked_add(version_bytes))
        .and_then(|n| n.checked_add(id.len() + 64))
        .filter(|n| *n <= REVIEW_BYTES)
        .ok_or(DatabaseError::SnapshotTooLarge)?;
    Ok((record, provenance, version))
}

pub(super) fn snapshot(
    db: &Connection,
    table: &str,
    id: &str,
) -> Result<Versioned<Value>, DatabaseError> {
    let mut used = 0;
    let (primary, provenance, version) = record(db, table, id, &mut used)?;
    let mut ids = Vec::new();
    if table == "behavior_events" {
        if let Some(raw) = primary.as_ref().and_then(|r| r.get("actionIds")) {
            let raw = raw.as_array().ok_or_else(invalid)?;
            let mut seen = HashSet::new();
            for value in raw {
                let id = value
                    .as_str()
                    .filter(|id| canonical_id(id))
                    .ok_or_else(invalid)?;
                if seen.insert(id) {
                    if ids.len() == DEPENDENCIES {
                        return Err(DatabaseError::SnapshotTooLarge);
                    }
                    ids.push(id.to_owned());
                }
            }
        }
    }
    let mut dependencies = Vec::with_capacity(ids.len());
    let mut resource_versions = vec![version];
    for id in ids {
        let (record, provenance, version) = record(db, "behavior_actions", &id, &mut used)?;
        dependencies.push(json!({"id":id,"record":record,"provenance":provenance}));
        resource_versions.push(version);
    }
    let result = Versioned::new(
        json!({"kind":versions::scope(table)?,"id":id,"record":primary,"provenance":provenance,"dependencies":dependencies}),
        resource_versions,
    );
    if serde_json::to_vec(&result)?.len() > REVIEW_BYTES {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    Ok(result)
}

/// The reviewed dependency fence shares the writer reservation with transfer.
pub(super) fn check_versions(
    db: &Connection,
    table: &str,
    id: &str,
    expected: &str,
    reviewed: &[ResourceVersion],
) -> Result<(), DatabaseError> {
    use crate::collaboration::ResourceKind;
    if reviewed.is_empty() || reviewed.len() > DEPENDENCIES + 1 {
        return Err(invalid());
    }
    let mut seen = HashSet::new();
    for (index, version) in reviewed.iter().enumerate() {
        if version.kind != ResourceKind::Automation
            || !canonical_id(&version.id)
            || version
                .revision
                .parse::<i64>()
                .ok()
                .filter(|n| *n >= 0 && n.to_string() == version.revision)
                .is_none()
            || !seen.insert((version.scope.as_str(), version.id.as_str()))
            || if index == 0 {
                version.scope != versions::scope(table)?
                    || version.id != id
                    || version.revision != expected
            } else {
                table != "behavior_events" || version.scope != "action"
            }
        {
            return Err(invalid());
        }
    }
    versions::check(db, table, id, Some(expected))?;
    let current = snapshot(db, table, id)?.resource_versions;
    if reviewed.len() != current.len()
        || reviewed
            .iter()
            .zip(&current)
            .any(|(before, after)| before.scope != after.scope || before.id != after.id)
    {
        return Err(invalid());
    }
    for (before, after) in reviewed.iter().zip(current) {
        if before.revision != after.revision {
            return Err(DatabaseError::RevisionConflict(after));
        }
    }
    Ok(())
}

impl DatabaseManager {
    pub(crate) fn behavior_review_versioned(
        &self,
        table: &str,
        id: &str,
    ) -> Result<Versioned<Value>, DatabaseError> {
        versions::scope(table)?;
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let result = snapshot(&tx, table, id)?;
        tx.commit()?;
        Ok(result)
    }
}
