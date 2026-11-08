use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::{json, Value};

use super::{
    util::{bool_int, now, required_value_string},
    DatabaseError, DatabaseManager,
};
use crate::collaboration::{ResourceKind, ResourceVersion, Versioned};

const SELECT: &str =
    "SELECT w.id,w.name,w.enabled,w.graph_json,w.created_at,w.updated_at,v.revision
    FROM automation_workflows w JOIN control_resource_versions v
    ON v.kind='workflow' AND v.scope='host' AND v.resource_id=w.id AND v.deleted=0";

impl DatabaseManager {
    /// Read only permitted rows into a bounded immutable baseline. Size is
    /// checked in SQL before materializing an arbitrary legacy graph body.
    pub(crate) fn load_workflow_snapshot(
        &self,
        grant: &crate::collaboration::Grant,
        page_bytes: usize,
        total_bytes: usize,
    ) -> Result<Vec<(Value, ResourceVersion)>, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction()?;
        let mut records = Vec::new();
        let mut used = 0usize;
        {
            let mut stmt=tx.prepare("SELECT CASE WHEN length(CAST(w.id AS BLOB))<=128 THEN w.id ELSE NULL END,w.name,w.enabled,w.graph_json,w.created_at,w.updated_at,v.revision,length(CAST(w.graph_json AS BLOB))+length(CAST(w.name AS BLOB)) FROM automation_workflows w JOIN control_resource_versions v ON v.kind='workflow' AND v.scope='host' AND v.resource_id=w.id AND v.deleted=0 ORDER BY w.id")?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let id: Option<String> = row.get(0)?;
                let Some(id) = id else {
                    if grant.resources.contains_key(&ResourceKind::Workflow) {
                        continue;
                    }
                    return Err(DatabaseError::SnapshotTooLarge);
                };
                if !grant.allows_resource(ResourceKind::Workflow, &id) {
                    continue;
                }
                let bytes: i64 = row.get(7)?;
                if bytes < 0 || bytes as u64 > page_bytes as u64 {
                    return Err(DatabaseError::SnapshotTooLarge);
                }
                let (record, version) = workflow_from_row(row)?;
                let bytes =
                    serde_json::to_vec(&record)?.len() + serde_json::to_vec(&version)?.len() + 2;
                used = used
                    .checked_add(bytes)
                    .ok_or(DatabaseError::SnapshotTooLarge)?;
                if bytes > page_bytes
                    || used > total_bytes.min(crate::collaboration::SNAPSHOT_BYTES)
                {
                    return Err(DatabaseError::SnapshotTooLarge);
                }
                records.push((record, version));
            }
        }
        tx.commit()?;
        Ok(records)
    }
    pub(crate) fn load_workflows(&self) -> Result<Versioned<Vec<Value>>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let mut statement =
            connection.prepare(&format!("{SELECT} ORDER BY w.updated_at DESC,w.name ASC"))?;
        let mut value = Vec::new();
        let mut resource_versions = Vec::new();
        for row in statement.query_map([], workflow_from_row)? {
            let (record, version) = row?;
            value.push(record);
            resource_versions.push(version);
        }
        Ok(Versioned::new(value, resource_versions))
    }

    pub(crate) fn load_workflow(
        &self,
        id: &str,
    ) -> Result<Versioned<Option<Value>>, DatabaseError> {
        self.read_workflow(id, None)
    }
    pub(crate) fn load_workflow_bounded(
        &self,
        id: &str,
        bytes: usize,
    ) -> Result<Versioned<Option<Value>>, DatabaseError> {
        self.read_workflow(id, Some(bytes))
    }
    fn read_workflow(
        &self,
        id: &str,
        budget: Option<usize>,
    ) -> Result<Versioned<Option<Value>>, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction()?;
        if let Some(bytes) = budget {
            let size:Option<i64>=tx.query_row("SELECT length(CAST(graph_json AS BLOB))+length(CAST(name AS BLOB)) FROM automation_workflows WHERE id=?1",[id],|r|r.get(0)).optional()?;
            if size.is_some_and(|size| size < 0 || size as u64 > bytes as u64) {
                return Err(DatabaseError::SnapshotTooLarge);
            }
        }
        let record = tx
            .query_row(&format!("{SELECT} WHERE w.id=?1"), [id], workflow_from_row)
            .optional()?;
        // Keep the missing row and tombstone lookup in the same read snapshot.
        let result = match record {
            Some((value, revision)) => Versioned::new(Some(value), vec![revision]),
            None => versioned(None, id, current_revision(&tx, id)?),
        };
        if budget.is_some_and(|bytes| serde_json::to_vec(&result).map_or(true, |v| v.len() > bytes))
        {
            return Err(DatabaseError::SnapshotTooLarge);
        }
        tx.commit()?;
        Ok(result)
    }

    fn workflow_transaction<T>(
        &self,
        apply: impl FnOnce(&Transaction<'_>) -> Result<T, DatabaseError>,
    ) -> Result<T, DatabaseError> {
        let mut connection = self.open(&self.automation_path())?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result = apply(&tx)?;
        // Subprocess tests terminate without destructors on each side of the
        // actual SQLite commit. This failpoint does not exist in host builds.
        #[cfg(test)]
        if std::env::var("TIKTOOLS_COMMAND_CRASH_STAGE").as_deref() == Ok("before_commit") {
            std::process::exit(86);
        }
        tx.commit()?;
        #[cfg(test)]
        if std::env::var("TIKTOOLS_COMMAND_CRASH_STAGE").as_deref() == Ok("after_commit") {
            std::process::exit(87);
        }
        Ok(result)
    }
    pub(crate) fn save_workflow(
        &self,
        graph: &Value,
        expected: Option<&str>,
    ) -> Result<Versioned<Value>, DatabaseError> {
        self.workflow_transaction(|tx| save(tx, graph, expected))
    }
    pub(crate) fn delete_workflow(
        &self,
        id: &str,
        expected: Option<&str>,
    ) -> Result<Versioned<bool>, DatabaseError> {
        self.workflow_transaction(|tx| delete(tx, id, expected))
    }
    pub(crate) fn set_workflow_enabled(
        &self,
        id: &str,
        enabled: bool,
        expected: Option<&str>,
    ) -> Result<Versioned<Value>, DatabaseError> {
        self.workflow_transaction(|tx| toggle(tx, id, enabled, expected))
    }
    pub(crate) fn execute_workflow_command(
        &self,
        operation: &crate::control::WorkflowMutation,
        command: &super::CommandTransaction,
    ) -> Result<(Versioned<Value>, bool), DatabaseError> {
        command
            .request
            .validate()
            .map_err(|_| DatabaseError::Invalid("invalid command metadata".into()))?;
        let hash = super::commands::payload_hash(
            operation.method(),
            &operation.params(),
            &command.request.expected_revision,
        )?;
        self.workflow_transaction(|tx| {
            if let Some(auth) = &command.authorization {
                let identity = operation
                    .authorize_removal(auth)
                    .map_err(DatabaseError::CommandAuthorization)?;
                if identity.principal_id != command.principal_id
                    || identity.session_id != command.session_id
                {
                    return Err(DatabaseError::CommandAuthorization(
                        crate::collaboration::Error::Forbidden,
                    ));
                }
            }
            super::commands::prune(tx)?;
            if let Some(result) = super::commands::replay(tx, command, &hash)? {
                return Ok((result, false));
            }
            let expected = Some(command.request.expected_revision.as_str());
            let result = match operation {
                crate::control::WorkflowMutation::Save(graph) => save(tx, graph, expected)?,
                crate::control::WorkflowMutation::Delete(id) => {
                    delete(tx, id, expected)?.map(|deleted| json!({"id":id,"deleted":deleted}))
                }
                crate::control::WorkflowMutation::SetEnabled { id, enabled } => {
                    let result = toggle(tx, id, *enabled, expected)?;
                    if command.principal_id == "local-owner" {
                        result
                    } else {
                        // Remote deactivation receipts must fit even when an
                        // owner graph exceeds this connection's frame budget.
                        // Retain this exact bounded result for command queries.
                        result.map(|_| json!({"id":id,"enabled":enabled}))
                    }
                }
            };
            let result = super::commands::finish(
                tx,
                command,
                &hash,
                operation.method(),
                &command.request.expected_revision,
                result,
            )?;
            Ok((result, true))
        })
    }
}

