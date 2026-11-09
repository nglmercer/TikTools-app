//! Host-owned authority. Editable behavior JSON is never an authority source.
use super::*;
use crate::collaboration::{Authenticated, Authorization, Capability, ResourceKind};
use serde::Serialize;

/// Trusted host data, intentionally not deserializable from a command body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum BehaviorAuthority {
    Owner,
    Delegated {
        #[serde(rename = "principalId")]
        principal_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BehaviorProvenance {
    pub authority: BehaviorAuthority,
    /// Changes to any definition, enablement, dependency list or authority
    /// invalidate an approval bound to the old generation.
    pub generation: i64,
}
/// The runtime receives authority and its retained resource state together.
/// Unlike editable records, this host-owned projection is not deserializable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BehaviorRuntimeAuthority {
    pub authority: BehaviorAuthority,
    pub generation: i64,
    pub revision: i64,
    pub deleted: bool,
}
fn valid_identity(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id.trim() == id
        && !id.contains('*')
        && !id.chars().any(char::is_control)
}
fn canonical_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.trim() == id
        && !id.contains('*')
        && !id.chars().any(char::is_control)
}
fn decode(
    kind: String,
    principal: Option<String>,
    session: Option<String>,
    generation: i64,
) -> Result<BehaviorProvenance, DatabaseError> {
    if generation <= 0 {
        return Err(DatabaseError::Invalid(
            "behavior authority generation is invalid".into(),
        ));
    }
    let authority = match (kind.as_str(), principal, session) {
        ("owner", None, None) => BehaviorAuthority::Owner,
        ("delegated", Some(principal_id), Some(session_id))
            if valid_identity(&principal_id) && valid_identity(&session_id) =>
        {
            BehaviorAuthority::Delegated {
                principal_id,
                session_id,
            }
        }
        _ => {
            return Err(DatabaseError::Invalid(
                "behavior authority is invalid".into(),
            ))
        }
    };
    Ok(BehaviorProvenance {
        authority,
        generation,
    })
}
pub(super) fn get(
    db: &Connection,
    table: &str,
    id: &str,
) -> Result<Option<BehaviorProvenance>, DatabaseError> {
    let raw:Option<(String,Option<String>,Option<String>,i64)>=db.query_row(
        "SELECT authority,principal_id,session_id,generation FROM behavior_provenance WHERE scope=?1 AND resource_id=?2",
        params![versions::scope(table)?,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    raw.map(|(kind, p, s, g)| decode(kind, p, s, g)).transpose()
}
/// Check both sides of the retained identity relation before an inner join can
/// hide an existing definition or tombstone as an apparently new inline ID.
pub(super) fn validate_runtime_catalog(db: &Connection) -> Result<(), DatabaseError> {
    for table in ["behavior_actions", "behavior_events"] {
        let scope = versions::scope(table)?;
        let invalid: bool = db.query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM {table} b LEFT JOIN control_resource_versions v ON v.kind='automation' AND v.scope=?1 AND v.resource_id=b.id WHERE v.revision IS NULL OR v.revision<=0 OR v.deleted<>0) OR EXISTS(SELECT 1 FROM control_resource_versions v LEFT JOIN {table} b ON b.id=v.resource_id WHERE v.kind='automation' AND v.scope=?1 AND ((v.deleted=0 AND b.id IS NULL) OR (v.deleted=1 AND b.id IS NOT NULL)))"
            ),
            [scope],
            |r| r.get(0),
        )?;
        if invalid {
            return Err(DatabaseError::Invalid(
                "behavior version history is inconsistent".into(),
            ));
        }
    }
    let invalid: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM control_resource_versions v LEFT JOIN behavior_provenance p ON p.scope=v.scope AND p.resource_id=v.resource_id WHERE v.kind='automation' AND NOT(v.scope='library' AND v.resource_id='catalog') AND (v.scope NOT IN ('action','event') OR v.revision<=0 OR v.deleted NOT IN (0,1) OR p.resource_id IS NULL))",
        [],
        |r| r.get(0),
    )?;
    if invalid {
        return Err(DatabaseError::Invalid(
            "behavior authority is missing or inconsistent".into(),
        ));
    }
    super::super::automation_library::validate_runtime_version(db)?;
    Ok(())
}
pub(super) fn rows(
    db: &Connection,
) -> Result<Vec<(String, String, BehaviorRuntimeAuthority)>, DatabaseError> {
    let mut stmt=db.prepare("SELECT p.scope,p.resource_id,p.authority,p.principal_id,p.session_id,p.generation,v.deleted,v.revision FROM behavior_provenance p LEFT JOIN control_resource_versions v ON v.kind='automation' AND v.scope=p.scope AND v.resource_id=p.resource_id ORDER BY p.scope,p.resource_id")?;
    let mut query = stmt.query([])?;
    let mut result = Vec::new();
    while let Some(row) = query.next()? {
        let scope: String = row.get(0)?;
        if !matches!(scope.as_str(), "action" | "event") {
            return Err(DatabaseError::Invalid(
                "unknown behavior authority family".into(),
            ));
        }
        let deleted: Option<i64> = row.get(6)?;
        if !matches!(deleted, Some(0 | 1)) {
            return Err(DatabaseError::Invalid(
                "behavior version history is missing".into(),
            ));
        }
        let revision: Option<i64> = row.get(7)?;
        let revision = revision
            .filter(|revision| *revision > 0)
            .ok_or_else(|| DatabaseError::Invalid("behavior version history is invalid".into()))?;
        let provenance = decode(row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)?;
        if deleted == Some(0) || matches!(provenance.authority, BehaviorAuthority::Delegated { .. })
        {
            result.push((
                scope,
                row.get(1)?,
                BehaviorRuntimeAuthority {
                    authority: provenance.authority,
                    generation: provenance.generation,
                    revision,
                    deleted: deleted == Some(1),
                },
            ));
        }
    }
    Ok(result)
}
/// Called by every behavior version transition in the same business TX. An
/// absent historical sidecar is corruption, never a reason to infer ownership.
pub(super) fn advance(
    db: &Connection,
    table: &str,
    id: &str,
    previous_revision: i64,
) -> Result<(), DatabaseError> {
    match get(db, table, id)? {
        Some(p) => {
            if previous_revision == 0 {
                return Err(DatabaseError::Invalid(
                    "behavior version history is missing".into(),
                ));
            }
            let generation = p.generation.checked_add(1).ok_or_else(|| {
                DatabaseError::Invalid("behavior authority generation exhausted".into())
            })?;
            db.execute(
                "UPDATE behavior_provenance SET generation=?3 WHERE scope=?1 AND resource_id=?2",
                params![versions::scope(table)?, id, generation],
            )?;
        }
        None if previous_revision == 0 => {
            db.execute("INSERT INTO behavior_provenance(scope,resource_id,authority,generation) VALUES (?1,?2,'owner',1)",params![versions::scope(table)?,id])?;
        }
        None => {
            return Err(DatabaseError::Invalid(
                "behavior authority is missing".into(),
            ))
        }
    }
    Ok(())
}
/// Proof created only after live authorization and retained-identity checks.
/// It is not constructible from remote JSON or outside this module.
pub(super) struct AuthoringPermit {
    pub(super) identity: Authenticated,
    scope: &'static str,
    id: String,
    previous_revision: i64,
    previous_generation: Option<i64>,
    allow_owner_edits: bool,
    pub(super) affected_events: Vec<String>,
}
pub(super) fn authorize_authoring(
    db: &Connection,
    table: &str,
    id: &str,
    auth: &Authorization,
    allow_owner_edits: bool,
) -> Result<AuthoringPermit, DatabaseError> {
    let identity = auth
        .snapshot()
        .map_err(DatabaseError::CommandAuthorization)?;
    let family = crate::collaboration::AutomationFamily::from_scope(versions::scope(table)?)
        .ok_or_else(|| DatabaseError::Invalid("unknown behavior family".into()))?;
    if !canonical_id(id)
        || identity.grant.validate().is_err()
        || !identity
            .grant
            .capabilities
            .contains(&Capability::AutomationRead)
        || !identity
            .grant
            .capabilities
            .contains(&Capability::AutomationEdit)
        || identity.grant.resources.contains_key(&ResourceKind::Live)
        || !identity.grant.allows_automation(family, id)
    {
        return Err(DatabaseError::CommandAuthorization(
            crate::collaboration::Error::Forbidden,
        ));
    }
    let allow_owner_edits = allow_owner_edits
        && identity
            .grant
            .automation_delegation
            .as_ref()
            .is_some_and(|policy| policy.allow_owner_edits);
    let affected_events = if family == crate::collaboration::AutomationFamily::Action {
        protect_action_dependents(
            db,
            id,
            &identity.principal_id,
            &identity.grant,
            allow_owner_edits,
        )?
    } else {
        Vec::new()
    };
    let previous_revision = versions::current(db, table, id)?;
    let provenance = get(db, table, id)?;
    let previous_generation = match provenance {
        Some(BehaviorProvenance {
            authority:
                BehaviorAuthority::Delegated {
                    ref principal_id, ..
                },
            generation,
        }) if principal_id == &identity.principal_id && previous_revision > 0 => Some(generation),
        Some(BehaviorProvenance {
            authority: BehaviorAuthority::Owner,
            generation,
        }) if allow_owner_edits && previous_revision > 0 => Some(generation),
        Some(_) => {
            return Err(DatabaseError::CommandAuthorization(
                crate::collaboration::Error::Forbidden,
            ))
        }
        None => {
            let exists: bool = db.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
                [id],
                |r| r.get(0),
            )?;
            if previous_revision != 0 || exists {
                return Err(DatabaseError::CommandAuthorization(
                    crate::collaboration::Error::Forbidden,
                ));
            }
            None
        }
    };
    Ok(AuthoringPermit {
        identity,
        scope: family.scope(),
        id: id.into(),
        previous_revision,
        previous_generation,
        allow_owner_edits,
        affected_events,
    })
}
/// A disabled/new action can still change a referenced event's captured
/// dependency generation. Protect owner/foreign event graphs, including their
/// disabled and currently unresolved references, before a delegated action write.
/// This scan stays inside the caller's immediate writer transaction. Nothing
/// from the inspected definitions or IDs is returned in a denial.
fn protect_action_dependents(
    db: &Connection,
    action_id: &str,
    author: &str,
    grant: &crate::collaboration::Grant,
    allow_owner_edits: bool,
) -> Result<Vec<String>, DatabaseError> {
    let denied = || DatabaseError::CommandAuthorization(crate::collaboration::Error::Forbidden);
    // Avoid materializing an arbitrarily large owner script merely to discover
    // its references. A definition that cannot fit this inspection budget
    // requires owner intervention rather than a weaker remote write.
    let mut statement = db
        .prepare("SELECT CASE WHEN length(CAST(id AS BLOB))<=128 THEN id ELSE NULL END,CASE WHEN length(CAST(payload_json AS BLOB))<=262144 THEN payload_json ELSE NULL END FROM behavior_events ORDER BY id")
        .map_err(|_| denied())?;
    let mut query = statement.query([]).map_err(|_| denied())?;
    let mut inspected = 0usize;
    let mut affected = Vec::new();
    while let Some(row) = query.next().map_err(|_| denied())? {
        let event_id: String = row.get(0).map_err(|_| denied())?;
        let payload: String = row.get(1).map_err(|_| denied())?;
        inspected = inspected
            .checked_add(event_id.len())
            .and_then(|n| n.checked_add(payload.len()))
            .ok_or_else(denied)?;
        if inspected > crate::collaboration::SNAPSHOT_BYTES {
            return Err(denied());
        }
        let provenance = get(db, "behavior_events", &event_id)
            .map_err(|_| denied())?
            .ok_or_else(denied)?;
        if versions::current(db, "behavior_events", &event_id).map_err(|_| denied())? <= 0 {
            return Err(denied());
        }
        let definition: Value = serde_json::from_str(&payload).map_err(|_| denied())?;
        let object = definition.as_object().ok_or_else(denied)?;
        let dependencies = match object.get("actionIds") {
            None => continue,
            Some(Value::Array(ids)) => ids,
            _ => return Err(denied()),
        };
        if dependencies.iter().any(|id| !id.is_string()) {
            return Err(denied());
        }
        if dependencies.iter().any(|id| id.as_str() == Some(action_id)) {
            let own = matches!(provenance.authority, BehaviorAuthority::Delegated { ref principal_id, .. } if principal_id == author);
            let owner = matches!(provenance.authority, BehaviorAuthority::Owner)
                && allow_owner_edits
                && grant
                    .allows_automation(crate::collaboration::AutomationFamily::Event, &event_id);
            if !own && !owner {
                return Err(denied());
            }
            affected.push(event_id);
        }
    }
    Ok(affected)
}
/// Call only after save_record succeeded in the SAME transaction. New records
/// momentarily have owner bookkeeping inside that uncommitted TX; this replaces
/// it before publication. Existing delegated writes retain their origin while
/// recording the currently authoring session. The version advance already
/// invalidated approvals, so this does not bump the generation a second time.
pub(super) fn mark_delegated_new(
    db: &Connection,
    table: &str,
    id: &str,
    permit: &AuthoringPermit,
) -> Result<(), DatabaseError> {
    let scope = versions::scope(table)?;
    let next = permit
        .previous_revision
        .checked_add(1)
        .ok_or_else(|| DatabaseError::Invalid("automation revision exhausted".into()))?;
    let provenance = get(db, table, id)?
        .ok_or_else(|| DatabaseError::Invalid("behavior authority is missing".into()))?;
    let expected_generation = permit.previous_generation.map_or(Ok(1), |g| {
        g.checked_add(1)
            .ok_or_else(|| DatabaseError::Invalid("behavior authority generation exhausted".into()))
    })?;
    let previous_authority_valid = match (&provenance.authority, permit.previous_generation) {
        (BehaviorAuthority::Owner, None) => true,
        (BehaviorAuthority::Owner, Some(_)) if permit.allow_owner_edits => true,
        (BehaviorAuthority::Delegated { principal_id, .. }, Some(_)) => {
            principal_id == &permit.identity.principal_id
        }
        _ => false,
    };
    let disabled: bool = db.query_row(
        &format!("SELECT enabled=0 FROM {table} WHERE id=?1"),
        [id],
        |r| r.get(0),
    )?;
    if scope != permit.scope
        || id != permit.id
        || versions::current(db, table, id)? != next
        || provenance.generation != expected_generation
        || !previous_authority_valid
        || !disabled
    {
        return Err(DatabaseError::Invalid(
            "behavior authoring transition changed".into(),
        ));
    }
    db.execute("UPDATE behavior_provenance SET authority='delegated',principal_id=?3,session_id=?4 WHERE scope=?1 AND resource_id=?2",params![scope,id,permit.identity.principal_id,permit.identity.session_id])?;
    Ok(())
}
impl DatabaseManager {
    #[cfg(test)]
    pub(crate) fn behavior_provenance(
        &self,
        table: &str,
        id: &str,
    ) -> Result<Option<BehaviorProvenance>, DatabaseError> {
        get(&self.open(&self.automation_path())?, table, id)
    }
    pub(crate) fn behavior_provenance_versioned(
        &self,
        table: &str,
        id: &str,
    ) -> Result<Versioned<Option<BehaviorProvenance>>, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let result = Versioned::new(
            get(&tx, table, id)?,
            vec![versions::version(
                table,
                id,
                versions::current(&tx, table, id)?,
            )?],
        );
        tx.commit()?;
        Ok(result)
    }
    /// Explicit trusted local-owner transition. Editing/recreating JSON never
    /// calls this method and therefore cannot adopt another principal's work.
    #[cfg(test)]
    pub(crate) fn adopt_behavior_owner(
        &self,
        table: &str,
        id: &str,
        expected: &str,
    ) -> Result<Versioned<BehaviorProvenance>, DatabaseError> {
        self.behavior_transaction(|tx| adopt_owner(tx, table, id, expected))
    }
}
pub(super) fn adopt_owner(
    tx: &Connection,
    table: &str,
    id: &str,
    expected: &str,
) -> Result<Versioned<BehaviorProvenance>, DatabaseError> {
    versions::scope(table)?;

    let previous = versions::check(tx, table, id, Some(expected))?;
    let exists: bool = tx.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
        [id],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(DatabaseError::NotFound);
    }
    let provenance = get(tx, table, id)?
        .ok_or_else(|| DatabaseError::Invalid("behavior authority is missing".into()))?;
    if provenance.authority == BehaviorAuthority::Owner {
        return Ok(Versioned::new(
            provenance,
            vec![versions::version(table, id, previous)?],
        ));
    }
    // Adoption changes execution authority, so it always leaves delegated work
    // inactive. A separate checked owner enable is required after review.
    tx.execute(
        &format!("UPDATE {table} SET enabled=0,updated_at=?2 WHERE id=?1"),
        params![id, now()],
    )?;
    tx.execute("UPDATE behavior_provenance SET authority='owner',principal_id=NULL,session_id=NULL WHERE scope=?1 AND resource_id=?2",params![versions::scope(table)?,id])?;
    let version = versions::advance(tx, table, id, previous, false, "adopted", now())?;
    Ok(Versioned::new(
        get(tx, table, id)?
            .ok_or_else(|| DatabaseError::Invalid("behavior authority is missing".into()))?,
        vec![version],
    ))
}

#[cfg(test)]
mod tests;
