use rusqlite::{params, Connection};
use serde_json::{Map, Value};

use crate::ipc::messages::PointsConfig;
#[cfg(feature = "persistence")]
use crate::services::{PointsWriteOp, ViewerState};

use super::{
    util::{bool_int, insert_optional_string, now},
    DatabaseError, DatabaseManager,
};

/// Row writer shared by the one-shot APIs and the owned points writer.
/// The writer thread prepares each statement once per batch and reuses it
/// for every op in that batch, so steady-state awards reuse compiled
/// statements without a statement-cache dependency.
const UPSERT_VIEWER_SQL: &str = "INSERT INTO viewers (
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
               last_seen = excluded.last_seen";

const INSERT_TRANSACTION_SQL: &str =
    "INSERT INTO points_transactions (unique_id, amount, reason, created_at)
                 VALUES (?, ?, ?, ?)";

const VIEWER_COLUMNS_SQL: &str = "SELECT unique_id, user_id, nickname, avatar_url, points, level,
                    is_subscriber, total_chats, total_coins, total_likes,
                    total_shares, first_seen, last_seen
             FROM viewers";

mod commands;
mod approved;
mod projection;
#[cfg(test)]
mod tests;
mod versions;

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
    /// Warms only the leaderboard head for the bounded hot cache. Large
    /// viewer populations never require loading the complete database.
    pub(crate) fn load_top_viewers(&self, limit: usize) -> Result<Vec<Value>, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let mut statement = connection.prepare(&format!(
            "{VIEWER_COLUMNS_SQL} ORDER BY points DESC, total_coins DESC, total_chats DESC, unique_id ASC LIMIT ?"
        ))?;
        let rows = statement.query_map([limit as i64], row_to_viewer_value)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Direct indexed lookup by the `viewers.unique_id` primary key. This is
    /// the durable leg of `PointsService::viewer`: O(index) regardless of
    /// leaderboard position, replacing the old first-1,000 scan.
    pub(crate) fn load_viewer_by_id(
        &self,
        unique_id: &str,
    ) -> Result<Option<Value>, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        let mut statement =
            connection.prepare(&format!("{VIEWER_COLUMNS_SQL} WHERE unique_id = ?"))?;
        let mut rows = statement.query_map([unique_id], row_to_viewer_value)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Opens the owned writer's long-lived connection: WAL so readers
    /// (leaderboard warm-up, viewer reloads) never block the writer, and a
    /// busy timeout so a momentarily locked database waits instead of
    /// failing the batch.
    pub(crate) fn open_points_writer(&self) -> Result<Connection, DatabaseError> {
        let connection = self.open(&self.points_path())?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000; PRAGMA synchronous=NORMAL;",
        )?;
        Ok(connection)
    }

    /// Folds one writer batch into a single transaction with cached prepared
    /// statements. FIFO order inside the batch preserves per-viewer event
    /// ordering; every nonzero award keeps its transaction row.
    pub(crate) fn apply_points_batch(
        &self,
        connection: &mut Connection,
        batch: &[PointsWriteOp],
    ) -> Result<(), DatabaseError> {
        // Statements prepare once per batch and are reused for every op in
        // it; the transaction folds the whole batch into one commit.
        let transaction = connection.transaction()?;
        let mut upsert = transaction.prepare(UPSERT_VIEWER_SQL)?;
        let mut insert_tx = transaction.prepare(INSERT_TRANSACTION_SQL)?;
        let mut adjust = transaction.prepare(
            "UPDATE viewers SET points = ?, level = ?, last_seen = ? WHERE unique_id = ?",
        )?;
        let mut reset_one =
            transaction.prepare("UPDATE viewers SET points = 0, level = 1 WHERE unique_id = ?")?;
        let mut reset_all = transaction.prepare("UPDATE viewers SET points = 0, level = 1")?;
        let mut clear_tx = transaction.prepare("DELETE FROM points_transactions")?;
        let mut versions = versions::Reservations::new(&transaction)?;
        for op in batch {
            match op {
                PointsWriteOp::Save {
                    viewer,
                    reason,
                    awarded,
                    ..
                } => {
                    upsert.execute(params![
                        viewer.unique_id.as_str(),
                        viewer.user_id.as_deref(),
                        Some(viewer.nickname.as_str()),
                        viewer.avatar_url.as_deref(),
                        viewer.points,
                        i64::from(viewer.level),
                        bool_int(viewer.is_subscriber),
                        viewer.total_chats,
                        viewer.total_coins,
                        viewer.total_likes,
                        viewer.total_shares,
                        viewer.first_seen,
                        viewer.last_seen,
                    ])?;
                    if *awarded != 0.0 {
                        insert_tx.execute(params![
                            viewer.unique_id.as_str(),
                            *awarded,
                            *reason,
                            now()
                        ])?;
                    }
                }
                PointsWriteOp::Adjust {
                    unique_id,
                    points,
                    level,
                    ..
                } => {
                    adjust.execute(params![*points, *level, now(), unique_id.as_str()])?;
                }
                PointsWriteOp::Reset { unique_id, .. } => match unique_id {
                    Some(unique_id) => {
                        reset_one.execute([unique_id.as_str()])?;
                        insert_tx.execute(params![unique_id.as_str(), 0.0, "reset", now()])?;
                    }
                    None => {
                        reset_all.execute([])?;
                        clear_tx.execute([])?;
                    }
                },
            }
            versions.reserve(&transaction, op)?;
        }
        drop(upsert);
        drop(insert_tx);
        drop(adjust);
        drop(reset_one);
        drop(reset_all);
        drop(clear_tx);
        drop(versions);
        transaction.commit()?;
        Ok(())
    }

    /// Applies one op on its own transaction. The writer uses this for the
    /// per-op fallback after a failed batch: an op that cannot be applied
    /// as a whole is retried as a whole and never half-applied, so a retry
    /// can never re-insert the award row it already wrote.
    pub(crate) fn apply_points_op(
        &self,
        connection: &mut Connection,
        op: &PointsWriteOp,
    ) -> Result<(), DatabaseError> {
        let transaction = connection.transaction()?;
        apply_points_op_on(&transaction, op)?;
        versions::Reservations::new(&transaction)?.reserve(&transaction, op)?;
        transaction.commit()?;
        Ok(())
    }
}

