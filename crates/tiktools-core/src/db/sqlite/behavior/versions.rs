use super::*;
use crate::collaboration::{ResourceKind, ResourceVersion, Versioned};

pub(super) fn scope(table: &str) -> Result<&'static str, DatabaseError> {
    match table {
        "behavior_actions" => Ok("action"),
        "behavior_events" => Ok("event"),
        _ => Err(DatabaseError::Invalid("unknown behavior family".into())),
    }
}
pub(super) fn version(
    table: &str,
    id: &str,
    revision: i64,
) -> Result<ResourceVersion, DatabaseError> {
    Ok(ResourceVersion {
        kind: ResourceKind::Automation,
        scope: scope(table)?.into(),
        id: id.into(),
        revision: revision.to_string(),
    })
}
pub(super) fn current(db: &Connection, table: &str, id: &str) -> Result<i64, DatabaseError> {
    Ok(db.query_row("SELECT revision FROM control_resource_versions WHERE kind='automation' AND scope=?1 AND resource_id=?2",params![scope(table)?,id],|r|r.get(0)).optional()?.unwrap_or(0))
}
pub(super) fn check(
    db: &Connection,
    table: &str,
    id: &str,
    expected: Option<&str>,
) -> Result<i64, DatabaseError> {
    let previous = current(db, table, id)?;
    // A live preexisting row cannot be a never-created resource. Only schema
    // migration may seed legacy metadata, never a later ordinary owner write.
    if previous == 0 {
        let exists: bool = db.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
            [id],
            |r| r.get(0),
        )?;
        if exists {
            return Err(DatabaseError::Invalid(
                "behavior version history is missing".into(),
            ));
        }
    }
    if let Some(expected) = expected {
        let parsed = expected
            .parse::<i64>()
            .ok()
            .filter(|n| *n >= 0 && n.to_string() == expected)
            .ok_or_else(|| DatabaseError::Invalid("invalid automation expectedRevision".into()))?;
        if parsed != previous {
            return Err(DatabaseError::RevisionConflict(version(
                table, id, previous,
            )?));
        }
    }
    Ok(previous)
}
pub(super) fn advance(
    db: &Connection,
    table: &str,
    id: &str,
    previous: i64,
    deleted: bool,
    change: &str,
    at: i64,
) -> Result<ResourceVersion, DatabaseError> {
    let next = previous
        .checked_add(1)
        .ok_or_else(|| DatabaseError::Invalid("automation revision exhausted".into()))?;
    provenance::advance(db, table, id, previous)?;
    super::approvals::invalidate(db, scope(table)?, id, at)?;
    db.execute("INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted) VALUES ('automation',?1,?2,?3,?4) ON CONFLICT(kind,scope,resource_id) DO UPDATE SET revision=excluded.revision,deleted=excluded.deleted",params![scope(table)?,id,next,bool_int(deleted)])?;
    db.execute("INSERT INTO control_resource_outbox(kind,scope,resource_id,revision,change,committed_at) VALUES ('automation',?1,?2,?3,?4,?5)",params![scope(table)?,id,next,change,at])?;
    version(table, id, next)
}
pub(super) fn canonical(
    id: String,
    name: String,
    enabled: bool,
    payload: String,
) -> Result<Value, DatabaseError> {
    let mut value: Value = serde_json::from_str(&payload)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| DatabaseError::Invalid("behavior payload must be an object".into()))?;
    object.insert("id".into(), id.into());
    object.insert("name".into(), name.into());
    object.insert("enabled".into(), enabled.into());
    Ok(value)
}
pub(super) fn row(db: &Connection, table: &str, id: &str) -> Result<Option<Value>, DatabaseError> {
    scope(table)?;
    let value: Option<(String, String, bool, String)> = db
        .query_row(
            &format!("SELECT id,name,enabled,payload_json FROM {table} WHERE id=?1"),
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    value
        .map(|(id, name, enabled, payload)| canonical(id, name, enabled, payload))
        .transpose()
}
pub(super) fn rows(db: &Connection, table: &str) -> Result<Versioned<Vec<Value>>, DatabaseError> {
    let mut stmt=db.prepare(&format!("SELECT b.id,b.name,b.enabled,b.payload_json,v.revision FROM {table} b JOIN control_resource_versions v ON v.kind='automation' AND v.scope=?1 AND v.resource_id=b.id AND v.deleted=0 ORDER BY b.updated_at DESC,b.name ASC,b.id ASC"))?;
    let mut query = stmt.query([scope(table)?])?;
    let mut values = Vec::new();
    let mut versions = Vec::new();
    while let Some(row) = query.next()? {
        let id: String = row.get(0)?;
        let revision: i64 = row.get(4)?;
        if revision <= 0 {
            return Err(DatabaseError::Invalid(
                "automation version is missing".into(),
            ));
        }
        versions.push(version(table, &id, revision)?);
        values.push(canonical(id, row.get(1)?, row.get(2)?, row.get(3)?)?);
    }
    Ok(Versioned::new(values, versions))
}
