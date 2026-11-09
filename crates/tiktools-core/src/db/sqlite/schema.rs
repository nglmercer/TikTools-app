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
-- The source revision remains after deletion, so remove/recreate and ABA
-- changes cannot make a stale Cloud review valid again.
CREATE TABLE IF NOT EXISTS plugin_sync_source_versions (
  plugin_id TEXT PRIMARY KEY,
  generation INTEGER NOT NULL CHECK(generation > 0)
);
-- A local mutation made without Cloud admission remains reviewable across
-- restarts. No identity or consent may be inferred from this marker.
CREATE TABLE IF NOT EXISTS plugin_sync_untracked_local (
  plugin_id TEXT PRIMARY KEY,
  generation INTEGER NOT NULL CHECK(generation > 0)
);
-- Native-only package operation fence, global across accounts. An unfinished
-- mutation must survive process death until filesystem/runtime reconciliation.
CREATE TABLE IF NOT EXISTS plugin_package_mutation_intents (
  plugin_id TEXT NOT NULL PRIMARY KEY,
  intent_id TEXT NOT NULL UNIQUE,
  operation TEXT NOT NULL CHECK(operation IN ('install','update','rollback','uninstall')),
  operation_digest TEXT NOT NULL,
  phase TEXT NOT NULL CHECK(phase IN ('prepared','mutating')),
  source_generation INTEGER NOT NULL CHECK(source_generation > 0)
);
-- Retain resolved IDs so an interrupted caller cannot replay filesystem work.
-- Capacity is reserved before mutation; no evidence is automatically evicted.
CREATE TABLE IF NOT EXISTS plugin_package_mutation_receipts (
  intent_id TEXT NOT NULL PRIMARY KEY,
  plugin_id TEXT NOT NULL,
  operation TEXT NOT NULL CHECK(operation IN ('install','update','rollback','uninstall')),
  operation_digest TEXT NOT NULL,
  resolution TEXT NOT NULL CHECK(resolution IN ('cancelled','reviewed')),
  source_generation INTEGER NOT NULL CHECK(source_generation > 0)
);
-- Native-only durable candidates; identity is partitioned and never a token.
-- Acknowledgement must compare the generation so it cannot erase a newer edit.
CREATE TABLE IF NOT EXISTS plugin_sync_outbox (
  origin TEXT NOT NULL,
  account_id TEXT NOT NULL,
  plugin_id TEXT NOT NULL,
  generation INTEGER NOT NULL CHECK(generation > 0),
  config_generation TEXT NOT NULL,
  -- Runtime confirmation is scoped to one host lifetime and must be
  -- re-established after restart before a candidate may be acknowledged.
  runtime_epoch TEXT,
  -- JSON null is a reviewed absence, 'direct' is a new direct edit and SQL
  -- NULL is an older unclassified candidate that requires review.
  reviewed_base TEXT,
  version TEXT,
  enabled INTEGER CHECK(enabled IN (0,1)),
  CHECK((version IS NULL AND enabled IS NULL) OR
        (version IS NOT NULL AND enabled IS NOT NULL)),
  PRIMARY KEY(origin, account_id, plugin_id)
);
-- A staged v1 pending value retains its source and consent fence after the
-- outbox entry is consumed. Without this receipt another account's later
-- local edit could leave an obsolete pending value eligible for upload.
CREATE TABLE IF NOT EXISTS plugin_sync_receipts (
  origin TEXT NOT NULL,
  account_id TEXT NOT NULL,
  plugin_id TEXT NOT NULL,
  generation INTEGER NOT NULL CHECK(generation > 0),
  config_generation TEXT NOT NULL,
  runtime_epoch TEXT NOT NULL,
  pending_json TEXT NOT NULL,
  PRIMARY KEY(origin, account_id, plugin_id)
);
-- A conflict decision is frozen before either the v1 JSON state or its
-- receipt is changed. At most one unresolved decision is admitted per account.
CREATE TABLE IF NOT EXISTS plugin_sync_rebase_intents (
  origin TEXT NOT NULL,
  account_id TEXT NOT NULL,
  plugin_id TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  PRIMARY KEY(origin, account_id)
);
-- An application decision survives crashes without treating the persisted
-- remote value as permission to run a plugin. The opaque native payload
-- contains no credential or plugin source.
CREATE TABLE IF NOT EXISTS plugin_sync_apply_intents (
  intent_id TEXT PRIMARY KEY,
  plugin_id TEXT NOT NULL,
  phase TEXT NOT NULL CHECK(phase IN ('prepared','ready','runtime_pending','applied','finalized')),
  payload_json TEXT NOT NULL,
  committed_generation INTEGER,
  runtime_epoch TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_plugin_sync_apply_active
  ON plugin_sync_apply_intents(plugin_id) WHERE phase != 'finalized';
-- Discovery pages should not scan retained finalized history on every restart.
CREATE INDEX IF NOT EXISTS idx_plugin_sync_apply_recovery
  ON plugin_sync_apply_intents(intent_id) WHERE phase != 'finalized';
-- An independently retained witness lets startup reject a damaged phase or
-- finalized row. It is written in the same transaction as finalization.
CREATE TABLE IF NOT EXISTS plugin_sync_apply_finalizations (
  intent_id TEXT PRIMARY KEY,
  plugin_id TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  committed_generation INTEGER NOT NULL,
  runtime_epoch TEXT NOT NULL
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
CREATE TABLE IF NOT EXISTS automation_library_catalog (
  id INTEGER PRIMARY KEY CHECK(id=1),
  revision INTEGER NOT NULL CHECK(typeof(revision)='integer' AND revision>=0),
  catalog_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS automation_library_migration (
  id INTEGER PRIMARY KEY CHECK(id=1),
  version INTEGER NOT NULL CHECK(version=1)
);
CREATE TABLE IF NOT EXISTS owner_automation_library (
  command_id TEXT PRIMARY KEY,
  pending INTEGER NOT NULL CHECK(pending IN (0,1)),
  payload_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_owner_library_pending
  ON owner_automation_library(pending) WHERE pending=1;
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
        migrate_competition_revisions(&mut automation)?;
        migrate_plugin_sync_outbox(&mut automation)?;
        migrate_package_journal(&mut automation)?;
        migrate_effect_kinds(&mut automation)?;

        self.ensure_analytics_schema()?;
        Ok(())
    }
    pub(in crate::db) fn open(&self, path: &std::path::Path) -> Result<Connection, DatabaseError> {
        Ok(Connection::open(path)?)
    }
}

/// Competition definitions and lifecycle state share one generation-independent
/// revision. SQLite triggers cover every writer (including timer/finalization
/// paths) and preserve tombstones when an ID is deleted and recreated.
fn migrate_competition_revisions(db: &mut Connection) -> Result<(), DatabaseError> {
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(r#"
CREATE TABLE IF NOT EXISTS competition_control_revisions (
  competition_id TEXT PRIMARY KEY,
  revision INTEGER NOT NULL CHECK(typeof(revision)='integer' AND revision>0),
  deleted INTEGER NOT NULL CHECK(deleted IN (0,1))
);
CREATE TABLE IF NOT EXISTS competition_revision_migration (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1)
);
INSERT INTO competition_control_revisions(competition_id,revision,deleted)
  SELECT id,1,0 FROM competition_definitions
  WHERE NOT EXISTS(SELECT 1 FROM competition_revision_migration WHERE singleton=1);
INSERT OR IGNORE INTO competition_revision_migration(singleton) VALUES (1);
DROP TRIGGER IF EXISTS competition_revision_insert;
DROP TRIGGER IF EXISTS competition_revision_definition_update;
DROP TRIGGER IF EXISTS competition_revision_definition_delete;
DROP TRIGGER IF EXISTS competition_revision_state_update;
CREATE TRIGGER competition_revision_insert AFTER INSERT ON competition_definitions BEGIN
  INSERT INTO competition_control_revisions(competition_id,revision,deleted) VALUES (NEW.id,1,0)
  ON CONFLICT(competition_id) DO UPDATE SET
    revision=CASE WHEN deleted=1 THEN revision+1 ELSE RAISE(ABORT,'competition revision inconsistent') END,
    deleted=0;
  INSERT INTO control_resource_outbox(kind,scope,resource_id,revision,change,committed_at)
    SELECT 'competition','host',NEW.id,revision,'created',CAST((julianday('now')-2440587.5)*86400000 AS INTEGER)
    FROM competition_control_revisions WHERE competition_id=NEW.id;
END;
CREATE TRIGGER competition_revision_definition_update AFTER UPDATE ON competition_definitions BEGIN
  SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM competition_control_revisions WHERE competition_id=NEW.id AND deleted=0)
    THEN RAISE(ABORT,'competition revision missing') END;
  UPDATE competition_control_revisions SET revision=revision+1 WHERE competition_id=NEW.id;
  INSERT INTO control_resource_outbox(kind,scope,resource_id,revision,change,committed_at)
    SELECT 'competition','host',NEW.id,revision,'updated',CAST((julianday('now')-2440587.5)*86400000 AS INTEGER)
    FROM competition_control_revisions WHERE competition_id=NEW.id;
END;
CREATE TRIGGER competition_revision_definition_delete AFTER DELETE ON competition_definitions BEGIN
  SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM competition_control_revisions WHERE competition_id=OLD.id AND deleted=0)
    THEN RAISE(ABORT,'competition revision missing') END;
  UPDATE competition_control_revisions SET revision=revision+1,deleted=1 WHERE competition_id=OLD.id;
  INSERT INTO control_resource_outbox(kind,scope,resource_id,revision,change,committed_at)
    SELECT 'competition','host',OLD.id,revision,'deleted',CAST((julianday('now')-2440587.5)*86400000 AS INTEGER)
    FROM competition_control_revisions WHERE competition_id=OLD.id;
END;
CREATE TRIGGER competition_revision_state_update AFTER UPDATE ON competition_state
WHEN OLD.state_json IS NOT NEW.state_json BEGIN
  SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM competition_control_revisions WHERE competition_id=NEW.competition_id AND deleted=0)
    THEN RAISE(ABORT,'competition revision missing') END;
  UPDATE competition_control_revisions SET revision=revision+1 WHERE competition_id=NEW.competition_id;
  INSERT INTO control_resource_outbox(kind,scope,resource_id,revision,change,committed_at)
    SELECT 'competition','host',NEW.competition_id,revision,'state-changed',CAST((julianday('now')-2440587.5)*86400000 AS INTEGER)
    FROM competition_control_revisions WHERE competition_id=NEW.competition_id;
