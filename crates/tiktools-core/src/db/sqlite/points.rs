use rusqlite::{params, Connection};
use serde_json::{Map, Value};

use crate::ipc::messages::PointsConfig;

use super::{
    util::{bool_int, insert_optional_string, now, required_value_string},
    DatabaseError, DatabaseManager,
};

impl DatabaseManager {
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
}

pub(crate) fn insert_points_config(
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