fn row_to_viewer_value(row: &rusqlite::Row<'_>) -> Result<Value, rusqlite::Error> {
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
}

/// Applies one writer op on a connection or an open transaction, using
/// cached prepared statements. Transaction rows are preserved for every
/// nonzero award; resets and adjustments keep their exact historical
/// semantics (adjust touches points/level only, full reset clears the
/// transaction history, single reset appends a zero-amount marker row).
fn apply_points_op_on(connection: &Connection, op: &PointsWriteOp) -> Result<(), DatabaseError> {
    match op {
        PointsWriteOp::Save {
            viewer,
            reason,
            awarded,
            ..
        } => {
            upsert_viewer_state(connection, viewer)?;
            if *awarded != 0.0 {
                connection
                    .prepare(INSERT_TRANSACTION_SQL)?
                    .execute(params![viewer.unique_id.as_str(), *awarded, *reason, now()])?;
            }
            Ok(())
        }
        PointsWriteOp::Adjust {
            unique_id,
            points,
            level,
            ..
        } => {
            connection
                .prepare(
                    "UPDATE viewers SET points = ?, level = ?, last_seen = ? WHERE unique_id = ?",
                )?
                .execute(params![*points, *level, now(), unique_id.as_str()])?;
            Ok(())
        }
        PointsWriteOp::Reset { unique_id, .. } => match unique_id {
            Some(unique_id) => {
                connection
                    .prepare("UPDATE viewers SET points = 0, level = 1 WHERE unique_id = ?")?
                    .execute([unique_id.as_str()])?;
                connection
                    .prepare(INSERT_TRANSACTION_SQL)?
                    .execute(params![unique_id.as_str(), 0.0, "reset", now()])?;
                Ok(())
            }
            None => {
                connection
                    .prepare("UPDATE viewers SET points = 0, level = 1")?
                    .execute([])?;
                connection
                    .prepare("DELETE FROM points_transactions")?
                    .execute([])?;
                Ok(())
            }
        },
    }
}

fn upsert_viewer_state(connection: &Connection, viewer: &ViewerState) -> Result<(), DatabaseError> {
    connection.prepare(UPSERT_VIEWER_SQL)?.execute(params![
        viewer.unique_id.as_str(),
        viewer.user_id.as_deref(),
        Some(viewer.nickname.as_str()),
        viewer.avatar_url.as_deref(),
        viewer.points,
        i64::from(viewer.level),
        bool_int(viewer.is_subscriber),
        viewer.total_chats,
        viewer.total_coins,
        viewer.total_likes,
        viewer.total_shares,
        viewer.first_seen,
        viewer.last_seen,
    ])?;
    Ok(())
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
