use rusqlite::Connection;

use crate::ipc::messages::PointsConfig;

use super::{points::insert_points_config, util::now, DatabaseError, DatabaseManager};

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
    pub(in crate::db) fn initialize_schema(&self) -> Result<(), DatabaseError> {
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
    pub(in crate::db) fn open(&self, path: &std::path::Path) -> Result<Connection, DatabaseError> {
        Ok(Connection::open(path)?)
    }
}