END;
"#)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod competition_revision_tests {
    use super::*;

    #[test]
    fn migration_and_writers_preserve_revisions_across_delete_and_recreate() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch(COORDINATION_SCHEMA).unwrap();
        db.execute_batch("CREATE TABLE competition_definitions(id TEXT PRIMARY KEY, definition_json TEXT NOT NULL); CREATE TABLE competition_state(competition_id TEXT PRIMARY KEY, state_json TEXT NOT NULL); INSERT INTO competition_definitions VALUES ('a','old'); INSERT INTO competition_state VALUES ('a','draft');").unwrap();
        migrate_competition_revisions(&mut db).unwrap();
        let revision = |db: &Connection| {
            db.query_row("SELECT revision,deleted FROM competition_control_revisions WHERE competition_id='a'", [], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).unwrap()
        };
        assert_eq!(revision(&db), (1, 0));
        db.execute(
            "UPDATE competition_state SET state_json='active' WHERE competition_id='a'",
            [],
        )
        .unwrap();
        assert_eq!(revision(&db), (2, 0));
        db.execute(
            "UPDATE competition_definitions SET definition_json='changed' WHERE id='a'",
            [],
        )
        .unwrap();
        assert_eq!(revision(&db), (3, 0));
        db.execute("DELETE FROM competition_definitions WHERE id='a'", [])
            .unwrap();
        assert_eq!(revision(&db), (4, 1));
        db.execute("INSERT INTO competition_definitions VALUES ('a','new')", [])
            .unwrap();
        assert_eq!(revision(&db), (5, 0));
        migrate_competition_revisions(&mut db).unwrap();
        assert_eq!(revision(&db), (5, 0));
        let changes: Vec<(i64, String)> = db
            .prepare("SELECT revision,change FROM control_resource_outbox WHERE kind='competition' AND resource_id='a' ORDER BY cursor")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            changes,
            vec![
                (2, "state-changed".into()),
                (3, "updated".into()),
                (4, "deleted".into()),
                (5, "created".into())
            ]
        );
        db.execute(
            "DELETE FROM competition_control_revisions WHERE competition_id='a'",
            [],
        )
        .unwrap();
        assert!(db
            .execute(
                "UPDATE competition_state SET state_json='paused' WHERE competition_id='a'",
                []
            )
            .is_err());
        migrate_competition_revisions(&mut db).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM competition_control_revisions",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
}

