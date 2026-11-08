//! Additive commands share the balance transaction, never the optimistic queue.
//! The service drains earlier snapshots under its mutation gate before entry.
use super::*;
use crate::{
    collaboration::{AdditiveCommandRequest, ResourceKind, ResourceVersion, Versioned},
    control::PointsAdjustment,
    services::PointAward,
};
use rusqlite::{OptionalExtension, TransactionBehavior};

const PROFILE_BYTES: usize = 16 * 1024;
type Command = super::super::commands::CommandTransaction<AdditiveCommandRequest>;

pub(super) fn bounded_viewer(
    db: &Connection,
    id: &str,
    bytes: usize,
) -> Result<Option<ViewerState>, DatabaseError> {
    let size:Option<i64>=db.query_row("SELECT length(CAST(unique_id AS BLOB))+coalesce(length(CAST(user_id AS BLOB)),0)+coalesce(length(CAST(nickname AS BLOB)),0)+coalesce(length(CAST(avatar_url AS BLOB)),0) FROM viewers WHERE unique_id=?1",[id],|r|r.get(0)).optional()?;
    let Some(size) = size else {
        return Ok(None);
    };
    if size < 0 || size as u64 > bytes as u64 {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    let value = db.query_row(
        &format!("{VIEWER_COLUMNS_SQL} WHERE unique_id=?1"),
        [id],
        row_to_viewer_value,
    )?;
    if !value
        .get("points")
        .and_then(Value::as_f64)
        .is_some_and(|n| n.is_finite() && n >= 0.0)
        || !value
            .get("level")
            .and_then(Value::as_u64)
            .is_some_and(|n| n >= 1 && n <= u64::from(u32::MAX))
    {
        return Err(DatabaseError::PointsStateInvalid);
    }
    ViewerState::from_value(&value)
        .map(Some)
        .ok_or(DatabaseError::PointsStateInvalid)
}

pub(super) fn revision(db: &Connection, id: &str) -> Result<i64, DatabaseError> {
    Ok(db.query_row("SELECT revision FROM control_resource_versions WHERE kind='viewer' AND scope='host' AND resource_id=?1",[id],|r|r.get(0)).optional()?.unwrap_or(0))
}

fn authorize(
    operation: &PointsAdjustment,
    command: &Command,
    effective: Option<f64>,
) -> Result<(), DatabaseError> {
    if let Some(auth) = &command.authorization {
        let identity = operation
            .authorize(auth, effective)
            .map_err(DatabaseError::CommandAuthorization)?;
        if identity.principal_id != command.principal_id
            || identity.session_id != command.session_id
        {
            return Err(DatabaseError::CommandAuthorization(
                crate::collaboration::Error::Forbidden,
            ));
        }
    }
    Ok(())
}

fn hash(operation: &PointsAdjustment, command: &Command) -> Result<String, DatabaseError> {
    operation
        .validate()
        .map_err(|_| DatabaseError::Invalid("invalid points adjustment".into()))?;
    command
        .request
        .validate()
        .map_err(|_| DatabaseError::Invalid("invalid command metadata".into()))?;
    let mut params = serde_json::json!({"uniqueId":operation.unique_id,"delta":operation.delta});
    // Preserve hashes for every existing unchecked intent and tombstone.
    if let Some(currency) = &operation.expected_currency {
        params["expectedCurrency"] = currency.clone().into();
    }
    super::super::commands::additive_hash("points.adjust", &params)
}

fn replay(
    db: &Connection,
    operation: &PointsAdjustment,
    command: &Command,
    hash: &str,
) -> Result<Option<Versioned<Value>>, DatabaseError> {
    authorize(operation, command, None)?;
    let result = super::super::commands::replay(db, command, hash)?;
    if let Some(result) = &result {
        let delta = result
            .value
            .get("delta")
            .and_then(Value::as_f64)
            .filter(|n| n.is_finite())
            .ok_or(DatabaseError::PointsStateInvalid)?;
        authorize(operation, command, Some(delta))?;
    }
    Ok(result)
}

impl DatabaseManager {
    pub(crate) fn replay_points_command(
        &self,
        operation: &PointsAdjustment,
        command: &Command,
    ) -> Result<Versioned<Value>, DatabaseError> {
        let hash = hash(operation, command)?;
        let mut db = self.open(&self.points_path())?;
        db.busy_timeout(std::time::Duration::from_millis(100))?;
        // A retained result is immutable and this path performs no writes.
        // Keep a coherent read snapshot without competing with queued awards
        // for writer admission; replay must never wait for a cache flush.
        let tx = db.transaction()?;
        let result = replay(&tx, operation, command, &hash)?.ok_or(DatabaseError::NotFound)?;
        tx.commit()?;
        Ok(result)
    }

    pub(crate) fn execute_points_command(
        &self,
        operation: &PointsAdjustment,
        config: &PointsConfig,
        command: &Command,
        admission: std::time::Duration,
        #[cfg(test)] after_admission: Option<crate::services::AdmissionPause>,
    ) -> Result<(Versioned<Value>, Option<ViewerState>), DatabaseError> {
        let hash = hash(operation, command)?;
        let mut db = self.open(&self.points_path())?;
        db.busy_timeout(admission)?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        #[cfg(test)]
        if let Some(pause) = after_admission {
            pause.wait();
        }
        // This check occurs after actual SQLite writer admission, not just
        // after the outer mutex. A queued command may have lost its grant.
        if let Some(result) = replay(&tx, operation, command, &hash)? {
            tx.commit()?;
            return Ok((result, None));
        }
        if operation
            .expected_currency
            .as_ref()
            .is_some_and(|name| *name != config.currency_name)
        {
            return Err(DatabaseError::CurrencyChanged);
        }
        let timestamp = now();
        let mut viewer =
            bounded_viewer(&tx, &operation.unique_id, PROFILE_BYTES)?.unwrap_or_else(|| {
                ViewerState {
                    unique_id: operation.unique_id.clone(),
                    user_id: None,
                    nickname: operation.unique_id.clone(),
                    avatar_url: None,
                    points: 0.0,
                    level: 1,
                    is_subscriber: false,
                    total_chats: 0,
                    total_coins: 0,
                    total_likes: 0,
                    total_shares: 0,
                    first_seen: timestamp,
                    last_seen: timestamp,
                }
            });
        let before = revision(&tx, &operation.unique_id)?;
        let mut delta = operation.delta;
        if viewer.is_subscriber && config.sub_bonus_multiplier > 0.0 && delta > 0.0 {
            delta += delta * (config.sub_bonus_multiplier / 100.0);
        }
        delta = (delta * 100.0).round() / 100.0;
        let total = (((viewer.points + delta).max(0.0)) * 100.0).round() / 100.0;
        if !delta.is_finite() || !total.is_finite() {
            return Err(DatabaseError::Invalid(
                "points arithmetic exceeds its finite range".into(),
            ));
        }
        authorize(operation, command, Some(delta))?;
        viewer.points = total;
        viewer.level =
            ((total / config.points_per_level.max(10.0)).floor() as u32).saturating_add(1);
        viewer.last_seen = timestamp;
        // Reuse the legacy manual-award semantics and its atomic invalidation,
        // without changing unrelated identity/counter fields.
        let op = PointsWriteOp::Save {
            seq: 0,
            viewer: viewer.clone(),
            reason: "manual",
            awarded: delta,
        };
        apply_points_op_on(&tx, &op)?;
        versions::Reservations::new(&tx)?.reserve(&tx, &op)?;
        let award = PointAward {
            unique_id: viewer.unique_id.clone(),
            delta,
            total_points: total,
            level: viewer.level,
            currency_name: config.currency_name.clone(),
        };
        let result = Versioned::new(
            serde_json::to_value(award)?,
            vec![ResourceVersion {
                kind: ResourceKind::Viewer,
                scope: "host".into(),
                id: viewer.unique_id.clone(),
                revision: revision(&tx, &viewer.unique_id)?.to_string(),
            }],
        );
        let result = super::super::commands::finish(
            &tx,
            command,
            &hash,
            "points.adjust",
            &before.to_string(),
            result,
        )?;
        #[cfg(test)]
        if std::env::var("TIKTOOLS_POINTS_COMMAND_CRASH_STAGE").as_deref() == Ok("before_commit") {
            std::process::exit(96);
        }
        tx.commit()?;
        #[cfg(test)]
        if std::env::var("TIKTOOLS_POINTS_COMMAND_CRASH_STAGE").as_deref() == Ok("after_commit") {
            std::process::exit(97);
        }
        Ok((result, Some(viewer)))
    }
}
