//! SQLite schema and conservative persistence helpers.
//!
//! The SQL intentionally mirrors the current `src/db/points-db.ts` and
//! `src/db/automation-db.ts` tables. Domain payloads that have not yet gained
//! a Rust type stay as JSON so an existing database can be opened without a
//! destructive migration.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use thiserror::Error;

use crate::ipc::messages::PointsConfig;

use super::DatabaseManager;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stored record is invalid: {0}")]
    Invalid(String),
}

const POINTS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS points_config (
  id INTEGER PRIMARY KEY,
  currency_name TEXT DEFAULT 'Points',
  points_per_coin REAL DEFAULT 1.0,
  points_per_coin_enabled INTEGER DEFAULT 1,
  points_per_share REAL DEFAULT 3.0,
  points_per_share_enabled INTEGER DEFAULT 1,
  points_per_chat REAL DEFAULT 1.0,
  points_per_chat_enabled INTEGER DEFAULT 1,
  points_per_like REAL DEFAULT 0.1,
  points_per_like_enabled INTEGER DEFAULT 1,
  points_per_follow REAL DEFAULT 5.0,
  points_per_follow_enabled INTEGER DEFAULT 1,
  points_per_join REAL DEFAULT 0.5,
  points_per_join_enabled INTEGER DEFAULT 0,
  sub_bonus_multiplier REAL DEFAULT 0.0,
  points_per_level INTEGER DEFAULT 100,
  updated_at INTEGER DEFAULT 0
);
CREATE TABLE IF NOT EXISTS viewers (
  unique_id TEXT PRIMARY KEY,
  user_id TEXT,
  nickname TEXT,
  avatar_url TEXT,
  points REAL DEFAULT 0,
  level INTEGER DEFAULT 1,
  is_subscriber INTEGER DEFAULT 0,
  total_chats INTEGER DEFAULT 0,
  total_coins INTEGER DEFAULT 0,
  total_likes INTEGER DEFAULT 0,
  total_shares INTEGER DEFAULT 0,
  first_seen INTEGER,
  last_seen INTEGER
);
CREATE TABLE IF NOT EXISTS points_transactions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  unique_id TEXT,
  amount REAL,
  reason TEXT,
  metadata TEXT,
  created_at INTEGER
);
CREATE TABLE IF NOT EXISTS creator_history (
  unique_id TEXT PRIMARY KEY,
  room_id TEXT,
  nickname TEXT,
  avatar_url TEXT,
  title TEXT,
  last_connected INTEGER,
  connect_count INTEGER DEFAULT 1,
  display_id TEXT
);
CREATE TABLE IF NOT EXISTS app_state (
  key TEXT PRIMARY KEY,
  value TEXT,
  updated_at INTEGER
);
CREATE TABLE IF NOT EXISTS gift_catalog (
  id TEXT PRIMARY KEY,
  name TEXT,
  diamond_count INTEGER DEFAULT 0,
  icon_url TEXT,
  updated_at INTEGER
);
"#;

