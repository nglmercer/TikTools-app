use rusqlite::{params, OptionalExtension};
use serde_json::{json, Map, Value};

use super::{
    util::{bool_int, now, required_value_string},
    DatabaseError, DatabaseManager,
};

impl DatabaseManager {
    pub(crate) fn load_workflows(&self) -> Result<Vec<Value>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare(
            "SELECT id, name, enabled, graph_json, created_at, updated_at
             FROM automation_workflows ORDER BY updated_at DESC, name ASC",
        )?;
        let rows = statement.query_map([], workflow_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub(crate) fn save_workflow(&self, graph: &Value) -> Result<Value, DatabaseError> {
        let object = graph
            .as_object()
            .ok_or_else(|| DatabaseError::Invalid("workflow graph must be an object".to_owned()))?;
        let id = required_value_string(object, "id")?;
        let name = required_value_string(object, "name")?;
        let enabled = object
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let connection = self.open(&self.automation_path())?;
        let existing: Option<i64> = connection
            .query_row(
                "SELECT created_at FROM automation_workflows WHERE id = ?",
                [&id],
                |row| row.get(0),
            )
            .optional()?;
        let created_at = existing.unwrap_or_else(now);
        let updated_at = now();
        connection.execute(
            "INSERT INTO automation_workflows
             (id, name, enabled, graph_json, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name,
               enabled = excluded.enabled, graph_json = excluded.graph_json,
               updated_at = excluded.updated_at",
            params![
                id,
                name,
                bool_int(enabled),
                serde_json::to_string(graph)?,
                created_at,
                updated_at
            ],
        )?;
        Ok(workflow_record(
            object,
            graph.clone(),
            created_at,
            updated_at,
        ))
    }
    pub(crate) fn delete_workflow(&self, id: &str) -> Result<bool, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        Ok(connection.execute("DELETE FROM automation_workflows WHERE id = ?", [id])? > 0)
    }
    pub(crate) fn set_workflow_enabled(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<Value, DatabaseError> {
        let workflow = self
            .load_workflows()?
            .into_iter()
            .find(|value| value.get("id").and_then(Value::as_str) == Some(id))
            .ok_or_else(|| DatabaseError::Invalid(format!("unknown workflow: {id}")))?;
        let mut graph = workflow
            .get("graph")
            .cloned()
            .ok_or_else(|| DatabaseError::Invalid("workflow has no graph".to_owned()))?;
        graph["enabled"] = Value::Bool(enabled);
        self.save_workflow(&graph)
    }
}

fn workflow_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let enabled = row.get::<_, i64>(2)? != 0;
    let graph: Value = serde_json::from_str(&row.get::<_, String>(3)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(json!({
        "id": id,
        "name": name,
        "enabled": enabled,
        "graph": graph,
        "createdAt": row.get::<_, i64>(4)?,
        "updatedAt": row.get::<_, i64>(5)?
    }))
}
fn workflow_record(
    object: &Map<String, Value>,
    graph: Value,
    created_at: i64,
    updated_at: i64,
) -> Value {
    json!({
        "id": object.get("id"),
        "name": object.get("name"),
        "enabled": object.get("enabled").and_then(Value::as_bool).unwrap_or(false),
        "graph": graph,
        "createdAt": created_at,
        "updatedAt": updated_at
    })
}
