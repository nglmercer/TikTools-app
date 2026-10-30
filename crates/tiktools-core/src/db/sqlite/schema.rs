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
-- Justified indexes: per-viewer transaction history lookups and the
-- leaderboard warm-up / direct viewer reloads that replace the old
-- full-table scans. Both serve the bounded hot-cache design.
CREATE INDEX IF NOT EXISTS idx_points_transactions_viewer ON points_transactions(unique_id);
CREATE INDEX IF NOT EXISTS idx_viewers_points ON viewers(points DESC);
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
CREATE TABLE IF NOT EXISTS aggregation_definitions (
  id TEXT PRIMARY KEY,
  creator_id TEXT NOT NULL,
  room_id TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  definition_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_aggregation_scope ON aggregation_definitions(creator_id, room_id);
-- Latest ranking snapshot per definition, rewritten periodically while running
-- and at clean shutdown. Rows only ever lag live state (undercount, never replay);
-- aggregation_checkpoint_run records whether the last run closed cleanly.
CREATE TABLE IF NOT EXISTS aggregation_session_checkpoints (
  ranking_id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL,
  checkpoint_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS aggregation_checkpoint_owner (
  singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
  owner TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS aggregation_checkpoint_run (
  singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
  status TEXT NOT NULL CHECK(status IN ('open', 'clean'))
);
-- Idempotency keys for externally reported ranking contributions (plugins,
-- agents). A key is recorded before the score is applied: a retry after a
-- crash undercounts at most once and can never double count.
CREATE TABLE IF NOT EXISTS aggregation_external_events (
  event_id TEXT PRIMARY KEY,
  accepted_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_aggregation_external_age ON aggregation_external_events(accepted_at);
CREATE TABLE IF NOT EXISTS competition_definitions (
  id TEXT PRIMARY KEY,
  ranking_id TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  definition_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_competition_ranking ON competition_definitions(ranking_id);
CREATE TABLE IF NOT EXISTS competition_state (
  competition_id TEXT PRIMARY KEY,
  state_json TEXT NOT NULL
);
INSERT OR IGNORE INTO competition_state(competition_id, state_json)
  SELECT id, '{"lifecycle":"draft","generation":0,"deadlineMs":null,"remainingMs":null}' FROM competition_definitions;
-- A generation is marked open before it accepts votes. Only a drained host
-- may checkpoint it as clean; an unclean restart must not forget possible loss.
CREATE TABLE IF NOT EXISTS competition_integrity_checkpoints (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('open','clean','degraded')),
  PRIMARY KEY (competition_id, generation)
);
CREATE TABLE IF NOT EXISTS competition_integrity_owners (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  owner TEXT NOT NULL,
  PRIMARY KEY (competition_id, generation)
);
CREATE TABLE IF NOT EXISTS competition_participation_counts (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  participants INTEGER NOT NULL DEFAULT 0,
  contributions INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (competition_id, generation)
);
CREATE TABLE IF NOT EXISTS competition_participants (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  participant_id TEXT NOT NULL,
  option_id TEXT NOT NULL,
  votes INTEGER NOT NULL CHECK(votes > 0),
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (competition_id, generation, participant_id)
);
CREATE INDEX IF NOT EXISTS idx_competition_participants_option ON competition_participants(competition_id, generation, option_id);
CREATE TABLE IF NOT EXISTS competition_contributions (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  event_identity TEXT NOT NULL,
  participant_id TEXT NOT NULL,
  option_id TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (competition_id, generation, event_identity)
);
CREATE INDEX IF NOT EXISTS idx_competition_contributions_created ON competition_contributions(competition_id, generation, created_at);
-- One receipt per canonical source event, including events ignored before
-- selection or during pause. Scores are decimal text to retain full u64.
-- High-water marks are retained for the entire competition generation;
-- they are updated in the same transaction as contribution admission.
-- Gift-only voters count as participants without creating a command selection.
CREATE TABLE IF NOT EXISTS competition_gift_participants (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  participant_id TEXT NOT NULL,
  PRIMARY KEY (competition_id, generation, participant_id)
);
CREATE TABLE IF NOT EXISTS competition_gift_receipts (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  event_identity TEXT NOT NULL,
  participant_id TEXT NOT NULL,
  gift_id TEXT NOT NULL,
  group_id TEXT NOT NULL,
  observed_quantity TEXT NOT NULL,
  price TEXT NOT NULL,
  PRIMARY KEY (competition_id, generation, event_identity)
);
CREATE TABLE IF NOT EXISTS competition_gift_combo_prices (
  competition_id TEXT NOT NULL, generation INTEGER NOT NULL, participant_id TEXT NOT NULL,
  gift_id TEXT NOT NULL, group_id TEXT NOT NULL, price TEXT NOT NULL,
  PRIMARY KEY (competition_id, generation, participant_id, gift_id, group_id)
);
CREATE TABLE IF NOT EXISTS competition_gift_combo_counts (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  count INTEGER NOT NULL CHECK(count >= 0),
  PRIMARY KEY (competition_id, generation)
);
CREATE TABLE IF NOT EXISTS competition_gift_combos (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  participant_id TEXT NOT NULL,
  gift_id TEXT NOT NULL,
  group_id TEXT NOT NULL,
  quantity TEXT NOT NULL,
  diamonds TEXT NOT NULL,
  PRIMARY KEY (competition_id, generation, participant_id, gift_id, group_id)
);
CREATE TABLE IF NOT EXISTS competition_scored_contributions (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  source TEXT NOT NULL,
  event_identity TEXT NOT NULL,
  participant_id TEXT NOT NULL,
  option_id TEXT,
  score TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (competition_id, generation, source, event_identity)
);
CREATE INDEX IF NOT EXISTS idx_competition_scores_option ON competition_scored_contributions(competition_id, generation, option_id);
CREATE TABLE IF NOT EXISTS competition_results (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  ranking_id TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  result_json TEXT NOT NULL,
  PRIMARY KEY (competition_id, generation)
);
CREATE INDEX IF NOT EXISTS idx_competition_results_created ON competition_results(competition_id, created_at);
CREATE TABLE IF NOT EXISTS competition_action_outbox (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  action_id TEXT NOT NULL,
  idempotency_key TEXT NOT NULL UNIQUE,
  action_json TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'uncertain')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (competition_id, generation),
  FOREIGN KEY (competition_id, generation) REFERENCES competition_results(competition_id, generation)
);
CREATE INDEX IF NOT EXISTS idx_competition_outbox_status ON competition_action_outbox(status, created_at);
-- Separate table is a non-destructive upgrade for existing outboxes.
CREATE TABLE IF NOT EXISTS competition_action_diagnostics (
  competition_id TEXT NOT NULL,
  generation INTEGER NOT NULL,
  reason TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (competition_id, generation),
  FOREIGN KEY (competition_id, generation) REFERENCES competition_action_outbox(competition_id, generation)
);

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
