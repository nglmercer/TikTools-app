use rusqlite::{params, OptionalExtension};
use serde_json::{json, Map, Value};

use super::{
    util::{insert_optional_string, now},
    DatabaseError, DatabaseManager,
};

impl DatabaseManager {
    pub(crate) fn load_app_state(&self) -> Result<Map<String, Value>, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let mut statement = connection.prepare("SELECT key, value FROM app_state")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut state = Map::new();
        for row in rows {
            let (key, value) = row?;
            state.insert(key, Value::String(value));
        }
        Ok(state)
    }
    pub(crate) fn save_app_state(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        let connection = self.open(&self.points_path())?;
        connection.execute(
            "INSERT INTO app_state (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now()],
        )?;
        Ok(())
    }
    pub(crate) fn delete_app_state(&self, key: &str) -> Result<(), DatabaseError> {
        let connection = self.open(&self.points_path())?;
        connection.execute("DELETE FROM app_state WHERE key = ?", [key])?;
        Ok(())
    }
    pub(crate) fn load_app_state_prefix(
        &self,
        prefix: &str,
    ) -> Result<Map<String, Value>, DatabaseError> {
        let escaped = prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("{escaped}%");
        let connection = self.open(&self.points_path())?;
        let mut statement =
            connection.prepare("SELECT key, value FROM app_state WHERE key LIKE ? ESCAPE '\\'")?;
        let rows = statement.query_map([pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut state = Map::new();
        for row in rows {
            let (key, value) = row?;
            state.insert(key, Value::String(value));
        }
        Ok(state)
    }
    pub(crate) fn clear_creator_history(&self) -> Result<(), DatabaseError> {
        let connection = self.open(&self.points_path())?;
        connection.execute("DELETE FROM creator_history", [])?;
        connection.execute(
            "DELETE FROM app_state WHERE key IN ('lastCreator', 'lastRoomId', 'lastTitle')",
            [],
        )?;
        Ok(())
    }
    pub(crate) fn save_creator(
        &self,
        unique_id: &str,
        room_id: Option<&str>,
        nickname: Option<&str>,
        avatar_url: Option<&str>,
        title: Option<&str>,
        display_id: Option<&str>,
    ) -> Result<Value, DatabaseError> {
        let unique_id = unique_id.trim().trim_start_matches('@');
        if unique_id.is_empty() {
            return Err(DatabaseError::Invalid(
                "creator unique id cannot be empty".to_owned(),
            ));
        }
        let now = now();
        {
            let connection = self.open(&self.points_path())?;
            connection.execute(
                "INSERT INTO creator_history
                 (unique_id, room_id, nickname, avatar_url, title, last_connected, connect_count, display_id)
                 VALUES (?, ?, ?, ?, ?, ?, 1, ?)
                 ON CONFLICT(unique_id) DO UPDATE SET
                   room_id = COALESCE(excluded.room_id, creator_history.room_id),
                   nickname = COALESCE(excluded.nickname, creator_history.nickname),
                   avatar_url = COALESCE(excluded.avatar_url, creator_history.avatar_url),
                   title = COALESCE(excluded.title, creator_history.title),
                   last_connected = excluded.last_connected,
                   connect_count = creator_history.connect_count + 1,
                   display_id = COALESCE(excluded.display_id, creator_history.display_id)",
                params![
                    unique_id,
                    room_id,
                    nickname.or(Some(unique_id)),
                    avatar_url,
                    title,
                    now,
                    display_id.or(Some(unique_id)),
                ],
            )?;
            for (key, value) in [
                ("lastCreator", unique_id),
                ("lastRoomId", room_id.unwrap_or_default()),
                ("lastTitle", title.unwrap_or_default()),
            ] {
                if !value.is_empty() {
                    connection.execute(
                        "INSERT INTO app_state (key, value, updated_at) VALUES (?, ?, ?)
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                        params![key, value, now],
                    )?;
                }
            }
        }
        self.load_creator(Some(unique_id))?.ok_or_else(|| {
            DatabaseError::Invalid("creator was not readable after saving".to_owned())
        })
    }
    pub(crate) fn load_creator(
        &self,
        unique_id: Option<&str>,
    ) -> Result<Option<Value>, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let unique_id = match unique_id {
            Some(unique_id) => Some(unique_id.trim().trim_start_matches('@').to_owned()),
            None => connection
                .query_row(
                    "SELECT value FROM app_state WHERE key = 'lastCreator'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()?,
        };
        let Some(unique_id) = unique_id.filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        connection
            .query_row(
                "SELECT unique_id, room_id, nickname, avatar_url, title,
                        last_connected, connect_count, display_id
                 FROM creator_history WHERE unique_id = ?",
                [&unique_id],
                creator_from_row,
            )
            .optional()
            .map_err(DatabaseError::from)
    }
    pub(crate) fn load_recent_creators(&self, limit: i64) -> Result<Vec<Value>, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let mut statement = connection.prepare(
            "SELECT unique_id, room_id, nickname, avatar_url, title,
                    last_connected, connect_count, display_id
             FROM creator_history ORDER BY last_connected DESC LIMIT ?",
        )?;
        let rows = statement.query_map([limit.clamp(0, 1000)], creator_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub(crate) fn load_gift_catalog(&self) -> Result<Vec<Value>, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let mut statement = connection.prepare(
            "SELECT id, name, diamond_count, icon_url FROM gift_catalog
             ORDER BY diamond_count ASC, name ASC",
        )?;
        let rows = statement.query_map([], |row| {
            let mut object = Map::new();
            object.insert("id".to_owned(), Value::String(row.get(0)?));
            object.insert("name".to_owned(), Value::String(row.get(1)?));
            object.insert(
                "diamondCount".to_owned(),
                Value::from(row.get::<_, i64>(2)?),
            );
            insert_optional_string(&mut object, "iconUrl", row.get(3)?);
            Ok(Value::Object(object))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub(crate) fn save_gift_catalog(&self, gifts: &[Value]) -> Result<(), DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let now = now();
        for gift in gifts {
            let Some(object) = gift.as_object() else {
                continue;
            };
            let Some(id) = object
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
            else {
                continue;
            };
            let Some(name) = object
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
            else {
                continue;
            };
            let diamond_count = object
                .get("diamondCount")
                .and_then(Value::as_u64)
                .unwrap_or_default()
                .min(i64::MAX as u64) as i64;
            connection.execute(
                "INSERT INTO gift_catalog (id, name, diamond_count, icon_url, updated_at)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   diamond_count = excluded.diamond_count,
                   icon_url = COALESCE(excluded.icon_url, gift_catalog.icon_url),
                   updated_at = excluded.updated_at",
                params![
                    id,
                    name,
                    diamond_count,
                    object.get("iconUrl").and_then(Value::as_str),
                    now,
                ],
            )?;
        }
        Ok(())
    }
}

fn creator_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let unique_id: String = row.get(0)?;
    let display_id: Option<String> = row.get(7)?;
    Ok(json!({
        "uniqueId": unique_id,
        "roomId": row.get::<_, Option<String>>(1)?,
        "nickname": row.get::<_, Option<String>>(2)?,
        "avatarUrl": row.get::<_, Option<String>>(3)?,
        "title": row.get::<_, Option<String>>(4)?,
        "lastConnected": row.get::<_, Option<i64>>(5)?.unwrap_or_default(),
        "connectCount": row.get::<_, Option<i64>>(6)?.unwrap_or(1),
        "displayId": display_id.unwrap_or(unique_id)
    }))
}