const AUTOMATION_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS automation_workflows (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  graph_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS behavior_actions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  payload_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS behavior_events (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  payload_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS behavior_plugins (
  id TEXT PRIMARY KEY,
  installed INTEGER NOT NULL DEFAULT 0,
  enabled INTEGER NOT NULL DEFAULT 1,
  updated_at INTEGER NOT NULL
);
"#;

impl DatabaseManager {
    pub(super) fn initialize_schema(&self) -> Result<(), DatabaseError> {
        let points = self.open(&self.points_path())?;
        points.execute_batch(POINTS_SCHEMA)?;
        let count: i64 =
            points.query_row("SELECT COUNT(*) FROM points_config", [], |row| row.get(0))?;
        if count == 0 {
            let config = PointsConfig::default();
            insert_points_config(&points, &config, now())?;
        }

        let automation = self.open(&self.automation_path())?;
        automation.execute_batch(AUTOMATION_SCHEMA)?;

        self.ensure_analytics_schema()?;
        Ok(())
    }

    pub(super) fn open(&self, path: &std::path::Path) -> Result<Connection, DatabaseError> {
        Ok(Connection::open(path)?)
    }

    pub(crate) fn load_points_config(&self) -> Result<PointsConfig, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let config = connection.query_row(
            "SELECT currency_name, points_per_coin, points_per_coin_enabled,
                    points_per_share, points_per_share_enabled, points_per_chat,
                    points_per_chat_enabled, points_per_like, points_per_like_enabled,
                    points_per_follow, points_per_follow_enabled, points_per_join,
                    points_per_join_enabled, sub_bonus_multiplier, points_per_level
             FROM points_config WHERE id = 1",
            [],
            |row| {
                Ok(PointsConfig {
                    currency_name: row.get(0)?,
                    points_per_coin: row.get(1)?,
                    points_per_coin_enabled: row.get::<_, i64>(2)? != 0,
                    points_per_share: row.get(3)?,
                    points_per_share_enabled: row.get::<_, i64>(4)? != 0,
                    points_per_chat: row.get(5)?,
                    points_per_chat_enabled: row.get::<_, i64>(6)? != 0,
                    points_per_like: row.get(7)?,
                    points_per_like_enabled: row.get::<_, i64>(8)? != 0,
                    points_per_follow: row.get(9)?,
                    points_per_follow_enabled: row.get::<_, i64>(10)? != 0,
                    points_per_join: row.get(11)?,
                    points_per_join_enabled: row.get::<_, i64>(12)? != 0,
                    sub_bonus_multiplier: row.get(13)?,
                    points_per_level: row.get(14)?,
                })
            },
        )?;
        Ok(config)
    }

    pub(crate) fn save_points_config(&self, config: &PointsConfig) -> Result<(), DatabaseError> {
        let connection = self.open(&self.points_path())?;
        connection.execute(
            "UPDATE points_config SET currency_name = ?, points_per_coin = ?,
             points_per_coin_enabled = ?, points_per_share = ?,
             points_per_share_enabled = ?, points_per_chat = ?,
             points_per_chat_enabled = ?, points_per_like = ?,
             points_per_like_enabled = ?, points_per_follow = ?,
             points_per_follow_enabled = ?, points_per_join = ?,
             points_per_join_enabled = ?, sub_bonus_multiplier = ?,
             points_per_level = ?, updated_at = ? WHERE id = 1",
            params![
                config.currency_name,
                config.points_per_coin,
                bool_int(config.points_per_coin_enabled),
                config.points_per_share,
                bool_int(config.points_per_share_enabled),
                config.points_per_chat,
                bool_int(config.points_per_chat_enabled),
                config.points_per_like,
                bool_int(config.points_per_like_enabled),
                config.points_per_follow,
                bool_int(config.points_per_follow_enabled),
                config.points_per_join,
                bool_int(config.points_per_join_enabled),
                config.sub_bonus_multiplier,
                config.points_per_level,
                now(),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn load_point_viewers(&self) -> Result<Vec<Value>, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let mut statement = connection.prepare(
            "SELECT unique_id, user_id, nickname, avatar_url, points, level,
                    is_subscriber, total_chats, total_coins, total_likes,
                    total_shares, first_seen, last_seen
             FROM viewers ORDER BY points DESC, total_coins DESC, total_chats DESC",
        )?;
        let rows = statement.query_map([], |row| {
            let mut object = Map::new();
            object.insert("uniqueId".to_owned(), Value::String(row.get(0)?));
            insert_optional_string(&mut object, "userId", row.get(1)?);
            insert_optional_string(&mut object, "nickname", row.get(2)?);
            insert_optional_string(&mut object, "avatarUrl", row.get(3)?);
            object.insert("points".to_owned(), Value::from(row.get::<_, f64>(4)?));
            object.insert("level".to_owned(), Value::from(row.get::<_, i64>(5)?));
            object.insert(
                "isSubscriber".to_owned(),
                Value::from(row.get::<_, i64>(6)? != 0),
            );
            object.insert("totalChats".to_owned(), Value::from(row.get::<_, i64>(7)?));
            object.insert("totalCoins".to_owned(), Value::from(row.get::<_, i64>(8)?));
            object.insert("totalLikes".to_owned(), Value::from(row.get::<_, i64>(9)?));
            object.insert(
                "totalShares".to_owned(),
                Value::from(row.get::<_, i64>(10)?),
            );
            object.insert("firstSeen".to_owned(), Value::from(row.get::<_, i64>(11)?));
            object.insert("lastSeen".to_owned(), Value::from(row.get::<_, i64>(12)?));
            Ok(Value::Object(object))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn update_viewer_points(
        &self,
        unique_id: &str,
        points: f64,
        level: u32,
    ) -> Result<bool, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let changed = connection.execute(
            "UPDATE viewers SET points = ?, level = ?, last_seen = ? WHERE unique_id = ?",
            params![points, level, now(), unique_id],
        )?;
        Ok(changed > 0)
    }

    pub(crate) fn save_viewer(
        &self,
        viewer: &Value,
        reason: &str,
        awarded: f64,
    ) -> Result<(), DatabaseError> {
        let object = viewer
            .as_object()
            .ok_or_else(|| DatabaseError::Invalid("viewer record must be an object".to_owned()))?;
        let unique_id = required_value_string(object, "uniqueId")?;
        let connection = self.open(&self.points_path())?;
        connection.execute(
            "INSERT INTO viewers (
             unique_id, user_id, nickname, avatar_url, points, level,
             is_subscriber, total_chats, total_coins, total_likes, total_shares,
             first_seen, last_seen)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(unique_id) DO UPDATE SET
               user_id = COALESCE(excluded.user_id, viewers.user_id),
               nickname = COALESCE(excluded.nickname, viewers.nickname),
               avatar_url = COALESCE(excluded.avatar_url, viewers.avatar_url),
               points = excluded.points,
               level = excluded.level,
               is_subscriber = excluded.is_subscriber,
               total_chats = excluded.total_chats,
               total_coins = excluded.total_coins,
               total_likes = excluded.total_likes,
               total_shares = excluded.total_shares,
               first_seen = COALESCE(viewers.first_seen, excluded.first_seen),
               last_seen = excluded.last_seen",
            params![
                unique_id,
                object.get("userId").and_then(Value::as_str),
                object.get("nickname").and_then(Value::as_str),
                object.get("avatarUrl").and_then(Value::as_str),
                object
                    .get("points")
                    .and_then(Value::as_f64)
                    .unwrap_or_default(),
                object.get("level").and_then(Value::as_i64).unwrap_or(1),
                bool_int(
                    object
                        .get("isSubscriber")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                ),
                object
                    .get("totalChats")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
                object
                    .get("totalCoins")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
                object
                    .get("totalLikes")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
                object
                    .get("totalShares")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
                object
                    .get("firstSeen")
                    .and_then(Value::as_i64)
                    .unwrap_or_else(now),
                object
                    .get("lastSeen")
                    .and_then(Value::as_i64)
                    .unwrap_or_else(now),
            ],
        )?;
        if awarded != 0.0 {
            connection.execute(
                "INSERT INTO points_transactions (unique_id, amount, reason, created_at)
                 VALUES (?, ?, ?, ?)",
                params![unique_id, awarded, reason, now()],
            )?;
        }
        Ok(())
    }

    pub(crate) fn reset_viewers(&self, unique_id: Option<&str>) -> Result<(), DatabaseError> {
        let connection = self.open(&self.points_path())?;
        match unique_id {
            Some(unique_id) => {
                connection.execute(
                    "UPDATE viewers SET points = 0, level = 1 WHERE unique_id = ?",
                    [unique_id],
                )?;
                connection.execute(
                    "INSERT INTO points_transactions (unique_id, amount, reason, created_at)
                     VALUES (?, 0, 'reset', ?)",
                    params![unique_id, now()],
                )?;
            }
            None => {
                connection.execute("UPDATE viewers SET points = 0, level = 1", [])?;
                connection.execute("DELETE FROM points_transactions", [])?;
            }
        }
        Ok(())
    }

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

    pub(crate) fn load_behavior_snapshot(&self) -> Result<Value, DatabaseError> {
        let actions = self.load_behavior_rows("behavior_actions")?;
        let events = self.load_behavior_rows("behavior_events")?;
        let plugins = self.load_plugin_states()?;
        Ok(json!({
            "actions": actions,
            "events": events,
            "plugins": plugins,
            "actionTypes": [],
            "translations": {}
        }))
    }

    pub(crate) fn save_behavior(&self, table: &str, value: &Value) -> Result<Value, DatabaseError> {
        let object = value.as_object().ok_or_else(|| {
            DatabaseError::Invalid("behavior record must be an object".to_owned())
        })?;
        let id = required_value_string(object, "id")?;
        let name = required_value_string(object, "name")?;
        let enabled = object
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let connection = self.open(&self.automation_path())?;
        let existing: Option<i64> = connection
            .query_row(
                &format!("SELECT created_at FROM {table} WHERE id = ?"),
                [&id],
                |row| row.get(0),
            )
            .optional()?;
        let created_at = existing.unwrap_or_else(now);
        let updated_at = now();
        connection.execute(
            &format!(
                "INSERT INTO {table} (id, name, enabled, payload_json, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name,
                   enabled = excluded.enabled, payload_json = excluded.payload_json,
                   updated_at = excluded.updated_at"
            ),
            params![
                id,
                name,
                bool_int(enabled),
                serde_json::to_string(value)?,
                created_at,
                updated_at
            ],
        )?;
        Ok(value.clone())
    }

    pub(crate) fn delete_behavior(&self, table: &str, id: &str) -> Result<bool, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let changed = connection.execute(&format!("DELETE FROM {table} WHERE id = ?"), [id])?;
        if changed > 0 && table == "behavior_actions" {
            remove_action_references(&connection, id)?;
        }
        Ok(changed > 0)
    }

    pub(crate) fn set_behavior_enabled(
        &self,
        table: &str,
        id: &str,
        enabled: bool,
    ) -> Result<Value, DatabaseError> {
        let value = self
            .load_behavior_rows(table)?
            .into_iter()
            .find(|value| value.get("id").and_then(Value::as_str) == Some(id))
            .ok_or_else(|| DatabaseError::Invalid(format!("unknown behavior: {id}")))?;
        let mut value = value;
        value["enabled"] = Value::Bool(enabled);
        self.save_behavior(table, &value)
    }

    pub(crate) fn set_plugin_state(
        &self,
        id: &str,
        installed: bool,
        enabled: bool,
    ) -> Result<Value, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        connection.execute(
            "INSERT INTO behavior_plugins (id, installed, enabled, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET installed = excluded.installed,
               enabled = excluded.enabled, updated_at = excluded.updated_at",
            params![id, bool_int(installed), bool_int(enabled), now()],
        )?;
        Ok(json!({"id": id, "installed": installed, "enabled": enabled}))
    }

    pub(crate) fn remove_plugin_state(&self, id: &str) -> Result<(), DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        connection.execute("DELETE FROM behavior_plugins WHERE id = ?", [id])?;
        Ok(())
    }

    fn load_behavior_rows(&self, table: &str) -> Result<Vec<Value>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare(&format!(
            "SELECT id, name, enabled, payload_json, created_at, updated_at
             FROM {table} ORDER BY updated_at DESC, name ASC"
        ))?;
        let rows = statement.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let enabled: bool = row.get::<_, i64>(2)? != 0;
            let payload: String = row.get(3)?;
            Ok((id, name, enabled, payload))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (id, name, enabled, payload) = row?;
            let mut object = serde_json::from_str::<Value>(&payload)?
                .as_object()
                .cloned()
                .ok_or_else(|| DatabaseError::Invalid(format!("{table}/{id} is not an object")))?;
            object.insert("id".to_owned(), Value::String(id));
            object.insert("name".to_owned(), Value::String(name));
            object.insert("enabled".to_owned(), Value::Bool(enabled));
            result.push(Value::Object(object));
        }
        Ok(result)
    }

    fn load_plugin_states(&self) -> Result<Vec<Value>, DatabaseError> {
        let connection = self.open(&self.automation_path())?;
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

fn insert_points_config(
    connection: &Connection,
    config: &PointsConfig,
    updated_at: i64,
) -> Result<(), DatabaseError> {
    connection.execute(
        "INSERT INTO points_config (
         id, currency_name, points_per_coin, points_per_coin_enabled,
         points_per_share, points_per_share_enabled, points_per_chat,
         points_per_chat_enabled, points_per_like, points_per_like_enabled,
         points_per_follow, points_per_follow_enabled, points_per_join,
         points_per_join_enabled, sub_bonus_multiplier, points_per_level, updated_at)
         VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            config.currency_name,
            config.points_per_coin,
            bool_int(config.points_per_coin_enabled),
            config.points_per_share,
            bool_int(config.points_per_share_enabled),
            config.points_per_chat,
            bool_int(config.points_per_chat_enabled),
            config.points_per_like,
            bool_int(config.points_per_like_enabled),
            config.points_per_follow,
            bool_int(config.points_per_follow_enabled),
            config.points_per_join,
            bool_int(config.points_per_join_enabled),
            config.sub_bonus_multiplier,
            config.points_per_level,
            updated_at,
        ],
    )?;
    Ok(())
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

