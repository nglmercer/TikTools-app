//! Transactional command primitives. These functions use the business writer's
//! existing transaction, never another SQLite connection or an in-memory cache.
use super::{util::now, DatabaseError};
#[cfg(test)]
mod tests;
use crate::collaboration::{
    hash_secret, CommandOutcome, CommandReceipt, CommandRequest, CommandResource, CommandState,
    ResourceKind, Versioned,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

const OUTCOME_TTL: i64 = 7 * 24 * 60 * 60 * 1000;
const MAX_STORE_COMMAND_IDS: i64 = 1_000_000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandDatabase {
    Automation,
    Points,
}

/// Created by trusted host code; it is not deserializable from RPC data.
pub(crate) struct CommandTransaction<R = CommandRequest> {
    pub principal_id: String,
    pub session_id: String,
    /// Live host authorization; rechecked after SQLite writer admission.
    pub authorization: Option<crate::collaboration::Authorization>,
    pub request: R,
    pub current_instance_id: String,
    pub max_ids_per_principal: u32,
}

pub(crate) trait CommandIdentity {
    fn command_id(&self) -> &str;
    fn host_instance_id(&self) -> &str;
}
impl CommandIdentity for CommandRequest {
    fn command_id(&self) -> &str {
        &self.command_id
    }
    fn host_instance_id(&self) -> &str {
        &self.host_instance_id
    }
}
impl CommandIdentity for crate::collaboration::AdditiveCommandRequest {
    fn command_id(&self) -> &str {
        &self.command_id
    }
    fn host_instance_id(&self) -> &str {
        &self.host_instance_id
    }
}

fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(values) => {
            let sorted: std::collections::BTreeMap<_, _> = values.iter().collect();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, value)| (key.clone(), canonical(value)))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

pub(super) fn payload_hash(
    method: &str,
    params: &Value,
    expected: &str,
) -> Result<String, DatabaseError> {
    // Instance, wire request ID and session are fencing/transport data. They
    // must not change the semantic identity of an already committed command.
    Ok(hash_secret(&serde_json::to_string(&canonical(&json!({
        "method":method,"params":params,"expectedRevision":expected,
    })))?))
}

pub(super) fn additive_hash(method: &str, params: &Value) -> Result<String, DatabaseError> {
    Ok(hash_secret(&serde_json::to_string(&canonical(&json!({
        "method":method,"params":params,
    })))?))
}

pub(super) fn prune(db: &Connection) -> Result<(), DatabaseError> {
    db.execute("UPDATE control_commands SET result_json=NULL,versions_json=NULL,outcome_expired=1 WHERE committed_at<?1 AND outcome_expired=0 AND state IN ('succeeded','failed','unknown')", [now().saturating_sub(OUTCOME_TTL)])?;
    Ok(())
}

pub(super) fn lookup(
    db: &Connection,
    principal: &str,
    id: &str,
) -> Result<Option<CommandOutcome>, DatabaseError> {
    let row = db.query_row(
        "SELECT method,resource_kind,resource_scope,resource_id,host_instance_id,state,committed_at,result_json,versions_json,outcome_expired FROM control_commands WHERE principal_id=?1 AND command_id=?2",
        params![principal,id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?,row.get::<_,i64>(6)?,row.get::<_,Option<String>>(7)?,row.get::<_,Option<String>>(8)?,row.get::<_,bool>(9)?)),
    ).optional()?;
    let Some((
        method,
        kind,
        scope,
        resource,
        instance,
        state,
        timestamp,
        result,
        versions,
        expired,
    )) = row
    else {
        return Ok(None);
    };
    if expired || timestamp < now().saturating_sub(OUTCOME_TTL) {
        return Err(DatabaseError::CommandExpired);
    }
    let kind: ResourceKind = serde_json::from_value(Value::String(kind))?;
    let state: CommandState = serde_json::from_value(Value::String(state))?;
    Ok(Some(CommandOutcome {
        principal_id: principal.into(),
        method,
        host_instance_id: instance,
        command: CommandReceipt {
            command_id: id.into(),
            state,
            resource: Some(CommandResource {
                kind,
                scope,
                id: resource,
            }),
            committed_at: Some(timestamp),
        },
        result: result
            .map(|text| serde_json::from_str(&text))
            .transpose()?
            .unwrap_or(Value::Null),
        resource_versions: versions
            .map(|text| serde_json::from_str(&text))
            .transpose()?
            .unwrap_or_default(),
    }))
}