/// Development builds may contain unresolved operations without a frozen
/// digest. Preserve them as NULL, which journal readers reject for review;
/// inventing a digest would authorize a different package operation.
fn migrate_package_journal(db: &mut Connection) -> Result<(), DatabaseError> {
    db.execute_batch("PRAGMA synchronous=FULL;")?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let present: i64 = tx.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('plugin_package_mutation_intents') WHERE name='operation_digest'",
        [], |row| row.get(0),
    )?;
    if present == 0 {
        tx.execute_batch(
            "ALTER TABLE plugin_package_mutation_intents ADD COLUMN operation_digest TEXT;",
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Interim development builds created this outbox before consent generations
/// were recorded. Preserve every candidate and leave its generation NULL:
/// readers must require a fresh review instead of inventing prior consent.
fn migrate_plugin_sync_outbox(db: &mut Connection) -> Result<(), DatabaseError> {
    db.execute_batch("PRAGMA synchronous=FULL;")?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let present: i64 = tx.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('plugin_sync_outbox') WHERE name='config_generation'",
        [],
        |row| row.get(0),
    )?;
    if present == 0 {
        tx.execute_batch("ALTER TABLE plugin_sync_outbox ADD COLUMN config_generation TEXT;")?;
    }
    let present: i64 = tx.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('plugin_sync_outbox') WHERE name='runtime_epoch'",
        [],
        |row| row.get(0),
    )?;
    if present == 0 {
        tx.execute_batch("ALTER TABLE plugin_sync_outbox ADD COLUMN runtime_epoch TEXT;")?;
    }
    let present: i64 = tx.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('plugin_sync_outbox') WHERE name='reviewed_base'",
        [],
        |row| row.get(0),
    )?;
    if present == 0 {
        tx.execute_batch("ALTER TABLE plugin_sync_outbox ADD COLUMN reviewed_base TEXT;")?;
    }
    tx.commit()?;
    Ok(())
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