fn save(
    tx: &Transaction<'_>,
    graph: &Value,
    expected: Option<&str>,
) -> Result<Versioned<Value>, DatabaseError> {
    let object = graph
        .as_object()
        .ok_or_else(|| DatabaseError::Invalid("workflow graph must be an object".to_owned()))?;
    let id = required_value_string(object, "id")?;
    validate_id(&id)?;
    let name = required_value_string(object, "name")?;
    let enabled = object
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let previous = check_revision(tx, &id, expected)?;
    let created_at: Option<i64> = tx
        .query_row(
            "SELECT created_at FROM automation_workflows WHERE id=?1",
            [&id],
            |row| row.get(0),
        )
        .optional()?;
    let created_at = created_at.unwrap_or_else(now);
    let updated_at = now();
    tx.execute(
        "INSERT INTO automation_workflows (id,name,enabled,graph_json,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name,enabled=excluded.enabled,
               graph_json=excluded.graph_json,updated_at=excluded.updated_at",
        params![
            id,
            name,
            bool_int(enabled),
            serde_json::to_string(graph)?,
            created_at,
            updated_at
        ],
    )?;
    let revision = advance(tx, &id, previous, false, "saved", updated_at)?;
    let value = json!({"id":id,"name":name,"enabled":enabled,"graph":graph,"createdAt":created_at,"updatedAt":updated_at});
    Ok(versioned(value, &id, revision))
}
fn delete(
    tx: &Transaction<'_>,
    id: &str,
    expected: Option<&str>,
) -> Result<Versioned<bool>, DatabaseError> {
    validate_id(id)?;
    let previous = check_revision(tx, id, expected)?;
    let removed = tx.execute("DELETE FROM automation_workflows WHERE id=?1", [id])? > 0;
    let revision = if removed {
        advance(tx, id, previous, true, "deleted", now())?
    } else {
        previous
    };
    Ok(versioned(removed, id, revision))
}
fn toggle(
    tx: &Transaction<'_>,
    id: &str,
    enabled: bool,
    expected: Option<&str>,
) -> Result<Versioned<Value>, DatabaseError> {
    validate_id(id)?;
    let previous = check_revision(tx, id, expected)?;
    let (mut record, _) = tx
        .query_row(&format!("{SELECT} WHERE w.id=?1"), [id], workflow_from_row)
        .optional()?
        .ok_or(DatabaseError::NotFound)?;
    let mut graph = record["graph"].clone();
    if !graph.is_object() {
        return Err(DatabaseError::Invalid(
            "workflow graph must be an object".into(),
        ));
    }
    graph["enabled"] = Value::Bool(enabled);
    let updated_at = now();
    tx.execute(
        "UPDATE automation_workflows SET enabled=?1,graph_json=?2,updated_at=?3 WHERE id=?4",
        params![
            bool_int(enabled),
            serde_json::to_string(&graph)?,
            updated_at,
            id
        ],
    )?;
    let revision = advance(
        tx,
        id,
        previous,
        false,
        if enabled { "enabled" } else { "disabled" },
        updated_at,
    )?;
    record["enabled"] = Value::Bool(enabled);
    record["graph"] = graph;
    record["updatedAt"] = json!(updated_at);
    Ok(versioned(record, id, revision))
}