pub(super) fn replay<R: CommandIdentity>(
    db: &Connection,
    command: &CommandTransaction<R>,
    hash: &str,
) -> Result<Option<Versioned<Value>>, DatabaseError> {
    let previous: Option<String> = db
        .query_row(
            "SELECT payload_hash FROM control_commands WHERE principal_id=?1 AND command_id=?2",
            params![command.principal_id, command.request.command_id()],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(previous) = previous {
        if previous != hash {
            return Err(DatabaseError::CommandIdReused);
        }
        let outcome = lookup(db, &command.principal_id, command.request.command_id())?
            .ok_or(DatabaseError::NotFound)?;
        return Ok(Some(Versioned {
            value: outcome.result,
            resource_versions: outcome.resource_versions,
            command: Some(outcome.command),
            host_instance_id: Some(command.current_instance_id.clone()),
        }));
    }
    if command.request.host_instance_id() != command.current_instance_id {
        return Err(DatabaseError::HostRestarted);
    }
    let count: i64 = db.query_row(
        "SELECT count(*) FROM control_commands WHERE principal_id=?1",
        [&command.principal_id],
        |row| row.get(0),
    )?;
    if count >= i64::from(command.max_ids_per_principal) {
        return Err(DatabaseError::CommandCapacity);
    }
    let total: i64 = db.query_row("SELECT count(*) FROM control_commands", [], |row| {
        row.get(0)
    })?;
    if total >= MAX_STORE_COMMAND_IDS {
        return Err(DatabaseError::CommandCapacity);
    }
    Ok(None)
}

pub(super) fn finish<R: CommandIdentity>(
    db: &Connection,
    command: &CommandTransaction<R>,
    hash: &str,
    method: &str,
    before: &str,
    mut result: Versioned<Value>,
) -> Result<Versioned<Value>, DatabaseError> {
    let resource = result
        .resource_versions
        .first()
        .ok_or_else(|| DatabaseError::Invalid("command resource version is missing".into()))?;
    let kind = serde_json::to_value(resource.kind)?
        .as_str()
        .ok_or_else(|| DatabaseError::Invalid("invalid command resource kind".into()))?
        .to_owned();
    let timestamp = now();
    db.execute("INSERT INTO control_commands(principal_id,command_id,payload_hash,method,resource_kind,resource_scope,resource_id,session_id,host_instance_id,state,committed_at,result_json,versions_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'succeeded',?10,?11,?12)", params![command.principal_id,command.request.command_id(),hash,method,kind,resource.scope,resource.id,command.session_id,command.current_instance_id,timestamp,serde_json::to_string(&result.value)?,serde_json::to_string(&result.resource_versions)?])?;
    let event_id = hash_secret(&serde_json::to_string(&(
        "command",
        &command.principal_id,
        &command.request.command_id(),
    ))?);
    db.execute("INSERT INTO control_command_outbox(event_id,principal_id,session_id,command_id,method,resource_kind,resource_scope,resource_id,before_revision,after_revision,outcome_code,committed_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'succeeded',?11)",params![event_id,command.principal_id,command.session_id,command.request.command_id(),method,kind,resource.scope,resource.id,before,resource.revision,timestamp])?;
    if before != resource.revision {
        let linked=db.execute("INSERT INTO control_command_resources(event_id,resource_cursor) SELECT ?1,cursor FROM control_resource_outbox WHERE kind=?2 AND scope=?3 AND resource_id=?4 AND revision=?5",params![event_id,kind,resource.scope,resource.id,resource.revision.parse::<i64>().map_err(|_|DatabaseError::Invalid("invalid command resource revision".into()))?])?;
        if linked != 1 {
            return Err(DatabaseError::Invalid(
                "command invalidation is missing".into(),
            ));
        }
    }
    result.command = Some(CommandReceipt {
        command_id: command.request.command_id().to_owned(),
        state: CommandState::Succeeded,
        resource: Some(CommandResource {
            kind: resource.kind,
            scope: resource.scope.clone(),
            id: resource.id.clone(),
        }),
        committed_at: Some(timestamp),
    });
    result.host_instance_id = Some(command.current_instance_id.clone());
    Ok(result)
}

impl super::DatabaseManager {
    /// All public command coordinators call this under the host-wide gate and
    /// exclusive workspace lease. IDs and capacity span the business ledgers;
    /// only their owning transaction may commit a new ID. A crash therefore
    /// cannot create a reservation without its business result.
    pub(crate) fn check_command_admission(
        &self,
        owner: CommandDatabase,
        principal: &str,
        id: &str,
        limit: u32,
    ) -> Result<bool, DatabaseError> {
        let mut existing = false;
        let mut principal_count = 0i64;
        let mut total = 0i64;
        for (kind, path) in [
            (CommandDatabase::Automation, self.automation_path()),
            (CommandDatabase::Points, self.points_path()),
        ] {
            let db = self.open(&path)?;
            let found: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM control_commands WHERE principal_id=?1 AND command_id=?2)",params![principal,id], |r|r.get(0))?;
            if found && kind != owner {
                return Err(DatabaseError::CommandIdReused);
            }
            existing |= found;
            principal_count += db.query_row(
                "SELECT count(*) FROM control_commands WHERE principal_id=?1",
                [principal],
                |r| r.get::<_, i64>(0),
            )?;
            total += db.query_row("SELECT count(*) FROM control_commands", [], |r| {
                r.get::<_, i64>(0)
            })?;
        }
        if !existing && (principal_count >= i64::from(limit) || total >= MAX_STORE_COMMAND_IDS) {
            return Err(DatabaseError::CommandCapacity);
        }
        Ok(existing)
    }

    fn command_location(
        &self,
        principal: &str,
        id: &str,
    ) -> Result<Option<(std::path::PathBuf, CommandResource)>, DatabaseError> {
        let mut found = None;
        for path in [self.automation_path(), self.points_path()] {
            let db = self.open(&path)?;
            let row:Option<(String,String,String)>=db.query_row("SELECT resource_kind,resource_scope,resource_id FROM control_commands WHERE principal_id=?1 AND command_id=?2", params![principal,id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?;
            if let Some((kind, scope, id)) = row {
                let kind = serde_json::from_value(Value::String(kind))?;
                if found.is_some() {
                    return Err(DatabaseError::CommandLedgerInvalid);
                }
                found = Some((path, CommandResource { kind, scope, id }));
            }
        }
        Ok(found)
    }

    pub(crate) fn command_resource(
        &self,
        principal: &str,
        id: &str,
    ) -> Result<Option<CommandResource>, DatabaseError> {
        Ok(self
            .command_location(principal, id)?
            .map(|(_, resource)| resource))
    }
    pub(crate) fn command_outcome(
        &self,
        principal: &str,
        id: &str,
    ) -> Result<Option<CommandOutcome>, DatabaseError> {
        let Some((path, _)) = self.command_location(principal, id)? else {
            return Ok(None);
        };
        let db = self.open(&path)?;
        prune(&db)?;
        lookup(&db, principal, id)
    }
    pub(crate) fn prune_command_outcomes(&self) -> Result<(), DatabaseError> {
        prune(&self.open(&self.automation_path())?)?;
        prune(&self.open(&self.points_path())?)
    }
}
