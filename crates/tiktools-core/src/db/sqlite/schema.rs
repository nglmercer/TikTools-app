use rusqlite::Connection;

use crate::ipc::messages::PointsConfig;

use super::{points::insert_points_config, util::now, DatabaseError, DatabaseManager};

// Shared definitions, instantiated in each business database so resource writes
// and their coordination reservations always share a SQLite transaction.
const COORDINATION_SCHEMA: &str = r#"
-- Co-located with business writes: compare-and-set, tombstones and invalidation
-- reservation commit together, including writes from legacy local callers.
CREATE TABLE IF NOT EXISTS control_resource_versions (
  kind TEXT NOT NULL,
  scope TEXT NOT NULL,
  resource_id TEXT NOT NULL,
  revision INTEGER NOT NULL CHECK(revision > 0),
  deleted INTEGER NOT NULL CHECK(deleted IN (0, 1)),
  PRIMARY KEY(kind, scope, resource_id)
);
CREATE TABLE IF NOT EXISTS control_resource_outbox (
  cursor INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL,
  scope TEXT NOT NULL,
  resource_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  change TEXT NOT NULL,
  committed_at INTEGER NOT NULL,
  UNIQUE(kind, scope, resource_id, revision)
);
CREATE TABLE IF NOT EXISTS control_commands (
  principal_id TEXT NOT NULL,
  command_id TEXT NOT NULL,
  payload_hash TEXT NOT NULL,
  method TEXT NOT NULL,
  resource_kind TEXT NOT NULL,
  resource_scope TEXT NOT NULL,
  resource_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  host_instance_id TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('accepted','running','succeeded','failed','unknown')),
  committed_at INTEGER NOT NULL,
  result_json TEXT,
  versions_json TEXT,
  outcome_expired INTEGER NOT NULL DEFAULT 0 CHECK(outcome_expired IN (0,1)),
  PRIMARY KEY(principal_id, command_id)
);
CREATE INDEX IF NOT EXISTS idx_control_commands_retention ON control_commands(committed_at);
-- Audit delivery may retry independently of the original business command.
-- No params, graphs, scripts, credentials or chat are copied into this outbox.
CREATE TABLE IF NOT EXISTS control_command_outbox (
  event_id TEXT PRIMARY KEY,
  principal_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  command_id TEXT NOT NULL,
  method TEXT NOT NULL,
  resource_kind TEXT NOT NULL,
  resource_scope TEXT NOT NULL,
  resource_id TEXT NOT NULL,
  before_revision TEXT NOT NULL,
  after_revision TEXT NOT NULL,
  outcome_code TEXT NOT NULL,
  committed_at INTEGER NOT NULL,
  dispatched_at INTEGER,
  UNIQUE(principal_id, command_id)
);
CREATE TABLE IF NOT EXISTS control_outbox_delivery (
  resource_cursor INTEGER PRIMARY KEY REFERENCES control_resource_outbox(cursor) ON DELETE CASCADE,
  delivered_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS control_command_resources (
  event_id TEXT PRIMARY KEY REFERENCES control_command_outbox(event_id) ON DELETE CASCADE,
  resource_cursor INTEGER NOT NULL UNIQUE REFERENCES control_resource_outbox(cursor) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_control_command_delivery ON control_command_outbox(committed_at,event_id) WHERE dispatched_at IS NULL;
-- Old command reservations can be linked without confusing a no-op delete
-- with the earlier command that actually produced the same resource revision.
INSERT OR IGNORE INTO control_command_resources(event_id,resource_cursor)
  SELECT c.event_id,r.cursor FROM control_command_outbox c JOIN control_resource_outbox r
  ON c.resource_kind=r.kind AND c.resource_scope=r.scope AND c.resource_id=r.resource_id
  AND c.after_revision=CAST(r.revision AS TEXT) WHERE c.before_revision<>c.after_revision;
"#;

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
-- Remote and cold-cache boards need the complete deterministic order. The
-- original points-only index stays compatible with older local consumers.
CREATE INDEX IF NOT EXISTS idx_viewers_rank ON viewers(points DESC,total_coins DESC,total_chats DESC,unique_id ASC);
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
-- Fixed-size private metadata only. Identity tombstones are never pruned:
-- expiration can hide an outcome but must never authorize another dispatch.
CREATE TABLE IF NOT EXISTS automation_external_effect_host (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1),
  instance_id TEXT NOT NULL CHECK(length(instance_id)=64)
);
CREATE TABLE IF NOT EXISTS automation_external_effects (
  effect_id TEXT PRIMARY KEY CHECK(length(effect_id)=64),
  kind TEXT NOT NULL CHECK(kind IN ('http','audio','plugin','emit','script')),
  binding_digest TEXT NOT NULL CHECK(length(binding_digest)=64),
  request_digest TEXT NOT NULL CHECK(length(request_digest)=64),
  instance_id TEXT NOT NULL CHECK(length(instance_id)=64),
  state TEXT NOT NULL CHECK(state IN ('accepted','dispatched','succeeded','failed','unknown')),
  accepted_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  outcome_expired INTEGER NOT NULL DEFAULT 0 CHECK(outcome_expired IN (0,1))
);
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
INSERT OR IGNORE INTO control_resource_versions(kind, scope, resource_id, revision, deleted)
  SELECT 'workflow', 'host', id, 1, 0 FROM automation_workflows;
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
-- Metadata only: the owner's WebView/browser never owns a durable command ID
-- or stores proposal source. One unresolved authority transfer spans clients.
CREATE TABLE IF NOT EXISTS owner_automation_adoptions (
  command_id TEXT PRIMARY KEY,
  pending INTEGER NOT NULL CHECK(pending IN (0,1)),
  payload_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_owner_adoption_pending
  ON owner_automation_adoptions(pending) WHERE pending=1;
-- Metadata-only approval/revocation intents share native admission with the
-- adoption journal. Reviewed versions/digests/plans contain no source or grant.
CREATE TABLE IF NOT EXISTS owner_automation_approvals (
  command_id TEXT PRIMARY KEY,
  pending INTEGER NOT NULL CHECK(pending IN (0,1)),
  payload_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_owner_approval_pending
  ON owner_automation_approvals(pending) WHERE pending=1;
-- Single-record Owner saves, deletes and toggles retain only native metadata.
-- All three native journals check each other's pending state in one transaction.
CREATE TABLE IF NOT EXISTS owner_automation_definitions (
  command_id TEXT PRIMARY KEY,
  pending INTEGER NOT NULL CHECK(pending IN (0,1)),
  payload_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_owner_definition_pending
  ON owner_automation_definitions(pending) WHERE pending=1;
-- Approval identities survive definition deletion and command-result pruning.
-- No legacy enabled record acquires an approval during migration.
CREATE TABLE IF NOT EXISTS behavior_execution_approvals (
  scope TEXT NOT NULL CHECK(scope IN ('action','event')),
  resource_id TEXT NOT NULL,
  generation INTEGER NOT NULL CHECK(generation > 0),
  state TEXT NOT NULL CHECK(state IN ('active','invalidated','revoked')),
  policy_version INTEGER NOT NULL CHECK(policy_version > 0),
  manifest_digest TEXT NOT NULL,
  binding_json TEXT NOT NULL,
  owner_principal_id TEXT NOT NULL,
  owner_session_id TEXT NOT NULL,
  approved_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(scope,resource_id)
);
CREATE TABLE IF NOT EXISTS behavior_execution_approval_dependencies (
  approval_scope TEXT NOT NULL CHECK(approval_scope IN ('action','event')),
  approval_id TEXT NOT NULL,
  position INTEGER NOT NULL CHECK(position >= 0 AND position <= 512),
  definition_scope TEXT NOT NULL CHECK(definition_scope IN ('action','event')),
  definition_id TEXT NOT NULL,
  revision INTEGER NOT NULL CHECK(revision >= 0),
  provenance_generation INTEGER CHECK(provenance_generation > 0),
  present INTEGER NOT NULL CHECK(present IN (0,1)),
  content_digest TEXT,
  PRIMARY KEY(approval_scope,approval_id,position),
  UNIQUE(approval_scope,approval_id,definition_scope,definition_id)
);
CREATE INDEX IF NOT EXISTS idx_behavior_execution_approval_reverse
  ON behavior_execution_approval_dependencies(definition_scope,definition_id);
-- Authority is host-owned and independent of editable JSON. Tombstones retain
-- provenance so delete/recreate cannot claim an owner/other-principal ID.
BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS behavior_provenance (
  scope TEXT NOT NULL CHECK(scope IN ('action','event')),
  resource_id TEXT NOT NULL,
  authority TEXT NOT NULL CHECK(authority IN ('owner','delegated')),
  principal_id TEXT,
  session_id TEXT,
  generation INTEGER NOT NULL CHECK(generation > 0),
  PRIMARY KEY(scope,resource_id),
  CHECK((authority='owner' AND principal_id IS NULL AND session_id IS NULL) OR
        (authority='delegated' AND principal_id IS NOT NULL AND length(principal_id)>0 AND session_id IS NOT NULL AND length(session_id)>0))
);
CREATE TABLE IF NOT EXISTS behavior_provenance_migration (
  id INTEGER PRIMARY KEY CHECK(id=1),
  version INTEGER NOT NULL CHECK(version=1)
);
-- Seed legacy versions with authority once; a later missing version is corrupt
-- retained history, not a newly discovered legacy definition.
INSERT OR IGNORE INTO control_resource_versions(kind,scope,resource_id,revision,deleted)
  SELECT 'automation','action',id,1,0 FROM behavior_actions
  WHERE NOT EXISTS(SELECT 1 FROM behavior_provenance_migration WHERE id=1);
INSERT OR IGNORE INTO control_resource_versions(kind,scope,resource_id,revision,deleted)
  SELECT 'automation','event',id,1,0 FROM behavior_events
  WHERE NOT EXISTS(SELECT 1 FROM behavior_provenance_migration WHERE id=1);
INSERT OR IGNORE INTO behavior_provenance(scope,resource_id,authority,generation)
  SELECT scope,resource_id,'owner',1 FROM control_resource_versions
  WHERE kind='automation' AND scope IN ('action','event')
    AND NOT EXISTS(SELECT 1 FROM behavior_provenance_migration WHERE id=1);
INSERT OR IGNORE INTO behavior_provenance_migration(id,version) VALUES (1,1);
COMMIT;

"#;
impl DatabaseManager {
    pub(in crate::db) fn initialize_schema(&self) -> Result<(), DatabaseError> {
        let points = self.open(&self.points_path())?;
        points.execute_batch(POINTS_SCHEMA)?;
        points.execute_batch(COORDINATION_SCHEMA)?;
        points.execute_batch("INSERT OR IGNORE INTO control_resource_versions(kind,scope,resource_id,revision,deleted) SELECT 'viewer','host',unique_id,1,0 FROM viewers;")?;
        let count: i64 =
            points.query_row("SELECT COUNT(*) FROM points_config", [], |row| row.get(0))?;
        if count == 0 {
            let config = PointsConfig::default();
            insert_points_config(&points, &config, now())?;
        }

        let mut automation = self.open(&self.automation_path())?;
        // The workflow seed references the shared version table.
        automation.execute_batch(COORDINATION_SCHEMA)?;
        automation.execute_batch(AUTOMATION_SCHEMA)?;
        migrate_effect_kinds(&mut automation)?;

        self.ensure_analytics_schema()?;
        Ok(())
    }
    pub(in crate::db) fn open(&self, path: &std::path::Path) -> Result<Connection, DatabaseError> {
        Ok(Connection::open(path)?)
    }
}

/// Extend the unreleased journal's closed kind constraint without deleting any
/// accepted/unknown/tombstoned identity. Copy/constraint validation and rename
/// are one owned SQLite transaction; failure leaves the old table intact.
fn migrate_effect_kinds(db: &mut Connection) -> Result<(), DatabaseError> {
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let schema: String = tx.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='automation_external_effects'",
        [],
        |row| row.get(0),
    )?;
    if !schema.contains("'script'") {
        tx.execute_batch("CREATE TABLE automation_external_effects_v2 (
          effect_id TEXT PRIMARY KEY CHECK(length(effect_id)=64),
          kind TEXT NOT NULL CHECK(kind IN ('http','audio','plugin','emit','script')),
          binding_digest TEXT NOT NULL CHECK(length(binding_digest)=64),
          request_digest TEXT NOT NULL CHECK(length(request_digest)=64),
          instance_id TEXT NOT NULL CHECK(length(instance_id)=64),
          state TEXT NOT NULL CHECK(state IN ('accepted','dispatched','succeeded','failed','unknown')),
          accepted_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
          outcome_expired INTEGER NOT NULL DEFAULT 0 CHECK(outcome_expired IN (0,1))
        );
        INSERT INTO automation_external_effects_v2 SELECT effect_id,kind,binding_digest,request_digest,instance_id,state,accepted_at,updated_at,outcome_expired FROM automation_external_effects;
        DROP TABLE automation_external_effects;
        ALTER TABLE automation_external_effects_v2 RENAME TO automation_external_effects;")?;
        let kinds: String = tx.query_row("SELECT sql FROM sqlite_master WHERE type='table' AND name='automation_external_effects'", [], |row| row.get(0))?;
        if !kinds.contains("'script'") {
            return Err(DatabaseError::Invalid(
                "effect journal migration failed".into(),
            ));
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod effect_migration_tests {
    use super::*;
    fn old_schema(db: &Connection, emit: bool) {
        let kinds = if emit {
            "'http','audio','plugin','emit'"
        } else {
            "'http','audio','plugin'"
        };
        db.execute_batch(&format!(
            "CREATE TABLE automation_external_effects (
          effect_id TEXT PRIMARY KEY CHECK(length(effect_id)=64),
          kind TEXT NOT NULL CHECK(kind IN ({kinds})),
          binding_digest TEXT NOT NULL, request_digest TEXT NOT NULL, instance_id TEXT NOT NULL,
          state TEXT NOT NULL, accepted_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
          outcome_expired INTEGER NOT NULL DEFAULT 0);"
        ))
        .unwrap();
        for (index, state) in ["accepted", "dispatched", "succeeded", "failed", "unknown"]
            .iter()
            .enumerate()
        {
            db.execute(
                "INSERT INTO automation_external_effects VALUES(?1,'http',?2,?3,?4,?5,10,20,?6)",
                rusqlite::params![
                    format!("{index:064x}"),
                    "b".repeat(64),
                    "c".repeat(64),
                    "d".repeat(64),
                    state,
                    i32::from(index == 4)
                ],
            )
            .unwrap();
        }
    }
    type EffectRow = (
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        bool,
    );
    fn rows(db: &Connection) -> Vec<EffectRow> {
        let mut stmt = db
            .prepare("SELECT * FROM automation_external_effects ORDER BY effect_id")
            .unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    }
    #[test]
    fn legacy_effect_migration_preserves_every_retained_identity_and_accepts_new_kinds() {
        for emit in [false, true] {
            let mut db = Connection::open_in_memory().unwrap();
            old_schema(&db, emit);
            let before = rows(&db);
            migrate_effect_kinds(&mut db).unwrap();
            migrate_effect_kinds(&mut db).unwrap();
            assert_eq!(rows(&db), before);
            for (index, kind) in ["emit", "script"].into_iter().enumerate() {
                db.execute("INSERT INTO automation_external_effects VALUES(?1,?2,?3,?3,?3,'unknown',10,20,0)",
                    rusqlite::params![format!("{:064x}",100+index),kind,"a".repeat(64)]).unwrap();
            }
            assert_eq!(rows(&db).len(), 7);
        }
    }
    #[test]
    fn invalid_legacy_effect_migration_rolls_back_without_erasing_history() {
        let mut db = Connection::open_in_memory().unwrap();
        old_schema(&db, false);
        db.execute(
            "UPDATE automation_external_effects SET state='corrupt' WHERE effect_id=?1",
            [format!("{:064x}", 0)],
        )
        .unwrap();
        let before = rows(&db);
        assert!(migrate_effect_kinds(&mut db).is_err());
        assert_eq!(rows(&db), before);
        let schema: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='automation_external_effects'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!schema.contains("'script'"));
        let shadow: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='automation_external_effects_v2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(shadow, 0);
    }
}