fn remove_action_references(connection: &Connection, action_id: &str) -> Result<(), DatabaseError> {
    let mut statement = connection
        .prepare("SELECT id, payload_json FROM behavior_events WHERE payload_json IS NOT NULL")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut updates = Vec::new();
    for row in rows {
        let (id, payload) = row?;
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
        action_ids.retain(|value| value.as_str() != Some(action_id));
        if action_ids.len() != original {
            updates.push((id, Value::Object(object)));
        }
    }
    drop(statement);
    for (id, value) in updates {
        let object = value
            .as_object()
            .ok_or_else(|| DatabaseError::Invalid("event is not an object".to_owned()))?;
        let name = required_value_string(object, "name")?;
        let enabled = object
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        connection.execute(
            "UPDATE behavior_events SET name = ?, enabled = ?, payload_json = ?, updated_at = ? WHERE id = ?",
            params![name, bool_int(enabled), serde_json::to_string(&value)?, now(), id],
        )?;
    }
    Ok(())
}

fn required_value_string(object: &Map<String, Value>, key: &str) -> Result<String, DatabaseError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| DatabaseError::Invalid(format!("record field `{key}` is missing")))
}

fn insert_optional_string(object: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        object.insert(key.to_owned(), Value::String(value));
    }
}

fn bool_int(value: bool) -> i64 {
    i64::from(value)
}

fn now() -> i64 {
    chrono_like_now()
}

fn chrono_like_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::paths::AppPaths;

    fn paths() -> AppPaths {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tiktools-rust-sqlite-{suffix}"));
        AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin-plugins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root,
        }
    }

    #[test]
    fn initializes_compatible_schemas_and_round_trips_records() {
        let paths = paths();
        paths.ensure_directories().unwrap();
        let database = DatabaseManager::new(paths.clone());
        let config = database.load_points_config().unwrap();
        assert_eq!(config.currency_name, "Points");

        let graph = json!({
            "schemaVersion": 1,
            "id": "workflow-1",
            "name": "Demo",
            "enabled": false,
            "nodes": [],
            "edges": []
        });
        let saved = database.save_workflow(&graph).unwrap();
        assert_eq!(saved["id"], "workflow-1");
        assert_eq!(database.load_workflows().unwrap().len(), 1);

        let action = json!({"id":"action-1","name":"Action","enabled":true,"type":"code"});
        database.save_behavior("behavior_actions", &action).unwrap();
        let snapshot = database.load_behavior_snapshot().unwrap();
        assert_eq!(snapshot["actions"][0]["id"], "action-1");

        let _ = fs::remove_dir_all(paths.root);
    }
}