fn current_revision(db: &Connection, id: &str) -> Result<i64, DatabaseError> {
    Ok(db.query_row("SELECT revision FROM control_resource_versions WHERE kind='workflow' AND scope='host' AND resource_id=?1", [id], |row| row.get(0)).optional()?.unwrap_or(0))
}

fn validate_id(id: &str) -> Result<(), DatabaseError> {
    if id.is_empty()
        || id.len() > 128
        || id.trim() != id
        || id.contains('*')
        || id.chars().any(char::is_control)
    {
        return Err(DatabaseError::Invalid(
            "workflow ID must be canonical and at most 128 bytes".into(),
        ));
    }
    Ok(())
}

fn check_revision(db: &Connection, id: &str, expected: Option<&str>) -> Result<i64, DatabaseError> {
    let previous = current_revision(db, id)?;
    if let Some(expected) = expected {
        let parsed = expected
            .parse::<i64>()
            .ok()
            .filter(|number| *number >= 0 && number.to_string() == expected)
            .ok_or_else(|| DatabaseError::Invalid("invalid workflow expectedRevision".into()))?;
        if parsed != previous {
            return Err(DatabaseError::RevisionConflict(version(id, previous)));
        }
    }
    Ok(previous)
}

fn advance(
    db: &Connection,
    id: &str,
    previous: i64,
    deleted: bool,
    change: &str,
    timestamp: i64,
) -> Result<i64, DatabaseError> {
    let next = previous
        .checked_add(1)
        .ok_or_else(|| DatabaseError::Invalid("workflow revision exhausted".into()))?;
    db.execute("INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted) VALUES ('workflow','host',?1,?2,?3) ON CONFLICT(kind,scope,resource_id) DO UPDATE SET revision=excluded.revision,deleted=excluded.deleted", params![id,next,bool_int(deleted)])?;
    db.execute("INSERT INTO control_resource_outbox(kind,scope,resource_id,revision,change,committed_at) VALUES ('workflow','host',?1,?2,?3,?4)", params![id,next,change,timestamp])?;
    Ok(next)
}

fn version(id: &str, revision: i64) -> ResourceVersion {
    ResourceVersion {
        kind: ResourceKind::Workflow,
        id: id.to_owned(),
        scope: "host".into(),
        revision: revision.to_string(),
    }
}
fn versioned<T>(value: T, id: &str, revision: i64) -> Versioned<T> {
    Versioned::new(value, vec![version(id, revision)])
}

fn workflow_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(Value, ResourceVersion)> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let enabled = row.get::<_, i64>(2)? != 0;
    let graph: Value = serde_json::from_str(&row.get::<_, String>(3)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok((
        json!({"id":id,"name":name,"enabled":enabled,"graph":graph,"createdAt":row.get::<_,i64>(4)?,"updatedAt":row.get::<_,i64>(5)?}),
        version(&id, row.get(6)?),
    ))
}
